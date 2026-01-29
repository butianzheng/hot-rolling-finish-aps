/**
 * Dashboard - 重导出分解后的组件
 *
 * 重构后：399 行 → 分解为多个模块化组件
 * - index.tsx (~100 行): 主容器组件
 * - useDashboard.ts (~115 行): 状态管理 Hook
 * - dashboardColumns.tsx (~70 行): 表格列配置
 * - types.ts (~65 行): 类型定义和常量
 * - RefreshControlBar.tsx (~100 行): 刷新控制栏
 * - StatisticsCards.tsx (~95 行): 统计卡片
 *
 * 主组件行数: 399 → 100 (-75%)
 */

export { default } from './dashboard-components';
export * from './dashboard-components/types';
export * from './dashboard-components/useDashboard';
