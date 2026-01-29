/**
 * CSV 文件解析工具
 * 用于材料导入功能的 CSV 文件读取和预览
 */

import type { PreviewRow } from '../types/import';

/**
 * 分割 CSV 行（支持引号处理）
 * @param line CSV 行文本
 * @returns 分割后的字段数组
 */
export function splitCsvLine(line: string): string[] {
  // Minimal CSV splitter with basic quote handling; good enough for our fixture data.
  const out: string[] = [];
  let cur = '';
  let inQuotes = false;

  for (let i = 0; i < line.length; i += 1) {
    const ch = line[i];
    if (ch === '"') {
      const next = line[i + 1];
      if (inQuotes && next === '"') {
        cur += '"';
        i += 1;
        continue;
      }
      inQuotes = !inQuotes;
      continue;
    }

    if (ch === ',' && !inQuotes) {
      out.push(cur.trim());
      cur = '';
      continue;
    }

    cur += ch;
  }

  out.push(cur.trim());
  return out;
}

/**
 * 解析 CSV 内容并生成预览数据
 * @param content CSV 文件内容
 * @param maxRows 最大预览行数
 * @returns { headers, rows, totalRows } 表头、预览行、总行数
 */
export function parseCsvPreview(
  content: string,
  maxRows: number
): { headers: string[]; rows: PreviewRow[]; totalRows: number } {
  const normalized = content.replace(/^\uFEFF/, '').replace(/\r\n/g, '\n').replace(/\r/g, '\n');
  const allLines = normalized.split('\n').filter((l) => l.trim().length > 0);
  if (allLines.length === 0) return { headers: [], rows: [], totalRows: 0 };

  const headers = splitCsvLine(allLines[0]);
  const dataLines = allLines.slice(1);
  const rows: PreviewRow[] = dataLines.slice(0, maxRows).map((line) => {
    const values = splitCsvLine(line);
    const row: PreviewRow = {};
    headers.forEach((h, idx) => {
      row[h] = values[idx] ?? '';
    });
    return row;
  });

  return { headers, rows, totalRows: dataLines.length };
}
