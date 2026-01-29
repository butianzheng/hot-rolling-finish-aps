/**
 * 产能池统计卡片
 */

import React from 'react';
import { Card, Col, Row, Statistic } from 'antd';
import { formatCapacity } from '../../utils/formatters';
import type { TotalStats } from './types';

export interface StatisticsCardsProps {
  stats: TotalStats;
}

export const StatisticsCards: React.FC<StatisticsCardsProps> = ({ stats }) => {
  return (
    <Row gutter={16} style={{ marginBottom: 16 }}>
      <Col span={8}>
        <Card>
          <Statistic
            title="总目标产能"
            value={formatCapacity(stats.totalTarget)}
            suffix="吨"
          />
        </Card>
      </Col>
      <Col span={8}>
        <Card>
          <Statistic
            title="总已用产能"
            value={formatCapacity(stats.totalUsed)}
            suffix="吨"
            valueStyle={{ color: '#1890ff' }}
          />
        </Card>
      </Col>
      <Col span={8}>
        <Card>
          <Statistic
            title="总剩余产能"
            value={formatCapacity(stats.totalAvailable)}
            suffix="吨"
            valueStyle={{
              color: stats.totalAvailable < 500 ? '#cf1322' : '#52c41a',
            }}
          />
        </Card>
      </Col>
    </Row>
  );
};

export default StatisticsCards;
