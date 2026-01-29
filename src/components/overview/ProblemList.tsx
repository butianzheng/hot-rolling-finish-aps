/**
 * ProblemList - 重导出分解后的组件
 *
 * 重构后：421 行 → 分解为多个模块化组件
 * - index.tsx (~85 行): 主容器组件
 * - ProblemCard.tsx (~240 行): 单个问题卡片组件
 * - types.ts (~90 行): 类型定义和配置函数
 *
 * 主组件行数: 421 → 85 (-80%)
 */

export { default } from '../problem-list';
export * from '../problem-list/types';
