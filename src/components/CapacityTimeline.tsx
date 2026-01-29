/**
 * CapacityTimeline - 重导出分解后的组件
 *
 * 重构后：357 行 → 分解为多个模块化组件
 * - index.tsx (~85 行): 主容器组件
 * - useCapacityTimeline.ts (~60 行): 计算 Hook
 * - StackedBarChart.tsx (~150 行): 堆叠条形图
 * - Legend.tsx (~50 行): 图例组件
 * - types.ts (~45 行): 类型定义和常量
 *
 * 主组件行数: 357 → 85 (-76%)
 */

export { CapacityTimeline, default } from './capacity-timeline';
export * from './capacity-timeline/types';
export * from './capacity-timeline/useCapacityTimeline';
