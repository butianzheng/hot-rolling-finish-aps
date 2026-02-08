// ==========================================
// D5: 轧制活动警报 TypeScript 类型定义
// ==========================================
// 对应 Rust: src/decision/api/dto.rs
// RollCampaignAlertResponse, RollCampaignAlertDto
// ==========================================

// ==========================================
// D5 请求类型
// ==========================================

/**
 * D5 请求: 查询轧制活动警报
 */
export interface GetRollCampaignAlertRequest {
  /** 方案版本 ID（必填） */
  versionId: string;

  /** 期望计划修订号（可选，用于防陈旧读取） */
  expectedPlanRev?: number;

  /** 机组代码过滤（可选，为空则查询所有机组） */
  machineCodes?: string[];

  /** 警报等级过滤（可选，如: ['WARNING', 'HARD_STOP']） */
  alertLevelFilter?: string[];

  /** 警报类型过滤（可选） */
  alertTypeFilter?: string[];

  /** 日期范围起始（可选，YYYY-MM-DD） */
  dateFrom?: string;

  /** 日期范围结束（可选，YYYY-MM-DD） */
  dateTo?: string;

  /** 返回条数限制（可选） */
  limit?: number;
}

// ==========================================
// D5 响应类型 (对齐 Rust DTO: src/decision/api/dto.rs L417-442)
// ==========================================

// 导入通用类型（避免重复定义）
import type { TypeCount } from './d2-order-failure';

/**
 * 换辊警报项（单个机组）
 * 对应 Rust: RollAlertDto
 */
export interface RollCampaignAlert {
  /** 机组代码 */
  machineCode: string;

  /** 换辊窗口ID */
  campaignId: string;

  /** 换辊窗口开始日期 */
  campaignStartDate: string;

  /** 当前累积吨位 */
  currentTonnageT: number;

  /** 建议阈值（软限制） */
  softLimitT: number;

  /** 强制上限（硬限制） */
  hardLimitT: number;

  /** 剩余吨位（距硬限制） */
  remainingTonnageT: number;

  /** 警报等级 */
  alertLevel: string;

  /** 警报类型 */
  alertType: string;

  /** 预计达到硬限制的日期 */
  estimatedHardStopDate: string | null;

  /** 当前周期起点（YYYY-MM-DD HH:MM:SS） */
  campaignStartAt?: string;

  /** 计划换辊时刻（YYYY-MM-DD HH:MM:SS，可人工微调） */
  plannedChangeAt?: string | null;

  /** 计划停机时长（分钟） */
  plannedDowntimeMinutes?: number;

  /** 预计触达软限制日期时间（YYYY-MM-DD HH:MM:SS） */
  estimatedSoftReachAt?: string | null;

  /** 预计触达硬限制日期时间（YYYY-MM-DD HH:MM:SS） */
  estimatedHardReachAt?: string | null;

  /** 警报消息 */
  alertMessage: string;

  /** 影响描述 */
  impactDescription: string;

  /** 建议操作列表 */
  recommendedActions: string[];
}

/**
 * 换辊警报摘要
 * 对应 Rust: RollAlertSummaryDto
 */
export interface RollAlertSummary {
  /** 警报总数 */
  totalAlerts: number;

  /** 按等级统计 */
  byLevel: TypeCount[];

  /** 按类型统计 */
  byType: TypeCount[];

  /** 接近硬停止的数量 */
  nearHardStopCount: number;
}

/**
 * D5 响应: 轧制活动警报响应
 * 对应 Rust: RollCampaignAlertsResponse
 */
export interface RollCampaignAlertResponse {
  /** 方案版本 ID */
  versionId: string;

  /** 数据截止时间 */
  asOf: string;

  /** 警报项列表 */
  items: RollCampaignAlert[];

  /** 记录总数 */
  totalCount: number;

  /** 摘要统计 */
  summary: RollAlertSummary;
}

// ==========================================
// 兼容类型（用于前端显示逻辑）
// ==========================================

/**
 * 换辊状态（基于 alertLevel 派生）
 */
export type RollStatus = 'NORMAL' | 'SUGGEST' | 'WARNING' | 'HARD_STOP';

/**
 * 将 alertLevel 转换为 RollStatus
 */
export function parseAlertLevel(alertLevel: string): RollStatus {
  const upper = String(alertLevel || '').toUpperCase();

  // D5 领域新口径（后端 decision_roll_campaign_alert.alert_level）:
  // NONE/WARNING/CRITICAL/EMERGENCY → NORMAL/SUGGEST/WARNING/HARD_STOP
  if (upper === 'EMERGENCY') return 'HARD_STOP';
  if (upper === 'CRITICAL') return 'WARNING';
  if (upper === 'WARNING') return 'SUGGEST';
  if (upper === 'NONE') return 'NORMAL';

  // 兼容旧口径/前端自定义状态
  if (upper === 'HARD_STOP') return 'HARD_STOP';
  if (upper === 'SUGGEST') return 'SUGGEST';
  if (upper === 'NORMAL') return 'NORMAL';

  // 兼容风险等级类枚举
  if (upper === 'HIGH') return 'WARNING';
  if (upper === 'MEDIUM') return 'SUGGEST';
  return 'NORMAL';
}

// ==========================================
// 辅助函数
// ==========================================

/**
 * 判断是否有警报
 */
export function hasAlert(status: RollStatus): boolean {
  return status !== 'NORMAL';
}

/**
 * 判断是否为严重警报
 */
export function isSevereAlert(status: RollStatus): boolean {
  return status === 'WARNING' || status === 'HARD_STOP';
}

/**
 * 获取换辊状态颜色
 */
export const ROLL_STATUS_COLORS: Record<RollStatus, string> = {
  NORMAL: '#52c41a',      // 绿色
  SUGGEST: '#1677ff',     // 蓝色
  WARNING: '#faad14',     // 橙色
  HARD_STOP: '#ff4d4f',   // 红色
};

/**
 * 获取警报等级颜色（基于 alertLevel 字符串）
 */
export function getAlertLevelColor(alertLevel: string): string {
  const status = parseAlertLevel(alertLevel);
  return ROLL_STATUS_COLORS[status];
}

/**
 * 获取换辊状态标签
 */
export const ROLL_STATUS_LABELS: Record<RollStatus, string> = {
  NORMAL: '正常',
  SUGGEST: '建议换辊',
  WARNING: '警告',
  HARD_STOP: '硬停止',
};

/**
 * 获取警报等级标签（基于 alertLevel 字符串）
 */
export function getAlertLevelLabel(alertLevel: string): string {
  const status = parseAlertLevel(alertLevel);
  return ROLL_STATUS_LABELS[status];
}

/**
 * 计算利用率百分比
 */
export function calculateUtilization(
  currentTonnage: number,
  softLimit: number
): number {
  if (softLimit <= 0) return 0;
  return Math.round((currentTonnage / softLimit) * 100);
}
