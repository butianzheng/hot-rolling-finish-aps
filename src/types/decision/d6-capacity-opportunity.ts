// ==========================================
// D6: 容量机会 TypeScript 类型定义
// ==========================================
// 对应 Rust: src/decision/api/dto.rs
// CapacityOpportunityResponse, CapacityOpportunityDto
// ==========================================

// ==========================================
// D6 请求类型
// ==========================================

/**
 * D6 请求: 查询容量优化机会
 */
export interface GetCapacityOpportunityRequest {
  /** 方案版本 ID（必填） */
  versionId: string;

  /** 期望计划修订号（可选，用于防陈旧读取） */
  expectedPlanRev?: number;

  /** 日期范围起始（可选，ISO DATE: YYYY-MM-DD） */
  dateFrom?: string;

  /** 日期范围结束（可选，ISO DATE: YYYY-MM-DD） */
  dateTo?: string;

  /** 机组代码过滤（可选，为空则查询所有机组） */
  machineCodes?: string[];

  /** 机会类型过滤（可选） */
  opportunityTypeFilter?: string[];

  /** 最小机会吨位阈值（吨，可选，默认 10.0） */
  minOpportunityT?: number;

  /** 返回条数限制（可选） */
  limit?: number;
}

// ==========================================
// D6 响应类型 (对齐 Rust DTO: src/decision/api/dto.rs L466-519)
// ==========================================

// 导入通用类型（避免重复定义）
import type { TypeCount } from './d2-order-failure';

/**
 * 容量优化机会类型（用于 UI 显示）
 */
export type OpportunityType =
  | 'UNDERUTILIZED'    // 未充分利用 - 容量使用率低
  | 'MOVABLE_LOAD'     // 可移动负载 - 可以调整材料到这个日期
  | 'STRUCTURE_FIX'    // 结构优化 - 可以优化结构比例
  | 'URGENT_INSERTION' // 紧急插入 - 可以插入紧急订单
  | 'LOAD_BALANCE';    // 负载均衡 - 可以分散其他日期的负载

/**
 * 场景 DTO
 * 对应 Rust: ScenarioDto
 */
export interface Scenario {
  name: string;
  adjustment: string;
  utilPct: number;
  riskScore: number;
  affectedMaterialCount: number;
}

/**
 * 敏感性分析 DTO
 * 对应 Rust: SensitivityAnalysisDto
 */
export interface SensitivityAnalysis {
  scenarios: Scenario[];
  bestScenarioIndex: number;
}

/**
 * 容量优化机会 DTO
 * 对应 Rust: CapacityOpportunityDto (L477-492)
 */
export interface CapacityOpportunity {
  /** 机组代码 */
  machineCode: string;

  /** 排产日期 */
  planDate: string;

  /** 机会类型 */
  opportunityType: string;

  /** 当前利用率（百分比） */
  currentUtilPct: number;

  /** 目标容量（吨） */
  targetCapacityT: number;

  /** 已使用容量（吨） */
  usedCapacityT: number;

  /** 机会空间（吨） */
  opportunitySpaceT: number;

  /** 优化后利用率（百分比） */
  optimizedUtilPct: number;

  /** 敏感性分析（可选） */
  sensitivity?: SensitivityAnalysis;

  /** 描述 */
  description: string;

  /** 建议操作列表 */
  recommendedActions: string[];

  /** 潜在收益列表 */
  potentialBenefits: string[];
}

/**
 * 容量优化机会摘要 DTO
 * 对应 Rust: CapacityOpportunitySummaryDto (L512-518)
 */
export interface CapacityOpportunitySummary {
  /** 机会总数 */
  totalOpportunities: number;

  /** 总机会空间（吨） */
  totalOpportunitySpaceT: number;

  /** 按类型统计 */
  byType: TypeCount[];

  /** 平均当前利用率（百分比） */
  avgCurrentUtilPct: number;

  /** 平均优化后利用率（百分比） */
  avgOptimizedUtilPct: number;
}

/**
 * D6 响应: 容量优化机会响应
 * 对应 Rust: CapacityOpportunityResponse (L466-474)
 */
export interface CapacityOpportunityResponse {
  /** 方案版本 ID */
  versionId: string;

  /** 数据截止时间 */
  asOf: string;

  /** 机会项列表 */
  items: CapacityOpportunity[];

  /** 记录总数 */
  totalCount: number;

  /** 摘要统计 */
  summary: CapacityOpportunitySummary;
}

// ==========================================
// 兼容类型（用于旧代码过渡）
// ==========================================

/**
 * 旧版单日容量机会（兼容性别名）
 * @deprecated 使用 CapacityOpportunity 代替
 */
export type DailyCapacityOpportunity = CapacityOpportunity;

// ==========================================
// 辅助函数
// ==========================================

/**
 * 判断容量是否未充分利用
 */
export function isUnderutilized(utilizationPct: number): boolean {
  return utilizationPct < 70; // 利用率低于70%视为未充分利用
}

/**
 * 判断是否为高优先级机会
 */
export function isHighPriorityOpportunity(priorityScore: number): boolean {
  return priorityScore >= 70;
}

/**
 * 获取机会类型颜色
 */
export const OPPORTUNITY_TYPE_COLORS: Record<OpportunityType, string> = {
  UNDERUTILIZED: '#52c41a',    // 绿色
  MOVABLE_LOAD: '#1677ff',     // 蓝色
  STRUCTURE_FIX: '#722ed1',    // 紫色
  URGENT_INSERTION: '#ff4d4f', // 红色
  LOAD_BALANCE: '#faad14',     // 橙色
};

/**
 * 获取机会类型标签
 */
export const OPPORTUNITY_TYPE_LABELS: Record<OpportunityType, string> = {
  UNDERUTILIZED: '未充分利用',
  MOVABLE_LOAD: '可移动负载',
  STRUCTURE_FIX: '结构优化',
  URGENT_INSERTION: '紧急插入',
  LOAD_BALANCE: '负载均衡',
};

/**
 * 根据利用率获取状态颜色
 */
export function getUtilizationColor(utilizationPct: number): string {
  if (utilizationPct < 60) return '#52c41a';  // 绿色 - 容量充裕
  if (utilizationPct < 80) return '#1677ff';  // 蓝色 - 正常
  if (utilizationPct < 100) return '#faad14'; // 橙色 - 接近满载
  return '#ff4d4f'; // 红色 - 超载
}
