/**
 * 冷坨高压力内容
 */

import React from 'react';
import { Button, Descriptions, Space, Table, Tag, Typography, theme } from 'antd';
import type { ColumnsType } from 'antd/es/table';
import type { ColdStockBucket, ReasonItem } from '../../../types/decision';
import { getPressureLevelColor, getPressureLevelLabel, ReasonTable, getHighlightStyle, type WorkbenchCallback } from './shared';
import { formatNumber, formatWeight } from '../../../utils/formatters';

const { Text } = Typography;

export interface ColdStockContentProps {
  buckets: ColdStockBucket[];
  machineCodeFilter?: string | null;
  ageBinFilter?: string | null;
  pressureLevelFilter?: string | null;
  onGoWorkbench?: WorkbenchCallback;
  onViewDetail: (record: ColdStockBucket) => void;
}

const levelOrder: Record<ColdStockBucket['pressureLevel'], number> = {
  CRITICAL: 3,
  HIGH: 2,
  MEDIUM: 1,
  LOW: 0,
};

const ageOrder: Record<ColdStockBucket['ageBin'], number> = {
  '30+': 3,
  '15-30': 2,
  '8-14': 1,
  '0-7': 0,
};

export const ColdStockContent: React.FC<ColdStockContentProps> = ({
  buckets,
  machineCodeFilter,
  ageBinFilter,
  pressureLevelFilter,
  onGoWorkbench,
  onViewDetail,
}) => {
  const { token } = theme.useToken();

  const rows = [...buckets].sort((a, b) => {
    const sa = machineCodeFilter && a.machineCode === machineCodeFilter ? 0 : 1;
    const sb = machineCodeFilter && b.machineCode === machineCodeFilter ? 0 : 1;
    if (sa !== sb) return sa - sb;

    const la = levelOrder[a.pressureLevel] ?? 0;
    const lb = levelOrder[b.pressureLevel] ?? 0;
    if (la !== lb) return lb - la;
    if (a.pressureScore !== b.pressureScore) return b.pressureScore - a.pressureScore;
    const aa = ageOrder[a.ageBin] ?? 0;
    const ab = ageOrder[b.ageBin] ?? 0;
    if (aa !== ab) return ab - aa;
    if (a.maxAgeDays !== b.maxAgeDays) return b.maxAgeDays - a.maxAgeDays;
    if (a.machineCode !== b.machineCode) return a.machineCode.localeCompare(b.machineCode);
    return a.ageBin.localeCompare(b.ageBin);
  });

  const hasBucketFilter = !!(ageBinFilter || pressureLevelFilter);
  const selectedBucket = hasBucketFilter
    ? buckets.find((b) => {
        if (machineCodeFilter && b.machineCode !== machineCodeFilter) return false;
        if (ageBinFilter && b.ageBin !== ageBinFilter) return false;
        if (pressureLevelFilter && b.pressureLevel !== pressureLevelFilter) return false;
        return true;
      }) || null
    : null;

  const selectedMachineBuckets = machineCodeFilter
    ? buckets.filter((b) => b.machineCode === machineCodeFilter)
    : [];

  const machineSummary = machineCodeFilter
    ? selectedMachineBuckets.reduce(
        (acc, cur) => {
          acc.count += Number(cur.count || 0);
          acc.weightT += Number(cur.weightT || 0);
          acc.highPressureCount +=
            cur.pressureLevel === 'HIGH' || cur.pressureLevel === 'CRITICAL' ? Number(cur.count || 0) : 0;
          acc.maxPressureScore = Math.max(acc.maxPressureScore, Number(cur.pressureScore || 0));
          acc.maxAgeDays = Math.max(acc.maxAgeDays, Number(cur.maxAgeDays || 0));
          return acc;
        },
        { count: 0, weightT: 0, highPressureCount: 0, maxPressureScore: 0, maxAgeDays: 0 }
      )
    : null;

  const worstBucketForMachine =
    !hasBucketFilter && machineCodeFilter
      ? [...selectedMachineBuckets].sort((a, b) => {
          const la = levelOrder[a.pressureLevel] ?? 0;
          const lb = levelOrder[b.pressureLevel] ?? 0;
          if (la !== lb) return lb - la;
          if (a.pressureScore !== b.pressureScore) return b.pressureScore - a.pressureScore;
          const aa = ageOrder[a.ageBin] ?? 0;
          const ab = ageOrder[b.ageBin] ?? 0;
          if (aa !== ab) return ab - aa;
          return b.maxAgeDays - a.maxAgeDays;
        })[0] || null
      : null;

  const displayedBucket = selectedBucket || worstBucketForMachine;

  const columns: ColumnsType<ColdStockBucket> = [
    { title: '机组', dataIndex: 'machineCode', key: 'machineCode', width: 90 },
    { title: '库龄', dataIndex: 'ageBin', key: 'ageBin', width: 90 },
    {
      title: '压力',
      dataIndex: 'pressureLevel',
      key: 'pressureLevel',
      width: 120,
      render: (v: ColdStockBucket['pressureLevel']) => (
        <Tag color={getPressureLevelColor(v)}>{getPressureLevelLabel(v)}</Tag>
      ),
    },
    {
      title: '分数',
      dataIndex: 'pressureScore',
      key: 'pressureScore',
      width: 90,
      render: (v: number) => (Number.isFinite(v) ? formatNumber(Number(v), 2) : '-'),
    },
    { title: '数量', dataIndex: 'count', key: 'count', width: 80 },
    {
      title: '重量（吨）',
      dataIndex: 'weightT',
      key: 'weightT',
      width: 120,
      render: (v: number) => formatWeight(v),
    },
    {
      title: '平均库龄',
      dataIndex: 'avgAgeDays',
      key: 'avgAgeDays',
      width: 110,
      render: (v: number) => `${formatNumber(v, 2)}天`,
    },
    { title: '最大库龄', dataIndex: 'maxAgeDays', key: 'maxAgeDays', width: 100 },
    {
      title: '结构缺口',
      dataIndex: 'structureGap',
      key: 'structureGap',
      width: 180,
      ellipsis: true,
      render: (v: string) => {
        const s = String(v || '').trim();
        if (!s || s === 'NONE' || s === '无') return '-';
        return s;
      },
    },
    {
      title: '首因',
      dataIndex: 'reasons',
      key: 'reasons',
      width: 260,
      ellipsis: true,
      render: (reasons: ReasonItem[]) =>
        Array.isArray(reasons) && reasons.length > 0 ? String(reasons[0]?.msg || '-') : '-',
    },
    {
      title: '操作',
      key: 'action',
      width: onGoWorkbench ? 160 : 90,
      render: (_: unknown, record: ColdStockBucket) => (
        <Space size={8}>
          {onGoWorkbench ? (
            <Button
              size="small"
              type="primary"
              onClick={() =>
                onGoWorkbench({
                  workbenchTab: 'materials',
                  machineCode: record.machineCode,
                  context: 'coldStock',
                })
              }
            >
              处理
            </Button>
          ) : null}
          <Button size="small" onClick={() => onViewDetail(record)}>
            详情
          </Button>
        </Space>
      ),
    },
  ];

  return (
    <Space direction="vertical" size={12} style={{ width: '100%' }}>
      {machineSummary ? (
        <Space wrap align="center">
          <Tag color="blue">{machineCodeFilter}</Tag>
          <Text type="secondary">
            冷坨 {machineSummary.count} 件 · {formatWeight(machineSummary.weightT)} · 高压{' '}
            {machineSummary.highPressureCount} 件 · 峰值 {formatNumber(machineSummary.maxPressureScore, 2)}
          </Text>
          {onGoWorkbench ? (
            <Button
              size="small"
              type="primary"
              onClick={() =>
                onGoWorkbench({
                  workbenchTab: 'materials',
                  machineCode: machineCodeFilter,
                  context: 'coldStock',
                })
              }
            >
              去处理
            </Button>
          ) : null}
        </Space>
      ) : null}

      {displayedBucket ? (
        <>
          <Space wrap align="center">
            <Tag color={getPressureLevelColor(displayedBucket.pressureLevel)}>
              {getPressureLevelLabel(displayedBucket.pressureLevel)} / {formatNumber(displayedBucket.pressureScore, 2)}
            </Tag>
            <Text strong>
              {displayedBucket.machineCode} · {displayedBucket.ageBin}
            </Text>
            {!hasBucketFilter && machineCodeFilter ? <Tag style={{ marginInlineEnd: 0 }}>最高压力桶</Tag> : null}
          </Space>

          <Descriptions column={4} bordered size="small">
            <Descriptions.Item label="数量">{displayedBucket.count}</Descriptions.Item>
            <Descriptions.Item label="重量">{formatWeight(displayedBucket.weightT)}</Descriptions.Item>
            <Descriptions.Item label="平均库龄">{formatNumber(displayedBucket.avgAgeDays, 2)}天</Descriptions.Item>
            <Descriptions.Item label="最大库龄">{displayedBucket.maxAgeDays}天</Descriptions.Item>
          </Descriptions>

          {String(displayedBucket.structureGap || '').trim() &&
          String(displayedBucket.structureGap || '').trim() !== 'NONE' &&
          String(displayedBucket.structureGap || '').trim() !== '无' ? (
            <div>
              <Text strong>结构缺口</Text>
              <div style={{ marginTop: 6, color: token.colorTextSecondary }}>
                {String(displayedBucket.structureGap || '').trim()}
              </div>
            </div>
          ) : null}

          <ReasonTable reasons={displayedBucket.reasons || []} emptyText="暂无压库原因明细" />
        </>
      ) : null}

      <Table
        rowKey={(r) => `${r.machineCode}-${r.ageBin}-${r.pressureLevel}`}
        size="small"
        columns={columns}
        dataSource={rows}
        pagination={{ pageSize: 20 }}
        scroll={{ x: 1180 }}
        onRow={(record) => ({
          style: getHighlightStyle(
            !!(
              (machineCodeFilter || ageBinFilter || pressureLevelFilter) &&
              (!machineCodeFilter || record.machineCode === machineCodeFilter) &&
              (!ageBinFilter || record.ageBin === ageBinFilter) &&
              (!pressureLevelFilter || record.pressureLevel === pressureLevelFilter)
            ),
            token
          ),
        })}
      />
    </Space>
  );
};

export default ColdStockContent;
