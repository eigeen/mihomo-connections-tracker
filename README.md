# Mihomo 连接跟踪器 - 主从架构版

此程序用于跟踪 Mihomo 的连接信息，并将其存储在数据库中。支持主从架构，允许多个从节点将数据同步到一个中央主节点，同时支持离线操作。

## 功能特点

- **主从架构**：支持一个主节点和多个从节点
- **数据同步**：从节点自动将连接数据同步到主节点
- **离线工作**：主节点不可用时，从节点在本地存储数据
- **自动恢复**：主节点恢复在线后，从节点自动同步本地数据
- **认证支持**：主节点和从节点之间的API通信支持令牌认证
- **独立运行**：主节点支持独立运行模式，可直接连接本地Mihomo收集数据
- **实时监控**：支持实时查看连接状态和流量变化
- **统计分析**：主节点提供丰富的统计API，支持多种筛选和聚合操作
- **高性能设计**：采用异步模型，支持高并发连接处理
- **模块化架构**：按照Rust设计规范进行模块化设计，易于扩展

## 使用方法

程序有两种运行模式：主节点服务器和从节点客户端。

### 构建程序

```bash
cargo build --release
```

### 运行主节点

```bash
./mihomo-connections-tracker master \
  --database master.db \
  --listen-host 0.0.0.0 \
  --listen-port 8080 \
  --api-token your-secret-token
```

参数说明：
- `--database`：数据库文件路径
- `--listen-host`：监听地址
- `--listen-port`：监听端口
- `--api-token`：可选的API认证令牌

### 主节点独立运行（连接本地Mihomo）

```bash
./mihomo-connections-tracker master \
  --database master.db \
  --listen-host 0.0.0.0 \
  --listen-port 8080 \
  --api-token your-secret-token \
  --mihomo-host 127.0.0.1 \
  --mihomo-port 9090 \
  --mihomo-token ""
```

附加参数说明（独立运行模式）：
- `--mihomo-host`：本地Mihomo API服务器地址
- `--mihomo-port`：本地Mihomo API服务器端口
- `--mihomo-token`：本地Mihomo API认证令牌（如果有）

### 运行从节点

```bash
./mihomo-connections-tracker agent \
  --mihomo-host 127.0.0.1 \
  --mihomo-port 9090 \
  --mihomo-token "" \
  --master-url http://master-ip:8080 \
  --master-token your-secret-token \
  --local-database agent.db \
  --sync-interval 60 \
  --agent-id my-unique-agent-id
```

参数说明：
- `--mihomo-host`：Mihomo API服务器地址（默认为127.0.0.1）
- `--mihomo-port`：Mihomo API服务器端口（默认为9090）
- `--mihomo-token`：Mihomo API认证令牌（如果有）
- `--master-url`：主节点URL（可选，不指定则仅在本地存储）
- `--master-token`：主节点认证令牌（可选）
- `--local-database`：本地数据库文件路径
- `--sync-interval`：同步间隔（秒）
- `--agent-id`：可选的节点标识符（如果不指定将自动生成）

## 离线模式

如果从节点未配置 `--master-url`，或者主节点临时不可用，从节点将在本地数据库存储所有连接数据。当主节点恢复可用时，从节点将自动同步所有累积的数据。

## 数据库结构

程序使用SQLite数据库存储连接数据。主节点数据库包含来自所有从节点的数据，每条记录都带有来源节点的标识符。

## 实时监控功能

程序会在控制台实时显示连接状态变化：
- 新建连接：显示连接详情，包括网络类型、源/目标地址、规则信息等
- 关闭连接：显示连接ID和累计流量统计
- 流量变化：实时监控并更新连接的流量数据

## 主节点API

主节点提供以下HTTP API端点：

- `GET /api/v1/health`：健康检查，返回"OK"
- `POST /api/v1/sync`：从节点同步数据的端点（需要认证）

### 统计API

主节点提供以下统计分析API：

#### 基本统计

- `GET /api/v1/stats`：统一的统计接口，支持多种查询参数

### 查询参数

统计API支持以下查询参数：

- `type`：统计类型（summary/group/timeseries）
- `group_by`：分组字段（network/rule/process/destination/geoip）
- `interval`：时间间隔（hour/day/week/month）
- `metric`：度量指标（connections/download/upload/total）
- `from`：开始时间，ISO 8601格式
- `to`：结束时间，ISO 8601格式
- `agent_id`：按代理ID筛选
- `network`：按网络类型筛选
- `rule`：按规则筛选
- `process`：按进程名筛选
- `destination`：按目标地址筛选
- `geoip`：按国家/地区筛选
- `limit`：限制返回结果数量
- `sort_by`：排序字段（count/download/upload）
- `sort_order`：排序顺序（asc/desc）

### 示例

获取连接总数统计：
```
GET /api/v1/stats?type=summary
```

获取按规则分组的流量统计：
```
GET /api/v1/stats?type=group&group_by=rule
```

获取最近24小时的按小时连接数时间序列：
```
GET /api/v1/stats?type=timeseries&interval=hour&metric=connections&from=2023-09-01T00:00:00Z&to=2023-09-02T00:00:00Z
```

获取特定从节点的TCP连接统计：
```
GET /api/v1/stats?type=summary&agent_id=agent1&network=tcp
```

## 技术详情

- 使用异步Rust（Tokio）构建
- WebSocket实时连接到Mihomo API
- SQLite用于数据存储
- HTTP API使用Warp框架构建
- 响应式设计，支持高并发操作
- 优化的数据传输和存储结构
