/**
 * CapacityTimeline - 主组件
 *
 * 重构后：357 行 → ~85 行 (-76%)
 */

import React from 'react';
import { Card, Space, Typography, Tooltip, Progress } from 'antd';
import { ToolOutlined, WarningOutlined } from '@ant-design/icons';
import { FONT_FAMILIES } from '../../theme';
import type { CapacityTimelineProps } from './types';
import { useCapacityTimeline } from './useCapacityTimeline';
import { StackedBarChart } from './StackedBarChart';
import { Legend } from './Legend';

const { Text, Title } = Typography;

const CapacityTimelineComponent: React.FC<CapacityTimelineProps> = ({ data, height = 120 }) => {
  const {
    utilizationPercent,
    isOverLimit,
    rollStatusColor,
    segments,
  } = useCapacityTimeline(data);

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
              <ToolOutlined style={{ color: rollStatusColor, fontSize: 16 }} />
              <Text style={{ fontFamily: FONT_FAMILIES.MONOSPACE, color: rollStatusColor }}>
                {data.rollCampaignProgress}t
              </Text>
            </Space>
          </Tooltip>
        </div>

        {/* 堆叠条形图 */}
        <StackedBarChart
          data={data}
          segments={segments}
          utilizationPercent={utilizationPercent}
          height={height}
        />

        {/* 图例 */}
        <Legend />

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

export default CapacityTimeline;
