/**
 * 导出工具函数
 * 支持 CSV、JSON、TSV 等格式的数据导出
 */

export type ExportFormat = 'csv' | 'json' | 'tsv';

interface ExportOptions {
  filename: string;
  format?: ExportFormat;
  sheetName?: string; // 仅用于 Excel（未来扩展）
}

/**
 * 将对象数组导出为 CSV 格式
 */
function convertToCSV(data: Record<string, unknown>[]): string {
  if (data.length === 0) {
    return '';
  }

  // 获取所有字段名
  const headers = Object.keys(data[0]);

  // 生成 CSV 头
  const csvHeaders = headers
    .map((header) => {
      // 对包含逗号或引号的字段进行转义
      const value = String(header);
      if (value.includes(',') || value.includes('"') || value.includes('\n')) {
        return `"${value.replace(/"/g, '""')}"`;
      }
      return value;
    })
    .join(',');

  // 生成 CSV 行
  const csvRows = data.map((row) => {
    return headers
      .map((header) => {
        let value = row[header];

        // 处理不同的数据类型
        if (value === null || value === undefined) {
          return '';
        }

        if (typeof value === 'object') {
          value = JSON.stringify(value);
        } else {
          value = String(value);
        }

        // 确保 value 是字符串类型后再调用字符串方法
        const strValue = String(value);
        // 对包含特殊字符的值进行转义
        if (strValue.includes(',') || strValue.includes('"') || strValue.includes('\n')) {
          return `"${strValue.replace(/"/g, '""')}"`;
        }

        return strValue;
      })
      .join(',');
  });

  return [csvHeaders, ...csvRows].join('\n');
}

/**
 * 将对象数组导出为 TSV 格式
 */
function convertToTSV(data: Record<string, unknown>[]): string {
  if (data.length === 0) {
    return '';
  }

  const headers = Object.keys(data[0]);

  // 生成 TSV 头
  const tsvHeaders = headers.join('\t');

  // 生成 TSV 行
  const tsvRows = data.map((row) => {
    return headers
      .map((header) => {
        let value = row[header];

        if (value === null || value === undefined) {
          return '';
        }

        if (typeof value === 'object') {
          value = JSON.stringify(value);
        } else {
          value = String(value);
        }

        // 确保 value 是字符串类型后再调用字符串方法
        const strValue = String(value);
        // 处理制表符和换行符
        return strValue.replace(/\t/g, ' ').replace(/\n/g, ' ');
      })
      .join('\t');
  });

  return [tsvHeaders, ...tsvRows].join('\n');
}

/**
 * 触发浏览器下载
 */
function downloadFile(content: string, filename: string, mimeType: string): void {
  const blob = new Blob([content], { type: `${mimeType};charset=utf-8;` });
  const link = document.createElement('a');
  const url = URL.createObjectURL(blob);

  link.href = url;
  link.download = filename;
  link.style.display = 'none';

  document.body.appendChild(link);
  link.click();
  document.body.removeChild(link);

  // 清理
  URL.revokeObjectURL(url);
}

function buildTimestampedFilename(filename: string, ext: string): string {
  const timestamp = new Date().toISOString().replace(/[:.]/g, '-').slice(0, -5);
  const filenameParts = filename.split('.');
  const baseName = filenameParts.length > 1 ? filenameParts.slice(0, -1).join('.') : filename;
  return `${baseName}_${timestamp}.${ext}`;
}

/**
 * 导出数据
 */
export function exportData(
  data: Record<string, unknown>[],
  options: ExportOptions
): void {
  if (!data || data.length === 0) {
    throw new Error('没有数据可导出');
  }

  const format = options.format || 'csv';
  const fullFilename = buildTimestampedFilename(options.filename, format);

  let content: string;
  let mimeType: string;

  switch (format) {
    case 'csv':
      content = convertToCSV(data);
      mimeType = 'text/csv';
      break;

    case 'tsv':
      content = convertToTSV(data);
      mimeType = 'text/tab-separated-values';
      break;

    case 'json':
      content = JSON.stringify(data, null, 2);
      mimeType = 'application/json';
      break;

    default:
      throw new Error(`不支持的导出格式: ${format}`);
  }

  downloadFile(content, fullFilename, mimeType);
}

/**
 * 导出 CSV
 */
export function exportCSV(data: Record<string, unknown>[], filename: string): void {
  exportData(data, { filename, format: 'csv' });
}

/**
 * 导出 JSON
 */
export function exportJSON(data: Record<string, unknown>[], filename: string): void {
  exportData(data, { filename, format: 'json' });
}

/**
 * 导出 TSV
 */
export function exportTSV(data: Record<string, unknown>[], filename: string): void {
  exportData(data, { filename, format: 'tsv' });
}

/**
 * 导出 Markdown 文本（用于报告导出）
 */
export function exportMarkdown(markdown: string, filename: string): void {
  const fullFilename = buildTimestampedFilename(filename, 'md');
  downloadFile(markdown, fullFilename, 'text/markdown');
}

/**
 * 导出 HTML 文本（用于报告导出，可在浏览器/打印为 PDF）
 */
export function exportHTML(html: string, filename: string): void {
  const fullFilename = buildTimestampedFilename(filename, 'html');
  downloadFile(html, fullFilename, 'text/html');
}

/**
 * 获取导出菜单项
 */
export function getExportMenuItems(
  data: Record<string, unknown>[],
  filename: string
): Array<{ label: string; key: string; onClick: () => void }> {
  return [
    {
      label: '导出为 CSV',
      key: 'csv',
      onClick: () => {
        try {
          exportCSV(data, filename);
        } catch (error: unknown) {
          console.error('导出 CSV 失败:', error instanceof Error ? error.message : String(error));
        }
      },
    },
    {
      label: '导出为 JSON',
      key: 'json',
      onClick: () => {
        try {
          exportJSON(data, filename);
        } catch (error: unknown) {
          console.error('导出 JSON 失败:', error instanceof Error ? error.message : String(error));
        }
      },
    },
    {
      label: '导出为 TSV',
      key: 'tsv',
      onClick: () => {
        try {
          exportTSV(data, filename);
        } catch (error: unknown) {
          console.error('导出 TSV 失败:', error instanceof Error ? error.message : String(error));
        }
      },
    },
  ];
}
