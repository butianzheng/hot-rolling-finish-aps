/**
 * 机组堵塞热力图组件（基于ECharts）
 *
 * 重构后：277 行 → ~50 行 (-82%)
 */

import React, { useMemo } from 'react';
import ReactECharts from 'echarts-for-react';
import type { BottleneckHeatmapProps } from './types';
import { useHeatmapData } from './useHeatmapData';
import { createChartOption } from './chartConfig';

export const BottleneckHeatmap: React.FC<BottleneckHeatmapProps> = ({
  data,
  onPointClick,
  selectedPoint,
  height = 500,
}) => {
  const { heatmapData, machines, dates, maxBottleneckScore } = useHeatmapData(data);

  const option = useMemo(
    () => createChartOption({ data, heatmapData, machines, dates, maxBottleneckScore }),
    [data, heatmapData, machines, dates, maxBottleneckScore]
  );

  const onEvents = useMemo(
    () => ({
      click: (params: any) => {
        if (params.componentType === 'series' && params.data) {
          const [dateIndex, machineIndex] = params.data;
          const date = dates[dateIndex];
          const machine = machines[machineIndex];
          if (onPointClick) {
            onPointClick(machine, date);
          }
        }
      },
    }),
    [onPointClick, dates, machines]
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

      {selectedPoint && (
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
          已选择: {selectedPoint.machine} - {selectedPoint.date}
        </div>
      )}
    </div>
  );
};

export default React.memo(BottleneckHeatmap);
