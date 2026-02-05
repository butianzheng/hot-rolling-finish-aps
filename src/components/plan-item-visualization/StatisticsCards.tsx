/**
 * 统计卡片组件
 */

import React from 'react';
import { Card, Col, Row, Statistic } from 'antd';
import type { Statistics } from './types';

export interface StatisticsCardsProps {
  statistics: Statistics | null;
}

export const StatisticsCards: React.FC<StatisticsCardsProps> = ({ statistics }) => {
  if (!statistics) return null;

  return (
    <Row gutter={16} style={{ marginBottom: 16 }}>
      <Col span={6}>
        <Card>
          <Statistic title="总排产数" value={statistics.total_items} suffix="个" />
        </Card>
      </Col>
      <Col span={6}>
        <Card>
          <Statistic
            title="总吨位"
            value={statistics.total_weight.toFixed(3)}
            suffix="吨"
          />
        </Card>
      </Col>
      <Col span={6}>
        <Card>
          <Statistic
            title="冻结材料"
            value={statistics.frozen_count}
            suffix="个"
            valueStyle={{ color: '#8c8c8c' }}
          />
        </Card>
      </Col>
      <Col span={6}>
        <Card>
          <Statistic
            title="紧急材料(L2+)"
            value={
              (statistics.by_urgent_level['L2'] || 0) +
              (statistics.by_urgent_level['L3'] || 0)
            }
            suffix="个"
            valueStyle={{ color: '#cf1322' }}
          />
        </Card>
      </Col>
    </Row>
  );
};

export default StatisticsCards;
