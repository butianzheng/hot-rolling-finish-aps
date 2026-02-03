# 宽厚路径规则（v0.6）编码开发计划

> **版本**: v0.6
> **依据规范**: spec/Engine_Specs_v0.3_Integrated.md 章节 14-18
> **状态**: ✅ 已落地（核心引擎/前端闭环/测试已完成）

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
| Domain | MaterialState 扩展（user_confirmed* 字段对齐） | P0 |
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

### 1.4 项目扫描结论（2026-02-02）

> 本节基于对当前仓库代码的实际扫描，目的：把“计划”对齐成可以直接开工的 TODO，并标注需要适配的现有结构。

**已验证可复用的现有基础**:
- 数据模型/枚举：`src/domain/types.rs` 已包含 AnchorSource / PathViolationType / PathRuleStatus
- 换辊领域模型：`src/domain/roller.rs` 已包含 path_anchor_* 字段与 update/reset 方法
- 审计动作：`src/domain/action_log.rs` 已包含 PathOverrideConfirm / RollCycleReset
- 配置体系：`src/config/config_manager.rs` + `src/api/config_api.rs` + 前端配置管理页 `src/components/config-management/*`
- Tauri 命令层：`src/app/tauri_commands.rs`（snake_case）+ `src/main.rs` 统一注册
- 排产主流程入口：`src/engine/orchestrator.rs` 调用 `src/engine/capacity_filler.rs::fill_single_day`
- 测试基座：`tests/` 已存在多类 API/Engine/E2E 测试，可按同风格追加

**当前缺口（Gap）**:
- Engine：✅ 已实现 `src/engine/path_rule.rs`、`src/engine/anchor_resolver.rs` 并在 `src/engine/mod.rs` 注册导出
- Domain/Repo 对齐：✅ 已对齐 `material_state.user_confirmed*` 与 `roller_campaign.path_anchor_* / anchor_source` 的映射与仓储方法
- API/Tauri：✅ 已实现 `src/api/path_rule_api.rs` 并完成 `src/app/state.rs` 注入、`src/app/tauri_commands.rs` 命令包装、`src/main.rs` 注册；前端 `src/api/tauri.ts` / `src/api/ipcSchemas.ts` 已补齐调用与 schema
- 前端：暂无 PathOverrideConfirmModal / RollCycleAnchorCard；配置管理页未显示 path_rule_* / seed_s2_* 配置键；工作台未集成人工确认闭环
- 关键设计点：✅ 已采用方案 A（`PATH_OVERRIDE_REQUIRED` 作为 skipped reason），并提供 `PathRuleApi.list_path_override_pending` 查询入口

### 1.5 TODO List（按里程碑推进）

#### M0（P0）数据结构与仓储对齐（先做，避免后续返工）

- [x] Domain 枚举：`src/domain/types.rs`
- [x] RollerCampaign 锚点字段：`src/domain/roller.rs`
- [x] ActionType：`src/domain/action_log.rs`
- [x] 迁移脚本：`migrations/v0.6_path_rule_extension.sql`
- [x] 对齐 MaterialState：为 `src/domain/material.rs` 增加 `user_confirmed/user_confirmed_at/user_confirmed_by/user_confirmed_reason`
- [x] 对齐 MaterialStateRepository：更新 `src/repository/material_repo.rs` 的 INSERT/SELECT/快照结构以读写 user_confirmed* 列，并补充“人工确认”写入方法
- [x] 对齐 RollerCampaignRepository：更新 `src/repository/roller_repo.rs` 的 SELECT/INSERT/UPDATE 以读写 path_anchor_* 与 anchor_source
- [x] 明确迁移执行方式：补充“如何应用 v0.6 SQL”的说明（如已有脚本/流程，记录到本文或相关 docs）

**迁移执行方式（SQLite）**：

> v0.6 的迁移文件为 `migrations/v0.6_path_rule_extension.sql`，其中 `ALTER TABLE ... ADD COLUMN` **不具备幂等性**（重复执行会报 duplicate column name）。

1) **开发/测试环境（推荐）**：直接重建 DB（避免处理历史脏数据/重复字段）
- 使用 `scripts/dev_db/schema.sql`（已对齐 v0.6 字段）重建并灌数：
  - `bash scripts/dev_db/reset_and_seed.sh`
- 或直接跑二进制（等价）：
  - `cargo run --bin reset_and_seed_full_scenario_db --`

2) **已有 DB 就地升级（保留历史数据）**
- 先备份 DB（强烈建议）：
  - `cp hot_rolling_aps.db backups/hot_rolling_aps.db.bak.$(date +%Y%m%d_%H%M%S)`
- 执行迁移（只执行一次）：
  - `sqlite3 hot_rolling_aps.db < migrations/v0.6_path_rule_extension.sql`
- 验证字段是否存在：
  - `sqlite3 hot_rolling_aps.db "PRAGMA table_info(material_state);"`
  - `sqlite3 hot_rolling_aps.db "PRAGMA table_info(roller_campaign);"`

3) **DB 路径说明（避免“改了一个库，应用读另一个库”）**
- 默认开发运行时 DB 位于用户数据目录 `hot-rolling-aps-dev/hot_rolling_aps.db`（首次启动会从项目根目录 `./hot_rolling_aps.db` 复制种子库）。如需指定 DB 路径，可设置环境变量：
  - `HOT_ROLLING_APS_DB_PATH=/path/to/hot_rolling_aps.db`

#### M1（P0）核心引擎实现 + 单元测试（不接 UI 也能自证正确）

- [x] 新增 `src/engine/path_rule.rs`：实现 PathRuleEngine（含 PathRuleConfig、Anchor、check 逻辑）
- [x] 新增 `src/engine/anchor_resolver.rs`：实现 AnchorResolver（优先级 + S2 种子策略）
- [x] 更新 `src/engine/mod.rs`：注册并导出新模块（便于 orchestrator/测试复用）
- [x] 单元测试：新增 `tests/path_rule_engine_test.rs`、`tests/anchor_resolver_test.rs`（覆盖文档列出的要点）

#### M2（P0/P1）与排产主流程集成（对齐现有 orchestrator/filler 结构）

- [x] 设计决策：确定 `OVERRIDE_REQUIRED` 在当前系统的承载方式
  - 方案 A（推荐，改动较小）：作为 `skipped_materials` 的一种 reason（例如 `PATH_OVERRIDE_REQUIRED`），前端从“跳过列表”发起确认；确认后再次重算即可入池
  - 方案 B（更完整）：扩展 CapacityFiller/Orchestrator 返回结构，单独输出 `pending_confirmation`，并提供持久化/查询入口
- [x] 修改 `src/engine/capacity_filler.rs::fill_single_day`：在产能门控前增加路径门控（HardViolation→跳过；OverrideRequired→按选定方案输出/暂存；Ok→继续）
- [x] 锚点生命周期：在每次入池后更新锚点；换辊/重置时清空锚点（依赖 roller_repo 的持久化接口）
- [x] 审计：人工确认与换辊重置写入 action_log（已具备 ActionType，需补齐落库调用点）

#### M3（P1）API/Tauri 对外能力（支持前端闭环）

- [x] 新增 `src/api/path_rule_api.rs`：提供配置读取/更新、待确认列表、确认突破、锚点查询、换辊重置等接口（按现有 API 风格内置 DTO）
- [x] 更新 `src/api/mod.rs`：导出 PathRuleApi
- [x] 更新 `src/app/state.rs`：在 AppState 注入 PathRuleApi（依赖相关 repos/config）
- [x] 更新 `src/app/tauri_commands.rs`：新增 tauri commands（snake_case）并按既有 map_api_error 返回 JSON
- [x] 更新 `src/main.rs`：在 invoke_handler 注册新命令
- [x] 前端接入：更新 `src/api/tauri.ts` + `src/api/ipcSchemas.ts`，补齐调用与 schema 校验

#### M4（P2）前端页面/组件（可视化 + 人工确认）

- [x] 配置管理页扩展：在 `src/components/config-management/types.ts` 增加 path_rule_* / seed_s2_* 的 labels 与 descriptions；如需专用面板再新增 `PathRuleConfigPanel.tsx`
- [x] 独立设置面板：在 `src/pages/SettingsCenter.tsx` 增加“路径规则”tab 并挂载 `src/components/settings/PathRuleConfigPanel.tsx`；工作台“设置/工具”增加入口
- [x] 人工确认弹窗：新增 `src/components/path-override-confirm/PathOverrideConfirmModal.tsx`，从待确认列表发起单个/批量确认
- [x] 锚点状态卡：新增 `src/components/roll-cycle-anchor/RollCycleAnchorCard.tsx`，展示 anchor_source/path_anchor_* 并支持“手动换辊/重置”
- [x] 工作台集成：`src/pages/PlanningWorkbench.tsx` 集成“待确认提示 → 弹窗确认 → 再次重算/刷新”

#### M5（P1/P2）集成测试与验收（回归现有能力）

- [x] 集成测试：新增 `tests/path_rule_integration_test.rs`（覆盖“门控 + 确认 + 再入池 + 审计”）
- [x] E2E 测试：新增 `tests/path_rule_e2e_test.rs`（覆盖“重算→待确认→确认→再重算→入池”；换辊重置已在集成测试覆盖）
- [x] 回归：跑通现有 `tests/*` 中的引擎/接口用例，确保未破坏既有排产流程

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
// 现有入口：fill_single_day（由 src/engine/orchestrator.rs 调用）
// 目标：在“产能门控”前增加“路径门控”，并维护 roll campaign 的锚点状态。
//
// 关键适配点：
// - 当前函数签名仅返回 (plan_items, skipped_materials)，没有 pending_confirmation；需在实现阶段选定输出方案（见 1.5/M2）。
// - 锚点解析需要 width/thickness：frozen_items 是 PlanItem 列表，需由 orchestrator 用 material_id 关联到 MaterialMaster/State 后再构造 summary。

impl CapacityFiller {
    pub fn fill_single_day(
        &self,
        capacity_pool: &mut CapacityPool,
        candidates: Vec<(MaterialMaster, MaterialState)>,
        frozen_items: Vec<PlanItem>,
        version_id: &str,
    ) -> (Vec<PlanItem>, Vec<(MaterialMaster, MaterialState, String)>) {
        // 0) 先把 frozen_items 原样入池，sequence_no 从 frozen_items.len()+1 开始
        // 1) AnchorResolver.resolve(...) 计算初始锚点（FrozenLast/LockedLast/UserConfirmedLast/SeedS2）
        // 2) 遍历 candidates（含 Locked）：
        //    - path_rule_engine.check(width, thickness, state.urgent_level, current_anchor, state.user_confirmed)
        //      - HardViolation => skipped.push((m, s, "PATH_HARD_VIOLATION: ..."))
        //      - OverrideRequired => skipped.push((m, s, "PATH_OVERRIDE_REQUIRED: ...")) 或写入 pending 列表
        //      - Ok => 继续
        //    - capacity_pool.can_add_material(weight) 等现有逻辑不变（Locked 的产能红线仍优先）
        //    - 入池后更新 current_anchor（并在需要时持久化到 roller_campaign）
        todo!()
    }
}
```

---

### 2.2 Repository 层

#### 2.2.1 roller_repo 扩展 (P1)

**文件**: `src/repository/roller_repo.rs`

**需要新增/扩展的方法**（注意：当前仓储实现内部持有连接，不需要在签名中传入 `&Connection`）:

```rust
/// 更新换辊周期锚点
pub fn update_campaign_anchor(
    &self,
    version_id: &str,
    machine_code: &str,
    campaign_no: i32,
    anchor_material_id: Option<&str>,
    anchor_width_mm: Option<f64>,
    anchor_thickness_mm: Option<f64>,
    anchor_source: AnchorSource,
) -> RepositoryResult<()>;

/// 重置换辊周期（换辊时调用）
pub fn reset_campaign_for_roll_change(
    &self,
    version_id: &str,
    machine_code: &str,
    new_campaign_no: i32,
    start_date: NaiveDate,
) -> RepositoryResult<()>;

/// 查询当前活跃的换辊周期（现有 find_active_campaign 方法需扩展字段映射）
pub fn find_active_campaign(
    &self,
    version_id: &str,
    machine_code: &str,
) -> RepositoryResult<Option<RollerCampaign>>;
```

---

#### 2.2.2 material_repo 扩展 (P1)

**文件**: `src/repository/material_repo.rs`

**需要新增/扩展的方法**（注意：当前仓储实现内部持有连接，不需要在签名中传入 `&Connection`）:

```rust
/// 更新材料人工确认状态
pub fn update_user_confirmation(
    &self,
    material_id: &str,
    confirmed_by: &str,
    reason: &str,
) -> RepositoryResult<()>;

/// 查询待人工确认的材料列表（版本口径需在实现时明确：material_state 无 version_id，可用 last_calc_version_id 或 join plan_item）
pub fn list_pending_confirmations(
    &self,
    machine_code: &str,
    plan_date: NaiveDate,
) -> RepositoryResult<Vec<MaterialState>>;

/// 批量查询人工确认材料（用于锚点解析）
pub fn list_user_confirmed_materials(
    &self,
    machine_code: &str,
) -> RepositoryResult<Vec<MaterialSummary>>;
```

---

### 2.3 API 层

#### 2.3.1 path_rule_api.rs (P1)

**文件**: `src/api/path_rule_api.rs`

**API 方法**（由 `src/app/tauri_commands.rs` 包装为 `#[tauri::command]`）:

```rust
// src/api/path_rule_api.rs（伪代码：展示方法清单；实现风格可参考 src/api/config_api.rs / roller_api.rs）

use crate::api::error::ApiResult;

pub struct PathRuleApi {
    // 典型依赖：ConfigManager / MaterialStateRepository / RollerCampaignRepository / ActionLogRepository ...
}

impl PathRuleApi {
    pub fn get_path_rule_config(&self) -> ApiResult<PathRuleConfigDto> {
        todo!()
    }

    pub fn update_path_rule_config(
        &self,
        config: PathRuleConfigDto,
        operator: &str,
        reason: &str,
    ) -> ApiResult<()> {
        todo!()
    }

    pub fn list_path_override_pending(
        &self,
        version_id: &str,
        machine_code: &str,
        plan_date: chrono::NaiveDate,
    ) -> ApiResult<Vec<PathOverridePendingDto>> {
        todo!()
    }

    pub fn confirm_path_override(
        &self,
        version_id: &str,
        material_id: &str,
        confirmed_by: &str,
        reason: &str,
    ) -> ApiResult<()> {
        todo!()
    }

    pub fn batch_confirm_path_override(
        &self,
        version_id: &str,
        material_ids: &[String],
        confirmed_by: &str,
        reason: &str,
    ) -> ApiResult<BatchConfirmResultDto> {
        todo!()
    }

    pub fn get_roll_cycle_anchor(
        &self,
        version_id: &str,
        machine_code: &str,
    ) -> ApiResult<RollCycleAnchorDto> {
        todo!()
    }

    pub fn reset_roll_cycle(
        &self,
        version_id: &str,
        machine_code: &str,
        actor: &str,
    ) -> ApiResult<()> {
        todo!()
    }
}
```

**Tauri Commands 位置**: `src/app/tauri_commands.rs`（参考 `list_materials` 等：从 `AppState` 调用 `state.path_rule_api.*`，然后 `serde_json::to_string` 返回给前端）。

**DTO 定义**:

```rust
// src/api/path_rule_api.rs（DTO 可与 API 同文件/同模块定义；当前仓库未使用 dto/ 子模块）

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

#[derive(Serialize, Deserialize)]
pub struct BatchConfirmResultDto {
    pub success_count: i32,
    pub fail_count: i32,
    pub failed_material_ids: Vec<String>,
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
    // 说明：命令函数定义在 src/app/tauri_commands.rs（由 app/mod.rs 重导出），这里直接注册函数名
    get_path_rule_config,
    update_path_rule_config,
    list_path_override_pending,
    confirm_path_override,
    batch_confirm_path_override,
    get_roll_cycle_anchor,
    reset_roll_cycle,
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

### 3.3 前端 Tauri API 封装

**文件**: `src/api/tauri.ts`

```typescript
import { IpcClient } from './ipcClient';
import { z, zodValidator, PathRuleConfigSchema, PathOverridePendingSchema, RollCycleAnchorSchema } from './ipcSchemas';

export const pathRuleApi = {
  getPathRuleConfig() {
    return IpcClient.call('get_path_rule_config', {}, {
      validate: zodValidator(PathRuleConfigSchema, 'get_path_rule_config'),
    });
  },

  updatePathRuleConfig(config: any, operator: string, reason: string) {
    return IpcClient.call('update_path_rule_config', {
      config,
      operator,
      reason,
    });
  },

  listPendingOverrides(versionId: string, machineCode: string, planDate: string) {
    return IpcClient.call('list_path_override_pending', {
      version_id: versionId,
      machine_code: machineCode,
      plan_date: planDate,
    }, {
      validate: zodValidator(z.array(PathOverridePendingSchema), 'list_path_override_pending'),
    });
  },

  confirmOverride(versionId: string, materialId: string, confirmedBy: string, reason: string) {
    return IpcClient.call('confirm_path_override', {
      version_id: versionId,
      material_id: materialId,
      confirmed_by: confirmedBy,
      reason,
    });
  },

  batchConfirmOverride(versionId: string, materialIds: string[], confirmedBy: string, reason: string) {
    return IpcClient.call('batch_confirm_path_override', {
      version_id: versionId,
      material_ids: materialIds,
      confirmed_by: confirmedBy,
      reason,
    });
  },

  getRollCycleAnchor(versionId: string, machineCode: string) {
    return IpcClient.call('get_roll_cycle_anchor', {
      version_id: versionId,
      machine_code: machineCode,
    }, {
      validate: zodValidator(RollCycleAnchorSchema, 'get_roll_cycle_anchor'),
    });
  },

  resetRollCycle(versionId: string, machineCode: string, actor: string) {
    return IpcClient.call('reset_roll_cycle', {
      version_id: versionId,
      machine_code: machineCode,
      actor,
    });
  },
};
```

---

### 3.4 React Query Hooks

**文件**: `src/hooks/queries/use-path-rule-queries.ts`

```typescript
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { pathRuleApi } from '../../api/tauri';

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
    queryFn: () => pathRuleApi.getPathRuleConfig(),
  });
}

export function useUpdatePathRuleConfig() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: ({ config, operator, reason }: any) => pathRuleApi.updatePathRuleConfig(config, operator, reason),
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
| Phase 0 | 数据结构/Repo 对齐（MaterialState + RollerCampaign） | 1 天 | 无 |
| Phase 1 | PathRuleEngine + AnchorResolver | 2-3 天 | Phase 0 |
| Phase 2 | CapacityFiller/Orchestrator 集成 | 1-2 天 | Phase 1 |
| Phase 3 | API 层 + Tauri 命令 | 1 天 | Phase 2 |
| Phase 4 | 单元测试 + 集成测试 | 2 天 | Phase 2 |
| Phase 5 | 前端组件开发 | 2-3 天 | Phase 3 |
| Phase 6 | 页面集成 + E2E 测试 | 1-2 天 | Phase 5 |

**总计**: 10-14 天（若选择“方案 B：pending_confirmation 持久化”，Phase 2/3 可能 +1~2 天）

---

## 六、文件清单

### 6.1 新建文件

| 文件路径 | 说明 |
|----------|------|
| `src/engine/path_rule.rs` | PathRuleEngine 实现 |
| `src/engine/anchor_resolver.rs` | AnchorResolver 实现 |
| `src/api/path_rule_api.rs` | 路径规则 API |
| `src/components/path-override-confirm/PathOverrideConfirmModal.tsx` | 人工确认弹窗 |
| `src/components/roll-cycle-anchor/RollCycleAnchorCard.tsx` | 锚点状态卡片 |
| `src/components/config-management/PathRuleConfigPanel.tsx` | 配置面板 |
| `src/hooks/queries/use-path-rule-queries.ts` | React Query Hooks（可选，按现有 hooks/queries 风格） |
| `tests/path_rule_engine_test.rs` | 引擎单元测试 |
| `tests/anchor_resolver_test.rs` | 解析器单元测试 |
| `tests/path_rule_integration_test.rs` | 集成测试 |
| `tests/path_rule_e2e_test.rs` | E2E 测试 |

### 6.2 修改文件

| 文件路径 | 修改内容 |
|----------|----------|
| `src/domain/material.rs` | 补齐 user_confirmed* 字段 |
| `src/app/state.rs` | 注入 PathRuleApi（依赖 repos/config） |
| `src/app/tauri_commands.rs` | 新增 path rule 相关命令包装（snake_case + map_api_error） |
| `src/engine/mod.rs` | 添加 path_rule, anchor_resolver 模块 |
| `src/engine/capacity_filler.rs` | 在 fill_single_day 集成 PathRuleEngine |
| `src/engine/orchestrator.rs` | （如需）扩展输出/对齐 pending_confirmation 方案 |
| `src/repository/roller_repo.rs` | 映射并维护锚点字段（path_anchor_* / anchor_source） |
| `src/repository/material_repo.rs` | 映射 user_confirmed* 并实现人工确认写入 |
| `src/api/mod.rs` | 添加 path_rule_api 模块 |
| `src/main.rs` | 注册 Tauri 命令 |
| `src/api/tauri.ts` | 前端增加 path rule 相关调用 |
| `src/api/ipcSchemas.ts` | 前端增加对应 schema 校验 |
| `src/components/config-management/types.ts` | 增加 path_rule_* / seed_s2_* 键的 labels/descriptions |
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
