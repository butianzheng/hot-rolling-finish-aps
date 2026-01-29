/**
 * 风险趋势图
 */

import React from 'react';
import dayjs from 'dayjs';
import type { RiskDaySummary } from './types';

export interface TrendChartProps {
  riskSnapshots: RiskDaySummary[];
}

export const TrendChart: React.FC<TrendChartProps> = ({ riskSnapshots }) => {
  const maxScore = Math.max(...riskSnapshots.map((s) => s.risk_score), 0);

  return (
    <div style={{ padding: '20px' }}>
      <div style={{ display: 'flex', alignItems: 'flex-end', height: '300px', gap: '10px' }}>
        {riskSnapshots.map((snapshot, index) => {
          const height = maxScore > 0 ? (snapshot.risk_score / maxScore) * 100 : 0;
          const color =
            snapshot.risk_score > 60
              ? '#cf1322'
              : snapshot.risk_score > 40
              ? '#fa8c16'
              : '#52c41a';

          return (
            <div
              key={index}
              style={{
                flex: 1,
                display: 'flex',
                flexDirection: 'column',
                alignItems: 'center',
                gap: '8px',
              }}
            >
              <div
                style={{
                  fontSize: '12px',
                  fontWeight: 'bold',
                  color: color,
                }}
              >
                {snapshot.risk_score}
              </div>
              <div
                style={{
                  width: '100%',
                  height: `${height}%`,
                  backgroundColor: color,
                  borderRadius: '4px 4px 0 0',
                  transition: 'all 0.3s',
                  cursor: 'pointer',
                }}
                title={`${snapshot.plan_date}: ${snapshot.risk_score}`}
              />
              <div
                style={{
                  fontSize: '11px',
                  color: '#8c8c8c',
                  transform: 'rotate(-45deg)',
                  whiteSpace: 'nowrap',
                }}
              >
                {dayjs(snapshot.plan_date).format('MM-DD')}
              </div>
            </div>
          );
        })}
      </div>
    </div>
  );
};

export default TrendChart;
