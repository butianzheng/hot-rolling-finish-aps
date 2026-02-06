// ==========================================
// D1决策：风险热力图页面
// ==========================================
// 职责: 展示30天风险趋势热力图，支持下钻查看风险原因
// ==========================================

import React, { useState, useMemo } from 'react';
import { Card, Row, Col, Statistic, Table, Tag, Spin, Alert, Space, Select } from 'antd';
import {
  WarningOutlined,
  ThunderboltOutlined,
  CalendarOutlined,
  InfoCircleOutlined,
} from '@ant-design/icons';
import type { ColumnsType } from 'antd/es/table';
import { useRecentDaysRisk } from '../../hooks/queries/use-decision-queries';
import { useActiveVersionId } from '../../stores/use-global-store';
import { RiskCalendarHeatmap } from '../../components/charts/RiskCalendarHeatmap';
import { EmptyState } from '../../components/EmptyState';
import type { DrilldownSpec } from '../../hooks/useRiskOverviewData';
import type { DaySummary, ReasonItem } from '../../types/decision';
import { RISK_LEVEL_COLORS, RISK_LEVEL_LABELS, isHighRiskDay } from '../../types/decision/d1-day-summary';

// ==========================================
// 主组件
// ==========================================

interface D1RiskHeatmapProps {
  embedded?: boolean;
  onOpenDrilldown?: (spec: DrilldownSpec) => void;
}

export const D1RiskHeatmap: React.FC<D1RiskHeatmapProps> = ({ embedded, onOpenDrilldown }) => {
  const versionId = useActiveVersionId();
  const [selectedDays, setSelectedDays] = useState<number>(30);
  const [selectedDate, setSelectedDate] = useState<string | null>(null);
  const effectiveDays = embedded ? 30 : selectedDays;

  // 获取风险数据
  const { data, isLoading, error } = useRecentDaysRisk(versionId, effectiveDays);

  // 计算统计数据
  const stats = useMemo(() => {
    if (!data?.items || data.items.length === 0) {
      return {
        avgRiskScore: 0,
        highRiskDays: 0,
        maxRiskDay: null as DaySummary | null,
        totalUrgentFailures: 0,
      };
    }

    const avgRiskScore =
      data.items.reduce((sum, item) => sum + item.riskScore, 0) / data.items.length;
    const highRiskDays = data.items.filter(isHighRiskDay).length;
    const maxRiskDay = data.items.reduce((max, item) =>
      item.riskScore > max.riskScore ? item : max
    );
    const totalUrgentFailures = data.items.reduce(
      (sum, item) => sum + item.urgentFailureCount,
      0
    );

    return {
      avgRiskScore: Math.round(avgRiskScore * 10) / 10,
      highRiskDays,
      maxRiskDay,
      totalUrgentFailures,
    };
  }, [data]);

  // 选中日期的详细信息
  const selectedDayData = useMemo(() => {
    if (!selectedDate || !data?.items) return null;
    return data.items.find((item) => item.planDate === selectedDate) || null;
  }, [selectedDate, data]);

  // 热力图点击处理
  const handleDateClick = (date: string) => {
    setSelectedDate(date);
    if (embedded && onOpenDrilldown) {
      onOpenDrilldown({ kind: 'risk', planDate: date });
    }
  };

  // ==========================================
  // 加载状态
  // ==========================================

  if (isLoading) {
    return (
      <div style={{ textAlign: 'center', padding: embedded ? '40px 0' : '100px 0' }}>
        <Spin size="large" tip="正在加载风险数据...">
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
            <CalendarOutlined style={{ marginRight: 8 }} />
            D1决策：风险热力图
          </h2>
          <p style={{ color: '#8c8c8c', marginBottom: 16 }}>
            展示未来{selectedDays}天的排产风险趋势，点击日期查看详细原因
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
              title="平均风险分数"
              value={stats.avgRiskScore}
              precision={2}
              suffix="/ 100"
              valueStyle={{
                color: stats.avgRiskScore > 60 ? '#ff4d4f' : '#52c41a',
              }}
            />
          </Card>
        </Col>
        <Col span={6}>
          <Card size={embedded ? 'small' : undefined}>
            <Statistic
              title="高风险天数"
              value={stats.highRiskDays}
              suffix={`/ ${data?.items.length || 0}`}
              prefix={<WarningOutlined />}
              valueStyle={{
                color: stats.highRiskDays > 0 ? '#faad14' : '#52c41a',
              }}
            />
          </Card>
        </Col>
        <Col span={6}>
          <Card size={embedded ? 'small' : undefined}>
            <Statistic
              title="最高风险日"
              value={stats.maxRiskDay?.planDate || 'N/A'}
              valueStyle={{ fontSize: '20px' }}
              suffix={
                stats.maxRiskDay && (
                  <Tag color={RISK_LEVEL_COLORS[stats.maxRiskDay.riskLevel]}>
                    {stats.maxRiskDay.riskScore.toFixed(0)}
                  </Tag>
                )
              }
            />
          </Card>
        </Col>
        <Col span={6}>
          <Card size={embedded ? 'small' : undefined}>
            <Statistic
              title="紧急订单失败总数"
              value={stats.totalUrgentFailures}
              prefix={<ThunderboltOutlined />}
              valueStyle={{
                color: stats.totalUrgentFailures > 0 ? '#ff4d4f' : '#52c41a',
              }}
            />
          </Card>
        </Col>
      </Row>

      {/* 热力图 */}
      <Card
        title="风险热力图"
        size={embedded ? 'small' : undefined}
        style={{ marginBottom: embedded ? 12 : 24 }}
        extra={
          <Space>
            <InfoCircleOutlined />
            <span style={{ fontSize: 12, color: '#8c8c8c' }}>
              点击日期查看详细原因
            </span>
          </Space>
        }
      >
        {data && data.items.length > 0 ? (
          <RiskCalendarHeatmap
            data={data.items}
            onDateClick={handleDateClick}
            selectedDate={selectedDate}
          />
        ) : (
          <EmptyState type="date" style={{ padding: '40px 0' }} />
        )}
      </Card>

      {/* 选中日期的详细信息 */}
      {!embedded && selectedDayData && (
        <Card
          title={`${selectedDayData.planDate} 风险详情`}
          size={embedded ? 'small' : undefined}
          extra={
            <Tag color={RISK_LEVEL_COLORS[selectedDayData.riskLevel]}>
              {RISK_LEVEL_LABELS[selectedDayData.riskLevel] || selectedDayData.riskLevel} - {selectedDayData.riskScore.toFixed(2)}
            </Tag>
          }
        >
          <Row gutter={16} style={{ marginBottom: '16px' }}>
            <Col span={6}>
              <Statistic
                title="容量利用率"
                value={selectedDayData.capacityUtilPct}
                precision={2}
                suffix="%"
              />
            </Col>
            <Col span={6}>
              <Statistic
                title="超载重量"
                value={selectedDayData.overloadWeightT}
                precision={3}
                suffix="吨"
              />
            </Col>
            <Col span={6}>
              <Statistic
                title="紧急订单失败"
                value={selectedDayData.urgentFailureCount}
                suffix="个"
              />
            </Col>
            <Col span={6}>
              <Statistic
                title="涉及机组"
                value={selectedDayData.involvedMachines.length}
                suffix="台"
              />
            </Col>
          </Row>

          <h4 style={{ marginTop: '16px', marginBottom: '8px' }}>主要风险原因</h4>
          <ReasonsTable reasons={selectedDayData.topReasons} />

          {selectedDayData.involvedMachines.length > 0 && (
            <>
              <h4 style={{ marginTop: '16px', marginBottom: '8px' }}>涉及机组</h4>
              <div>
                {selectedDayData.involvedMachines.map((machine) => (
                  <Tag key={machine} color="blue" style={{ marginBottom: '8px' }}>
                    {machine}
                  </Tag>
                ))}
              </div>
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

/**
 * 原因代码中文翻译映射
 */
const REASON_CODE_LABELS: Record<string, string> = {
  CAPACITY_UTILIZATION: '产能利用率',
  LOW_REMAINING_CAPACITY: '剩余产能不足',
  HIGH_CAPACITY_PRESSURE: '产能压力高',
  STRUCTURE_GAP: '结构性缺口',
  COLD_STOCK_AGING: '冷料库龄',
  ROLL_CHANGE_CONFLICT: '换辊冲突',
  URGENCY_BACKLOG: '紧急订单积压',
  MATURITY_CONSTRAINT: '适温约束',
  OVERLOAD_RISK: '超载风险',
  SCHEDULING_CONFLICT: '排产冲突',
};

function getReasonCodeLabel(code: string): string {
  return REASON_CODE_LABELS[code] || code;
}

const ReasonsTable: React.FC<ReasonsTableProps> = ({ reasons }) => {
  const columns: ColumnsType<ReasonItem> = [
    {
      title: '原因代码',
      dataIndex: 'code',
      key: 'code',
      width: 140,
      render: (code: string) => (
        <Tag color="blue" style={{ maxWidth: '130px', overflow: 'hidden', textOverflow: 'ellipsis' }}>
          {getReasonCodeLabel(code)}
        </Tag>
      ),
    },
    {
      title: '原因描述',
      dataIndex: 'msg',
      key: 'msg',
      ellipsis: { showTitle: true },
      width: 320,
    },
    {
      title: '权重',
      dataIndex: 'weight',
      key: 'weight',
      width: 100,
      render: (weight: number) => (
        <span>{(weight * 100).toFixed(2)}%</span>
      ),
      sorter: (a, b) => a.weight - b.weight,
      defaultSortOrder: 'descend',
    },
    {
      title: '影响订单数',
      dataIndex: 'affectedCount',
      key: 'affectedCount',
      width: 90,
      render: (count?: number) => (typeof count === 'number' ? count : '-'),
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
// 默认导出（用于React.lazy）
// ==========================================
export default D1RiskHeatmap;
