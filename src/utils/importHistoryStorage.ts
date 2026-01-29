/**
 * 导入历史的 LocalStorage 管理工具
 * 用于持久化材料导入历史记录
 */

import type { ImportHistoryItem } from '../types/import';
import { IMPORT_HISTORY_KEY, IMPORT_HISTORY_MAX } from '../types/import';

/**
 * 从 localStorage 读取导入历史
 * @returns 导入历史数组（如果解析失败则返回空数组）
 */
export function safeReadImportHistory(): ImportHistoryItem[] {
  if (typeof window === 'undefined') return [];
  try {
    const raw = window.localStorage.getItem(IMPORT_HISTORY_KEY);
    if (!raw) return [];
    const parsed = JSON.parse(raw);
    if (!Array.isArray(parsed)) return [];
    return parsed
      .map((it: any) => ({
        id: String(it?.id ?? ''),
        created_at: String(it?.created_at ?? ''),
        operator: String(it?.operator ?? ''),
        file_path: String(it?.file_path ?? ''),
        source_batch_id: String(it?.source_batch_id ?? ''),
        import_batch_id: it?.import_batch_id == null ? null : String(it.import_batch_id),
        imported: Number(it?.imported ?? 0),
        updated: Number(it?.updated ?? 0),
        conflicts: Number(it?.conflicts ?? 0),
        elapsed_ms: it?.elapsed_ms == null ? null : Number(it.elapsed_ms),
      }))
      .filter((it) => it.id);
  } catch {
    return [];
  }
}

/**
 * 将导入历史写入 localStorage
 * @param items 导入历史数组
 */
export function safeWriteImportHistory(items: ImportHistoryItem[]) {
  if (typeof window === 'undefined') return;
  try {
    window.localStorage.setItem(IMPORT_HISTORY_KEY, JSON.stringify(items));
  } catch {
    // ignore
  }
}

/**
 * 追加导入历史记录（自动限制最大数量）
 * @param item 新的历史记录项
 * @param currentHistory 当前历史记录
 * @returns 更新后的历史记录数组
 */
export function appendImportHistory(
  item: ImportHistoryItem,
  currentHistory: ImportHistoryItem[]
): ImportHistoryItem[] {
  const next = [item, ...currentHistory.filter((x) => x.id !== item.id)].slice(0, IMPORT_HISTORY_MAX);
  safeWriteImportHistory(next);
  return next;
}

/**
 * 清空导入历史
 */
export function clearImportHistory() {
  safeWriteImportHistory([]);
}
