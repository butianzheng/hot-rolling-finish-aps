// ==========================================
// D4: 机组堵塞概况 TypeScript 类型定义
// ==========================================
// 对应 Rust: src/decision/api/dto.rs
// MachineBottleneckProfileResponse, BottleneckPointDto
// ==========================================

import type { ReasonItem } from './d1-day-summary';

// ==========================================
// D4 请求类型
// ==========================================

/**
 * D4 请求: 查询机组堵塞概况
 */
export interface GetMachineBottleneckProfileRequest {
  /** 方案版本 ID（必填） */
  versionId: string;

  /** 日期范围起始（必填，ISO DATE） */
  dateFrom: string;

  /** 日期范围结束（必填，ISO DATE） */
  dateTo: string;

  /** 机组代码过滤（可选，为空表示所有机组） */
  machineCodes?: string[];

  /** 堵塞等级过滤（可选） */
  bottleneckLevelFilter?: BottleneckLevel[];

  /** 堵塞类型过滤（可选） */
  bottleneckTypeFilter?: BottleneckType[];

  /** 返回条数限制（可选，默认 50） */
  limit?: number;
}

// ==========================================
// D4 响应类型
// ==========================================

/**
 * 堵塞等级枚举
 */
export type BottleneckLevel = 'NONE' | 'LOW' | 'MEDIUM' | 'HIGH' | 'CRITICAL';

/**
 * 堵塞类型枚举
 */
export type BottleneckType = 'Capacity' | 'Structure' | 'RollChange' | 'ColdStock' | 'Mixed';

/**
 * 堵塞点 DTO
 */
export interface BottleneckPoint {
  /** 机组代码 */
  machineCode: string;

  /** 计划日期 (YYYY-MM-DD) */
  planDate: string;

  /** 堵塞分数 (0-100) */
  bottleneckScore: number;

  /** 堵塞等级 */
  bottleneckLevel: BottleneckLevel;

  /** 堵塞类型列表 */
  bottleneckTypes: BottleneckType[];

  /** 产能利用率 (%) */
  capacityUtilPct: number;

  /** 待排产材料数量 */
  pendingMaterialCount: number;

  /** 待排产重量 (t) */
  pendingWeightT: number;

  /** 已排产材料数量 */
  scheduledMaterialCount: number;

  /** 已排产重量 (t) */
  scheduledWeightT: number;

  /** 堵塞原因（按影响降序） */
  reasons: ReasonItem[];

  /** 推荐操作（可选） */
  recommendedActions?: string[];
}

/**
 * 机组统计 DTO
 */
export interface MachineStats {
  machineCode: string;
  avgScore: number;
  maxScore: number;
  bottleneckDays: number;
}

/**
 * 热力图统计 DTO
 */
export interface HeatmapStats {
  /** 平均堵塞分数 */
  avgScore: number;

  /** 最大堵塞分数 */
  maxScore: number;

  /** 堵塞天数（分数 > 50） */
  bottleneckDaysCount: number;

  /** 按机组的统计 */
  byMachine: MachineStats[];
}

/**
 * D4 响应: 机组堵塞概况
 */
export interface MachineBottleneckProfileResponse {
  /** 方案版本 ID */
  versionId: string;

  /** 查询时间戳 */
  asOf: string;

  /** 堵塞点列表 */
  items: BottleneckPoint[];

  /** 总记录数 */
  totalCount: number;

  /** 热力图统计（可选，用于前端渲染） */
  heatmapStats?: HeatmapStats;
}

// ==========================================
// 辅助类型（用于UI展示）
// ==========================================

/**
 * 堵塞等级颜色映射
 */
export const BOTTLENECK_LEVEL_COLORS: Record<BottleneckLevel, string> = {
  NONE: '#d9d9d9', // 灰色
  LOW: '#52c41a', // 绿色
  MEDIUM: '#1677ff', // 蓝色
  HIGH: '#faad14', // 橙色
  CRITICAL: '#ff4d4f', // 红色
};

/**
 * 堵塞等级中文名称映射
 */
export const BOTTLENECK_LEVEL_LABELS: Record<BottleneckLevel, string> = {
  NONE: '无堵塞',
  LOW: '轻度提醒',
  MEDIUM: '中度提醒',
  HIGH: '堵塞',
  CRITICAL: '严重堵塞',
};

/**
 * 堵塞类型中文名称映射
 */
export const BOTTLENECK_TYPE_LABELS: Record<BottleneckType, string> = {
  Capacity: '产能不足',
  Structure: '结构性堵塞',
  RollChange: '换辊影响',
  ColdStock: '冷料压库',
  Mixed: '混合因素',
};

/**
 * 堵塞类型颜色映射
 */
export const BOTTLENECK_TYPE_COLORS: Record<BottleneckType, string> = {
  Capacity: '#ff4d4f',
  Structure: '#faad14',
  RollChange: '#1677ff',
  ColdStock: '#13c2c2',
  Mixed: '#722ed1',
};

/**
 * 获取堵塞等级颜色
 */
export function getBottleneckLevelColor(level: BottleneckLevel): string {
  return BOTTLENECK_LEVEL_COLORS[level];
}

/**
 * 获取堵塞等级标签
 */
export function getBottleneckLevelLabel(level: BottleneckLevel): string {
  return BOTTLENECK_LEVEL_LABELS[level];
}

/**
 * 获取堵塞类型标签
 */
export function getBottleneckTypeLabel(type: BottleneckType): string {
  return BOTTLENECK_TYPE_LABELS[type];
}

/**
 * 获取堵塞类型颜色
 */
export function getBottleneckTypeColor(type: BottleneckType): string {
  return BOTTLENECK_TYPE_COLORS[type];
}

/**
 * 判断是否为严重堵塞
 */
export function isSevereBottleneck(point: BottleneckPoint): boolean {
  return point.bottleneckLevel === 'HIGH' || point.bottleneckLevel === 'CRITICAL';
}

/**
 * 按堵塞分数降序排序
 */
export function sortByBottleneckScore(a: BottleneckPoint, b: BottleneckPoint): number {
  return b.bottleneckScore - a.bottleneckScore;
}

/**
 * 转换为热力图数据格式（用于ECharts）
 */
export function toHeatmapData(
  points: BottleneckPoint[]
): Array<[string, string, number]> {
  return points.map((point) => [
    point.planDate,
    point.machineCode,
    point.bottleneckScore,
  ]);
}
