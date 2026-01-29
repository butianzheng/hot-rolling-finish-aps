/**
 * CapacityTimelineContainer - 主组件
 *
 * 重构后：253 行 → ~55 行 (-78%)
 */

import React from 'react';
import { Alert, Empty, Space, Spin } from 'antd';
import { CapacityTimeline } from '../CapacityTimeline';
import type { CapacityTimelineContainerProps } from './types';
import { useCapacityTimelineContainer } from './useCapacityTimelineContainer';
import { ToolBar } from './ToolBar';

export const CapacityTimelineContainer: React.FC<CapacityTimelineContainerProps> = ({ machineCode }) => {
  const state = useCapacityTimelineContainer(machineCode);

  return (
    <div>
      {/* 工具栏 */}
      <ToolBar
        dateRange={state.dateRange}
        onDateRangeChange={state.setDateRange}
        selectedMachine={state.selectedMachine}
        onMachineChange={state.setSelectedMachine}
        machineOptions={state.machineOptions}
        onRefresh={state.loadTimelineData}
        loading={state.loading}
      />

      {/* 无版本提示 */}
      {!state.activeVersionId && (
        <Alert
          message="请先激活排产版本"
          description="产能时间线依赖排产版本数据，激活版本后可查看多天产能分布。"
          type="warning"
          showIcon
        />
      )}

      {/* 时间线列表 */}
      {state.timelineData.length === 0 ? (
        <Empty description="暂无数据" />
      ) : (
        <Spin spinning={state.loading}>
          <Space direction="vertical" style={{ width: '100%' }} size={0}>
            {state.timelineData.map((data, index) => (
              <CapacityTimeline key={`${data.machineCode}__${data.date}__${index}`} data={data} />
            ))}
          </Space>
        </Spin>
      )}
    </div>
  );
};

export default CapacityTimelineContainer;
