// ==========================================
// 风险日历热力图组件（基于ECharts）
// ==========================================
// 职责: 使用ECharts日历热力图展示风险分数
// ==========================================

import React, { useMemo } from 'react';
import ReactECharts from 'echarts-for-react';
import type { EChartsOption } from 'echarts';
import type { DaySummary } from '../../types/decision';
import { RISK_LEVEL_COLORS } from '../../types/decision/d1-day-summary';

// ==========================================
// Props定义
// ==========================================

export interface RiskCalendarHeatmapProps {
  data: DaySummary[];
  onDateClick?: (date: string) => void;
  selectedDate?: string | null;
  height?: number;
}

// ==========================================
// 主组件
// ==========================================

export const RiskCalendarHeatmap: React.FC<RiskCalendarHeatmapProps> = ({
  data,
  onDateClick,
  selectedDate,
  height = 400,
}) => {
  // ==========================================
  // 数据处理
  // ==========================================

  const { heatmapData, dateRange, maxRiskScore } = useMemo(() => {
    if (!data || data.length === 0) {
      return {
        heatmapData: [],
        dateRange: ['', ''],
        maxRiskScore: 100,
      };
    }

    // 转换为ECharts热力图数据格式: [日期, 风险分数]
    const heatmapData = data.map((item) => [item.planDate, item.riskScore]);

    // 计算日期范围
    const dates = data.map((item) => item.planDate).sort();
    const dateRange = [dates[0], dates[dates.length - 1]];

    // 计算最大风险分数（用于颜色映射）
    const maxRiskScore = Math.max(...data.map((item) => item.riskScore), 100);

    return { heatmapData, dateRange, maxRiskScore };
  }, [data]);

  // ==========================================
  // ECharts配置
  // ==========================================

  const option: EChartsOption = useMemo(() => {
    // 根据数据范围计算单元格大小
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
              ${dayData.overloadWeightT > 0 ? `<div style="color: #ff4d4f;">超载: ${dayData.overloadWeightT.toFixed(1)}吨</div>` : ''}
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
            RISK_LEVEL_COLORS.LOW, // 0-25: 绿色
            '#1677ff', // 25-50: 蓝色
            RISK_LEVEL_COLORS.HIGH, // 50-75: 橙色
            RISK_LEVEL_COLORS.CRITICAL, // 75-100: 红色
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
          show: dataLength > 7, // 少于7天时不显示星期标签
          firstDay: 1, // 从周一开始
          nameMap: ['日', '一', '二', '三', '四', '五', '六'],
        },
        monthLabel: {
          show: dataLength > 30, // 少于30天时不显示月份标签
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
            show: dataLength <= 14, // 14天以内显示数值
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
  }, [data, heatmapData, dateRange, maxRiskScore]);

  // ==========================================
  // 事件处理
  // ==========================================

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

  // ==========================================
  // 渲染
  // ==========================================

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

      {/* 选中日期标识（可选） */}
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

// ==========================================
// 工具函数
// ==========================================

/**
 * 根据风险分数获取颜色
 */
function getRiskColor(score: number): string {
  if (score < 25) return RISK_LEVEL_COLORS.LOW;
  if (score < 50) return '#1677ff';
  if (score < 75) return RISK_LEVEL_COLORS.HIGH;
  return RISK_LEVEL_COLORS.CRITICAL;
}
