/**
 * 危险日期卡片
 */

import React from 'react';
import { Card, Empty, List, Space, Statistic, Tag, Typography } from 'antd';
import { WarningOutlined } from '@ant-design/icons';
import { FONT_FAMILIES } from '../../theme';
import type { DangerDayData } from '../../types/dashboard';
import { getRiskColor } from './types';

const { Title, Text } = Typography;

export interface DangerDayCardProps {
  dangerDay: DangerDayData | null;
}

export const DangerDayCard: React.FC<DangerDayCardProps> = ({ dangerDay }) => {
  return (
    <Card
      hoverable
      style={{
        height: '100%',
        borderLeft: `4px solid ${dangerDay ? getRiskColor(dangerDay.riskLevel) : '#52c41a'}`,
      }}
    >
      <Space direction="vertical" style={{ width: '100%' }} size={16}>
        <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between' }}>
          <Title level={5} style={{ margin: 0 }}>
            <WarningOutlined style={{ marginRight: 8, color: getRiskColor(dangerDay?.riskLevel || 'low') }} />
            危险日期
          </Title>
          {dangerDay && (
            <Tag color={getRiskColor(dangerDay.riskLevel)}>
              {dangerDay.riskLevel.toUpperCase()}
            </Tag>
          )}
        </div>

        {dangerDay ? (
          <>
            <Statistic
              title="最高风险日"
              value={dangerDay.date}
              valueStyle={{
                fontSize: 24,
                fontWeight: 'bold',
                color: getRiskColor(dangerDay.riskLevel),
                fontFamily: FONT_FAMILIES.MONOSPACE,
              }}
            />
            <div>
              <Text type="secondary" style={{ fontSize: 12 }}>
                风险原因:
              </Text>
              <List
                size="small"
                dataSource={dangerDay.reasons}
                renderItem={(item) => (
                  <List.Item style={{ padding: '4px 0', border: 'none' }}>
                    <Text style={{ fontSize: 13 }}>• {item}</Text>
                  </List.Item>
                )}
              />
            </div>
          </>
        ) : (
          <Empty description="暂无风险" image={Empty.PRESENTED_IMAGE_SIMPLE} />
        )}
      </Space>
    </Card>
  );
};

export default DangerDayCard;
