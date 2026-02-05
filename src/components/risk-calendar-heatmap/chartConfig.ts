/**
 * ECharts 日历热力图配置生成
 */

import type { EChartsOption } from 'echarts';
import type { DaySummary } from '../../types/decision';
import { RISK_LEVEL_COLORS } from '../../types/decision/d1-day-summary';
import { getRiskColor } from './types';

interface ChartConfigParams {
  data: DaySummary[];
  heatmapData: [string, number][];
  dateRange: [string, string];
  maxRiskScore: number;
}

export function createChartOption({
  data,
  heatmapData,
  dateRange,
  maxRiskScore,
}: ChartConfigParams): EChartsOption {
  const dataLength = data.length;
  const cellSize: [number, number] | 'auto' = dataLength <= 7 ? [60, 60] : dataLength <= 14 ? [50, 50] : dataLength <= 30 ? [40, 40] : 'auto';

  return {
    tooltip: {
      position: 'top',
      formatter: (params: any) => {
        const date = params.data[0];
        const riskScore = params.data[1];
        const dayData = data.find((item) => item.planDate === date);

        if (!dayData) return '';

        return `
          <div style="padding: 8px;">
            <div style="font-weight: bold; margin-bottom: 8px;">
              ${date}
            </div>
            <div>风险分数: <span style="color: ${getRiskColor(riskScore)}; font-weight: bold;">${riskScore.toFixed(1)}</span></div>
            <div>风险等级: <span style="color: ${RISK_LEVEL_COLORS[dayData.riskLevel]};">${dayData.riskLevel}</span></div>
            <div>容量利用率: ${dayData.capacityUtilPct.toFixed(1)}%</div>
            <div>紧急订单失败: ${dayData.urgentFailureCount}个</div>
            ${dayData.overloadWeightT > 0 ? `<div style="color: #ff4d4f;">超载: ${dayData.overloadWeightT.toFixed(2)}吨</div>` : ''}
            <div style="margin-top: 4px; font-size: 12px; color: #8c8c8c;">点击查看详情</div>
          </div>
        `;
      },
    },

    visualMap: {
      min: 0,
      max: maxRiskScore,
      calculable: true,
      orient: 'horizontal',
      left: 'center',
      bottom: '10',
      inRange: {
        color: [
          RISK_LEVEL_COLORS.LOW,
          '#1677ff',
          RISK_LEVEL_COLORS.HIGH,
          RISK_LEVEL_COLORS.CRITICAL,
        ],
      },
      text: ['高风险', '低风险'],
      textStyle: {
        color: '#333',
      },
    },

    calendar: {
      top: 60,
      left: 50,
      right: 30,
      cellSize: cellSize,
      range: dateRange,
      itemStyle: {
        borderWidth: 2,
        borderColor: '#fff',
        color: '#f0f0f0',
      },
      splitLine: {
        show: true,
        lineStyle: {
          color: '#e0e0e0',
          width: 2,
        },
      },
      dayLabel: {
        show: dataLength > 7,
        firstDay: 1,
        nameMap: ['日', '一', '二', '三', '四', '五', '六'],
      },
      monthLabel: {
        show: dataLength > 30,
        nameMap: 'ZH',
      },
      yearLabel: {
        show: true,
        position: 'top',
        formatter: '{start} - {end}',
      },
    },

    series: [
      {
        type: 'heatmap',
        coordinateSystem: 'calendar',
        data: heatmapData,
        label: {
          show: dataLength <= 14,
          formatter: (params: any) => {
            return params.data[1].toFixed(0);
          },
          fontSize: 12,
        },
        emphasis: {
          itemStyle: {
            shadowBlur: 10,
            shadowColor: 'rgba(0, 0, 0, 0.5)',
            borderWidth: 3,
            borderColor: '#000',
          },
        },
        itemStyle: {
          borderRadius: 4,
        },
      },
    ],
  };
}
