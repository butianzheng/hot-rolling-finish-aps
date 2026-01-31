/**
 * 产能概览容器 - 改进版本
 *
 * 新增功能：
 * - 支持选中物料高亮显示
 * - 日期范围与排程视图同步
 * - 与统一联动状态管理器集成
 */

import React, { useMemo, useCallback, useEffect } from 'react';
import { Space, Spin, Empty, Button } from 'antd';
import { ReloadOutlined } from '@ant-design/icons';
import dayjs, { Dayjs } from 'dayjs';
import { useCapacityTimelineContainer } from './useCapacityTimelineContainer';
import { CapacityTimeline } from '../CapacityTimeline';
import { ToolBar } from './ToolBar';
import { WorkbenchSyncState, WorkbenchSyncAPI } from '@/hooks/useWorkbenchSync';

export interface CapacityTimelineContainerProps {
  machineCode: string | null;

  // 新增：日期范围同步
  dateRange?: [Dayjs, Dayjs];
  autoDateRange?: boolean;
  onDateRangeChange?: (range: [Dayjs, Dayjs]) => void;

  // 新增：选中物料高亮
  selectedMaterialIds?: string[];
  onMaterialSelect?: (materialId: string, add: boolean) => void;
  onMaterialsSelect?: (materialIds: string[], replace: boolean) => void;

  // 新增：聚焦支持
  focusedMaterialId?: string | null;
  onFocusMaterial?: (materialId: string) => void;

  // 操作 API
  syncApi?: WorkbenchSyncAPI;
}

export const CapacityTimelineContainer: React.FC<CapacityTimelineContainerProps> = ({
  machineCode,
  dateRange: externalDateRange,
  autoDateRange = true,
  onDateRangeChange,
  selectedMaterialIds = [],
  onMaterialSelect,
  onMaterialsSelect,
  focusedMaterialId,
  onFocusMaterial,
  syncApi,
}) => {
  const {
    timelineData,
    machineOptions,
    selectedMachine,
    setSelectedMachine,
    dateRange: internalDateRange,
    setDateRange: setInternalDateRange,
    loading,
    error,
    refetch,
  } = useCapacityTimelineContainer(machineCode);

  // 选择有效的日期范围（优先使用外部传入）
  const effectiveDateRange = useMemo(() => {
    return externalDateRange || internalDateRange;
  }, [externalDateRange, internalDateRange]);

  // 当外部日期范围变化时，同步到内部状态
  useEffect(() => {
    if (externalDateRange && autoDateRange) {
      setInternalDateRange(externalDateRange);
    }
  }, [externalDateRange, autoDateRange, setInternalDateRange]);

  // 计算根据日期范围过滤的时间线数据
  const filteredTimelineData = useMemo(() => {
    if (!timelineData || !effectiveDateRange) return timelineData;

    const [start, end] = effectiveDateRange;

    return timelineData
      .map(machine => ({
        ...machine,
        data: machine.data.filter(day => {
          const dayDate = dayjs(day.date);
          return dayDate.isBetween(start, end, null, '[]');
        }),
      }))
      .filter(machine => machine.data.length > 0);
  }, [timelineData, effectiveDateRange]);

  // 处理日期范围变化
  const handleDateRangeChange = useCallback((newRange: [Dayjs, Dayjs]) => {
    setInternalDateRange(newRange);
    onDateRangeChange?.(newRange);
    syncApi?.setDateRange(newRange);
  }, [setInternalDateRange, onDateRangeChange, syncApi]);

  // 处理物料选择
  const handleMaterialSelect = useCallback((materialId: string, add: boolean) => {
    onMaterialSelect?.(materialId, add);
    if (add) {
      syncApi?.toggleMaterialSelection(materialId);
    }
  }, [onMaterialSelect, syncApi]);

  // 处理机组切换
  const handleMachineChange = useCallback((newMachine: string) => {
    setSelectedMachine(newMachine);
    syncApi?.focusMachine(newMachine);
  }, [setSelectedMachine, syncApi]);

  // 容错处理
  if (error) {
    return (
      <div style={{ padding: '20px', textAlign: 'center' }}>
        <Empty
          description="产能数据加载失败"
          style={{ marginBottom: '16px' }}
        />
        <Button
          icon={<ReloadOutlined />}
          onClick={() => refetch()}
          type="primary"
        >
          重试
        </Button>
      </div>
    );
  }

  return (
    <Spin spinning={loading} delay={200}>
      <Space direction="vertical" style={{ width: '100%' }} size="middle">
        <ToolBar
          machineCode={selectedMachine}
          onMachineChange={handleMachineChange}
          machineOptions={machineOptions}
          dateRange={effectiveDateRange}
          onDateRangeChange={handleDateRangeChange}
          autoDateRange={autoDateRange}
          onAutoDateRangeChange={(auto) => {
            if (!auto) {
              syncApi?.setDateRange(effectiveDateRange);
            } else {
              syncApi?.resetDateRangeToAuto();
            }
          }}
          onRefresh={() => refetch()}
        />

        <div style={{ overflowX: 'auto', padding: '0 8px' }}>
          {filteredTimelineData && filteredTimelineData.length > 0 ? (
            <CapacityTimeline
              data={filteredTimelineData}
              selectedMaterialIds={selectedMaterialIds}
              focusedMaterialId={focusedMaterialId}
              onMaterialSelect={handleMaterialSelect}
              onMaterialFocus={onFocusMaterial}
            />
          ) : (
            <Empty
              description={`${selectedMachine === 'all' ? '该日期范围' : `${selectedMachine}的`}无排程项`}
              style={{ marginTop: '40px' }}
            />
          )}
        </div>
      </Space>
    </Spin>
  );
};

export default CapacityTimelineContainer;
