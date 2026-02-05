/**
 * 产能池日历主组件
 * 职责：以标准月历网格形式展示产能数据（多月视图）
 */

import React, { useMemo } from 'react';
import { Card, Space, Tag, Spin, Alert, Empty, Typography, Tooltip } from 'antd';
import dayjs from 'dayjs';
import { useCapacityCalendar, CAPACITY_COLORS } from '../../hooks/useCapacityCalendar';
import type { ViewMode } from './types';
import type { CapacityPoolCalendarData } from '../../api/ipcSchemas/machineConfigSchemas';

const { Text } = Typography;

// 星期标签
const WEEK_DAYS = ['日', '一', '二', '三', '四', '五', '六'];

export interface CapacityCalendarProps {
  versionId: string;
  machineCode: string;
  dateFrom: string;
  dateTo: string;
  viewMode: ViewMode;
}

// 获取产能颜色
function getCapacityColor(utilization: number): string {
  if (utilization < 0.001) return 'transparent';
  if (utilization < 0.7) return CAPACITY_COLORS.充裕;
  if (utilization < 0.85) return CAPACITY_COLORS.适中;
  if (utilization <= 1.0) return CAPACITY_COLORS.紧张;
  return CAPACITY_COLORS.超限;
}

// 生成单个月的网格数据
function generateMonthGrid(year: number, month: number): (number | null)[][] {
  const firstDay = dayjs(new Date(year, month, 1));
  const lastDay = dayjs(new Date(year, month + 1, 0));
  const daysInMonth = lastDay.date();
  const startDayOfWeek = firstDay.day(); // 0-6, 0=周日

  const grid: (number | null)[][] = [];
  let week: (number | null)[] = [];

  // 填充月初空白
  for (let i = 0; i < startDayOfWeek; i++) {
    week.push(null);
  }

  // 填充日期
  for (let day = 1; day <= daysInMonth; day++) {
    week.push(day);
    if (week.length === 7) {
      grid.push(week);
      week = [];
    }
  }

  // 填充月末空白
  if (week.length > 0) {
    while (week.length < 7) {
      week.push(null);
    }
    grid.push(week);
  }

  return grid;
}

export const CapacityCalendar: React.FC<CapacityCalendarProps> = ({
  versionId,
  machineCode,
  dateFrom,
  dateTo,
  viewMode,
}) => {
  const {
    calendarData,
    calendarLoading,
    calendarError,
    statistics,
  } = useCapacityCalendar(versionId, machineCode, dateFrom, dateTo);

  // 数据映射表（日期 -> 数据）
  const dataMap = useMemo(() => {
    const map = new Map<string, CapacityPoolCalendarData>();
    calendarData.forEach((d) => {
      map.set(d.plan_date, d);
    });
    return map;
  }, [calendarData]);

  // 计算要显示的月份列表（从当前月开始的4个月）
  const monthsToShow = useMemo(() => {
    const currentDate = dayjs();
    const months: { year: number; month: number; label: string }[] = [];
    for (let i = 0; i < 4; i++) {
      const m = currentDate.add(i, 'month');
      months.push({
        year: m.year(),
        month: m.month(),
        label: m.format('YYYY年M月'),
      });
    }
    return months;
  }, []);

  // 渲染单个日期格子
  const renderDateCell = (
    day: number | null,
    year: number,
    month: number,
    rowIndex: number,
    colIndex: number
  ) => {
    if (day === null) {
      return (
        <div
          key={`${rowIndex}-${colIndex}`}
          style={{
            width: 32,
            height: 32,
            display: 'flex',
            alignItems: 'center',
            justifyContent: 'center',
          }}
        />
      );
    }

    const dateStr = dayjs(new Date(year, month, day)).format('YYYY-MM-DD');
    const data = dataMap.get(dateStr);
    const isToday = dayjs().format('YYYY-MM-DD') === dateStr;
    const utilization = data?.utilization_pct || 0;
    const bgColor = getCapacityColor(utilization);
    const hasData = data !== undefined;

    const cellContent = (
      <div
        style={{
          width: 32,
          height: 32,
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'center',
          cursor: hasData ? 'pointer' : 'default',
          borderRadius: 4,
          backgroundColor: hasData ? bgColor : 'transparent',
          border: isToday ? '2px solid #1677ff' : '1px solid #f0f0f0',
          color: hasData && utilization >= 0.7 ? '#fff' : '#262626',
          fontWeight: isToday ? 600 : 400,
          fontSize: 13,
          transition: 'all 0.15s',
        }}
        onMouseEnter={(e) => {
          if (hasData) {
            e.currentTarget.style.transform = 'scale(1.1)';
            e.currentTarget.style.boxShadow = '0 2px 8px rgba(0,0,0,0.15)';
          }
        }}
        onMouseLeave={(e) => {
          e.currentTarget.style.transform = 'scale(1)';
          e.currentTarget.style.boxShadow = 'none';
        }}
      >
        {day}
      </div>
    );

    if (hasData && data) {
      return (
        <Tooltip
          key={`${rowIndex}-${colIndex}`}
          title={
            <div style={{ fontSize: 12, lineHeight: 1.6 }}>
              <strong>{dateStr}</strong>
              <br />
              利用率: <strong>{(utilization * 100).toFixed(1)}%</strong>
              <br />
              已用: {data.used_capacity_t.toFixed(3)}t
              <br />
              目标: {data.target_capacity_t.toFixed(3)}t
              <br />
              极限: {data.limit_capacity_t.toFixed(3)}t
            </div>
          }
        >
          {cellContent}
        </Tooltip>
      );
    }

    return <div key={`${rowIndex}-${colIndex}`}>{cellContent}</div>;
  };

  // 渲染单个月份的日历
  const renderSingleMonth = (year: number, month: number, label: string) => {
    const grid = generateMonthGrid(year, month);

    return (
      <div key={`${year}-${month}`} style={{ flex: '1 1 auto', minWidth: 260 }}>
        {/* 月份标题 */}
        <div
          style={{
            textAlign: 'center',
            fontWeight: 600,
            fontSize: 13,
            marginBottom: 8,
            color: '#262626',
          }}
        >
          {label}
        </div>

        {/* 星期标签行 */}
        <div style={{ display: 'flex', gap: 2, marginBottom: 4 }}>
          {WEEK_DAYS.map((day, index) => (
            <div
              key={index}
              style={{
                width: 32,
                height: 24,
                display: 'flex',
                alignItems: 'center',
                justifyContent: 'center',
                fontSize: 11,
                color: '#8c8c8c',
                fontWeight: 500,
              }}
            >
              {day}
            </div>
          ))}
        </div>

        {/* 日期网格 */}
        <div style={{ display: 'flex', flexDirection: 'column', gap: 2 }}>
          {grid.map((week, rowIndex) => (
            <div key={rowIndex} style={{ display: 'flex', gap: 2 }}>
              {week.map((day, colIndex) =>
                renderDateCell(day, year, month, rowIndex, colIndex)
              )}
            </div>
          ))}
        </div>
      </div>
    );
  };

  // 统计卡片 - 紧凑样式
  const renderStatistics = () => (
    <Space direction="vertical" size={6} style={{ width: '100%' }}>
      {/* 数据统计 */}
      <div style={{ display: 'flex', flexWrap: 'wrap', gap: 6, alignItems: 'center' }}>
        <Tag color="blue" style={{ margin: 0, fontSize: 11, padding: '0 6px' }}>
          总目标: {statistics.totalTarget.toFixed(3)}t
        </Tag>
        <Tag color="cyan" style={{ margin: 0, fontSize: 11, padding: '0 6px' }}>
          总已用: {statistics.totalUsed.toFixed(3)}t
        </Tag>
        <Tag
          color={statistics.totalRemaining > 0 ? 'green' : 'red'}
          style={{ margin: 0, fontSize: 11, padding: '0 6px' }}
        >
          总剩余: {statistics.totalRemaining.toFixed(3)}t
        </Tag>
        <Tag color="purple" style={{ margin: 0, fontSize: 11, padding: '0 6px' }}>
          平均利用率: {(statistics.avgUtilization * 100).toFixed(1)}%
        </Tag>
        {statistics.overLimitCount > 0 && (
          <Tag color="red" style={{ margin: 0, fontSize: 11, padding: '0 6px' }}>
            超限天数: {statistics.overLimitCount}
          </Tag>
        )}
      </div>

      {/* 色彩图例 */}
      <div
        style={{
          display: 'flex',
          alignItems: 'center',
          gap: 8,
          fontSize: 11,
          flexWrap: 'nowrap',
          overflowX: 'auto',
        }}
      >
        <Text type="secondary" style={{ fontSize: 11, fontWeight: 500, flexShrink: 0 }}>
          图例:
        </Text>
        <span style={{ display: 'flex', alignItems: 'center', gap: 3, flexShrink: 0 }}>
          <span style={{ width: 14, height: 14, backgroundColor: CAPACITY_COLORS.充裕, borderRadius: 2 }} />
          <span>充裕({statistics.充裕Count})</span>
        </span>
        <span style={{ display: 'flex', alignItems: 'center', gap: 3, flexShrink: 0 }}>
          <span style={{ width: 14, height: 14, backgroundColor: CAPACITY_COLORS.适中, borderRadius: 2 }} />
          <span>适中({statistics.适中Count})</span>
        </span>
        <span style={{ display: 'flex', alignItems: 'center', gap: 3, flexShrink: 0 }}>
          <span style={{ width: 14, height: 14, backgroundColor: CAPACITY_COLORS.紧张, borderRadius: 2 }} />
          <span>紧张({statistics.紧张Count})</span>
        </span>
        <span style={{ display: 'flex', alignItems: 'center', gap: 3, flexShrink: 0 }}>
          <span style={{ width: 14, height: 14, backgroundColor: CAPACITY_COLORS.超限, borderRadius: 2 }} />
          <span>超限({statistics.超限Count})</span>
        </span>
        <span style={{ display: 'flex', alignItems: 'center', gap: 3, flexShrink: 0 }}>
          <span
            style={{
              width: 14,
              height: 14,
              border: '1.5px dashed #d0d7de',
              backgroundColor: 'transparent',
              borderRadius: 2,
            }}
          />
          <span>无数据</span>
        </span>
      </div>
    </Space>
  );

  return (
    <Card
      title={
        <Space size={8}>
          <Text strong style={{ fontSize: 14 }}>
            {machineCode}
          </Text>
          <Text type="secondary" style={{ fontSize: 12, fontWeight: 400 }}>
            {dateFrom} ~ {dateTo} · {calendarData.length}天
          </Text>
        </Space>
      }
      size="small"
      headStyle={{ padding: '8px 12px' }}
      bodyStyle={{ padding: 12 }}
    >
      {calendarLoading ? (
        <div style={{ textAlign: 'center', padding: '40px 0' }}>
          <Spin tip="加载中..." size="small" />
        </div>
      ) : calendarError ? (
        <Alert
          type="error"
          message="加载失败"
          description={calendarError.message}
          showIcon
          style={{ fontSize: 12 }}
        />
      ) : calendarData.length === 0 ? (
        <Empty
          description={<span style={{ fontSize: 12 }}>该日期范围内无产能数据</span>}
          imageStyle={{ height: 60 }}
        />
      ) : (
        <Space direction="vertical" style={{ width: '100%' }} size={12}>
          {/* 统计信息 */}
          {renderStatistics()}

          {/* 多月日历网格 - 2x2布局 */}
          {viewMode === 'day' && (
            <div
              style={{
                display: 'grid',
                gridTemplateColumns: 'repeat(2, 1fr)',
                gap: 24,
                padding: 8,
              }}
            >
              {monthsToShow.map((m) => renderSingleMonth(m.year, m.month, m.label))}
            </div>
          )}

          {viewMode === 'month' && (
            <Text type="secondary" style={{ fontSize: 12 }}>
              月视图开发中...
            </Text>
          )}

          {/* 提示信息 */}
          <Text type="secondary" style={{ fontSize: 11 }}>
            💡 提示：鼠标悬停日期可查看详情
          </Text>
        </Space>
      )}
    </Card>
  );
};

export default CapacityCalendar;
