/**
 * CSV 文件解析工具
 * 用于材料导入功能的 CSV 文件读取和预览
 *
 * 支持两种模式：
 * 1. 同步解析 - 适合小文件 (<1MB)
 * 2. Web Worker 异步解析 - 适合大文件，避免阻塞主线程
 */

import type { PreviewRow } from '../types/import';
import type { CsvWorkerMessage, CsvWorkerResult, CsvWorkerError } from '../workers/csvParser.worker';

/** 大文件阈值（字节），超过此值使用 Worker */
export const LARGE_FILE_THRESHOLD = 1 * 1024 * 1024; // 1MB

/** Worker 单例，延迟初始化 */
let csvWorker: Worker | null = null;

/**
 * 获取或创建 CSV 解析 Worker
 * 使用 Vite 的 worker 导入语法
 */
function getCsvWorker(): Worker {
  if (!csvWorker) {
    csvWorker = new Worker(new URL('../workers/csvParser.worker.ts', import.meta.url), {
      type: 'module',
    });
  }
  return csvWorker;
}

/**
 * 终止 Worker（可用于清理资源）
 */
export function terminateCsvWorker(): void {
  if (csvWorker) {
    csvWorker.terminate();
    csvWorker = null;
  }
}

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
 * 解析 CSV 内容并生成预览数据（同步版本）
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

/**
 * 使用 Web Worker 异步解析 CSV（适合大文件）
 * @param content CSV 文件内容
 * @param maxRows 最大预览行数
 * @returns Promise 返回解析结果
 */
export function parseCsvPreviewAsync(
  content: string,
  maxRows: number
): Promise<{ headers: string[]; rows: PreviewRow[]; totalRows: number }> {
  return new Promise((resolve, reject) => {
    const worker = getCsvWorker();

    const handleMessage = (event: MessageEvent<CsvWorkerResult | CsvWorkerError>) => {
      worker.removeEventListener('message', handleMessage);
      worker.removeEventListener('error', handleError);

      if (event.data.type === 'PARSE_RESULT') {
        resolve(event.data.payload as { headers: string[]; rows: PreviewRow[]; totalRows: number });
      } else if (event.data.type === 'PARSE_ERROR') {
        reject(new Error(event.data.payload.message));
      }
    };

    const handleError = (error: ErrorEvent) => {
      worker.removeEventListener('message', handleMessage);
      worker.removeEventListener('error', handleError);
      reject(new Error(error.message || 'Worker 执行失败'));
    };

    worker.addEventListener('message', handleMessage);
    worker.addEventListener('error', handleError);

    const message: CsvWorkerMessage = {
      type: 'PARSE_PREVIEW',
      payload: { content, maxRows },
    };
    worker.postMessage(message);
  });
}

/**
 * 智能解析 CSV：根据文件大小自动选择同步或异步模式
 * @param content CSV 文件内容
 * @param maxRows 最大预览行数
 * @param forceAsync 强制使用异步模式
 * @returns Promise 返回解析结果
 */
export async function parseCsvPreviewSmart(
  content: string,
  maxRows: number,
  forceAsync = false
): Promise<{ headers: string[]; rows: PreviewRow[]; totalRows: number }> {
  const contentSize = new Blob([content]).size;

  // 大文件或强制异步：使用 Worker
  if (forceAsync || contentSize >= LARGE_FILE_THRESHOLD) {
    console.log(`[CSV Parser] 使用 Web Worker 解析大文件 (${(contentSize / 1024 / 1024).toFixed(2)} MB)`);
    return parseCsvPreviewAsync(content, maxRows);
  }

  // 小文件：同步解析
  return parseCsvPreview(content, maxRows);
}
