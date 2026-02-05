/**
 * 预览/试算弹窗组件
 */

import React from 'react';
import { Alert, Button, DatePicker, InputNumber, Modal, Select, Space, Typography } from 'antd';
import type { Dayjs } from 'dayjs';
import type { OptimizeStrategy, SimulateResult } from './types';
import { STRATEGY_OPTIONS } from './types';

interface PreviewModalProps {
  open: boolean;
  strategyLabel: string;
  strategy: OptimizeStrategy;
  baseDate: Dayjs;
  windowDaysOverride: number | null;
  simulateLoading: boolean;
  executeLoading: boolean;
  simulateResult: SimulateResult | null;
  activeVersionId: string | null;
  onClose: () => void;
  onExecute: () => void;
  onSimulate: () => void;
  onBaseDateChange: (date: Dayjs) => void;
  onStrategyChange: (strategy: OptimizeStrategy) => void;
  onWindowDaysOverrideChange: (v: number | null) => void;
}

export const PreviewModal: React.FC<PreviewModalProps> = ({
  open,
  strategyLabel,
  strategy,
  baseDate,
  windowDaysOverride,
  simulateLoading,
  executeLoading,
  simulateResult,
  activeVersionId,
  onClose,
  onExecute,
  onSimulate,
  onBaseDateChange,
  onStrategyChange,
  onWindowDaysOverrideChange,
}) => {
  return (
    <Modal
      title={`一键优化 - ${strategyLabel}`}
      open={open}
      onCancel={onClose}
      onOk={onExecute}
      okText="执行重算"
      okButtonProps={{ disabled: !activeVersionId }}
      confirmLoading={executeLoading}
    >
      <Space direction="vertical" style={{ width: '100%' }} size={12}>
        <Alert
          type="info"
          showIcon
          message="说明"
          description={
            <div>
              <div>
                试算（simulate_recalc）仅返回排产数量等摘要，不落库、不写日志；执行重算会落库并触发
                plan_updated 事件。
              </div>
              <div style={{ marginTop: 4 }}>
                当前策略：{strategyLabel}；窗口天数：
                {windowDaysOverride != null ? String(windowDaysOverride) : '跟随版本配置'}
              </div>
            </div>
          }
        />

        <Space wrap>
          <span>策略</span>
          <Select
            value={strategy}
            onChange={(v) => onStrategyChange(v as OptimizeStrategy)}
            style={{ minWidth: 160 }}
            options={STRATEGY_OPTIONS}
          />
        </Space>

        <Space wrap>
          <span>窗口天数</span>
          <InputNumber
            min={1}
            max={60}
            value={windowDaysOverride ?? undefined}
            onChange={(v) => onWindowDaysOverrideChange(v == null ? null : Number(v))}
            placeholder="跟随版本"
            style={{ width: 160 }}
          />
          <Button size="small" onClick={() => onWindowDaysOverrideChange(null)}>
            跟随版本
          </Button>
        </Space>

        <Space wrap>
          <span>基准日期</span>
          <DatePicker
            value={baseDate}
            onChange={(d) => d && onBaseDateChange(d)}
            format="YYYY-MM-DD"
          />
          <Button loading={simulateLoading} onClick={onSimulate}>
            试算预览
          </Button>
        </Space>

        {simulateResult ? (
          <Alert
            type="success"
            showIcon
            message={String(simulateResult?.message || '试算完成')}
            description={
              <Space size={12} wrap>
                <Typography.Text type="secondary">
                  排产数量: {Number(simulateResult?.plan_items_count ?? 0)}
                </Typography.Text>
                <Typography.Text type="secondary">
                  冻结数量: {Number(simulateResult?.frozen_items_count ?? 0)}
                </Typography.Text>
              </Space>
            }
          />
        ) : (
          <Typography.Text type="secondary" style={{ fontSize: 12 }}>
            点击"试算预览"查看摘要；如需KPI/风险变化对比，需要后端补充影响分析数据结构。
          </Typography.Text>
        )}
      </Space>
    </Modal>
  );
};
