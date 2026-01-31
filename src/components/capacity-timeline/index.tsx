/**
 * CapacityTimeline - 主组件
 *
 * 重构后：357 行 → ~85 行 (-76%)
 * 增强：产能影响预测功能
 */

import React, { useMemo } from 'react';
import { Card, Space, Typography, Tooltip, Progress } from 'antd';
import { ToolOutlined, WarningOutlined } from '@ant-design/icons';
import { FONT_FAMILIES } from '../../theme';
import type { CapacityTimelineProps } from './types';
import { useCapacityTimeline } from './useCapacityTimeline';
import { StackedBarChart } from './StackedBarChart';
import { Legend } from './Legend';
import { CapacityImpactPanel } from '../CapacityImpactPanel';
import { predictRemovalImpact } from '../../services/capacityImpactService';

const { Text, Title } = Typography;

const CapacityTimelineComponent: React.FC<CapacityTimelineProps> = ({
  data,
  height = 120,
  selectedMaterialIds = [],
  focusedMaterialId,
  materials = [],
}) => {
  const {
    utilizationPercent,
    isOverLimit,
    rollStatusColor,
    segments,
  } = useCapacityTimeline(data);

  // 检查该日期是否包含选中的物料
  const materialIds = data.materialIds || [];
  const hasSelectedMaterial = selectedMaterialIds.some(id => materialIds.includes(id));
  const hasFocusedMaterial = focusedMaterialId && materialIds.includes(focusedMaterialId);

  // 计算产能影响预测（仅在有选中物料且存在于该时间线时计算）
  const capacityImpact = useMemo(() => {
    if (!hasSelectedMaterial || selectedMaterialIds.length === 0) {
      return null;
    }

    // 过滤出在该时间线中的选中物料
    const selectedInThisTimeline = materials.filter(
      m =>
        selectedMaterialIds.includes(m.material_id) &&
        materialIds.includes(m.material_id)
    );

    if (selectedInThisTimeline.length === 0) {
      return null;
    }

    return predictRemovalImpact(data, selectedInThisTimeline);
  }, [hasSelectedMaterial, selectedMaterialIds, materials, materialIds, data]);

  return (
    <Card
      size="small"
      style={{
        marginBottom: 16,
        borderRadius: 8,
        // 选中状态：添加蓝色边框
        border: hasSelectedMaterial ? '2px solid #1890ff' : undefined,
        // 聚焦状态：添加阴影
        boxShadow: hasFocusedMaterial
          ? '0 0 8px rgba(24, 144, 255, 0.6)'
          : hasSelectedMaterial
          ? '0 0 4px rgba(24, 144, 255, 0.3)'
          : undefined,
        // 选中状态：添加背景色
        backgroundColor: hasSelectedMaterial ? 'rgba(24, 144, 255, 0.05)' : undefined,
        transition: 'all 0.2s ease',
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

        {/* 产能影响预测面板（选中物料时显示） */}
        {capacityImpact && <CapacityImpactPanel prediction={capacityImpact} compact />}

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
