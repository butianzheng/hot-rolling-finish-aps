// ==========================================
// D2: 订单失败集合 TypeScript 类型定义
// ==========================================
// 对应 Rust: src/decision/api/dto.rs
// OrderFailureSetResponse, OrderFailureDto
// ==========================================

// ==========================================
// D2 请求类型
// ==========================================

/**
 * D2 请求: 查询紧急订单失败集合
 */
export interface ListOrderFailureSetRequest {
  /** 方案版本 ID（必填） */
  versionId: string;

  /** 期望计划修订号（可选，用于防陈旧读取） */
  expectedPlanRev?: number;

  /** 失败类型过滤（可选） */
  failTypeFilter?: FailType[];

  /** 紧急等级过滤（可选） */
  urgencyLevelFilter?: UrgencyLevel[];

  /** 机组代码过滤（可选） */
  machineCodes?: string[];

  /** 交货日期范围起始（可选） */
  dueDateFrom?: string;

  /** 交货日期范围结束（可选） */
  dueDateTo?: string;

  /** 完成率阈值过滤（可选） */
  completionRateThreshold?: number;

  /** 分页：限制条数（可选，默认 50） */
  limit?: number;

  /** 分页：偏移量（可选，默认 0） */
  offset?: number;
}

/**
 * D2M 请求: 查询材料失败集合（材料维度）
 */
export interface ListMaterialFailureSetRequest {
  /** 方案版本 ID（必填） */
  versionId: string;

  /** 期望计划修订号（可选，用于防陈旧读取） */
  expectedPlanRev?: number;

  /** 失败类型过滤（可选） */
  failTypeFilter?: FailType[];

  /** 紧急等级过滤（可选） */
  urgencyLevelFilter?: UrgencyLevel[];

  /** 机组代码过滤（可选） */
  machineCodes?: string[];

  /** 交货日期范围起始（可选） */
  dueDateFrom?: string;

  /** 交货日期范围结束（可选） */
  dueDateTo?: string;

  /** 完成率阈值过滤（可选） */
  completionRateThreshold?: number;

  /** 问题范围（可选，默认 UNSCHEDULED_ONLY） */
  problemScope?: MaterialFailureProblemScope;

  /** 仅看未排产材料（可选） */
  onlyUnscheduled?: boolean;

  /** 分页：限制条数（可选，默认 50） */
  limit?: number;

  /** 分页：偏移量（可选，默认 0） */
  offset?: number;
}

// ==========================================
// D2 响应类型
// ==========================================

/**
 * 失败类型枚举
 */
export type FailType =
  | 'Overdue' // 超期未完成
  | 'NearDueImpossible' // 临期无法完成
  | 'CapacityShortage' // 产能不足
  | 'StructureConflict' // 结构冲突
  | 'ColdStockNotReady' // 冷料未适温
  | 'Other' // 其他
  | (string & {}); // 允许后端扩展新类型，前端做兜底展示

/**
 * 紧急等级枚举（分层制，非评分）
 */
export type UrgencyLevel = 'L0' | 'L1' | 'L2' | 'L3';

/**
 * D2M 问题范围
 */
export type MaterialFailureProblemScope = 'UNSCHEDULED_ONLY' | 'DUE_WINDOW_CRITICAL';

/**
 * 阻塞因素 DTO
 */
export interface BlockingFactor {
  /** 因素类型 */
  factorType: string;

  /** 描述 */
  description: string;

  /** 影响权重 */
  impact: number;

  /** 受影响的材料数量 */
  affectedMaterialCount: number;
}

/**
 * 订单失败 DTO
 */
export interface OrderFailure {
  /** 合同号 */
  contractNo: string;

  /** 主材料号（优先未排产材料，用于精确定位） */
  materialId?: string;

  /** 交期 */
  dueDate: string;

  /** 距离交期天数 */
  daysToDue: number;

  /** 紧急等级 */
  urgencyLevel: UrgencyLevel;

  /** 失败类型 */
  failType: FailType;

  /** 完成率 (%)，后端返回 0-100 */
  completionRate: number;

  /** 总重量 (t) */
  totalWeightT: number;

  /** 已排产重量 (t) */
  scheduledWeightT: number;

  /** 未排产重量 (t) */
  unscheduledWeightT: number;

  /** 机组代码 */
  machineCode: string;

  /** 阻塞因素 */
  blockingFactors: BlockingFactor[];

  /** 失败原因 */
  failureReasons: string[];

  /** 推荐操作（可选） */
  recommendedActions?: string[];
}

/**
 * 类型统计 DTO
 */
export interface TypeCount {
  typeName: string;
  count: number;
  weightT: number;
}

/**
 * 订单失败摘要 DTO
 */
export interface OrderFailureSummary {
  /** 总失败数 */
  totalFailures: number;

  /** 按失败类型统计 */
  byFailType: TypeCount[];

  /** 按紧急度统计 */
  byUrgency: TypeCount[];

  /** 未排产总重量 (t) */
  totalUnscheduledWeightT: number;
}

/**
 * D2 响应: 订单失败集合
 */
export interface OrderFailureSetResponse {
  versionId: string;
  asOf: string;
  items: OrderFailure[];
  totalCount: number;
  summary: OrderFailureSummary;
}

/**
 * 材料失败 DTO（材料维度）
 */
export interface MaterialFailure {
  materialId: string;
  contractNo: string;
  dueDate: string;
  daysToDue: number;
  urgencyLevel: UrgencyLevel;
  failType: FailType;
  completionRate: number;
  weightT: number;
  unscheduledWeightT: number;
  machineCode: string;
  isScheduled: boolean;
  blockingFactors: BlockingFactor[];
  failureReasons: string[];
  recommendedActions?: string[];
}

/**
 * 材料失败摘要 DTO
 */
export interface MaterialFailureSummary {
  totalFailedMaterials: number;
  totalFailedContracts: number;
  overdueMaterials: number;
  unscheduledMaterials: number;
  totalUnscheduledWeightT: number;
  byFailType: TypeCount[];
  byUrgency: TypeCount[];
}

/**
 * 材料失败-合同聚合 DTO
 */
export interface MaterialFailureContractAggregate {
  contractNo: string;
  materialCount: number;
  unscheduledCount: number;
  overdueCount: number;
  earliestDueDate: string;
  maxUrgencyLevel: UrgencyLevel;
  representativeMaterialId: string;
}

/**
 * D2M 响应: 材料失败集合
 */
export interface MaterialFailureSetResponse {
  versionId: string;
  asOf: string;
  items: MaterialFailure[];
  totalCount: number;
  summary: MaterialFailureSummary;
  contractAggregates: MaterialFailureContractAggregate[];
}

// ==========================================
// 辅助类型（用于UI展示）
// ==========================================

/**
 * 紧急等级颜色映射
 */
export const URGENCY_LEVEL_COLORS: Record<UrgencyLevel, string> = {
  L3: '#ff4d4f', // 红色 - 紧急/红线
  L2: '#faad14', // 橙色 - 高优先级
  L1: '#1677ff', // 蓝色 - 中等优先级
  L0: '#8c8c8c', // 灰色 - 正常
};

/**
 * 紧急等级中文名称映射
 */
export const URGENCY_LEVEL_LABELS: Record<UrgencyLevel, string> = {
  L3: '超紧急',
  L2: '紧急',
  L1: '较紧急',
  L0: '正常',
};

/**
 * 失败类型颜色映射
 */
export const FAIL_TYPE_COLORS: Record<FailType, string> = {
  Overdue: '#ff4d4f',
  NearDueImpossible: '#faad14',
  CapacityShortage: '#fa8c16',
  StructureConflict: '#1677ff',
  ColdStockNotReady: '#13c2c2',
  Other: '#8c8c8c',
};

/**
 * 失败类型中文名称映射
 */
export const FAIL_TYPE_LABELS: Record<FailType, string> = {
  Overdue: '超期未完成',
  NearDueImpossible: '临期无法完成',
  CapacityShortage: '产能不足',
  StructureConflict: '结构冲突',
  ColdStockNotReady: '冷料未适温',
  Other: '其他',
};

/**
 * 获取紧急等级颜色
 */
export function getUrgencyLevelColor(level: UrgencyLevel): string {
  return URGENCY_LEVEL_COLORS[level];
}

/**
 * 获取紧急等级标签
 */
export function getUrgencyLevelLabel(level: UrgencyLevel): string {
  return URGENCY_LEVEL_LABELS[level];
}

/**
 * 获取失败类型颜色
 */
export function getFailTypeColor(type: FailType): string {
  return FAIL_TYPE_COLORS[type];
}

/**
 * 获取失败类型标签
 */
export function getFailTypeLabel(type: FailType): string {
  return FAIL_TYPE_LABELS[type];
}

/**
 * 判断是否为严重失败
 */
export function isSevereFailure(order: OrderFailure): boolean {
  return order.failType === 'Overdue' || order.completionRate < 50;
}

/**
 * 格式化完成率为百分比
 */
export function formatCompletionRate(rate: number): string {
  return `${rate.toFixed(1)}%`;
}

/**
 * 按紧急等级排序（L0 > L1 > L2 > L3）
 */
export function sortByUrgencyLevel(a: OrderFailure, b: OrderFailure): number {
  const levelOrder: Record<UrgencyLevel, number> = { L3: 0, L2: 1, L1: 2, L0: 3 };
  return levelOrder[a.urgencyLevel] - levelOrder[b.urgencyLevel];
}
