/**
 * PlanManagement - 重导出分解后的组件
 *
 * 重构后：将大体量页面逻辑下沉至 `src/components/plan-management/*`
 * - index.tsx: 主容器
 * - useVersionComparison.ts: 版本对比逻辑与派生数据
 * - columns.tsx / exportHelpers.ts: 已存在的可复用模块
 */

export { default } from './plan-management';
