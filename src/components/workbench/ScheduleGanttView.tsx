/**
 * ScheduleGanttView - 重导出分解后的组件
 *
 * 重构后：935 行 → 分解为多个模块化组件
 * - index.tsx (~180 行): 主容器组件
 * - useGanttData.ts (~190 行): 数据处理 Hook
 * - GanttRow.tsx (~220 行): 行渲染组件
 * - GanttToolbar.tsx (~70 行): 工具栏和图例
 * - CellDetailModal.tsx (~150 行): 单元格详情弹窗
 * - types.ts (~55 行): 类型和常量
 * - utils.ts (~75 行): 工具函数
 *
 * 主组件行数: 935 → 180 (-81%)
 */

export { default } from '../schedule-gantt-view';
export * from '../schedule-gantt-view/types';
