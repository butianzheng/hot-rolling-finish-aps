/**
 * BottleneckHeatmap - 重导出分解后的组件
 *
 * 重构后：277 行 → 分解为多个模块化组件
 * - index.tsx (~50 行): 主容器组件
 * - useHeatmapData.ts (~45 行): 数据处理 Hook
 * - chartConfig.ts (~120 行): ECharts 配置生成
 * - types.ts (~45 行): 类型和常量定义
 *
 * 主组件行数: 277 → 50 (-82%)
 */

export { default, BottleneckHeatmap } from '../bottleneck-heatmap';
export * from '../bottleneck-heatmap/types';
