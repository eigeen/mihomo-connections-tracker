use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashSet;
use chrono::{DateTime, Utc};
use std::sync::Arc;
use crate::db::Database;

// 用于在服务器和客户端之间同步的结构
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SyncPackage {
    pub agent_id: String,
    pub connections: Vec<ConnectionRecord>,
    pub timestamp: DateTime<Utc>,
}

// 数据库中存储的连接记录
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ConnectionRecord {
    pub id: String,
    pub download: i64,
    pub upload: i64,
    pub last_updated: String,
    pub start: String,
    pub network: String,
    pub conn_type: String,
    pub source_ip: String,
    pub destination_ip: String,
    pub source_geoip: String,
    pub destination_geoip: String,
    pub source_ip_asn: String,
    pub destination_ip_asn: String,
    pub source_port: String,
    pub destination_port: String,
    pub inbound_ip: String,
    pub inbound_port: String,
    pub inbound_name: String,
    pub inbound_user: String,
    pub host: String,
    pub dns_mode: String,
    pub uid: i64,
    pub process: String,
    pub process_path: String,
    pub special_proxy: String,
    pub special_rules: String,
    pub remote_destination: String,
    pub dscp: i64,
    pub sniff_host: String,
    pub chains: String,
    pub rule: String,
    pub rule_payload: String,
    pub agent_id: Option<String>,
}

// 从 Mihomo API 接收的数据结构
#[derive(Deserialize, Debug, Clone)]
pub struct GlobalData {
    #[serde(deserialize_with = "deserialize_connections")]
    pub connections: Vec<Connection>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct Connection {
    pub id: String,
    pub download: i64,
    pub upload: i64,
    pub start: String, // ISO8601 string
    pub metadata: ConnectionMetadata,
    #[serde(deserialize_with = "deserialize_chains")]
    pub chains: Vec<String>,
    pub rule: String,
    #[serde(rename = "rulePayload")]
    pub rule_payload: String,
}

#[derive(Deserialize, Debug, Clone)]
pub struct ConnectionMetadata {
    pub network: String,
    #[serde(rename = "type")]
    pub conn_type: String,
    #[serde(rename = "sourceIP")]
    pub source_ip: String,
    #[serde(rename = "destinationIP")]
    pub destination_ip: String,
    #[serde(rename = "sourceGeoIP")]
    pub source_geoip: Value,
    #[serde(rename = "destinationGeoIP")]
    pub destination_geoip: Value,
    #[serde(rename = "sourceIPASN")]
    pub source_ip_asn: String,
    #[serde(rename = "destinationIPASN")]
    pub destination_ip_asn: String,
    #[serde(rename = "sourcePort")]
    pub source_port: String,
    #[serde(rename = "destinationPort")]
    pub destination_port: String,
    #[serde(rename = "inboundIP")]
    pub inbound_ip: String,
    #[serde(rename = "inboundPort")]
    pub inbound_port: String,
    #[serde(rename = "inboundName")]
    pub inbound_name: String,
    #[serde(rename = "inboundUser")]
    pub inbound_user: String,
    pub host: String,
    #[serde(rename = "dnsMode")]
    pub dns_mode: String,
    pub uid: i64,
    pub process: String,
    #[serde(rename = "processPath")]
    pub process_path: String,
    #[serde(rename = "specialProxy")]
    pub special_proxy: String,
    #[serde(rename = "specialRules")]
    pub special_rules: String,
    #[serde(rename = "remoteDestination")]
    pub remote_destination: String,
    pub dscp: i64,
    #[serde(rename = "sniffHost")]
    pub sniff_host: String,
}

// 连接跟踪器的状态
#[derive(Debug)]
pub struct ConnectionState {
    pub flow_cache: std::collections::HashMap<String, (i64, i64)>, // id -> (download, upload)
    pub active_connections: HashSet<String>,
}

impl ConnectionState {
    pub fn new() -> Self {
        Self {
            flow_cache: std::collections::HashMap::new(),
            active_connections: HashSet::new(),
        }
    }
}

// 从 Connection 转换为 ConnectionRecord
pub fn connection_to_record(conn: &Connection, agent_id: Option<String>) -> ConnectionRecord {
    let last_updated = Utc::now().to_rfc3339();
    let chains = serde_json::to_string(&conn.chains).unwrap_or_else(|_| "[]".to_string());
    let source_geoip = serde_json::to_string(&conn.metadata.source_geoip)
        .unwrap_or_else(|_| "{}".to_string());
    let destination_geoip = serde_json::to_string(&conn.metadata.destination_geoip)
        .unwrap_or_else(|_| "{}".to_string());

    ConnectionRecord {
        id: conn.id.clone(),
        download: conn.download,
        upload: conn.upload,
        last_updated,
        start: conn.start.clone(),
        network: conn.metadata.network.clone(),
        conn_type: conn.metadata.conn_type.clone(),
        source_ip: conn.metadata.source_ip.clone(),
        destination_ip: conn.metadata.destination_ip.clone(),
        source_geoip,
        destination_geoip,
        source_ip_asn: conn.metadata.source_ip_asn.clone(),
        destination_ip_asn: conn.metadata.destination_ip_asn.clone(),
        source_port: conn.metadata.source_port.clone(),
        destination_port: conn.metadata.destination_port.clone(),
        inbound_ip: conn.metadata.inbound_ip.clone(),
        inbound_port: conn.metadata.inbound_port.clone(),
        inbound_name: conn.metadata.inbound_name.clone(),
        inbound_user: conn.metadata.inbound_user.clone(),
        host: conn.metadata.host.clone(),
        dns_mode: conn.metadata.dns_mode.clone(),
        uid: conn.metadata.uid,
        process: conn.metadata.process.clone(),
        process_path: conn.metadata.process_path.clone(),
        special_proxy: conn.metadata.special_proxy.clone(),
        special_rules: conn.metadata.special_rules.clone(),
        remote_destination: conn.metadata.remote_destination.clone(),
        dscp: conn.metadata.dscp,
        sniff_host: conn.metadata.sniff_host.clone(),
        chains,
        rule: conn.rule.clone(),
        rule_payload: conn.rule_payload.clone(),
        agent_id,
    }
}

pub fn deserialize_connections<'de, D>(deserializer: D) -> Result<Vec<Connection>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let opt = Option::<Vec<Connection>>::deserialize(deserializer)?;
    Ok(opt.unwrap_or_default())
}

pub fn deserialize_chains<'de, D>(deserializer: D) -> Result<Vec<String>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let mut chains: Vec<String> = Vec::deserialize(deserializer)?;
    chains.reverse();
    Ok(chains)
}

// 格式化流量大小为人类可读格式
pub fn format_bytes(bytes: i64) -> String {
    const KB: i64 = 1024;
    const MB: i64 = KB * 1024;
    const GB: i64 = MB * 1024;
    
    if bytes >= GB {
        format!("{:.2} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.2} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.2} KB", bytes as f64 / KB as f64)
    } else {
        format!("{} B", bytes)
    }
}

// 处理连接更新的公共函数 - 主从节点都可以调用
pub fn process_connections(
    data: &GlobalData,
    state_lock: &mut ConnectionState,
    db: Arc<Database>,
    agent_id: Option<String>
) -> Result<(), String> {
    // 获取当前连接ID集合
    let current_conn_ids: std::collections::HashSet<_> = data.connections.iter()
        .map(|c| c.id.clone())
        .collect();
    
    // 第一步：检测并显示新连接 - 这些需要插入数据库
    let new_conns: Vec<_> = current_conn_ids
        .difference(&state_lock.active_connections)
        .cloned()
        .collect();
    
    if !new_conns.is_empty() {
        println!("新连接 ({}):", new_conns.len());
        for conn_id in &new_conns {
            if let Some(conn) = data.connections.iter().find(|c| &c.id == conn_id) {
                println!(
                    "  - {} {}:{} -> {}:{} (规则: {}; {}, 链路: {})",
                    conn.metadata.network,
                    conn.metadata.source_ip,
                    conn.metadata.source_port,
                    conn.metadata.destination_ip,
                    conn.metadata.destination_port,
                    conn.rule,
                    conn.rule_payload,
                    conn.chains.join("->")
                );
            }
        }
    }
    
    // 第二步：检测并显示关闭的连接
    let closed_conns: Vec<_> = state_lock.active_connections
        .difference(&current_conn_ids)
        .cloned()
        .collect();
    
    if !closed_conns.is_empty() {
        println!("关闭连接 ({}):", closed_conns.len());
        for conn_id in &closed_conns {
            if let Some(conn_info) = state_lock.flow_cache.get(conn_id) {
                let download = conn_info.0;
                let upload = conn_info.1;
                println!(
                    "  - {} [↑: {}, ↓: {}, 总计: {}]",
                    conn_id,
                    format_bytes(upload),
                    format_bytes(download),
                    format_bytes(upload + download)
                );
            } else {
                println!("  - {}", conn_id);
            }
            // 从缓存中删除已关闭的连接信息
            state_lock.flow_cache.remove(conn_id);
        }
    }
    
    // 第三步：检测流量变化 - 这些需要更新数据库
    // 收集需要更新数据库的连接
    let mut connections_to_update = Vec::new();
    // 收集仅流量有变化的连接（用于显示）
    let mut flow_changed_connections = Vec::new();
    
    for conn in &data.connections {
        // 检查是否是新连接
        let is_new_connection = new_conns.contains(&conn.id);
        
        // 检查流量是否有变化
        let has_flow_change = if let Some((old_download, old_upload)) = state_lock.flow_cache.get(&conn.id) {
            *old_upload != conn.upload || *old_download != conn.download
        } else {
            // 如果缓存中没有，也视为需要更新
            true
        };
        
        // 如果是新连接或有流量变化，需要更新数据库
        if is_new_connection || has_flow_change {
            connections_to_update.push(conn);
        }
        
        // 如果只是流量变化（非新连接），添加到流量变化列表
        if has_flow_change && !is_new_connection {
            flow_changed_connections.push((
                conn.metadata.network.clone(),
                format!("{}:{}", conn.metadata.source_ip, conn.metadata.source_port),
                format!("{}:{}", conn.metadata.destination_ip, conn.metadata.destination_port),
                conn.upload,
                conn.download
            ));
        }
        
        // 更新流量缓存
        state_lock.flow_cache.insert(conn.id.clone(), (conn.download, conn.upload));
    }
    
    // 第四步：显示流量变化的连接
    if !flow_changed_connections.is_empty() {
        println!("连接流量更新 ({}):", flow_changed_connections.len());
        for (network, source, destination, upload, download) in &flow_changed_connections {
            let total = upload + download;
            println!(
                "  - {} {} -> {} [↑: {}, ↓: {}, 总计: {}]",
                network,
                source,
                destination,
                format_bytes(*upload),
                format_bytes(*download),
                format_bytes(total)
            );
        }
    }
    
    // 最后：更新当前状态
    state_lock.active_connections = current_conn_ids;
    
    // 只有当有变化的连接时才更新数据库
    if !connections_to_update.is_empty() {
        // 在后台任务中处理数据库更新
        let db_clone = db.clone();
        let agent_id_clone = agent_id.clone();
        
        // 将需要更新的连接克隆一份
        let connections_to_update: Vec<Connection> = connections_to_update.iter()
            .map(|&conn| conn.clone())
            .collect();
        
        // 立即执行数据库更新
        tokio::spawn(async move {
            // 准备要更新的记录
            let records: Vec<_> = connections_to_update.iter()
                .map(|conn| connection_to_record(conn, agent_id_clone.clone()))
                .collect();
            
            // 执行批量更新
            if let Err(e) = db_clone.batch_upsert_records(&records).await {
                eprintln!("批量更新连接数据错误: {}", e);
            } else {
                println!("更新了 {} 个连接记录", records.len());
            }
        });
    }
    
    Ok(())
} 