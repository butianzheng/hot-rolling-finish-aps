/**
 * 批量调整模态框
 * 职责：支持选中多个日期进行批量产能调整
 */

import React, { useEffect } from 'react';
import {
  Modal,
  Form,
  InputNumber,
  Input,
  Space,
  Tag,
  Alert,
  message,
} from 'antd';
import { capacityApi } from '../../api/tauri';

export interface BatchAdjustModalProps {
  open: boolean;
  onClose: () => void;
  versionId: string;
  machineCode: string;
  selectedDates: string[];
  onUpdated?: () => void;
}

export const BatchAdjustModal: React.FC<BatchAdjustModalProps> = ({
  open,
  onClose,
  versionId,
  machineCode,
  selectedDates,
  onUpdated,
}) => {
  const [form] = Form.useForm();
  const [submitting, setSubmitting] = React.useState(false);

  // 重置表单
  useEffect(() => {
    if (open) {
      form.resetFields();
    }
  }, [open, form]);

  // 提交批量调整
  const handleSubmit = async () => {
    try {
      const values = await form.validateFields();
      const reason = values.reason?.trim() || '批量调整';

      // 构造批量更新请求
      const updates = selectedDates.map((date) => ({
        machine_code: machineCode,
        plan_date: date,
        target_capacity_t: values.target_capacity_t,
        limit_capacity_t: values.limit_capacity_t,
      }));

      setSubmitting(true);
      const response = await capacityApi.batchUpdateCapacityPools(
        updates,
        reason,
        'system', // TODO: 从用户上下文获取
        versionId
      );

      message.success(
        `批量调整完成：更新 ${response.updated} 条，跳过 ${response.skipped} 条`
      );
      onClose();
      onUpdated?.();
    } catch (e: any) {
      console.error('【批量调整弹窗】更新失败：', e);
      message.error(e?.message || '批量调整失败');
    } finally {
      setSubmitting(false);
    }
  };

  return (
    <Modal
      title="批量调整产能"
      open={open}
      onCancel={onClose}
      onOk={handleSubmit}
      confirmLoading={submitting}
      width={500}
      okText="确认调整"
      cancelText="取消"
    >
      <Space direction="vertical" style={{ width: '100%' }} size={16}>
        {/* 选中日期信息 */}
        <Alert
          type="info"
          message={`已选择 ${selectedDates.length} 个日期`}
          description={
            <div style={{ marginTop: 8 }}>
              <Space wrap>
                {selectedDates.slice(0, 10).map((date) => (
                  <Tag key={date}>{date}</Tag>
                ))}
                {selectedDates.length > 10 && (
                  <Tag>... 还有 {selectedDates.length - 10} 个</Tag>
                )}
              </Space>
            </div>
          }
        />

        {/* 调整表单 */}
        <Form form={form} layout="vertical">
          <Form.Item
            label="目标产能 (吨/天)"
            name="target_capacity_t"
            rules={[
              { required: true, message: '请输入目标产能' },
              { type: 'number', min: 100, message: '目标产能必须 ≥ 100' },
            ]}
          >
            <InputNumber
              style={{ width: '100%' }}
              precision={3}
              step={10}
              placeholder="例如: 1200.000"
            />
          </Form.Item>

          <Form.Item
            label="极限产能 (吨/天)"
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
            tooltip="极限产能通常为目标产能的 105%-120%"
          >
            <InputNumber
              style={{ width: '100%' }}
              precision={3}
              step={10}
              placeholder="例如: 1260.000"
            />
          </Form.Item>

          <Form.Item
            label="调整原因"
            name="reason"
            rules={[{ required: true, message: '请填写调整原因（审计要求）' }]}
          >
            <Input.TextArea
              rows={3}
              placeholder="例如：根据长假期间产能规划调整"
            />
          </Form.Item>
        </Form>

        {/* 警告提示 */}
        <Alert
          type="warning"
          showIcon
          message="注意事项"
          description={
            <ul style={{ margin: 0, paddingLeft: 20 }}>
              <li>批量调整将覆盖所选日期的产能配置</li>
              <li>已有已用产能 &gt; 0 的记录将被跳过</li>
              <li>所有调整操作将记录到操作日志</li>
            </ul>
          }
        />
      </Space>
    </Modal>
  );
};

export default BatchAdjustModal;
