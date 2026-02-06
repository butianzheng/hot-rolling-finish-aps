// ==========================================
// D4决策：机组堵塞矩阵页面
// ==========================================
// 职责: 展示机组×日期二维堵塞热力图，支持下钻查看堵塞原因
// ==========================================

import React, { useEffect, useState, useMemo } from 'react';
import { Card, Row, Col, Statistic, Table, Tag, Spin, Alert, Space, Select, Descriptions } from 'antd';
import {
  WarningOutlined,
  BuildOutlined,
  InfoCircleOutlined,
} from '@ant-design/icons';
import type { ColumnsType } from 'antd/es/table';
import { useSearchParams } from 'react-router-dom';
import { useRecentDaysBottleneck } from '../../hooks/queries/use-decision-queries';
import type { DrilldownSpec } from '../../hooks/useRiskOverviewData';
import { useActiveVersionId } from '../../stores/use-global-store';
import { BottleneckHeatmap } from '../../components/charts/BottleneckHeatmap';
import { EmptyState } from '../../components/EmptyState';
import type { ReasonItem, BottleneckType } from '../../types/decision';

// ==========================================
// 堵塞类型颜色映射
// ==========================================
const BOTTLENECK_TYPE_COLORS: Record<BottleneckType, string> = {
  Capacity: '#ff4d4f',
  Structure: '#faad14',
  RollChange: '#1677ff',
  ColdStock: '#52c41a',
  Mixed: '#722ed1',
};

const BOTTLENECK_LEVEL_COLORS = {
  NONE: '#d9d9d9',
  LOW: '#52c41a',
  MEDIUM: '#1677ff',
  HIGH: '#faad14',
  CRITICAL: '#ff4d4f',
} as const;

// ==========================================
// 主组件
// ==========================================

interface D4BottleneckProps {
  embedded?: boolean;
  onOpenDrilldown?: (spec: DrilldownSpec) => void;
}

export const D4Bottleneck: React.FC<D4BottleneckProps> = ({ embedded, onOpenDrilldown }) => {
  const versionId = useActiveVersionId();
  const [searchParams] = useSearchParams();
  const [selectedDays, setSelectedDays] = useState<number>(30);
  const effectiveDays = embedded ? 30 : selectedDays;
  const [selectedPoint, setSelectedPoint] = useState<{
    machine: string;
    date: string;
  } | null>(null);

  // 获取堵塞数据
  const { data, isLoading, error } = useRecentDaysBottleneck(versionId, effectiveDays);

  // 支持 Dashboard drill-down：/decision/d4-bottleneck?machine=H031&date=YYYY-MM-DD
  useEffect(() => {
    if (embedded) return;
    const machine = searchParams.get('machine');
    const date = searchParams.get('date');
    if (machine && date) {
      setSelectedPoint({ machine, date });
    }
  }, [embedded, searchParams]);

  // 计算统计数据
  const stats = useMemo(() => {
    if (!data?.items || data.items.length === 0) {
      return {
        avgBottleneckScore: 0,
        highBottleneckCount: 0,
        criticalBottleneckCount: 0,
        affectedMachineCount: 0,
      };
    }

    const avgBottleneckScore =
      data.items.reduce((sum, item) => sum + item.bottleneckScore, 0) / data.items.length;
    const highBottleneckCount = data.items.filter(
      (item) => item.bottleneckLevel === 'HIGH' || item.bottleneckLevel === 'CRITICAL'
    ).length;
    const criticalBottleneckCount = data.items.filter(
      (item) => item.bottleneckLevel === 'CRITICAL'
    ).length;

    // 统计涉及的机组数
    const machines = new Set(data.items.map((item) => item.machineCode));
    const affectedMachineCount = machines.size;

    return {
      avgBottleneckScore: Math.round(avgBottleneckScore * 10) / 10,
      highBottleneckCount,
      criticalBottleneckCount,
      affectedMachineCount,
    };
  }, [data]);

  // 选中点的详细信息
  const selectedPointData = useMemo(() => {
    if (!selectedPoint || !data?.items) return null;
    return (
      data.items.find(
        (item) =>
          item.machineCode === selectedPoint.machine && item.planDate === selectedPoint.date
      ) || null
    );
  }, [selectedPoint, data]);

  // 热力图点击处理
  const handlePointClick = (machine: string, date: string) => {
    setSelectedPoint({ machine, date });
    if (embedded && onOpenDrilldown) {
      onOpenDrilldown({ kind: 'bottleneck', machineCode: machine, planDate: date });
    }
  };

  // ==========================================
  // 加载状态
  // ==========================================

  if (isLoading) {
    return (
      <div style={{ textAlign: 'center', padding: embedded ? '40px 0' : '100px 0' }}>
        <Spin size="large" tip="正在加载堵塞数据...">
          <div style={{ minHeight: 80 }} />
        </Spin>
      </div>
    );
  }

  // ==========================================
  // 错误状态
  // ==========================================

  if (error) {
    return (
      <Alert
        message="数据加载失败"
        description={error.message || '未知错误'}
        type="error"
        showIcon
        style={{ margin: embedded ? 0 : '20px' }}
      />
    );
  }

  if (!versionId) {
    return (
      <Alert
        message="未选择排产版本"
        description="请先在主界面选择一个排产版本"
        type="warning"
        showIcon
        style={{ margin: embedded ? 0 : '20px' }}
      />
    );
  }

  // ==========================================
  // 主界面
  // ==========================================

  return (
    <div style={{ padding: embedded ? 0 : 24 }}>
      {!embedded ? (
        <div style={{ marginBottom: 24 }}>
          <h2>
            <BuildOutlined style={{ marginRight: 8 }} />
            D4决策：机组堵塞矩阵
          </h2>
          <p style={{ color: '#8c8c8c', marginBottom: 16 }}>
            展示未来{selectedDays}天各机组的堵塞/提醒情况（仅 HIGH/CRITICAL 视为堵塞）
          </p>

          <Space>
            <span>查看范围：</span>
            <Select
              value={selectedDays}
              onChange={setSelectedDays}
              style={{ width: 120 }}
              options={[
                { label: '7天', value: 7 },
                { label: '14天', value: 14 },
                { label: '30天', value: 30 },
                { label: '60天', value: 60 },
              ]}
            />
          </Space>
        </div>
      ) : null}

      {/* 统计卡片 */}
      <Row gutter={embedded ? 12 : 16} style={{ marginBottom: embedded ? 12 : 24 }}>
        <Col span={6}>
          <Card size={embedded ? 'small' : undefined}>
            <Statistic
              title="平均堵塞分数"
              value={stats.avgBottleneckScore}
              precision={1}
              suffix="/ 100"
              valueStyle={{
                color: stats.avgBottleneckScore > 60 ? '#ff4d4f' : '#52c41a',
              }}
            />
          </Card>
        </Col>
        <Col span={6}>
          <Card size={embedded ? 'small' : undefined}>
            <Statistic
              title="堵塞点位(HIGH/CRITICAL)"
              value={stats.highBottleneckCount}
              suffix={`/ ${data?.items.length || 0}`}
              prefix={<WarningOutlined />}
              valueStyle={{
                color: stats.highBottleneckCount > 0 ? '#faad14' : '#52c41a',
              }}
            />
          </Card>
        </Col>
        <Col span={6}>
          <Card size={embedded ? 'small' : undefined}>
            <Statistic
              title="严重堵塞点位"
              value={stats.criticalBottleneckCount}
              prefix={<WarningOutlined />}
              valueStyle={{
                color: stats.criticalBottleneckCount > 0 ? '#ff4d4f' : '#52c41a',
              }}
            />
          </Card>
        </Col>
        <Col span={6}>
          <Card size={embedded ? 'small' : undefined}>
            <Statistic
              title="涉及机组数"
              value={stats.affectedMachineCount}
              prefix={<BuildOutlined />}
              valueStyle={{ fontSize: '24px' }}
            />
          </Card>
        </Col>
      </Row>

      {/* 堵塞矩阵热力图 */}
      <Card
        title="机组堵塞矩阵（机组×日期）"
        size={embedded ? 'small' : undefined}
        style={{ marginBottom: embedded ? 12 : 24 }}
        extra={
          <Space>
            <InfoCircleOutlined />
            <span style={{ fontSize: 12, color: '#8c8c8c' }}>
              LOW/MEDIUM 为提醒，点击单元格查看详细原因
            </span>
          </Space>
        }
      >
        {data && data.items.length > 0 ? (
          <BottleneckHeatmap
            data={data.items}
            onPointClick={handlePointClick}
            selectedPoint={selectedPoint}
          />
        ) : (
          <EmptyState type="data" style={{ padding: '40px 0' }} />
        )}
      </Card>

      {/* 选中点位的详细信息 */}
      {!embedded && selectedPointData && (
        <Card
          title={`${selectedPointData.machineCode} - ${selectedPointData.planDate} 堵塞详情`}
          size={embedded ? 'small' : undefined}
          extra={
            <Tag color={BOTTLENECK_LEVEL_COLORS[selectedPointData.bottleneckLevel]}>
              {selectedPointData.bottleneckLevel} - {selectedPointData.bottleneckScore.toFixed(2)}
            </Tag>
          }
        >
          {/* 基础信息 */}
          <Descriptions column={4} bordered size="small" style={{ marginBottom: '16px' }}>
            <Descriptions.Item label="堵塞分数">
              {selectedPointData.bottleneckScore.toFixed(2)}
            </Descriptions.Item>
            <Descriptions.Item label="容量利用率">
              {selectedPointData.capacityUtilPct.toFixed(2)}%
            </Descriptions.Item>
            <Descriptions.Item label="已排材料数">
              {selectedPointData.scheduledMaterialCount ?? 0}
            </Descriptions.Item>
            <Descriptions.Item label="已排重量">
              {(selectedPointData.scheduledWeightT ?? 0).toFixed(3)} 吨
            </Descriptions.Item>
            <Descriptions.Item label="未排材料数(≤当日)">
              {selectedPointData.pendingMaterialCount}
            </Descriptions.Item>
            <Descriptions.Item label="未排重量(≤当日)">
              {selectedPointData.pendingWeightT.toFixed(3)} 吨
            </Descriptions.Item>
          </Descriptions>

          {/* 堵塞类型 */}
          <h4 style={{ marginTop: '16px', marginBottom: '8px' }}>堵塞类型</h4>
          <div style={{ marginBottom: '16px' }}>
            {selectedPointData.bottleneckTypes.map((type) => (
              <Tag key={type} color={BOTTLENECK_TYPE_COLORS[type]} style={{ marginBottom: '8px' }}>
                {getBottleneckTypeLabel(type)}
              </Tag>
            ))}
          </div>

          {/* 堵塞原因 */}
          <h4 style={{ marginTop: '16px', marginBottom: '8px' }}>堵塞原因</h4>
          <ReasonsTable reasons={selectedPointData.reasons} />

          {/* 推荐行动 */}
          {selectedPointData.recommendedActions &&
            selectedPointData.recommendedActions.length > 0 && (
              <>
                <h4 style={{ marginTop: '16px', marginBottom: '8px' }}>推荐行动</h4>
                <ul>
                  {selectedPointData.recommendedActions.map((action, index) => (
                    <li key={index} style={{ marginBottom: '4px' }}>
                      {action}
                    </li>
                  ))}
                </ul>
              </>
            )}
        </Card>
      )}
    </div>
  );
};

// ==========================================
// 原因表格组件
// ==========================================

interface ReasonsTableProps {
  reasons: ReasonItem[];
}

const ReasonsTable: React.FC<ReasonsTableProps> = ({ reasons }) => {
  const columns: ColumnsType<ReasonItem> = [
    {
      title: '原因代码',
      dataIndex: 'code',
      key: 'code',
      width: 150,
      render: (code: string) => <Tag>{code}</Tag>,
    },
    {
      title: '原因描述',
      dataIndex: 'msg',
      key: 'msg',
      ellipsis: true,
    },
    {
      title: '权重',
      dataIndex: 'weight',
      key: 'weight',
      width: 100,
      render: (weight: number) => <span>{(weight * 100).toFixed(1)}%</span>,
      sorter: (a, b) => a.weight - b.weight,
      defaultSortOrder: 'descend',
    },
    {
      title: '影响订单数',
      dataIndex: 'affectedCount',
      key: 'affectedCount',
      width: 120,
      render: (count?: number) => count || '-',
    },
  ];

  return (
    <Table
      columns={columns}
      dataSource={reasons}
      rowKey="code"
      size="small"
      pagination={false}
    />
  );
};

// ==========================================
// 工具函数
// ==========================================

function getBottleneckTypeLabel(type: BottleneckType): string {
  const labels: Record<BottleneckType, string> = {
    Capacity: '容量堵塞',
    Structure: '结构堵塞',
    RollChange: '换辊堵塞',
    ColdStock: '冷料堵塞',
    Mixed: '混合堵塞',
  };
  return labels[type];
}

// ==========================================
// 默认导出（用于React.lazy）
// ==========================================
export default D4Bottleneck;
