/**
 * 阻塞紧急订单卡片
 */

import React from 'react';
import { Card, Empty, List, Space, Statistic, Tag, Typography } from 'antd';
import { ClockCircleOutlined, FireOutlined } from '@ant-design/icons';
import { FONT_FAMILIES } from '../../theme';
import type { BlockedUrgentOrderRow } from './types';

const { Title, Text } = Typography;

export interface BlockedOrdersCardProps {
  blockedOrders: BlockedUrgentOrderRow[];
}

export const BlockedOrdersCard: React.FC<BlockedOrdersCardProps> = ({ blockedOrders }) => {
  return (
    <Card hoverable style={{ height: '100%' }}>
      <Space direction="vertical" style={{ width: '100%' }} size={16}>
        <Title level={5} style={{ margin: 0 }}>
          <FireOutlined style={{ marginRight: 8, color: '#ff4d4f' }} />
          阻塞紧急订单
        </Title>

        {blockedOrders.length > 0 ? (
          <>
            <Statistic
              title="阻塞数量"
              value={blockedOrders.length}
              suffix="件"
              valueStyle={{ color: '#ff4d4f' }}
            />
            <List
              size="small"
              dataSource={blockedOrders.slice(0, 3)}
              renderItem={(item) => (
                <List.Item style={{ padding: '8px 0' }}>
                  <Space direction="vertical" size={4} style={{ width: '100%' }}>
                    <div style={{ display: 'flex', justifyContent: 'space-between' }}>
                      <Text strong style={{ fontFamily: FONT_FAMILIES.MONOSPACE, fontSize: 13 }}>
                        {item.contractNo}
                      </Text>
                      <Tag color={item.urgencyLevel === 'L3' ? '#ff4d4f' : '#faad14'}>
                        {item.urgencyLevel}
                      </Tag>
                    </div>
                    <Text type="secondary" style={{ fontSize: 12 }}>
                      <ClockCircleOutlined /> 距交期 {item.daysToDue} 天 - {item.failType}
                    </Text>
                  </Space>
                </List.Item>
              )}
            />
          </>
        ) : (
          <Empty description="无阻塞订单" image={Empty.PRESENTED_IMAGE_SIMPLE} />
        )}
      </Space>
    </Card>
  );
};

export default BlockedOrdersCard;
