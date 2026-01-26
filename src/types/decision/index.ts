// ==========================================
// 决策层类型定义 - 统一导出
// ==========================================

// D1: 日期风险摘要
export * from './d1-day-summary';

// D2: 订单失败集合 (TypeCount 首次定义在这里)
export * from './d2-order-failure';

// D3: 冷料压库概况
export * from './d3-cold-stock';

// D4: 机组堵塞概况
export * from './d4-bottleneck';

// D5: 轧制活动警报 (排除 TypeCount 避免冲突)
export type {
  GetRollCampaignAlertRequest,
  RollCampaignAlert,
  RollAlertSummary,
  RollCampaignAlertResponse,
  RollStatus,
} from './d5-roll-campaign';
export {
  parseAlertLevel,
  hasAlert,
  isSevereAlert,
  ROLL_STATUS_COLORS,
  ROLL_STATUS_LABELS,
  getAlertLevelColor,
  getAlertLevelLabel,
  calculateUtilization,
} from './d5-roll-campaign';

// D6: 容量优化机会 (排除 TypeCount 避免冲突)
export type {
  GetCapacityOpportunityRequest,
  OpportunityType,
  Scenario,
  SensitivityAnalysis,
  CapacityOpportunity,
  CapacityOpportunitySummary,
  CapacityOpportunityResponse,
  DailyCapacityOpportunity,
} from './d6-capacity-opportunity';
export {
  isUnderutilized,
  isHighPriorityOpportunity,
  OPPORTUNITY_TYPE_COLORS,
  OPPORTUNITY_TYPE_LABELS,
  getUtilizationColor,
} from './d6-capacity-opportunity';
