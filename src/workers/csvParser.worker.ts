/**
 * CSV 解析 Web Worker
 * 在后台线程处理大型 CSV 文件，避免阻塞主线程
 */

export interface CsvWorkerMessage {
  type: 'PARSE_PREVIEW';
  payload: {
    content: string;
    maxRows: number;
  };
}

export interface CsvWorkerResult {
  type: 'PARSE_RESULT';
  payload: {
    headers: string[];
    rows: Record<string, string>[];
    totalRows: number;
  };
}

export interface CsvWorkerError {
  type: 'PARSE_ERROR';
  payload: {
    message: string;
  };
}

/**
 * 分割 CSV 行（支持引号处理）
 */
function splitCsvLine(line: string): string[] {
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
 */
function parseCsvPreview(
  content: string,
  maxRows: number
): { headers: string[]; rows: Record<string, string>[]; totalRows: number } {
  const normalized = content.replace(/^\uFEFF/, '').replace(/\r\n/g, '\n').replace(/\r/g, '\n');
  const allLines = normalized.split('\n').filter((l) => l.trim().length > 0);
  if (allLines.length === 0) return { headers: [], rows: [], totalRows: 0 };

  const headers = splitCsvLine(allLines[0]);
  const dataLines = allLines.slice(1);
  const rows: Record<string, string>[] = dataLines.slice(0, maxRows).map((line) => {
    const values = splitCsvLine(line);
    const row: Record<string, string> = {};
    headers.forEach((h, idx) => {
      row[h] = values[idx] ?? '';
    });
    return row;
  });

  return { headers, rows, totalRows: dataLines.length };
}

// Worker 消息处理
self.onmessage = (event: MessageEvent<CsvWorkerMessage>) => {
  const { type, payload } = event.data;

  if (type === 'PARSE_PREVIEW') {
    try {
      const result = parseCsvPreview(payload.content, payload.maxRows);
      const response: CsvWorkerResult = {
        type: 'PARSE_RESULT',
        payload: result,
      };
      self.postMessage(response);
    } catch (error: any) {
      const errorResponse: CsvWorkerError = {
        type: 'PARSE_ERROR',
        payload: { message: error?.message || '解析失败' },
      };
      self.postMessage(errorResponse);
    }
  }
};

export {};
