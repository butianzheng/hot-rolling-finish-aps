/**
 * 物料变更明细卡片
 */

import React from 'react';
import { Alert, Button, Card, Input, Select, Space, Table, Tag, Typography } from 'antd';
import type { EChartsOption } from 'echarts';
import type { VersionDiff } from '../../types/comparison';
import type { LocalVersionDiffSummary } from './localTypes';
import { Chart } from './Chart';

const { Text } = Typography;

interface MaterialDiffCardProps {
  loadLocalCompareDetail: boolean;
  planItemsLoading?: boolean;
  planItemsError?: Error | null;
  localDiffResult: {
    diffs: VersionDiff[];
    summary: LocalVersionDiffSummary;
  } | null;
  filteredDiffs: VersionDiff[];
  diffSearchText: string;
  diffTypeFilter: 'ALL' | 'ADDED' | 'REMOVED' | 'MOVED' | 'MODIFIED';
  diffSummaryChartOption?: EChartsOption | null;
  onLoadLocalCompareDetail?: () => void;
  onDiffSearchChange?: (text: string) => void;
  onDiffTypeFilterChange?: (type: 'ALL' | 'ADDED' | 'REMOVED' | 'MOVED' | 'MODIFIED') => void;
  onExportDiffs?: (format: 'csv' | 'json') => Promise<void>;
}

export const MaterialDiffCard: React.FC<MaterialDiffCardProps> = ({
  loadLocalCompareDetail,
  planItemsLoading,
  planItemsError,
  localDiffResult,
  filteredDiffs,
  diffSearchText,
  diffTypeFilter,
  diffSummaryChartOption,
  onLoadLocalCompareDetail,
  onDiffSearchChange,
  onDiffTypeFilterChange,
  onExportDiffs,
}) => {
  return (
    <Card
      title="物料变更明细（本地计算）"
      size="small"
      extra={
        <Space>
          <Button
            size="small"
            onClick={() => onLoadLocalCompareDetail?.()}
            disabled={loadLocalCompareDetail}
          >
            {loadLocalCompareDetail ? '已加载明细' : '加载明细'}
          </Button>
          <Button size="small" onClick={() => onExportDiffs?.('csv')} disabled={!localDiffResult}>
            导出差异(CSV)
          </Button>
          <Button size="small" onClick={() => onExportDiffs?.('json')} disabled={!localDiffResult}>
            导出差异(JSON)
          </Button>
        </Space>
      }
    >
      {!loadLocalCompareDetail ? (
        <Alert
          type="info"
          showIcon
          message="为提升性能，默认不加载全量排产明细"
          description="点击右上角「加载明细」后，将拉取两个版本的 plan_item 用于本地计算：变更明细/产能变化等。"
        />
      ) : planItemsLoading ? (
        <Alert type="info" showIcon message="正在加载排产明细，用于计算差异…" />
      ) : planItemsError ? (
        <Alert
          type="error"
          showIcon
          message="排产明细加载失败，无法生成本地差异"
          description={String((planItemsError as any)?.message || planItemsError)}
        />
      ) : !localDiffResult ? (
        <Alert type="info" showIcon message="暂无差异数据" />
      ) : (
        <Space direction="vertical" style={{ width: '100%' }} size={10}>
          <Space wrap>
            <Tag color="blue">总变更 {localDiffResult.summary.totalChanges}</Tag>
            <Tag color="green">新增 {localDiffResult.summary.addedCount}</Tag>
            <Tag color="red">删除 {localDiffResult.summary.removedCount}</Tag>
            <Tag color="gold">移动 {localDiffResult.summary.movedCount}</Tag>
            <Tag color="purple">修改 {localDiffResult.summary.modifiedCount}</Tag>
          </Space>

          {diffSummaryChartOption ? (
            <Chart option={diffSummaryChartOption} height={180} />
          ) : (
            <Text type="secondary" style={{ fontSize: 12 }}>
              暂无可视化统计
            </Text>
          )}

          <Space wrap>
            <Input
              placeholder="搜索物料/From/To…"
              value={diffSearchText}
              onChange={(e) => onDiffSearchChange?.(e.target.value)}
              style={{ width: 260 }}
              allowClear
            />
            <Select
              value={diffTypeFilter}
              style={{ width: 220 }}
              onChange={(v) => onDiffTypeFilterChange?.(v)}
              options={[
                { value: 'ALL', label: '全部类型' },
                { value: 'MOVED', label: '移动' },
                { value: 'MODIFIED', label: '修改' },
                { value: 'ADDED', label: '新增' },
                { value: 'REMOVED', label: '删除' },
              ]}
            />
            <Text type="secondary" style={{ fontSize: 12 }}>
              显示 {filteredDiffs.length} / {localDiffResult.diffs.length}
            </Text>
          </Space>

          <Table<VersionDiff>
            size="small"
            rowKey={(r) => r.materialId}
            pagination={false}
            dataSource={filteredDiffs}
            columns={[
              {
                title: '类型',
                dataIndex: 'changeType',
                width: 90,
                render: (v) => {
                  const color = v === 'REMOVED' ? 'red' : v === 'ADDED' ? 'green' : v === 'MOVED' ? 'gold' : 'purple';
                  return <Tag color={color}>{v}</Tag>;
                },
              },
              {
                title: '物料',
                dataIndex: 'materialId',
                width: 200,
                render: (v) => (
                  <Text code copyable>
                    {String(v)}
                  </Text>
                ),
              },
              {
                title: 'From',
                key: 'from',
                width: 260,
                render: (_, r) => {
                  const s = r.previousState;
                  return s ? `${s.machine_code}/${s.plan_date}/序${s.seq_no}` : '-';
                },
              },
              {
                title: 'To',
                key: 'to',
                width: 260,
                render: (_, r) => {
                  const s = r.currentState;
                  return s ? `${s.machine_code}/${s.plan_date}/序${s.seq_no}` : '-';
                },
              },
              {
                title: '紧急',
                key: 'urgent',
                width: 90,
                render: (_, r) => {
                  const u = r.currentState?.urgent_level ?? r.previousState?.urgent_level ?? '';
                  return u ? <Tag>{u}</Tag> : '-';
                },
              },
              {
                title: '重量',
                key: 'weight',
                width: 90,
                render: (_, r) => {
                  const w = r.currentState?.weight_t ?? r.previousState?.weight_t ?? null;
                  if (w == null || !Number.isFinite(Number(w))) return '-';
                  return `${Number(w).toFixed(3)}t`;
                },
              },
            ]}
            virtual
            scroll={{ y: 320 }}
          />
        </Space>
      )}
    </Card>
  );
};
