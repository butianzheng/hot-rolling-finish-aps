/**
 * 材料导入相关的共享类型定义
 * 用于 MaterialImport 组件和相关工具函数
 */

export type DqViolation = {
  row_number: number;
  material_id: string | null;
  level: 'Error' | 'Warning' | 'Info' | 'Conflict' | string;
  field: string;
  message: string;
};

export type DqSummary = {
  total_rows: number;
  success: number;
  blocked: number;
  warning: number;
  conflict: number;
};

export type ImportMaterialsResponse = {
  imported: number;
  updated: number;
  conflicts: number;
  batch_id: string;
  import_batch_id?: string;
  dq_summary?: DqSummary;
  dq_violations?: DqViolation[];
  elapsed_ms?: number;
};

export type ImportHistoryItem = {
  id: string;
  created_at: string;
  operator: string;
  file_path: string;
  source_batch_id: string;
  import_batch_id: string | null;
  imported: number;
  updated: number;
  conflicts: number;
  elapsed_ms: number | null;
};

export type ImportConflict = {
  conflict_id: string;
  batch_id: string;
  row_number: number;
  material_id: string | null;
  conflict_type: string;
  raw_data: string;
  reason: string;
  resolved: boolean;
  created_at: string;
};

export type ImportConflictListResponse = {
  conflicts: ImportConflict[];
  total: number;
  limit: number;
  offset: number;
};

export type PreviewRow = Record<string, string>;

// 常量定义
export const REQUIRED_HEADERS = ['材料号', '材料实际重量', '下道机组代码'];
export const IMPORT_HISTORY_KEY = 'aps_import_history';
export const IMPORT_HISTORY_MAX = 30;
