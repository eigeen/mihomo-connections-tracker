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
    "records_processed": 1
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

### 3.5 错误处理

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

## 5. 前端开发指南

### 5.1 技术栈建议

- **框架**: React/Vue.js/Angular
- **UI库**: Ant Design/Element UI/Material UI
- **图表库**: ECharts/Chart.js/D3.js
- **状态管理**: Redux/Vuex/MobX
- **API请求**: Axios/Fetch API

### 5.2 主要页面/组件

#### 5.2.1 仪表盘

- 总览卡片（活跃连接数、总流量、上传/下载流量）
- 流量趋势图（24小时/7天/30天）
- 地理分布地图（基于destination_geoip）
- 热门应用/进程排行
- 热门目标域名/IP排行

#### 5.2.2 连接列表

- 实时连接列表（支持分页、搜索和排序）
- 每个连接的详细信息（包括源/目标、规则、流量等）
- 流量实时更新指示
- 支持按不同条件过滤（网络类型、规则、进程等）

#### 5.2.3 统计分析

- 按不同维度的数据透视图（规则、地理位置、应用等）
- 流量时间序列分析
- 自定义时间范围选择器
- 可导出的数据报表

#### 5.2.4 节点管理（可选）

- Agent节点列表和状态监控
- 每个节点的连接数据独立视图
- 节点同步状态监控

### 5.3 数据获取和刷新策略

- 仪表盘数据：每30秒自动刷新一次
- 连接列表：实时数据或每5-10秒刷新一次
- 统计分析：按需加载，用户可手动刷新
- 考虑使用WebSocket实现实时数据更新（如果后端支持）

### 5.4 前端认证处理

```javascript
// Axios 请求示例（带认证）
import axios from 'axios';

const api = axios.create({
  baseURL: 'http://<master-host>:<master-port>/api/v1',
  timeout: 10000,
});

// 添加认证令牌
api.interceptors.request.use(config => {
  const token = localStorage.getItem('api_token');
  if (token) {
    config.headers['Authorization'] = `Bearer ${token}`;
  }
  return config;
});

// 调用API示例
async function fetchStatsSummary() {
  try {
    const response = await api.get('/stats', {
      params: { type: 'summary' }
    });
    return response.data.data;
  } catch (error) {
    console.error('获取统计摘要失败:', error);
    throw error;
  }
}
```

### 5.5 数据可视化示例

```javascript
// ECharts 地理分布图示例
function renderGeoDistribution(data) {
  const chart = echarts.init(document.getElementById('geo-chart'));
  
  const option = {
    title: {
      text: '连接地理分布',
      subtext: '按国家/地区统计',
      left: 'center'
    },
    tooltip: {
      trigger: 'item',
      formatter: '{b}: {c} ({d}%)'
    },
    visualMap: {
      min: 0,
      max: Math.max(...data.map(item => item.count)),
      left: 'left',
      top: 'bottom',
      text: ['高', '低'],
      calculable: true
    },
    series: [
      {
        name: '连接数',
        type: 'map',
        map: 'world',
        roam: true,
        emphasis: {
          label: {
            show: true
          }
        },
        data: data.map(item => ({
          name: item.destination_geoip,
          value: item.count
        }))
      }
    ]
  };
  
  chart.setOption(option);
}
```

### 5.6 前端界面设计方案

基于AdGuard Home风格的界面设计，将页面分为三个主要垂直板块：

#### 5.6.1 顶部统计总览（第一排）

**布局**：4个等宽卡片横向排列，每个卡片包含以下元素：
- 核心指标的醒目大数字
- 配套图标和标题
- 同比上期的增长/下降百分比（可选）

**卡片内容**：
- **连接总数**：显示活跃连接数量
- **总流量**：显示上传+下载总流量
- **下载流量**：显示下载的数据总量
- **上传流量**：显示上传的数据总量

**颜色方案**：
- 连接总数：蓝色调
- 总流量：紫色调
- 下载流量：绿色调
- 上传流量：橙色调

#### 5.6.2 中部分组统计（第二排）

**布局**：一个宽卡片面板，包含：
- 顶部控制栏（分组字段选择器、图表类型切换按钮）
- 主体是横向条形图

**功能元素**：
- **分组字段选择器**：下拉菜单，选项包括：
  - 规则(rule)
  - 地理位置(geoip)
  - 应用程序(process)
  - 网络类型(network)
  - 代理链路(chains)
  - 目标地址(destination)
  - 主机名(host)

- **图表类型切换**：图标按钮组，选项包括：
  - 横向条形图（默认）
  - 饼图
  - 表格视图

**图表展示**：
- 横向条形图，每条包含：
  - 左侧显示分组名称（如国家名称、规则名称等）
  - 右侧是条形图，长度代表数值大小
  - 条形末端显示具体数值
  - 不同的指标用不同颜色区分（连接数、下载、上传）

**交互功能**：
- 鼠标悬停显示详细信息（包括连接数、下载、上传等全部指标）
- 点击某个分组项可以进入该分组的详细数据页面

#### 5.6.3 底部时间序列（第三排）

**布局**：单个大型卡片，包含：
- 顶部控制栏（时间间隔选择、指标选择、日期范围选择器）
- 主体为折线图区域

**功能元素**：
- **时间间隔选择器**：标签页式切换，选项包括：
  - 按小时(hour)
  - 按天(day)
  - 按周(week)
  - 按月(month)

- **指标选择器**：下拉菜单，选项包括：
  - 连接数(connections)
  - 下载流量(download)
  - 上传流量(upload)
  - 总流量(total)

- **日期范围选择器**：日历控件，可选择查询的时间范围

**图表展示**：
- 折线图，x轴为时间，y轴为所选指标值
- 可选择显示平滑曲线或直线
- 图表下方有迷你缩略图，可拖动选择查看区域

**交互功能**：
- 鼠标悬停显示具体时间点的详细数据
- 支持缩放和平移操作
- 点击图例可以切换显示/隐藏某些数据系列

#### 5.6.4 响应式设计

- 在桌面端，三个板块垂直排列，每个板块占据全宽
- 在平板端，保持垂直排列但减少边距和内边距
- 在移动端，调整卡片为单列布局，图表自动缩放

#### 5.6.5 设计示意图

```
+---------------------------------------------------------------------+
|                                                                     |
|  +----------+    +----------+    +----------+    +----------+       |
|  | 连接总数  |    |  总流量   |    | 下载流量  |    | 上传流量  |       |
|  |  2,345   |    |  4.2 GB  |    |  3.1 GB  |    |  1.1 GB  |       |
|  |  +10%    |    |   +5%    |    |   +3%    |    |   +12%   |       |
|  +----------+    +----------+    +----------+    +----------+       |
|                                                                     |
|  +------------------------------------------------------------------+
|  | 分组统计                                   [网络] [条形图] [刷新] |
|  |                                                                  |
|  |  中国(CN)  |████████████████████████|  1.2 GB                    |
|  |  美国(US)  |████████████████|  850 MB                            |
|  |  香港(HK)  |███████████|  650 MB                                 |
|  |  新加坡(SG)|██████|  350 MB                                      |
|  |  日本(JP)  |█████|  300 MB                                       |
|  |                                                                  |
|  +------------------------------------------------------------------+
|                                                                     |
|  +------------------------------------------------------------------+
|  | 时间序列统计   [小时] [天] [周] [月]  [总流量▼]  [2023-10-01 ~ 2023-10-07] |
|  |                                                                  |
|  |    ^                                                             |
|  |    |                     ●                                       |
|  |    |                ●         ●       ●                          |
|  |    |           ●                  ●       ●                      |
|  |    |       ●                                                     |
|  |    |   ●                                                         |
|  |    +------------------------------------------------------------->|
|  |    00:00   04:00   08:00   12:00   16:00   20:00   00:00        |
|  |                                                                  |
|  |  [缩略导航条...................................................] |
|  +------------------------------------------------------------------+
|                                                                     |
+---------------------------------------------------------------------+
```

## 6. 开发建议和最佳实践

1. **封装API调用**：创建统一的API服务层，处理认证、错误处理和数据转换
2. **适配移动设备**：确保UI响应式设计，支持桌面和移动设备访问
3. **数据缓存策略**：实现适当的缓存策略减少重复请求，提高用户体验
4. **错误处理**：完善的错误处理和用户友好的错误提示
5. **国际化支持**：考虑多语言支持（至少中英文）
6. **主题支持**：提供明暗两种主题模式
7. **性能优化**：
   - 列表虚拟滚动处理大量数据
   - 组件懒加载
   - 数据按需加载和分页

## 7. 开发流程建议

1. **设计阶段**：
   - 创建基本UI/UX设计和页面布局
   - 定义组件结构和层次关系
   - 设计状态管理方案

2. **开发阶段**：
   - 实现基础组件和页面框架
   - 集成API调用和数据获取
   - 开发数据可视化组件
   - 实现实时更新功能

3. **测试阶段**：
   - 开发环境下进行单元测试和集成测试
   - 测试不同数据量下的性能表现
   - 测试不同网络条件下的应用行为

4. **部署阶段**：
   - 构建生产版本
   - 配置与后端服务的集成
   - 监控和优化

## 8. 运行环境要求

- 支持现代浏览器（Chrome、Firefox、Safari、Edge等）
- 同源部署或配置CORS（已在后端支持）
- 合理的客户端内存用量（考虑到可能的大量连接数据）

---

如需更多技术支持或有疑问，请参考项目GitHub仓库或联系开发团队。