// ==========================================
// 冷料库龄堆叠柱状图组件（基于ECharts）
// ==========================================
// 职责: 使用ECharts堆叠柱状图展示各机组的库龄分布
// ==========================================

import React, { useMemo } from 'react';
import ReactECharts from 'echarts-for-react';
import type { EChartsOption } from 'echarts';
import type { ColdStockBucket, AgeBin } from '../../types/decision';

// ==========================================
// Props定义
// ==========================================

export interface ColdStockChartProps {
  data: ColdStockBucket[];
  onMachineClick?: (machine: string) => void;
  selectedMachine?: string | null;
  height?: number;
  displayMode?: 'count' | 'weight'; // 显示数量或重量
}

// ==========================================
// 库龄区间颜色和标签
// ==========================================

const AGE_BIN_COLORS: Record<AgeBin, string> = {
  '0-7': '#52c41a',
  '8-14': '#1677ff',
  '15-30': '#faad14',
  '30+': '#ff4d4f',
};

const AGE_BIN_LABELS: Record<AgeBin, string> = {
  '0-7': '0-7天',
  '8-14': '8-14天',
  '15-30': '15-30天',
  '30+': '30天以上',
};

const AGE_BIN_ORDER: AgeBin[] = ['0-7', '8-14', '15-30', '30+'];

// ==========================================
// 主组件
// ==========================================

export const ColdStockChart: React.FC<ColdStockChartProps> = ({
  data,
  onMachineClick,
  selectedMachine,
  height = 400,
  displayMode = 'count',
}) => {
  // ==========================================
  // 数据处理
  // ==========================================

  const { machines, seriesData } = useMemo(() => {
    if (!data || data.length === 0) {
      return {
        machines: [],
        seriesData: {} as Record<AgeBin, number[]>,
      };
    }

    // 提取唯一机组并排序
    const machineSet = new Set<string>();
    data.forEach((bucket) => machineSet.add(bucket.machineCode));
    const machines = Array.from(machineSet).sort();

    // 为每个库龄区间创建系列数据
    const seriesData: Record<AgeBin, number[]> = {
      '0-7': [],
      '8-14': [],
      '15-30': [],
      '30+': [],
    };

    // 填充数据
    machines.forEach((machine) => {
      AGE_BIN_ORDER.forEach((ageBin) => {
        const bucket = data.find(
          (b) => b.machineCode === machine && b.ageBin === ageBin
        );
        const value = bucket
          ? displayMode === 'count'
            ? bucket.count
            : bucket.weightT
          : 0;
        seriesData[ageBin].push(value);
      });
    });

    return { machines, seriesData };
  }, [data, displayMode]);

  // ==========================================
  // ECharts配置
  // ==========================================

  const option: EChartsOption = useMemo(() => {
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
          show: false, // 堆叠图不显示标签，避免拥挤
        },
      })),
    };
  }, [machines, seriesData, displayMode]);

  // ==========================================
  // 事件处理
  // ==========================================

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

      {/* 选中机组标识 */}
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
