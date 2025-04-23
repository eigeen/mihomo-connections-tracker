use crate::common::ConnectionRecord;
use sqlx::{
    Error as SqlxError, Executor, Sqlite, SqlitePool, query as sqlx_query,
    sqlite::SqliteConnectOptions, Row,
};

// 数据库连接池
pub struct Database {
    pub pool: SqlitePool,
}

impl Database {
    // 初始化数据库
    pub async fn new(database_path: &str) -> Result<Self, SqlxError> {
        let options = SqliteConnectOptions::new()
            .filename(database_path)
            .create_if_missing(true);
        let pool = SqlitePool::connect_with(options).await?;

        // 创建连接表
        pool.execute(
            r#"
            CREATE TABLE IF NOT EXISTS connections (
                id TEXT,
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
                rule_payload TEXT,
                agent_id TEXT,
                PRIMARY KEY (id, agent_id)
            )
            "#,
        )
        .await?;

        // 创建同步状态表（用于从机）
        pool.execute(
            r#"
            CREATE TABLE IF NOT EXISTS sync_state (
                id INTEGER PRIMARY KEY,
                last_sync_time TEXT,
                pending_records INTEGER,
                last_connection_id TEXT
            )
            "#,
        )
        .await?;

        // 初始化同步状态（如果尚未有记录）
        pool.execute(
            r#"
            INSERT OR IGNORE INTO sync_state (id, last_sync_time, pending_records, last_connection_id)
            VALUES (1, datetime('now'), 0, '')
            "#,
        )
        .await?;

        Ok(Self { pool })
    }

    // 查询尚未同步的记录数量
    pub async fn count_pending_records(&self) -> Result<i64, SqlxError> {
        // 使用简单字符串查询
        let row = sqlx::query("SELECT COUNT(*) as count FROM connections WHERE last_updated > (SELECT last_sync_time FROM sync_state WHERE id = 1)")
            .fetch_one(&self.pool)
            .await?;
            
        let count: i64 = row.get(0);
        Ok(count)
    }

    // 更新同步状态
    pub async fn update_sync_state(&self, time: &str, pending: i64) -> Result<(), SqlxError> {
        sqlx::query("UPDATE sync_state SET last_sync_time = ?, pending_records = ? WHERE id = 1")
            .bind(time)
            .bind(pending)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    // 从数据库中获取待同步的记录
    pub async fn get_pending_records(&self, limit: i64) -> Result<Vec<ConnectionRecord>, SqlxError> {
        let rows = sqlx::query(
            r#"
            SELECT 
                id, conn_download, conn_upload, last_updated, start, 
                network, type as conn_type, source_ip, destination_ip, 
                source_geoip, destination_geoip, source_ip_asn, destination_ip_asn, 
                source_port, destination_port, inbound_ip, inbound_port, 
                inbound_name, inbound_user, host, dns_mode, uid, process, 
                process_path, special_proxy, special_rules, remote_destination, 
                dscp, sniff_host, chains, rule, rule_payload, agent_id
            FROM connections
            WHERE last_updated > (SELECT last_sync_time FROM sync_state WHERE id = 1)
            ORDER BY last_updated ASC
            LIMIT ?
            "#
        )
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;

        let mut records = Vec::new();
        for row in rows {
            let record = ConnectionRecord {
                id: row.get("id"),
                download: row.get("conn_download"),
                upload: row.get("conn_upload"),
                last_updated: row.get("last_updated"),
                start: row.get("start"),
                network: row.get("network"),
                conn_type: row.get("conn_type"),
                source_ip: row.get("source_ip"),
                destination_ip: row.get("destination_ip"),
                source_geoip: row.get("source_geoip"),
                destination_geoip: row.get("destination_geoip"),
                source_ip_asn: row.get("source_ip_asn"),
                destination_ip_asn: row.get("destination_ip_asn"),
                source_port: row.get("source_port"),
                destination_port: row.get("destination_port"),
                inbound_ip: row.get("inbound_ip"),
                inbound_port: row.get("inbound_port"),
                inbound_name: row.get("inbound_name"),
                inbound_user: row.get("inbound_user"),
                host: row.get("host"),
                dns_mode: row.get("dns_mode"),
                uid: row.get("uid"),
                process: row.get("process"),
                process_path: row.get("process_path"),
                special_proxy: row.get("special_proxy"),
                special_rules: row.get("special_rules"),
                remote_destination: row.get("remote_destination"),
                dscp: row.get("dscp"),
                sniff_host: row.get("sniff_host"),
                chains: row.get("chains"),
                rule: row.get("rule"),
                rule_payload: row.get("rule_payload"),
                agent_id: row.get("agent_id"),
            };
            records.push(record);
        }

        Ok(records)
    }

    // 将连接记录保存到数据库中
    pub async fn upsert_connection_record<'a, E>(&self, exec: E, record: &ConnectionRecord) -> Result<(), SqlxError>
    where
        E: Executor<'a, Database = Sqlite>,
    {
        sqlx_query(
            r#"
            INSERT INTO connections (
                id, conn_download, conn_upload, last_updated, start, network, type,
                source_ip, destination_ip, source_geoip, destination_geoip, source_ip_asn,
                destination_ip_asn, source_port, destination_port, inbound_ip, inbound_port,
                inbound_name, inbound_user, host, dns_mode, uid, process, process_path,
                special_proxy, special_rules, remote_destination, dscp, sniff_host, chains,
                rule, rule_payload, agent_id
            )
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            ON CONFLICT(id, agent_id) DO UPDATE SET
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
        .bind(&record.id)
        .bind(record.download)
        .bind(record.upload)
        .bind(&record.last_updated)
        .bind(&record.start)
        .bind(&record.network)
        .bind(&record.conn_type)
        .bind(&record.source_ip)
        .bind(&record.destination_ip)
        .bind(&record.source_geoip)
        .bind(&record.destination_geoip)
        .bind(&record.source_ip_asn)
        .bind(&record.destination_ip_asn)
        .bind(&record.source_port)
        .bind(&record.destination_port)
        .bind(&record.inbound_ip)
        .bind(&record.inbound_port)
        .bind(&record.inbound_name)
        .bind(&record.inbound_user)
        .bind(&record.host)
        .bind(&record.dns_mode)
        .bind(record.uid)
        .bind(&record.process)
        .bind(&record.process_path)
        .bind(&record.special_proxy)
        .bind(&record.special_rules)
        .bind(&record.remote_destination)
        .bind(record.dscp)
        .bind(&record.sniff_host)
        .bind(&record.chains)
        .bind(&record.rule)
        .bind(&record.rule_payload)
        .bind(&record.agent_id)
        .execute(exec)
        .await?;
        Ok(())
    }

    // 批量插入连接记录
    pub async fn batch_upsert_records(&self, records: &[ConnectionRecord]) -> Result<(), SqlxError> {
        let mut tx = self.pool.begin().await?;
        for record in records {
            self.upsert_connection_record(&mut *tx, record).await?;
        }
        tx.commit().await?;
        Ok(())
    }
    
    // 执行计数查询
    pub async fn execute_count_query(&self, sql: &str, params: &[String]) -> Result<i64, SqlxError> {
        let mut query = sqlx::query(sql);
        
        // 绑定参数
        for param in params {
            query = query.bind(param);
        }
        
        // 执行查询
        let row = query.fetch_one(&self.pool).await?;
        let count: i64 = row.get(0);
        
        Ok(count)
    }
    
    // 执行流量查询
    pub async fn execute_traffic_query(&self, sql: &str, params: &[String]) -> Result<(i64, i64), SqlxError> {
        let mut query = sqlx::query(sql);
        
        // 绑定参数
        for param in params {
            query = query.bind(param);
        }
        
        // 执行查询
        let row = query.fetch_one(&self.pool).await?;
        
        // 获取下载和上传流量，如果为空则返回0
        let download: Option<i64> = row.try_get(0).unwrap_or(Some(0));
        let upload: Option<i64> = row.try_get(1).unwrap_or(Some(0));
        
        Ok((download.unwrap_or(0), upload.unwrap_or(0)))
    }
    
    // 执行分组查询
    pub async fn execute_group_query(&self, sql: &str, params: &[String]) -> Result<Vec<serde_json::Value>, SqlxError> {
        let mut query = sqlx::query(sql);
        
        // 绑定参数
        for param in params {
            query = query.bind(param);
        }
        
        // 执行查询
        let rows = query.fetch_all(&self.pool).await?;
        
        // 转换结果为JSON数组
        let mut results = Vec::new();
        for row in rows {
            let group_key: String = row.get(0);
            let count: i64 = row.get(1);
            let download: Option<i64> = row.try_get(2).unwrap_or(Some(0));
            let upload: Option<i64> = row.try_get(3).unwrap_or(Some(0));
            
            results.push(serde_json::json!({
                "key": group_key,
                "count": count,
                "download": download.unwrap_or(0),
                "upload": upload.unwrap_or(0),
                "total": download.unwrap_or(0) + upload.unwrap_or(0)
            }));
        }
        
        Ok(results)
    }
    
    // 执行时间序列查询
    pub async fn execute_timeseries_query(&self, sql: &str, params: &[String]) -> Result<Vec<serde_json::Value>, SqlxError> {
        let mut query = sqlx::query(sql);
        
        // 绑定参数
        for param in params {
            query = query.bind(param);
        }
        
        // 执行查询
        let rows = query.fetch_all(&self.pool).await?;
        
        // 转换结果为JSON数组
        let mut results = Vec::new();
        for row in rows {
            let time_point: String = row.get(0);
            let value: i64 = row.get(1);
            
            results.push(serde_json::json!({
                "time": time_point,
                "value": value
            }));
        }
        
        Ok(results)
    }
} 