/**
 * Dashboard 统计卡片组件
 */

import React from 'react';
import { Card, Col, Row, Statistic } from 'antd';
import {
  ClockCircleOutlined,
  DatabaseOutlined,
  ThunderboltOutlined,
  WarningOutlined,
} from '@ant-design/icons';
import { formatNumber } from '../../utils/formatters';
import type { BottleneckPointRow, ColdStockBucketRow, OrderFailureRow, OrderFailureSetResponse, ColdStockProfileResponse } from './types';

export interface StatisticsCardsProps {
  orderFailures: OrderFailureRow[];
  orderFailureSummary: OrderFailureSetResponse['summary'];
  coldStockBuckets: ColdStockBucketRow[];
  coldStockSummary: ColdStockProfileResponse['summary'];
  mostCongestedPoint: BottleneckPointRow | null;
  onNavigate: (path: string) => void;
}

export const StatisticsCards: React.FC<StatisticsCardsProps> = ({
  orderFailures,
  orderFailureSummary,
  coldStockBuckets,
  coldStockSummary,
  mostCongestedPoint,
  onNavigate,
}) => {
  return (
    <Row gutter={16} style={{ marginBottom: 24 }}>
      <Col span={6}>
        <Card
          hoverable
          style={{ cursor: 'pointer' }}
          onClick={() => onNavigate('/overview?tab=d2')}
        >
          <Statistic
            title="未满足紧急单"
            value={orderFailureSummary?.totalFailures ?? orderFailures.length}
            prefix={<WarningOutlined />}
            valueStyle={{ color: '#cf1322' }}
          />
        </Card>
      </Col>
      <Col span={6}>
        <Card
          hoverable
          style={{ cursor: 'pointer' }}
          onClick={() => onNavigate('/overview?tab=d3')}
        >
          <Statistic
            title="冷料数量"
            value={coldStockSummary?.totalColdStockCount ?? coldStockBuckets.reduce((sum, b) => sum + (b.count || 0), 0)}
            prefix={<ClockCircleOutlined />}
            valueStyle={{ color: '#faad14' }}
          />
        </Card>
      </Col>
      <Col span={6}>
        <Card hoverable>
          <Statistic
            title="冷料总重(吨)"
            value={formatNumber(
              coldStockSummary?.totalColdStockWeightT ??
                coldStockBuckets.reduce((sum, b) => sum + (b.weightT || 0), 0),
              2
            )}
            prefix={<DatabaseOutlined />}
          />
        </Card>
      </Col>
      <Col span={6}>
        <Card
          hoverable
          style={{ cursor: mostCongestedPoint ? 'pointer' : 'default' }}
          onClick={() => {
            if (!mostCongestedPoint) return;
            const qs = new URLSearchParams({
              machine: mostCongestedPoint.machineCode,
              date: mostCongestedPoint.planDate,
            }).toString();
            onNavigate(`/overview?tab=d4&${qs}`);
          }}
        >
          <Statistic
            title="最拥堵机组"
            value={mostCongestedPoint?.machineCode || '-'}
            prefix={<ThunderboltOutlined />}
            valueStyle={{ color: '#1890ff' }}
          />
        </Card>
      </Col>
    </Row>
  );
};

export default StatisticsCards;
