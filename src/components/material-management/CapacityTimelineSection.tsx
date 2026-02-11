/**
 * 产能时间线区块
 */

import React from 'react';
import { Alert, Button, Collapse, DatePicker, Empty, Select, Space, Spin } from 'antd';
import { ReloadOutlined } from '@ant-design/icons';
import type { Dayjs } from 'dayjs';
import { CapacityTimeline } from '../CapacityTimeline';
import type { CapacityTimelineData } from '../../types/capacity';
import type { OpenScheduleCellOptions } from '../capacity-timeline/types';

export interface CapacityTimelineSectionProps {
  machineOptions: Array<{ label: string; value: string }>;
  timelineMachine: string;
  timelineDate: Dayjs;
  timelineData: CapacityTimelineData[];
  timelineLoading: boolean;
  timelineError: string | null;
  activeVersionId: string | null;
  onMachineChange: (machine: string) => void;
  onDateChange: (date: Dayjs) => void;
  onReload: () => void;
  onOpenScheduleCell?: (
    machineCode: string,
    date: string,
    materialIds: string[],
    options?: OpenScheduleCellOptions
  ) => void;
}

export const CapacityTimelineSection: React.FC<CapacityTimelineSectionProps> = ({
  machineOptions,
  timelineMachine,
  timelineDate,
  timelineData,
  timelineLoading,
  timelineError,
  activeVersionId,
  onMachineChange,
  onDateChange,
  onReload,
  onOpenScheduleCell,
}) => {
  return (
    <Collapse
      defaultActiveKey={['capacity']}
      style={{ marginBottom: 16 }}
      items={[
        {
          key: 'capacity',
          label: '产能时间线',
          children: (
            <div>
              <Space style={{ marginBottom: 12 }} size={12} wrap>
                <span>机组</span>
                <Select
                  value={timelineMachine}
                  style={{ width: 160 }}
                  placeholder="请选择机组"
                  options={[{ label: '全部机组', value: 'all' }, ...machineOptions]}
                  showSearch
                  optionFilterProp="label"
                  onChange={(value) => onMachineChange(value)}
                />
                <span>日期</span>
                <DatePicker
                  value={timelineDate}
                  onChange={(d) => d && onDateChange(d)}
                  format="YYYY-MM-DD"
                  allowClear={false}
                />
                <Button icon={<ReloadOutlined />} onClick={onReload} loading={timelineLoading}>
                  刷新
                </Button>
              </Space>

              {!activeVersionId ? (
                <Alert
                  message="产能时间线需要激活排产版本"
                  description={'请先在"排产方案"中激活一个版本后再查看排产产能分布。'}
                  type="warning"
                  showIcon
                />
              ) : timelineError ? (
                <Alert message="产能时间线加载失败" description={timelineError} type="error" showIcon />
              ) : (
                <Spin spinning={timelineLoading}>
                  <div style={{ minHeight: 80 }}>
                    {timelineData.length > 0 ? (
                      <Space direction="vertical" style={{ width: '100%' }} size={8}>
                        {timelineData.map((row) => (
                          <CapacityTimeline
                            key={`${row.machineCode}__${row.date}`}
                            data={row}
                            onOpenScheduleCell={onOpenScheduleCell}
                          />
                        ))}
                      </Space>
                    ) : (
                      <Empty description="暂无产能时间线数据" />
                    )}
                  </div>
                </Spin>
              )}
            </div>
          ),
        },
      ]}
    />
  );
};

export default CapacityTimelineSection;
