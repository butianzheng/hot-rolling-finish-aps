/**
 * CapacityTimeline - 主组件
 *
 * 重构后：357 行 → ~85 行 (-76%)
 * 增强：产能影响预测功能
 */

import React, { useMemo } from 'react';
import { Card, Space, Typography, Tooltip, Progress, Tag } from 'antd';
import { ToolOutlined, WarningOutlined, RightOutlined } from '@ant-design/icons';
import { FONT_FAMILIES } from '../../theme';
import type { CapacityTimelineProps } from './types';
import { useCapacityTimeline } from './useCapacityTimeline';
import { StackedBarChart } from './StackedBarChart';
import { Legend } from './Legend';
import { CapacityImpactPanel } from '../CapacityImpactPanel';
import { predictRemovalImpact } from '../../services/capacityImpactService';
import type { PlanItemStatusFilter } from '../../utils/planItemStatus';

const { Text, Title } = Typography;

const CapacityTimelineComponent: React.FC<CapacityTimelineProps> = ({
  data,
  height = 120,
  selectedMaterialIds = [],
  focusedMaterialId,
  materials = [],
  onOpenScheduleCell,
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

  const statusSummary = data.statusSummary;
  const adjustableCount = useMemo(() => {
    if (!statusSummary) return 0;
    return Math.max(0, statusSummary.totalCount - statusSummary.lockedInPlanCount);
  }, [statusSummary]);

  const openScheduleCell = (options?: { statusFilter?: PlanItemStatusFilter }) => {
    onOpenScheduleCell?.(data.machineCode, data.date, materialIds, options);
  };

  const clickable = !!onOpenScheduleCell;

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
            <Title
              level={5}
              style={{ margin: 0 }}
            >
              {data.date} - {data.machineCode}
            </Title>
            <Text type="secondary" style={{ fontFamily: FONT_FAMILIES.MONOSPACE }}>
              {data.actualCapacity.toFixed(2)}t / {data.targetCapacity.toFixed(2)}t
            </Text>
            {isOverLimit && (
              <Tooltip title={`超出限制产能 ${data.limitCapacity.toFixed(2)}t`}>
                <WarningOutlined style={{ color: '#ff4d4f', fontSize: 16 }} />
              </Tooltip>
            )}
            {statusSummary ? (
              <Space size={6}>
                <Tag
                  color="blue"
                  style={{ cursor: clickable ? 'pointer' : undefined }}
                  onClick={() => clickable && openScheduleCell({ statusFilter: 'ALL' })}
                  title={`已排 ${statusSummary.totalCount} 件 / ${statusSummary.totalWeightT.toFixed(2)}t${clickable ? '（点击快筛并打开明细）' : ''}`}
                >
                  已排 {statusSummary.totalCount}
                </Tag>
                {statusSummary.lockedInPlanCount > 0 ? (
                  <Tag
                    color="purple"
                    style={{ cursor: clickable ? 'pointer' : undefined }}
                    onClick={() => clickable && openScheduleCell({ statusFilter: 'LOCKED' })}
                    title={`冻结 ${statusSummary.lockedInPlanCount} 件 / ${statusSummary.lockedInPlanWeightT.toFixed(2)}t${clickable ? '（点击快筛并打开明细）' : ''}`}
                  >
                    冻结 {statusSummary.lockedInPlanCount}
                  </Tag>
                ) : null}
                {statusSummary.forceReleaseCount > 0 ? (
                  <Tag
                    color="red"
                    style={{ cursor: clickable ? 'pointer' : undefined }}
                    onClick={() => clickable && openScheduleCell({ statusFilter: 'FORCE_RELEASE' })}
                    title={`强制放行 ${statusSummary.forceReleaseCount} 件 / ${statusSummary.forceReleaseWeightT.toFixed(2)}t${clickable ? '（点击快筛并打开明细）' : ''}`}
                  >
                    强放 {statusSummary.forceReleaseCount}
                  </Tag>
                ) : null}
                {adjustableCount > 0 ? (
                  <Tag
                    color="green"
                    style={{ cursor: clickable ? 'pointer' : undefined }}
                    onClick={() => clickable && openScheduleCell({ statusFilter: 'ADJUSTABLE' })}
                    title={`可调（非冻结）${adjustableCount} 件${clickable ? '（点击快筛并打开明细）' : ''}`}
                  >
                    可调 {adjustableCount}
                  </Tag>
                ) : null}
              </Space>
            ) : null}
          </Space>

          <Space size={12} align="center">
            {clickable ? (
              <Text type="secondary" style={{ fontSize: 12, cursor: 'pointer' }} onClick={() => openScheduleCell()}>
                同日明细 <RightOutlined />
              </Text>
            ) : null}

            {/* 轧辊状态 */}
            <Tooltip title={`轧辊吨位: ${data.rollCampaignProgress.toFixed(2)}t / ${data.rollChangeThreshold.toFixed(2)}t`}>
              <Space size={8}>
                <ToolOutlined style={{ color: rollStatusColor, fontSize: 16 }} />
                <Text style={{ fontFamily: FONT_FAMILIES.MONOSPACE, color: rollStatusColor }}>
                  {data.rollCampaignProgress.toFixed(2)}t
                </Text>
              </Space>
            </Tooltip>
          </Space>
        </div>

        {/* 产能影响预测面板（选中物料时显示） */}
        {capacityImpact && <CapacityImpactPanel prediction={capacityImpact} compact />}

        {/* 堆叠条形图（点击联动甘特同日明细） */}
        <div
          onClick={() => clickable && openScheduleCell()}
          style={{ cursor: clickable ? 'pointer' : undefined }}
          title={clickable ? '点击查看该机组/日期的排程明细（甘特图）' : undefined}
        >
          <StackedBarChart
            data={data}
            segments={segments}
            utilizationPercent={utilizationPercent}
            height={height}
          />
        </div>

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
