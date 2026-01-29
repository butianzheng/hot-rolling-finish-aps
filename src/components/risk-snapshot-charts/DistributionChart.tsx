/**
 * 风险分布图
 */

import React from 'react';
import { Card, Col, Row, Statistic } from 'antd';
import { formatNumber } from '../../utils/formatters';
import type { RiskDaySummary } from './types';
import { riskLevelColors } from './types';

export interface DistributionChartProps {
  riskSnapshots: RiskDaySummary[];
}

export const DistributionChart: React.FC<DistributionChartProps> = ({ riskSnapshots }) => {
  const distribution: Record<string, number> = {};
  riskSnapshots.forEach((snapshot) => {
    distribution[snapshot.risk_level] = (distribution[snapshot.risk_level] || 0) + 1;
  });

  const total = riskSnapshots.length;
  const data = Object.entries(distribution).map(([level, count]) => ({
    level,
    count,
    percentage: formatNumber((count / total) * 100, 1),
    color: riskLevelColors[level] || '#d9d9d9',
  }));

  return (
    <div style={{ padding: '20px' }}>
      <Row gutter={16}>
        {data.map((item, index) => (
          <Col span={6} key={index}>
            <Card
              style={{
                textAlign: 'center',
                borderColor: item.color,
                borderWidth: 2,
              }}
            >
              <Statistic
                title={item.level}
                value={item.count}
                suffix={`天 (${item.percentage}%)`}
                valueStyle={{ color: item.color }}
              />
            </Card>
          </Col>
        ))}
      </Row>
    </div>
  );
};

export default DistributionChart;
