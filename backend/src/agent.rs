use std::error::Error;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::signal;
use tokio::time::interval;
use uuid::Uuid;
use chrono::Utc;
use crate::api::{MihomoClient, MasterClient};
use crate::config::AgentConfig;
use crate::common::{ConnectionState, SyncPackage, process_connections};
use crate::db::Database;

// 从节点状态
struct AgentState {
    db: Arc<Database>,
    conn_state: Arc<Mutex<ConnectionState>>,
    master_client: Arc<MasterClient>,
    agent_id: String,
}

// 运行从节点客户端
pub async fn run(config: AgentConfig) -> Result<(), Box<dyn Error>> {
    println!("初始化从节点数据库...");
    let database = Arc::new(Database::new(&config.local_database).await?);
    
    // 生成或使用提供的节点ID
    let agent_id = config.agent_id.unwrap_or_else(|| Uuid::new_v4().to_string());
    
    println!("配置信息:");
    println!("  - 节点ID: {}", agent_id);
    println!("  - Mihomo API: {}:{}", config.mihomo_host, config.mihomo_port);
    println!("  - 本地数据库: {}", config.local_database);
    if let Some(master_url) = &config.master_url {
        println!("  - 主节点URL: {}", master_url);
        println!("  - 主节点认证: {}", config.master_token.as_ref().map_or("未启用", |_| "已启用"));
    } else {
        println!("  - 离线模式: 数据仅存储在本地");
    }
    println!("  - 同步间隔: {}秒", config.sync_interval);
    println!("  - 数据保留天数: {}天", config.data_retention_days);
    
    // 创建MihomoAPI客户端
    let mihomo_client = MihomoClient::new(
        config.mihomo_host.clone(), 
        config.mihomo_port,
        config.mihomo_token.clone()
    );
    
    // 创建从节点状态
    let conn_state = Arc::new(Mutex::new(ConnectionState::new()));
    
    // 创建主节点API客户端（如果配置了主节点）
    let master_client = if let Some(master_url) = &config.master_url {
        Arc::new(MasterClient::new(
            master_url.clone(),
            config.master_token.clone()
        ))
    } else {
        Arc::new(MasterClient::new(
            "http://no-master-configured".to_string(),
            None
        ))
    };
    
    // 创建从节点状态
    let state = Arc::new(AgentState {
        db: database.clone(),
        conn_state: conn_state.clone(),
        master_client: master_client.clone(),
        agent_id: agent_id.clone(),
    });
    
    // 启动同步任务（如果配置了主节点）
    if let Some(_) = &config.master_url {
        let sync_state = state.clone();
        let sync_interval = config.sync_interval;
        let retention_days = config.data_retention_days;
        let _handle = tokio::spawn(async move {
            if let Err(e) = sync_task(sync_state, sync_interval, retention_days).await {
                eprintln!("同步任务错误: {}", e);
            }
        });
    }
    
    println!("从节点客户端已启动");
    println!("按Ctrl+C关闭客户端");
    
    // 启动WebSocket客户端并等待中断信号
    tokio::select! {
        // 启动连接处理
        result = mihomo_client.connect(move |data| {
            // 同步处理连接数据
            let state_ref = &state;
            let db = &state_ref.db;
            let agent_id = Some(state_ref.agent_id.clone());
            
            // 获取并锁定当前状态
            let mut state_lock = match state_ref.conn_state.lock() {
                Ok(lock) => lock,
                Err(_) => return Err("无法获取状态锁".to_string()),
            };
            
            // 使用公共函数处理连接更新
            process_connections(&data, &mut state_lock, db.clone(), agent_id)
        }) => {
            if let Err(e) = result {
                eprintln!("客户端错误: {:?}", e);
                return Err(e);
            }
            Ok(())
        }
        // 监听中断信号
        _ = signal::ctrl_c() => {
            println!("收到关闭信号，正在关闭客户端...");
            Ok(())
        }
    }
}

// 同步任务 - 定期将本地数据同步到主节点
async fn sync_task(state: Arc<AgentState>, interval_secs: u64, retention_days: i64) -> Result<(), Box<dyn Error + Send + Sync>> {
    let mut interval = interval(Duration::from_secs(interval_secs));
    
    loop {
        interval.tick().await;
        
        // 检查主节点是否在线
        let master_online = state.master_client.is_online().await;
        if !master_online {
            println!("主节点离线，跳过同步...");
            continue;
        }
        
        // 获取待同步记录数量
        match state.db.count_pending_records().await {
            Ok(count) => {
                if count == 0 {
                    println!("没有需要同步的记录");
                    
                    // 即使没有新记录需要同步，也可以执行清理任务
                    match state.db.cleanup_old_records(retention_days).await {
                        Ok(deleted) => {
                            if deleted > 0 {
                                println!("已清理 {} 条超过{}天的旧数据", deleted, retention_days);
                            }
                        },
                        Err(e) => {
                            eprintln!("清理旧数据失败: {:?}", e);
                        }
                    }
                    
                    continue;
                }
                
                println!("发现 {} 条待同步记录", count);
                
                // 每次同步最多1000条记录
                const BATCH_SIZE: i64 = 1000;
                let batches = (count + BATCH_SIZE - 1) / BATCH_SIZE;
                
                for i in 0..batches {
                    match state.db.get_pending_records(BATCH_SIZE).await {
                        Ok(records) => {
                            if records.is_empty() {
                                break;
                            }
                            
                            println!("同步批次 {}/{}，包含 {} 条记录", 
                                i + 1, batches, records.len());
                            
                            // 创建同步包
                            let sync_package = SyncPackage {
                                agent_id: state.agent_id.clone(),
                                connections: records,
                                timestamp: Utc::now(),
                            };
                            
                            // 发送到主节点
                            match state.master_client.sync_data(&sync_package).await {
                                Ok(_) => {
                                    println!("成功同步批次 {}/{}", i + 1, batches);
                                    
                                    // 更新同步状态
                                    let now = Utc::now().to_rfc3339();
                                    if let Err(e) = state.db.update_sync_state(&now, 0).await {
                                        eprintln!("更新同步状态失败: {:?}", e);
                                    }
                                },
                                Err(e) => {
                                    eprintln!("同步批次 {}/{} 失败: {:?}", i + 1, batches, e);
                                    break;
                                }
                            }
                        },
                        Err(e) => {
                            eprintln!("获取待同步记录失败: {:?}", e);
                            break;
                        }
                    }
                }
                
                // 在所有数据同步完成后，清理旧数据
                match state.db.cleanup_old_records(retention_days).await {
                    Ok(deleted) => {
                        if deleted > 0 {
                            println!("已清理 {} 条超过{}天的旧数据", deleted, retention_days);
                        }
                    },
                    Err(e) => {
                        eprintln!("清理旧数据失败: {:?}", e);
                    }
                }
            },
            Err(e) => {
                eprintln!("检查待同步记录数量失败: {:?}", e);
            }
        }
    }
    
    // 这句代码在实际中永远不会执行，因为循环内没有break，但需要返回正确类型
    #[allow(unreachable_code)]
    Ok(())
} 