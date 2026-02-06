/**
 * 导入 Tab 内容组件
 * 处理文件选择、CSV 预览、导入执行和结果展示
 */

import React, { useMemo } from 'react';
import {
  Alert,
  Button,
  Card,
  Col,
  Descriptions,
  Divider,
  Form,
  Input,
  Row,
  Space,
  Spin,
  Statistic,
  Table,
  Typography,
} from 'antd';
import type { ColumnsType } from 'antd/es/table';
import { FolderOpenOutlined, ReloadOutlined, UploadOutlined } from '@ant-design/icons';
import { formatMs } from '../../utils/importFormatters';
import {
  REQUIRED_HEADERS,
  type DqSummary,
  type ImportMaterialsResponse,
  type PreviewRow,
} from '../../types/import';

const { Text } = Typography;

export interface ImportTabContentProps {
  // 环境
  isTauriRuntime: boolean;
  currentUser: string;

  // 文件选择 & 预览
  selectedFilePath: string;
  previewHeaders: string[];
  previewRows: PreviewRow[];
  previewTotalRows: number;
  previewLoading: boolean;
  missingHeaders: string[];

  // 导入参数
  batchId: string;
  onBatchIdChange: (id: string) => void;
  mappingProfileId: string;
  onMappingProfileIdChange: (id: string) => void;

  // 导入执行
  importLoading: boolean;
  importResult: ImportMaterialsResponse | null;
  dqStats: {
    summary: DqSummary | undefined;
    byLevel: Record<string, number>;
    topFields: Array<{ field: string; count: number }>;
  };

  // 回调
  onSelectFile: () => Promise<void>;
  onImport: () => Promise<void>;
  onRefreshPreview: (filePath: string) => Promise<void>;
  onNavigateToWorkbench: () => void;
}

export const ImportTabContent: React.FC<ImportTabContentProps> = ({
  isTauriRuntime,
  currentUser,
  selectedFilePath,
  previewHeaders,
  previewRows,
  previewTotalRows,
  previewLoading,
  missingHeaders,
  batchId,
  onBatchIdChange,
  mappingProfileId,
  onMappingProfileIdChange,
  importLoading,
  importResult,
  dqStats,
  onSelectFile,
  onImport,
  onRefreshPreview,
  onNavigateToWorkbench,
}) => {
  // 预览表格列定义
  const previewColumns: ColumnsType<PreviewRow> = useMemo(() => {
    return previewHeaders.map((h) => ({
      title: h,
      dataIndex: h,
      key: h,
      width: 140,
      render: (v: any) => <span style={{ fontFamily: 'monospace' }}>{String(v ?? '')}</span>,
    }));
  }, [previewHeaders]);

  return (
    <Row gutter={[16, 16]}>
      <Col xs={24} lg={14}>
        <Card
          title="文件与参数"
          extra={
            <Space>
              <Button icon={<FolderOpenOutlined />} onClick={onSelectFile} disabled={!isTauriRuntime}>
                选择 CSV
              </Button>
              <Button
                type="primary"
                icon={<UploadOutlined />}
                onClick={onImport}
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
                  <Form.Item
                    label="批次标识"
                    tooltip="用于前端与审计标识；实际落库批次编号将在导入后返回"
                  >
                    <Input
                      value={batchId}
                      onChange={(e) => onBatchIdChange(e.target.value)}
                      placeholder="例如：批次_20260126_001"
                    />
                  </Form.Item>
                </Col>
                <Col xs={24} md={12}>
                  <Form.Item label="映射配置编号（可选）" tooltip="预留字段，当前后端暂未启用映射配置">
                    <Input
                      value={mappingProfileId}
                      onChange={(e) => onMappingProfileIdChange(e.target.value)}
                      placeholder="可留空"
                    />
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
                      dataSource={previewRows.map((r, idx) => ({ ...r, __key: String(idx) }) as any)}
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
                  <Statistic
                    title="冲突入队"
                    value={Number(importResult.conflicts || 0)}
                    valueStyle={{ color: '#faad14' }}
                  />
                </Col>
                <Col span={12}>
                  <Statistic
                    title="阻断（错误）"
                    value={Number(importResult.dq_summary?.blocked || 0)}
                    valueStyle={{ color: '#ff4d4f' }}
                  />
                </Col>
                <Col span={12}>
                  <Statistic
                    title="警告（提示）"
                    value={Number(importResult.dq_summary?.warning || 0)}
                    valueStyle={{ color: '#1677ff' }}
                  />
                </Col>
              </Row>

              <Descriptions column={1} size="small" bordered>
                <Descriptions.Item label="来源批次标识">
                  <Text style={{ fontFamily: 'monospace' }}>{importResult.batch_id || '-'}</Text>
                </Descriptions.Item>
                <Descriptions.Item label="导入批次标识">
                  <Text style={{ fontFamily: 'monospace' }}>{importResult.import_batch_id || '-'}</Text>
                </Descriptions.Item>
                <Descriptions.Item label="耗时">{formatMs(importResult.elapsed_ms)}</Descriptions.Item>
              </Descriptions>

              {dqStats.topFields.length > 0 && (
                <Card size="small" title="数据质量摘要（高频字段）">
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
                <Button onClick={onNavigateToWorkbench}>去计划工作台查看</Button>
                <Button
                  icon={<ReloadOutlined />}
                  onClick={() => {
                    if (selectedFilePath) onRefreshPreview(selectedFilePath);
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
  );
};

export default ImportTabContent;
