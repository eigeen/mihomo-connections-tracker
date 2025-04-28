# Mihomo 连接跟踪器 (Mihomo Connections Tracker) - 前端开发文档

## 1. 项目概述

Mihomo 连接跟踪器是一个用于跟踪和监控 Mihomo (Clash Meta 内核) 连接信息的系统。该项目采用主从架构，支持多个从节点（Agent）将数据同步到一个中央主节点（Master），同时支持离线操作。

### 1.1 系统架构

- **主节点（Master）**：中央服务器，接收和存储来自代理节点的连接数据，并提供数据查询API
- **从节点（Agent）**：部署在各个 Mihomo 实例所在的设备上，收集连接数据并发送到主节点
- **数据库**：基于 SQLite，用于存储连接记录和同步状态

## 2. 前端开发目标

开发一个直观、高效的前端界面，用于：

- 实时监控所有 Mihomo 连接信息
- 提供丰富的数据统计和可视化功能
- 支持按不同维度（规则、流量、地域等）进行数据过滤和聚合分析
- 提供时间序列数据分析

## 3. API 接口详情

### 3.1 基础信息

- **API 基础 URL**: `http://<master-host>:<master-port>/api/v1`
- **认证方式**: Bearer Token（如果配置了`--api-token`）
  - 请求头格式: `Authorization: Bearer <token>`
- **响应格式**: JSON
- **统一响应结构**:
  ```json
  {
    "status": "success|error",
    "data": <数据对象或数组>,
    "message": <错误信息，成功时为null>
  }
  ```

### 3.2 健康检查

```
GET /api/v1/health
```

用于检查服务器是否正常运行。无需认证。

**响应**: 成功返回 `OK` 字符串

### 3.3 数据同步 API

```
POST /api/v1/sync
```

用于 Agent 向 Master 同步连接数据。需要认证（如果配置了`--api-token`）。

**请求体**:
```json
{
  "agent_id": "my-unique-agent-id",
  "connections": [
    {
      "id": "6c5cef8b-db4d",
      "download": 102400,
      "upload": 51200,
      "last_updated": "2023-01-01T12:30:45.123Z",
      "start": "2023-01-01T12:30:00.000Z",
      "network": "TCP",
      "conn_type": "HTTP",
      "source_ip": "192.168.1.100",
      "destination_ip": "203.0.113.1",
      "source_geoip": "{\"country\":\"CN\",\"province\":\"Beijing\",\"city\":\"Beijing\"}",
      "destination_geoip": "{\"country\":\"US\",\"province\":\"California\",\"city\":\"San Francisco\"}",
      "source_port": "52100",
      "destination_port": "443",
      "host": "example.com",
      "process": "chrome",
      "chains": "[\"DIRECT\"]",
      "rule": "DIRECT",
      "rule_payload": ""
    }
    // 更多连接记录...
  ],
  "timestamp": "2023-01-01T12:30:50.000Z"
}
```

**响应**:
```json
{
  "status": "success",
  "data": {
    "message": "数据同步成功",
    "count": 1
  },
  "message": null
}
```

**错误响应**:
```json
{
  "status": "error",
  "data": null,
  "message": "无效的请求数据"
}
```

### 3.4 统计数据 API

```
GET /api/v1/stats
```

用于获取各类统计数据。需要认证（如果配置了`--api-token`）。

**查询参数**:

| 参数 | 类型 | 描述 | 示例 |
|------|------|------|------|
| type | string | **必需** 统计类型: summary, group, timeseries | `type=group` |
| group_by | string | （group类型必需）分组依据: network, rule, process, destination, geoip, host, chains | `group_by=geoip` |
| interval | string | （timeseries类型必需）时间间隔: hour, day, week, month | `interval=hour` |
| metric | string | 统计指标: connections, download, upload, total | `metric=total` |
| from | string | 开始时间 (ISO 8601格式) | `from=2023-01-01T00:00:00Z` |
| to | string | 结束时间 (ISO 8601格式) | `to=2023-01-31T23:59:59Z` |
| agent_id | string | 按代理ID过滤 | `agent_id=my-agent` |
| network | string | 按网络类型过滤 | `network=TCP` |
| rule | string | 按规则过滤 | `rule=DIRECT` |
| process | string | 按进程名过滤 | `process=chrome` |
| destination | string | 按目标地址过滤 | `destination=8.8.8.8` |
| geoip | string | 按国家/地区过滤 | `geoip=CN` |
| limit | number | 限制返回结果数量 | `limit=10` |
| sort_by | string | 排序字段: count, download, upload, total | `sort_by=download` |
| sort_order | string | 排序顺序: asc, desc | `sort_order=desc` |

#### 3.4.1 统计汇总

**请求**: `GET /api/v1/stats?type=summary`

获取所有连接数据的汇总统计信息，包括连接总数、总下载流量、总上传流量和总流量。

**支持的过滤参数**: agent_id, network, rule, process, destination, geoip, from, to

**响应**:
```json
{
  "status": "success",
  "data": {
    "count": 1200,
    "download": 2500000000,
    "upload": 500000000,
    "total": 3000000000
  },
  "message": null
}
```

#### 3.4.2 分组统计

**请求**: `GET /api/v1/stats?type=group&group_by=geoip&limit=5&sort_by=total&sort_order=desc`

按指定字段对连接数据进行分组统计，返回每个分组的连接数和流量信息。

**支持的分组字段**:
- `network`: 按网络类型分组 (TCP, UDP)
- `rule`: 按规则分组 (DIRECT, PROXY 等)
- `process`: 按进程名分组
- `destination`: 按目标IP地址分组
- `geoip`: 按目标地理位置分组
- `host`: 按主机名分组
- `chains`: 按代理链路分组 (提取最后一个节点)

**响应示例**（按 geoip 分组）:
```json
{
  "status": "success",
  "data": [
    {
      "destination_geoip": "CN",
      "count": 450,
      "download": 1200000000,
      "upload": 300000000,
      "total": 1500000000
    },
    {
      "destination_geoip": "US",
      "count": 350,
      "download": 800000000,
      "upload": 100000000,
      "total": 900000000
    },
    // ...更多结果
  ],
  "message": null
}
```

**响应示例**（按 rule 分组）:
```json
{
  "status": "success",
  "data": [
    {
      "rule": "DIRECT",
      "count": 600,
      "download": 1500000000,
      "upload": 350000000,
      "total": 1850000000
    },
    {
      "rule": "PROXY (example.com)",
      "count": 450,
      "download": 900000000,
      "upload": 200000000,
      "total": 1100000000
    },
    // ...更多结果
  ],
  "message": null
}
```

**响应示例**（按 chains 分组）:
```json
{
  "status": "success",
  "data": [
    {
      "node": "HK-01",
      "count": 250,
      "download": 700000000,
      "upload": 100000000,
      "total": 800000000
    },
    {
      "node": "US-03",
      "count": 180,
      "download": 500000000,
      "upload": 80000000,
      "total": 580000000
    },
    // ...更多结果
  ],
  "message": null
}
```

#### 3.4.3 时间序列统计

**请求**: `GET /api/v1/stats?type=timeseries&interval=hour&metric=total&from=2023-01-01T00:00:00Z&to=2023-01-01T23:59:59Z`

按时间间隔统计数据，生成时间序列数据点，用于趋势分析和图表展示。

**支持的时间间隔**:
- `hour`: 按小时统计
- `day`: 按天统计
- `week`: 按周统计
- `month`: 按月统计

**支持的指标**:
- `connections`: 连接数
- `download`: 下载流量
- `upload`: 上传流量
- `total`: 总流量

**响应**:
```json
{
  "status": "success",
  "data": [
    {
      "time": "2023-01-01T00:00:00Z",
      "value": 120000000
    },
    {
      "time": "2023-01-01T01:00:00Z",
      "value": 150000000
    },
    {
      "time": "2023-01-01T02:00:00Z",
      "value": 180000000
    },
    // ...更多时间点
  ],
  "message": null
}
```

### 3.5 连接数据 API (已实现)

> 注意：这是一个新增的API端点，用于获取活跃连接数据。

```
GET /api/v1/connections
```

获取当前活跃的连接数据列表，支持过滤和分页。

**查询参数**:

| 参数 | 类型 | 描述 | 示例 |
|------|------|------|------|
| from | string | 开始时间 (ISO 8601格式) | `from=2023-01-01T00:00:00Z` |
| to | string | 结束时间 (ISO 8601格式) | `to=2023-01-31T23:59:59Z` |
| agent_id | string | 按代理ID过滤 | `agent_id=my-agent` |
| network | string | 按网络类型过滤 | `network=TCP` |
| rule | string | 按规则过滤 | `rule=DIRECT` |
| process | string | 按进程名过滤 | `process=chrome` |
| destination | string | 按目标地址过滤 | `destination=8.8.8.8` |
| geoip | string | 按国家/地区过滤 | `geoip=CN` |
| limit | number | 限制返回结果数量 | `limit=50` |
| offset | number | 分页偏移 | `offset=0` |
| sort_by | string | 排序字段: start, download, upload, total | `sort_by=download` |
| sort_order | string | 排序顺序: asc, desc | `sort_order=desc` |

**响应**:
```json
{
  "status": "success",
  "data": [
    {
      "id": "6c5cef8b-db4d",
      "agent_id": "my-agent-1",
      "download": 102400,
      "upload": 51200,
      "last_updated": "2023-01-01T12:30:45.123Z",
      "start": "2023-01-01T12:30:00.000Z",
      "network": "TCP",
      "conn_type": "HTTP",
      "source_ip": "192.168.1.100",
      "destination_ip": "203.0.113.1",
      "destination_geoip": "US",
      "source_port": "52100",
      "destination_port": "443",
      "host": "example.com",
      "process": "chrome",
      "chains": "[\"DIRECT\"]",
      "rule": "DIRECT",
      "rule_payload": ""
    },
    // ...更多连接数据
  ],
  "total_count": 127, // 总记录数
  "message": null
}
```

### 3.6 代理节点管理 API (已实现)

```
GET /api/v1/agents
```

获取所有代理节点的状态信息。

**响应**:
```json
{
  "status": "success",
  "data": [
    {
      "agent_id": "my-agent-1",
      "last_sync": "2023-01-01T12:30:50.000Z",
      "connection_count": 27,
      "total_download": 1024000000,
      "total_upload": 512000000,
      "status": "online"
    },
    {
      "agent_id": "my-agent-2",
      "last_sync": "2023-01-01T12:29:50.000Z",
      "connection_count": 15,
      "total_download": 512000000,
      "total_upload": 256000000,
      "status": "online"
    }
  ],
  "message": null
}
```

### 3.7 错误处理

当请求不成功时，API 将返回适当的 HTTP 状态码和错误消息:

| 状态码 | 描述 | 示例响应 |
|--------|------|-----------|
| 400 | 请求参数错误 | `{"status":"error","data":null,"message":"不支持的统计类型: invalid"}` |
| 401 | 未授权 | `{"status":"error","data":null,"message":"认证失败"}` |
| 404 | 资源不存在 | `{"status":"error","data":null,"message":"请求的资源不存在"}` |
| 500 | 服务器内部错误 | `{"status":"error","data":null,"message":"数据库查询失败"}` |

## 4. 数据模型

### 4.1 连接记录 (ConnectionRecord)

连接记录包含以下主要字段：

| 字段名 | 类型 | 描述 |
|--------|------|------|
| id | string | 连接唯一标识符 |
| download | number | 下载流量（字节） |
| upload | number | 上传流量（字节） |
| last_updated | string | 最后更新时间 (ISO 8601格式) |
| start | string | 连接开始时间 (ISO 8601格式) |
| network | string | 网络类型 (TCP, UDP等) |
| conn_type | string | 连接类型 |
| source_ip | string | 源IP地址 |
| destination_ip | string | 目标IP地址 |
| source_geoip | string | 源地址地理位置信息 (JSON) |
| destination_geoip | string | 目标地址地理位置信息 (JSON) |
| source_ip_asn | string | 源IP的ASN信息 |
| destination_ip_asn | string | 目标IP的ASN信息 |
| source_port | string | 源端口 |
| destination_port | string | 目标端口 |
| inbound_ip | string | 入站IP |
| inbound_port | string | 入站端口 |
| inbound_name | string | 入站名称 |
| inbound_user | string | 入站用户 |
| host | string | 目标主机名 |
| dns_mode | string | DNS解析模式 |
| uid | number | 用户ID |
| process | string | 进程名 |
| process_path | string | 进程路径 |
| special_proxy | string | 特殊代理 |
| special_rules | string | 特殊规则 |
| remote_destination | string | 远程目标 |
| dscp | number | DSCP值 |
| sniff_host | string | 嗅探的主机名 |
| chains | string | 代理链路 (JSON数组) |
| rule | string | 匹配的规则 |
| rule_payload | string | 规则载荷 |
| agent_id | string | 代理节点ID |

## 5. 已实现的前端组件

### 5.1 主要组件

前端已实现以下核心组件，提供了完整的数据可视化和交互体验：

#### 5.1.1 仪表盘 (Dashboard)

- 总览页面，包含统计卡片、分组统计和时间序列图表
- 基于选项卡的设计，支持切换不同数据视图
- 支持切换到连接列表和代理节点视图

#### 5.1.2 统计卡片 (StatCards)

- 显示关键数据指标：连接总数、总流量、下载和上传流量
- 支持刷新数据
- 根据当前过滤条件动态更新

#### 5.1.3 分组统计 (GroupedStats)

- 支持按多个维度分组数据：国家/地区、规则、进程、网络类型、代理链路、目标IP和主机
- 提供条形图和表格两种显示模式
- 自动对数据进行排序，突出显示重要数据

#### 5.1.4 时间序列统计 (TimeSeriesStats)

- 支持按小时、天、周、月不同时间粒度显示数据
- 支持选择不同指标：连接数、下载、上传和总流量
- 包含缩略导航条，方便查看更长时间范围的数据

#### 5.1.5 连接列表 (ConnectionsList)

- 显示详细的连接数据，包括源/目标、规则、流量等信息
- 支持搜索、过滤和排序
- 实时刷新显示连接状态

#### 5.1.6 过滤器 (FiltersPopover)

- 提供统一的过滤界面，支持按多种条件筛选数据
- 包含日期范围选择器，支持选择自定义时间段
- 所有筛选条件在不同组件间共享，保持一致性

#### 5.1.7 设置对话框 (SettingsDialog)

- 提供应用设置界面，包括API URL和令牌配置
- 支持暗/亮主题切换
- 支持保存用户偏好设置

### 5.2 技术栈实现

前端使用以下技术栈实现：

- **框架**: React 18 + Next.js 14
- **UI库**: Shadcn UI (基于Tailwind CSS)
- **图表库**: Recharts
- **状态管理**: React Context API
- **API请求**: 原生Fetch API
- **编程语言**: TypeScript

## 6. 前端界面设计

### 6.1 布局结构

前端界面采用现代化的布局结构：

1. **顶部导航栏**
   - 应用标题和logo
   - 主题切换按钮
   - 设置按钮

2. **主要标签栏**
   - 概览标签（默认）
   - 连接标签
   - 代理节点标签

3. **概览页面**
   - 统计卡片（第一行）
   - 全局过滤器和日期选择器
   - 分组统计图表/表格
   - 时间序列图表

4. **连接列表页面**
   - 连接搜索和过滤器
   - 可分页的连接表格
   - 刷新按钮

5. **代理节点页面**
   - 代理节点状态卡片
   - 节点详情和统计信息

### 6.2 响应式设计

界面已实现完全响应式设计：

- 在桌面端提供最完整的布局，充分利用屏幕空间
- 在平板设备上，卡片自动调整为流式布局
- 在移动设备上，组件垂直堆叠，保持良好的可读性和可用性

## 7. 开发实践

### 7.1 API集成

前端已实现与所有API端点的完整集成：

```typescript
// API 客户端示例
export async function fetchStatsSummary(filters = {}) {
  const queryParams = new URLSearchParams({
    type: "summary",
    ...filters,
  })

  return apiRequest(`/stats?${queryParams.toString()}`)
}

export async function fetchGroupedStats(groupBy: string, filters = {}) {
  const queryParams = new URLSearchParams({
    type: "group",
    group_by: groupBy,
    sort_by: "total",
    sort_order: "desc",
    limit: "10",
    ...filters,
  })

  return apiRequest(`/stats?${queryParams.toString()}`)
}

export async function fetchTimeSeriesStats(params: Record<string, any>) {
  const { interval, metric, from, to, ...filters } = params

  const queryParams = new URLSearchParams({
    type: "timeseries",
    interval: interval || "hour",
    metric: metric || "total",
    ...filters,
  })

  if (from) queryParams.append("from", from)
  if (to) queryParams.append("to", to)

  return apiRequest(`/stats?${queryParams.toString()}`)
}
```

### 7.2 数据可视化

前端使用Recharts库实现了丰富的数据可视化功能：

- **条形图**：用于展示分组统计数据，便于比较不同分组值
- **折线图**：用于展示时间序列数据，清晰展示趋势变化
- **自定义图例和提示框**：增强数据可读性
- **交互式缩放和平移**：支持探索大型数据集
- **自动适应颜色**：根据数据类型和分组动态调整

### 7.3 数据刷新策略

针对不同组件实现了优化的数据刷新策略：

- **统计卡片**：每30秒自动刷新一次
- **连接列表**：每10秒自动刷新一次
- **分组统计和时间序列**：根据用户交互按需刷新
- **支持手动刷新**：每个组件都提供刷新按钮

## 8. 部署指南

### 8.1 前端部署

1. **构建生产版本**：
   ```bash
   cd frontend
   npm run build
   ```

2. **部署静态文件**：
   - 将`frontend/out`目录下的文件部署到Web服务器
   - 或使用Vercel/Netlify等服务托管

3. **配置API地址**：
   - 在前端访问时，初始设置界面配置API地址和令牌
   - 设置保存在浏览器本地存储中

### 8.2 后端部署

1. **构建后端**：
   ```bash
   cd backend
   cargo build --release
   ```

2. **启动主节点**：
   ```bash
   ./target/release/mihomo-connections-tracker master \
     --host 0.0.0.0 \
     --port 8080 \
     --db-path /data/connections.db \
     --api-token YOUR_SECRET_TOKEN
   ```

3. **启动从节点**：
   ```bash
   ./target/release/mihomo-connections-tracker agent \
     --mihomo-host 127.0.0.1 \
     --mihomo-port 9090 \
     --mihomo-token YOUR_MIHOMO_TOKEN \
     --master-url http://master-host:8080 \
     --master-token YOUR_SECRET_TOKEN \
     --agent-id my-unique-agent-id
   ```

## 9. 性能优化

为保证系统在大量连接数据下的性能，已实现以下优化：

1. **数据分页**：连接列表支持分页加载
2. **按需加载**：图表和表格仅在需要时加载数据
3. **数据缓存**：前端实现临时数据缓存减少重复请求
4. **懒加载组件**：使用React的懒加载减少初始加载时间
5. **SQL查询优化**：后端使用索引和优化查询提高数据库性能
6. **数据清理**：自动清理旧数据减轻数据库负担

---

如需更多技术支持或有疑问，请参考项目GitHub仓库或联系开发团队。