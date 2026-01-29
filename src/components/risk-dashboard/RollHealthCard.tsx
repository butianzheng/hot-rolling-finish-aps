/**
 * 轧辊健康度卡片
 */

import React from 'react';
import { Card, Progress, Space, Statistic, Tag, Typography } from 'antd';
import { ToolOutlined } from '@ant-design/icons';
import { FONT_FAMILIES } from '../../theme';
import type { RollCampaignHealth } from '../../types/dashboard';
import { getRollStatusColor } from './types';

const { Title, Text } = Typography;

export interface RollHealthCardProps {
  roll: RollCampaignHealth;
}

export const RollHealthCard: React.FC<RollHealthCardProps> = ({ roll }) => {
  return (
    <Card hoverable>
      <Space direction="vertical" style={{ width: '100%' }} size={16}>
        <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
          <Title level={5} style={{ margin: 0 }}>
            <ToolOutlined style={{ marginRight: 8, color: getRollStatusColor(roll.status) }} />
            轧辊健康度 - {roll.machineCode}
          </Title>
          <Tag color={getRollStatusColor(roll.status)}>
            {roll.status.toUpperCase()}
          </Tag>
        </div>

        <div>
          <Text type="secondary">当前吨位</Text>
          <div style={{ display: 'flex', alignItems: 'baseline', gap: 8, marginTop: 4 }}>
            <Text
              strong
              style={{
                fontSize: 32,
                fontFamily: FONT_FAMILIES.MONOSPACE,
                color: getRollStatusColor(roll.status),
              }}
            >
              {roll.currentTonnage}
            </Text>
            <Text type="secondary">/ {roll.threshold} 吨</Text>
          </div>
        </div>

        <Progress
          percent={(roll.currentTonnage / roll.threshold) * 100}
          strokeColor={getRollStatusColor(roll.status)}
          status={roll.status === 'critical' ? 'exception' : 'active'}
          format={(percent) => `${percent?.toFixed(1)}%`}
        />

        <div style={{ display: 'flex', justifyContent: 'space-between' }}>
          <Statistic
            title="距离更换"
            value={roll.threshold - roll.currentTonnage}
            suffix="吨"
            valueStyle={{ fontSize: 20, color: getRollStatusColor(roll.status) }}
          />
        </div>
      </Space>
    </Card>
  );
};

export default RollHealthCard;
