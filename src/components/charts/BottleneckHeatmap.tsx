// ==========================================
// 机组堵塞热力图组件（基于ECharts）
// ==========================================
// 职责: 使用ECharts二维热力图展示机组×日期堵塞矩阵
// ==========================================

import React, { useMemo } from 'react';
import ReactECharts from 'echarts-for-react';
import type { EChartsOption } from 'echarts';
import type { BottleneckPoint, BottleneckLevel } from '../../types/decision';

// ==========================================
// Props定义
// ==========================================

export interface BottleneckHeatmapProps {
  data: BottleneckPoint[];
  onPointClick?: (machine: string, date: string) => void;
  selectedPoint?: { machine: string; date: string } | null;
  height?: number;
}

// ==========================================
// 堵塞等级颜色映射
// ==========================================

const BOTTLENECK_LEVEL_COLORS: Record<BottleneckLevel, string> = {
  NONE: '#d9d9d9',
  LOW: '#52c41a',
  MEDIUM: '#1677ff',
  HIGH: '#faad14',
  CRITICAL: '#ff4d4f',
};

// ==========================================
// 主组件
// ==========================================

export const BottleneckHeatmap: React.FC<BottleneckHeatmapProps> = ({
  data,
  onPointClick,
  selectedPoint,
  height = 500,
}) => {
  // ==========================================
  // 数据处理
  // ==========================================

  const { heatmapData, machines, dates, maxBottleneckScore } = useMemo(() => {
    if (!data || data.length === 0) {
      return {
        heatmapData: [],
        machines: [],
        dates: [],
        maxBottleneckScore: 100,
      };
    }

    // 提取唯一的机组和日期
    const machineSet = new Set<string>();
    const dateSet = new Set<string>();

    data.forEach((item) => {
      machineSet.add(item.machineCode);
      dateSet.add(item.planDate);
    });

    const machines = Array.from(machineSet).sort();
    const dates = Array.from(dateSet).sort();

    // 转换为ECharts热力图数据格式: [日期索引, 机组索引, 堵塞分数]
    const heatmapData = data.map((item) => {
      const dateIndex = dates.indexOf(item.planDate);
      const machineIndex = machines.indexOf(item.machineCode);
      return [dateIndex, machineIndex, item.bottleneckScore];
    });

    // 计算最大堵塞分数（用于颜色映射）
    const maxBottleneckScore = Math.max(...data.map((item) => item.bottleneckScore), 100);

    return { heatmapData, machines, dates, maxBottleneckScore };
  }, [data]);

  // ==========================================
  // ECharts配置
  // ==========================================

  const option: EChartsOption = useMemo(() => {
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
          rotate: dates.length > 14 ? 45 : 0, // 超过14天时斜放标签
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
            BOTTLENECK_LEVEL_COLORS.LOW, // 0-25: 绿色
            BOTTLENECK_LEVEL_COLORS.MEDIUM, // 25-50: 蓝色
            BOTTLENECK_LEVEL_COLORS.HIGH, // 50-75: 橙色
            BOTTLENECK_LEVEL_COLORS.CRITICAL, // 75-100: 红色
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
            show: dates.length <= 14 && machines.length <= 8, // 数据量不多时显示数值
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
  }, [data, heatmapData, machines, dates, maxBottleneckScore]);

  // ==========================================
  // 事件处理
  // ==========================================

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

      {/* 选中点位标识（可选） */}
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

// ==========================================
// 工具函数
// ==========================================

/**
 * 根据堵塞分数获取颜色
 */
function getBottleneckColor(score: number): string {
  if (score < 25) return BOTTLENECK_LEVEL_COLORS.LOW;
  if (score < 50) return BOTTLENECK_LEVEL_COLORS.MEDIUM;
  if (score < 75) return BOTTLENECK_LEVEL_COLORS.HIGH;
  return BOTTLENECK_LEVEL_COLORS.CRITICAL;
}
