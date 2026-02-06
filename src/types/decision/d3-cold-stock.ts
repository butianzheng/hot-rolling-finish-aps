// ==========================================
// D3: 冷料压库概况 TypeScript 类型定义
// ==========================================
// 对应 Rust: src/decision/api/dto.rs
// ColdStockProfileResponse, ColdStockBucketDto
// ==========================================

import type { ReasonItem } from './d1-day-summary';

// ==========================================
// D3 请求类型
// ==========================================

/**
 * D3 请求: 查询冷料压库概况
 */
export interface GetColdStockProfileRequest {
  /** 方案版本 ID */
  versionId: string;

  /** 机组代码过滤（可选） */
  machineCodes?: string[];

  /** 压力等级过滤（可选） */
  pressureLevelFilter?: PressureLevel[];

  /** 年龄分桶过滤（可选） */
  ageBinFilter?: AgeBin[];

  /** 返回条数限制（可选） */
  limit?: number;
}

// ==========================================
// D3 响应类型
// ==========================================

/**
 * 压力等级枚举
 */
export type PressureLevel = 'LOW' | 'MEDIUM' | 'HIGH' | 'CRITICAL';

/**
 * 年龄分桶枚举
 */
export type AgeBin =
  | '0-7' // 0-7天
  | '8-14' // 8-14天
  | '15-30' // 15-30天
  | '30+'; // 30天以上

/**
 * 趋势方向枚举
 */
export type TrendDirection = 'RISING' | 'STABLE' | 'FALLING';

/**
 * 结构性缺口
 *
 * 后端目前返回的是“描述性文本”（例如："集中在规格 1250毫米×3.5毫米 (10 块)" 或 "无"），
 * 并非严格的枚举值，因此这里使用 string 表示。
 */
export type StructureGap = string;

/**
 * 冷料趋势数据对象
 */
export interface ColdStockTrend {
  /** 趋势方向 */
  direction: TrendDirection;

  /** 变化率 (%) */
  changeRatePct: number;

  /** 基线天数 */
  baselineDays: number;
}

/**
 * 冷料分桶数据对象
 */
export interface ColdStockBucket {
  /** 机组代码 */
  machineCode: string;

  /** 年龄分桶 */
  ageBin: AgeBin;

  /** 数量 */
  count: number;

  /** 重量 (t) */
  weightT: number;

  /** 压力分数 (0-100) */
  pressureScore: number;

  /** 压力等级 */
  pressureLevel: PressureLevel;

  /** 平均库龄（天） */
  avgAgeDays: number;

  /** 最大库龄（天） */
  maxAgeDays: number;

  /** 结构性缺口 */
  structureGap: StructureGap;

  /** 原因列表 */
  reasons: ReasonItem[];

  /** 趋势（可选） */
  trend?: ColdStockTrend;
}

/**
 * 机组库存统计数据对象
 */
export interface MachineStockStats {
  machineCode: string;
  count: number;
  weightT: number;
  avgPressureScore: number;
}

/**
 * 年龄分桶统计数据对象
 */
export interface AgeBinStats {
  ageBin: AgeBin;
  count: number;
  weightT: number;
}

/**
 * 冷料摘要数据对象
 */
export interface ColdStockSummary {
  /** 冷料总数量 */
  totalColdStockCount: number;

  /** 冷料总重量 (t) */
  totalColdStockWeightT: number;

  /** 平均库龄（天） */
  avgAgeDays: number;

  /** 高压力数量 */
  highPressureCount: number;

  /** 按机组统计 */
  byMachine: MachineStockStats[];

  /** 按年龄分桶统计 */
  byAgeBin: AgeBinStats[];
}

/**
 * D3 响应: 冷料压库概况
 */
export interface ColdStockProfileResponse {
  versionId: string;
  asOf: string;
  items: ColdStockBucket[];
  totalCount: number;
  summary: ColdStockSummary;
}

// ==========================================
// 辅助类型（用于UI展示）
// ==========================================

/**
 * 压力等级颜色映射
 */
export const PRESSURE_LEVEL_COLORS: Record<PressureLevel, string> = {
  LOW: '#52c41a', // 绿色
  MEDIUM: '#1677ff', // 蓝色
  HIGH: '#faad14', // 橙色
  CRITICAL: '#ff4d4f', // 红色
};

/**
 * 压力等级中文名称映射
 */
export const PRESSURE_LEVEL_LABELS: Record<PressureLevel, string> = {
  LOW: '低压力',
  MEDIUM: '中压力',
  HIGH: '高压力',
  CRITICAL: '极高压力',
};

/**
 * 年龄分桶中文名称映射
 */
export const AGE_BIN_LABELS: Record<AgeBin, string> = {
  '0-7': '0-7天',
  '8-14': '8-14天',
  '15-30': '15-30天',
  '30+': '30天以上',
};

/**
 * 年龄分桶颜色映射（渐变色）
 */
export const AGE_BIN_COLORS: Record<AgeBin, string> = {
  '0-7': '#52c41a',
  '8-14': '#1677ff',
  '15-30': '#faad14',
  '30+': '#ff4d4f',
};

/**
 * 趋势方向图标映射
 */
export const TREND_DIRECTION_ICONS: Record<TrendDirection, string> = {
  RISING: '↑',
  STABLE: '→',
  FALLING: '↓',
};

/**
 * 趋势方向颜色映射
 */
export const TREND_DIRECTION_COLORS: Record<TrendDirection, string> = {
  RISING: '#ff4d4f', // 上升（恶化）- 红色
  STABLE: '#1677ff', // 稳定 - 蓝色
  FALLING: '#52c41a', // 下降（好转）- 绿色
};

/**
 * 结构性缺口中文名称映射（兼容旧枚举值）
 */
export const STRUCTURE_GAP_LABELS: Record<string, string> = {
  SIZE_MISMATCH: '尺寸不匹配',
  GRADE_CONFLICT: '钢种冲突',
  NO_CAMPAIGN: '无换辊计划',
  CAPACITY_FULL: '产能满载',
  NONE: '无缺口',
  无: '无缺口',
};

/**
 * 获取压力等级颜色
 */
export function getPressureLevelColor(level: PressureLevel): string {
  return PRESSURE_LEVEL_COLORS[level];
}

/**
 * 获取压力等级标签
 */
export function getPressureLevelLabel(level: PressureLevel): string {
  return PRESSURE_LEVEL_LABELS[level];
}

/**
 * 获取年龄分桶标签
 */
export function getAgeBinLabel(bin: AgeBin): string {
  return AGE_BIN_LABELS[bin];
}

/**
 * 获取年龄分桶颜色
 */
export function getAgeBinColor(bin: AgeBin): string {
  return AGE_BIN_COLORS[bin];
}

/**
 * 获取趋势图标
 */
export function getTrendIcon(direction: TrendDirection): string {
  return TREND_DIRECTION_ICONS[direction];
}

/**
 * 获取趋势颜色
 */
export function getTrendColor(direction: TrendDirection): string {
  return TREND_DIRECTION_COLORS[direction];
}

/**
 * 获取结构性缺口标签
 */
export function getStructureGapLabel(gap: StructureGap): string {
  const trimmed = (gap || '').trim();
  if (!trimmed || trimmed === 'NONE' || trimmed === '无') {
    return '无缺口';
  }
  return STRUCTURE_GAP_LABELS[trimmed] ?? trimmed;
}

/**
 * 判断是否为高压力冷料
 */
export function isHighPressure(bucket: ColdStockBucket): boolean {
  return bucket.pressureLevel === 'HIGH' || bucket.pressureLevel === 'CRITICAL';
}

/**
 * 判断是否为长库龄材料
 */
export function isOldStock(bucket: ColdStockBucket): boolean {
  return bucket.ageBin === '30+' || bucket.avgAgeDays > 30;
}

/**
 * 按压力分数降序排序
 */
export function sortByPressureScore(a: ColdStockBucket, b: ColdStockBucket): number {
  return b.pressureScore - a.pressureScore;
}

/**
 * 转换为堆积柱状图数据格式（用于ECharts）
 */
export function toStackedBarData(buckets: ColdStockBucket[]): {
  machines: string[];
  ageBins: AgeBin[];
  data: Record<AgeBin, number[]>;
} {
  const machines = Array.from(new Set(buckets.map((b) => b.machineCode))).sort();
  const ageBins: AgeBin[] = ['0-7', '8-14', '15-30', '30+'];

  const data: Record<AgeBin, number[]> = {
    '0-7': [],
    '8-14': [],
    '15-30': [],
    '30+': [],
  };

  machines.forEach((machine) => {
    ageBins.forEach((bin) => {
      const bucket = buckets.find(
        (b) => b.machineCode === machine && b.ageBin === bin
      );
      data[bin].push(bucket ? bucket.weightT : 0);
    });
  });

  return { machines, ageBins, data };
}
