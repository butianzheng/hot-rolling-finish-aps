/**
 * 冷料库龄堆叠柱状图组件（基于ECharts）
 *
 * 重构后：239 行 → ~55 行 (-77%)
 */

import React, { useMemo } from 'react';
import ReactECharts from 'echarts-for-react';
import type { ColdStockChartProps } from './types';
import { useColdStockData } from './useColdStockData';
import { createChartOption } from './chartConfig';

export const ColdStockChart: React.FC<ColdStockChartProps> = ({
  data,
  onMachineClick,
  selectedMachine,
  height = 400,
  displayMode = 'count',
}) => {
  const { machines, seriesData } = useColdStockData(data, displayMode);

  const option = useMemo(
    () => createChartOption({ machines, seriesData, displayMode }),
    [machines, seriesData, displayMode]
  );

  const onEvents = useMemo(
    () => ({
      click: (params: any) => {
        if (params.componentType === 'series') {
          const machine = params.name;
          if (onMachineClick) {
            onMachineClick(machine);
          }
        }
      },
    }),
    [onMachineClick]
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

      {selectedMachine && (
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
          已选择: {selectedMachine}
        </div>
      )}
    </div>
  );
};

export default React.memo(ColdStockChart);
