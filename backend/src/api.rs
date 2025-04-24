use crate::common::{GlobalData, SyncPackage};
use std::error::Error;
use std::time::Duration;
use std::convert::Infallible;
use tokio_tungstenite::connect_async;
use futures_util::StreamExt;
use reqwest::Client;
use std::sync::Arc;
use warp::Filter;
use warp::http::StatusCode;
use warp::reply::Json;
use serde_json::json;
use serde::{Deserialize, Serialize};
use warp::http::Method;
use warp::http::header::{AUTHORIZATION, CONTENT_TYPE};

// Mihomo API客户端
pub struct MihomoClient {
    host: String,
    port: u16,
    token: String,
}

impl MihomoClient {
    pub fn new(host: String, port: u16, token: String) -> Self {
        Self { host, port, token }
    }

    // 连接到Mihomo WebSocket API并处理数据，使用同步回调函数
    pub async fn connect<F>(&self, mut callback: F) -> Result<(), Box<dyn Error>>
    where
        F: FnMut(GlobalData) -> Result<(), String> + Send + 'static,
    {
        let ws_uri = format!(
            "ws://{}:{}/connections?token={}",
            self.host, self.port, self.token
        );

        println!("连接到Mihomo API: {}:{}...", self.host, self.port);

        loop {
            match connect_async(&ws_uri).await {
                Ok((ws_stream, _)) => {
                    println!("已连接到Mihomo API");
                    let (_, mut read) = ws_stream.split();
                    
                    // 直接等待消息，不设置超时
                    loop {
                        match read.next().await {
                            Some(Ok(message)) => {
                                if message.is_text() {
                                    let text = message.into_text()?;
                                    match serde_json::from_str::<GlobalData>(&text) {
                                        Ok(data) => {
                                            if let Err(e) = callback(data) {
                                                println!("处理数据出错: {}", e);
                                            }
                                        },
                                        Err(e) => {
                                            println!("解析消息失败: {}\n原始数据: {}", e, text);
                                        }
                                    };
                                }
                            },
                            Some(Err(e)) => {
                                println!("WebSocket错误，重新连接: {:?}", e);
                                break;
                            },
                            None => {
                                // 流结束，这表示连接已经正常关闭
                                println!("WebSocket连接已关闭，重新连接中...");
                                break;
                            }
                        }
                    }
                },
                Err(e) => {
                    println!("连接错误: {:?}, 重试中...", e);
                    tokio::time::sleep(Duration::from_secs(5)).await;
                }
            }
        }
    }
}

// 主节点API客户端
pub struct MasterClient {
    client: Client,
    master_url: String,
    master_token: Option<String>,
}

impl MasterClient {
    pub fn new(master_url: String, master_token: Option<String>) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(10))
            .build()
            .expect("创建HTTP客户端失败");
        
        Self { client, master_url, master_token }
    }

    // 检查主节点是否在线
    pub async fn is_online(&self) -> bool {
        let health_url = format!("{}/api/v1/health", self.master_url);
        match self.client.get(&health_url).send().await {
            Ok(response) => response.status().is_success(),
            Err(_) => false,
        }
    }

    // 同步数据到主节点
    pub async fn sync_data(&self, package: &SyncPackage) -> Result<(), Box<dyn Error + Send + Sync>> {
        let sync_url = format!("{}/api/v1/sync", self.master_url);
        let mut request = self.client.post(&sync_url);
        
        if let Some(token) = &self.master_token {
            request = request.header("Authorization", format!("Bearer {}", token));
        }
        
        let response = request
            .json(package)
            .send()
            .await?;
            
        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await?;
            return Err(format!("同步失败: {} - {}", status, text).into());
        }
        
        Ok(())
    }
}

// API 响应结构
#[derive(Serialize)]
struct ApiResponse<T: Serialize> {
    status: String,
    data: Option<T>,
    message: Option<String>,
}

impl<T: Serialize> ApiResponse<T> {
    // 创建成功响应
    fn success(data: T) -> Json {
        warp::reply::json(&json!({
            "status": "success",
            "data": data,
            "message": null
        }))
    }
}

// 统一的API响应处理
type ApiResult = Result<Json, warp::Rejection>;

// 主节点API服务器
pub struct MasterServer {
    database: Arc<crate::db::Database>,
    api_token: Option<String>,
}

impl MasterServer {
    pub fn new(database: Arc<crate::db::Database>, api_token: Option<String>) -> Self {
        Self { database, api_token }
    }

    // 构建API认证过滤器
    fn with_auth(&self) -> impl Filter<Extract = (), Error = warp::Rejection> + Clone {
        let token = self.api_token.clone();
        
        warp::header::optional::<String>("authorization")
            .and_then(move |auth: Option<String>| {
                let token = token.clone();
                async move {
                    // 如果未设置令牌，则无需验证
                    if token.is_none() {
                        return Ok(());
                    }
                    
                    match auth {
                        Some(auth_header) if auth_header.starts_with("Bearer ") => {
                            let provided_token = auth_header[7..].to_string();
                            if provided_token == token.unwrap() {
                                Ok(())
                            } else {
                                Err(warp::reject::custom(ApiError::Unauthorized))
                            }
                        }
                        _ => Err(warp::reject::custom(ApiError::Unauthorized)),
                    }
                }
            })
            .untuple_one()
    }

    // 构建数据库访问过滤器
    fn with_db(&self) -> impl Filter<Extract = (Arc<crate::db::Database>,), Error = Infallible> + Clone {
        let db = self.database.clone();
        warp::any().map(move || db.clone())
    }

    // 启动API服务器
    pub async fn start(&self, host: &str, port: u16) -> Result<(), Box<dyn Error>> {
        // 健康检查路由
        let health_route = warp::path!("api" / "v1" / "health")
            .and(warp::get())
            .map(|| "OK");
        
        // 同步路由
        let sync_route = warp::path!("api" / "v1" / "sync")
            .and(warp::post())
            .and(self.with_auth())
            .and(warp::body::json())
            .and(self.with_db())
            .and_then(|sync_package: SyncPackage, db: Arc<crate::db::Database>| async move {
                handle_sync(sync_package, db).await
            });
        
        // 统计路由 - 统一入口
        let stats_route = warp::path!("api" / "v1" / "stats")
            .and(warp::get())
            .and(self.with_auth())
            .and(warp::query::<StatsQuery>())
            .and(self.with_db())
            .and_then(|query: StatsQuery, db: Arc<crate::db::Database>| async move {
                handle_stats(query, db).await
            });

        // 配置 CORS
        let cors = warp::cors()
            .allow_any_origin()
            .allow_methods(vec![Method::GET, Method::POST, Method::OPTIONS])
            .allow_headers(vec![CONTENT_TYPE, AUTHORIZATION])
            .allow_credentials(false);
            
        // 合并所有路由并添加 CORS 和错误处理
        let routes = health_route
            .or(sync_route)
            .or(stats_route)
            .with(cors)
            .recover(handle_rejection);
            
        println!("启动主节点API服务器在 {}:{}...", host, port);
        let socket_addr: std::net::SocketAddr = format!("{}:{}", host, port)
            .parse()
            .map_err(|e| format!("无效的地址: {}", e))?;
            
        warp::serve(routes)
            .run(socket_addr)
            .await;
            
        Ok(())
    }
}

// API错误类型
#[allow(dead_code)]
#[derive(Debug)]
enum ApiError {
    Unauthorized,                // 未授权
    BadRequest(String),          // 请求参数错误
    DatabaseError(String),       // 数据库错误
    NotFound,                    // 资源不存在
    InternalError(String),       // 内部服务器错误
}

impl warp::reject::Reject for ApiError {}

// 统一的统计查询参数
#[derive(Deserialize, Debug)]
struct StatsQuery {
    r#type: String,               // 统计类型: summary, group, timeseries
    group_by: Option<String>,     // 分组字段(用于group类型): network, rule, process, destination, geoip
    interval: Option<String>,     // 时间间隔(用于timeseries类型): hour, day, week, month
    metric: Option<String>,       // 统计指标: connections, download, upload, total
    from: Option<String>,         // 开始时间，ISO 8601格式
    to: Option<String>,           // 结束时间，ISO 8601格式
    agent_id: Option<String>,     // 代理ID过滤
    network: Option<String>,      // 网络类型过滤
    rule: Option<String>,         // 规则过滤
    process: Option<String>,      // 进程名过滤
    destination: Option<String>,  // 目标地址过滤
    geoip: Option<String>,        // 国家/地区过滤
    limit: Option<u32>,           // 限制返回结果数量
    sort_by: Option<String>,      // 排序字段
    sort_order: Option<String>,   // 排序顺序 (asc, desc)
}

// 处理统计请求的统一入口
async fn handle_stats(
    query: StatsQuery,
    db: Arc<crate::db::Database>,
) -> ApiResult {
    // 根据统计类型分发到不同的处理函数
    match query.r#type.as_str() {
        "summary" => handle_stats_summary(query, db).await,
        "group" => handle_stats_group(query, db).await,
        "timeseries" => handle_stats_timeseries(query, db).await,
        _ => Err(warp::reject::custom(ApiError::BadRequest(
            format!("不支持的统计类型: {}", query.r#type)
        )))
    }
}

// 处理汇总统计请求
async fn handle_stats_summary(
    query: StatsQuery,
    db: Arc<crate::db::Database>,
) -> ApiResult {
    // 构建基础SQL和参数
    let (count_sql, count_params) = build_base_filter(
        "SELECT COUNT(*) as count FROM connections",
        &query
    );
    
    let (traffic_sql, traffic_params) = build_base_filter(
        "SELECT SUM(conn_download) as download, SUM(conn_upload) as upload FROM connections",
        &query
    );
    
    // 执行查询
    let count_result = db.execute_count_query(&count_sql, &count_params).await;
    let traffic_result = db.execute_traffic_query(&traffic_sql, &traffic_params).await;
    
    match (count_result, traffic_result) {
        (Ok(count), Ok((download, upload))) => {
            Ok(ApiResponse::success(json!({
                "count": count,
                "download": download,
                "upload": upload,
                "total": download + upload
            })))
        },
        (Err(e), _) | (_, Err(e)) => {
            eprintln!("统计查询错误: {:?}", e);
            Err(warp::reject::custom(ApiError::DatabaseError(
                format!("统计查询失败: {}", e)
            )))
        }
    }
}

// 处理分组统计请求
async fn handle_stats_group(
    query: StatsQuery,
    db: Arc<crate::db::Database>,
) -> ApiResult {
    // 检查是否指定了分组字段
    let group_field = match &query.group_by {
        Some(field) => field.as_str(),
        None => return Err(warp::reject::custom(ApiError::BadRequest(
            "分组统计必须指定 group_by 参数".to_string()
        )))
    };
    
    // 确定分组字段对应的数据库列名
    let (column_name, is_chains_field) = match group_field {
        "network" => ("network", false),
        "rule" => ("rule", false),
        "process" => ("process", false),
        "destination" => ("destination_ip", false),
        "geoip" => ("destination_geoip", false),
        "host" => ("host", false),
        "chains" => ("chains", true), // 特殊处理，需要提取最后一个节点
        _ => return Err(warp::reject::custom(ApiError::BadRequest(
            format!("不支持的分组字段: {}", group_field)
        )))
    };
    
    // 对于rule字段，我们需要做特殊处理，合并rule和rule_payload
    let use_combined_rule = group_field == "rule";
    
    // 如果是chains字段，我们需要自定义SQL查询来提取最后一个节点
    let base_sql = if use_combined_rule {
        // 如果是按规则分组，我们合并rule和rule_payload字段
        format!(
            "SELECT rule, rule_payload, COUNT(*) as count, SUM(conn_download) as download, SUM(conn_upload) as upload FROM connections"
        )
    } else if is_chains_field {
        // 对于chains字段，需要提取最后一个节点
        // 注：此处使用SQLite的JSON函数，提取数组的最后一个元素
        // SQLite 3.38.0及以上版本支持使用负索引 $[-1]
        // 但我们使用更安全的方法，先获取数组为JSON文本，然后在Rust中处理
        format!(
            "SELECT 
                chains,
                COUNT(*) as count, 
                SUM(conn_download) as download, 
                SUM(conn_upload) as upload 
             FROM connections"
        )
    } else {
        format!(
            "SELECT {}, COUNT(*) as count, SUM(conn_download) as download, SUM(conn_upload) as upload FROM connections",
            column_name
        )
    };
    
    let (mut sql, params) = build_base_filter(&base_sql, &query);
    
    // 添加分组和排序
    if use_combined_rule {
        sql.push_str(" GROUP BY rule, rule_payload"); // 按两个字段分组
    } else if is_chains_field {
        sql.push_str(" GROUP BY chains"); // 按chains分组
    } else {
        sql.push_str(&format!(" GROUP BY {}", column_name));
    }
    
    if let Some(sort_by) = &query.sort_by {
        let sort_field = match sort_by.as_str() {
            "count" => "count",
            "download" => "download",
            "upload" => "upload",
            "total" => "(download + upload)",
            _ => "count"
        };
        
        let sort_order = match query.sort_order.as_deref() {
            Some("asc") => "ASC",
            _ => "DESC"
        };
        
        sql.push_str(&format!(" ORDER BY {} {}", sort_field, sort_order));
    } else {
        // 默认排序 - 根据分组类型可能想要不同的默认排序
        if let Some(metric) = &query.metric {
            match metric.as_str() {
                "connections" => sql.push_str(" ORDER BY count DESC"),
                "download" => sql.push_str(" ORDER BY download DESC"),
                "upload" => sql.push_str(" ORDER BY upload DESC"),
                "total" => sql.push_str(" ORDER BY (download + upload) DESC"),
                _ => sql.push_str(" ORDER BY count DESC")
            }
        } else {
            sql.push_str(" ORDER BY count DESC");
        }
    }
    
    // 添加限制
    if let Some(limit) = query.limit {
        sql.push_str(&format!(" LIMIT {}", limit));
    }
    
    // 执行查询
    match db.execute_group_query(&sql, &params).await {
        Ok(mut results) => {
            // 对于 rule 分组，我们需要在结果中合并 rule 和 rule_payload
            if use_combined_rule {
                results = results.into_iter().map(|mut item| {
                    if let Some(obj) = item.as_object_mut() {
                        // 获取 rule 和 rule_payload
                        let rule = obj.get("rule").and_then(|r| r.as_str()).unwrap_or("").to_string();
                        let payload = obj.get("rule_payload").and_then(|p| p.as_str()).unwrap_or("").to_string();
                        
                        // 创建组合规则值
                        let combined_rule = if !payload.is_empty() {
                            format!("{} ({})", rule, payload)
                        } else {
                            rule
                        };
                        
                        // 更新 rule 字段并删除 rule_payload
                        obj.insert("rule".to_string(), serde_json::Value::String(combined_rule));
                        obj.remove("rule_payload");
                    }
                    item
                }).collect();
            }
            
            // 如果是chains分组，需要将last_node重命名为node
            if is_chains_field {
                results = results.into_iter().map(|mut item| {
                    if let Some(obj) = item.as_object_mut() {
                        if let Some(chains_value) = obj.remove("chains") {
                            // 从chains中提取最后一个节点
                            let node_value = if let Some(chains_str) = chains_value.as_str() {
                                // 尝试解析JSON数组并获取最后一个元素
                                let clean_chains = chains_str.trim().replace('\'', "\"");
                                if let Ok(array) = serde_json::from_str::<Vec<String>>(&clean_chains) {
                                    if !array.is_empty() {
                                        serde_json::Value::String(array.last().unwrap().clone())
                                    } else {
                                        serde_json::Value::String("unknown".to_string())
                                    }
                                } else {
                                    // 如果解析失败，保留原始值
                                    serde_json::Value::String(chains_str.to_string())
                                }
                            } else {
                                // 如果不是字符串，保留原始值
                                chains_value
                            };
                            
                            obj.insert("node".to_string(), node_value);
                        }
                    }
                    item
                }).collect();
                
                // 对结果按节点名称进行聚合
                let mut node_stats: std::collections::HashMap<String, serde_json::Value> = std::collections::HashMap::new();
                
                for item in results {
                    if let Some(obj) = item.as_object() {
                        if let (Some(node), Some(count), Some(download), Some(upload)) = (
                            obj.get("node").and_then(|v| v.as_str()),
                            obj.get("count").and_then(|v| v.as_i64()),
                            obj.get("download").and_then(|v| v.as_i64()),
                            obj.get("upload").and_then(|v| v.as_i64())
                        ) {
                            let entry = node_stats.entry(node.to_string()).or_insert_with(|| {
                                serde_json::json!({
                                    "node": node,
                                    "count": 0,
                                    "download": 0,
                                    "upload": 0
                                })
                            });
                            
                            if let Some(entry_obj) = entry.as_object_mut() {
                                // 更新计数
                                if let Some(entry_count) = entry_obj.get("count").and_then(|v| v.as_i64()) {
                                    entry_obj["count"] = serde_json::Value::Number(serde_json::Number::from(entry_count + count));
                                }
                                
                                // 更新下载
                                if let Some(entry_download) = entry_obj.get("download").and_then(|v| v.as_i64()) {
                                    entry_obj["download"] = serde_json::Value::Number(serde_json::Number::from(entry_download + download));
                                }
                                
                                // 更新上传
                                if let Some(entry_upload) = entry_obj.get("upload").and_then(|v| v.as_i64()) {
                                    entry_obj["upload"] = serde_json::Value::Number(serde_json::Number::from(entry_upload + upload));
                                }
                            }
                        }
                    }
                }
                
                // 转换回列表并排序
                let mut aggregated_results: Vec<serde_json::Value> = node_stats.values().cloned().collect();
                
                // 根据排序字段排序
                if let Some(sort_by) = &query.sort_by {
                    let sort_field = match sort_by.as_str() {
                        "count" => "count",
                        "download" => "download",
                        "upload" => "upload",
                        "total" => "total", // 这里会使用下面添加的total字段
                        _ => "count"
                    };
                    
                    let sort_order = match query.sort_order.as_deref() {
                        Some("asc") => true,
                        _ => false  // 默认降序
                    };
                    
                    aggregated_results.sort_by(|a, b| {
                        let a_val = a.get(sort_field).and_then(|v| v.as_i64()).unwrap_or(0);
                        let b_val = b.get(sort_field).and_then(|v| v.as_i64()).unwrap_or(0);
                        
                        if sort_order {
                            a_val.cmp(&b_val)  // 升序
                        } else {
                            b_val.cmp(&a_val)  // 降序
                        }
                    });
                }
                
                // 应用limit
                if let Some(limit) = query.limit {
                    if (limit as usize) < aggregated_results.len() {
                        aggregated_results.truncate(limit as usize);
                    }
                }
                
                results = aggregated_results;
            }
            
            // 为每个结果添加 total 字段
            results = results.into_iter().map(|mut item| {
                if let Some(obj) = item.as_object_mut() {
                    let download = obj.get("download").and_then(|d| d.as_i64()).unwrap_or(0);
                    let upload = obj.get("upload").and_then(|u| u.as_i64()).unwrap_or(0);
                    obj.insert("total".to_string(), serde_json::Value::Number(serde_json::Number::from(download + upload)));
                }
                item
            }).collect();
            
            Ok(ApiResponse::success(results))
        },
        Err(e) => {
            eprintln!("分组统计查询错误: {:?}", e);
            Err(warp::reject::custom(ApiError::DatabaseError(
                format!("分组统计查询失败: {}", e)
            )))
        }
    }
}

// 处理时间序列统计请求
async fn handle_stats_timeseries(
    query: StatsQuery,
    db: Arc<crate::db::Database>,
) -> ApiResult {
    // 确保有开始和结束时间
    let from = match &query.from {
        Some(time) => time.clone(),
        None => return Err(warp::reject::custom(ApiError::BadRequest(
            "时间序列查询必须指定 from 参数".to_string()
        )))
    };
    
    let to = match &query.to {
        Some(time) => time.clone(),
        None => return Err(warp::reject::custom(ApiError::BadRequest(
            "时间序列查询必须指定 to 参数".to_string()
        )))
    };
    
    // 获取时间间隔和指标
    let interval = query.interval.as_deref().unwrap_or("day");
    let metric = query.metric.as_deref().unwrap_or("connections");
    
    // 根据选择的时间间隔确定SQL时间格式
    let time_format = match interval {
        "hour" => "%Y-%m-%d %H:00:00",
        "day" => "%Y-%m-%d",
        "week" => "%Y-%W",
        "month" => "%Y-%m",
        _ => return Err(warp::reject::custom(ApiError::BadRequest(
            format!("不支持的时间间隔: {}", interval)
        )))
    };
    
    // 根据选择的指标确定要计算的字段
    let (select_expr, group_expr) = match metric {
        "connections" => ("COUNT(*) as value", ""),
        "download" => ("SUM(conn_download) as value", ""),
        "upload" => ("SUM(conn_upload) as value", ""),
        "total" => ("SUM(conn_download + conn_upload) as value", ""),
        _ => return Err(warp::reject::custom(ApiError::BadRequest(
            format!("不支持的指标: {}", metric)
        )))
    };
    
    // 构建查询SQL
    let base_sql = format!(
        "SELECT strftime('{}', datetime(last_updated)) as time_point, {} FROM connections",
        time_format, select_expr
    );
    
    let (mut sql, mut params) = build_base_filter(&base_sql, &query);
    
    // 确保有时间范围过滤
    if !params.iter().any(|p| p == &from) {
        sql.push_str(" AND last_updated >= ?");
        params.push(from.clone());
    }
    
    if !params.iter().any(|p| p == &to) {
        sql.push_str(" AND last_updated <= ?");
        params.push(to.clone());
    }
    
    // 添加分组
    sql.push_str(&format!(" GROUP BY time_point{}", group_expr));
    sql.push_str(" ORDER BY time_point ASC");
    
    // 执行查询
    match db.execute_timeseries_query(&sql, &params).await {
        Ok(results) => {
            Ok(ApiResponse::success(results))
        },
        Err(e) => {
            eprintln!("时间序列查询错误: {:?}", e);
            Err(warp::reject::custom(ApiError::DatabaseError(
                format!("时间序列查询失败: {}", e)
            )))
        }
    }
}

// 构建基础SQL查询过滤条件
fn build_base_filter(base_sql: &str, query: &StatsQuery) -> (String, Vec<String>) {
    let mut sql = format!("{} WHERE 1=1", base_sql);
    let mut params: Vec<String> = Vec::new();
    
    // 添加时间范围过滤
    if let Some(from) = &query.from {
        sql.push_str(" AND last_updated >= ?");
        params.push(from.clone());
    }
    
    if let Some(to) = &query.to {
        sql.push_str(" AND last_updated <= ?");
        params.push(to.clone());
    }
    
    // 添加其他筛选条件
    if let Some(agent_id) = &query.agent_id {
        sql.push_str(" AND agent_id = ?");
        params.push(agent_id.clone());
    }
    
    if let Some(network) = &query.network {
        sql.push_str(" AND network = ?");
        params.push(network.clone());
    }
    
    if let Some(rule) = &query.rule {
        sql.push_str(" AND rule = ?");
        params.push(rule.clone());
    }
    
    if let Some(process) = &query.process {
        sql.push_str(" AND process LIKE ?");
        params.push(format!("%{}%", process));
    }
    
    if let Some(destination) = &query.destination {
        sql.push_str(" AND destination_ip LIKE ?");
        params.push(format!("%{}%", destination));
    }
    
    if let Some(geoip) = &query.geoip {
        sql.push_str(" AND destination_geoip LIKE ?");
        params.push(format!("%{}%", geoip));
    }
    
    (sql, params)
}

// 处理同步请求
async fn handle_sync(
    sync_package: SyncPackage,
    db: Arc<crate::db::Database>,
) -> ApiResult {
    match db.batch_upsert_records(&sync_package.connections).await {
        Ok(_) => {
            println!(
                "成功从节点 {} 同步了 {} 条记录",
                sync_package.agent_id,
                sync_package.connections.len()
            );
            Ok(ApiResponse::success(json!({
                "message": "数据同步成功",
                "count": sync_package.connections.len(),
            })))
        }
        Err(e) => {
            eprintln!("同步出错: {:?}", e);
            Err(warp::reject::custom(ApiError::DatabaseError(
                format!("数据同步失败: {}", e)
            )))
        }
    }
}

// 处理请求错误
async fn handle_rejection(err: warp::Rejection) -> Result<impl warp::Reply, Infallible> {
    let (code, message) = if err.is_not_found() {
        (StatusCode::NOT_FOUND, "未找到请求的资源".to_string())
    } else if let Some(e) = err.find::<ApiError>() {
        match e {
            ApiError::Unauthorized => 
                (StatusCode::UNAUTHORIZED, "认证失败".to_string()),
            ApiError::BadRequest(msg) => 
                (StatusCode::BAD_REQUEST, msg.clone()),
            ApiError::DatabaseError(msg) => 
                (StatusCode::INTERNAL_SERVER_ERROR, msg.clone()),
            ApiError::NotFound => 
                (StatusCode::NOT_FOUND, "未找到请求的资源".to_string()),
            ApiError::InternalError(msg) => 
                (StatusCode::INTERNAL_SERVER_ERROR, msg.clone()),
        }
    } else {
        eprintln!("未处理的拒绝: {:?}", err);
        (StatusCode::INTERNAL_SERVER_ERROR, "内部服务器错误".to_string())
    };
    
    Ok(warp::reply::with_status(
        warp::reply::json(&json!({
            "status": "error",
            "data": null,
            "message": message,
        })),
        code
    ))
} 