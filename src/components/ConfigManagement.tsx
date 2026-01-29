/**
 * 配置管理 - 重导出分解后的组件
 *
 * 重构后：452 行 → 分解为多个模块化组件
 * - index.tsx (~90 行): 主容器组件
 * - useConfigManagement.ts (~230 行): 状态管理 Hook
 * - configColumns.tsx (~75 行): 表格列配置
 * - types.ts (~35 行): 类型定义和常量
 * - StatisticsCards.tsx (~45 行): 统计卡片
 * - FilterBar.tsx (~55 行): 筛选栏
 * - EditConfigModal.tsx (~85 行): 编辑模态框
 *
 * 主组件行数: 452 → 90 (-80%)
 */

export { default } from './config-management';
export * from './config-management/types';
export * from './config-management/useConfigManagement';
