# 系统架构文档

**版本**: v1.0
**更新日期**: 2026-01-27

---

## 一、整体架构

```
┌─────────────────────────────────────────────────────────────────┐
│                      Tauri 桌面应用                              │
│  ┌─────────────────────┐     ┌───────────────────────────────┐  │
│  │    React 前端        │ IPC │       Rust 后端               │  │
│  │                     │ ←→  │                               │  │
│  │  • 23 个组件        │     │  • 16 个引擎                  │  │
│  │  • 13 个路由        │     │  • 12 个仓储                  │  │
│  │  • Zustand 状态     │     │  • 7 个 API                   │  │
│  │  • TanStack Query   │     │  • SQLite 数据库              │  │
│  └─────────────────────┘     └───────────────────────────────┘  │
└─────────────────────────────────────────────────────────────────┘
```

---

## 二、后端分层架构

```
┌─────────────────────────────────────┐
│     Tauri 命令层 (53 个命令)        │
├─────────────────────────────────────┤
│     AppState (依赖注入容器)         │
├─────────────────────────────────────┤
│     API 层 (7 个 API 实现)          │
│  • MaterialApi  • PlanApi           │
│  • DashboardApi • ConfigApi         │
│  • RollerApi    • ImportApi         │
│  • DecisionApi                      │
├─────────────────────────────────────┤
│     Engine 层 (16 个业务引擎)       │
│  • EligibilityEngine (适温准入)     │
│  • UrgencyEngine (紧急等级)         │
│  • PrioritySorter (优先级排序)      │
│  • CapacityFiller (产能填充)        │
│  • RecalcEngine (重算/联动)         │
│  • RiskEngine (风险计算)            │
│  • ImpactSummaryEngine (影响摘要)   │
│  • ...                              │
├─────────────────────────────────────┤
│     Repository 层 (12 个数据仓储)   │
│  • MaterialMasterRepo               │
│  • MaterialStateRepo                │
│  • PlanRepo / PlanVersionRepo       │
│  • CapacityPoolRepo                 │
│  • ...                              │
├─────────────────────────────────────┤
│     Domain 层 (领域模型)            │
│  • Material • Plan • Capacity       │
│  • Risk • Roller • Types            │
├─────────────────────────────────────┤
│     SQLite 数据库                   │
└─────────────────────────────────────┘
```

---

## 三、前端架构

```
┌─────────────────────────────────────┐
│           App.tsx (根组件)          │
├─────────────────────────────────────┤
│     React Router (13 个路由)        │
├─────────────────────────────────────┤
│     页面组件                        │
│  • Dashboard (驾驶舱)               │
│  • DecisionBoard (D1-D6)            │
│  • MaterialManagement               │
│  • PlanManagement                   │
│  • ConfigManagement                 │
├─────────────────────────────────────┤
│     状态管理                        │
│  • Zustand (全局状态)               │
│  • TanStack Query (服务端缓存)      │
├─────────────────────────────────────┤
│     IPC 通信层                      │
│  • tauri.ts (API 封装)              │
│  • ipcClient.tsx (错误处理/重试)    │
│  • eventBus.ts (事件监听)           │
├─────────────────────────────────────┤
│     Tauri IPC                       │
└─────────────────────────────────────┘
```

---

## 四、决策支持层 (D1-D6)

```
┌─────────────────────────────────────────────────────────────┐
│                    DecisionApi                               │
├─────────────────────────────────────────────────────────────┤
│  D1: get_decision_day_summary      → 哪天最危险             │
│  D2: list_order_failure_set        → 哪些紧急单无法完成     │
│  D3: get_cold_stock_profile        → 哪些冷料压库           │
│  D4: get_machine_bottleneck_profile→ 哪个机组最堵           │
│  D5: list_roll_campaign_alerts     → 换辊是否异常           │
│  D6: get_capacity_opportunity      → 是否存在产能优化空间   │
└─────────────────────────────────────────────────────────────┘
                              ↓
┌─────────────────────────────────────────────────────────────┐
│                    决策读模型表                              │
│  • day_summaries      • order_failures                      │
│  • cold_stocks        • bottlenecks                         │
│  • roll_alerts        • capacity_opportunities              │
└─────────────────────────────────────────────────────────────┘
```

---

## 五、数据流向

### 5.1 排产计算流程

```
材料导入 → 适温判定 → 紧急等级计算 → 优先级排序
    ↓
产能池填充 → 排产方案生成 → 风险计算 → 决策视图刷新
```

### 5.2 前端数据获取

```
Component → TanStack Query → Decision Service → IPC Client → Tauri → Rust API
```

---

## 六、工业约束实现

| 红线 | 实现位置 |
|------|---------|
| 冻结区保护 | `engine/recalc.rs`, `api/validator.rs` |
| 适温约束 | `engine/eligibility.rs`, `engine/capacity_filler.rs` |
| 分层紧急 | `engine/urgency.rs`, `domain/types.rs` |
| 产能优先 | `engine/capacity_filler.rs` |
| 可解释性 | 全层级 (reason 字段) |

---

## 七、关键配置

### 7.1 适温配置

```rust
winter_min_temp_days: 3  // 冬季适温天数
summer_min_temp_days: 4  // 夏季适温天数
machine_offset_days: 4   // 机组偏移天数
```

### 7.2 紧急等级阈值

```rust
n1_days: [2, 3, 5]      // L2 阈值
n2_days: [7, 10, 14]    // L1 阈值
```

### 7.3 产能配置

```rust
suggested_threshold: 1500  // 建议换辊阈值 (吨)
hard_limit: 2500          // 强制上限 (吨)
```
