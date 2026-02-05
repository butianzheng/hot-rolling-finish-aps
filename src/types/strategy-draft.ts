/**
 * 策略草案类型定义
 * 从 StrategyDraftComparison.tsx 提取
 */

import type { z } from 'zod';
import type { MaterialMasterSchema, MaterialStateSchema } from '../api/ipcSchemas';

export type StrategyKey = string;

export type StrategyPreset = {
  key: StrategyKey;
  title: string;
  description: string;
  kind?: 'preset' | 'custom';
  base_strategy?: string;
  strategy_id?: string;
  /** 策略参数（JSON 结构，需运行时验证） */
  parameters?: Record<string, unknown>;
};

export type CustomStrategyProfile = {
  strategy_id: string;
  title: string;
  description?: string | null;
  base_strategy: string;
  /** 策略参数（JSON 结构，需运行时验证） */
  parameters?: Record<string, unknown>;
};

export type StrategyDraftSummary = {
  draft_id: string;
  base_version_id: string;
  strategy: StrategyKey;
  plan_items_count: number;
  frozen_items_count: number;
  calc_items_count: number;
  mature_count: number;
  immature_count: number;
  total_capacity_used_t: number;
  overflow_days: number;
  moved_count: number;
  added_count: number;
  removed_count: number;
  squeezed_out_count: number;
  message: string;
};

export type GenerateStrategyDraftsResponse = {
  base_version_id: string;
  plan_date_from: string;
  plan_date_to: string;
  drafts: StrategyDraftSummary[];
  message: string;
};

export type ListStrategyDraftsResponse = GenerateStrategyDraftsResponse;

export type ApplyStrategyDraftResponse = {
  version_id: string;
  success: boolean;
  message: string;
};

export type StrategyDraftDiffItem = {
  material_id: string;
  change_type: 'MOVED' | 'ADDED' | 'SQUEEZED_OUT' | string;
  from_plan_date?: string | null;
  from_machine_code?: string | null;
  from_seq_no?: number | null;
  to_plan_date?: string | null;
  to_machine_code?: string | null;
  to_seq_no?: number | null;
  to_assign_reason?: string | null;
  to_urgent_level?: string | null;
  to_sched_state?: string | null;
  material_state_snapshot?: MaterialStateSnapshot | null;
};

export type MaterialStateSnapshot = {
  sched_state?: string | null;
  urgent_level?: string | null;
  rush_level?: string | null;
  lock_flag?: boolean | null;
  force_release_flag?: boolean | null;
  manual_urgent_flag?: boolean | null;
  in_frozen_zone?: boolean | null;
  ready_in_days?: number | null;
  earliest_sched_date?: string | null;
  scheduled_date?: string | null;
  scheduled_machine_code?: string | null;
  seq_no?: number | null;
};

export type GetStrategyDraftDetailResponse = {
  draft_id: string;
  base_version_id: string;
  plan_date_from: string;
  plan_date_to: string;
  strategy: StrategyKey;
  diff_items: StrategyDraftDiffItem[];
  diff_items_total: number;
  diff_items_truncated: boolean;
  message: string;
};

/** 物料详情负载（用于临时序列化/传递） */
export type MaterialDetailPayload = {
  master: z.infer<typeof MaterialMasterSchema> | null;
  state: z.infer<typeof MaterialStateSchema> | null;
};

export type ActionLogRow = {
  action_id: string;
  version_id: string | null;
  action_type: string;
  action_ts: string;
  actor: string;
  /** 操作负载（JSON 结构，需运行时验证） */
  payload_json?: Record<string, unknown>;
  /** 影响汇总（JSON 结构，需运行时验证） */
  impact_summary_json?: Record<string, unknown>;
  machine_code?: string | null;
  date_range_start?: string | null;
  date_range_end?: string | null;
  detail?: string | null;
};

export type SqueezedHintCache = Record<
  string,
  { status: 'loading' | 'ready' | 'error'; sections?: Array<{ title: string; lines: string[] }>; error?: string }
>;

export type SqueezedHintSection = {
  title: string;
  lines: string[];
};

/** 预置策略列表（后端未返回时的 fallback） */
export const FALLBACK_STRATEGIES: StrategyPreset[] = [
  { key: 'balanced', title: '均衡方案', description: '在交付/产能/库存之间保持均衡', kind: 'preset' },
  { key: 'urgent_first', title: '紧急优先', description: '优先保障 L3/L2 紧急订单', kind: 'preset' },
  { key: 'capacity_first', title: '产能优先', description: '优先提升产能利用率，减少溢出', kind: 'preset' },
  { key: 'cold_stock_first', title: '冷料消化', description: '优先消化冷料/压库物料', kind: 'preset' },
];

/** 最大计划天数限制 */
export const MAX_DAYS = 60;

/** 生成自定义策略的 key */
export function makeCustomStrategyKey(strategyId: string): string {
  return `custom:${String(strategyId || '').trim()}`;
}
