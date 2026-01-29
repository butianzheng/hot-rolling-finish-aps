/**
 * 风险快照分析 - 重导出分解后的组件
 *
 * 重构后：638 行 → 分解为多个模块化组件
 * - index.tsx (~140 行): 主容器组件
 * - useRiskSnapshotCharts.ts (~170 行): 状态管理 Hook
 * - riskSnapshotColumns.tsx (~90 行): 表格列配置
 * - types.ts (~45 行): 类型定义
 * - FilterBar.tsx (~110 行): 筛选栏
 * - RiskMetricsCards.tsx (~70 行): 风险指标卡片
 * - TrendChart.tsx (~70 行): 风险趋势图
 * - DistributionChart.tsx (~55 行): 风险分布图
 *
 * 主组件行数: 638 → 140 (-78%)
 */

export { default } from './risk-snapshot-charts';
export * from './risk-snapshot-charts/types';
export * from './risk-snapshot-charts/useRiskSnapshotCharts';
