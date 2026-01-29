/**
 * 产能池管理 - 重导出分解后的组件
 *
 * 重构后：594 行 → 分解为多个模块化组件
 * - index.tsx (~130 行): 主容器组件
 * - useCapacityPoolManagement.ts (~300 行): 状态管理 Hook
 * - capacityPoolColumns.tsx (~90 行): 表格列配置
 * - types.ts (~25 行): 类型定义
 * - StatisticsCards.tsx (~50 行): 统计卡片
 * - FilterBar.tsx (~55 行): 筛选栏
 * - EditCapacityModal.tsx (~85 行): 编辑模态框
 * - BatchEditModal.tsx (~80 行): 批量调整模态框
 *
 * 主组件行数: 594 → 130 (-78%)
 */

export { default } from './capacity-pool-management';
export * from './capacity-pool-management/types';
export * from './capacity-pool-management/useCapacityPoolManagement';
