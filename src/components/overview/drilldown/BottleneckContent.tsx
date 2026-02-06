/**
 * 堵塞矩阵内容
 */

import React from 'react';
import { Button, Descriptions, Space, Table, Tag, Typography, theme } from 'antd';
import type { ColumnsType } from 'antd/es/table';
import type { BottleneckPoint } from '../../../types/decision';
import {
  BOTTLENECK_LEVEL_COLORS,
  BOTTLENECK_LEVEL_LABELS,
  getBottleneckTypeColor,
  getBottleneckTypeLabel,
  ReasonTable,
  ActionsList,
  getHighlightStyle,
  type WorkbenchCallback,
} from './shared';

const { Text } = Typography;

export interface BottleneckContentProps {
  bottlenecks: BottleneckPoint[];
  machineCodeFilter?: string | null;
  planDateFilter?: string | null;
  onGoWorkbench?: WorkbenchCallback;
  onViewDetail: (record: BottleneckPoint) => void;
}

export const BottleneckContent: React.FC<BottleneckContentProps> = ({
  bottlenecks,
  machineCodeFilter,
  planDateFilter,
  onGoWorkbench,
  onViewDetail,
}) => {
  const { token } = theme.useToken();

  const rows = [...bottlenecks].sort((a, b) => b.bottleneckScore - a.bottleneckScore);

  const selectedPoint =
    machineCodeFilter && planDateFilter
      ? bottlenecks.find((p) => p.machineCode === machineCodeFilter && p.planDate === planDateFilter) || null
      : null;

  const columns: ColumnsType<BottleneckPoint> = [
    { title: '机组', dataIndex: 'machineCode', key: 'machineCode', width: 90 },
    { title: '日期', dataIndex: 'planDate', key: 'planDate', width: 110 },
    {
      title: '等级',
      dataIndex: 'bottleneckLevel',
      key: 'bottleneckLevel',
      width: 110,
      render: (v: BottleneckPoint['bottleneckLevel']) => (
        <Tag color={BOTTLENECK_LEVEL_COLORS[v] || '#8c8c8c'}>{BOTTLENECK_LEVEL_LABELS[v] || v}</Tag>
      ),
    },
    { title: '分数', dataIndex: 'bottleneckScore', key: 'bottleneckScore', width: 90 },
    {
      title: '利用率',
      dataIndex: 'capacityUtilPct',
      key: 'capacityUtilPct',
      width: 100,
      render: (v: number) => `${Number(v || 0).toFixed(1)}%`,
    },
    { title: '未排数', dataIndex: 'pendingMaterialCount', key: 'pendingMaterialCount', width: 90 },
    { title: '未排(吨)', dataIndex: 'pendingWeightT', key: 'pendingWeightT', width: 100 },
    {
      title: '操作',
      key: 'action',
      width: onGoWorkbench ? 160 : 90,
      render: (_, record) => (
        <Space size={8}>
          {onGoWorkbench ? (
            <Button
              size="small"
              type="primary"
              onClick={() => onGoWorkbench({ workbenchTab: 'capacity', machineCode: record.machineCode })}
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
      {selectedPoint ? (
        <>
          <Space wrap align="center">
            <Tag color={BOTTLENECK_LEVEL_COLORS[selectedPoint.bottleneckLevel] || '#8c8c8c'}>
              {BOTTLENECK_LEVEL_LABELS[selectedPoint.bottleneckLevel] || selectedPoint.bottleneckLevel} / {selectedPoint.bottleneckScore.toFixed(1)}
            </Tag>
            <Text strong>
              {selectedPoint.machineCode} · {selectedPoint.planDate}
            </Text>
            {onGoWorkbench ? (
              <Button
                size="small"
                type="primary"
                onClick={() => onGoWorkbench({ workbenchTab: 'capacity', machineCode: selectedPoint.machineCode })}
              >
                去处理
              </Button>
            ) : null}
          </Space>

          <Descriptions column={4} bordered size="small">
            <Descriptions.Item label="堵塞分数">{selectedPoint.bottleneckScore.toFixed(1)}</Descriptions.Item>
            <Descriptions.Item label="容量利用率">{selectedPoint.capacityUtilPct.toFixed(1)}%</Descriptions.Item>
            <Descriptions.Item label="未排材料数(≤当日)">{selectedPoint.pendingMaterialCount}</Descriptions.Item>
            <Descriptions.Item label="未排重量(≤当日)">{selectedPoint.pendingWeightT.toFixed(3)}t</Descriptions.Item>
          </Descriptions>

          {Array.isArray(selectedPoint.bottleneckTypes) && selectedPoint.bottleneckTypes.length > 0 ? (
            <Space wrap>
              {selectedPoint.bottleneckTypes.map((t) => (
                <Tag key={t} color={getBottleneckTypeColor(t)} style={{ marginInlineEnd: 0 }}>
                  {getBottleneckTypeLabel(t)}
                </Tag>
              ))}
            </Space>
          ) : null}

          <ReasonTable reasons={selectedPoint.reasons || []} emptyText="暂无堵塞原因明细" />

          <ActionsList
            title="推荐行动"
            actions={selectedPoint.recommendedActions || []}
            colorTextSecondary={token.colorTextSecondary}
          />
        </>
      ) : null}

      <Table
        rowKey={(r) => `${r.machineCode}-${r.planDate}-${r.bottleneckLevel}`}
        size="small"
        columns={columns}
        dataSource={rows}
        pagination={{ pageSize: 20 }}
        onRow={(record) => ({
          style: getHighlightStyle(
            !!(
              machineCodeFilter &&
              planDateFilter &&
              record.machineCode === machineCodeFilter &&
              record.planDate === planDateFilter
            ),
            token
          ),
        })}
      />
    </Space>
  );
};

export default BottleneckContent;
