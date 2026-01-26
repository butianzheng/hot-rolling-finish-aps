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
import type { DaySummary, ReasonItem } from '../../types/decision';
import { RISK_LEVEL_COLORS, isHighRiskDay } from '../../types/decision/d1-day-summary';

// ==========================================
// 主组件
// ==========================================

export const D1RiskHeatmap: React.FC = () => {
  const versionId = useActiveVersionId();
  const [selectedDays, setSelectedDays] = useState<number>(30);
  const [selectedDate, setSelectedDate] = useState<string | null>(null);

  // 获取风险数据
  const { data, isLoading, error } = useRecentDaysRisk(versionId, selectedDays);

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
  };

  // ==========================================
  // 加载状态
  // ==========================================

  if (isLoading) {
    return (
      <div style={{ textAlign: 'center', padding: '100px 0' }}>
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
        style={{ margin: '20px' }}
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
        style={{ margin: '20px' }}
      />
    );
  }

  // ==========================================
  // 主界面
  // ==========================================

  return (
    <div style={{ padding: '24px' }}>
      {/* 页面标题 */}
      <div style={{ marginBottom: '24px' }}>
        <h2>
          <CalendarOutlined style={{ marginRight: '8px' }} />
          D1决策：风险热力图
        </h2>
        <p style={{ color: '#8c8c8c', marginBottom: '16px' }}>
          展示未来{selectedDays}天的排产风险趋势，点击日期查看详细原因
        </p>

        {/* 天数选择器 */}
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

      {/* 统计卡片 */}
      <Row gutter={16} style={{ marginBottom: '24px' }}>
        <Col span={6}>
          <Card>
            <Statistic
              title="平均风险分数"
              value={stats.avgRiskScore}
              precision={1}
              suffix="/ 100"
              valueStyle={{
                color: stats.avgRiskScore > 60 ? '#ff4d4f' : '#52c41a',
              }}
            />
          </Card>
        </Col>
        <Col span={6}>
          <Card>
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
          <Card>
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
          <Card>
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
        style={{ marginBottom: '24px' }}
        extra={
          <Space>
            <InfoCircleOutlined />
            <span style={{ fontSize: '12px', color: '#8c8c8c' }}>
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
          <div style={{ textAlign: 'center', padding: '60px 0', color: '#8c8c8c' }}>
            暂无数据
          </div>
        )}
      </Card>

      {/* 选中日期的详细信息 */}
      {selectedDayData && (
        <Card
          title={`${selectedDayData.planDate} 风险详情`}
          extra={
            <Tag color={RISK_LEVEL_COLORS[selectedDayData.riskLevel]}>
              {selectedDayData.riskLevel} - {selectedDayData.riskScore.toFixed(1)}
            </Tag>
          }
        >
          <Row gutter={16} style={{ marginBottom: '16px' }}>
            <Col span={6}>
              <Statistic
                title="容量利用率"
                value={selectedDayData.capacityUtilPct}
                precision={1}
                suffix="%"
              />
            </Col>
            <Col span={6}>
              <Statistic
                title="超载重量"
                value={selectedDayData.overloadWeightT}
                precision={1}
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
      render: (weight: number) => (
        <span>{(weight * 100).toFixed(1)}%</span>
      ),
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
// 默认导出（用于React.lazy）
// ==========================================
export default D1RiskHeatmap;
