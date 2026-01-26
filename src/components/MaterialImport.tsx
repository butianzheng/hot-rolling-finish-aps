import React, { useCallback, useMemo, useState } from 'react';
import {
  Alert,
  Button,
  Card,
  Col,
  Descriptions,
  Divider,
  Form,
  Input,
  Modal,
  Row,
  Select,
  Space,
  Spin,
  Statistic,
  Table,
  Tabs,
  Tag,
  Typography,
  message,
} from 'antd';
import type { ColumnsType, TablePaginationConfig } from 'antd/es/table';
import {
  CheckCircleOutlined,
  ExclamationCircleOutlined,
  FolderOpenOutlined,
  ReloadOutlined,
  UploadOutlined,
} from '@ant-design/icons';
import { open } from '@tauri-apps/api/dialog';
import { readTextFile } from '@tauri-apps/api/fs';
import { useNavigate } from 'react-router-dom';
import { importApi } from '../api/tauri';
import { useCurrentUser, useGlobalActions } from '../stores/use-global-store';

const { Title, Text, Paragraph } = Typography;

type DqViolation = {
  row_number: number;
  material_id: string | null;
  level: 'Error' | 'Warning' | 'Info' | 'Conflict' | string;
  field: string;
  message: string;
};

type DqSummary = {
  total_rows: number;
  success: number;
  blocked: number;
  warning: number;
  conflict: number;
};

type ImportMaterialsResponse = {
  imported: number;
  updated: number;
  conflicts: number;
  batch_id: string;
  import_batch_id?: string;
  dq_summary?: DqSummary;
  dq_violations?: DqViolation[];
  elapsed_ms?: number;
};

type ImportConflict = {
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

type ImportConflictListResponse = {
  conflicts: ImportConflict[];
  total: number;
  limit: number;
  offset: number;
};

type PreviewRow = Record<string, string>;

const REQUIRED_HEADERS = ['材料号', '材料实际重量', '下道机组代码'];

function splitCsvLine(line: string): string[] {
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

function parseCsvPreview(content: string, maxRows: number): { headers: string[]; rows: PreviewRow[]; totalRows: number } {
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

function formatMs(ms?: number): string {
  if (typeof ms !== 'number' || Number.isNaN(ms)) return '-';
  if (ms < 1000) return `${ms}ms`;
  return `${(ms / 1000).toFixed(2)}s`;
}

function conflictTypeLabel(t: string): string {
  const map: Record<string, string> = {
    PrimaryKeyMissing: '主键缺失',
    PrimaryKeyDuplicate: '主键重复',
    ForeignKeyViolation: '外键/引用错误',
    DataTypeError: '数据类型错误',
  };
  return map[t] || t || '-';
}

const MaterialImport: React.FC = () => {
  const navigate = useNavigate();
  const currentUser = useCurrentUser();
  const { setImporting } = useGlobalActions();

  const isTauriRuntime = typeof window !== 'undefined' && !!(window as any).__TAURI__;

  const [activeTab, setActiveTab] = useState<'import' | 'conflicts'>('import');
  const [selectedFilePath, setSelectedFilePath] = useState<string>('');
  const [previewHeaders, setPreviewHeaders] = useState<string[]>([]);
  const [previewRows, setPreviewRows] = useState<PreviewRow[]>([]);
  const [previewTotalRows, setPreviewTotalRows] = useState<number>(0);
  const [previewLoading, setPreviewLoading] = useState(false);

  const [batchId, setBatchId] = useState<string>(() => `BATCH_${Date.now()}`);
  const [mappingProfileId, setMappingProfileId] = useState<string>('');

  const [importLoading, setImportLoading] = useState(false);
  const [importResult, setImportResult] = useState<ImportMaterialsResponse | null>(null);

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
  const [rawModal, setRawModal] = useState<{ open: boolean; title: string; content: string }>({
    open: false,
    title: '',
    content: '',
  });

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
      // field 可能包含多个字段，以逗号分隔；这里按原值统计即可
      byField[v.field] = (byField[v.field] ?? 0) + 1;
    }

    const topFields = Object.entries(byField)
      .sort((a, b) => b[1] - a[1])
      .slice(0, 8)
      .map(([field, count]) => ({ field, count }));

    return { summary, byLevel, topFields, violations };
  }, [importResult]);

  const previewColumns: ColumnsType<PreviewRow> = useMemo(() => {
    const cols = previewHeaders.map((h) => ({
      title: h,
      dataIndex: h,
      key: h,
      width: 140,
      render: (v: any) => <span style={{ fontFamily: 'monospace' }}>{String(v ?? '')}</span>,
    }));
    return cols;
  }, [previewHeaders]);

  const loadPreview = useCallback(async (filePath: string) => {
    setPreviewLoading(true);
    try {
      const content = await readTextFile(filePath);
      const parsed = parseCsvPreview(content, 20);
      setPreviewHeaders(parsed.headers);
      setPreviewRows(parsed.rows);
      setPreviewTotalRows(parsed.totalRows);
    } catch (e: any) {
      console.error('[MaterialImport] preview failed:', e);
      setPreviewHeaders([]);
      setPreviewRows([]);
      setPreviewTotalRows(0);
      message.error(e?.message || '读取文件失败');
    } finally {
      setPreviewLoading(false);
    }
  }, []);

  const handleSelectFile = useCallback(async () => {
    if (!isTauriRuntime) {
      message.warning('材料导入需要在 Tauri 桌面端运行');
      return;
    }

    try {
      const selected = await open({
        multiple: false,
        filters: [{ name: 'CSV 文件', extensions: ['csv'] }],
      });

      if (selected && typeof selected === 'string') {
        if (!selected.toLowerCase().endsWith('.csv')) {
          message.error('当前仅支持 CSV (.csv) 格式文件');
          return;
        }

        setSelectedFilePath(selected);
        await loadPreview(selected);
        message.success('文件已选择');
      }
    } catch (e: any) {
      console.error('[MaterialImport] select file failed:', e);
      message.error(e?.message || '文件选择失败');
    }
  }, [isTauriRuntime, loadPreview]);

  const loadConflicts = useCallback(
    async (opts?: { status?: 'OPEN' | 'RESOLVED' | 'ALL'; batchId?: string; page?: number; pageSize?: number }) => {
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
          batch || undefined
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
      } catch (e: any) {
        console.error('[MaterialImport] load conflicts failed:', e);
        setConflicts([]);
        setConflictPagination((prev) => ({ ...prev, total: 0 }));
        message.error(e?.message || '加载冲突列表失败');
      } finally {
        setConflictsLoading(false);
      }
    },
    [conflictBatchId, conflictPagination.current, conflictPagination.pageSize, conflictStatus]
  );

  const doImport = useCallback(async () => {
    setImportLoading(true);
    setImporting(true);
    setImportResult(null);

    try {
      const result = (await importApi.importMaterials(
        selectedFilePath,
        batchId.trim() || `BATCH_${Date.now()}`,
        mappingProfileId.trim() || undefined
      )) as ImportMaterialsResponse;

      setImportResult(result);

      const internalBatch = String(result?.import_batch_id || '').trim();
      if (internalBatch) {
        setConflictBatchId(internalBatch);
      }

      message.success(`导入完成：成功 ${Number(result?.imported || 0)} 条，冲突 ${Number(result?.conflicts || 0)} 条`);

      // 自动切到冲突页并加载本批次 OPEN 冲突
      if (Number(result?.conflicts || 0) > 0 && internalBatch) {
        setActiveTab('conflicts');
        setConflictStatus('OPEN');
        await loadConflicts({ status: 'OPEN', batchId: internalBatch, page: 1 });
      }
    } catch (e: any) {
      console.error('[MaterialImport] import failed:', e);
      message.error(e?.message || '导入失败');
    } finally {
      setImportLoading(false);
      setImporting(false);
    }
  }, [batchId, loadConflicts, mappingProfileId, selectedFilePath, setImporting]);

  const handleImport = useCallback(async () => {
    if (!selectedFilePath) {
      message.warning('请先选择 CSV 文件');
      return;
    }

    if (missingHeaders.length > 0) {
      Modal.confirm({
        title: '检测到列名缺失',
        content: (
          <div>
            <p>
              预览表头缺少必需列：<Text strong>{missingHeaders.join('、')}</Text>
            </p>
            <p>继续导入可能导致大量冲突/阻断，仍要继续吗？</p>
          </div>
        ),
        okText: '继续导入',
        cancelText: '取消',
        onOk: async () => {
          await doImport();
        },
      });
      return;
    }

    await doImport();
  }, [doImport, missingHeaders, selectedFilePath]);

  const handleResolveConflict = useCallback(
    async (conflictId: string, action: 'KEEP_EXISTING' | 'OVERWRITE' | 'MERGE') => {
      try {
        await importApi.resolveImportConflict(conflictId, action, `由${currentUser}处理`, currentUser);
        message.success('冲突已处理');
        await loadConflicts();
      } catch (e: any) {
        console.error('[MaterialImport] resolve conflict failed:', e);
        message.error(e?.message || '处理冲突失败');
      }
    },
    [currentUser, loadConflicts]
  );

  const conflictColumns: ColumnsType<ImportConflict> = useMemo(
    () => [
      {
        title: '状态',
        dataIndex: 'resolved',
        key: 'resolved',
        width: 90,
        render: (resolved: boolean) => (
          <Tag color={resolved ? 'green' : 'red'} icon={resolved ? <CheckCircleOutlined /> : <ExclamationCircleOutlined />}>
            {resolved ? '已处理' : 'OPEN'}
          </Tag>
        ),
      },
      {
        title: '批次',
        dataIndex: 'batch_id',
        key: 'batch_id',
        width: 180,
        ellipsis: true,
        render: (v: string) => <span style={{ fontFamily: 'monospace' }}>{v}</span>,
      },
      {
        title: '行号',
        dataIndex: 'row_number',
        key: 'row_number',
        width: 80,
      },
      {
        title: '材料号',
        dataIndex: 'material_id',
        key: 'material_id',
        width: 140,
        render: (v: string | null) => (v ? <Tag color="blue">{v}</Tag> : '-'),
      },
      {
        title: '冲突类型',
        dataIndex: 'conflict_type',
        key: 'conflict_type',
        width: 160,
        render: (t: string) => conflictTypeLabel(t),
      },
      {
        title: '原因',
        dataIndex: 'reason',
        key: 'reason',
        ellipsis: true,
        render: (v: string) => v || '-',
      },
      {
        title: '原始数据',
        dataIndex: 'raw_data',
        key: 'raw_data',
        width: 120,
        render: (raw: string, record) => (
          <Button
            size="small"
            onClick={() =>
              setRawModal({
                open: true,
                title: `冲突原始数据（${record.material_id || record.conflict_id}）`,
                content: raw || '{}',
              })
            }
          >
            查看
          </Button>
        ),
      },
      {
        title: '处理',
        key: 'actions',
        width: 220,
        render: (_: any, record) => (
          <Space>
            <Button
              size="small"
              disabled={record.resolved}
              onClick={() => handleResolveConflict(record.conflict_id, 'KEEP_EXISTING')}
            >
              保留现有
            </Button>
            <Button
              size="small"
              danger
              disabled={record.resolved}
              onClick={() =>
                Modal.confirm({
                  title: '确认覆盖？',
                  content: '将用导入数据覆盖现有材料主数据。此操作不可逆。',
                  okText: '覆盖',
                  cancelText: '取消',
                  onOk: () => handleResolveConflict(record.conflict_id, 'OVERWRITE'),
                })
              }
            >
              覆盖
            </Button>
          </Space>
        ),
      },
    ],
    [handleResolveConflict]
  );

  return (
    <div style={{ padding: 24 }}>
      <Title level={2} style={{ marginBottom: 0 }}>
        材料导入
      </Title>
      <Paragraph type="secondary" style={{ marginTop: 8 }}>
        当前导入通道基于后端 MaterialImporter：CSV 解析 → 字段映射 → 清洗/派生 → DQ 校验 → 冲突入队 → 落库。
      </Paragraph>

      {!isTauriRuntime && (
        <Alert
          type="warning"
          showIcon
          message="当前运行环境不支持材料导入"
          description="材料导入依赖 Tauri 桌面端的文件选择与本地文件读取能力，请在 Tauri 窗口中使用该功能。"
          style={{ marginBottom: 16 }}
        />
      )}

      <Tabs
        activeKey={activeTab}
        onChange={(k) => {
          const key = k as 'import' | 'conflicts';
          setActiveTab(key);
          if (key === 'conflicts') {
            // 进入冲突页时，如果已有批次ID则自动加载
            loadConflicts({ page: 1 }).catch(() => void 0);
          }
        }}
        items={[
          {
            key: 'import',
            label: '导入',
            children: (
              <Row gutter={[16, 16]}>
                <Col xs={24} lg={14}>
                  <Card
                    title="文件与参数"
                    extra={
                      <Space>
                        <Button icon={<FolderOpenOutlined />} onClick={handleSelectFile} disabled={!isTauriRuntime}>
                          选择 CSV
                        </Button>
                        <Button
                          type="primary"
                          icon={<UploadOutlined />}
                          onClick={handleImport}
                          loading={importLoading}
                          disabled={!isTauriRuntime || !selectedFilePath}
                        >
                          开始导入
                        </Button>
                      </Space>
                    }
                  >
                    <Space direction="vertical" size={12} style={{ width: '100%' }}>
                      <Alert
                        type="info"
                        showIcon
                        message="文件格式要求（当前版本）"
                        description={
                          <div>
                            <div>1) 仅支持 CSV（UTF-8，逗号分隔，首行为表头）</div>
                            <div>
                              2) 必需列：<Text strong>{REQUIRED_HEADERS.join('、')}</Text>
                            </div>
                          </div>
                        }
                      />

                      <Descriptions column={1} size="small" bordered>
                        <Descriptions.Item label="文件路径">
                          {selectedFilePath ? (
                            <Text style={{ fontFamily: 'monospace' }}>{selectedFilePath}</Text>
                          ) : (
                            <Text type="secondary">未选择</Text>
                          )}
                        </Descriptions.Item>
                        <Descriptions.Item label="预览行数">
                          {previewTotalRows ? `${Math.min(20, previewTotalRows)}/${previewTotalRows}` : '-'}
                        </Descriptions.Item>
                        <Descriptions.Item label="导入人">{currentUser}</Descriptions.Item>
                      </Descriptions>

                      <Form layout="vertical">
                        <Row gutter={12}>
                          <Col xs={24} md={12}>
                            <Form.Item label="批次标识（source_batch_id）" tooltip="用于前端/审计标识；实际落库批次ID将在导入后返回">
                              <Input value={batchId} onChange={(e) => setBatchId(e.target.value)} placeholder="例如：BATCH_20260126_001" />
                            </Form.Item>
                          </Col>
                          <Col xs={24} md={12}>
                            <Form.Item label="映射配置ID（可选）" tooltip="预留字段，当前后端未启用映射配置">
                              <Input value={mappingProfileId} onChange={(e) => setMappingProfileId(e.target.value)} placeholder="可留空" />
                            </Form.Item>
                          </Col>
                        </Row>
                      </Form>

                      <Divider style={{ margin: '8px 0' }} />

                      <Card size="small" title="文件预览" styles={{ body: { padding: 0 } }}>
                        <Spin spinning={previewLoading}>
                          {selectedFilePath && previewHeaders.length > 0 ? (
                            <>
                              {missingHeaders.length > 0 && (
                                <Alert
                                  type="warning"
                                  showIcon
                                  message="预览表头缺少必需列"
                                  description={`缺少：${missingHeaders.join('、')}`}
                                  style={{ margin: 12 }}
                                />
                              )}
                              <Table<PreviewRow>
                                columns={previewColumns}
                                dataSource={previewRows.map((r, idx) => ({ ...r, __key: String(idx) } as any))}
                                rowKey="__key"
                                size="small"
                                pagination={false}
                                scroll={{ x: 'max-content', y: 320 }}
                              />
                            </>
                          ) : (
                            <div style={{ padding: 16 }}>
                              <Text type="secondary">选择文件后自动读取前 20 行用于预览</Text>
                            </div>
                          )}
                        </Spin>
                      </Card>
                    </Space>
                  </Card>
                </Col>

                <Col xs={24} lg={10}>
                  <Card title="导入结果">
                    {!importResult && <Text type="secondary">尚未执行导入</Text>}

                    {importResult && (
                      <Space direction="vertical" size={12} style={{ width: '100%' }}>
                        <Row gutter={12}>
                          <Col span={12}>
                            <Statistic title="成功导入" value={Number(importResult.imported || 0)} />
                          </Col>
                          <Col span={12}>
                            <Statistic title="冲突入队" value={Number(importResult.conflicts || 0)} valueStyle={{ color: '#faad14' }} />
                          </Col>
                          <Col span={12}>
                            <Statistic title="阻断 (ERROR)" value={Number(importResult.dq_summary?.blocked || 0)} valueStyle={{ color: '#ff4d4f' }} />
                          </Col>
                          <Col span={12}>
                            <Statistic title="警告 (WARNING)" value={Number(importResult.dq_summary?.warning || 0)} valueStyle={{ color: '#1677ff' }} />
                          </Col>
                        </Row>

                        <Descriptions column={1} size="small" bordered>
                          <Descriptions.Item label="source_batch_id">
                            <Text style={{ fontFamily: 'monospace' }}>{importResult.batch_id || '-'}</Text>
                          </Descriptions.Item>
                          <Descriptions.Item label="import_batch_id">
                            <Text style={{ fontFamily: 'monospace' }}>{importResult.import_batch_id || '-'}</Text>
                          </Descriptions.Item>
                          <Descriptions.Item label="耗时">{formatMs(importResult.elapsed_ms)}</Descriptions.Item>
                        </Descriptions>

                        {dqStats.topFields.length > 0 && (
                          <Card size="small" title="DQ 摘要（Top 字段）">
                            <Table
                              size="small"
                              pagination={false}
                              dataSource={dqStats.topFields.map((t) => ({ ...t, key: t.field }))}
                              columns={[
                                { title: '字段', dataIndex: 'field', key: 'field', ellipsis: true },
                                { title: '次数', dataIndex: 'count', key: 'count', width: 90 },
                              ]}
                            />
                          </Card>
                        )}

                        <Space>
                          <Button onClick={() => navigate('/material')}>去材料管理查看</Button>
                          <Button
                            icon={<ReloadOutlined />}
                            onClick={() => {
                              if (selectedFilePath) loadPreview(selectedFilePath);
                            }}
                            disabled={!selectedFilePath}
                          >
                            刷新预览
                          </Button>
                        </Space>
                      </Space>
                    )}
                  </Card>
                </Col>
              </Row>
            ),
          },
          {
            key: 'conflicts',
            label: '冲突处理',
            children: (
              <Card
                title="导入冲突队列"
                extra={
                  <Space>
                    <Button icon={<ReloadOutlined />} onClick={() => loadConflicts({ page: 1 })} loading={conflictsLoading}>
                      刷新
                    </Button>
                  </Space>
                }
              >
                <Space direction="vertical" size={12} style={{ width: '100%' }}>
                  <Row gutter={12}>
                    <Col xs={24} md={6}>
                      <div style={{ marginBottom: 6 }}>状态</div>
                      <Select
                        value={conflictStatus}
                        style={{ width: '100%' }}
                        onChange={(v) => {
                          setConflictStatus(v);
                          loadConflicts({ status: v, page: 1 }).catch(() => void 0);
                        }}
                        options={[
                          { value: 'OPEN', label: 'OPEN' },
                          { value: 'RESOLVED', label: 'RESOLVED' },
                          { value: 'ALL', label: '全部' },
                        ]}
                      />
                    </Col>
                    <Col xs={24} md={12}>
                      <div style={{ marginBottom: 6 }}>批次ID（import_batch_id）</div>
                      <Input
                        value={conflictBatchId}
                        placeholder="留空=查询所有批次"
                        onChange={(e) => setConflictBatchId(e.target.value)}
                        onPressEnter={() => loadConflicts({ page: 1 }).catch(() => void 0)}
                      />
                    </Col>
                    <Col xs={24} md={6} style={{ display: 'flex', alignItems: 'end' }}>
                      <Button type="primary" onClick={() => loadConflicts({ page: 1 })} loading={conflictsLoading}>
                        查询
                      </Button>
                    </Col>
                  </Row>

                  <Table<ImportConflict>
                    loading={conflictsLoading}
                    columns={conflictColumns}
                    dataSource={conflicts}
                    rowKey="conflict_id"
                    pagination={conflictPagination}
                    onChange={(pagination) => {
                      const current = pagination.current ?? 1;
                      const pageSize = pagination.pageSize ?? 20;
                      loadConflicts({ page: current, pageSize }).catch(() => void 0);
                    }}
                    scroll={{ x: 1200 }}
                    size="middle"
                  />
                </Space>
              </Card>
            ),
          },
        ]}
      />

      <Modal
        title={rawModal.title}
        open={rawModal.open}
        onCancel={() => setRawModal((s) => ({ ...s, open: false }))}
        footer={[
          <Button
            key="copy"
            onClick={() => {
              navigator.clipboard.writeText(rawModal.content || '').then(
                () => message.success('已复制'),
                () => message.error('复制失败')
              );
            }}
          >
            复制
          </Button>,
          <Button key="close" type="primary" onClick={() => setRawModal((s) => ({ ...s, open: false }))}>
            关闭
          </Button>,
        ]}
        width={900}
        destroyOnClose
      >
        <pre style={{ maxHeight: 520, overflow: 'auto', margin: 0 }}>
          {(() => {
            try {
              const obj = JSON.parse(rawModal.content || '{}');
              return JSON.stringify(obj, null, 2);
            } catch {
              return rawModal.content || '';
            }
          })()}
        </pre>
      </Modal>
    </div>
  );
};

export default MaterialImport;
