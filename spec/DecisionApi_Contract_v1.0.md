# DecisionApi 契约规范（v1.0）

## 文档信息

- **版本**: v1.0
- **创建日期**: 2026-01-23
- **状态**: ✅ 已完成
- **依据**:
  - `spec/Claude_Dev_Master_Spec.md`
  - `spec/Tauri_API_Contract_v0.3_Integrated.md`
  - `REFACTOR_PLAN_v1.0.md` - 第 7 章
- **定位**: DecisionApi 层的 Tauri Command 契约规范

---

## 1. 概述

### 1.1 设计目标

DecisionApi 是决策支持层的前后端交互契约，提供 6 个核心决策查询命令，用于回答：

1. **D1**: 哪天最危险
2. **D2**: 哪些紧急单无法完成
3. **D3**: 哪些冷料压库
4. **D4**: 哪个机组最堵
5. **D5**: 换辊是否异常
6. **D6**: 是否存在产能优化空间

### 1.2 架构原则

- **决策支持优先**: 所有 API 返回可解释的决策结果，而非原始数据
- **版本隔离**: 每个查询必须指定 `version_id`，避免跨版本数据污染
- **工业可解释性**: 所有结果必须包含 `reasons` 或 `top_reasons` 字段
- **高性能查询**: 基于预计算的读模型表，避免实时计算
- **CQRS 模式**: 只读查询，不修改任何状态

### 1.3 通用约定

- **时间格式**: ISO 8601 日期格式 (`YYYY-MM-DD`)
- **时间戳**: ISO 8601 完整时间戳 (`YYYY-MM-DDTHH:MM:SS.sssZ`)
- **重量单位**: 吨 (t)，保留 3 位小数
- **百分比**: 0-100 的浮点数
- **分数**: 0-100 的浮点数，表示风险/压力/堵塞等程度

---

## 2. DecisionApi 命令清单

| 命令 | 优先级 | 说明 | 核心用例 |
|------|--------|------|----------|
| `get_decision_day_summary` | P1 | 查询未来 N 天的风险排行与解释 | D1: 哪天最危险 |
| `get_machine_bottleneck_profile` | P1 | 查询机组×日期的堵塞热力图与明细 | D4: 哪个机组最堵 |
| `list_order_failure_set` | P3 | 查询紧急订单的失败集合（可筛选） | D2: 哪些紧急单无法完成 |
| `get_cold_stock_profile` | P3 | 查询冷料分桶与压库趋势 | D3: 哪些冷料压库 |
| `list_roll_campaign_alerts` | P3 | 查询换辊预警列表 | D5: 换辊是否异常 |
| `get_capacity_opportunity` | P3 | 查询产能优化空间与敏感性分析 | D6: 是否存在产能优化空间 |

---

## 3. API 详细规范

### 3.1 D1: get_decision_day_summary

**用例**: 回答"未来 N 天哪天最危险"

#### 请求参数

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetDecisionDaySummaryRequest {
    /// 方案版本 ID（必填）
    pub version_id: String,

    /// 日期范围起始（必填，ISO DATE）
    pub date_from: String,

    /// 日期范围结束（必填，ISO DATE）
    pub date_to: String,

    /// 风险等级过滤（可选）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub risk_level_filter: Option<Vec<String>>,  // ["LOW", "MEDIUM", "HIGH", "CRITICAL"]

    /// 返回条数限制（可选，默认 10）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit: Option<u32>,

    /// 排序方式（可选，默认按风险分数降序）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sort_by: Option<String>,  // "risk_score" | "plan_date" | "capacity_util_pct"
}
```

#### 响应结构

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecisionDaySummaryResponse {
    /// 方案版本 ID
    pub version_id: String,

    /// 查询时间戳
    pub as_of: String,  // ISO 8601 timestamp

    /// 日期摘要列表
    pub items: Vec<DaySummary>,

    /// 总记录数
    pub total_count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DaySummary {
    /// 计划日期
    pub plan_date: String,

    /// 风险分数 (0-100)
    pub risk_score: f64,

    /// 风险等级
    pub risk_level: String,  // "LOW" | "MEDIUM" | "HIGH" | "CRITICAL"

    /// 产能利用率 (%)
    pub capacity_util_pct: f64,

    /// 超载吨数 (t)
    pub overload_weight_t: f64,

    /// 紧急单失败数量
    pub urgent_failure_count: u32,

    /// 主要风险原因（按权重降序）
    pub top_reasons: Vec<ReasonItem>,

    /// 涉及的机组列表
    pub involved_machines: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReasonItem {
    /// 原因代码
    pub code: String,  // "CAPACITY_OVERFLOW" | "ROLL_HARD_STOP" | "STRUCTURE_CONFLICT" | ...

    /// 原因描述
    pub msg: String,

    /// 权重 (0-1)
    pub weight: f64,

    /// 影响的材料数量（可选）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub affected_count: Option<u32>,
}
```

#### 示例

**请求**:
```json
{
  "version_id": "V20260123-001",
  "date_from": "2026-01-24",
  "date_to": "2026-01-30",
  "risk_level_filter": ["HIGH", "CRITICAL"],
  "limit": 5,
  "sort_by": "risk_score"
}
```

**响应**:
```json
{
  "version_id": "V20260123-001",
  "as_of": "2026-01-23T14:30:00.000Z",
  "items": [
    {
      "plan_date": "2026-01-27",
      "risk_score": 87.5,
      "risk_level": "CRITICAL",
      "capacity_util_pct": 142.3,
      "overload_weight_t": 523.450,
      "urgent_failure_count": 8,
      "top_reasons": [
        {
          "code": "CAPACITY_OVERFLOW",
          "msg": "机组 H032 产能超限 42.3%",
          "weight": 0.45,
          "affected_count": 12
        },
        {
          "code": "ROLL_HARD_STOP",
          "msg": "机组 H033 强制换辊导致产能损失",
          "weight": 0.35,
          "affected_count": 5
        },
        {
          "code": "URGENT_L3_BLOCKED",
          "msg": "3 个 L3 紧急单无法完成",
          "weight": 0.20,
          "affected_count": 3
        }
      ],
      "involved_machines": ["H032", "H033"]
    }
  ],
  "total_count": 3
}
```

#### 错误码

- `NotFound`: 指定的 `version_id` 不存在
- `DataQualityError`: 读模型未刷新或数据不完整
- `ConfigInvalid`: 日期范围无效（如 `date_from > date_to`）

---

### 3.2 D4: get_machine_bottleneck_profile

**用例**: 回答"哪个机组哪天最堵"

#### 请求参数

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetMachineBottleneckProfileRequest {
    /// 方案版本 ID（必填）
    pub version_id: String,

    /// 日期范围起始（必填，ISO DATE）
    pub date_from: String,

    /// 日期范围结束（必填，ISO DATE）
    pub date_to: String,

    /// 机组代码过滤（可选，为空表示所有机组）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub machine_codes: Option<Vec<String>>,

    /// 堵塞等级过滤（可选）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bottleneck_level_filter: Option<Vec<String>>,  // ["LOW", "MEDIUM", "HIGH", "CRITICAL"]

    /// 堵塞类型过滤（可选）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bottleneck_type_filter: Option<Vec<String>>,  // ["Capacity", "Structure", "RollChange", "ColdStock", "Mixed"]

    /// 返回条数限制（可选，默认 50）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit: Option<u32>,
}
```

#### 响应结构

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MachineBottleneckProfileResponse {
    /// 方案版本 ID
    pub version_id: String,

    /// 查询时间戳
    pub as_of: String,

    /// 堵塞点列表
    pub items: Vec<BottleneckPoint>,

    /// 总记录数
    pub total_count: u32,

    /// 热力图统计（可选，用于前端渲染）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub heatmap_stats: Option<HeatmapStats>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BottleneckPoint {
    /// 机组代码
    pub machine_code: String,

    /// 计划日期
    pub plan_date: String,

    /// 堵塞分数 (0-100)
    pub bottleneck_score: f64,

    /// 堵塞等级
    pub bottleneck_level: String,  // "LOW" | "MEDIUM" | "HIGH" | "CRITICAL"

    /// 堵塞类型列表
    pub bottleneck_types: Vec<String>,

    /// 产能利用率 (%)
    pub capacity_util_pct: f64,

    /// 待排产材料数量
    pub pending_material_count: u32,

    /// 待排产重量 (t)
    pub pending_weight_t: f64,

    /// 堵塞原因（按影响降序）
    pub reasons: Vec<ReasonItem>,

    /// 推荐操作（可选）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub recommended_actions: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeatmapStats {
    /// 平均堵塞分数
    pub avg_score: f64,

    /// 最大堵塞分数
    pub max_score: f64,

    /// 堵塞天数（分数 > 50）
    pub bottleneck_days_count: u32,

    /// 按机组的统计
    pub by_machine: Vec<MachineStats>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MachineStats {
    pub machine_code: String,
    pub avg_score: f64,
    pub max_score: f64,
    pub bottleneck_days: u32,
}
```

#### 示例

**请求**:
```json
{
  "version_id": "V20260123-001",
  "date_from": "2026-01-24",
  "date_to": "2026-01-30",
  "machine_codes": ["H032", "H033"],
  "bottleneck_level_filter": ["HIGH", "CRITICAL"],
  "limit": 10
}
```

**响应**:
```json
{
  "version_id": "V20260123-001",
  "as_of": "2026-01-23T14:30:00.000Z",
  "items": [
    {
      "machine_code": "H032",
      "plan_date": "2026-01-27",
      "bottleneck_score": 92.3,
      "bottleneck_level": "CRITICAL",
      "bottleneck_types": ["Capacity", "Structure"],
      "capacity_util_pct": 142.3,
      "pending_material_count": 28,
      "pending_weight_t": 1523.450,
      "reasons": [
        {
          "code": "CAPACITY_OVERFLOW",
          "msg": "产能池超限 523.45t，利用率 142.3%",
          "weight": 0.60,
          "affected_count": 18
        },
        {
          "code": "STRUCTURE_CONFLICT",
          "msg": "结构矛盾导致 10 个材料无法排入",
          "weight": 0.40,
          "affected_count": 10
        }
      ],
      "recommended_actions": [
        "调整产能池上限",
        "将部分材料转移至其他机组",
        "优先处理结构冲突材料"
      ]
    }
  ],
  "total_count": 5,
  "heatmap_stats": {
    "avg_score": 67.5,
    "max_score": 92.3,
    "bottleneck_days_count": 5,
    "by_machine": [
      {
        "machine_code": "H032",
        "avg_score": 75.2,
        "max_score": 92.3,
        "bottleneck_days": 4
      },
      {
        "machine_code": "H033",
        "avg_score": 59.8,
        "max_score": 78.1,
        "bottleneck_days": 2
      }
    ]
  }
}
```

#### 错误码

- `NotFound`: 指定的 `version_id` 不存在
- `DataQualityError`: 读模型未刷新或数据不完整
- `ConfigInvalid`: 日期范围或机组代码无效

---

### 3.3 D2: list_order_failure_set

**用例**: 回答"哪些紧急订单无法按期完成"

#### 请求参数

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListOrderFailureSetRequest {
    /// 方案版本 ID（必填）
    pub version_id: String,

    /// 失败类型过滤（可选）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fail_type_filter: Option<Vec<String>>,  // ["Overdue", "NearDueImpossible", "CapacityShortage", ...]

    /// 紧急等级过滤（可选）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub urgency_level_filter: Option<Vec<String>>,  // ["L0", "L1", "L2", "L3"]

    /// 机组代码过滤（可选）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub machine_codes: Option<Vec<String>>,

    /// 交货日期范围起始（可选）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub due_date_from: Option<String>,

    /// 交货日期范围结束（可选）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub due_date_to: Option<String>,

    /// 完成率阈值过滤（可选，例如 0.5 表示只显示完成率 < 50% 的订单）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub completion_rate_threshold: Option<f64>,

    /// 分页：限制条数（可选，默认 50）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit: Option<u32>,

    /// 分页：偏移量（可选，默认 0）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub offset: Option<u32>,
}
```

#### 响应结构

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderFailureSetResponse {
    /// 方案版本 ID
    pub version_id: String,

    /// 查询时间戳
    pub as_of: String,

    /// 失败订单列表
    pub items: Vec<OrderFailure>,

    /// 总记录数
    pub total_count: u32,

    /// 统计摘要
    pub summary: OrderFailureSummary,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderFailure {
    /// 合同号
    pub contract_no: String,

    /// 交货日期
    pub due_date: String,

    /// 距交货天数（负数表示已逾期）
    pub days_to_due: i32,

    /// 紧急等级
    pub urgency_level: String,  // "L0" | "L1" | "L2" | "L3"

    /// 失败类型
    pub fail_type: String,  // "Overdue" | "NearDueImpossible" | "CapacityShortage" | ...

    /// 完成率 (0-1)
    pub completion_rate: f64,

    /// 总重量 (t)
    pub total_weight_t: f64,

    /// 已排产重量 (t)
    pub scheduled_weight_t: f64,

    /// 未排产重量 (t)
    pub unscheduled_weight_t: f64,

    /// 机组代码
    pub machine_code: String,

    /// 阻塞因素（按影响降序）
    pub blocking_factors: Vec<BlockingFactor>,

    /// 失败原因
    pub failure_reasons: Vec<String>,

    /// 推荐操作（可选）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub recommended_actions: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockingFactor {
    /// 因素类型
    pub factor_type: String,  // "CapacityLimit" | "StructureMismatch" | "ColdStock" | ...

    /// 因素描述
    pub description: String,

    /// 影响程度 (0-1)
    pub impact: f64,

    /// 涉及的材料数量
    pub affected_material_count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderFailureSummary {
    /// 总失败订单数
    pub total_failures: u32,

    /// 按失败类型统计
    pub by_fail_type: Vec<TypeCount>,

    /// 按紧急等级统计
    pub by_urgency: Vec<TypeCount>,

    /// 总未完成重量 (t)
    pub total_unscheduled_weight_t: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TypeCount {
    pub type_name: String,
    pub count: u32,
    pub weight_t: f64,
}
```

#### 示例

**请求**:
```json
{
  "version_id": "V20260123-001",
  "urgency_level_filter": ["L3"],
  "completion_rate_threshold": 0.8,
  "limit": 20
}
```

**响应**:
```json
{
  "version_id": "V20260123-001",
  "as_of": "2026-01-23T14:30:00.000Z",
  "items": [
    {
      "contract_no": "CTR-2026-001234",
      "due_date": "2026-01-25",
      "days_to_due": 2,
      "urgency_level": "L3",
      "fail_type": "NearDueImpossible",
      "completion_rate": 0.35,
      "total_weight_t": 523.450,
      "scheduled_weight_t": 183.200,
      "unscheduled_weight_t": 340.250,
      "machine_code": "H032",
      "blocking_factors": [
        {
          "factor_type": "CapacityLimit",
          "description": "未来 2 天产能池已满，无法插入",
          "impact": 0.70,
          "affected_material_count": 8
        },
        {
          "factor_type": "ColdStock",
          "description": "5 个材料尚未适温，无法排入",
          "impact": 0.30,
          "affected_material_count": 5
        }
      ],
      "failure_reasons": [
        "产能不足",
        "部分材料未适温",
        "交货日期紧迫"
      ],
      "recommended_actions": [
        "紧急扩容产能池",
        "人工红线强制插入",
        "联系客户协商延期"
      ]
    }
  ],
  "total_count": 12,
  "summary": {
    "total_failures": 12,
    "by_fail_type": [
      {"type_name": "NearDueImpossible", "count": 8, "weight_t": 2340.500},
      {"type_name": "CapacityShortage", "count": 4, "weight_t": 1250.300}
    ],
    "by_urgency": [
      {"type_name": "L3", "count": 12, "weight_t": 3590.800}
    ],
    "total_unscheduled_weight_t": 3590.800
  }
}
```

#### 错误码

- `NotFound`: 指定的 `version_id` 不存在
- `DataQualityError`: 读模型未刷新或数据不完整

---

### 3.4 D3: get_cold_stock_profile

**用例**: 回答"哪些冷料长期压库"

#### 请求参数

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetColdStockProfileRequest {
    /// 方案版本 ID（必填）
    pub version_id: String,

    /// 机组代码过滤（可选）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub machine_codes: Option<Vec<String>>,

    /// 压库等级过滤（可选）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pressure_level_filter: Option<Vec<String>>,  // ["LOW", "MEDIUM", "HIGH", "CRITICAL"]

    /// 年龄分桶过滤（可选）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub age_bin_filter: Option<Vec<String>>,  // ["0-7", "8-14", "15-30", "30+"]

    /// 返回条数限制（可选，默认 50）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit: Option<u32>,
}
```

#### 响应结构

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColdStockProfileResponse {
    /// 方案版本 ID
    pub version_id: String,

    /// 查询时间戳
    pub as_of: String,

    /// 冷料分桶列表
    pub items: Vec<ColdStockBucket>,

    /// 总记录数
    pub total_count: u32,

    /// 统计摘要
    pub summary: ColdStockSummary,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColdStockBucket {
    /// 机组代码
    pub machine_code: String,

    /// 年龄分桶
    pub age_bin: String,  // "0-7" | "8-14" | "15-30" | "30+"

    /// 材料数量
    pub count: u32,

    /// 总重量 (t)
    pub weight_t: f64,

    /// 压库分数 (0-100)
    pub pressure_score: f64,

    /// 压库等级
    pub pressure_level: String,  // "LOW" | "MEDIUM" | "HIGH" | "CRITICAL"

    /// 平均库龄（天）
    pub avg_age_days: f64,

    /// 最大库龄（天）
    pub max_age_days: i32,

    /// 结构缺口描述（为何无法释放）
    pub structure_gap: String,

    /// 压库原因（按影响降序）
    pub reasons: Vec<ReasonItem>,

    /// 趋势（可选）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub trend: Option<ColdStockTrend>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColdStockTrend {
    /// 趋势方向
    pub direction: String,  // "RISING" | "STABLE" | "FALLING"

    /// 变化率 (%)
    pub change_rate_pct: f64,

    /// 对比基准（天数）
    pub baseline_days: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColdStockSummary {
    /// 总冷料数量
    pub total_cold_stock_count: u32,

    /// 总冷料重量 (t)
    pub total_cold_stock_weight_t: f64,

    /// 平均库龄（天）
    pub avg_age_days: f64,

    /// 高压库数量（HIGH + CRITICAL）
    pub high_pressure_count: u32,

    /// 按机组统计
    pub by_machine: Vec<MachineStockStats>,

    /// 按年龄分桶统计
    pub by_age_bin: Vec<AgeBinStats>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MachineStockStats {
    pub machine_code: String,
    pub count: u32,
    pub weight_t: f64,
    pub avg_pressure_score: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgeBinStats {
    pub age_bin: String,
    pub count: u32,
    pub weight_t: f64,
}
```

#### 示例

**请求**:
```json
{
  "version_id": "V20260123-001",
  "machine_codes": ["H032"],
  "pressure_level_filter": ["HIGH", "CRITICAL"],
  "limit": 10
}
```

**响应**:
```json
{
  "version_id": "V20260123-001",
  "as_of": "2026-01-23T14:30:00.000Z",
  "items": [
    {
      "machine_code": "H032",
      "age_bin": "30+",
      "count": 15,
      "weight_t": 823.450,
      "pressure_score": 92.5,
      "pressure_level": "CRITICAL",
      "avg_age_days": 45.3,
      "max_age_days": 67,
      "structure_gap": "H032 在未来 14 天内无对应结构的产能窗口",
      "reasons": [
        {
          "code": "STRUCTURE_MISMATCH",
          "msg": "结构 S1234 在未来 14 天无可用产能",
          "weight": 0.60,
          "affected_count": 10
        },
        {
          "code": "LOW_PRIORITY",
          "msg": "紧急等级 L0，持续被高优先级材料挤占",
          "weight": 0.40,
          "affected_count": 5
        }
      ],
      "trend": {
        "direction": "RISING",
        "change_rate_pct": 12.5,
        "baseline_days": 7
      }
    }
  ],
  "total_count": 8,
  "summary": {
    "total_cold_stock_count": 68,
    "total_cold_stock_weight_t": 3250.800,
    "avg_age_days": 28.5,
    "high_pressure_count": 23,
    "by_machine": [
      {"machine_code": "H032", "count": 35, "weight_t": 1650.400, "avg_pressure_score": 65.2},
      {"machine_code": "H033", "count": 33, "weight_t": 1600.400, "avg_pressure_score": 58.7}
    ],
    "by_age_bin": [
      {"age_bin": "0-7", "count": 10, "weight_t": 450.200},
      {"age_bin": "8-14", "count": 18, "weight_t": 890.300},
      {"age_bin": "15-30", "count": 25, "weight_t": 1087.850},
      {"age_bin": "30+", "count": 15, "weight_t": 822.450}
    ]
  }
}
```

#### 错误码

- `NotFound`: 指定的 `version_id` 不存在
- `DataQualityError`: 读模型未刷新或数据不完整

---

### 3.5 D5: list_roll_campaign_alerts

**用例**: 回答"换辊是否异常"

#### 请求参数

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListRollCampaignAlertsRequest {
    /// 方案版本 ID（必填）
    pub version_id: String,

    /// 机组代码过滤（可选）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub machine_codes: Option<Vec<String>>,

    /// 预警等级过滤（可选）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub alert_level_filter: Option<Vec<String>>,  // ["INFO", "WARNING", "CRITICAL"]

    /// 预警类型过滤（可选）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub alert_type_filter: Option<Vec<String>>,  // ["NearSoftLimit", "ExceedSoftLimit", "NearHardStop", "AtHardStop", ...]

    /// 日期范围起始（可选）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub date_from: Option<String>,

    /// 日期范围结束（可选）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub date_to: Option<String>,

    /// 返回条数限制（可选，默认 50）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit: Option<u32>,
}
```

#### 响应结构

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RollCampaignAlertsResponse {
    /// 方案版本 ID
    pub version_id: String,

    /// 查询时间戳
    pub as_of: String,

    /// 换辊预警列表
    pub items: Vec<RollAlert>,

    /// 总记录数
    pub total_count: u32,

    /// 统计摘要
    pub summary: RollAlertSummary,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RollAlert {
    /// 机组代码
    pub machine_code: String,

    /// 当前活跃换辊批次 ID
    pub campaign_id: String,

    /// 换辊批次开始日期
    pub campaign_start_date: String,

    /// 当前累计吨数 (t)
    pub current_tonnage_t: f64,

    /// 软建议吨数 (t)
    pub soft_limit_t: f64,

    /// 硬停机吨数 (t)
    pub hard_limit_t: f64,

    /// 剩余吨数（距硬停机）(t)
    pub remaining_tonnage_t: f64,

    /// 预警等级
    pub alert_level: String,  // "INFO" | "WARNING" | "CRITICAL"

    /// 预警类型
    pub alert_type: String,  // "NearSoftLimit" | "ExceedSoftLimit" | "NearHardStop" | "AtHardStop" | ...

    /// 预计触发硬停机日期（可选）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub estimated_hard_stop_date: Option<String>,

    /// 预警消息
    pub alert_message: String,

    /// 影响说明
    pub impact_description: String,

    /// 推荐操作
    pub recommended_actions: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RollAlertSummary {
    /// 总预警数
    pub total_alerts: u32,

    /// 按预警等级统计
    pub by_level: Vec<TypeCount>,

    /// 按预警类型统计
    pub by_type: Vec<TypeCount>,

    /// 临近硬停机的机组数量
    pub near_hard_stop_count: u32,
}
```

#### 示例

**请求**:
```json
{
  "version_id": "V20260123-001",
  "alert_level_filter": ["WARNING", "CRITICAL"],
  "limit": 10
}
```

**响应**:
```json
{
  "version_id": "V20260123-001",
  "as_of": "2026-01-23T14:30:00.000Z",
  "items": [
    {
      "machine_code": "H033",
      "campaign_id": "RC-H033-20260115",
      "campaign_start_date": "2026-01-15",
      "current_tonnage_t": 2380.500,
      "soft_limit_t": 1500.000,
      "hard_limit_t": 2500.000,
      "remaining_tonnage_t": 119.500,
      "alert_level": "CRITICAL",
      "alert_type": "NearHardStop",
      "estimated_hard_stop_date": "2026-01-24",
      "alert_message": "机组 H033 距离强制换辊仅剩 119.5t，预计明天触发硬停机",
      "impact_description": "如果触发硬停机，将导致当日产能损失约 500t，影响 15 个紧急订单",
      "recommended_actions": [
        "立即安排换辊作业",
        "紧急调整明日排产计划",
        "将部分材料转移至 H032"
      ]
    }
  ],
  "total_count": 3,
  "summary": {
    "total_alerts": 3,
    "by_level": [
      {"type_name": "CRITICAL", "count": 1, "weight_t": 0.0},
      {"type_name": "WARNING", "count": 2, "weight_t": 0.0}
    ],
    "by_type": [
      {"type_name": "NearHardStop", "count": 1, "weight_t": 0.0},
      {"type_name": "ExceedSoftLimit", "count": 2, "weight_t": 0.0}
    ],
    "near_hard_stop_count": 1
  }
}
```

#### 错误码

- `NotFound`: 指定的 `version_id` 不存在
- `DataQualityError`: 读模型未刷新或数据不完整

---

### 3.6 D6: get_capacity_opportunity

**用例**: 回答"是否存在产能优化空间"

#### 请求参数

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetCapacityOpportunityRequest {
    /// 方案版本 ID（必填）
    pub version_id: String,

    /// 机组代码过滤（可选）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub machine_codes: Option<Vec<String>>,

    /// 日期范围起始（可选）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub date_from: Option<String>,

    /// 日期范围结束（可选）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub date_to: Option<String>,

    /// 机会类型过滤（可选）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub opportunity_type_filter: Option<Vec<String>>,  // ["Underutilized", "Rebalance", "StructureOptimize", ...]

    /// 最小优化空间（吨，可选，默认 50t）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min_opportunity_t: Option<f64>,

    /// 返回条数限制（可选，默认 50）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit: Option<u32>,
}
```

#### 响应结构

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapacityOpportunityResponse {
    /// 方案版本 ID
    pub version_id: String,

    /// 查询时间戳
    pub as_of: String,

    /// 机会点列表
    pub items: Vec<CapacityOpportunity>,

    /// 总记录数
    pub total_count: u32,

    /// 统计摘要
    pub summary: CapacityOpportunitySummary,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapacityOpportunity {
    /// 机组代码
    pub machine_code: String,

    /// 计划日期
    pub plan_date: String,

    /// 机会类型
    pub opportunity_type: String,  // "Underutilized" | "Rebalance" | "StructureOptimize" | ...

    /// 当前产能利用率 (%)
    pub current_util_pct: f64,

    /// 目标产能 (t)
    pub target_capacity_t: f64,

    /// 已用产能 (t)
    pub used_capacity_t: f64,

    /// 可优化空间 (t)
    pub opportunity_space_t: f64,

    /// 优化后预计利用率 (%)
    pub optimized_util_pct: f64,

    /// 敏感性分析（可选）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sensitivity: Option<SensitivityAnalysis>,

    /// 机会描述
    pub description: String,

    /// 推荐操作
    pub recommended_actions: Vec<String>,

    /// 潜在收益
    pub potential_benefits: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SensitivityAnalysis {
    /// 场景列表（基于 dry_run_recalc）
    pub scenarios: Vec<Scenario>,

    /// 最佳场景索引
    pub best_scenario_index: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Scenario {
    /// 场景名称
    pub name: String,

    /// 调整说明
    pub adjustment: String,

    /// 调整后利用率 (%)
    pub util_pct: f64,

    /// 调整后风险分数
    pub risk_score: f64,

    /// 影响的材料数量
    pub affected_material_count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapacityOpportunitySummary {
    /// 总机会点数量
    pub total_opportunities: u32,

    /// 总可优化空间 (t)
    pub total_opportunity_space_t: f64,

    /// 按机会类型统计
    pub by_type: Vec<TypeCount>,

    /// 平均当前利用率 (%)
    pub avg_current_util_pct: f64,

    /// 平均优化后利用率 (%)
    pub avg_optimized_util_pct: f64,
}
```

#### 示例

**请求**:
```json
{
  "version_id": "V20260123-001",
  "machine_codes": ["H032"],
  "min_opportunity_t": 100.0,
  "limit": 5
}
```

**响应**:
```json
{
  "version_id": "V20260123-001",
  "as_of": "2026-01-23T14:30:00.000Z",
  "items": [
    {
      "machine_code": "H032",
      "plan_date": "2026-01-26",
      "opportunity_type": "Underutilized",
      "current_util_pct": 62.5,
      "target_capacity_t": 1500.000,
      "used_capacity_t": 937.500,
      "opportunity_space_t": 562.500,
      "optimized_util_pct": 95.0,
      "sensitivity": {
        "scenarios": [
          {
            "name": "适度填充",
            "adjustment": "从其他机组转移 200t 低优先级材料",
            "util_pct": 75.8,
            "risk_score": 25.3,
            "affected_material_count": 8
          },
          {
            "name": "激进填充",
            "adjustment": "从其他机组转移 500t 材料",
            "util_pct": 95.8,
            "risk_score": 42.1,
            "affected_material_count": 20
          }
        ],
        "best_scenario_index": 0
      },
      "description": "H032 在 2026-01-26 有 562.5t 空闲产能，可从 H033 转移部分材料平衡负载",
      "recommended_actions": [
        "从 H033 转移结构匹配的低优先级材料",
        "优先考虑冷料和 L0 材料",
        "避免转移 L2/L3 紧急材料"
      ],
      "potential_benefits": [
        "提升整体产能利用率 12.5%",
        "减轻 H033 堵塞压力",
        "加快冷料消化速度"
      ]
    }
  ],
  "total_count": 4,
  "summary": {
    "total_opportunities": 4,
    "total_opportunity_space_t": 1250.800,
    "by_type": [
      {"type_name": "Underutilized", "count": 3, "weight_t": 950.600},
      {"type_name": "Rebalance", "count": 1, "weight_t": 300.200}
    ],
    "avg_current_util_pct": 65.2,
    "avg_optimized_util_pct": 88.5
  }
}
```

#### 错误码

- `NotFound`: 指定的 `version_id` 不存在
- `DataQualityError`: 读模型未刷新或数据不完整

---

## 4. 错误码规范

### 4.1 通用错误码

继承自 `Tauri_API_Contract_v0.3_Integrated.md` 的错误码：

- `NotFound`: 资源不存在（如 `version_id` 不存在）
- `DataQualityError`: 读模型未刷新或数据不完整
- `ConfigInvalid`: 请求参数无效（如日期范围错误）
- `ConstraintViolation`: 违反业务约束（通常不会在只读查询中出现）

### 4.2 DecisionApi 专用错误码

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DecisionApiError {
    /// 读模型未刷新
    ReadModelStale {
        version_id: String,
        last_refreshed_at: Option<String>,
    },

    /// 读模型刷新失败
    ReadModelRefreshFailed {
        version_id: String,
        error_message: String,
    },

    /// 请求参数验证失败
    InvalidRequest {
        field: String,
        message: String,
    },

    /// 数据不完整（部分表未刷新）
    PartialData {
        missing_tables: Vec<String>,
    },
}
```

### 4.3 错误响应格式

```json
{
  "code": "DataQualityError",
  "message": "读模型数据不完整",
  "details": {
    "version_id": "V20260123-001",
    "missing_tables": ["decision_day_summary", "decision_machine_bottleneck"]
  }
}
```

---

## 5. 契约级约束

### 5.1 工程约束（继承自 Master Spec）

1. **只读查询**: DecisionApi 只读取读模型，不修改任何状态
2. **版本隔离**: 所有查询必须指定 `version_id`，确保版本隔离
3. **可解释性**: 所有结果必须包含 `reasons` 或 `top_reasons` 字段
4. **读模型依赖**: 所有查询依赖预计算的 `decision_*` 表
5. **无实时计算**: 禁止在查询时进行复杂计算，必须查询预计算结果

### 5.2 数据质量保证

1. **刷新状态检查**: 查询前检查 `decision_refresh_log` 表，确保读模型已刷新
2. **时间戳一致性**: 所有响应必须包含 `as_of` 时间戳，表明数据快照时间
3. **部分数据处理**: 如果部分表未刷新，返回 `DataQualityError` 而非部分结果

### 5.3 性能约束

1. **查询超时**: 所有查询必须在 2 秒内完成（单机 SQLite）
2. **分页支持**: 大结果集必须支持 `limit` 和 `offset` 分页
3. **索引依赖**: 所有查询必须利用表索引，避免全表扫描

### 5.4 兼容性约束

1. **向后兼容**: DTO 结构演进时，使用 `#[serde(skip_serializing_if = "Option::is_none")]` 处理可选字段
2. **版本化**: 如需破坏性变更，创建新的 API 版本（如 `v2/get_decision_day_summary`）
3. **降级机制**: DecisionApi 异常时，前端可降级回传统 API（如 `get_risk_snapshot`）

---

## 6. 实现建议

### 6.1 推荐目录结构

```
src/decision/
├── api/
│   ├── mod.rs
│   ├── decision_api.rs       # DecisionApi trait 定义
│   ├── decision_api_impl.rs  # DecisionApi 实现
│   └── dto.rs                # 所有 DTO 结构体
├── repository/
│   ├── day_summary_repo.rs
│   ├── bottleneck_repo.rs
│   ├── order_failure_repo.rs
│   ├── cold_stock_repo.rs
│   ├── roll_alert_repo.rs
│   └── capacity_opportunity_repo.rs
└── ...
```

### 6.2 Tauri Command 映射

在 `src/app/tauri_commands.rs` 中添加：

```rust
use crate::decision::api::{DecisionApi, DecisionApiImpl};

#[tauri::command]
pub async fn get_decision_day_summary(
    state: State<'_, AppState>,
    request: GetDecisionDaySummaryRequest,
) -> Result<DecisionDaySummaryResponse, String> {
    let decision_api = DecisionApiImpl::new(state.db_pool.clone());
    decision_api.get_decision_day_summary(request)
        .map_err(|e| format!("{:?}", e))
}

// ... 其他 5 个命令类似
```

### 6.3 前端调用示例（TypeScript）

```typescript
import { invoke } from '@tauri-apps/api/tauri';

interface GetDecisionDaySummaryRequest {
  version_id: string;
  date_from: string;
  date_to: string;
  risk_level_filter?: string[];
  limit?: number;
  sort_by?: string;
}

async function getMostRiskyDays(versionId: string, dateRange: [string, string]) {
  const response = await invoke<DecisionDaySummaryResponse>(
    'get_decision_day_summary',
    {
      request: {
        version_id: versionId,
        date_from: dateRange[0],
        date_to: dateRange[1],
        risk_level_filter: ['HIGH', 'CRITICAL'],
        limit: 5,
        sort_by: 'risk_score',
      }
    }
  );
  return response.items;
}
```

---

## 7. 测试要求

### 7.1 单元测试

每个 API 命令必须包含以下测试：

1. **正常路径**: 标准查询返回正确结果
2. **边界条件**: 空结果集、单条记录、大结果集
3. **参数验证**: 无效 `version_id`、无效日期范围
4. **数据质量**: 读模型未刷新时返回 `DataQualityError`
5. **过滤器**: 各种过滤条件组合

### 7.2 集成测试

1. **端到端流程**: 导入 → 重算 → 刷新读模型 → DecisionApi 查询
2. **版本隔离**: 多版本并行查询互不干扰
3. **性能基准**: 查询耗时 < 2 秒（单机 SQLite）

### 7.3 契约测试

使用 JSON Schema 或 Pact 验证：

1. 请求结构符合 DTO 定义
2. 响应结构符合 DTO 定义
3. 错误响应符合错误码规范

---

## 8. 版本演进策略

### 8.1 v1.0（当前版本）

- 定义 6 个核心 API 命令
- 基于预计算的读模型表
- 只读查询，无副作用

### 8.2 v1.1（潜在扩展）

- 添加订阅机制（WebSocket 推送读模型更新）
- 添加导出功能（导出决策报告为 Excel/PDF）
- 添加缓存层（Redis）用于高频查询

### 8.3 v2.0（破坏性变更）

- 如需重大重构，创建新版本 API
- 保留 v1.0 兼容性至少 6 个月
- 提供迁移指南

---

## 9. 附录

### 9.1 与现有 API 的关系

| 现有 API | DecisionApi 对应 | 差异 |
|---------|------------------|------|
| `get_risk_snapshot` | `get_decision_day_summary` | DecisionApi 聚合多机组风险，提供解释 |
| 无对应 | `get_machine_bottleneck_profile` | 新增机组堵塞分析 |
| 无对应 | `list_order_failure_set` | 新增订单失败分析 |
| 无对应 | `get_cold_stock_profile` | 新增冷料压库分析 |
| 无对应 | `list_roll_campaign_alerts` | 新增换辊预警 |
| `dry_run_recalc` | `get_capacity_opportunity` | DecisionApi 提供结构化机会分析 |

### 9.2 术语表

- **DecisionApi**: 决策支持层的 API 契约
- **读模型 (Read Model)**: 预计算的决策视图表（`decision_*`）
- **CQRS**: Command Query Responsibility Segregation，命令查询职责分离
- **版本隔离**: 每个 `plan_version` 拥有独立的读模型快照
- **可解释性**: 决策结果必须包含原因字段（`reasons`/`top_reasons`）

### 9.3 参考文档

- `spec/Claude_Dev_Master_Spec.md` - 系统宪法
- `spec/Tauri_API_Contract_v0.3_Integrated.md` - 现有 API 契约
- `REFACTOR_PLAN_v1.0.md` - 决策结构重构方案
- `DECISION_READ_MODELS.md` - 读模型设计文档
- `migrations/20260123_create_decision_read_models.sql` - 读模型 schema

---

## 10. 变更记录

| 版本 | 日期 | 变更内容 | 作者 |
|------|------|----------|------|
| v1.0 | 2026-01-23 | 初始版本，定义 6 个核心 DecisionApi 命令 | Claude |

---

**文档结束**
