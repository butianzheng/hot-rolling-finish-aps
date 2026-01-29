/**
 * ECharts 配置生成
 */

import type { EChartsOption } from 'echarts';
import type { BottleneckPoint } from '../../types/decision';
import { BOTTLENECK_LEVEL_COLORS, getBottleneckColor } from './types';

interface ChartConfigParams {
  data: BottleneckPoint[];
  heatmapData: [number, number, number][];
  machines: string[];
  dates: string[];
  maxBottleneckScore: number;
}

export function createChartOption({
  data,
  heatmapData,
  machines,
  dates,
  maxBottleneckScore,
}: ChartConfigParams): EChartsOption {
  return {
    tooltip: {
      position: 'top',
      formatter: (params: any) => {
        const [dateIndex, machineIndex, bottleneckScore] = params.data;
        const date = dates[dateIndex];
        const machine = machines[machineIndex];

        const pointData = data.find(
          (item) => item.machineCode === machine && item.planDate === date
        );

        if (!pointData) return '';

        return `
          <div style="padding: 8px;">
            <div style="font-weight: bold; margin-bottom: 8px;">
              ${machine} - ${date}
            </div>
            <div>堵塞分数: <span style="color: ${getBottleneckColor(bottleneckScore)}; font-weight: bold;">${bottleneckScore.toFixed(1)}</span></div>
            <div>堵塞等级: <span style="color: ${BOTTLENECK_LEVEL_COLORS[pointData.bottleneckLevel]};">${pointData.bottleneckLevel}</span></div>
            <div>容量利用率: ${pointData.capacityUtilPct.toFixed(1)}%</div>
            <div>待排材料: ${pointData.pendingMaterialCount}个</div>
            <div>待排重量: ${pointData.pendingWeightT.toFixed(1)}吨</div>
            ${pointData.bottleneckTypes.length > 0 ? `<div>堵塞类型: ${pointData.bottleneckTypes.join(', ')}</div>` : ''}
            <div style="margin-top: 4px; font-size: 12px; color: #8c8c8c;">点击查看详情</div>
          </div>
        `;
      },
    },

    grid: {
      left: 120,
      right: 80,
      top: 60,
      bottom: 80,
      containLabel: true,
    },

    xAxis: {
      type: 'category',
      data: dates,
      splitArea: {
        show: true,
      },
      axisLabel: {
        interval: 0,
        rotate: dates.length > 14 ? 45 : 0,
        fontSize: 11,
      },
      name: '日期',
      nameLocation: 'middle',
      nameGap: dates.length > 14 ? 45 : 30,
    },

    yAxis: {
      type: 'category',
      data: machines,
      splitArea: {
        show: true,
      },
      axisLabel: {
        fontSize: 12,
        fontWeight: 'bold',
      },
      name: '机组',
      nameLocation: 'middle',
      nameGap: 100,
      nameRotate: 90,
    },

    visualMap: {
      min: 0,
      max: maxBottleneckScore,
      calculable: true,
      orient: 'horizontal',
      left: 'center',
      bottom: '10',
      inRange: {
        color: [
          BOTTLENECK_LEVEL_COLORS.LOW,
          BOTTLENECK_LEVEL_COLORS.MEDIUM,
          BOTTLENECK_LEVEL_COLORS.HIGH,
          BOTTLENECK_LEVEL_COLORS.CRITICAL,
        ],
      },
      text: ['高堵塞', '低堵塞'],
      textStyle: {
        color: '#333',
      },
    },

    series: [
      {
        type: 'heatmap',
        data: heatmapData,
        label: {
          show: dates.length <= 14 && machines.length <= 8,
          formatter: (params: any) => {
            return params.data[2].toFixed(0);
          },
          fontSize: 11,
        },
        emphasis: {
          itemStyle: {
            shadowBlur: 10,
            shadowColor: 'rgba(0, 0, 0, 0.5)',
            borderWidth: 2,
            borderColor: '#000',
          },
        },
        itemStyle: {
          borderRadius: 2,
        },
      },
    ],
  };
}
