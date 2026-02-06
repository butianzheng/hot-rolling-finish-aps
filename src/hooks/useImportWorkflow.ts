/**
 * 材料导入工作流状态管理 Hook
 * 集中管理所有状态和业务逻辑（不包含 JSX）
 */

import { useCallback, useMemo, useState } from 'react';
import { Modal, message } from 'antd';
import type { TablePaginationConfig } from 'antd/es/table';
import { open } from '@tauri-apps/api/dialog';
import { readTextFile } from '@tauri-apps/api/fs';
import { useNavigate } from 'react-router-dom';
import { parseCsvPreviewSmart } from '../utils/csvParser';
import { safeReadImportHistory, safeWriteImportHistory } from '../utils/importHistoryStorage';
import {
  REQUIRED_HEADERS,
  IMPORT_HISTORY_MAX,
  type DqViolation,
  type DqSummary,
  type ImportMaterialsResponse,
  type ImportHistoryItem,
  type ImportConflict,
  type ImportConflictListResponse,
  type PreviewRow,
} from '../types/import';
import { importApi } from '../api/tauri';

/**
 * useImportWorkflow Hook 返回接口
 */
export interface UseImportWorkflowReturn {
  // Tab 管理
  activeTab: 'import' | 'conflicts' | 'history';
  setActiveTab: (tab: 'import' | 'conflicts' | 'history') => void;

  // 文件 & 预览
  selectedFilePath: string;
  previewHeaders: string[];
  previewRows: PreviewRow[];
  previewTotalRows: number;
  previewLoading: boolean;
  missingHeaders: string[];
  loadPreview: (filePath: string) => Promise<void>;
  handleSelectFile: (isTauriRuntime: boolean) => Promise<void>;

  // 导入执行
  batchId: string;
  setBatchId: (id: string) => void;
  mappingProfileId: string;
  setMappingProfileId: (id: string) => void;
  importLoading: boolean;
  importResult: ImportMaterialsResponse | null;
  doImport: (options?: { currentUser: string; setImporting: (v: boolean) => void }) => Promise<void>;
  handleImport: (options?: { selectedFilePath?: string; missingHeaders?: string[] }) => Promise<void>;
  dqStats: {
    summary: DqSummary | undefined;
    byLevel: Record<string, number>;
    topFields: Array<{ field: string; count: number }>;
    violations: DqViolation[];
  };

  // 冲突管理
  conflictStatus: 'OPEN' | 'RESOLVED' | 'ALL';
  setConflictStatus: (status: 'OPEN' | 'RESOLVED' | 'ALL') => void;
  conflictBatchId: string;
  setConflictBatchId: (id: string) => void;
  conflicts: ImportConflict[];
  conflictPagination: TablePaginationConfig;
  conflictsLoading: boolean;
  loadConflicts: (opts?: {
    status?: 'OPEN' | 'RESOLVED' | 'ALL';
    batchId?: string;
    page?: number;
    pageSize?: number;
  }) => Promise<{ list: ImportConflict[]; total: number }>;
  handleResolveConflict: (conflictId: string, action: 'KEEP_EXISTING' | 'OVERWRITE' | 'MERGE', options?: { currentUser: string }) => Promise<void>;

  // 历史 & Modal
  importHistory: ImportHistoryItem[];
  rawModal: { open: boolean; title: string; content: string };
  setRawModal: (modal: { open: boolean; title: string; content: string }) => void;
}

/**
 * 材料导入工作流 Hook
 * 管理导入流程的所有状态和回调
 */
export function useImportWorkflow(): UseImportWorkflowReturn {
  const navigate = useNavigate();

  // ========== Tab 管理 ==========
  const [activeTab, setActiveTab] = useState<'import' | 'conflicts' | 'history'>('import');

  // ========== 文件选择 & 预览 ==========
  const [selectedFilePath, setSelectedFilePath] = useState<string>('');
  const [previewHeaders, setPreviewHeaders] = useState<string[]>([]);
  const [previewRows, setPreviewRows] = useState<PreviewRow[]>([]);
  const [previewTotalRows, setPreviewTotalRows] = useState<number>(0);
  const [previewLoading, setPreviewLoading] = useState(false);

  // ========== 导入参数 ==========
  const [batchId, setBatchId] = useState<string>(() => `BATCH_${Date.now()}`);
  const [mappingProfileId, setMappingProfileId] = useState<string>('');

  // ========== 导入执行 ==========
  const [importLoading, setImportLoading] = useState(false);
  const [importResult, setImportResult] = useState<ImportMaterialsResponse | null>(null);

  // ========== 冲突管理 ==========
  const [conflictsLoading, setConflictsLoading] = useState(false);
  const [conflictStatus, setConflictStatus] = useState<'OPEN' | 'RESOLVED' | 'ALL'>('OPEN');
  const [conflictBatchId, setConflictBatchId] = useState<string>('');
  const [conflicts, setConflicts] = useState<ImportConflict[]>([]);
  const [conflictPagination, setConflictPagination] = useState<TablePaginationConfig>({
    current: 1,
    pageSize: 20,
    total: 0,
    showSizeChanger: true,
  });

  // ========== 历史 & Modal ==========
  const [importHistory, setImportHistory] = useState<ImportHistoryItem[]>(() => safeReadImportHistory());
  const [rawModal, setRawModal] = useState<{ open: boolean; title: string; content: string }>({
    open: false,
    title: '',
    content: '',
  });

  // ========== 计算属性 ==========
  const missingHeaders = useMemo(() => {
    const headerSet = new Set(previewHeaders);
    return REQUIRED_HEADERS.filter((h) => !headerSet.has(h));
  }, [previewHeaders]);

  const dqStats = useMemo(() => {
    const summary = importResult?.dq_summary;
    const violations = Array.isArray(importResult?.dq_violations) ? importResult!.dq_violations! : [];

    const byLevel: Record<string, number> = {};
    const byField: Record<string, number> = {};
    for (const v of violations) {
      byLevel[v.level] = (byLevel[v.level] ?? 0) + 1;
      byField[v.field] = (byField[v.field] ?? 0) + 1;
    }

    const topFields = Object.entries(byField)
      .sort((a, b) => b[1] - a[1])
      .slice(0, 8)
      .map(([field, count]) => ({ field, count }));

    return { summary, byLevel, topFields, violations };
  }, [importResult]);

  // ========== 业务逻辑 - 文件操作 ==========
  const loadPreview = useCallback(async (filePath: string) => {
    setPreviewLoading(true);
    try {
      const content = await readTextFile(filePath);
      // 使用智能解析器：大文件自动使用 Web Worker
      const parsed = await parseCsvPreviewSmart(content, 20);
      setPreviewHeaders(parsed.headers);
      setPreviewRows(parsed.rows);
      setPreviewTotalRows(parsed.totalRows);
    } catch (e: any) {
      console.error('【材料导入】预览失败：', e);
      setPreviewHeaders([]);
      setPreviewRows([]);
      setPreviewTotalRows(0);
      message.error(e?.message || '读取文件失败');
    } finally {
      setPreviewLoading(false);
    }
  }, []);

  const handleSelectFile = useCallback(
    async (isTauriRuntime: boolean) => {
      if (!isTauriRuntime) {
        message.warning('材料导入需要在桌面端运行');
        return;
      }

      try {
        const selected = await open({
          multiple: false,
          filters: [{ name: '逗号分隔文件', extensions: ['csv'] }],
        });

        if (selected && typeof selected === 'string') {
          if (!selected.toLowerCase().endsWith('.csv')) {
            message.error('当前仅支持逗号分隔格式文件');
            return;
          }

          setSelectedFilePath(selected);
          await loadPreview(selected);
          message.success('文件已选择');
        }
      } catch (e: any) {
        console.error('【材料导入】选择文件失败：', e);
        message.error(e?.message || '文件选择失败');
      }
    },
    [loadPreview],
  );

  // ========== 业务逻辑 - 导入操作 ==========
  const appendHistory = useCallback(
    (result: ImportMaterialsResponse, currentUser: string) => {
      const now = new Date().toISOString();
      const internalBatch = String(result?.import_batch_id || '').trim() || null;
      const fallbackId = internalBatch || String(result?.batch_id || '').trim() || `IMPORT_${Date.now()}`;

      const item: ImportHistoryItem = {
        id: fallbackId,
        created_at: now,
        operator: currentUser || 'admin',
        file_path: selectedFilePath || '-',
        source_batch_id: batchId.trim() || '-',
        import_batch_id: internalBatch,
        imported: Number(result?.imported ?? 0),
        updated: Number(result?.updated ?? 0),
        conflicts: Number(result?.conflicts ?? 0),
        elapsed_ms: typeof result?.elapsed_ms === 'number' ? result.elapsed_ms : null,
      };

      setImportHistory((prev) => {
        const next = [item, ...prev.filter((x) => x.id !== item.id)].slice(0, IMPORT_HISTORY_MAX);
        safeWriteImportHistory(next);
        return next;
      });
    },
    [batchId, selectedFilePath],
  );

  const loadConflicts = useCallback(
    async (opts?: {
      status?: 'OPEN' | 'RESOLVED' | 'ALL';
      batchId?: string;
      page?: number;
      pageSize?: number;
    }): Promise<{ list: ImportConflict[]; total: number }> => {
      const status = opts?.status ?? conflictStatus;
      const batch = (opts?.batchId ?? conflictBatchId).trim();
      const page = opts?.page ?? (conflictPagination.current as number) ?? 1;
      const pageSize = opts?.pageSize ?? (conflictPagination.pageSize as number) ?? 20;

      setConflictsLoading(true);
      try {
        const apiStatus = status === 'ALL' ? undefined : status;
        const resp = (await importApi.listImportConflicts(
          apiStatus,
          pageSize,
          (page - 1) * pageSize,
          batch || undefined,
        )) as ImportConflictListResponse;

        const list = Array.isArray(resp?.conflicts) ? resp.conflicts : [];
        const total = typeof resp?.total === 'number' ? resp.total : list.length;
        setConflicts(list);
        setConflictPagination((prev) => ({
          ...prev,
          current: page,
          pageSize,
          total,
        }));
        return { list, total };
      } catch (e: any) {
        console.error('【材料导入】加载冲突失败：', e);
        setConflicts([]);
        setConflictPagination((prev) => ({ ...prev, total: 0 }));
        message.error(e?.message || '加载冲突列表失败');
        return { list: [], total: 0 };
      } finally {
        setConflictsLoading(false);
      }
    },
    [conflictBatchId, conflictPagination.current, conflictPagination.pageSize, conflictStatus],
  );

  const doImport = useCallback(
    async (importOptions?: { currentUser: string; setImporting: (v: boolean) => void }) => {
      const currentUser = importOptions?.currentUser || 'admin';
      const setImportingFunc = importOptions?.setImporting || (() => void 0);

      setImportLoading(true);
      setImportingFunc(true);
      setImportResult(null);

      let shouldNavigateToWorkbench = false;

      try {
        const result = (await importApi.importMaterials(
          selectedFilePath,
          batchId.trim() || `BATCH_${Date.now()}`,
          mappingProfileId.trim() || undefined,
        )) as ImportMaterialsResponse;

        setImportResult(result);
        appendHistory(result, currentUser);

        const internalBatch = String(result?.import_batch_id || '').trim();
        if (internalBatch) {
          setConflictBatchId(internalBatch);
        }

        message.success(
          `导入完成：成功 ${Number(result?.imported || 0)} 条，冲突 ${Number(result?.conflicts || 0)} 条`,
        );

        // 自动切到冲突页并加载本批次 OPEN 冲突
        if (Number(result?.conflicts || 0) > 0 && internalBatch) {
          setActiveTab('conflicts');
          setConflictStatus('OPEN');
          await loadConflicts({ status: 'OPEN', batchId: internalBatch, page: 1 });
        } else {
          // 无冲突：导入成功后自动跳转到计划工作台
          shouldNavigateToWorkbench = true;
        }
      } catch (e: any) {
        console.error('【材料导入】执行导入失败：', e);
        message.error(e?.message || '导入失败');
        // H5修复：导入失败后清空相关状态，避免显示过期的导入结果
        setImportResult(null);
        setBatchId(`BATCH_${Date.now()}`);
      } finally {
        setImportLoading(false);
        setImportingFunc(false);
      }

      if (shouldNavigateToWorkbench) {
        navigate('/workbench', { replace: true });
      }
    },
    [batchId, loadConflicts, mappingProfileId, navigate, selectedFilePath, appendHistory],
  );

  const handleImport = useCallback(
    async (importOptions?: { selectedFilePath?: string; missingHeaders?: string[] }) => {
      const filePath = importOptions?.selectedFilePath ?? selectedFilePath;
      const missing = importOptions?.missingHeaders ?? missingHeaders;

      if (!filePath) {
        message.warning('请先选择逗号分隔文件');
        return;
      }

      if (missing.length > 0) {
        return new Promise<void>((resolve) => {
          Modal.confirm({
            title: '检测到列名缺失',
            content: `预览表头缺少必需列：${missing.join('、')}。继续导入可能导致大量冲突/阻断，仍要继续吗？`,
            okText: '继续导入',
            cancelText: '取消',
            onOk: async () => {
              await doImport();
              resolve();
            },
            onCancel: () => {
              resolve();
            },
          });
        });
      }

      await doImport();
    },
    [doImport, missingHeaders, selectedFilePath],
  );

  // ========== 业务逻辑 - 冲突处理 ==========
  const handleResolveConflict = useCallback(
    async (conflictId: string, action: 'KEEP_EXISTING' | 'OVERWRITE' | 'MERGE', options?: { currentUser: string }) => {
      const currentUser = options?.currentUser || 'admin';

      try {
        await importApi.resolveImportConflict(conflictId, action, `由${currentUser}处理`, currentUser);
        message.success('冲突已处理');
        const res = await loadConflicts();
        if (conflictStatus === 'OPEN' && res.total === 0 && conflictBatchId.trim()) {
          Modal.success({
            title: '本批次冲突已全部处理',
            content: '可以进入计划工作台继续排产操作。',
            okText: '去计划工作台',
            onOk: () => navigate('/workbench'),
          });
        }
      } catch (e: any) {
        console.error('【材料导入】处理冲突失败：', e);
        message.error(e?.message || '处理冲突失败');
      }
    },
    [conflictBatchId, conflictStatus, loadConflicts, navigate],
  );

  return {
    activeTab,
    setActiveTab,
    selectedFilePath,
    previewHeaders,
    previewRows,
    previewTotalRows,
    previewLoading,
    missingHeaders,
    loadPreview,
    handleSelectFile,
    batchId,
    setBatchId,
    mappingProfileId,
    setMappingProfileId,
    importLoading,
    importResult,
    doImport,
    handleImport,
    dqStats,
    conflictStatus,
    setConflictStatus,
    conflictBatchId,
    setConflictBatchId,
    conflicts,
    conflictPagination,
    conflictsLoading,
    loadConflicts,
    handleResolveConflict,
    importHistory,
    rawModal,
    setRawModal,
  };
}
