# API 快速参考

**版本**: v1.0
**更新日期**: 2026-01-27

---

## Tauri 命令总览

共 **53 个** Tauri 命令，分为 8 大模块。

---

## 一、材料管理 (7 个命令)

| 命令 | 说明 |
|-----|------|
| `list_materials` | 查询材料列表 |
| `get_material_detail` | 获取材料详情 |
| `list_ready_materials` | 查询适温待排材料 |
| `batch_lock_materials` | 批量锁定材料 |
| `batch_force_release` | 批量强制放行 |
| `batch_set_urgent` | 批量设置紧急等级 |
| `list_materials_by_urgent_level` | 按紧急等级查询 |

---

## 二、排产方案 (15 个命令)

| 命令 | 说明 |
|-----|------|
| `create_plan` | 创建排产方案 |
| `list_plans` | 查询方案列表 |
| `get_plan_detail` | 获取方案详情 |
| `delete_plan` | 删除方案 |
| `create_plan_version` | 创建方案版本 |
| `list_plan_versions` | 查询版本列表 |
| `activate_version` | 激活版本 |
| `get_active_version` | 获取当前激活版本 |
| `recalc_full` | 全量重算 |
| `recalc_partial` | 局部重算 |
| `simulate_recalc` | 模拟重算 (不保存) |
| `move_items` | 移动计划项 |
| `batch_move_items` | 批量移动 |
| `compare_versions` | 版本对比 |
| `get_impact_summary` | 获取影响摘要 |

---

## 三、决策支持 (6 个命令)

| 命令 | 说明 | 对应决策 |
|-----|------|---------|
| `get_decision_day_summary` | 日期风险摘要 | D1 |
| `list_order_failure_set` | 订单失败列表 | D2 |
| `get_cold_stock_profile` | 冷料压库分析 | D3 |
| `get_machine_bottleneck_profile` | 机组堵塞分析 | D4 |
| `list_roll_campaign_alerts` | 换辊预警 | D5 |
| `get_capacity_opportunity` | 产能优化空间 | D6 |

---

## 四、驾驶舱 (9 个命令)

| 命令 | 说明 |
|-----|------|
| `list_risk_snapshots` | 风险快照列表 |
| `get_most_risky_date` | 最危险日期 |
| `get_unsatisfied_urgent_materials` | 未满足的紧急材料 |
| `get_cold_stock_materials` | 冷料压库列表 |
| `get_most_congested_machine` | 最拥挤机组 |
| `list_action_logs` | 操作日志 |
| `get_global_kpi` | 全局 KPI |
| `get_dashboard_summary` | 驾驶舱摘要 |
| `refresh_dashboard` | 刷新驾驶舱 |

---

## 五、配置管理 (6 个命令)

| 命令 | 说明 |
|-----|------|
| `list_configs` | 配置列表 |
| `get_config` | 获取单个配置 |
| `update_config` | 更新配置 |
| `batch_update_configs` | 批量更新 |
| `get_config_snapshot` | 配置快照 |
| `restore_config_from_snapshot` | 恢复配置 |

---

## 六、换辊管理 (5 个命令)

| 命令 | 说明 |
|-----|------|
| `list_roll_campaigns` | 换辊活动列表 |
| `get_active_roll_campaign` | 当前换辊活动 |
| `list_needs_roll_change` | 需要换辊的机组 |
| `create_roll_campaign` | 创建换辊活动 |
| `close_roll_campaign` | 关闭换辊活动 |

---

## 七、数据导入 (3 个命令)

| 命令 | 说明 |
|-----|------|
| `import_materials` | 导入材料数据 |
| `list_import_conflicts` | 导入冲突列表 |
| `resolve_import_conflict` | 处理导入冲突 |

---

## 八、产能管理 (2 个命令)

| 命令 | 说明 |
|-----|------|
| `get_capacity_pools` | 查询产能池 |
| `update_capacity_pool` | 更新产能池 |

---

## 常用调用示例

### 前端调用 (TypeScript)

```typescript
import { invoke } from '@tauri-apps/api/tauri';

// 查询材料列表
const materials = await invoke('list_materials', {
  machineCode: 'H032',
  limit: 50,
  offset: 0
});

// 获取决策日摘要
const summary = await invoke('get_decision_day_summary', {
  versionId: 'v001',
  dateFrom: '2026-01-01',
  dateTo: '2026-01-31'
});
```

---

## 错误码

| 错误码 | 说明 |
|-------|------|
| `FROZEN_ZONE_PROTECTION` | 违反冻结区保护 |
| `MATURITY_CONSTRAINT_VIOLATION` | 违反适温约束 |
| `CAPACITY_CONSTRAINT_VIOLATION` | 违反产能约束 |
| `OPTIMISTIC_LOCK_FAILURE` | 并发冲突 |
| `NOT_FOUND` | 资源不存在 |
| `VALIDATION_ERROR` | 参数验证失败 |

---

详细 API 规范请参考: [spec/Tauri_API_Contract_v0.3_Integrated.md](../spec/Tauri_API_Contract_v0.3_Integrated.md)
