/**
 * MaterialInspector - 重导出分解后的组件
 *
 * 重构后：335 行 → 分解为多个模块化组件
 * - index.tsx (~90 行): 主容器组件
 * - useMaterialInspector.ts (~55 行): 状态管理 Hook
 * - BasicInfoSection.tsx (~45 行): 基本信息区
 * - StatusInfoSection.tsx (~55 行): 状态信息区
 * - EngineReasonSection.tsx (~70 行): 引擎推理原因区
 * - ActionHistorySection.tsx (~60 行): 操作历史区
 * - types.ts (~45 行): 类型定义
 *
 * 主组件行数: 335 → 90 (-73%)
 */

export { MaterialInspector, default } from './material-inspector';
export * from './material-inspector/types';
export * from './material-inspector/useMaterialInspector';
