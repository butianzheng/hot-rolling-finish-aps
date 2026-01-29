/**
 * 策略编辑弹窗组件
 */

import React from 'react';
import { Card, Form, Input, InputNumber, Modal, Select, Space } from 'antd';
import type { FormInstance } from 'antd';
import type { ModalMode } from './types';

interface StrategyFormModalProps {
  open: boolean;
  mode: ModalMode;
  saving: boolean;
  form: FormInstance;
  baseStrategyOptions: { value: string; label: string }[];
  onSave: () => void;
  onCancel: () => void;
}

export const StrategyFormModal: React.FC<StrategyFormModalProps> = ({
  open,
  mode,
  saving,
  form,
  baseStrategyOptions,
  onSave,
  onCancel,
}) => {
  const title = mode === 'edit'
    ? '编辑自定义策略'
    : mode === 'copy'
      ? '复制自定义策略'
      : '新建自定义策略';

  return (
    <Modal
      title={title}
      open={open}
      onCancel={onCancel}
      onOk={onSave}
      okText="保存"
      cancelText="取消"
      confirmLoading={saving}
      width={720}
    >
      <Form form={form} layout="vertical">
        <Form.Item
          label="策略ID（仅字母/数字/_/-）"
          name="strategy_id"
          rules={[{ required: true, message: '请输入策略ID' }]}
        >
          <Input disabled={mode === 'edit'} placeholder="例如：custom_balanced_ab12cd" />
        </Form.Item>

        <Form.Item
          label="策略名称"
          name="title"
          rules={[{ required: true, message: '请输入策略名称' }]}
        >
          <Input placeholder="例如：均衡方案-偏紧急" />
        </Form.Item>

        <Form.Item label="说明" name="description">
          <Input.TextArea rows={2} placeholder="可选：说明该策略适用场景/注意事项" />
        </Form.Item>

        <Form.Item
          label="基于预设策略"
          name="base_strategy"
          rules={[{ required: true, message: '请选择基于策略' }]}
        >
          <Select options={baseStrategyOptions} style={{ width: 240 }} />
        </Form.Item>

        <Card size="small" title="参数（当前仅保存展示）" style={{ marginBottom: 12 }}>
          <Space wrap size={12}>
            <Form.Item label="urgent_weight" name={['parameters', 'urgent_weight']}>
              <InputNumber min={0} max={100} style={{ width: 140 }} />
            </Form.Item>
            <Form.Item label="capacity_weight" name={['parameters', 'capacity_weight']}>
              <InputNumber min={0} max={100} style={{ width: 140 }} />
            </Form.Item>
            <Form.Item label="cold_stock_weight" name={['parameters', 'cold_stock_weight']}>
              <InputNumber min={0} max={100} style={{ width: 160 }} />
            </Form.Item>
            <Form.Item label="due_date_weight" name={['parameters', 'due_date_weight']}>
              <InputNumber min={0} max={100} style={{ width: 150 }} />
            </Form.Item>
            <Form.Item label="rolling_output_age_weight" name={['parameters', 'rolling_output_age_weight']}>
              <InputNumber min={0} max={100} style={{ width: 210 }} />
            </Form.Item>
            <Form.Item label="cold_stock_age_threshold_days" name={['parameters', 'cold_stock_age_threshold_days']}>
              <InputNumber min={0} max={365} style={{ width: 240 }} />
            </Form.Item>
            <Form.Item label="overflow_tolerance_pct (0~1)" name={['parameters', 'overflow_tolerance_pct']}>
              <InputNumber min={0} max={1} step={0.01} style={{ width: 210 }} />
            </Form.Item>
          </Space>
        </Card>

        <Form.Item
          label="保存原因（用于审计）"
          name="reason"
          rules={[{ required: true, message: '请输入保存原因' }]}
        >
          <Input.TextArea rows={2} placeholder="例如：为保障 L3 订单，提升紧急权重" />
        </Form.Item>
      </Form>
    </Modal>
  );
};
