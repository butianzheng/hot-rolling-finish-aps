/**
 * 状态信息区组件
 */

import React from 'react';
import { Descriptions, Typography } from 'antd';
import { UrgencyTag } from '../UrgencyTag';
import type { Material } from './types';

const { Text } = Typography;

export interface StatusInfoSectionProps {
  material: Material;
}

export const StatusInfoSection: React.FC<StatusInfoSectionProps> = ({ material }) => {
  return (
    <Descriptions column={1} size="small" bordered>
      <Descriptions.Item label="排产状态">
        {material.sched_state}
      </Descriptions.Item>
      <Descriptions.Item label="紧急等级">
        <UrgencyTag level={material.urgent_level} reason={material.urgent_reason} />
      </Descriptions.Item>
      <Descriptions.Item label="人工紧急">
        {material.manual_urgent_flag ? (
          <Text type="danger" strong>是</Text>
        ) : (
          <Text type="secondary">否</Text>
        )}
      </Descriptions.Item>
      <Descriptions.Item label="锁定状态">
        {material.lock_flag ? (
          <Text type="warning" strong>已锁定</Text>
        ) : (
          <Text type="secondary">未锁定</Text>
        )}
      </Descriptions.Item>
      <Descriptions.Item label="冻结区">
        {material.is_frozen ? (
          <Text type="warning" strong>是</Text>
        ) : (
          <Text type="secondary">否</Text>
        )}
      </Descriptions.Item>
      <Descriptions.Item label="适温状态">
        {material.is_mature ? (
          <Text type="success" strong>已适温</Text>
        ) : (
          <Text type="warning" strong>未适温</Text>
        )}
      </Descriptions.Item>
    </Descriptions>
  );
};

export default StatusInfoSection;
