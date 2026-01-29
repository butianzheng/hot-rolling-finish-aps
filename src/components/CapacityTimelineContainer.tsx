/**
 * CapacityTimelineContainer - 重导出分解后的组件
 *
 * 重构后：253 行 → 分解为多个模块化组件
 * - index.tsx (~55 行): 主容器组件
 * - useCapacityTimelineContainer.ts (~190 行): 状态管理 Hook
 * - ToolBar.tsx (~55 行): 工具栏组件
 * - types.ts (~25 行): 类型定义
 *
 * 主组件行数: 253 → 55 (-78%)
 */

export { CapacityTimelineContainer, default } from './capacity-timeline-container';
export * from './capacity-timeline-container/types';
export * from './capacity-timeline-container/useCapacityTimelineContainer';
