/**
 * VersionComparisonModal 类型定义
 */

import type { EChartsOption } from 'echarts';
import type { BackendVersionComparisonResult, VersionDiff } from '../../types/comparison';
import { LocalCapacityDeltaRow, LocalVersionDiffSummary } from './localTypes';

export interface VersionComparisonModalProps {
  // 显示状态
  open: boolean;
  onClose: () => void;

  // 后端对比结果
  compareResult: BackendVersionComparisonResult | null;
  compareKpiRows: Array<{ key: string; metric: string; a: string; b: string; delta: string }>;
  compareKpiLoading?: boolean;
  compareKpiError?: Error | null;

  // 本地差异计算
  localDiffResult: {
    diffs: VersionDiff[];
    summary: LocalVersionDiffSummary;
  } | null;
  loadLocalCompareDetail: boolean;
  planItemsLoading?: boolean;
  planItemsErrorA?: Error | null;
  planItemsErrorB?: Error | null;

  // 产能分析
  localCapacityRows: {
    rows: LocalCapacityDeltaRow[];
    overflowRows: LocalCapacityDeltaRow[];
    totalA: number;
    totalB: number;
    dateFrom: string | null;
    dateTo: string | null;
    machines: string[];
  } | null;
  localCapacityRowsBase: {
    rows: LocalCapacityDeltaRow[];
    totalA: number;
    totalB: number;
    dateFrom: string | null;
    dateTo: string | null;
    machines: string[];
  } | null;
  capacityPoolsErrorA?: Error | null;
  capacityPoolsErrorB?: Error | null;
  showAllCapacityRows?: boolean;

  // 回顾性笔记
  retrospectiveNote?: string;
  retrospectiveSavedAt?: string | null;

  // 搜索和过滤
  diffSearchText?: string;
  diffTypeFilter?: 'ALL' | 'ADDED' | 'REMOVED' | 'MOVED' | 'MODIFIED';

  // 图表选项
  diffSummaryChartOption?: EChartsOption | null;
  capacityTrendOption?: EChartsOption | null;
  riskTrendOption?: EChartsOption | null;

  // 回调
  onActivateVersion?: (versionId: string) => Promise<void>;
  onLoadLocalCompareDetail?: () => void;
  onToggleShowAllCapacityRows?: () => void;
  onRetrospectiveNoteChange?: (note: string) => void;
  onRetrospectiveNoteSave?: () => void;
  onDiffSearchChange?: (text: string) => void;
  onDiffTypeFilterChange?: (type: 'ALL' | 'ADDED' | 'REMOVED' | 'MOVED' | 'MODIFIED') => void;
  onExportDiffs?: (format: 'csv' | 'json') => Promise<void>;
  onExportCapacity?: (format: 'csv' | 'json') => Promise<void>;
  onExportReport?: (format: 'json' | 'markdown' | 'html') => Promise<void>;
}

// 重导出 localTypes
export type { LocalCapacityDeltaRow, LocalVersionDiffSummary } from './localTypes';
