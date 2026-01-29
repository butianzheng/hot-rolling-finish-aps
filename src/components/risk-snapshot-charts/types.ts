/**
 * 风险快照分析 - 类型定义
 */

export interface ReasonItem {
  code: string;
  msg: string;
  weight: number;
  affected_count?: number;
}

export interface RiskDaySummary {
  plan_date: string;
  risk_score: number;
  risk_level: string;
  capacity_util_pct: number;
  overload_weight_t: number;
  urgent_failure_count: number;
  top_reasons: ReasonItem[];
  involved_machines: string[];
}

export interface DecisionDaySummaryResponse {
  items: RiskDaySummary[];
}

export interface RiskSnapshotChartsProps {
  onNavigateToPlan?: () => void;
}

export interface VersionOption {
  value: string;
  label: string;
}

// 风险等级颜色映射
export const riskLevelColors: Record<string, string> = {
  CRITICAL: 'red',
  HIGH: 'volcano',
  MEDIUM: 'orange',
  LOW: 'green',
};
