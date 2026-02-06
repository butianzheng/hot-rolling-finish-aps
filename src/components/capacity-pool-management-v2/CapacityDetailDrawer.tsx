/**
 * 产能详情抽屉
 * 职责：显示选中日期的产能详情并支持快速调整
 */

import React, { useState, useEffect } from 'react';
import {
  Drawer,
  Descriptions,
  Form,
  InputNumber,
  Input,
  Button,
  Space,
  Tag,
  message,
  Progress,
} from 'antd';
import { EditOutlined } from '@ant-design/icons';
import { capacityApi } from '../../api/tauri';
import type { CapacityPoolCalendarData } from '../../api/ipcSchemas/machineConfigSchemas';
import { formatNumber } from '../../utils/formatters';

export interface CapacityDetailDrawerProps {
  open: boolean;
  onClose: () => void;
  versionId: string;
  data: CapacityPoolCalendarData | null;
  onUpdated?: () => void;
}

export const CapacityDetailDrawer: React.FC<CapacityDetailDrawerProps> = ({
  open,
  onClose,
  versionId,
  data,
  onUpdated,
}) => {
  const [form] = Form.useForm();
  const [editing, setEditing] = useState(false);
  const [submitting, setSubmitting] = useState(false);

  // 重置表单
  useEffect(() => {
    if (data) {
      form.setFieldsValue({
        target_capacity_t: data.target_capacity_t,
        limit_capacity_t: data.limit_capacity_t,
      });
      setEditing(false);
    }
  }, [data, form]);

  // 提交调整
  const handleSubmit = async () => {
    try {
      const values = await form.validateFields();
      const reason = values.reason?.trim() || '日历视图手动调整';

      setSubmitting(true);
      await capacityApi.updateCapacityPool(
        data!.machine_code,
        data!.plan_date,
        values.target_capacity_t,
        values.limit_capacity_t,
        reason,
        'system', // TODO: 从用户上下文获取
        versionId
      );

      message.success('产能调整成功');
      setEditing(false);
      onUpdated?.();
    } catch (e: any) {
      console.error('【产能详情抽屉】更新失败：', e);
      message.error(e?.message || '调整失败');
    } finally {
      setSubmitting(false);
    }
  };

  if (!data) return null;

  // 计算利用率百分比
  const utilizationPercent = Math.min(
    (data.used_capacity_t / data.target_capacity_t) * 100,
    150
  );

  // 判断状态
  const getStatus = () => {
    const util = data.utilization_pct;
    if (util === 0) return { text: '无排产', color: 'default' };
    if (util < 0.7) return { text: '充裕', color: 'success' };
    if (util < 0.85) return { text: '适中', color: 'processing' };
    if (util <= 1.0) return { text: '紧张', color: 'warning' };
    return { text: '超限', color: 'error' };
  };

  const status = getStatus();

  return (
    <Drawer
      title={`产能详情 - ${data.plan_date}`}
      open={open}
      onClose={onClose}
      width={480}
      extra={
        !editing && (
          <Button
            type="primary"
            icon={<EditOutlined />}
            size="small"
            onClick={() => setEditing(true)}
          >
            快速调整
          </Button>
        )
      }
    >
      <Space direction="vertical" style={{ width: '100%' }} size={16}>
        {/* 状态标签 */}
        <div>
          <Tag color={status.color} style={{ fontSize: 14, padding: '4px 12px' }}>
            {status.text}
          </Tag>
          <Tag color="blue">{data.machine_code}</Tag>
        </div>

        {/* 利用率进度条 */}
        <Progress
          percent={parseFloat(utilizationPercent.toFixed(2))}
          status={data.utilization_pct > 1 ? 'exception' : 'normal'}
          strokeColor={data.color}
        />

        {/* 详情信息 */}
        <Descriptions column={1} bordered size="small">
          <Descriptions.Item label="目标产能">
            {formatNumber(data.target_capacity_t, 3)} 吨
          </Descriptions.Item>
          <Descriptions.Item label="极限产能">
            {formatNumber(data.limit_capacity_t, 3)} 吨
          </Descriptions.Item>
          <Descriptions.Item label="已用产能">
            {formatNumber(data.used_capacity_t, 3)} 吨
          </Descriptions.Item>
          <Descriptions.Item label="剩余产能">
            {formatNumber(Math.max(0, data.target_capacity_t - data.used_capacity_t), 3)} 吨
          </Descriptions.Item>
          <Descriptions.Item label="利用率">
            {formatNumber(data.utilization_pct * 100, 2)}%
          </Descriptions.Item>
          {data.used_capacity_t > data.limit_capacity_t && (
            <Descriptions.Item label="超限">
              <Tag color="red">
                超出 {formatNumber(data.used_capacity_t - data.limit_capacity_t, 3)} 吨
              </Tag>
            </Descriptions.Item>
          )}
        </Descriptions>

        {/* 调整表单 */}
        {editing && (
          <Form form={form} layout="vertical">
            <Form.Item
              label="目标产能（吨）"
              name="target_capacity_t"
              rules={[{ required: true, message: '请输入目标产能' }]}
            >
              <InputNumber
                style={{ width: '100%' }}
                precision={3}
                step={10}
              />
            </Form.Item>

            <Form.Item
              label="极限产能（吨）"
              name="limit_capacity_t"
              rules={[
                { required: true, message: '请输入极限产能' },
                ({ getFieldValue }) => ({
                  validator(_, value) {
                    if (!value || value >= getFieldValue('target_capacity_t')) {
                      return Promise.resolve();
                    }
                    return Promise.reject(new Error('极限产能必须 ≥ 目标产能'));
                  },
                }),
              ]}
            >
              <InputNumber
                style={{ width: '100%' }}
                precision={3}
                step={10}
              />
            </Form.Item>

            <Form.Item
              label="调整原因（可选）"
              name="reason"
            >
              <Input.TextArea
                rows={2}
                placeholder="例如：根据实际排产需求调整"
              />
            </Form.Item>

            <Form.Item>
              <Space>
                <Button
                  type="primary"
                  onClick={handleSubmit}
                  loading={submitting}
                >
                  保存
                </Button>
                <Button onClick={() => setEditing(false)}>
                  取消
                </Button>
              </Space>
            </Form.Item>
          </Form>
        )}
      </Space>
    </Drawer>
  );
};

export default CapacityDetailDrawer;
