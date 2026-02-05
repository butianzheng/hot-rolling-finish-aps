/**
 * 堆叠条形图组件
 */

import React from 'react';
import { Tooltip } from 'antd';
import { URGENCY_COLORS, ROLL_CHANGE_THRESHOLDS } from '../../theme';
import type { CapacityTimelineData } from '../../types/capacity';
import type { SegmentWithWidth } from './types';

export interface StackedBarChartProps {
  data: CapacityTimelineData;
  segments: SegmentWithWidth[];
  utilizationPercent: number;
  height: number;
}

export const StackedBarChart: React.FC<StackedBarChartProps> = ({
  data,
  segments,
  utilizationPercent,
  height,
}) => {
  return (
    <div
      style={{
        position: 'relative',
        height: height,
        borderRadius: 4,
        overflow: 'hidden',
        border: '1px solid rgba(0, 0, 0, 0.12)',
      }}
    >
      {/* 背景网格线 */}
      <div
        style={{
          position: 'absolute',
          top: 0,
          left: 0,
          right: 0,
          bottom: 0,
          display: 'flex',
        }}
      >
        {[0, 25, 50, 75, 100].map((percent) => (
          <div
            key={percent}
            style={{
              position: 'absolute',
              left: `${percent}%`,
              top: 0,
              bottom: 0,
              width: 1,
              backgroundColor: 'rgba(0, 0, 0, 0.06)',
            }}
          />
        ))}
      </div>

      {/* 目标产能线 */}
      <div
        style={{
          position: 'absolute',
          left: '100%',
          top: 0,
          bottom: 0,
          width: 2,
          backgroundColor: '#1677ff',
          zIndex: 2,
        }}
      />

      {/* 限制产能线 */}
      {data.limitCapacity !== data.targetCapacity && (
        <div
          style={{
            position: 'absolute',
            left: `${(data.limitCapacity / data.targetCapacity) * 100}%`,
            top: 0,
            bottom: 0,
            width: 2,
            backgroundColor: '#ff4d4f',
            zIndex: 2,
          }}
        />
      )}

      {/* 轧辊更换标记 */}
      {data.rollChangeThreshold === ROLL_CHANGE_THRESHOLDS.WARNING && (
        <div
          style={{
            position: 'absolute',
            left: `${(ROLL_CHANGE_THRESHOLDS.WARNING / data.targetCapacity) * 100}%`,
            top: 0,
            bottom: 0,
            width: 2,
            backgroundColor: '#faad14',
            zIndex: 1,
            opacity: 0.5,
          }}
        />
      )}
      {data.rollChangeThreshold === ROLL_CHANGE_THRESHOLDS.CRITICAL && (
        <div
          style={{
            position: 'absolute',
            left: `${(ROLL_CHANGE_THRESHOLDS.CRITICAL / data.targetCapacity) * 100}%`,
            top: 0,
            bottom: 0,
            width: 2,
            backgroundColor: '#ff4d4f',
            zIndex: 1,
            opacity: 0.5,
          }}
        />
      )}

      {/* 堆叠分段 */}
      <div
        style={{
          position: 'absolute',
          top: 0,
          left: 0,
          bottom: 0,
          display: 'flex',
          width: `${utilizationPercent}%`,
        }}
      >
        {segments.map((seg, index) => {
          const color =
            URGENCY_COLORS[
              `${seg.urgencyLevel}_${
                seg.urgencyLevel === 'L3'
                  ? 'EMERGENCY'
                  : seg.urgencyLevel === 'L2'
                  ? 'HIGH'
                  : seg.urgencyLevel === 'L1'
                  ? 'MEDIUM'
                  : 'NORMAL'
              }` as keyof typeof URGENCY_COLORS
            ];

          return (
            <Tooltip
              key={index}
              title={
                <div>
                  <div style={{ fontWeight: 'bold' }}>{seg.urgencyLevel}</div>
                  <div>吨位: {seg.tonnage.toFixed(3)}t</div>
                  <div>材料数: {seg.materialCount} 件</div>
                </div>
              }
            >
              <div
                style={{
                  flex: seg.widthPercent,
                  backgroundColor: color,
                  cursor: 'help',
                  transition: 'opacity 0.2s',
                  display: 'flex',
                  alignItems: 'center',
                  justifyContent: 'center',
                  color: '#fff',
                  fontWeight: 'bold',
                  fontSize: 12,
                }}
                onMouseEnter={(e) => (e.currentTarget.style.opacity = '0.8')}
                onMouseLeave={(e) => (e.currentTarget.style.opacity = '1')}
              >
                {seg.widthPercent > 10 && seg.urgencyLevel}
              </div>
            </Tooltip>
          );
        })}
      </div>
    </div>
  );
};

export default StackedBarChart;
