// ==========================================
// D1: 日期风险摘要 TypeScript 类型定义
// ==========================================
// 对应 Rust: src/decision/api/dto.rs
// DecisionDaySummaryResponse, DaySummaryDto
// ==========================================

// ==========================================
// 通用类型（被多个决策使用）
// ==========================================

/**
 * 原因项（风险原因、阻塞原因等）
 */
export interface ReasonItem {
  /** 原因代码 */
  code: string;

  /** 原因描述 */
  msg: string;

  /** 权重 (0-1) */
  weight: number;

  /** 影响的材料数量（可选） */
  affectedCount?: number;
}

// ==========================================
// D1 请求类型
// ==========================================

/**
 * D1 请求: 查询日期风险摘要
 */
export interface GetDecisionDaySummaryRequest {
  /** 方案版本 ID（必填） */
  versionId: string;

  /** 日期范围起始（必填，ISO DATE: YYYY-MM-DD） */
  dateFrom: string;

  /** 日期范围结束（必填，ISO DATE: YYYY-MM-DD） */
  dateTo: string;

  /** 风险等级过滤（可选） */
  riskLevelFilter?: ('LOW' | 'MEDIUM' | 'HIGH' | 'CRITICAL')[];

  /** 返回条数限制（可选，默认 10） */
  limit?: number;

  /** 排序方式（可选，默认按风险分数降序） */
  sortBy?: 'risk_score' | 'plan_date' | 'capacity_util_pct';
}

// ==========================================
// D1 响应类型
// ==========================================

/**
 * 风险等级枚举
 */
export type RiskLevel = 'LOW' | 'MEDIUM' | 'HIGH' | 'CRITICAL';

/**
 * 日期摘要 DTO
 */
export interface DaySummary {
  /** 计划日期 (YYYY-MM-DD) */
  planDate: string;

  /** 风险分数 (0-100) */
  riskScore: number;

  /** 风险等级 */
  riskLevel: RiskLevel;

  /** 产能利用率 (%) */
  capacityUtilPct: number;

  /** 超载吨数 (t) */
  overloadWeightT: number;

  /** 紧急单失败数量 */
  urgentFailureCount: number;

  /** 主要风险原因（按权重降序） */
  topReasons: ReasonItem[];

  /** 涉及的机组列表 */
  involvedMachines: string[];
}

/**
 * D1 响应: 日期风险摘要列表
 */
export interface DecisionDaySummaryResponse {
  /** 方案版本 ID */
  versionId: string;

  /** 查询时间戳 (ISO 8601) */
  asOf: string;

  /** 日期摘要列表 */
  items: DaySummary[];

  /** 总记录数 */
  totalCount: number;
}

// ==========================================
// 辅助类型（用于UI展示）
// ==========================================

/**
 * 风险等级颜色映射
 */
export const RISK_LEVEL_COLORS: Record<RiskLevel, string> = {
  LOW: '#52c41a', // 绿色
  MEDIUM: '#1677ff', // 蓝色
  HIGH: '#faad14', // 橙色
  CRITICAL: '#ff4d4f', // 红色
};

/**
 * 风险等级中文名称映射
 */
export const RISK_LEVEL_LABELS: Record<RiskLevel, string> = {
  LOW: '低风险',
  MEDIUM: '中风险',
  HIGH: '高风险',
  CRITICAL: '严重风险',
};

/**
 * 获取风险等级颜色
 */
export function getRiskLevelColor(level: RiskLevel): string {
  return RISK_LEVEL_COLORS[level];
}

/**
 * 获取风险等级标签
 */
export function getRiskLevelLabel(level: RiskLevel): string {
  return RISK_LEVEL_LABELS[level];
}

/**
 * 判断是否为高风险日期
 */
export function isHighRiskDay(daySummary: DaySummary): boolean {
  return daySummary.riskLevel === 'HIGH' || daySummary.riskLevel === 'CRITICAL';
}

/**
 * 按风险分数降序排序
 */
export function sortByRiskScore(a: DaySummary, b: DaySummary): number {
  return b.riskScore - a.riskScore;
}
