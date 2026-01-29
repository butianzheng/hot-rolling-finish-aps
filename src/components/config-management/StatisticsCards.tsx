/**
 * 配置统计卡片
 */

import React from 'react';
import { Card, Col, Row } from 'antd';
import { scopeTypeColors } from './types';

export interface StatisticsCardsProps {
  totalCount: number;
  scopeTypeCounts: Record<string, number>;
}

export const StatisticsCards: React.FC<StatisticsCardsProps> = ({
  totalCount,
  scopeTypeCounts,
}) => {
  return (
    <Row gutter={16} style={{ marginBottom: 16 }}>
      <Col span={6}>
        <Card>
          <div style={{ textAlign: 'center' }}>
            <div style={{ fontSize: 24, fontWeight: 'bold', color: '#1890ff' }}>
              {totalCount}
            </div>
            <div style={{ color: '#8c8c8c' }}>总配置数</div>
          </div>
        </Card>
      </Col>
      {Object.entries(scopeTypeCounts).map(([type, count]) => (
        <Col span={6} key={type}>
          <Card>
            <div style={{ textAlign: 'center' }}>
              <div style={{ fontSize: 24, fontWeight: 'bold', color: scopeTypeColors[type] }}>
                {count}
              </div>
              <div style={{ color: '#8c8c8c' }}>{type}</div>
            </div>
          </Card>
        </Col>
      ))}
    </Row>
  );
};

export default StatisticsCards;
