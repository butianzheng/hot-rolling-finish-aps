# 宽厚路径规则（v0.6）编码开发计划

> **版本**: v0.6
> **依据规范**: spec/Engine_Specs_v0.3_Integrated.md 章节 14-18
> **状态**: 待实施

---

## 一、实施概览

### 1.1 功能范围

| 功能模块 | 说明 |
|----------|------|
| PathRuleEngine | 宽厚路径规则引擎，判定材料是否满足"由宽到窄、由厚到薄"约束 |
| AnchorResolver | 锚点解析器，按优先级解析当前换辊周期的路径锚点 |
| RollCycle 重置 | 换辊时重置锚点与累计状态 |
| 人工确认突破 | 高紧急度(L2/L3)材料违规时允许人工确认突破 |
| S2 种子策略 | 无冻结/锁定材料时，使用统计方法生成初始锚点 |

### 1.2 已完成项 ✅

| 文件 | 内容 |
|------|------|
| `src/domain/types.rs` | AnchorSource, PathViolationType, PathRuleStatus 枚举 |
| `src/domain/roller.rs` | RollerCampaign 锚点字段及方法 |
| `src/domain/action_log.rs` | PathOverrideConfirm, RollCycleReset ActionType |
| `migrations/v0.6_path_rule_extension.sql` | 数据库迁移脚本 |

### 1.3 待实施项 📋

| 层级 | 模块 | 优先级 |
|------|------|--------|
| Engine | PathRuleEngine | P0 |
| Engine | AnchorResolver | P0 |
| Engine | CapacityFiller 集成 | P0 |
| Repository | roller_repo 扩展 | P1 |
| Repository | material_repo 扩展 | P1 |
| API | path_rule_api.rs | P1 |
| API | Tauri commands 扩展 | P1 |
| Frontend | PathOverrideConfirmModal | P2 |
| Frontend | 配置管理页面扩展 | P2 |
| Frontend | 换辊锚点状态展示 | P2 |
| Tests | 单元测试 | P0 |
| Tests | 集成测试 | P1 |

---

## 二、后端实施计划

### 2.1 Engine 层

#### 2.1.1 PathRuleEngine (P0)

**文件**: `src/engine/path_rule.rs`

**接口设计**:

```rust
// src/engine/path_rule.rs

use crate::domain::types::{PathRuleStatus, PathViolationType, UrgentLevel};

/// 路径规则检查结果
#[derive(Debug, Clone)]
pub struct PathRuleResult {
    pub status: PathRuleStatus,
    pub violation_type: Option<PathViolationType>,
    pub width_delta_mm: f64,
    pub thickness_delta_mm: f64,
}

/// 路径规则配置
#[derive(Debug, Clone)]
pub struct PathRuleConfig {
    pub enabled: bool,
    pub width_tolerance_mm: f64,
    pub thickness_tolerance_mm: f64,
    pub override_allowed_urgency_levels: Vec<UrgentLevel>,
}

impl Default for PathRuleConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            width_tolerance_mm: 50.0,
            thickness_tolerance_mm: 1.0,
            override_allowed_urgency_levels: vec![UrgentLevel::L2, UrgentLevel::L3],
        }
    }
}

/// 锚点状态
#[derive(Debug, Clone)]
pub struct Anchor {
    pub width_mm: f64,
    pub thickness_mm: f64,
}

/// PathRuleEngine - 宽厚路径规则引擎
pub struct PathRuleEngine {
    config: PathRuleConfig,
}

impl PathRuleEngine {
    pub fn new(config: PathRuleConfig) -> Self {
        Self { config }
    }

    /// 检查材料是否满足路径约束
    ///
    /// # 参数
    /// - `candidate_width_mm`: 候选材料宽度
    /// - `candidate_thickness_mm`: 候选材料厚度
    /// - `candidate_urgent_level`: 候选材料紧急等级
    /// - `anchor`: 当前锚点（None 表示无锚点，跳过检查）
    /// - `user_confirmed`: 是否已人工确认
    ///
    /// # 返回
    /// PathRuleResult
    pub fn check(
        &self,
        candidate_width_mm: f64,
        candidate_thickness_mm: f64,
        candidate_urgent_level: UrgentLevel,
        anchor: Option<&Anchor>,
        user_confirmed: bool,
    ) -> PathRuleResult {
        // 未启用路径规则，直接返回 OK
        if !self.config.enabled {
            return PathRuleResult {
                status: PathRuleStatus::Ok,
                violation_type: None,
                width_delta_mm: 0.0,
                thickness_delta_mm: 0.0,
            };
        }

        // 无锚点，直接返回 OK（首块材料）
        let anchor = match anchor {
            Some(a) => a,
            None => {
                return PathRuleResult {
                    status: PathRuleStatus::Ok,
                    violation_type: None,
                    width_delta_mm: 0.0,
                    thickness_delta_mm: 0.0,
                };
            }
        };

        // 计算超限量
        let width_delta = candidate_width_mm - anchor.width_mm - self.config.width_tolerance_mm;
        let thickness_delta = candidate_thickness_mm - anchor.thickness_mm - self.config.thickness_tolerance_mm;

        let width_exceeded = width_delta > 0.0;
        let thickness_exceeded = thickness_delta > 0.0;

        // 无违规
        if !width_exceeded && !thickness_exceeded {
            return PathRuleResult {
                status: PathRuleStatus::Ok,
                violation_type: None,
                width_delta_mm: 0.0,
                thickness_delta_mm: 0.0,
            };
        }

        // 判断违规类型
        let violation_type = if width_exceeded && thickness_exceeded {
            PathViolationType::BothExceeded
        } else if width_exceeded {
            PathViolationType::WidthExceeded
        } else {
            PathViolationType::ThicknessExceeded
        };

        // 已人工确认，返回 OK（带违规标记）
        if user_confirmed {
            return PathRuleResult {
                status: PathRuleStatus::Ok,
                violation_type: Some(violation_type),
                width_delta_mm: width_delta.max(0.0),
                thickness_delta_mm: thickness_delta.max(0.0),
            };
        }

        // 判断是否允许人工突破
        let override_allowed = self.config.override_allowed_urgency_levels.contains(&candidate_urgent_level);

        let status = if override_allowed {
            PathRuleStatus::OverrideRequired
        } else {
            PathRuleStatus::HardViolation
        };

        PathRuleResult {
            status,
            violation_type: Some(violation_type),
            width_delta_mm: width_delta.max(0.0),
            thickness_delta_mm: thickness_delta.max(0.0),
        }
    }
}
```

**单元测试要点**:
- 无锚点时返回 OK
- 满足约束时返回 OK
- 宽度超限时返回正确违规类型
- 厚度超限时返回正确违规类型
- 双超限时返回 BOTH_EXCEEDED
- L0/L1 超限返回 HARD_VIOLATION
- L2/L3 超限返回 OVERRIDE_REQUIRED
- 已确认材料返回 OK（带违规标记）
- 禁用规则时直接返回 OK

---

#### 2.1.2 AnchorResolver (P0)

**文件**: `src/engine/anchor_resolver.rs`

**接口设计**:

```rust
// src/engine/anchor_resolver.rs

use crate::domain::types::AnchorSource;
use crate::engine::path_rule::Anchor;

/// 锚点解析结果
#[derive(Debug, Clone)]
pub struct ResolvedAnchor {
    pub source: AnchorSource,
    pub material_id: Option<String>,
    pub anchor: Option<Anchor>,
}

/// 候选材料摘要（用于锚点解析）
#[derive(Debug, Clone)]
pub struct MaterialSummary {
    pub material_id: String,
    pub width_mm: f64,
    pub thickness_mm: f64,
    pub seq_no: i32,
    pub user_confirmed_at: Option<String>,
}

/// S2 种子策略配置
#[derive(Debug, Clone)]
pub struct SeedS2Config {
    pub percentile: f64,           // 默认 0.95
    pub small_sample_threshold: i32, // 默认 10
}

impl Default for SeedS2Config {
    fn default() -> Self {
        Self {
            percentile: 0.95,
            small_sample_threshold: 10,
        }
    }
}

/// AnchorResolver - 锚点解析器
pub struct AnchorResolver {
    seed_config: SeedS2Config,
}

impl AnchorResolver {
    pub fn new(seed_config: SeedS2Config) -> Self {
        Self { seed_config }
    }

    /// 按优先级解析锚点
    ///
    /// 优先级: FROZEN_LAST -> LOCKED_LAST -> USER_CONFIRMED_LAST -> SEED_S2 -> NONE
    ///
    /// # 参数
    /// - `frozen_items`: 冻结区材料列表（按 seq_no 排序）
    /// - `locked_items`: 锁定区材料列表（按 seq_no 排序）
    /// - `user_confirmed_items`: 人工确认材料列表（按 user_confirmed_at 排序）
    /// - `candidates`: 候选材料列表（用于 S2 种子策略）
    ///
    /// # 返回
    /// ResolvedAnchor
    pub fn resolve(
        &self,
        frozen_items: &[MaterialSummary],
        locked_items: &[MaterialSummary],
        user_confirmed_items: &[MaterialSummary],
        candidates: &[MaterialSummary],
    ) -> ResolvedAnchor {
        // 1. 冻结区最后一块
        if let Some(last) = frozen_items.iter().max_by_key(|m| m.seq_no) {
            return ResolvedAnchor {
                source: AnchorSource::FrozenLast,
                material_id: Some(last.material_id.clone()),
                anchor: Some(Anchor {
                    width_mm: last.width_mm,
                    thickness_mm: last.thickness_mm,
                }),
            };
        }

        // 2. 锁定区最后一块
        if let Some(last) = locked_items.iter().max_by_key(|m| m.seq_no) {
            return ResolvedAnchor {
                source: AnchorSource::LockedLast,
                material_id: Some(last.material_id.clone()),
                anchor: Some(Anchor {
                    width_mm: last.width_mm,
                    thickness_mm: last.thickness_mm,
                }),
            };
        }

        // 3. 人工确认队列最后一块
        if let Some(last) = user_confirmed_items.iter()
            .filter(|m| m.user_confirmed_at.is_some())
            .max_by(|a, b| a.user_confirmed_at.cmp(&b.user_confirmed_at))
        {
            return ResolvedAnchor {
                source: AnchorSource::UserConfirmedLast,
                material_id: Some(last.material_id.clone()),
                anchor: Some(Anchor {
                    width_mm: last.width_mm,
                    thickness_mm: last.thickness_mm,
                }),
            };
        }

        // 4. S2 种子策略
        if !candidates.is_empty() {
            if let Some(anchor) = self.compute_seed_s2(candidates) {
                return ResolvedAnchor {
                    source: AnchorSource::SeedS2,
                    material_id: None,
                    anchor: Some(anchor),
                };
            }
        }

        // 5. 无锚点
        ResolvedAnchor {
            source: AnchorSource::None,
            material_id: None,
            anchor: None,
        }
    }

    /// S2 种子策略计算
    ///
    /// - 样本数 >= small_sample_threshold: 取 percentile 分位点
    /// - 样本数 < 阈值: 取 max
    fn compute_seed_s2(&self, candidates: &[MaterialSummary]) -> Option<Anchor> {
        let widths: Vec<f64> = candidates.iter()
            .map(|m| m.width_mm)
            .filter(|w| *w > 0.0)
            .collect();
        let thicknesses: Vec<f64> = candidates.iter()
            .map(|m| m.thickness_mm)
            .filter(|t| *t > 0.0)
            .collect();

        if widths.is_empty() || thicknesses.is_empty() {
            return None;
        }

        let anchor_width = self.compute_upper_bound(&widths);
        let anchor_thickness = self.compute_upper_bound(&thicknesses);

        Some(Anchor {
            width_mm: anchor_width,
            thickness_mm: anchor_thickness,
        })
    }

    /// 计算上界（分位点或 max）
    fn compute_upper_bound(&self, values: &[f64]) -> f64 {
        let mut sorted: Vec<f64> = values.to_vec();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

        if sorted.len() >= self.seed_config.small_sample_threshold as usize {
            // 大样本：取分位点
            let idx = ((sorted.len() as f64 * self.seed_config.percentile) as usize)
                .min(sorted.len() - 1);
            sorted[idx]
        } else {
            // 小样本：取 max
            *sorted.last().unwrap_or(&0.0)
        }
    }
}
```

**单元测试要点**:
- 冻结区优先级最高
- 锁定区次之
- 人工确认区再次
- S2 种子策略兜底
- 无候选时返回 NONE
- S2 大样本分位点计算
- S2 小样本 max 计算

---

#### 2.1.3 CapacityFiller 集成 (P0)

**文件**: `src/engine/capacity_filler.rs`

**修改要点**:

```rust
// 在 CapacityFiller 的 fill 方法中集成 PathRuleEngine

impl CapacityFiller {
    pub fn fill_with_path_rule(
        &self,
        capacity_pool: &mut CapacityPool,
        candidates: Vec<MaterialCandidate>,
        frozen_items: &[PlanItem],
        roll_cycle_state: &mut RollerCampaign,
        path_rule_engine: &PathRuleEngine,
        anchor_resolver: &AnchorResolver,
    ) -> FillResult {
        // 1. 解析初始锚点
        let resolved_anchor = anchor_resolver.resolve(
            &self.to_summary_list(frozen_items),
            &self.get_locked_items(),
            &self.get_user_confirmed_items(),
            &self.to_summary_list(&candidates),
        );

        // 更新 roll_cycle_state
        if let Some(ref anchor) = resolved_anchor.anchor {
            roll_cycle_state.update_anchor(
                resolved_anchor.material_id.clone(),
                anchor.width_mm,
                anchor.thickness_mm,
                resolved_anchor.source,
            );
        }

        let mut current_anchor = resolved_anchor.anchor;
        let mut fill_result = FillResult::default();

        // 2. 遍历候选材料
        for candidate in candidates {
            // 路径门控
            let path_result = path_rule_engine.check(
                candidate.width_mm,
                candidate.thickness_mm,
                candidate.urgent_level,
                current_anchor.as_ref(),
                candidate.user_confirmed,
            );

            match path_result.status {
                PathRuleStatus::HardViolation => {
                    fill_result.skipped.push(SkippedMaterial {
                        material_id: candidate.material_id.clone(),
                        reason: "PATH_HARD_VIOLATION".to_string(),
                        violation_type: path_result.violation_type,
                    });
                    continue;
                }
                PathRuleStatus::OverrideRequired => {
                    fill_result.pending_confirmation.push(PendingConfirmation {
                        material_id: candidate.material_id.clone(),
                        violation_type: path_result.violation_type.unwrap(),
                        width_delta_mm: path_result.width_delta_mm,
                        thickness_delta_mm: path_result.thickness_delta_mm,
                        anchor_width_mm: current_anchor.as_ref().map(|a| a.width_mm),
                        anchor_thickness_mm: current_anchor.as_ref().map(|a| a.thickness_mm),
                    });
                    continue;
                }
                PathRuleStatus::Ok => {
                    // 继续产能门控
                }
            }

            // 产能门控
            if !capacity_pool.can_add(candidate.weight_t) {
                fill_result.skipped.push(SkippedMaterial {
                    material_id: candidate.material_id.clone(),
                    reason: "CAPACITY_EXCEEDED".to_string(),
                    violation_type: None,
                });
                continue;
            }

            // 添加材料
            capacity_pool.add(candidate.clone());
            fill_result.filled.push(FilledMaterial {
                material_id: candidate.material_id.clone(),
                violation_flags: path_result.violation_type.map(|v| ViolationFlags {
                    path_violation: Some(PathViolationDetail {
                        violation_type: v,
                        user_confirmed: candidate.user_confirmed,
                    }),
                }),
            });

            // 更新锚点
            current_anchor = Some(Anchor {
                width_mm: candidate.width_mm,
                thickness_mm: candidate.thickness_mm,
            });
        }

        fill_result
    }
}
```

---

### 2.2 Repository 层

#### 2.2.1 roller_repo 扩展 (P1)

**文件**: `src/repository/roller_repo.rs`

**新增方法**:

```rust
/// 更新换辊周期锚点
pub fn update_campaign_anchor(
    &self,
    conn: &Connection,
    version_id: &str,
    machine_code: &str,
    campaign_no: i32,
    anchor_material_id: Option<&str>,
    anchor_width_mm: f64,
    anchor_thickness_mm: f64,
    anchor_source: AnchorSource,
) -> Result<(), RepoError>;

/// 重置换辊周期（换辊时调用）
pub fn reset_campaign_for_roll_change(
    &self,
    conn: &Connection,
    version_id: &str,
    machine_code: &str,
    new_campaign_no: i32,
    start_date: NaiveDate,
) -> Result<(), RepoError>;

/// 查询当前活跃的换辊周期
pub fn get_active_campaign(
    &self,
    conn: &Connection,
    version_id: &str,
    machine_code: &str,
) -> Result<Option<RollerCampaign>, RepoError>;
```

---

#### 2.2.2 material_repo 扩展 (P1)

**文件**: `src/repository/material_repo.rs`

**新增方法**:

```rust
/// 更新材料人工确认状态
pub fn update_user_confirmation(
    &self,
    conn: &Connection,
    version_id: &str,
    material_id: &str,
    confirmed_by: &str,
    reason: &str,
) -> Result<(), RepoError>;

/// 查询待人工确认的材料列表
pub fn list_pending_confirmations(
    &self,
    conn: &Connection,
    version_id: &str,
    machine_code: &str,
    plan_date: NaiveDate,
) -> Result<Vec<MaterialState>, RepoError>;

/// 批量查询人工确认材料（用于锚点解析）
pub fn list_user_confirmed_materials(
    &self,
    conn: &Connection,
    version_id: &str,
    machine_code: &str,
) -> Result<Vec<MaterialSummary>, RepoError>;
```

---

### 2.3 API 层

#### 2.3.1 path_rule_api.rs (P1)

**文件**: `src/api/path_rule_api.rs`

**Tauri Commands**:

```rust
// src/api/path_rule_api.rs

use tauri::command;

/// 获取路径规则配置
#[command]
pub fn get_path_rule_config() -> Result<PathRuleConfigDto, String>;

/// 更新路径规则配置
#[command]
pub fn update_path_rule_config(config: PathRuleConfigDto) -> Result<(), String>;

/// 获取待人工确认的路径违规材料
#[command]
pub fn list_path_override_pending(
    version_id: String,
    machine_code: String,
    plan_date: String,
) -> Result<Vec<PathOverridePendingDto>, String>;

/// 确认路径违规突破
#[command]
pub fn confirm_path_override(
    version_id: String,
    material_id: String,
    confirmed_by: String,
    reason: String,
) -> Result<(), String>;

/// 批量确认路径违规突破
#[command]
pub fn batch_confirm_path_override(
    version_id: String,
    material_ids: Vec<String>,
    confirmed_by: String,
    reason: String,
) -> Result<BatchConfirmResult, String>;

/// 获取当前换辊周期锚点状态
#[command]
pub fn get_roll_cycle_anchor(
    version_id: String,
    machine_code: String,
) -> Result<RollCycleAnchorDto, String>;

/// 手动重置换辊周期
#[command]
pub fn reset_roll_cycle(
    version_id: String,
    machine_code: String,
    actor: String,
) -> Result<(), String>;
```

**DTO 定义**:

```rust
// src/api/dto/path_rule_dto.rs

#[derive(Serialize, Deserialize)]
pub struct PathRuleConfigDto {
    pub enabled: bool,
    pub width_tolerance_mm: f64,
    pub thickness_tolerance_mm: f64,
    pub override_allowed_urgency_levels: Vec<String>, // ["L2", "L3"]
    pub seed_s2_percentile: f64,
    pub seed_s2_small_sample_threshold: i32,
}

#[derive(Serialize, Deserialize)]
pub struct PathOverridePendingDto {
    pub material_id: String,
    pub material_no: String,
    pub width_mm: f64,
    pub thickness_mm: f64,
    pub urgent_level: String,
    pub violation_type: String,
    pub anchor_width_mm: f64,
    pub anchor_thickness_mm: f64,
    pub width_delta_mm: f64,
    pub thickness_delta_mm: f64,
}

#[derive(Serialize, Deserialize)]
pub struct RollCycleAnchorDto {
    pub version_id: String,
    pub machine_code: String,
    pub campaign_no: i32,
    pub cum_weight_t: f64,
    pub anchor_source: String,
    pub anchor_material_id: Option<String>,
    pub anchor_width_mm: Option<f64>,
    pub anchor_thickness_mm: Option<f64>,
    pub status: String,
}
```

---

#### 2.3.2 main.rs 注册命令 (P1)

**修改**: `src/main.rs`

在 `invoke_handler` 中添加新命令:

```rust
.invoke_handler(tauri::generate_handler![
    // ... 现有命令 ...
    // 路径规则相关
    api::path_rule_api::get_path_rule_config,
    api::path_rule_api::update_path_rule_config,
    api::path_rule_api::list_path_override_pending,
    api::path_rule_api::confirm_path_override,
    api::path_rule_api::batch_confirm_path_override,
    api::path_rule_api::get_roll_cycle_anchor,
    api::path_rule_api::reset_roll_cycle,
])
```

---

## 三、前端实施计划

### 3.1 组件开发

#### 3.1.1 PathOverrideConfirmModal (P2)

**文件**: `src/components/path-override-confirm/PathOverrideConfirmModal.tsx`

**功能**:
- 展示待确认的路径违规材料列表
- 显示违规详情：材料信息、违规类型、超限量、锚点值
- 输入确认原因（必填）
- 单个/批量确认操作

**UI 设计**:

```
┌─────────────────────────────────────────────────────────────┐
│ 路径违规人工确认                                      [×]  │
├─────────────────────────────────────────────────────────────┤
│ 以下材料违反宽厚路径规则，需人工确认后方可排入计划：        │
│                                                             │
│ ┌─────────────────────────────────────────────────────────┐ │
│ │ □ M001 | 宽度超限 | 材料: 1280mm | 锚点: 1200mm | +30mm │ │
│ │ □ M002 | 厚度超限 | 材料: 11.5mm | 锚点: 10.0mm | +0.5mm│ │
│ │ □ M003 | 双超限   | 宽度+50mm, 厚度+1.2mm              │ │
│ └─────────────────────────────────────────────────────────┘ │
│                                                             │
│ 确认原因: [紧急订单，客户要求优先交付________________] *必填│
│                                                             │
│ ⚠️ 确认后材料将标记为已突破，可能影响下游材料排产           │
│                                                             │
│                    [取消]  [确认选中 (3)]                    │
└─────────────────────────────────────────────────────────────┘
```

---

#### 3.1.2 RollCycleAnchorCard (P2)

**文件**: `src/components/roll-cycle-anchor/RollCycleAnchorCard.tsx`

**功能**:
- 展示当前换辊周期的锚点状态
- 显示锚点来源、宽度、厚度
- 支持手动重置（触发换辊）

**UI 设计**:

```
┌─────────────────────────────────────────────┐
│ 当前换辊周期锚点         H032 | 批次 #6     │
├─────────────────────────────────────────────┤
│ 锚点来源: 冻结区最后一块                    │
│ 锚点材料: M100                              │
│ 宽度锚点: 1150 mm                           │
│ 厚度锚点: 8.5 mm                            │
│ 累计吨位: 1234 / 2500 t (49.4%)            │
│ ████████████░░░░░░░░░░░░░                   │
├─────────────────────────────────────────────┤
│ [刷新锚点]           [手动换辊 ⚠️]          │
└─────────────────────────────────────────────┘
```

---

#### 3.1.3 PathRuleConfigPanel (P2)

**文件**: `src/components/config-management/PathRuleConfigPanel.tsx`

**功能**:
- 路径规则开关
- 宽度/厚度容差配置
- 允许突破的紧急等级配置
- S2 种子策略配置

**UI 设计**:

```
┌─────────────────────────────────────────────────────────────┐
│ 宽厚路径规则配置                                            │
├─────────────────────────────────────────────────────────────┤
│ 启用路径规则      [████ ON ]                                │
│                                                             │
│ ── 容差阈值 ──                                             │
│ 宽度容差 (mm)     [50.0        ]                           │
│ 厚度容差 (mm)     [1.0         ]                           │
│                                                             │
│ ── 突破规则 ──                                             │
│ 允许突破等级      [✓] L2 紧急  [✓] L3 红线                │
│                                                             │
│ ── S2 种子策略 ──                                          │
│ 上沿分位点        [0.95        ]                           │
│ 小样本阈值        [10          ]                           │
│                                                             │
│                              [重置默认]  [保存]            │
└─────────────────────────────────────────────────────────────┘
```

---

### 3.2 页面集成

#### 3.2.1 SettingsCenter 扩展

**文件**: `src/pages/SettingsCenter.tsx`

**修改要点**:
- 在配置管理 Tab 中添加"路径规则"配置面板
- 集成 `PathRuleConfigPanel` 组件

---

#### 3.2.2 PlanningWorkbench 集成

**文件**: `src/pages/PlanningWorkbench.tsx`

**修改要点**:
- 一键重算后检查是否有待确认材料
- 弹出 `PathOverrideConfirmModal`
- 在工作台右侧添加 `RollCycleAnchorCard`

---

### 3.3 API 客户端

**文件**: `src/api/pathRuleApi.ts`

```typescript
import { invoke } from '@tauri-apps/api/tauri';

export interface PathRuleConfig {
  enabled: boolean;
  widthToleranceMm: number;
  thicknessToleranceMm: number;
  overrideAllowedUrgencyLevels: string[];
  seedS2Percentile: number;
  seedS2SmallSampleThreshold: number;
}

export interface PathOverridePending {
  materialId: string;
  materialNo: string;
  widthMm: number;
  thicknessMm: number;
  urgentLevel: string;
  violationType: string;
  anchorWidthMm: number;
  anchorThicknessMm: number;
  widthDeltaMm: number;
  thicknessDeltaMm: number;
}

export interface RollCycleAnchor {
  versionId: string;
  machineCode: string;
  campaignNo: number;
  cumWeightT: number;
  anchorSource: string;
  anchorMaterialId?: string;
  anchorWidthMm?: number;
  anchorThicknessMm?: number;
  status: string;
}

export const pathRuleApi = {
  getConfig: () => invoke<PathRuleConfig>('get_path_rule_config'),
  updateConfig: (config: PathRuleConfig) => invoke('update_path_rule_config', { config }),

  listPendingOverrides: (versionId: string, machineCode: string, planDate: string) =>
    invoke<PathOverridePending[]>('list_path_override_pending', { versionId, machineCode, planDate }),

  confirmOverride: (versionId: string, materialId: string, confirmedBy: string, reason: string) =>
    invoke('confirm_path_override', { versionId, materialId, confirmedBy, reason }),

  batchConfirmOverride: (versionId: string, materialIds: string[], confirmedBy: string, reason: string) =>
    invoke('batch_confirm_path_override', { versionId, materialIds, confirmedBy, reason }),

  getRollCycleAnchor: (versionId: string, machineCode: string) =>
    invoke<RollCycleAnchor>('get_roll_cycle_anchor', { versionId, machineCode }),

  resetRollCycle: (versionId: string, machineCode: string, actor: string) =>
    invoke('reset_roll_cycle', { versionId, machineCode, actor }),
};
```

---

### 3.4 React Query Hooks

**文件**: `src/hooks/queries/usePathRuleQueries.ts`

```typescript
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { pathRuleApi } from '@/api/pathRuleApi';

export const pathRuleKeys = {
  config: ['pathRule', 'config'] as const,
  pending: (versionId: string, machineCode: string, planDate: string) =>
    ['pathRule', 'pending', versionId, machineCode, planDate] as const,
  anchor: (versionId: string, machineCode: string) =>
    ['pathRule', 'anchor', versionId, machineCode] as const,
};

export function usePathRuleConfig() {
  return useQuery({
    queryKey: pathRuleKeys.config,
    queryFn: () => pathRuleApi.getConfig(),
  });
}

export function useUpdatePathRuleConfig() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: pathRuleApi.updateConfig,
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: pathRuleKeys.config });
    },
  });
}

export function usePendingOverrides(versionId: string, machineCode: string, planDate: string) {
  return useQuery({
    queryKey: pathRuleKeys.pending(versionId, machineCode, planDate),
    queryFn: () => pathRuleApi.listPendingOverrides(versionId, machineCode, planDate),
    enabled: !!versionId && !!machineCode && !!planDate,
  });
}

export function useConfirmOverride() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: ({ versionId, materialId, confirmedBy, reason }: {
      versionId: string;
      materialId: string;
      confirmedBy: string;
      reason: string;
    }) => pathRuleApi.confirmOverride(versionId, materialId, confirmedBy, reason),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['pathRule'] });
    },
  });
}

export function useRollCycleAnchor(versionId: string, machineCode: string) {
  return useQuery({
    queryKey: pathRuleKeys.anchor(versionId, machineCode),
    queryFn: () => pathRuleApi.getRollCycleAnchor(versionId, machineCode),
    enabled: !!versionId && !!machineCode,
  });
}

export function useResetRollCycle() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: ({ versionId, machineCode, actor }: {
      versionId: string;
      machineCode: string;
      actor: string;
    }) => pathRuleApi.resetRollCycle(versionId, machineCode, actor),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['pathRule'] });
    },
  });
}
```

---

## 四、测试计划

### 4.1 单元测试 (P0)

**文件**: `tests/path_rule_engine_test.rs`

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_no_anchor_returns_ok() { /* ... */ }

    #[test]
    fn test_within_tolerance_returns_ok() { /* ... */ }

    #[test]
    fn test_width_exceeded_l0_returns_hard_violation() { /* ... */ }

    #[test]
    fn test_width_exceeded_l3_returns_override_required() { /* ... */ }

    #[test]
    fn test_both_exceeded() { /* ... */ }

    #[test]
    fn test_user_confirmed_returns_ok_with_flag() { /* ... */ }

    #[test]
    fn test_disabled_rule_returns_ok() { /* ... */ }
}
```

**文件**: `tests/anchor_resolver_test.rs`

```rust
#[cfg(test)]
mod tests {
    #[test]
    fn test_frozen_priority() { /* ... */ }

    #[test]
    fn test_locked_fallback() { /* ... */ }

    #[test]
    fn test_user_confirmed_fallback() { /* ... */ }

    #[test]
    fn test_seed_s2_large_sample() { /* ... */ }

    #[test]
    fn test_seed_s2_small_sample() { /* ... */ }

    #[test]
    fn test_no_candidates_returns_none() { /* ... */ }
}
```

---

### 4.2 集成测试 (P1)

**文件**: `tests/path_rule_integration_test.rs`

```rust
#[cfg(test)]
mod tests {
    #[test]
    fn test_capacity_filler_with_path_rule() { /* ... */ }

    #[test]
    fn test_roll_cycle_reset_clears_anchor() { /* ... */ }

    #[test]
    fn test_user_confirmation_flow() { /* ... */ }

    #[test]
    fn test_action_log_recorded() { /* ... */ }
}
```

---

### 4.3 E2E 测试 (P2)

**文件**: `tests/path_rule_e2e_test.rs`

```rust
#[cfg(test)]
mod tests {
    #[test]
    fn test_full_override_workflow() {
        // 1. 创建版本
        // 2. 导入材料（含违规材料）
        // 3. 触发重算
        // 4. 检查待确认列表
        // 5. 确认突破
        // 6. 再次重算
        // 7. 验证材料入池
        // 8. 检查审计日志
    }

    #[test]
    fn test_roll_change_resets_anchor() {
        // 1. 创建版本
        // 2. 填充材料至硬限
        // 3. 触发换辊
        // 4. 验证锚点重置
        // 5. 验证 campaign_no 递增
        // 6. 检查审计日志
    }
}
```

---

## 五、实施时间线

| 阶段 | 任务 | 估算工作量 | 依赖 |
|------|------|------------|------|
| Phase 1 | PathRuleEngine + AnchorResolver | 2-3 天 | 无 |
| Phase 2 | CapacityFiller 集成 | 1-2 天 | Phase 1 |
| Phase 3 | Repository 层扩展 | 1 天 | Phase 2 |
| Phase 4 | API 层 + Tauri 命令 | 1 天 | Phase 3 |
| Phase 5 | 单元测试 + 集成测试 | 2 天 | Phase 4 |
| Phase 6 | 前端组件开发 | 2-3 天 | Phase 4 |
| Phase 7 | 页面集成 + E2E 测试 | 1-2 天 | Phase 6 |

**总计**: 10-14 天

---

## 六、文件清单

### 6.1 新建文件

| 文件路径 | 说明 |
|----------|------|
| `src/engine/path_rule.rs` | PathRuleEngine 实现 |
| `src/engine/anchor_resolver.rs` | AnchorResolver 实现 |
| `src/api/path_rule_api.rs` | 路径规则 API |
| `src/api/dto/path_rule_dto.rs` | DTO 定义 |
| `src/components/path-override-confirm/PathOverrideConfirmModal.tsx` | 人工确认弹窗 |
| `src/components/roll-cycle-anchor/RollCycleAnchorCard.tsx` | 锚点状态卡片 |
| `src/components/config-management/PathRuleConfigPanel.tsx` | 配置面板 |
| `src/api/pathRuleApi.ts` | 前端 API 客户端 |
| `src/hooks/queries/usePathRuleQueries.ts` | React Query Hooks |
| `tests/path_rule_engine_test.rs` | 引擎单元测试 |
| `tests/anchor_resolver_test.rs` | 解析器单元测试 |
| `tests/path_rule_integration_test.rs` | 集成测试 |
| `tests/path_rule_e2e_test.rs` | E2E 测试 |

### 6.2 修改文件

| 文件路径 | 修改内容 |
|----------|----------|
| `src/engine/mod.rs` | 添加 path_rule, anchor_resolver 模块 |
| `src/engine/capacity_filler.rs` | 集成 PathRuleEngine |
| `src/repository/roller_repo.rs` | 添加锚点管理方法 |
| `src/repository/material_repo.rs` | 添加人工确认方法 |
| `src/api/mod.rs` | 添加 path_rule_api 模块 |
| `src/main.rs` | 注册 Tauri 命令 |
| `src/pages/SettingsCenter.tsx` | 添加配置面板 |
| `src/pages/PlanningWorkbench.tsx` | 集成人工确认流程 |

---

## 七、风险与注意事项

### 7.1 工业红线

- **冻结区保护**: 锚点解析时冻结区优先级最高，不改变冻结材料
- **人工最终控制**: OVERRIDE_REQUIRED 必须人工确认，不能自动通过
- **审计记录**: 所有突破操作必须记录到 action_log

### 7.2 兼容性

- 新增字段使用 NULL 默认值，兼容现有数据
- 路径规则可通过配置禁用，不影响现有流程
- 前端组件按需加载，不影响现有页面性能

### 7.3 性能考虑

- S2 种子策略使用排序算法，时间复杂度 O(n log n)
- 锚点更新在内存中进行，每次填充后持久化
- 前端使用 React Query 缓存，避免重复请求

---

## 八、验收标准

### 8.1 功能验收

- [ ] PathRuleEngine 正确判定路径违规
- [ ] AnchorResolver 按优先级解析锚点
- [ ] CapacityFiller 正确集成路径门控
- [ ] 人工确认流程完整可用
- [ ] 换辊重置锚点正确
- [ ] 配置项可通过前端修改
- [ ] 审计日志记录完整

### 8.2 测试验收

- [ ] 单元测试覆盖率 ≥ 80%
- [ ] 集成测试通过
- [ ] E2E 测试通过

### 8.3 文档验收

- [ ] API 接口文档完整
- [ ] 前端组件文档完整
- [ ] 配置项说明完整

---

## 附录 A: 配置项速查

| 配置项 | 默认值 | 说明 |
|--------|--------|------|
| path_rule_enabled | true | 是否启用路径规则 |
| path_width_tolerance_mm | 50.0 | 宽度容差 (mm) |
| path_thickness_tolerance_mm | 1.0 | 厚度容差 (mm) |
| path_override_allowed_urgency_levels | L2,L3 | 允许突破的等级 |
| seed_s2_percentile | 0.95 | S2 上沿分位点 |
| seed_s2_small_sample_threshold | 10 | S2 小样本阈值 |

---

## 附录 B: ActionType 速查

| ActionType | 说明 | payload 关键字段 |
|------------|------|-----------------|
| PathOverrideConfirm | 路径突破人工确认 | material_id, violation_type, confirm_reason |
| RollCycleReset | 换辊周期重置 | machine_code, previous_campaign_no, reset_trigger |
