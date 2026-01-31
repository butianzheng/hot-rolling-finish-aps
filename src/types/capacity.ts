// ==========================================
// 产能时间线数据类型
// ==========================================

export interface CapacitySegment {
  urgencyLevel: 'L0' | 'L1' | 'L2' | 'L3';
  tonnage: number; // 吨位
  materialCount: number; // 材料数量
}

export interface CapacityTimelineData {
  date: string;
  machineCode: string;
  targetCapacity: number; // 目标产能（吨）
  limitCapacity: number; // 限制产能（吨）
  actualCapacity: number; // 实际排产（吨）
  segments: CapacitySegment[]; // 按紧急度分段
  rollCampaignProgress: number; // 轧辊吨位进度
  rollChangeThreshold: number; // 轧辊更换阈值 (1500 或 2500)
  materialIds?: string[]; // 该日期包含的所有物料ID（用于高亮）
}
