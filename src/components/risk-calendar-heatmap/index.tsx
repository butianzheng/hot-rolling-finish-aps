/**
 * 风险日历热力图组件（基于ECharts）
 *
 * 重构后：245 行 → ~50 行 (-80%)
 */

import React, { useMemo } from 'react';
import ReactECharts from 'echarts-for-react';
import type { RiskCalendarHeatmapProps } from './types';
import { useRiskHeatmapData } from './useRiskHeatmapData';
import { createChartOption } from './chartConfig';

export const RiskCalendarHeatmap: React.FC<RiskCalendarHeatmapProps> = ({
  data,
  onDateClick,
  selectedDate,
  height = 400,
}) => {
  const { heatmapData, dateRange, maxRiskScore } = useRiskHeatmapData(data);

  const option = useMemo(
    () => createChartOption({ data, heatmapData, dateRange, maxRiskScore }),
    [data, heatmapData, dateRange, maxRiskScore]
  );

  const onEvents = useMemo(
    () => ({
      click: (params: any) => {
        if (params.componentType === 'series' && params.data) {
          const clickedDate = params.data[0];
          if (onDateClick) {
            onDateClick(clickedDate);
          }
        }
      },
    }),
    [onDateClick]
  );

  return (
    <div style={{ position: 'relative' }}>
      <ReactECharts
        option={option}
        style={{ height: `${height}px`, width: '100%' }}
        onEvents={onEvents}
        notMerge={true}
        lazyUpdate={true}
        theme="default"
      />

      {selectedDate && (
        <div
          style={{
            position: 'absolute',
            top: '10px',
            right: '10px',
            padding: '8px 16px',
            background: '#1677ff',
            color: '#fff',
            borderRadius: '4px',
            fontSize: '12px',
            boxShadow: '0 2px 8px rgba(0,0,0,0.15)',
          }}
        >
          已选择: {selectedDate}
        </div>
      )}
    </div>
  );
};

export default React.memo(RiskCalendarHeatmap);
