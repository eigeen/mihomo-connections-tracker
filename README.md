# Mihomo 连接跟踪器 - 主从架构版

此程序用于跟踪 [Mihomo](https://github.com/MetaCubeX/mihomo) (Clash Meta 内核) 的连接信息，并将其存储在数据库中。支持主从架构，允许多个从节点（Agent）将数据同步到一个中央主节点（Master），同时支持离线操作。

## 功能特点

- **主从架构**：支持一个 Master 节点和多个 Agent 节点。
- **数据同步**：Agent 自动将连接数据同步到 Master。
- **离线工作**：Master 不可用时，Agent 在本地存储数据。
- **自动恢复**：Master 恢复在线后，Agent 自动同步本地累积的数据。
- **认证支持**：Master 和 Agent 之间的 API 通信支持 Bearer Token 认证。
- **独立运行**：Master 支持独立运行模式，可直接连接本地 Mihomo 收集数据。
- **实时监控**：在控制台实时显示连接状态变化（新建、关闭、流量更新）。
- **统计分析 API**：Master 提供丰富的统计查询 API，支持多种筛选、聚合和时间序列分析。
- **高性能设计**：采用 Rust 和 Tokio 异步模型，支持高并发连接处理。
- **持久化存储**：使用 SQLite 数据库存储连接记录。

## 使用方法

程序通过命令行参数配置，分为两种运行模式：主节点服务器 (master) 和从节点客户端 (agent)。

### 构建程序

确保你已经安装了 Rust 工具链 ([rustup](https://rustup.rs/))。

```bash
cargo build --release
```

编译后的二进制文件位于 `./target/release/mihomo-connections-tracker` (文件名可能根据 `Cargo.toml` 中的设置有所不同)。以下示例假设二进制文件名为 `mihomo-connections-tracker`。

### 运行主节点 (Master)

Master 节点负责接收来自 Agent 的数据，存储数据，并提供查询 API。

```bash
./target/release/mihomo-connections-tracker master \
  --database ./master.db \
  --listen-host 0.0.0.0 \
  --listen-port 9091 \
  --api-token your-secret-token
```

**Master 节点参数说明:**

*   `--database <PATH>`: (必需) 主数据库文件路径 (SQLite)。
*   `--listen-host <HOST>`: (必需) API 服务器监听的主机地址。
*   `--listen-port <PORT>`: (必需) API 服务器监听的端口。
*   `--api-token <TOKEN>`: (可选) 设置 API 认证令牌。如果设置，Agent 连接和 API 查询都需要提供此令牌。

#### 主节点独立运行 (连接本地 Mihomo)

Master 节点也可以直接连接本地运行的 Mihomo 实例来收集数据，无需 Agent。

```bash
./target/release/mihomo-connections-tracker master \
  --database ./master.db \
  --listen-host 0.0.0.0 \
  --listen-port 9091 \
  --api-token your-secret-token \
  --mihomo-host 127.0.0.1 \
  --mihomo-port 9090 \
  --mihomo-token your-mihomo-secret-token
```

**附加参数说明 (独立运行模式):**

*   `--mihomo-host <HOST>`: (必需，若启用此模式) 本地 Mihomo API 服务器地址。
*   `--mihomo-port <PORT>`: (可选) 本地 Mihomo API 服务器端口 (默认: 9090)。
*   `--mihomo-token <TOKEN>`: (可选) 本地 Mihomo API 认证令牌 (如果 Mihomo 配置了 `secret`)。

### 运行从节点 (Agent)

Agent 节点连接本地 Mihomo 获取连接数据，并将数据发送到 Master 节点。如果 Master 节点不可用，数据会暂存在本地。

```bash
./target/release/mihomo-connections-tracker agent \
  --mihomo-host 127.0.0.1 \
  --mihomo-port 9090 \
  --mihomo-token your-mihomo-secret-token \
  --master-url http://<master_ip>:9091 \
  --master-token your-secret-token \
  --local-database ./agent.db \
  --agent-id my-unique-agent-id \
  --data-retention-days 7
```

**Agent 节点参数说明:**

*   `--mihomo-host <HOST>`: (必需) 本地 Mihomo API 服务器地址。
*   `--mihomo-port <PORT>`: (可选) 本地 Mihomo API 服务器端口 (默认: 9090)。
*   `--mihomo-token <TOKEN>`: (可选) 本地 Mihomo API 认证令牌 (如果 Mihomo 配置了 `secret`)。
*   `--master-url <URL>`: (必需) Master 节点的 API 地址 (例如 `http://192.168.1.10:9091`)。
*   `--master-token <TOKEN>`: (可选) Master 节点的 API 认证令牌 (如果 Master 配置了 `--api-token`)。
*   `--local-database <PATH>`: (必需) Agent 本地数据库文件路径 (SQLite)，用于离线缓存。
*   `--agent-id <ID>`: (必需) 当前 Agent 节点的唯一标识符。Master 通过此 ID 区分不同来源的数据。
*   `--sync-interval <SECONDS>`: (可选) 向 Master 同步数据的间隔时间（秒，默认 60）。
*   `--data-retention-days <DAYS>`: (可选) 数据保留天数，超过该天数的已同步数据会被自动删除（默认 7）。

## 离线模式

如果 Agent 在启动时未提供 `--master-url`，或者提供的 Master 节点暂时无法访问，Agent 将进入离线模式：
1.  连接数据将只存储在 `--local-database` 指定的本地 SQLite 文件中。
2.  如果配置了 `--master-url`，Agent 会定期尝试重新连接 Master。
3.  一旦 Master 恢复连接，Agent 会将本地缓存的所有未同步数据批量发送给 Master。

## 数据库

*   使用 SQLite 作为数据库。
*   Master 数据库 (`--database`) 存储所有 Agent 的聚合数据。
*   Agent 数据库 (`--local-database`) 存储该 Agent 的数据，主要用于离线缓存。
*   `connections` 表结构包含了详细的连接信息，主键为 `(id, agent_id)`。

## 实时监控

程序启动后，会在控制台实时打印连接事件日志：
*   **新连接**: 显示连接的基本信息 (协议, 源/目标 IP:Port, 规则, 代理链等)。
*   **连接关闭**: 显示连接 ID 和该连接的总计上/下行流量。
*   **流量更新**: (如果日志级别允许) 会定期显示活动连接的流量变化。

## 主节点 API

Master 节点提供一个 HTTP API 用于查询统计数据和接收 Agent 同步。

**API Base URL**: `http://<master-listen-host>:<master-listen-port>/api/v1`

**主要端点**:

*   `GET /api/v1/health`: 健康检查，无需认证。
*   `POST /api/v1/sync`: Agent 同步数据接口，**需要认证** (如果 Master 配置了 `--api-token`)。
*   `GET /api/v1/stats`: 获取统计数据接口，**需要认证** (如果 Master 配置了 `--api-token`)。

**详细的 API 文档，包括请求/响应结构、参数说明和示例，请参见 [API.md](api.md) 文件。**

## 技术栈

*   **语言**: Rust (Stable)
*   **异步运行时**: Tokio
*   **Web 框架 (API Server)**: Warp
*   **HTTP 客户端**: Reqwest
*   **WebSocket 客户端**: tokio-tungstenite
*   **数据库**: SQLite (via SQLx)
*   **命令行解析**: Clap
*   **序列化/反序列化**: Serde

---

**注意**: 请将示例命令中的 `./target/release/mihomo-connections-tracker` 替换为实际的二进制文件路径和名称。
