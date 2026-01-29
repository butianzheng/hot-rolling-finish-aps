/**
 * Drilldown Drawer - 重导出分解后的组件
 *
 * 重构后：1113 行 → 分解为多个模块化组件
 * - index.tsx (221 行): 主容器组件
 * - shared.tsx (154 行): 共享工具和组件
 * - OrdersContent.tsx (87 行): 订单失败集合
 * - ColdStockContent.tsx (253 行): 冷坨高压力
 * - BottleneckContent.tsx (161 行): 堵塞矩阵
 * - RollAlertContent.tsx (111 行): 换辊警报
 * - RiskDayContent.tsx (154 行): 风险摘要
 * - CapacityOpportunityContent.tsx (185 行): 容量优化机会
 *
 * 主组件行数: 1113 → 221 (-80%)
 */

export { default } from './drilldown';
export * from './drilldown';
