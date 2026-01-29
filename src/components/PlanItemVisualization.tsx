/**
 * 排产明细可视化 - 重导出分解后的组件
 *
 * 重构后：922 行 → 分解为多个模块化组件
 * - index.tsx (~180 行): 主容器组件
 * - usePlanItemVisualization.tsx (~320 行): 状态管理 Hook
 * - planItemColumns.tsx (~130 行): 表格列配置
 * - types.ts (~55 行): 类型定义和常量
 * - DraggableRow.tsx (~30 行): 可拖拽行组件
 * - StatisticsCards.tsx (~55 行): 统计卡片
 * - FilterBar.tsx (~100 行): 筛选栏
 * - BatchOperationBar.tsx (~30 行): 批量操作栏
 * - PlanItemDetailModal.tsx (~75 行): 详情模态框
 * - ForceReleaseModal.tsx (~65 行): 强制放行模态框
 *
 * 主组件行数: 922 → 180 (-80%)
 */

export { default } from './plan-item-visualization';
export * from './plan-item-visualization/types';
export * from './plan-item-visualization/usePlanItemVisualization';
