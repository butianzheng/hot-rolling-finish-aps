/**
 * RedLineGuard - 重导出分解后的组件
 *
 * 重构后：354 行 → 分解为多个模块化组件
 * - index.tsx (~65 行): 主容器组件
 * - types.ts (~95 行): 类型定义和配置
 * - utils.ts (~60 行): 工具函数
 * - CompactMode.tsx (~45 行): 紧凑模式组件
 * - DetailedMode.tsx (~95 行): 详细模式组件
 *
 * 主组件行数: 354 → 65 (-82%)
 */

export { default, RedLineGuard } from '../red-line-guard';
export * from '../red-line-guard/types';
export * from '../red-line-guard/utils';
