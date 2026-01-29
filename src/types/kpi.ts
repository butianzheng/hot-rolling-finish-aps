// ==========================================
// 全局 KPI 数据类型定义
// ==========================================

export interface GlobalKPI {
  // 风险指标
  mostRiskyDate?: string;
  riskLevel?: 'low' | 'medium' | 'high' | 'critical';

  // 紧急订单统计
  urgentOrdersCount: number;
  blockedUrgentCount: number;
  urgentBreakdown?: {
    L2: { total: number; blocked: number };
    L3: { total: number; blocked: number };
  };

  // 产能利用率
  capacityUtilization: number; // 百分比

  // 冷库压力
  coldStockCount: number;

  // 轧辊状态
  rollCampaignProgress: number; // 当前吨位
  rollChangeThreshold: number; // 阈值 (1500 或 2500)
  rollStatus: 'healthy' | 'warning' | 'critical';
}

export const defaultKPI: GlobalKPI = {
  urgentOrdersCount: 0,
  blockedUrgentCount: 0,
  capacityUtilization: 0,
  coldStockCount: 0,
  rollCampaignProgress: 0,
  rollChangeThreshold: 1500,
  rollStatus: 'healthy',
};
