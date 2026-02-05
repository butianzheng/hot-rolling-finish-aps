// ==========================================
// Tauri API 调用封装层（入口）
// ==========================================
// 职责: 统一对外导出各域 API（已按模块拆分）
// 重要: Rust 后端配置了 #[tauri::command(rename_all = "snake_case")]
//       前端必须使用 snake_case 参数名与 Rust 后端对齐
// ==========================================

export type { ErrorResponse } from './tauri/common';

export { importApi } from './tauri/importApi';
export { materialApi } from './tauri/materialApi';
export { planApi } from './tauri/planApi';
export { capacityApi } from './tauri/capacityApi';
export { machineConfigApi } from './tauri/machineConfigApi';
export { dashboardApi } from './tauri/dashboardApi';
export { configApi } from './tauri/configApi';
export { pathRuleApi } from './tauri/pathRuleApi';
export { rollApi } from './tauri/rollApi';
export { rhythmApi } from './tauri/rhythmApi';

// ==========================================
// Decision Service (D1-D6)
// ==========================================
// 统一对外出口：推荐在业务/Hook 中通过 `api/tauri.ts` 引用，避免绕过统一封装
export * from './tauri/decisionService';
