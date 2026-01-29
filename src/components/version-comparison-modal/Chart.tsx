/**
 * 懒加载图表组件
 */

import React from 'react';
import type { EChartsOption } from 'echarts';

const LazyECharts = React.lazy(() => import('echarts-for-react'));

interface ChartProps {
  option: EChartsOption;
  height: number;
}

export const Chart: React.FC<ChartProps> = ({ option, height }) => {
  return (
    <React.Suspense
      fallback={
        <div
          style={{
            height,
            width: '100%',
            background: '#fafafa',
            border: '1px dashed #d9d9d9',
            borderRadius: 6,
            display: 'flex',
            alignItems: 'center',
            justifyContent: 'center',
            color: '#8c8c8c',
            fontSize: 12,
          }}
        >
          图表加载中…
        </div>
      }
    >
      <LazyECharts option={option} style={{ height, width: '100%' }} notMerge lazyUpdate />
    </React.Suspense>
  );
};
