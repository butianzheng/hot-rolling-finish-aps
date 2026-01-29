/**
 * 材料管理 - 重导出分解后的组件
 *
 * 重构后：1000 行 → 分解为多个模块化组件
 * - index.tsx (320 行): 主容器组件
 * - useMaterialTimeline.ts (184 行): 产能时间线 Hook
 * - materialTableColumns.tsx (267 行): 表格列配置
 * - materialTypes.ts (99 行): 类型定义和工具函数
 * - CapacityTimelineSection.tsx (98 行): 产能时间线区块
 * - MaterialOperationModal.tsx (94 行): 操作确认模态框
 *
 * 主组件行数: 1000 → 320 (-68%)
 */

export { default } from './material-management';
export * from './material-management/materialTypes';
export * from './material-management/useMaterialTimeline';
