/**
 * 全局产能统计卡片
 * 职责：展示所有选中机组在选定日期范围内的总体产能统计
 */

import React from 'react';
import { Card, Col, Row, Statistic, Spin } from 'antd';
import { formatNumber } from '../../utils/formatters';

export interface GlobalStats {
  totalTarget: number;
  totalUsed: number;
  totalRemaining: number;
  avgUtilization: number;
  overLimitCount: number;
}

export interface GlobalStatisticsCardsProps {
  stats: GlobalStats;
  loading: boolean;
}

export const GlobalStatisticsCards: React.FC<GlobalStatisticsCardsProps> = ({
  stats,
  loading,
}) => {
  if (loading) {
    return (
      <Card size="small" bodyStyle={{ padding: 24 }}>
        <div style={{ textAlign: 'center' }}>
          <Spin tip="加载统计数据..." />
        </div>
      </Card>
    );
  }

  return (
    <Row gutter={12}>
      <Col span={6}>
        <Card size="small" bodyStyle={{ padding: '12px 16px' }}>
          <Statistic
            title="总目标产能"
            value={formatNumber(stats.totalTarget, 3)}
            suffix="吨"
            valueStyle={{ fontSize: 20, fontWeight: 600 }}
          />
        </Card>
      </Col>
      <Col span={6}>
        <Card size="small" bodyStyle={{ padding: '12px 16px' }}>
          <Statistic
            title="总已用产能"
            value={formatNumber(stats.totalUsed, 3)}
            suffix="吨"
            valueStyle={{ color: '#1890ff', fontSize: 20, fontWeight: 600 }}
          />
        </Card>
      </Col>
      <Col span={6}>
        <Card size="small" bodyStyle={{ padding: '12px 16px' }}>
          <Statistic
            title="总剩余产能"
            value={formatNumber(stats.totalRemaining, 3)}
            suffix="吨"
            valueStyle={{
              color: stats.totalRemaining < 500 ? '#cf1322' : '#52c41a',
              fontSize: 20,
              fontWeight: 600,
            }}
          />
        </Card>
      </Col>
      <Col span={6}>
        <Card size="small" bodyStyle={{ padding: '12px 16px' }}>
          <Statistic
            title="平均利用率"
            value={formatNumber(stats.avgUtilization * 100, 2)}
            suffix="%"
            valueStyle={{
              color:
                stats.avgUtilization > 1
                  ? '#cf1322'
                  : stats.avgUtilization > 0.85
                  ? '#faad14'
                  : '#52c41a',
              fontSize: 20,
              fontWeight: 600,
            }}
          />
          {stats.overLimitCount > 0 && (
            <div style={{ fontSize: 12, color: '#cf1322', marginTop: 4 }}>
              超限天数: {stats.overLimitCount}
            </div>
          )}
        </Card>
      </Col>
    </Row>
  );
};

export default GlobalStatisticsCards;
