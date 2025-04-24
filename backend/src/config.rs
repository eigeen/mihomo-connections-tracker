use clap::Args;

// 主节点配置
#[derive(Args, Debug)]
pub struct MasterConfig {
    /// 数据库路径
    #[clap(long, default_value = "master.db")]
    pub database: String,

    /// 监听地址
    #[clap(long, default_value = "0.0.0.0")]
    pub listen_host: String,

    /// 监听端口
    #[clap(long, default_value = "8080")]
    pub listen_port: u16,

    /// API 认证令牌
    #[clap(long)]
    pub api_token: Option<String>,
    
    /// Mihomo API服务器主机地址（可选，用于主节点独立运行）
    #[clap(long)]
    pub mihomo_host: Option<String>,

    /// Mihomo API服务器端口号（可选，用于主节点独立运行）
    #[clap(long)]
    pub mihomo_port: Option<u16>,

    /// Mihomo API认证令牌（可选，用于主节点独立运行）
    #[clap(long)]
    pub mihomo_token: Option<String>,
}

// 从节点配置
#[derive(Args, Debug)]
pub struct AgentConfig {
    /// Mihomo API服务器主机地址
    #[clap(long, default_value = "127.0.0.1")]
    pub mihomo_host: String,

    /// Mihomo API服务器端口号
    #[clap(long, default_value = "9090")]
    pub mihomo_port: u16,

    /// Mihomo API认证令牌
    #[clap(long, default_value = "")]
    pub mihomo_token: String,

    /// 主节点地址
    #[clap(long)]
    pub master_url: Option<String>,

    /// 主节点认证令牌
    #[clap(long)]
    pub master_token: Option<String>,

    /// 本地数据库路径（离线模式使用）
    #[clap(long, default_value = "agent.db")]
    pub local_database: String,

    /// 同步间隔（秒）
    #[clap(long, default_value = "60")]
    pub sync_interval: u64,
    
    /// 节点标识符
    #[clap(long)]
    pub agent_id: Option<String>,
} 