import { z } from 'zod';

export { z };

// ==========================================================
// IPC response schemas (Zod) - 入口
//
// Goal:
// - Catch backend/frontend contract drift early (before UI logic runs).
// - Be strict on required fields, permissive on unknown extra fields.
//
// Notes:
// - 各业务域 schema 已拆分到 `src/api/ipcSchemas/*`，此文件仅做统一导出。
// - 决策看板（D1-D6）严格 schema 位于 `src/api/ipcSchemas/decision.ts`；
//   此处通过 `dashboardSchemas.ts` 做 re-export 并保持 .passthrough() 向后兼容。
// ==========================================================

export function zodValidator<T extends z.ZodTypeAny>(schema: T, name: string) {
  return (value: unknown): z.infer<T> => {
    const parsed = schema.safeParse(value);
    if (parsed.success) return parsed.data;

    throw {
      code: 'IPC_SCHEMA_MISMATCH',
      message: `IPC 响应契约校验失败: ${name}`,
      details: {
        issues: parsed.error.issues,
      },
    };
  };
}

// Common success response for commands that return `{}`.
export const EmptyOkResponseSchema = z.object({}).passthrough();

export * from './ipcSchemas/importSchemas';
export * from './ipcSchemas/strategyDraftSchemas';
export * from './ipcSchemas/versionComparisonSchemas';
export * from './ipcSchemas/decisionRefreshSchemas';
export * from './ipcSchemas/dashboardSchemas';
export * from './ipcSchemas/materialSchemas';
export * from './ipcSchemas/planSchemas';
export * from './ipcSchemas/configSchemas';
export * from './ipcSchemas/capacitySchemas';
export * from './ipcSchemas/pathRuleSchemas';
export * from './ipcSchemas/rollSchemas';
export * from './ipcSchemas/actionLogSchemas';
export * from './ipcSchemas/rhythmSchemas';
export * from './ipcSchemas/machineConfigSchemas';

