# mihomo-connections-tracker

一个用于实时监控并记录 [mihomo](https://github.com/MetaCubeX/mihomo)（原Clash）网络连接信息的工具，将连接数据持久化存储到SQLite数据库。

## 功能特性

- 实时监控mihomo API的connections数据
- 自动维护连接生命周期（新建/关闭检测）
- 流量变化追踪与持久化存储
- 支持以下连接元数据记录：
  - 网络类型（TCP/UDP）
  - 源/目的IP和端口
  - GEOIP信息
  - 代理链路径
  - 匹配规则
  - 进程信息
  - 入站配置
  - DNS信息等
- 支持Token认证
- 自动重连机制
- 优雅停机处理

## 安装使用

### 前置要求

- Rust 1.70+ 环境
- mihomo API服务已启用

### 安装步骤

1. 克隆仓库：
```bash
git clone https://github.com/djkcyl/mihomo-connections-tracker.git
cd mihomo-connections-tracker
```

2. 编译项目：
```bash
cargo build --release
```

### 运行命令
```bash
./target/release/mihomo-connections-tracker \
    --host 127.0.0.1 \
    --port 9090 \
    --token your_api_token
```

## 命令行参数

| 参数    | 描述               | 默认值   |
|---------|--------------------|----------|
| --host  | API服务器地址      | 127.0.0.1|
| --port  | API端口号          | 9090     |
| --token | 认证令牌           | 空字符串 |

## 数据库结构

数据存储在`connections.db`文件中，表结构如下：

```sql
CREATE TABLE connections (
    id TEXT PRIMARY KEY,             -- 连接唯一标识
    conn_download INTEGER,           -- 累计下载字节
    conn_upload INTEGER,             -- 累计上传字节
    last_updated TEXT,               -- 最后更新时间(ISO8601)
    start TEXT,                      -- 连接开始时间(ISO8601)
    network TEXT,                    -- 网络类型(tcp/udp)
    type TEXT,                       -- 连接类型
    source_ip TEXT,                  -- 源IP地址
    destination_ip TEXT,             -- 目的IP地址
    source_geoip TEXT,               -- 源IP地理信息(JSON)
    destination_geoip TEXT,          -- 目的IP地理信息(JSON)
    -- 其余字段详见metadata结构...
);
```

## 实现细节

1. **连接管理**：
   - 使用WebSocket长连接接收实时数据
   - 通过HashSet比较检测连接变化
   - 流量变化检测使用缓存机制

2. **数据持久化**：
   - 使用SQLx进行异步数据库操作
   - Upsert操作保证数据更新原子性
   - 每200ms批量提交变更

3. **异常处理**：
   - 自动重连机制
   - Ctrl+C信号处理
   - 连接超时控制

4. **日志输出**：
   - 新连接建立时打印详细信息
   - 连接关闭时显示流量统计
   - 每次更新显示变更摘要
