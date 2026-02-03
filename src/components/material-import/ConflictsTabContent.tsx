/**
 * 冲突处理 Tab 内容组件
 * 处理导入冲突的查询、列表展示和处理操作
 */

import React, { useMemo, useState } from 'react';
import {
  Alert,
  Button,
  Card,
  Col,
  Input,
  message,
  Modal,
  Row,
  Select,
  Space,
  Table,
  Tag,
} from 'antd';
import type { ColumnsType, TablePaginationConfig } from 'antd/es/table';
import type { TableRowSelection } from 'antd/es/table/interface';
import {
  CheckCircleOutlined,
  ExclamationCircleOutlined,
  ReloadOutlined,
  WarningOutlined,
} from '@ant-design/icons';
import { conflictTypeLabel } from '../../utils/importFormatters';
import { getErrorMessage } from '../../utils/errorUtils';
import type { ImportConflict } from '../../types/import';
import { importApi } from '../../api/tauri';
import { useCurrentUser } from '../../stores/use-global-store';

export interface ConflictsTabContentProps {
  // 查询过滤
  conflictStatus: 'OPEN' | 'RESOLVED' | 'ALL';
  onStatusChange: (status: 'OPEN' | 'RESOLVED' | 'ALL') => void;
  conflictBatchId: string;
  onBatchIdChange: (id: string) => void;

  // 表格数据
  conflicts: ImportConflict[];
  conflictPagination: TablePaginationConfig;
  conflictsLoading: boolean;

  // 回调
  onLoadConflicts: (opts?: {
    status?: 'OPEN' | 'RESOLVED' | 'ALL';
    batchId?: string;
    page?: number;
    pageSize?: number;
  }) => Promise<{ list: ImportConflict[]; total: number }>;
  onResolveConflict: (
    conflictId: string,
    action: 'KEEP_EXISTING' | 'OVERWRITE' | 'MERGE',
  ) => Promise<void>;
  onBatchResolveConflicts?: (
    conflictIds: string[],
    action: 'KEEP_EXISTING' | 'OVERWRITE' | 'MERGE',
  ) => Promise<void>;
  onViewRawData: (title: string, content: string) => void;
}

export const ConflictsTabContent: React.FC<ConflictsTabContentProps> = ({
  conflictStatus,
  onStatusChange,
  conflictBatchId,
  onBatchIdChange,
  conflicts,
  conflictPagination,
  conflictsLoading,
  onLoadConflicts,
  onResolveConflict,
  onBatchResolveConflicts,
  onViewRawData,
}) => {
  const currentUser = useCurrentUser();

  // 批量选择状态
  const [selectedConflictIds, setSelectedConflictIds] = useState<React.Key[]>([]);
  const [batchResolveLoading, setBatchResolveLoading] = useState(false);

  // 批量处理冲突
  const handleBatchResolve = async (action: 'KEEP_EXISTING' | 'OVERWRITE') => {
    if (selectedConflictIds.length === 0) {
      message.warning('请先选择要处理的冲突');
      return;
    }

    const conflictIdStrings = selectedConflictIds.map((id) => String(id));

    // 覆盖操作需要二次确认
    if (action === 'OVERWRITE') {
      Modal.confirm({
        title: '批量覆盖确认',
        content: `确定要批量覆盖 ${selectedConflictIds.length} 条冲突吗？此操作将使用新数据替换现有数据，且不可逆。`,
        okText: '确认覆盖',
        okType: 'danger',
        cancelText: '取消',
        onOk: async () => {
          await executeBatchResolve(conflictIdStrings, action);
        },
      });
    } else {
      await executeBatchResolve(conflictIdStrings, action);
    }
  };

  // 执行批量处理
  const executeBatchResolve = async (
    conflictIdStrings: string[],
    action: 'KEEP_EXISTING' | 'OVERWRITE'
  ) => {
    setBatchResolveLoading(true);
    try {
      // 如果父组件提供了onBatchResolveConflicts回调，使用它
      if (onBatchResolveConflicts) {
        await onBatchResolveConflicts(conflictIdStrings, action);
      } else {
        // 否则直接调用API
        const result = await importApi.batchResolveImportConflicts(
          conflictIdStrings,
          action,
          `批量处理 - ${action}`,
          currentUser || 'admin'
        );

        message.success(
          `批量处理完成：成功 ${result.success_count} 条，失败 ${result.fail_count} 条`
        );

        // 如果所有冲突已处理，提示用户
        if (result.all_resolved) {
          Modal.info({
            title: '冲突全部处理完成',
            content: '该批次的所有冲突已处理，可以前往计划工作台查看导入结果。',
          });
        }
      }

      // 清空选择
      setSelectedConflictIds([]);

      // 刷新冲突列表
      await onLoadConflicts();
    } catch (err: any) {
      message.error(err.message || '批量处理失败');
    } finally {
      setBatchResolveLoading(false);
    }
  };

  // rowSelection配置
  const rowSelection: TableRowSelection<ImportConflict> = {
    selectedRowKeys: selectedConflictIds,
    onChange: (newSelectedRowKeys: React.Key[]) => {
      setSelectedConflictIds(newSelectedRowKeys);
    },
    getCheckboxProps: (record: ImportConflict) => ({
      disabled: record.resolved, // 已处理的冲突不可选
    }),
  };

  // 冲突表格列定义
  const conflictColumns: ColumnsType<ImportConflict> = useMemo(
    () => [
      {
        title: '状态',
        dataIndex: 'resolved',
        key: 'resolved',
        width: 90,
        render: (resolved: boolean) => (
          <Tag
            color={resolved ? 'green' : 'red'}
            icon={resolved ? <CheckCircleOutlined /> : <ExclamationCircleOutlined />}
          >
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
              onViewRawData(`冲突原始数据（${record.material_id || record.conflict_id}）`, raw || '{}')
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
              onClick={() => onResolveConflict(record.conflict_id, 'KEEP_EXISTING')}
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
                  onOk: () => onResolveConflict(record.conflict_id, 'OVERWRITE'),
                })
              }
            >
              覆盖
            </Button>
          </Space>
        ),
      },
    ],
    [onResolveConflict, onViewRawData],
  );

  return (
    <Card
      title="导入冲突队列"
      extra={
        <Space>
          <Button
            icon={<ReloadOutlined />}
            onClick={() => onLoadConflicts({ page: 1 })}
            loading={conflictsLoading}
          >
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
                onStatusChange(v);
                onLoadConflicts({ status: v, page: 1 }).catch((e: unknown) => {
                  console.error('加载冲突失败:', getErrorMessage(e));
                });
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
              onChange={(e) => onBatchIdChange(e.target.value)}
              onPressEnter={() => onLoadConflicts({ page: 1 }).catch((e: unknown) => {
                console.error('加载冲突失败:', getErrorMessage(e));
              })}
            />
          </Col>
          <Col xs={24} md={6} style={{ display: 'flex', alignItems: 'end' }}>
            <Button
              type="primary"
              onClick={() => onLoadConflicts({ page: 1 })}
              loading={conflictsLoading}
            >
              查询
            </Button>
          </Col>
        </Row>

        {/* 批量操作工具栏 */}
        {selectedConflictIds.length > 0 && (
          <Row gutter={12}>
            <Col xs={24}>
              <Alert
                message={`已选择 ${selectedConflictIds.length} 条冲突`}
                type="info"
                showIcon
                closable
                onClose={() => setSelectedConflictIds([])}
                action={
                  <Space>
                    <Button
                      size="small"
                      type="primary"
                      icon={<CheckCircleOutlined />}
                      onClick={() => handleBatchResolve('KEEP_EXISTING')}
                      loading={batchResolveLoading}
                    >
                      批量保留
                    </Button>
                    <Button
                      size="small"
                      type="primary"
                      danger
                      icon={<WarningOutlined />}
                      onClick={() => handleBatchResolve('OVERWRITE')}
                      loading={batchResolveLoading}
                    >
                      批量覆盖
                    </Button>
                  </Space>
                }
              />
            </Col>
          </Row>
        )}

        <Table<ImportConflict>
          loading={conflictsLoading}
          columns={conflictColumns}
          dataSource={conflicts}
          rowKey="conflict_id"
          rowSelection={rowSelection}
          pagination={conflictPagination}
          virtual
          onChange={(pagination) => {
            const current = pagination.current ?? 1;
            const pageSize = pagination.pageSize ?? 20;
            onLoadConflicts({ page: current, pageSize }).catch((e: unknown) => {
              console.error('加载冲突失败:', getErrorMessage(e));
            });
          }}
          scroll={{ x: 1200, y: 520 }}
          size="middle"
        />
      </Space>
    </Card>
  );
};

export default ConflictsTabContent;
