// ==========================================================
// Dashboard API 响应 Schema（优先级1 - D1-D6决策看板）
// ==========================================================
// P2-1 修复：从 decision-schema.ts 导入统一 schema，避免双重定义
// 保留 .passthrough() 以向后兼容（宽松解析）
//
// 注意：decision.ts 是真实源，这里只是 re-export 并添加 .passthrough()

import {
  CapacityOpportunityResponseSchema as CapacityOpportunityResponseSchemaStrict,
  ColdStockProfileResponseSchema as ColdStockProfileResponseSchemaStrict,
  DecisionDaySummaryResponseSchema as DecisionDaySummaryResponseSchemaStrict,
  MachineBottleneckProfileResponseSchema as MachineBottleneckProfileResponseSchemaStrict,
  OrderFailureSetResponseSchema as OrderFailureSetResponseSchemaStrict,
  RollCampaignAlertResponseSchema as RollCampaignAlertResponseSchemaStrict,
} from './decision';

// D1: 哪天最危险
export const DecisionDaySummaryResponseSchema = DecisionDaySummaryResponseSchemaStrict.passthrough();

// D2: 哪些紧急单无法完成
export const OrderFailureSetResponseSchema = OrderFailureSetResponseSchemaStrict.passthrough();

// D3: 哪些冷料压库
export const ColdStockProfileResponseSchema = ColdStockProfileResponseSchemaStrict.passthrough();

// D4: 哪个机组最堵
export const MachineBottleneckProfileResponseSchema =
  MachineBottleneckProfileResponseSchemaStrict.passthrough();

// D5: 换辊是否异常 (保留 plural 命名以兼容旧代码)
export const RollCampaignAlertsResponseSchema = RollCampaignAlertResponseSchemaStrict.passthrough();

// D6: 是否存在产能优化空间
export const CapacityOpportunityResponseSchema = CapacityOpportunityResponseSchemaStrict.passthrough();

