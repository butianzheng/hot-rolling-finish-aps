/**
 * Workbench 模块统一 queryKeys
 *
 * 用于 React Query 的缓存管理和 invalidateQueries
 * 遵循层级结构，方便批量失效相关查询
 */

export const workbenchQueryKeys = {
  /** 所有 workbench 相关查询 */
  all: ['workbench'] as const,

  /** planItems 相关查询 */
  planItems: {
    all: ['workbench', 'planItems'] as const,
    byVersion: (versionId: string | null) =>
      ['workbench', 'planItems', versionId] as const,
  },

  /** materials 相关查询 */
  materials: {
    all: ['workbench', 'materials'] as const,
    list: (params: { machine_code?: string; limit: number; offset: number }) =>
      ['workbench', 'materials', params] as const,
  },

  /** pathOverride 相关查询 */
  pathOverride: {
    all: ['workbench', 'pathOverride'] as const,
    pending: (versionId: string | null, machineCode: string | null, dateRange: string) =>
      ['workbench', 'pathOverride', 'pending', versionId, machineCode, dateRange] as const,
    summary: (versionId: string | null, dateFrom: string) =>
      ['workbench', 'pathOverride', 'summary', versionId, dateFrom] as const,
  },

  /** rollCycleAnchor 换辊周期锚点 */
  rollCycleAnchor: {
    all: ['workbench', 'rollCycleAnchor'] as const,
    byMachine: (versionId: string | null, machineCode: string | null) =>
      ['workbench', 'rollCycleAnchor', versionId, machineCode] as const,
  },
} as const;
