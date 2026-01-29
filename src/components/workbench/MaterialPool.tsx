/**
 * MaterialPool - 重导出分解后的组件
 *
 * 重构后：484 行 → 分解为多个模块化组件
 * - index.tsx (~120 行): 主容器组件
 * - useMaterialPool.ts (~130 行): 状态管理 Hook
 * - MaterialPoolRow.tsx (~100 行): 行渲染组件
 * - MaterialPoolToolbar.tsx (~75 行): 工具栏组件
 * - utils.ts (~90 行): 辅助函数
 * - types.ts (~55 行): 类型定义
 *
 * 主组件行数: 484 → 120 (-75%)
 */

export { default } from '../material-pool';
export * from '../material-pool/types';
