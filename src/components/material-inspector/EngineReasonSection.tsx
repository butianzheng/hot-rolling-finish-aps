/**
 * 引擎推理原因区组件
 */

import React from 'react';
import { Alert, Empty, Space, Typography } from 'antd';
import { InfoCircleOutlined } from '@ant-design/icons';
import type { Material } from './types';

const { Title, Paragraph } = Typography;

export interface EngineReasonSectionProps {
  material: Material;
}

export const EngineReasonSection: React.FC<EngineReasonSectionProps> = ({ material }) => {
  const hasReasons = material.urgent_reason || material.eligibility_reason || material.priority_reason;

  return (
    <>
      <Title level={5}>
        <Space>
          <InfoCircleOutlined />
          引擎推理原因
        </Space>
      </Title>

      {material.urgent_reason && (
        <Alert
          message="紧急等级判定"
          description={
            <Paragraph style={{ marginBottom: 0, fontSize: 13 }}>
              {material.urgent_reason}
            </Paragraph>
          }
          type="info"
          showIcon
          style={{ marginBottom: 12 }}
        />
      )}

      {material.eligibility_reason && (
        <Alert
          message="适温判定"
          description={
            <Paragraph style={{ marginBottom: 0, fontSize: 13 }}>
              {material.eligibility_reason}
            </Paragraph>
          }
          type="info"
          showIcon
          style={{ marginBottom: 12 }}
        />
      )}

      {material.priority_reason && (
        <Alert
          message="优先级排序"
          description={
            <Paragraph style={{ marginBottom: 0, fontSize: 13 }}>
              {material.priority_reason}
            </Paragraph>
          }
          type="info"
          showIcon
          style={{ marginBottom: 12 }}
        />
      )}

      {!hasReasons && (
        <Empty
          description="暂无引擎推理信息"
          image={Empty.PRESENTED_IMAGE_SIMPLE}
        />
      )}
    </>
  );
};

export default EngineReasonSection;
