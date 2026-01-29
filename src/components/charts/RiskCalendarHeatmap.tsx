/**
 * RiskCalendarHeatmap - 重导出分解后的组件
 *
 * 重构后：245 行 → 分解为多个模块化组件
 * - index.tsx (~50 行): 主容器组件
 * - useRiskHeatmapData.ts (~35 行): 数据处理 Hook
 * - chartConfig.ts (~115 行): ECharts 配置生成
 * - types.ts (~40 行): 类型和工具函数
 *
 * 主组件行数: 245 → 50 (-80%)
 */

export { default, RiskCalendarHeatmap } from '../risk-calendar-heatmap';
export * from '../risk-calendar-heatmap/types';
