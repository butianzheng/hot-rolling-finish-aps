/**
 * ECharts 堆叠柱状图配置生成
 */

import type { EChartsOption } from 'echarts';
import type { AgeBin } from '../../types/decision';
import { AGE_BIN_COLORS, AGE_BIN_LABELS, AGE_BIN_ORDER } from './types';

interface ChartConfigParams {
  machines: string[];
  seriesData: Record<AgeBin, number[]>;
  displayMode: 'count' | 'weight';
}

export function createChartOption({
  machines,
  seriesData,
  displayMode,
}: ChartConfigParams): EChartsOption {
  return {
    tooltip: {
      trigger: 'axis',
      axisPointer: {
        type: 'shadow',
      },
      formatter: (params: any) => {
        if (!Array.isArray(params)) return '';

        const machine = params[0].axisValue;
        let total = 0;
        let tooltipHtml = `<div style="padding: 8px;"><div style="font-weight: bold; margin-bottom: 8px;">${machine}</div>`;

        params.forEach((param: any) => {
          const value = param.value;
          total += value;
          tooltipHtml += `
            <div style="margin-bottom: 4px;">
              <span style="display:inline-block;width:10px;height:10px;background:${param.color};border-radius:50%;margin-right:8px;"></span>
              ${param.seriesName}: ${value.toFixed(displayMode === 'count' ? 0 : 1)}${displayMode === 'count' ? '个' : '吨'}
            </div>
          `;
        });

        tooltipHtml += `<div style="margin-top: 8px; padding-top: 8px; border-top: 1px solid #eee; font-weight: bold;">
          总计: ${total.toFixed(displayMode === 'count' ? 0 : 1)}${displayMode === 'count' ? '个' : '吨'}
        </div></div>`;

        return tooltipHtml;
      },
    },

    legend: {
      data: AGE_BIN_ORDER.map((bin) => AGE_BIN_LABELS[bin]),
      top: 10,
    },

    grid: {
      left: 60,
      right: 40,
      top: 60,
      bottom: 60,
      containLabel: true,
    },

    xAxis: {
      type: 'category',
      data: machines,
      axisLabel: {
        interval: 0,
        rotate: machines.length > 8 ? 45 : 0,
        fontSize: 11,
      },
      name: '机组',
      nameLocation: 'middle',
      nameGap: machines.length > 8 ? 40 : 25,
    },

    yAxis: {
      type: 'value',
      name: displayMode === 'count' ? '数量（个）' : '重量（吨）',
      nameLocation: 'middle',
      nameGap: 45,
    },

    series: AGE_BIN_ORDER.map((ageBin) => ({
      name: AGE_BIN_LABELS[ageBin],
      type: 'bar',
      stack: 'total',
      data: seriesData[ageBin],
      itemStyle: {
        color: AGE_BIN_COLORS[ageBin],
      },
      emphasis: {
        focus: 'series',
      },
      label: {
        show: false,
      },
    })),
  };
}
