/**
 * OneClickOptimizeMenu - 重导出分解后的组件
 *
 * 重构后：301 行 → 分解为多个模块化组件
 * - index.tsx (~85 行): 主容器组件
 * - useOneClickOptimize.ts (~160 行): 状态管理 Hook
 * - PreviewModal.tsx (~100 行): 预览/试算弹窗
 * - PostCreateModal.tsx (~65 行): 创建后操作弹窗
 * - types.ts (~40 行): 类型定义
 *
 * 主组件行数: 301 → 85 (-72%)
 */

export { default } from '../one-click-optimize';
export * from '../one-click-optimize/types';
