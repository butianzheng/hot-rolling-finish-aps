/**
 * 策略编辑弹窗组件
 */

import React from 'react';
import { Alert, Button, Card, Form, Input, InputNumber, Modal, Select, Space, Typography } from 'antd';
import type { FormInstance } from 'antd';
import type { ModalMode, StrategyPresetRow } from './types';

interface StrategyFormModalProps {
  open: boolean;
  mode: ModalMode;
  saving: boolean;
  form: FormInstance;
  baseStrategyOptions: { value: string; label: string }[];
  presetsByKey: Record<string, StrategyPresetRow>;
  onSave: () => void;
  onCancel: () => void;
}

export const StrategyFormModal: React.FC<StrategyFormModalProps> = ({
  open,
  mode,
  saving,
  form,
  baseStrategyOptions,
  presetsByKey,
  onSave,
  onCancel,
}) => {
  const title = mode === 'edit'
    ? '编辑自定义策略'
    : mode === 'copy'
      ? '复制自定义策略'
    : '新建自定义策略';

  const baseStrategyKey = Form.useWatch('base_strategy', form);
  const basePreset = presetsByKey?.[String(baseStrategyKey || '').trim()];
  const defaultParameters = basePreset?.default_parameters ?? null;
  const parameterTemplate = (defaultParameters as any)?.parameter_template;
  const canApplyTemplate = parameterTemplate && typeof parameterTemplate === 'object';

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
          label="策略编号（仅字母、数字、下划线、短横线）"
          name="strategy_id"
          rules={[{ required: true, message: '请输入策略编号' }]}
        >
          <Input disabled={mode === 'edit'} placeholder="例如：自定义_均衡方案_01" />
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

        <Card
          size="small"
          title="预设参数（只读）"
          extra={(
            <Button
              size="small"
              disabled={!canApplyTemplate}
              onClick={() => {
                if (!canApplyTemplate) return;
                form.setFieldsValue({ parameters: parameterTemplate });
              }}
            >
              填入模板
            </Button>
          )}
          style={{ marginBottom: 12 }}
        >
          <Typography.Text type="secondary">
            {basePreset?.title || String(baseStrategyKey || '') || '—'}
            {basePreset?.description ? `：${basePreset.description}` : ''}
          </Typography.Text>
          <pre style={{
            marginTop: 8,
            padding: 8,
            borderRadius: 6,
            background: '#fafafa',
            border: '1px solid #f0f0f0',
            maxHeight: 220,
            overflow: 'auto',
            fontSize: 12,
          }}
          >
            {defaultParameters ? JSON.stringify(defaultParameters, null, 2) : '—'}
          </pre>
        </Card>

        <Card size="small" title="参数（综合评分：分数越高越优先）" style={{ marginBottom: 12 }}>
          <Alert
            type="info"
            showIcon
            message="说明：参数用于“等级内排序”的加权评分"
            description={
              <div>
                <div>
                  评分公式（简化）：紧急权重×紧急等级 + 重量权重×吨位 + 冷坨权重×库龄 + 交期权重×交期紧迫度 +
                  出炉时长权重×出炉时长。
                </div>
                <div style={{ marginTop: 4 }}>
                  不同项单位不同（吨/天），建议从小步调参；同分时会回落到“基于预设策略”的排序规则以保持稳定。
                </div>
              </div>
            }
            style={{ marginBottom: 12 }}
          />
          <Space wrap size={12}>
            <Form.Item
              label={
                <span>
                  紧急权重 <Typography.Text type="secondary">（紧急参数）</Typography.Text>
                </span>
              }
              tooltip="0~100；值越大越优先排入更高紧急等级（二级/三级）的材料。"
              name={['parameters', 'urgent_weight']}
            >
              <InputNumber min={0} max={100} style={{ width: 140 }} />
            </Form.Item>
            <Form.Item
              label={
                <span>
                  重量权重 <Typography.Text type="secondary">（吨位参数）</Typography.Text>
                </span>
              }
              tooltip="0~100；按材料吨位加权，值越大越倾向优先排“大重量”材料（更容易填满产能）。"
              name={['parameters', 'capacity_weight']}
            >
              <InputNumber min={0} max={100} style={{ width: 140 }} />
            </Form.Item>
            <Form.Item
              label={
                <span>
                  冷坨权重 <Typography.Text type="secondary">（冷库参数）</Typography.Text>
                </span>
              }
              tooltip="0~100；按库龄（天）加权，值越大越优先消化库龄更大的材料。"
              name={['parameters', 'cold_stock_weight']}
            >
              <InputNumber min={0} max={100} style={{ width: 160 }} />
            </Form.Item>
            <Form.Item
              label={
                <span>
                  交期权重 <Typography.Text type="secondary">（交期参数）</Typography.Text>
                </span>
              }
              tooltip="0~100；按交期紧迫度加权（越临期/逾期越优先）。"
              name={['parameters', 'due_date_weight']}
            >
              <InputNumber min={0} max={100} style={{ width: 150 }} />
            </Form.Item>
            <Form.Item
              label={
                <span>
                  出炉时长权重{' '}
                  <Typography.Text type="secondary">（出炉时长参数）</Typography.Text>
                </span>
              }
              tooltip="0~100；按出炉时长（天）加权，值越大越优先排“出炉更久”的材料。"
              name={['parameters', 'rolling_output_age_weight']}
            >
              <InputNumber min={0} max={100} style={{ width: 210 }} />
            </Form.Item>
            <Form.Item
              label={
                <span>
                  冷坨起算阈值（天）{' '}
                  <Typography.Text type="secondary">（冷库阈值参数）</Typography.Text>
                </span>
              }
              tooltip="0~365；库龄低于该阈值的材料不计入“冷坨”评分（可避免轻微库龄扰动排序）。"
              name={['parameters', 'cold_stock_age_threshold_days']}
            >
              <InputNumber min={0} max={365} style={{ width: 240 }} />
            </Form.Item>
            <Form.Item
              label={
                <span>
                  产能溢出容忍（0~1）{' '}
                  <Typography.Text type="secondary">（溢出容忍参数）</Typography.Text>
                </span>
              }
              tooltip="预留参数：当前版本暂未参与排产计算；0.05 表示容忍 5% 溢出。"
              name={['parameters', 'overflow_tolerance_pct']}
            >
              <InputNumber min={0} max={1} step={0.01} style={{ width: 210 }} />
            </Form.Item>
          </Space>
        </Card>

        <Form.Item
          label="保存原因（用于审计）"
          name="reason"
          rules={[{ required: true, message: '请输入保存原因' }]}
        >
          <Input.TextArea rows={2} placeholder="例如：为保障三级订单，提升紧急权重" />
        </Form.Item>
      </Form>
    </Modal>
  );
};
