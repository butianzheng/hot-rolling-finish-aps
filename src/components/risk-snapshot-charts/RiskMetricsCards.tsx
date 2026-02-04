/**
 * 风险指标卡片
 */

import React from 'react';
import { Card, Col, Row, Statistic } from 'antd';
import { formatNumber } from '../../utils/formatters';
import type { DaySummary } from '../../types/decision';

export interface RiskMetricsCardsProps {
  riskSnapshots: DaySummary[];
  mostRiskyDate: string | null;
}

export const RiskMetricsCards: React.FC<RiskMetricsCardsProps> = ({
  riskSnapshots,
  mostRiskyDate,
}) => {
  if (riskSnapshots.length === 0) return null;

  const avgRiskScore =
    riskSnapshots.reduce((sum, s) => sum + s.riskScore, 0) / riskSnapshots.length;
  const totalUrgentFailures = riskSnapshots.reduce((sum, s) => sum + (s.urgentFailureCount || 0), 0);
  const totalOverloadT = riskSnapshots.reduce((sum, s) => sum + (s.overloadWeightT || 0), 0);

  // 获取最危险日期的风险分数
  const mostRiskySnapshot = riskSnapshots.find((s) => s.planDate === mostRiskyDate);
  const maxRiskScore = mostRiskySnapshot?.riskScore || 0;

  return (
    <Row gutter={16} style={{ marginBottom: 16 }}>
      <Col span={6}>
        <Card>
          <Statistic
            title="平均风险分数"
            value={formatNumber(avgRiskScore, 1)}
            valueStyle={{ color: avgRiskScore > 50 ? '#cf1322' : '#52c41a' }}
          />
        </Card>
      </Col>
      <Col span={6}>
        <Card style={{ borderColor: mostRiskyDate ? '#ff4d4f' : undefined }}>
          <Statistic
            title="最高风险分数"
            value={maxRiskScore}
            suffix={mostRiskyDate ? `(${mostRiskyDate})` : ''}
            valueStyle={{ color: '#cf1322' }}
          />
        </Card>
      </Col>
      <Col span={6}>
        <Card>
          <Statistic
            title="累计紧急单失败"
            value={totalUrgentFailures}
            suffix="单"
            valueStyle={{ color: totalUrgentFailures > 0 ? '#cf1322' : '#52c41a' }}
          />
        </Card>
      </Col>
      <Col span={6}>
        <Card>
          <Statistic
            title="累计超载吨数"
            value={formatNumber(totalOverloadT, 1)}
            suffix="t"
            valueStyle={{ color: '#fa8c16' }}
          />
        </Card>
      </Col>
    </Row>
  );
};

export default RiskMetricsCards;
