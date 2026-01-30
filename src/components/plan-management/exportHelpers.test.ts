/**
 * plan-management/exportHelpers.ts 单元测试
 */

import { describe, it, expect, vi, beforeEach } from 'vitest';
import type { ExportContext } from '../plan-management/types';
import type { BackendVersionComparisonResult } from '../../types/comparison';
import {
  exportCapacityDelta,
  exportDiffs,
  exportRetrospectiveReport,
  exportReportMarkdown,
  exportReportHTML,
} from '../plan-management/exportHelpers';

// Mock 导出工具
vi.mock('../../utils/exportUtils', () => ({
  exportCSV: vi.fn(),
  exportJSON: vi.fn(),
  exportMarkdown: vi.fn(),
  exportHTML: vi.fn(),
}));

// Mock Ant Design message
vi.mock('antd', () => ({
  message: {
    success: vi.fn(),
    error: vi.fn(),
  },
}));

import { exportCSV, exportJSON, exportMarkdown, exportHTML } from '../../utils/exportUtils';
import { message } from 'antd';

describe('exportHelpers', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  const mockCompareResult: BackendVersionComparisonResult = {
    version_id_a: 'v1',
    version_id_b: 'v2',
    moved_count: 5,
    added_count: 3,
    removed_count: 2,
    squeezed_out_count: 1,
    risk_delta: null,
    capacity_delta: null,
    config_changes: null,
    message: 'success',
  };

  describe('exportCapacityDelta', () => {
    it('CSV 格式导出应该正确映射字段', async () => {
      const context: ExportContext = {
        compareResult: mockCompareResult,
        currentUser: 'test_user',
        localDiffResult: null,
        localCapacityRows: {
          rows: [
            {
              machine_code: 'M1',
              date: '2026-01-30',
              used_a: 100,
              used_b: 120,
              delta: 20,
              target_a: 150,
              limit_a: 200,
              target_b: 160,
              limit_b: 210,
            },
          ],
          totalA: 100,
          totalB: 120,
          dateFrom: '2026-01-30',
          dateTo: '2026-01-30',
          machines: ['M1'],
          overflowRows: [],
        },
        retrospectiveNote: '',
      };

      await exportCapacityDelta('csv', context);
      expect(exportCSV).toHaveBeenCalled();
      const call = (exportCSV as any).mock.calls[0];
      expect(call[0]).toHaveLength(1);
      expect(call[0][0].machine_code).toBe('M1');
      expect(call[0][0].delta).toBe(20);
      expect(call[0][0].date).toBe('2026-01-30');
    });

    it('JSON 格式导出应该工作', async () => {
      const context: ExportContext = {
        compareResult: mockCompareResult,
        currentUser: 'test_user',
        localDiffResult: null,
        localCapacityRows: {
          rows: [{ machine_code: 'M1', date: '2026-01-30', used_a: 100, used_b: 120, delta: 20, target_a: null, limit_a: null, target_b: null, limit_b: null }],
          totalA: 100,
          totalB: 120,
          dateFrom: '2026-01-30',
          dateTo: '2026-01-30',
          machines: ['M1'],
          overflowRows: [],
        },
        retrospectiveNote: '',
      };

      await exportCapacityDelta('json', context);
      expect(exportJSON).toHaveBeenCalled();
    });

    it('数据为 null 时应该提前返回', async () => {
      const context: ExportContext = {
        compareResult: mockCompareResult,
        currentUser: 'test_user',
        localDiffResult: null,
        localCapacityRows: null,
        retrospectiveNote: '',
      };
      await exportCapacityDelta('csv', context);
      expect(exportCSV).not.toHaveBeenCalled();
    });
  });

  describe('exportDiffs', () => {
    it('应该正确映射版本差异数据', async () => {
      const context: ExportContext = {
        compareResult: mockCompareResult,
        currentUser: 'test_user',
        localDiffResult: {
          diffs: [
            {
              materialId: 'M001',
              changeType: 'MOVED',
              previousState: { material_id: 'M001', machine_code: 'MA', plan_date: '2026-01-30', seq_no: 1 },
              currentState: { material_id: 'M001', machine_code: 'MB', plan_date: '2026-01-31', seq_no: 2 },
            },
          ],
          summary: { totalChanges: 1, movedCount: 1, modifiedCount: 0, addedCount: 0, removedCount: 0 },
        },
        localCapacityRows: null,
        retrospectiveNote: '',
      };

      await exportDiffs('json', context);
      expect(exportJSON).toHaveBeenCalled();
      const call = (exportJSON as any).mock.calls[0];
      expect(call[0]).toHaveLength(1);
      expect(call[0][0].change_type).toBe('MOVED');
      expect(call[0][0].material_id).toBe('M001');
    });

    it('差异数据为 null 时应该提前返回', async () => {
      const context: ExportContext = {
        compareResult: mockCompareResult,
        currentUser: 'test_user',
        localDiffResult: null,
        localCapacityRows: null,
        retrospectiveNote: '',
      };
      await exportDiffs('csv', context);
      expect(exportCSV).not.toHaveBeenCalled();
    });
  });

  describe('exportRetrospectiveReport', () => {
    it('应该导出复盘总结 JSON', async () => {
      const context: ExportContext = {
        compareResult: mockCompareResult,
        currentUser: 'operator_001',
        localDiffResult: null,
        localCapacityRows: null,
        retrospectiveNote: '测试复盘笔记',
      };

      await exportRetrospectiveReport(context);
      expect(exportJSON).toHaveBeenCalled();
      expect(message.success).toHaveBeenCalledWith('已导出复盘总结（JSON）');
    });

    it('compareResult 缺失时应该提前返回', async () => {
      const context: any = {
        compareResult: null,
        currentUser: 'test',
        localDiffResult: null,
        localCapacityRows: null,
        retrospectiveNote: '',
      };
      await exportRetrospectiveReport(context);
      expect(exportJSON).not.toHaveBeenCalled();
    });

    it('错误时应该显示错误信息', async () => {
      vi.mocked(exportJSON).mockImplementationOnce(() => {
        throw new Error('Export failed');
      });

      const context: ExportContext = {
        compareResult: mockCompareResult,
        currentUser: 'test',
        localDiffResult: null,
        localCapacityRows: null,
        retrospectiveNote: '',
      };

      await exportRetrospectiveReport(context);
      expect(message.error).toHaveBeenCalledWith('Export failed');
    });
  });

  describe('exportReportMarkdown', () => {
    it('应该导出 Markdown 格式报告', async () => {
      const context: ExportContext = {
        compareResult: mockCompareResult,
        currentUser: 'test',
        localDiffResult: {
          diffs: [
            {
              materialId: 'M001',
              changeType: 'ADDED',
              previousState: null,
              currentState: { material_id: 'M001', machine_code: 'MA', plan_date: '2026-01-30', seq_no: 1 },
            },
          ],
          summary: { totalChanges: 1, movedCount: 0, modifiedCount: 0, addedCount: 1, removedCount: 0 },
        },
        localCapacityRows: {
          rows: [],
          totalA: 100,
          totalB: 120,
          dateFrom: '2026-01-30',
          dateTo: '2026-01-30',
          machines: ['M1'],
          overflowRows: [],
        },
        retrospectiveNote: '测试笔记',
      };

      await exportReportMarkdown(context);
      expect(exportMarkdown).toHaveBeenCalled();
      expect(message.success).toHaveBeenCalledWith('已导出（Markdown）');
    });
  });

  describe('exportReportHTML', () => {
    it('应该生成包含 XSS 转义的 HTML', async () => {
      const context: ExportContext = {
        compareResult: {
          ...mockCompareResult,
          version_id_a: 'v1<script>',
          version_id_b: 'v2',
        },
        currentUser: '<img src=x>',
        localDiffResult: null,
        localCapacityRows: null,
        retrospectiveNote: '<img src=x onerror=alert("xss")>',
      };

      await exportReportHTML(context);
      expect(exportHTML).toHaveBeenCalled();
      const html = (exportHTML as any).mock.calls[0][0];

      // 验证 XSS 转义
      expect(html).toContain('&lt;script&gt;');
      expect(html).not.toContain('<script>');
      expect(html).toContain('&lt;img');
      expect(html).not.toContain('<img src=x');
      expect(html).toContain('&quot;');
    });

    it('应该处理 null 的本地数据', async () => {
      const context: ExportContext = {
        compareResult: mockCompareResult,
        currentUser: 'test',
        localDiffResult: null,
        localCapacityRows: null,
        retrospectiveNote: '',
      };

      await exportReportHTML(context);
      expect(exportHTML).toHaveBeenCalled();
      expect(message.success).toHaveBeenCalledWith('已导出（HTML）');
    });

    it('错误时应该显示错误信息', async () => {
      vi.mocked(exportHTML).mockImplementationOnce(() => {
        throw new Error('HTML export failed');
      });

      const context: ExportContext = {
        compareResult: mockCompareResult,
        currentUser: 'test',
        localDiffResult: null,
        localCapacityRows: null,
        retrospectiveNote: '',
      };

      await exportReportHTML(context);
      expect(message.error).toHaveBeenCalledWith('HTML export failed');
    });
  });

  describe('XSS 安全测试', () => {
    it('HTML 导出应该防护所有 XSS 攻击向量', async () => {
      const xssPayloads = [
        '"><script>alert("xss")</script><"',
        "<img src=x onerror=\"alert('xss')\">",
        '<svg onload="alert(1)">',
        '<iframe src="javascript:alert(1)"></iframe>',
      ];

      for (const payload of xssPayloads) {
        vi.clearAllMocks();
        const context: ExportContext = {
          compareResult: mockCompareResult,
          currentUser: payload,
          localDiffResult: null,
          localCapacityRows: null,
          retrospectiveNote: payload,
        };

        await exportReportHTML(context);
        const html = (exportHTML as any).mock.calls[0][0];

        // 验证 HTML 特殊字符被正确转义
        // 这些标签相关的尖括号应该被转义
        expect(html).not.toContain('<script>');
        expect(html).not.toContain('<svg');
        expect(html).not.toContain('<iframe');
        // 转义后的尖括号应该存在
        expect(html).toContain('&lt;');
      }
    });

    it('应该特别防护常见的 XSS 向量', async () => {
      const context: ExportContext = {
        compareResult: mockCompareResult,
        currentUser: '"><script>alert(1)</script>',
        localDiffResult: null,
        localCapacityRows: null,
        retrospectiveNote: '"><script>alert(2)</script>',
      };

      await exportReportHTML(context);
      const html = (exportHTML as any).mock.calls[0][0];

      // 验证 script 标签被转义
      expect(html).not.toContain('<script>');
      expect(html).toContain('&lt;script&gt;');
    });
  });
});
