/**
 * 风险仪表盘 - 重导出分解后的组件
 *
 * 重构后：452 行 → 分解为多个模块化组件
 * - index.tsx (~70 行): 主容器组件
 * - useRiskDashboard.ts (~120 行): 状态管理 Hook
 * - types.ts (~50 行): 类型定义和工具函数
 * - DangerDayCard.tsx (~70 行): 危险日期卡片
 * - BlockedOrdersCard.tsx (~65 行): 阻塞紧急订单卡片
 * - ColdStockCard.tsx (~110 行): 冷库压力卡片
 * - RollHealthCard.tsx (~70 行): 轧辊健康度卡片
 *
 * 主组件行数: 452 → 70 (-85%)
 */

export { default } from './risk-dashboard';
export * from './risk-dashboard/types';
export * from './risk-dashboard/useRiskDashboard';
