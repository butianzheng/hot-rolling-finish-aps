/**
 * 产能变化卡片
 */

import React from 'react';
import { Alert, Button, Card, Space, Table, Tag, Typography } from 'antd';
import type { EChartsOption } from 'echarts';
import type { LocalCapacityDeltaRow } from './localTypes';
import { Chart } from './Chart';
import { formatNumber } from '../../utils/formatters';

const { Text } = Typography;

interface CapacityDeltaCardProps {
  loadLocalCompareDetail: boolean;
  localCapacityRows: {
    rows: LocalCapacityDeltaRow[];
    overflowRows: LocalCapacityDeltaRow[];
    totalA: number;
    totalB: number;
    dateFrom: string | null;
    dateTo: string | null;
    machines: string[];
  } | null;
  localCapacityRowsBase: {
    rows: LocalCapacityDeltaRow[];
    totalA: number;
    totalB: number;
    dateFrom: string | null;
    dateTo: string | null;
    machines: string[];
  } | null;
  capacityPoolsError?: Error | null;
  showAllCapacityRows: boolean;
  capacityTrendOption?: EChartsOption | null;
  onToggleShowAllCapacityRows?: () => void;
  onExportCapacity?: (format: 'csv' | 'json') => Promise<void>;
}

export const CapacityDeltaCard: React.FC<CapacityDeltaCardProps> = ({
  loadLocalCompareDetail,
  localCapacityRows,
  localCapacityRowsBase,
  capacityPoolsError,
  showAllCapacityRows,
  capacityTrendOption,
  onToggleShowAllCapacityRows,
  onExportCapacity,
}) => {
  return (
    <Card
      title="产能变化（后端优先，缺失时本地补算）"
      size="small"
      extra={
        <Space>
          <Button
            size="small"
            onClick={() => onToggleShowAllCapacityRows?.()}
            disabled={!localCapacityRowsBase}
          >
            {showAllCapacityRows ? '仅看变化' : '查看全量'}
          </Button>
          <Button size="small" onClick={() => onExportCapacity?.('csv')} disabled={!localCapacityRows}>
            导出产能（表格）
          </Button>
          <Button size="small" onClick={() => onExportCapacity?.('json')} disabled={!localCapacityRows}>
            导出产能（数据）
          </Button>
        </Space>
      }
    >
      {!loadLocalCompareDetail ? (
        <Alert
          type="info"
          showIcon
          message="未加载排产明细"
          description="点击上方「物料变更明细」区域右上角的「加载明细」，即可生成本地补算产能变化（用于后端缺失时兜底）。"
        />
      ) : !localCapacityRowsBase ? (
        <Alert type="info" showIcon message="暂无产能差异数据" />
      ) : (
        <Space direction="vertical" style={{ width: '100%' }} size={10}>
          <Text type="secondary" style={{ fontSize: 12 }}>
            说明：该卡片为本地补算口径，若“风险/产能变化”卡片已返回后端结果，请以后端口径为准。
          </Text>
          {capacityTrendOption ? (
            <Chart option={capacityTrendOption} height={220} />
          ) : (
            <Text type="secondary" style={{ fontSize: 12 }}>
              暂无产能趋势图
            </Text>
          )}

          <Space wrap>
            <Tag color="blue">总量甲 {formatNumber(localCapacityRowsBase.totalA, 3)}吨</Tag>
            <Tag color="blue">总量乙 {formatNumber(localCapacityRowsBase.totalB, 3)}吨</Tag>
            <Tag
              color={
                localCapacityRowsBase.totalB - localCapacityRowsBase.totalA >= 0 ? 'green' : 'red'
              }
            >
              变化值 {formatNumber(localCapacityRowsBase.totalB - localCapacityRowsBase.totalA, 3)}吨
            </Tag>
            {localCapacityRows ? (
              <Tag color={localCapacityRows.overflowRows.length > 0 ? 'red' : 'green'}>
                预计超上限 {localCapacityRows.overflowRows.length}
              </Tag>
            ) : null}
            <Text type="secondary" style={{ fontSize: 12 }}>
              {localCapacityRowsBase.dateFrom || '-'} ~ {localCapacityRowsBase.dateTo || '-'} · 机组{' '}
              {localCapacityRowsBase.machines.length}
            </Text>
          </Space>

          {capacityPoolsError ? (
            <Alert
              type="warning"
              showIcon
              message="产能池加载失败（仍可查看吨位差异）"
              description={String((capacityPoolsError as any)?.message || capacityPoolsError)}
            />
          ) : null}

          <Table<LocalCapacityDeltaRow>
            size="small"
            rowKey={(r) => `${r.machine_code}__${r.date}`}
            pagination={false}
            dataSource={localCapacityRows?.rows ?? localCapacityRowsBase.rows}
            columns={[
              { title: '日期', dataIndex: 'date', width: 120 },
              { title: '机组', dataIndex: 'machine_code', width: 90 },
              {
                title: '版本甲已用（吨）',
                dataIndex: 'used_a',
                width: 110,
                render: (v: number) => formatNumber(v, 3, { useGrouping: false }),
              },
              {
                title: '版本乙已用（吨）',
                dataIndex: 'used_b',
                width: 110,
                render: (v: number, r) => {
                  const threshold = r.limit_b ?? r.target_b ?? null;
                  const over = threshold != null && v > threshold + 1e-9;
                  return <span style={{ color: over ? '#cf1322' : undefined }}>{formatNumber(v, 3, { useGrouping: false })}</span>;
                },
              },
              {
                title: '变化值（吨）',
                dataIndex: 'delta',
                width: 110,
                render: (v: number) => (
                  <span style={{ color: v > 1e-9 ? '#3f8600' : v < -1e-9 ? '#cf1322' : undefined }}>
                    {formatNumber(v, 3, { useGrouping: false })}
                  </span>
                ),
              },
              {
                title: '版本乙目标/上限',
                key: 'capB',
                width: 160,
                render: (_, r) => {
                  const target = r.target_b;
                  const limit = r.limit_b;
                  if (target == null && limit == null) return '-';
                  const t = target == null ? '-' : formatNumber(target, 3, { useGrouping: false });
                  const l = limit == null ? '-' : formatNumber(limit, 3, { useGrouping: false });
                  return `${t} / ${l}`;
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
