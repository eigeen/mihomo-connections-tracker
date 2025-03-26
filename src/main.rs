use std::collections::{HashMap, HashSet};
use std::time::Duration;

use chrono::Utc;
use clap::Parser;
use futures_util::StreamExt;
use serde::Deserialize;
use serde_json::{Value, to_string as json_to_string, from_str as json_from_str};
use sqlx::{
    Error as SqlxError, Executor, Sqlite, SqlitePool, query as sqlx_query,
    sqlite::SqliteConnectOptions,
};
use tokio::time::timeout;
use tokio_tungstenite::connect_async;

const DATABASE_URL: &str = "connections.db";

/// 命令行参数配置
#[derive(Parser, Debug)]
#[clap(version = "1.0", author = "Your Name")]
struct Args {
    /// API服务器主机地址
    #[clap(long, default_value = "127.0.0.1")]
    host: String,

    /// API服务器端口号
    #[clap(long, default_value = "9090")]
    port: u16,

    /// 认证令牌
    #[clap(long, default_value = "")]
    token: String,
}

/// Structs to deserialize the incoming JSON.
#[derive(Deserialize, Debug)]
struct GlobalData {
    connections: Vec<Connection>,
}

#[derive(Deserialize, Debug)]
struct Connection {
    id: String,
    download: i64,
    upload: i64,
    start: String, // ISO8601 string
    metadata: ConnectionMetadata,
    chains: Vec<String>,
    rule: String,
    #[serde(rename = "rulePayload")]
    rule_payload: String,
}

#[derive(Deserialize, Debug)]
struct ConnectionMetadata {
    network: String,
    #[serde(rename = "type")]
    conn_type: String,
    #[serde(rename = "sourceIP")]
    source_ip: String,
    #[serde(rename = "destinationIP")]
    destination_ip: String,
    #[serde(rename = "sourceGeoIP")]
    source_geoip: Value,
    #[serde(rename = "destinationGeoIP")]
    destination_geoip: Value,
    #[serde(rename = "sourceIPASN")]
    source_ip_asn: String,
    #[serde(rename = "destinationIPASN")]
    destination_ip_asn: String,
    #[serde(rename = "sourcePort")]
    source_port: String,
    #[serde(rename = "destinationPort")]
    destination_port: String,
    #[serde(rename = "inboundIP")]
    inbound_ip: String,
    #[serde(rename = "inboundPort")]
    inbound_port: String,
    #[serde(rename = "inboundName")]
    inbound_name: String,
    #[serde(rename = "inboundUser")]
    inbound_user: String,
    host: String,
    #[serde(rename = "dnsMode")]
    dns_mode: String,
    uid: i64,
    process: String,
    #[serde(rename = "processPath")]
    process_path: String,
    #[serde(rename = "specialProxy")]
    special_proxy: String,
    #[serde(rename = "specialRules")]
    special_rules: String,
    #[serde(rename = "remoteDestination")]
    remote_destination: String,
    dscp: i64,
    #[serde(rename = "sniffHost")]
    sniff_host: String,
}

/// ConnectionTracker handles maintaining a cache of flows,
/// the set of active connections, and updating the database.
struct ConnectionTracker {
    pool: SqlitePool,
    flow_cache: HashMap<String, (i64, i64)>, // id -> (download, upload)
    active_connections: HashSet<String>,
    running: bool,
}

impl ConnectionTracker {
    /// Initialize the database and caches.
    async fn new() -> Result<Self, SqlxError> {
        let options = SqliteConnectOptions::new()
            .filename(DATABASE_URL)
            .create_if_missing(true);
        let pool = SqlitePool::connect_with(options).await?;

        // Create the connections table if it does not exist.
        pool.execute(
            r#"
            CREATE TABLE IF NOT EXISTS connections (
                id TEXT PRIMARY KEY,
                conn_download INTEGER,
                conn_upload INTEGER,
                last_updated TEXT,
                start TEXT,
                network TEXT,
                type TEXT,
                source_ip TEXT,
                destination_ip TEXT,
                source_geoip TEXT,
                destination_geoip TEXT,
                source_ip_asn TEXT,
                destination_ip_asn TEXT,
                source_port TEXT,
                destination_port TEXT,
                inbound_ip TEXT,
                inbound_port TEXT,
                inbound_name TEXT,
                inbound_user TEXT,
                host TEXT,
                dns_mode TEXT,
                uid INTEGER,
                process TEXT,
                process_path TEXT,
                special_proxy TEXT,
                special_rules TEXT,
                remote_destination TEXT,
                dscp INTEGER,
                sniff_host TEXT,
                chains TEXT,
                rule TEXT,
                rule_payload TEXT
            )
            "#,
        )
        .await?;

        Ok(Self {
            pool,
            flow_cache: HashMap::new(),
            active_connections: HashSet::new(),
            running: true,
        })
    }

    /// Close the connection tracker.
    async fn close(&mut self) {
        self.running = false;
        // The pool will be closed when it is dropped.
        println!("Tracker closed.");
    }

    /// Upsert a single connection record into the database.
    async fn upsert_connection<'a, E>(&self, exec: E, conn: &Connection) -> Result<(), SqlxError>
    where
        E: Executor<'a, Database = Sqlite>,
    {
        let last_updated = Utc::now().to_rfc3339();
        let start = conn.start.clone();
        let chains = conn.chains.join("->");
        let source_geoip =
            json_to_string(&conn.metadata.source_geoip).unwrap_or_else(|_| "{}".to_string());
        let destination_geoip =
            json_to_string(&conn.metadata.destination_geoip).unwrap_or_else(|_| "{}".to_string());

        sqlx_query(
            r#"
            INSERT INTO connections (
                id, conn_download, conn_upload, last_updated, start, network, type,
                source_ip, destination_ip, source_geoip, destination_geoip, source_ip_asn,
                destination_ip_asn, source_port, destination_port, inbound_ip, inbound_port,
                inbound_name, inbound_user, host, dns_mode, uid, process, process_path,
                special_proxy, special_rules, remote_destination, dscp, sniff_host, chains,
                rule, rule_payload
            )
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            ON CONFLICT(id) DO UPDATE SET
                conn_download=excluded.conn_download,
                conn_upload=excluded.conn_upload,
                last_updated=excluded.last_updated,
                start=excluded.start,
                network=excluded.network,
                type=excluded.type,
                source_ip=excluded.source_ip,
                destination_ip=excluded.destination_ip,
                source_geoip=excluded.source_geoip,
                destination_geoip=excluded.destination_geoip,
                source_ip_asn=excluded.source_ip_asn,
                destination_ip_asn=excluded.destination_ip_asn,
                source_port=excluded.source_port,
                destination_port=excluded.destination_port,
                inbound_ip=excluded.inbound_ip,
                inbound_port=excluded.inbound_port,
                inbound_name=excluded.inbound_name,
                inbound_user=excluded.inbound_user,
                host=excluded.host,
                dns_mode=excluded.dns_mode,
                uid=excluded.uid,
                process=excluded.process,
                process_path=excluded.process_path,
                special_proxy=excluded.special_proxy,
                special_rules=excluded.special_rules,
                remote_destination=excluded.remote_destination,
                dscp=excluded.dscp,
                sniff_host=excluded.sniff_host,
                chains=excluded.chains,
                rule=excluded.rule,
                rule_payload=excluded.rule_payload
            "#,
        )
        .bind(&conn.id)
        .bind(conn.download)
        .bind(conn.upload)
        .bind(last_updated)
        .bind(start)
        .bind(&conn.metadata.network)
        .bind(&conn.metadata.conn_type)
        .bind(&conn.metadata.source_ip)
        .bind(&conn.metadata.destination_ip)
        .bind(source_geoip)
        .bind(destination_geoip)
        .bind(&conn.metadata.source_ip_asn)
        .bind(&conn.metadata.destination_ip_asn)
        .bind(&conn.metadata.source_port)
        .bind(&conn.metadata.destination_port)
        .bind(&conn.metadata.inbound_ip)
        .bind(&conn.metadata.inbound_port)
        .bind(&conn.metadata.inbound_name)
        .bind(&conn.metadata.inbound_user)
        .bind(&conn.metadata.host)
        .bind(&conn.metadata.dns_mode)
        .bind(conn.metadata.uid)
        .bind(&conn.metadata.process)
        .bind(&conn.metadata.process_path)
        .bind(&conn.metadata.special_proxy)
        .bind(&conn.metadata.special_rules)
        .bind(&conn.metadata.remote_destination)
        .bind(conn.metadata.dscp)
        .bind(&conn.metadata.sniff_host)
        .bind(chains)
        .bind(&conn.rule)
        .bind(&conn.rule_payload)
        .execute(exec)
        .await?;
        Ok(())
    }

    /// Update connections based on new data.
    async fn update_connections(
        &mut self,
        data: GlobalData,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let now = Utc::now();
        let current_conn_ids: HashSet<_> = data.connections.iter().map(|c| c.id.clone()).collect();

        // Detect new connections.
        let new_conns: Vec<_> = current_conn_ids
            .difference(&self.active_connections)
            .cloned()
            .collect();

        if !new_conns.is_empty() {
            println!("New connections ({}):", new_conns.len());
            for conn_id in &new_conns {
                if let Some(conn) = data.connections.iter().find(|c| &c.id == conn_id) {
                    println!(
                        "  - {} {}:{} -> {}:{} (rule: {}; {}, chains: {})",
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

        // Detect closed connections.
        let closed_conns: Vec<_> = self
            .active_connections
            .difference(&current_conn_ids)
            .cloned()
            .collect();
        if !closed_conns.is_empty() {
            println!("Closed connections ({}):", closed_conns.len());
            for conn_id in &closed_conns {
                if let Some(&(download, upload)) = self.flow_cache.get(conn_id) {
                    println!(
                        "  - ID: {} (Download: {}, Upload: {})",
                        conn_id, download, upload
                    );
                }
                self.flow_cache.remove(conn_id);
            }
        }
        self.active_connections = current_conn_ids;

        // Detect flow changes.
        let mut changed_conns = Vec::new();
        for conn in &data.connections {
            let current_flow = (conn.download, conn.upload);
            if self
                .flow_cache
                .get(&conn.id)
                .map(|&flow| flow != current_flow)
                .unwrap_or(true)
            {
                changed_conns.push(conn);
                self.flow_cache.insert(conn.id.clone(), current_flow);
            }
        }

        // Upsert changed connections into the database.
        if !changed_conns.is_empty() {
            let mut tx = self.pool.begin().await?;
            for conn in &changed_conns {
                // 倒序 conn.chains

                self.upsert_connection(&mut tx, conn).await?;
            }
            tx.commit().await?;
            println!("Updated {} connections at {}", changed_conns.len(), now);
        } else {
            println!("No connection changes at {}", now);
        }
        Ok(())
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    let mut tracker = ConnectionTracker::new().await?;
    let ws_uri = format!(
        "ws://{}:{}/connections?token={}",
        args.host, args.port, args.token
    );

    println!("Connecting to mihomo API at {}:{}...", args.host, args.port);
    println!("Press Ctrl+C to shut down gracefully.");

    // Main loop: connect to the websocket and process incoming messages.
    loop {
        tokio::select! {
            _ = tokio::signal::ctrl_c() => {
                println!("Received shutdown signal...");
                tracker.close().await;
                break;
            },
            ws_result = connect_async(&ws_uri) => {
                match ws_result {
                    Ok((ws_stream, _)) => {
                        println!("Connected to mihomo API");
                        let (mut _write, mut read) = ws_stream.split();
                        // Process messages until a shutdown signal or connection error.
                        loop {
                            tokio::select! {
                                _ = tokio::signal::ctrl_c() => {
                                    println!("Received shutdown signal...");
                                    tracker.close().await;
                                    break;
                                },
                                msg = timeout(Duration::from_millis(200), read.next()) => {
                                    match msg {
                                        Ok(Some(Ok(message))) => {
                                            if message.is_text() {
                                                let text = message.into_text()?;
                                                let data: GlobalData = json_from_str(&text)?;
                                                tracker.update_connections(data).await?;
                                            }
                                        },
                                        Ok(Some(Err(e))) => {
                                            println!("Error processing message: {:?}", e);
                                            break;
                                        },
                                        Ok(_) => {
                                            println!("Connection closed, reconnecting...");
                                            break;
                                        },
                                        Err(_) => {
                                            // Timeout expired; simply continue the loop.
                                            continue;
                                        }
                                    }
                                }
                            }
                            if !tracker.running {
                                break;
                            }
                        }
                    },
                    Err(e) => {
                        println!("Connection error: {:?}, retrying...", e);
                        tokio::time::sleep(Duration::from_millis(200)).await;
                    }
                }
            }
        }
        if !tracker.running {
            break;
        }
    }
    println!("Shutting down gracefully...");
    tracker.close().await;
    println!("Program exited.");
    Ok(())
}
