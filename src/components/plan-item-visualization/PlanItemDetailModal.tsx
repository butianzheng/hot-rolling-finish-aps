/**
 * 排产明细详情模态框
 */

import React from 'react';
import { Button, Descriptions, Modal, Space, Tag } from 'antd';
import { formatWeight } from '../../utils/formatters';
import { urgentLevelColors, sourceTypeLabels, type PlanItem } from './types';

export interface PlanItemDetailModalProps {
  open: boolean;
  item: PlanItem | null;
  onClose: () => void;
}

export const PlanItemDetailModal: React.FC<PlanItemDetailModalProps> = ({
  open,
  item,
  onClose,
}) => {
  return (
    <Modal
      title="排产明细详情"
      open={open}
      onCancel={onClose}
      footer={[
        <Button key="close" onClick={onClose}>
          关闭
        </Button>,
      ]}
      width={700}
    >
      {item && (
        <Descriptions bordered column={2}>
          <Descriptions.Item label="材料ID" span={2}>
            {item.material_id}
          </Descriptions.Item>
          <Descriptions.Item label="钢种">{item.steel_grade}</Descriptions.Item>
          <Descriptions.Item label="吨位">{formatWeight(item.weight_t)}</Descriptions.Item>
          <Descriptions.Item label="机组">{item.machine_code}</Descriptions.Item>
          <Descriptions.Item label="排产日期">{item.plan_date}</Descriptions.Item>
          <Descriptions.Item label="序号">{item.seq_no}</Descriptions.Item>
          <Descriptions.Item label="紧急等级">
            <Tag color={urgentLevelColors[item.urgent_level || 'L0']}>
              {item.urgent_level}
            </Tag>
          </Descriptions.Item>
          <Descriptions.Item label="来源类型">
            <Tag color={sourceTypeLabels[item.source_type]?.color || 'default'}>
              {sourceTypeLabels[item.source_type]?.text || item.source_type}
            </Tag>
          </Descriptions.Item>
          <Descriptions.Item label="状态" span={2}>
            <Space>
              {item.locked_in_plan && <Tag color="purple">冻结</Tag>}
              {item.force_release_in_plan && <Tag color="orange">强制放行</Tag>}
              {!item.locked_in_plan && !item.force_release_in_plan && (
                <Tag color="green">正常</Tag>
              )}
            </Space>
          </Descriptions.Item>
          <Descriptions.Item label="排产状态">{item.sched_state}</Descriptions.Item>
          <Descriptions.Item label="落位原因" span={2}>
            {item.assign_reason}
          </Descriptions.Item>
        </Descriptions>
      )}
    </Modal>
  );
};

export default PlanItemDetailModal;
