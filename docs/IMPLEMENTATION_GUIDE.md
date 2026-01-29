# 前端重构关键修复方案 - 实现指南

## 概述

本文档提供3个P0关键修复的**完整实现代码**和**分步骤指南**。

---

# 修复1: 策略草案持久化 🔴 P0

## 问题再述

当前draft仅存于内存(OnceLock)，导致：
- 应用重启丢失
- 页面刷新丢失
- 多用户互相覆盖

## 修复步骤

### Step 1: 后端数据库表设计 (2小时)

**文件**: `scripts/dev_db/schema.sql`

```sql
-- 新增策略草案表
CREATE TABLE decision_strategy_draft (
    draft_id TEXT PRIMARY KEY,
    base_version_id TEXT NOT NULL,
    strategy_type TEXT NOT NULL CHECK(strategy_type IN (
        'balanced', 'urgent_first', 'capacity_first', 'cold_stock_first'
    )),
    status TEXT NOT NULL CHECK(status IN ('DRAFT', 'PUBLISHED', 'EXPIRED')),
    plan_date_from DATE NOT NULL,
    plan_date_to DATE NOT NULL,

    -- 操作员/锁定信息
    created_by TEXT NOT NULL,
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    created_version TEXT,  -- 创建时的应用版本
    expires_at DATETIME NOT NULL,  -- 默认72小时后过期

    -- 发布信息
    published_by TEXT,
    published_at DATETIME,
    published_as_version_id TEXT,  -- 若已发布，关联的version_id

    -- 并发控制
    locked_by TEXT,  -- 正在编辑的用户
    locked_at DATETIME,

    -- 数据存储
    snapshot_json TEXT NOT NULL,  -- 完整排产快照
    diff_items_json TEXT NOT NULL,  -- 变更明细
    kpi_summary_json TEXT,  -- KPI汇总

    FOREIGN KEY (base_version_id) REFERENCES plan_version(version_id),
    FOREIGN KEY (published_as_version_id) REFERENCES plan_version(version_id),
    INDEX idx_base_version (base_version_id),
    INDEX idx_status (status),
    INDEX idx_created_by (created_by),
    INDEX idx_expires_at (expires_at),
    INDEX idx_created_at (created_at DESC)
);

-- 新增操作日志扩展表，记录draft相关操作
ALTER TABLE action_log ADD COLUMN (
    draft_id TEXT REFERENCES decision_strategy_draft(draft_id)
);
```

**验证SQL**: 在SQLite中执行，确保表创建成功

```sql
SELECT * FROM decision_strategy_draft LIMIT 0;
-- 应该返回表结构，无行
```

---

### Step 2: 后端API改进 (10小时)

**文件**: `src/api/plan_api.rs`

#### 2.1 数据结构改进

```rust
// 在文件顶部添加新结构体

use uuid::Uuid;
use chrono::{DateTime, Utc, Duration};

/// 策略草案记录（带ID）
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct StrategyDraftRecordWithId {
    pub draft_id: String,           // UUID
    pub base_version_id: String,
    pub strategy_type: String,      // balanced, urgent_first, ...
    pub status: String,             // DRAFT, PUBLISHED, EXPIRED
    pub plan_date_from: String,
    pub plan_date_to: String,

    // 操作信息
    pub created_by: String,
    pub created_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,

    // 发布信息
    pub published_as_version_id: Option<String>,
    pub published_by: Option<String>,
    pub published_at: Option<DateTime<Utc>>,

    // 排产数据快照
    pub version_snapshot: PlanVersion,  // 完整排产方案
    pub diff_items: Vec<DiffItemInfo>,  // 变更明细
    pub kpi_summary: Option<KPISummary>, // KPI汇总
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct KPISummary {
    pub l3_completion_rate: f64,
    pub l2_completion_rate: f64,
    pub capacity_utilization: f64,
    pub capacity_overflow: f64,
    pub cold_stock_count: i32,
}

/// 生成多策略草案的改进响应
#[derive(Serialize, Deserialize)]
pub struct GenerateStrategyDraftsResponse {
    pub drafts: Vec<StrategyDraftRecordWithId>,
    pub total_count: i32,
    pub generated_at: DateTime<Utc>,
}

/// 应用草案生成版本的改进响应
#[derive(Serialize, Deserialize)]
pub struct ApplyStrategyDraftResponse {
    pub version_id: String,
    pub version_name: String,
    pub draft_id: String,
    pub published_at: DateTime<Utc>,
}

/// 查询草案列表的请求
#[derive(Serialize, Deserialize)]
pub struct ListStrategyDraftsRequest {
    pub base_version_id: String,
    pub status_filter: Option<String>,  // DRAFT, PUBLISHED, EXPIRED
    pub created_by_filter: Option<String>,
}
```

#### 2.2 改进generate_strategy_drafts()

```rust
#[tauri::command]
pub fn generate_strategy_drafts(
    base_version_id: &str,
    plan_date_from: String,  // "2026-01-20"
    plan_date_to: String,    // "2026-01-26"
    strategies: Vec<String>, // ["balanced", "urgent_first"]
    operator: &str,
) -> ApiResult<GenerateStrategyDraftsResponse> {
    // 1. 验证输入
    let base_version = db.plan_version.find_by_id(base_version_id)?;
    if base_version.status != "ACTIVE" {
        return Err(ApiError::VersionNotActive);
    }

    let date_from = NaiveDate::parse_from_str(&plan_date_from, "%Y-%m-%d")?;
    let date_to = NaiveDate::parse_from_str(&plan_date_to, "%Y-%m-%d")?;

    if (date_to - date_from).num_days() > 60 {
        return Err(ApiError::DateRangeTooLarge);
    }

    // 2. 生成多个草案
    let mut draft_records = Vec::new();
    let now = Utc::now();

    for strategy in strategies {
        // 2.1 生成草案ID
        let draft_id = Uuid::new_v4().to_string();

        // 2.2 执行排产计算
        let (version_snapshot, diff_items) = engine.recalc_with_strategy(
            &base_version,
            &strategy,
            date_from,
            date_to,
        )?;

        // 2.3 计算KPI汇总
        let kpi_summary = calculate_kpi_summary(&version_snapshot)?;

        // 2.4 构建草案记录
        let draft = StrategyDraftRecordWithId {
            draft_id: draft_id.clone(),
            base_version_id: base_version_id.to_string(),
            strategy_type: strategy.clone(),
            status: "DRAFT".to_string(),
            plan_date_from,
            plan_date_to,
            created_by: operator.to_string(),
            created_at: now,
            expires_at: now + Duration::hours(72),  // 72小时后过期
            published_as_version_id: None,
            published_by: None,
            published_at: None,
            version_snapshot,
            diff_items,
            kpi_summary: Some(kpi_summary),
        };

        // 2.5 保存到数据库
        db.decision_strategy_draft.insert(&draft)?;

        // 2.6 记录操作日志
        db.action_log.insert(ActionLog {
            action_id: Uuid::new_v4().to_string(),
            action_type: "GENERATE_DRAFT".to_string(),
            operator: operator.to_string(),
            timestamp: now,
            description: format!("Generated draft {} for strategy {}", draft_id, strategy),
            draft_id: Some(draft_id.clone()),
            ..Default::default()
        })?;

        draft_records.push(draft);
    }

    Ok(GenerateStrategyDraftsResponse {
        drafts: draft_records,
        total_count: strategies.len() as i32,
        generated_at: now,
    })
}
```

#### 2.3 新增apply_strategy_draft()

```rust
#[tauri::command]
pub fn apply_strategy_draft(
    draft_id: &str,
    version_name: String,
    parameters: Option<StrategyParameters>,  // 可选：微调参数
    note: String,
    operator: &str,
) -> ApiResult<ApplyStrategyDraftResponse> {
    // 1. 查询草案
    let mut draft = db.decision_strategy_draft.find_by_id(draft_id)?;

    if draft.status != "DRAFT" {
        return Err(ApiError::InvalidDraftStatus(format!(
            "Draft status is {}, expected DRAFT",
            draft.status
        )));
    }

    if draft.expires_at < Utc::now() {
        draft.status = "EXPIRED".to_string();
        db.decision_strategy_draft.update(&draft)?;
        return Err(ApiError::DraftExpired);
    }

    // 2. 并发保护：检查draft是否被锁定
    if let Some(locked_by) = &draft.locked_by {
        if locked_by != operator {
            return Err(ApiError::DraftLockedByOther(locked_by.clone()));
        }
    } else {
        // 锁定draft
        draft.locked_by = Some(operator.to_string());
        draft.locked_at = Some(Utc::now());
        db.decision_strategy_draft.update(&draft)?;
    }

    // 3. 如果提供了参数微调，需要重新计算
    let final_snapshot = if let Some(params) = parameters {
        engine.recalc_with_parameters(
            &draft.version_snapshot,
            &draft.strategy_type,
            params,
        )?
    } else {
        draft.version_snapshot.clone()
    };

    // 4. 创建新版本
    let new_version_id = format!("{}-{}",
        draft.strategy_type.replace('_', '-'),
        chrono::Local::now().format("%m%d-%H%M").to_string()
    );

    let new_version = PlanVersion {
        version_id: new_version_id.clone(),
        plan_id: draft.base_version_id.clone(),
        version_name,
        status: "INACTIVE".to_string(),  // 初始为inactive，需手动激活
        created_by: operator.to_string(),
        created_at: Utc::now(),
        strategy_used: Some(draft.strategy_type.clone()),
        note,
        items: final_snapshot.items.clone(),
        ..Default::default()
    };

    db.plan_version.insert(&new_version)?;

    // 5. 更新draft状态
    draft.status = "PUBLISHED".to_string();
    draft.published_as_version_id = Some(new_version_id.clone());
    draft.published_by = Some(operator.to_string());
    draft.published_at = Some(Utc::now());
    db.decision_strategy_draft.update(&draft)?;

    // 6. 记录操作日志
    db.action_log.insert(ActionLog {
        action_id: Uuid::new_v4().to_string(),
        action_type: "PUBLISH_DRAFT".to_string(),
        operator: operator.to_string(),
        timestamp: Utc::now(),
        description: format!("Published draft {} as version {}", draft_id, new_version_id),
        draft_id: Some(draft_id.to_string()),
        version_id: Some(new_version_id.clone()),
        ..Default::default()
    })?;

    Ok(ApplyStrategyDraftResponse {
        version_id: new_version_id,
        version_name: new_version.version_name,
        draft_id: draft_id.to_string(),
        published_at: Utc::now(),
    })
}
```

#### 2.4 新增list_strategy_drafts()

```rust
#[tauri::command]
pub fn list_strategy_drafts(
    base_version_id: String,
    status_filter: Option<String>,
) -> ApiResult<Vec<StrategyDraftRecordWithId>> {
    let mut query = db.decision_strategy_draft
        .where_base_version_id(&base_version_id);

    if let Some(status) = status_filter {
        query = query.where_status(&status);
    }

    let drafts = query
        .order_by_created_at_desc()
        .limit(100)
        .fetch_all()?;

    Ok(drafts)
}
```

#### 2.5 新增cleanup_expired_drafts()

```rust
#[tauri::command]
pub fn cleanup_expired_drafts() -> ApiResult<i32> {
    let now = Utc::now();
    let expired = db.decision_strategy_draft
        .where_status("DRAFT")
        .where_expires_at_before(now)
        .update(UpdateDraft {
            status: Some("EXPIRED".to_string()),
            ..Default::default()
        })?;

    Ok(expired)
}
```

**注意**: 可考虑在应用启动时自动调用cleanup_expired_drafts()

---

### Step 3: 前端类型定义更新 (2小时)

**文件**: `src/types/comparison.ts`

```typescript
// 替换现有的StrategyDraft定义

export interface StrategyDraftRecordWithId {
  // 唯一标识和状态
  draft_id: string;                    // ✅ 关键字段
  base_version_id: string;
  strategy_type: StrategyType;
  status: 'DRAFT' | 'PUBLISHED' | 'EXPIRED';  // ✅ 新增状态

  // 时间信息
  created_at: Date;                   // ✅ 新增
  created_by: string;                 // ✅ 新增
  expires_at: Date;                   // ✅ 新增

  // 发布信息
  published_as_version_id?: string;
  published_by?: string;
  published_at?: Date;

  // 日期范围
  plan_date_from: string;
  plan_date_to: string;

  // 排产数据
  version_snapshot: PlanVersion;
  diff_items: VersionDiffItem[];
  kpi_summary?: KPISummary;           // ✅ 新增
}

export interface KPISummary {
  l3_completion_rate: number;
  l2_completion_rate: number;
  capacity_utilization: number;
  capacity_overflow: number;
  cold_stock_count: number;
}

export type StrategyType = 'balanced' | 'urgent_first' | 'capacity_first' | 'cold_stock_first';

export interface GenerateStrategyDraftsResponse {
  drafts: StrategyDraftRecordWithId[];
  total_count: number;
  generated_at: Date;
}

export interface ApplyStrategyDraftResponse {
  version_id: string;
  version_name: string;
  draft_id: string;
  published_at: Date;
}
```

---

### Step 4: 前端API层更新 (2小时)

**文件**: `src/api/tauri.ts`

```typescript
// 更新planApi中的方法

export const planApi = {
  // ... 其他方法保留

  // 改进的生成草案方法
  async generate_strategy_drafts(
    baseVersionId: string,
    planDateFrom: string,
    planDateTo: string,
    strategies: StrategyType[],
    operator: string,
  ): Promise<GenerateStrategyDraftsResponse> {
    return invoke('generate_strategy_drafts', {
      base_version_id: baseVersionId,
      plan_date_from: planDateFrom,
      plan_date_to: planDateTo,
      strategies,
      operator,
    })
  },

  // 新增：应用草案为版本
  async apply_strategy_draft(
    draftId: string,
    versionName: string,
    parameters?: StrategyParameters,
    note?: string,
    operator?: string,
  ): Promise<ApplyStrategyDraftResponse> {
    return invoke('apply_strategy_draft', {
      draft_id: draftId,
      version_name: versionName,
      parameters,
      note,
      operator,
    })
  },

  // 新增：查询草案列表
  async list_strategy_drafts(
    baseVersionId: string,
    statusFilter?: 'DRAFT' | 'PUBLISHED' | 'EXPIRED',
  ): Promise<StrategyDraftRecordWithId[]> {
    return invoke('list_strategy_drafts', {
      base_version_id: baseVersionId,
      status_filter: statusFilter,
    })
  },

  // 新增：清理过期草案
  async cleanup_expired_drafts(): Promise<number> {
    return invoke('cleanup_expired_drafts', {})
  },
}
```

---

### Step 5: 前端Store改进 (2小时)

**文件**: `src/stores/use-plan-store.ts`

```typescript
import create from 'zustand'
import { StrategyDraftRecordWithId } from '@/types/comparison'

interface PlanState {
  // 现有字段保留
  plans: Plan[]
  selectedPlanId: string | null
  versions: PlanVersion[]
  selectedVersionId: string | null

  // ✅ 新增：策略草案管理
  draftVersions: StrategyDraftRecordWithId[]
  selectedDraftId: string | null
  isGeneratingDrafts: boolean
  isPublishingDraft: boolean
  draftError: string | null

  // Actions
  setPlans: (plans: Plan[]) => void
  // ... 其他actions保留

  // ✅ 新增：Draft相关actions
  setDraftVersions: (drafts: StrategyDraftRecordWithId[]) => void
  setSelectedDraftId: (id: string | null) => void
  addDraftVersion: (draft: StrategyDraftRecordWithId) => void
  removeDraftVersion: (draftId: string) => void
  updateDraftVersion: (draft: StrategyDraftRecordWithId) => void
  generateDrafts: (
    baseVersionId: string,
    dateFrom: string,
    dateTo: string,
    strategies: string[],
    operator: string,
  ) => Promise<StrategyDraftRecordWithId[]>
  publishDraft: (
    draftId: string,
    versionName: string,
    operator: string,
  ) => Promise<string>
  loadDrafts: (baseVersionId: string) => Promise<void>
  cleanupExpiredDrafts: () => Promise<void>
}

export const usePlanStore = create<PlanState>((set, get) => ({
  // 现有state保留
  plans: [],
  selectedPlanId: null,
  versions: [],
  selectedVersionId: null,

  // ✅ 新增state
  draftVersions: [],
  selectedDraftId: null,
  isGeneratingDrafts: false,
  isPublishingDraft: false,
  draftError: null,

  // 现有actions保留
  setPlans: (plans) => set({ plans }),

  // ✅ 新增actions
  setDraftVersions: (drafts) => set({ draftVersions: drafts }),
  setSelectedDraftId: (id) => set({ selectedDraftId: id }),

  addDraftVersion: (draft) => set((state) => ({
    draftVersions: [draft, ...state.draftVersions],
  })),

  removeDraftVersion: (draftId) => set((state) => ({
    draftVersions: state.draftVersions.filter((d) => d.draft_id !== draftId),
    selectedDraftId: state.selectedDraftId === draftId ? null : state.selectedDraftId,
  })),

  updateDraftVersion: (draft) => set((state) => ({
    draftVersions: state.draftVersions.map((d) =>
      d.draft_id === draft.draft_id ? draft : d,
    ),
  })),

  generateDrafts: async (baseVersionId, dateFrom, dateTo, strategies, operator) => {
    set({ isGeneratingDrafts: true, draftError: null })
    try {
      const response = await planApi.generate_strategy_drafts(
        baseVersionId,
        dateFrom,
        dateTo,
        strategies,
        operator,
      )
      set({ draftVersions: response.drafts })
      return response.drafts
    } catch (error) {
      const message = error instanceof Error ? error.message : '生成策略草案失败'
      set({ draftError: message })
      throw error
    } finally {
      set({ isGeneratingDrafts: false })
    }
  },

  publishDraft: async (draftId, versionName, operator) => {
    set({ isPublishingDraft: true, draftError: null })
    try {
      const response = await planApi.apply_strategy_draft(
        draftId,
        versionName,
        undefined,
        '从策略草案发布',
        operator,
      )

      // 更新draft状态为PUBLISHED
      const draft = get().draftVersions.find((d) => d.draft_id === draftId)
      if (draft) {
        get().updateDraftVersion({
          ...draft,
          status: 'PUBLISHED',
          published_as_version_id: response.version_id,
          published_at: response.published_at,
        })
      }

      // 自动加载新版本到versions列表
      const updatedVersions = await planApi.listVersions(
        draft?.base_version_id || '',
      )
      set({ versions: updatedVersions })

      return response.version_id
    } catch (error) {
      const message = error instanceof Error ? error.message : '发布策略草案失败'
      set({ draftError: message })
      throw error
    } finally {
      set({ isPublishingDraft: false })
    }
  },

  loadDrafts: async (baseVersionId) => {
    try {
      const drafts = await planApi.list_strategy_drafts(baseVersionId)
      set({ draftVersions: drafts })
    } catch (error) {
      console.error('Failed to load drafts:', error)
    }
  },

  cleanupExpiredDrafts: async () => {
    try {
      await planApi.cleanup_expired_drafts()
      // 刷新草案列表
      const state = get()
      if (state.versions.length > 0) {
        // 清理本地过期草案
        set({
          draftVersions: state.draftVersions.filter(
            (d) => d.status !== 'EXPIRED' && new Date(d.expires_at) > new Date(),
          ),
        })
      }
    } catch (error) {
      console.error('Failed to cleanup drafts:', error)
    }
  },
}))
```

---

### Step 6: 前端Hook实现 (2小时)

**文件**: `src/hooks/useStrategyDraft.ts` (新建)

```typescript
import { useCallback, useEffect } from 'react'
import { usePlanStore } from '@/stores/use-plan-store'
import { useGlobalStore } from '@/stores/use-global-store'
import { planApi } from '@/api/tauri'

export const useStrategyDraft = () => {
  const activeVersionId = useGlobalStore((s) => s.activeVersionId)
  const draftVersions = usePlanStore((s) => s.draftVersions)
  const isGeneratingDrafts = usePlanStore((s) => s.isGeneratingDrafts)
  const isPublishingDraft = usePlanStore((s) => s.isPublishingDraft)
  const draftError = usePlanStore((s) => s.draftError)

  const generateDrafts = usePlanStore((s) => s.generateDrafts)
  const publishDraft = usePlanStore((s) => s.publishDraft)
  const loadDrafts = usePlanStore((s) => s.loadDrafts)
  const setSelectedDraftId = usePlanStore((s) => s.setSelectedDraftId)
  const currentUser = useGlobalStore((s) => s.currentUser)

  // 获取当前有效的草案
  const validDrafts = useCallback(() => {
    return draftVersions.filter((d) => {
      const expiresAt = new Date(d.expires_at)
      return d.status === 'DRAFT' && expiresAt > new Date()
    })
  }, [draftVersions])

  // 检测过期草案
  useEffect(() => {
    const timer = setInterval(() => {
      const expired = draftVersions.some((d) => {
        const expiresAt = new Date(d.expires_at)
        return d.status === 'DRAFT' && expiresAt <= new Date()
      })

      if (expired) {
        // 刷新草案列表，自动清理过期的
        if (activeVersionId) {
          loadDrafts(activeVersionId)
        }
      }
    }, 30000) // 每30秒检查一次

    return () => clearInterval(timer)
  }, [draftVersions, activeVersionId, loadDrafts])

  // 初始化加载草案
  useEffect(() => {
    if (activeVersionId) {
      loadDrafts(activeVersionId)
    }
  }, [activeVersionId, loadDrafts])

  return {
    // 状态
    draftVersions: validDrafts(),
    allDrafts: draftVersions,
    isGeneratingDrafts,
    isPublishingDraft,
    error: draftError,

    // 方法
    generateDrafts: useCallback(
      async (dateFrom: string, dateTo: string, strategies: string[]) => {
        if (!activeVersionId || !currentUser) {
          throw new Error('缺少必要信息')
        }
        return generateDrafts(activeVersionId, dateFrom, dateTo, strategies, currentUser)
      },
      [activeVersionId, currentUser, generateDrafts],
    ),

    publishDraft: useCallback(
      async (draftId: string, versionName: string) => {
        if (!currentUser) throw new Error('缺少用户信息')
        return publishDraft(draftId, versionName, currentUser)
      },
      [currentUser, publishDraft],
    ),

    selectDraft: setSelectedDraftId,
    reloadDrafts: () => activeVersionId && loadDrafts(activeVersionId),
  }
}
```

---

### Step 7: 前端UI集成 (2小时)

**文件**: `src/components/comparison/StrategyDraftComparison.tsx` (改进)

```typescript
import React, { useState, useEffect } from 'react'
import { Skeleton, Button, Space, Tag, Alert, Spin, message } from 'antd'
import { useStrategyDraft } from '@/hooks/useStrategyDraft'
import { useGlobalStore } from '@/stores/use-global-store'

export const StrategyDraftComparison: React.FC = () => {
  const {
    draftVersions,
    isGeneratingDrafts,
    isPublishingDraft,
    error,
    generateDrafts,
    publishDraft,
    selectDraft,
  } = useStrategyDraft()

  const [selectedDraftId, setSelectedDraftId] = useState<string | null>(null)
  const [newVersionName, setNewVersionName] = useState('')
  const currentUser = useGlobalStore((s) => s.currentUser)

  // 处理策略生成
  const handleGenerateDrafts = async () => {
    try {
      await generateDrafts(
        '2026-01-20',
        '2026-01-26',
        ['balanced', 'urgent_first', 'capacity_first', 'cold_stock_first'],
      )
      message.success('策略草案生成成功，共4个')
    } catch (err) {
      message.error(`生成失败: ${err}`)
    }
  }

  // 处理草案发布
  const handlePublishDraft = async (draftId: string) => {
    if (!newVersionName.trim()) {
      message.error('请输入版本名称')
      return
    }

    try {
      await publishDraft(draftId, newVersionName)
      message.success('版本发布成功')
      setNewVersionName('')
    } catch (err) {
      message.error(`发布失败: ${err}`)
    }
  }

  // 草案过期警告
  const getExpirationInfo = (draft: any) => {
    const expiresAt = new Date(draft.expires_at)
    const now = new Date()
    const hoursLeft = Math.floor((expiresAt.getTime() - now.getTime()) / 3600000)

    if (hoursLeft <= 0) {
      return { status: 'expired', text: '已过期' }
    }
    if (hoursLeft <= 1) {
      return { status: 'warning', text: `即将过期 (${hoursLeft}小时后)` }
    }
    return { status: 'normal', text: `${hoursLeft}小时后过期` }
  }

  if (isGeneratingDrafts) {
    return <Spin tip="正在生成策略草案，请稍候..." />
  }

  return (
    <div style={{ padding: '24px' }}>
      <h2>策略草案对比</h2>

      {error && (
        <Alert
          type="error"
          message={`错误: ${error}`}
          closable
          style={{ marginBottom: '16px' }}
        />
      )}

      {draftVersions.length === 0 ? (
        <>
          <Alert
            type="info"
            message="暂无策略草案，点击下方按钮生成"
            style={{ marginBottom: '16px' }}
          />
          <Button type="primary" size="large" onClick={handleGenerateDrafts}>
            生成4种预设策略
          </Button>
        </>
      ) : (
        <>
          <Space style={{ marginBottom: '16px' }}>
            <span>已生成 {draftVersions.length} 个策略草案</span>
            <Button onClick={handleGenerateDrafts}>
              重新生成
            </Button>
          </Space>

          {/* 草案卡片列表 */}
          <div style={{ display: 'grid', gridTemplateColumns: 'repeat(2, 1fr)', gap: '16px' }}>
            {draftVersions.map((draft) => {
              const expInfo = getExpirationInfo(draft)
              const isSelected = selectedDraftId === draft.draft_id

              return (
                <div
                  key={draft.draft_id}
                  style={{
                    border: isSelected ? '2px solid #1890ff' : '1px solid #d9d9d9',
                    padding: '16px',
                    borderRadius: '4px',
                    cursor: 'pointer',
                    backgroundColor: isSelected ? '#f0f5ff' : '#fff',
                  }}
                  onClick={() => {
                    setSelectedDraftId(draft.draft_id)
                    selectDraft(draft.draft_id)
                  }}
                >
                  <div style={{ marginBottom: '8px' }}>
                    <strong>{draft.strategy_type}</strong>
                    <Tag
                      color={expInfo.status === 'normal' ? 'green' : 'orange'}
                      style={{ marginLeft: '8px' }}
                    >
                      {expInfo.text}
                    </Tag>
                  </div>

                  {/* KPI汇总展示 */}
                  {draft.kpi_summary && (
                    <div style={{ fontSize: '12px', color: '#666', marginBottom: '8px' }}>
                      <p>L3完成率: {(draft.kpi_summary.l3_completion_rate * 100).toFixed(1)}%</p>
                      <p>利用率: {(draft.kpi_summary.capacity_utilization * 100).toFixed(1)}%</p>
                      <p>冷坨数: {draft.kpi_summary.cold_stock_count}</p>
                    </div>
                  )}

                  {isSelected && (
                    <>
                      <input
                        type="text"
                        placeholder="输入版本名称 (如: 均衡方案-0129)"
                        value={newVersionName}
                        onChange={(e) => setNewVersionName(e.target.value)}
                        style={{
                          width: '100%',
                          padding: '8px',
                          marginBottom: '8px',
                          border: '1px solid #d9d9d9',
                          borderRadius: '4px',
                        }}
                      />
                      <Button
                        type="primary"
                        block
                        loading={isPublishingDraft}
                        onClick={() => handlePublishDraft(draft.draft_id)}
                      >
                        发布为正式版本
                      </Button>
                    </>
                  )}
                </div>
              )
            })}
          </div>
        </>
      )}
    </div>
  )
}
```

---

### Step 8: 单元测试 (2小时)

**文件**: `tests/strategy_draft_test.rs` (新建)

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::plan_api::*;

    #[tokio::test]
    async fn test_generate_strategy_drafts() {
        // 初始化测试数据库
        let db = setup_test_db().await;
        let base_version = create_test_version(&db).await;

        // 执行
        let response = generate_strategy_drafts(
            &base_version.version_id,
            "2026-01-20".to_string(),
            "2026-01-26".to_string(),
            vec!["balanced".to_string(), "urgent_first".to_string()],
            "test_user",
        ).await.unwrap();

        // 验证
        assert_eq!(response.drafts.len(), 2);
        assert_eq!(response.total_count, 2);

        for draft in &response.drafts {
            assert!(!draft.draft_id.is_empty());
            assert_eq!(draft.status, "DRAFT");
            assert_eq!(draft.created_by, "test_user");
            assert!(draft.expires_at > Utc::now());
        }
    }

    #[tokio::test]
    async fn test_apply_strategy_draft() {
        let db = setup_test_db().await;
        let base_version = create_test_version(&db).await;

        // 创建草案
        let response = generate_strategy_drafts(
            &base_version.version_id,
            "2026-01-20".to_string(),
            "2026-01-26".to_string(),
            vec!["balanced".to_string()],
            "test_user",
        ).await.unwrap();

        let draft_id = &response.drafts[0].draft_id;

        // 发布草案
        let publish_response = apply_strategy_draft(
            draft_id,
            "均衡方案-0129".to_string(),
            None,
            "Test publish".to_string(),
            "test_user",
        ).await.unwrap();

        // 验证
        assert!(!publish_response.version_id.is_empty());
        assert_eq!(publish_response.draft_id, *draft_id);

        // 验证draft状态已更新
        let updated_draft = db.decision_strategy_draft.find_by_id(draft_id).await.unwrap();
        assert_eq!(updated_draft.status, "PUBLISHED");
        assert_eq!(updated_draft.published_as_version_id, Some(publish_response.version_id));
    }

    #[tokio::test]
    async fn test_draft_expiration() {
        let db = setup_test_db().await;
        let base_version = create_test_version(&db).await;

        // 创建即将过期的草案
        let response = generate_strategy_drafts(...).await.unwrap();
        let draft_id = &response.drafts[0].draft_id;

        // 模拟时间推进
        travel_time_hours(73).await;

        // 验证过期检查
        let result = apply_strategy_draft(
            draft_id,
            "Test".to_string(),
            None,
            "".to_string(),
            "test_user",
        ).await;

        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), ApiError::DraftExpired));
    }
}
```

---

### 验收清单

修复1完成验收标准：

- [ ] 后端: decision_strategy_draft表创建成功
- [ ] 后端: generate_strategy_drafts() 返回draft_id
- [ ] 后端: apply_strategy_draft() 成功发布为新版本
- [ ] 后端: list_strategy_drafts() 可查询草案列表
- [ ] 前端: StrategyDraft类型包含draft_id和状态
- [ ] 前端: useStrategyDraft Hook正常工作
- [ ] 前端: StrategyDraftComparison 组件显示4个草案卡片
- [ ] 测试: 单元测试通过率100%
- [ ] 集成测试: 完整的生成→发布流程正常

---

# 修复2: 决策数据刷新通知 🔴 P0

## 问题再述

后端异步刷新 decision_* 读模型，前端无法知道何时完成，导致显示过期数据。

## 短期方案: 轮询检查

### Step 1: 后端新增refresh_status查询API (3小时)

**文件**: `src/api/decision_api.rs`

```rust
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct RefreshStatus {
    pub is_refreshing: bool,
    pub progress: i32,              // 0-100
    pub started_at: DateTime<Utc>,
    pub estimated_completion: Option<DateTime<Utc>>,
    pub last_completed_at: Option<DateTime<Utc>>,
    pub error: Option<String>,      // 刷新失败原因
}

#[tauri::command]
pub fn get_refresh_status(version_id: &str) -> ApiResult<RefreshStatus> {
    // 查询decision_refresh_log表获取最新状态
    let log = db.decision_refresh_log
        .where_version_id(version_id)
        .order_by_timestamp_desc()
        .first()?;

    let status = RefreshStatus {
        is_refreshing: log.status == "IN_PROGRESS",
        progress: log.progress.unwrap_or(0),
        started_at: log.timestamp,
        estimated_completion: log.status == "IN_PROGRESS"
            ? Some(log.timestamp + Duration::seconds(30))
            : None,
        last_completed_at: if log.status == "COMPLETED" {
            Some(log.timestamp)
        } else {
            None
        },
        error: if log.status == "FAILED" {
            Some(log.error_message.clone().unwrap_or_default())
        } else {
            None
        },
    };

    Ok(status)
}
```

### Step 2: 前端轮询Hook (2小时)

**文件**: `src/hooks/useDecisionRefresh.ts` (新建)

```typescript
import { useCallback, useRef, useState, useEffect } from 'react'
import { dashboardApi } from '@/api/tauri'

interface RefreshStatus {
  is_refreshing: boolean
  progress: number
  started_at: Date
  estimated_completion?: Date
  last_completed_at?: Date
  error?: string
}

export const useDecisionRefresh = (versionId: string | null) => {
  const [refreshStatus, setRefreshStatus] = useState<RefreshStatus | null>(null)
  const pollIntervalRef = useRef<NodeJS.Timeout | null>(null)
  const [isPolling, setIsPolling] = useState(false)

  // 检查刷新状态
  const checkRefreshStatus = useCallback(async () => {
    if (!versionId) return

    try {
      const status = await dashboardApi.getRefreshStatus(versionId)
      setRefreshStatus(status)

      // 如果刷新完成，停止轮询
      if (!status.is_refreshing) {
        setIsPolling(false)
        if (pollIntervalRef.current) {
          clearInterval(pollIntervalRef.current)
          pollIntervalRef.current = null
        }

        // 触发数据刷新事件
        if (status.error) {
          console.error('刷新失败:', status.error)
        } else {
          // 发送自定义事件，通知其他组件刷新数据
          window.dispatchEvent(
            new CustomEvent('decision-refresh-completed', {
              detail: { versionId, completedAt: status.last_completed_at },
            }),
          )
        }
      }
    } catch (error) {
      console.error('获取刷新状态失败:', error)
    }
  }, [versionId])

  // 启动轮询
  const startPolling = useCallback(() => {
    if (isPolling) return

    setIsPolling(true)
    checkRefreshStatus()

    // 每秒检查一次
    pollIntervalRef.current = setInterval(checkRefreshStatus, 1000)
  }, [checkRefreshStatus, isPolling])

  // 停止轮询
  const stopPolling = useCallback(() => {
    setIsPolling(false)
    if (pollIntervalRef.current) {
      clearInterval(pollIntervalRef.current)
      pollIntervalRef.current = null
    }
  }, [])

  // 清理
  useEffect(() => {
    return () => {
      if (pollIntervalRef.current) {
        clearInterval(pollIntervalRef.current)
      }
    }
  }, [])

  return {
    refreshStatus,
    isPolling,
    startPolling,
    stopPolling,
    checkRefreshStatus,
  }
}
```

### Step 3: 前端UI集成 (2小时)

**文件**: `src/components/overview/DecisionRefreshIndicator.tsx` (新建)

```typescript
import React, { useEffect } from 'react'
import { Progress, Alert, Spin } from 'antd'
import { useDecisionRefresh } from '@/hooks/useDecisionRefresh'
import { useGlobalStore } from '@/stores/use-global-store'

export const DecisionRefreshIndicator: React.FC = () => {
  const activeVersionId = useGlobalStore((s) => s.activeVersionId)
  const { refreshStatus, isPolling, startPolling, stopPolling } = useDecisionRefresh(
    activeVersionId,
  )

  // 监听导入/重算事件，启动刷新轮询
  useEffect(() => {
    const handleRefreshNeeded = () => {
      startPolling()
    }

    window.addEventListener('decision-refresh-needed', handleRefreshNeeded)
    return () => window.removeEventListener('decision-refresh-needed', handleRefreshNeeded)
  }, [startPolling])

  if (!refreshStatus || !isPolling) {
    return null
  }

  return (
    <div style={{ marginBottom: '16px' }}>
      {refreshStatus.is_refreshing ? (
        <>
          <Alert
            type="info"
            message={`正在刷新决策数据... ${refreshStatus.progress}%`}
            closable={false}
            style={{ marginBottom: '8px' }}
          />
          <Progress
            percent={refreshStatus.progress}
            status="active"
            showInfo={true}
          />
        </>
      ) : null}

      {refreshStatus.error && (
        <Alert
          type="error"
          message={`刷新失败: ${refreshStatus.error}`}
          closable={true}
          style={{ marginTop: '8px' }}
        />
      )}

      {!refreshStatus.is_refreshing && refreshStatus.last_completed_at && (
        <Alert
          type="success"
          message={`决策数据已更新 (${new Date(
            refreshStatus.last_completed_at,
          ).toLocaleTimeString()})`}
          closable={true}
        />
      )}
    </div>
  )
}
```

### 在RiskOverview中集成

**文件**: `src/pages/RiskOverview.tsx`

```typescript
import { DecisionRefreshIndicator } from '@/components/overview/DecisionRefreshIndicator'
import { useGlobalKPI } from '@/hooks/useGlobalKPI'
import { useQueryClient } from '@tanstack/react-query'

export const RiskOverview: React.FC = () => {
  const queryClient = useQueryClient()
  const { data, isLoading } = useGlobalKPI()

  // 监听刷新完成事件
  useEffect(() => {
    const handleRefreshCompleted = () => {
      // 无效化所有相关查询，自动触发重新fetch
      queryClient.invalidateQueries(['riskOverview'])
      queryClient.invalidateQueries(['globalKPI'])
      queryClient.invalidateQueries(['decisionDay'])
    }

    window.addEventListener(
      'decision-refresh-completed',
      handleRefreshCompleted,
    )
    return () =>
      window.removeEventListener('decision-refresh-completed', handleRefreshCompleted)
  }, [queryClient])

  return (
    <div style={{ padding: '24px' }}>
      {/* 刷新状态指示器 */}
      <DecisionRefreshIndicator />

      {/* 其他内容 */}
      {isLoading ? <Skeleton /> : <KPIBand data={data} />}
    </div>
  )
}
```

---

修复2完成验收标准：

- [ ] 后端: get_refresh_status() API可用
- [ ] 前端: useDecisionRefresh Hook轮询正常
- [ ] 前端: 导入完成后自动显示刷新进度
- [ ] 前端: 刷新完成后自动刷新页面数据
- [ ] UI: 显示刷新进度条和状态提示

---

# 修复3: 版本对比KPI聚合API 🔴 P1

## 改进方案

**后端** (src/api/plan_api.rs - 12小时):

```rust
#[derive(Serialize, Deserialize)]
pub struct KPIComparisonResult {
    pub l3_completion_rate: (f64, f64),     // (before, after)
    pub l2_completion_rate: (f64, f64),
    pub capacity_utilization: (f64, f64),
    pub capacity_overflow: (f64, f64),
    pub cold_stock_count: (i32, i32),
    pub urgent_items_scheduled: (i32, i32),
    pub delta: KPIDelta,
}

#[derive(Serialize, Deserialize)]
pub struct KPIDelta {
    pub l3_delta: i32,          // +20% or -5%
    pub l2_delta: i32,
    pub util_delta: f64,
    pub overflow_delta: f64,
}

#[tauri::command]
pub fn compare_versions_kpi(
    version_id_a: &str,
    version_id_b: &str,
) -> ApiResult<KPIComparisonResult> {
    // 查询两个版本的KPI数据
    // 计算变化delta
    // 返回对比结果
}
```

**前端** (src/components/comparison/KPIComparisonPanel.tsx - 6小时):

```typescript
export const KPIComparisonPanel: React.FC<{ versionA, versionB }> = ({
  versionA,
  versionB,
}) => {
  const [kpiComparison, setKpiComparison] = useState(null)

  useEffect(() => {
    planApi.compare_versions_kpi(versionA.id, versionB.id).then(setKpiComparison)
  }, [versionA, versionB])

  return (
    <Table
      columns={[
        { title: '指标', dataIndex: 'metric' },
        { title: versionA.name, dataIndex: 'valueA' },
        { title: versionB.name, dataIndex: 'valueB' },
        { title: '变化', dataIndex: 'delta' },
      ]}
      dataSource={transformKpiData(kpiComparison)}
    />
  )
}
```

---

## 总结

三个P0修复的总工作量：

| 修复项 | 后端 | 前端 | 总计 |
|--------|------|------|------|
| Draft持久化 | 16h | 8h | 24h |
| 刷新通知 | 3h | 4h | 7h |
| KPI对比API | 12h | 6h | 18h |
| **总计** | **31h** | **18h** | **49h** |

**建议分配**:
- 第1-2周: Draft + 刷新通知 (31h)
- 第7-8周: KPI对比API (18h)

完成这三个修复后，重构方案的可行性将从70%提升到95%以上。

