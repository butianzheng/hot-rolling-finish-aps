/**
 * ECharts 日历热力图配置生成
 */

import type { EChartsOption } from 'echarts';
import type { CapacityPoolCalendarData } from '../../api/ipcSchemas/machineConfigSchemas';
import { CAPACITY_COLORS } from '../../hooks/useCapacityCalendar';

/**
 * 生成日历热力图配置（多月网格布局）
 */
export function generateCalendarOption(
  data: CapacityPoolCalendarData[],
  year: string
): EChartsOption {
  // 转换数据格式：[[date, utilization], ...]
  const heatmapData = data.map((d) => [d.plan_date, d.utilization_pct]);

  // 创建4个月的calendar配置（按季度显示）
  const calendars: any[] = [];
  const series: any[] = [];

  // 计算起始月份（当前月份）
  const currentDate = new Date();
  const startMonth = currentDate.getMonth(); // 0-11

  // 2x2网格布局
  const positions = [
    { top: '8%', left: '5%', right: '52%', bottom: '52%' }, // 左上
    { top: '8%', left: '52%', right: '5%', bottom: '52%' }, // 右上
    { top: '52%', left: '5%', right: '52%', bottom: '8%' }, // 左下
    { top: '52%', left: '52%', right: '5%', bottom: '8%' }, // 右下
  ];

  for (let i = 0; i < 4; i++) {
    const monthIndex = (startMonth + i) % 12;
    const yearOffset = Math.floor((startMonth + i) / 12);
    const displayYear = parseInt(year) + yearOffset;
    const monthStr = `${displayYear}-${String(monthIndex + 1).padStart(2, '0')}`;

    calendars.push({
      ...positions[i],
      range: monthStr,
      orient: 'horizontal', // 关键：横向布局，日期从左到右排列
      cellSize: ['auto', 32], // 宽度自动，高度32px
      splitLine: {
        show: true,
        lineStyle: {
          color: '#e8e8e8',
          width: 1,
          type: 'solid',
        },
      },
      yearLabel: { show: false },
      monthLabel: {
        show: true,
        fontSize: 14,
        fontWeight: 'bold',
        color: '#262626',
        margin: 10,
        nameMap: 'cn',
      },
      dayLabel: {
        show: true, // 显示周几标签
        firstDay: 1, // 从周一开始
        fontSize: 11,
        color: '#8c8c8c',
        margin: 6,
        nameMap: ['日', '一', '二', '三', '四', '五', '六'],
      },
      itemStyle: {
        borderWidth: 1,
        borderColor: '#e8e8e8',
        borderType: 'solid',
        color: '#fafafa', // 无数据显示浅灰背景
      },
    });

    series.push({
      type: 'heatmap',
      coordinateSystem: 'calendar',
      calendarIndex: i,
      data: heatmapData,
      label: {
        show: true,
        formatter: (params: any) => {
          // 显示日期数字（1-31）
          const date = params.data[0];
          const day = new Date(date).getDate();
          return String(day);
        },
        fontSize: 13,
        fontWeight: '500',
        color: '#262626',
        position: 'inside', // 确保数字在格子内部
        align: 'center', // 水平居中
        verticalAlign: 'middle', // 垂直居中
      },
      itemStyle: {
        borderWidth: 1,
        borderColor: '#fff',
        borderType: 'solid',
        borderRadius: 2,
      },
      emphasis: {
        itemStyle: {
          shadowBlur: 6,
          shadowColor: 'rgba(0, 0, 0, 0.15)',
          borderWidth: 2,
          borderColor: '#1890ff',
        },
        label: {
          fontSize: 14,
          fontWeight: 'bold',
        },
      },
    });
  }

  return {
    tooltip: {
      position: 'top',
      formatter: (params: any) => {
        const date = params.data[0];
        const utilization = (params.data[1] * 100).toFixed(1);
        const item = data.find((d) => d.plan_date === date);
        if (!item) return `${date}<br/>无数据`;

        return `
          <div style="font-size: 12px; line-height: 1.6;">
            <strong style="font-size: 13px;">${date}</strong><br/>
            <span style="color: #3498db;">利用率:</span> <strong>${utilization}%</strong><br/>
            <span style="color: #27ae60;">已用:</span> ${item.used_capacity_t.toFixed(3)}t<br/>
            <span style="color: #95a5a6;">目标:</span> ${item.target_capacity_t.toFixed(3)}t<br/>
            <span style="color: #e67e22;">极限:</span> ${item.limit_capacity_t.toFixed(3)}t
          </div>
        `;
      },
    },
    visualMap: {
      show: false,
      min: 0,
      max: 1.2,
      type: 'piecewise',
      pieces: [
        { min: 0, max: 0.001, color: '#fafafa' }, // 无数据
        { min: 0.001, max: 0.7, color: CAPACITY_COLORS.充裕 },
        { min: 0.7, max: 0.85, color: CAPACITY_COLORS.适中 },
        { min: 0.85, max: 1.0, color: CAPACITY_COLORS.紧张 },
        { min: 1.0, max: 1.2, color: CAPACITY_COLORS.超限 },
      ],
      outOfRange: {
        color: '#fafafa',
      },
    },
    calendar: calendars,
    series,
  };
}

/**
 * 生成简化版日历配置（月视图）
 */
export function generateMonthCalendarOption(
  data: CapacityPoolCalendarData[],
  _dateFrom: string,
  _dateTo: string
): EChartsOption {
  // 按月聚合数据
  const monthlyData: Record<
    string,
    { totalUsed: number; totalTarget: number; count: number }
  > = {};

  data.forEach((d) => {
    const month = d.plan_date.substring(0, 7); // YYYY-MM
    if (!monthlyData[month]) {
      monthlyData[month] = { totalUsed: 0, totalTarget: 0, count: 0 };
    }
    monthlyData[month].totalUsed += d.used_capacity_t;
    monthlyData[month].totalTarget += d.target_capacity_t;
    monthlyData[month].count += 1;
  });

  // 转换为图表数据
  const months = Object.keys(monthlyData).sort();
  const utilizationData = months.map((month) => {
    const { totalUsed, totalTarget } = monthlyData[month];
    return totalTarget > 0 ? (totalUsed / totalTarget) * 100 : 0;
  });

  return {
    title: {
      text: '月度产能利用率',
      left: 'center',
      textStyle: {
        fontSize: 14,
        fontWeight: 'bold',
      },
    },
    tooltip: {
      trigger: 'axis',
      formatter: (params: any) => {
        const month = params[0].axisValue;
        const utilization = params[0].data.toFixed(1);
        const { totalUsed, totalTarget, count } = monthlyData[month];
        return `
          <div style="font-size: 12px; line-height: 1.6;">
            <strong>${month}</strong><br/>
            利用率: <strong>${utilization}%</strong><br/>
            已用: ${totalUsed.toFixed(3)}t<br/>
            目标: ${totalTarget.toFixed(3)}t<br/>
            天数: ${count}天
          </div>
        `;
      },
    },
    xAxis: {
      type: 'category',
      data: months,
      axisLabel: {
        fontSize: 11,
      },
    },
    yAxis: {
      type: 'value',
      name: '利用率 (%)',
      max: 120,
      nameTextStyle: {
        fontSize: 11,
      },
      axisLabel: {
        fontSize: 11,
      },
    },
    series: [
      {
        type: 'bar',
        data: utilizationData,
        itemStyle: {
          color: (params: any) => {
            const value = params.data / 100;
            if (value < 0.7) return CAPACITY_COLORS.充裕;
            if (value < 0.85) return CAPACITY_COLORS.适中;
            if (value <= 1.0) return CAPACITY_COLORS.紧张;
            return CAPACITY_COLORS.超限;
          },
          borderRadius: [4, 4, 0, 0],
        },
        label: {
          show: true,
          position: 'top',
          formatter: '{c}%',
          fontSize: 10,
        },
        markLine: {
          data: [
            { yAxis: 70, label: { formatter: '充裕线', fontSize: 10 } },
            { yAxis: 85, label: { formatter: '适中线', fontSize: 10 } },
            { yAxis: 100, label: { formatter: '目标线', fontSize: 10 } },
          ],
          lineStyle: { type: 'dashed', width: 1 },
          label: {
            fontSize: 10,
          },
        },
      },
    ],
    grid: {
      left: '10%',
      right: '5%',
      bottom: '10%',
      top: '15%',
    },
  };
}
