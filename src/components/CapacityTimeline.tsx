// ==========================================
// 产能时间线组件
// ==========================================
// 水平堆叠条形图，显示产能利用和轧辊状态
// 使用 useMemo 优化计算性能
// ==========================================

import React, { useMemo } from 'react';
import { Card, Space, Typography, Tooltip, Progress } from 'antd';
import { ToolOutlined, WarningOutlined } from '@ant-design/icons';
import { URGENCY_COLORS, ROLL_CHANGE_THRESHOLDS, FONT_FAMILIES } from '../theme';
import type { CapacityTimelineData } from '../types/capacity';

const { Text, Title } = Typography;

interface CapacityTimelineProps {
  data: CapacityTimelineData;
  height?: number;
}

const CapacityTimelineComponent: React.FC<CapacityTimelineProps> = ({ data, height = 120 }) => {
  // 计算产能利用率
  const utilizationPercent = useMemo(() => {
    const target = Number(data.targetCapacity);
    const actual = Number(data.actualCapacity);
    if (!Number.isFinite(target) || target <= 0) return 0;
    if (!Number.isFinite(actual) || actual <= 0) return 0;
    return (actual / target) * 100;
  }, [data.actualCapacity, data.targetCapacity]);

  // 计算是否超限
  const isOverLimit =
    Number.isFinite(data.actualCapacity) &&
    Number.isFinite(data.limitCapacity) &&
    data.actualCapacity > data.limitCapacity;

  // 计算轧辊状态
  const rollStatus = useMemo(() => {
    const progress = data.rollCampaignProgress;
    const threshold = data.rollChangeThreshold;
    if (progress >= threshold) return 'critical';
    if (progress >= ROLL_CHANGE_THRESHOLDS.WARNING) return 'warning';
    return 'healthy';
  }, [data.rollCampaignProgress, data.rollChangeThreshold]);

  const getRollStatusColor = () => {
    switch (rollStatus) {
      case 'critical':
        return '#ff4d4f';
      case 'warning':
        return '#faad14';
      default:
        return '#52c41a';
    }
  };

  // 计算每个分段的宽度百分比
  const segments = useMemo(() => {
    const total = Number(data.actualCapacity);
    const safeTotal = Number.isFinite(total) && total > 0 ? total : 0;
    return data.segments.map((seg) => ({
      ...seg,
      widthPercent: safeTotal > 0 ? (seg.tonnage / safeTotal) * 100 : 0,
    }));
  }, [data.segments, data.actualCapacity]);

  return (
    <Card
      size="small"
      style={{
        marginBottom: 16,
        borderRadius: 8,
      }}
    >
      <Space direction="vertical" style={{ width: '100%' }} size={12}>
        {/* 标题行 */}
        <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
          <Space size={16}>
            <Title level={5} style={{ margin: 0 }}>
              {data.date} - {data.machineCode}
            </Title>
            <Text type="secondary" style={{ fontFamily: FONT_FAMILIES.MONOSPACE }}>
              {data.actualCapacity.toFixed(1)}t / {data.targetCapacity.toFixed(1)}t
            </Text>
            {isOverLimit && (
              <Tooltip title={`超出限制产能 ${data.limitCapacity.toFixed(1)}t`}>
                <WarningOutlined style={{ color: '#ff4d4f', fontSize: 16 }} />
              </Tooltip>
            )}
          </Space>

          {/* 轧辊状态 */}
          <Tooltip
            title={`轧辊吨位: ${data.rollCampaignProgress}t / ${data.rollChangeThreshold}t`}
          >
            <Space size={8}>
              <ToolOutlined style={{ color: getRollStatusColor(), fontSize: 16 }} />
              <Text style={{ fontFamily: FONT_FAMILIES.MONOSPACE, color: getRollStatusColor() }}>
                {data.rollCampaignProgress}t
              </Text>
            </Space>
          </Tooltip>
        </div>

        {/* 堆叠条形图 */}
        <div
          style={{
            position: 'relative',
            height: height,
            borderRadius: 4,
            overflow: 'hidden',
            border: '1px solid rgba(0, 0, 0, 0.12)',
          }}
        >
          {/* 背景网格线 */}
          <div
            style={{
              position: 'absolute',
              top: 0,
              left: 0,
              right: 0,
              bottom: 0,
              display: 'flex',
            }}
          >
            {[0, 25, 50, 75, 100].map((percent) => (
              <div
                key={percent}
                style={{
                  position: 'absolute',
                  left: `${percent}%`,
                  top: 0,
                  bottom: 0,
                  width: 1,
                  backgroundColor: 'rgba(0, 0, 0, 0.06)',
                }}
              />
            ))}
          </div>

          {/* 目标产能线 */}
          <div
            style={{
              position: 'absolute',
              left: '100%',
              top: 0,
              bottom: 0,
              width: 2,
              backgroundColor: '#1677ff',
              zIndex: 2,
            }}
          />

          {/* 限制产能线 */}
          {data.limitCapacity !== data.targetCapacity && (
            <div
              style={{
                position: 'absolute',
                left: `${(data.limitCapacity / data.targetCapacity) * 100}%`,
                top: 0,
                bottom: 0,
                width: 2,
                backgroundColor: '#ff4d4f',
                zIndex: 2,
              }}
            />
          )}

          {/* 轧辊更换标记 */}
          {data.rollChangeThreshold === ROLL_CHANGE_THRESHOLDS.WARNING && (
            <div
              style={{
                position: 'absolute',
                left: `${(ROLL_CHANGE_THRESHOLDS.WARNING / data.targetCapacity) * 100}%`,
                top: 0,
                bottom: 0,
                width: 2,
                backgroundColor: '#faad14',
                zIndex: 1,
                opacity: 0.5,
              }}
            />
          )}
          {data.rollChangeThreshold === ROLL_CHANGE_THRESHOLDS.CRITICAL && (
            <div
              style={{
                position: 'absolute',
                left: `${(ROLL_CHANGE_THRESHOLDS.CRITICAL / data.targetCapacity) * 100}%`,
                top: 0,
                bottom: 0,
                width: 2,
                backgroundColor: '#ff4d4f',
                zIndex: 1,
                opacity: 0.5,
              }}
            />
          )}

          {/* 堆叠分段 */}
          <div
            style={{
              position: 'absolute',
              top: 0,
              left: 0,
              bottom: 0,
              display: 'flex',
              width: `${utilizationPercent}%`,
            }}
          >
            {segments.map((seg, index) => {
              const color =
                URGENCY_COLORS[
                  `${seg.urgencyLevel}_${
                    seg.urgencyLevel === 'L3'
                      ? 'EMERGENCY'
                      : seg.urgencyLevel === 'L2'
                      ? 'HIGH'
                      : seg.urgencyLevel === 'L1'
                      ? 'MEDIUM'
                      : 'NORMAL'
                  }` as keyof typeof URGENCY_COLORS
                ];

              return (
                <Tooltip
                  key={index}
                  title={
                    <div>
                      <div style={{ fontWeight: 'bold' }}>{seg.urgencyLevel}</div>
                      <div>吨位: {seg.tonnage.toFixed(1)}t</div>
                      <div>材料数: {seg.materialCount} 件</div>
                    </div>
                  }
                >
                  <div
                    style={{
                      flex: seg.widthPercent,
                      backgroundColor: color,
                      cursor: 'help',
                      transition: 'opacity 0.2s',
                      display: 'flex',
                      alignItems: 'center',
                      justifyContent: 'center',
                      color: '#fff',
                      fontWeight: 'bold',
                      fontSize: 12,
                    }}
                    onMouseEnter={(e) => (e.currentTarget.style.opacity = '0.8')}
                    onMouseLeave={(e) => (e.currentTarget.style.opacity = '1')}
                  >
                    {seg.widthPercent > 10 && seg.urgencyLevel}
                  </div>
                </Tooltip>
              );
            })}
          </div>
        </div>

        {/* 图例 */}
        <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
          <Space size={16}>
            <Space size={4}>
              <div
                style={{
                  width: 12,
                  height: 12,
                  backgroundColor: URGENCY_COLORS.L3_EMERGENCY,
                  borderRadius: 2,
                }}
              />
              <Text style={{ fontSize: 12 }}>L3 紧急</Text>
            </Space>
            <Space size={4}>
              <div
                style={{
                  width: 12,
                  height: 12,
                  backgroundColor: URGENCY_COLORS.L2_HIGH,
                  borderRadius: 2,
                }}
              />
              <Text style={{ fontSize: 12 }}>L2 高</Text>
            </Space>
            <Space size={4}>
              <div
                style={{
                  width: 12,
                  height: 12,
                  backgroundColor: URGENCY_COLORS.L1_MEDIUM,
                  borderRadius: 2,
                }}
              />
              <Text style={{ fontSize: 12 }}>L1 中</Text>
            </Space>
            <Space size={4}>
              <div
                style={{
                  width: 12,
                  height: 12,
                  backgroundColor: URGENCY_COLORS.L0_NORMAL,
                  borderRadius: 2,
                }}
              />
              <Text style={{ fontSize: 12 }}>L0 正常</Text>
            </Space>
          </Space>

          <Space size={16}>
            <Space size={4}>
              <div
                style={{
                  width: 2,
                  height: 12,
                  backgroundColor: '#1677ff',
                }}
              />
              <Text style={{ fontSize: 12 }}>目标产能</Text>
            </Space>
            <Space size={4}>
              <div
                style={{
                  width: 2,
                  height: 12,
                  backgroundColor: '#ff4d4f',
                }}
              />
              <Text style={{ fontSize: 12 }}>限制产能</Text>
            </Space>
            <Space size={4}>
              <div
                style={{
                  width: 2,
                  height: 12,
                  backgroundColor: '#faad14',
                  opacity: 0.5,
                }}
              />
              <Text style={{ fontSize: 12 }}>轧辊更换</Text>
            </Space>
          </Space>
        </div>

        {/* 产能利用率进度条 */}
        <Progress
          percent={utilizationPercent}
          status={isOverLimit ? 'exception' : utilizationPercent > 90 ? 'normal' : 'active'}
          strokeColor={isOverLimit ? '#ff4d4f' : '#1677ff'}
          format={(percent) => `${percent?.toFixed(1)}%`}
        />
      </Space>
    </Card>
  );
};

// 使用 React.memo 优化，只在 props 改变时重新渲染
export const CapacityTimeline = React.memo(CapacityTimelineComponent);
