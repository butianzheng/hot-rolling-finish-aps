/**
 * VersionComparisonModal - 重导出分解后的组件
 *
 * 重构后：666 行 → 分解为多个模块化组件
 * - index.tsx (~200 行): 主容器组件
 * - KpiCompareCard.tsx (~45 行): KPI 对比卡片
 * - MaterialDiffCard.tsx (~170 行): 物料变更明细卡片
 * - CapacityDeltaCard.tsx (~150 行): 产能变化卡片
 * - RetrospectiveCard.tsx (~55 行): 复盘总结卡片
 * - Chart.tsx (~40 行): 懒加载图表组件
 * - types.ts (~70 行): 类型定义
 *
 * 主组件行数: 666 → 200 (-70%)
 */

export { default, VersionComparisonModal } from '../version-comparison-modal';
export * from '../version-comparison-modal/types';
