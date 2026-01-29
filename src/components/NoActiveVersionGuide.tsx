/**
 * NoActiveVersionGuide - 重导出分解后的组件
 *
 * 重构后：204 行 → 分解为多个模块化组件
 * - index.tsx (~40 行): 主容器组件
 * - MainHintCard.tsx (~70 行): 主提示卡片
 * - QuickStartGuide.tsx (~90 行): 快速开始指南
 * - FAQCard.tsx (~35 行): 常见问题卡片
 * - types.ts (~50 行): 类型定义和常量
 *
 * 主组件行数: 204 → 40 (-80%)
 */

export { default } from './no-active-version-guide';
export * from './no-active-version-guide/types';
