/**
 * PlanManagement 导出相关类型定义
 */

import type { BackendVersionComparisonResult, VersionDiff } from '../../types/comparison';
import type { LocalCapacityDeltaRow, LocalVersionDiffSummary } from '../comparison/types';

/**
 * 容量池数据接口
 */
export interface CapacityPool {
  machine_code: string;
  plan_date: string;
  target_capacity_t: number | null;
  limit_capacity_t: number | null;
}

/**
 * 本地差异结果
 */
export interface LocalDiffResult {
  diffs: VersionDiff[];
  summary: LocalVersionDiffSummary;
}

/**
 * 本地容量差异行数据
 */
export type LocalCapacityRowType = LocalCapacityDeltaRow;

/**
 * 本地容量基础数据
 */
export interface LocalCapacityRowsBase {
  rows: LocalCapacityRowType[];
  dateFrom: string | null;
  dateTo: string | null;
  machines: string[];
  totalA: number;
  totalB: number;
}

/**
 * 本地容量完整数据（包含溢出行）
 */
export interface LocalCapacityRowsComplete extends LocalCapacityRowsBase {
  rows: LocalCapacityRowType[];
  overflowRows: LocalCapacityRowType[];
}

/**
 * 导出上下文
 */
export interface ExportContext {
  compareResult: BackendVersionComparisonResult;
  currentUser: string;
  localDiffResult: LocalDiffResult | null;
  localCapacityRows: LocalCapacityRowsComplete | null;
  retrospectiveNote: string;
}

/**
 * 配置变化项
 */
export interface ConfigChange {
  key: string;
  value_a: string | null;
  value_b: string | null;
}
