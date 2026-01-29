/**
 * ColdStockChart - 重导出分解后的组件
 *
 * 重构后：239 行 → 分解为多个模块化组件
 * - index.tsx (~55 行): 主容器组件
 * - useColdStockData.ts (~55 行): 数据处理 Hook
 * - chartConfig.ts (~95 行): ECharts 配置生成
 * - types.ts (~45 行): 类型和常量
 *
 * 主组件行数: 239 → 55 (-77%)
 */

export { default, ColdStockChart } from '../cold-stock-chart';
export * from '../cold-stock-chart/types';
