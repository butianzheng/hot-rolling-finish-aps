/**
 * KPIBand - 重导出分解后的组件
 *
 * 重构后：277 行 → 分解为多个模块化组件
 * - index.tsx (~130 行): 主容器组件
 * - KPICardWrapper.tsx (~50 行): 通用卡片包装组件
 * - types.ts (~70 行): 类型和工具函数
 *
 * 主组件行数: 277 → 130 (-53%)
 */

export { default } from '../kpi-band';
export * from '../kpi-band/types';
