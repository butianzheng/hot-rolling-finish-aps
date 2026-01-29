/**
 * StrategyProfilesPanel - 重导出分解后的组件
 *
 * 重构后：432 行 → 分解为多个模块化组件
 * - index.tsx (~50 行): 主容器组件
 * - useStrategyProfiles.ts (~180 行): 状态管理 Hook
 * - PresetsCard.tsx (~45 行): 预设策略卡片
 * - CustomProfilesTable.tsx (~115 行): 自定义策略表格
 * - StrategyFormModal.tsx (~100 行): 策略编辑弹窗
 * - types.ts (~50 行): 类型定义和常量
 *
 * 主组件行数: 432 → 50 (-88%)
 */

export { default } from '../strategy-profiles';
export * from '../strategy-profiles/types';
