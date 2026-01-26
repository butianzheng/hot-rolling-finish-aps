// ==========================================
// 风险仪表盘数据类型
// ==========================================

export interface DangerDayData {
  date: string;
  riskLevel: 'low' | 'medium' | 'high' | 'critical';
  capacityOverflow: number; // 产能溢出（吨）
  urgentBacklog: number; // L3 积压数量
  reasons: string[]; // 风险原因列表
}

export interface BlockedUrgentOrder {
  materialId: string;
  urgentLevel: string;
  daysBlocked: number;
  reason: string;
  machineCode: string;
}

export interface ColdStockMaterial {
  materialId: string;
  stockAgeDays: number;
  targetAgeDays: number;
  weight: number;
  steelMark: string;
}

export interface RollCampaignHealth {
  machineCode: string;
  currentTonnage: number;
  threshold: number; // 1500 或 2500
  status: 'healthy' | 'warning' | 'critical';
  estimatedRollsRemaining: number;
}
