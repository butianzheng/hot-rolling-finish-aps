import React from 'react';
import { Typography } from 'antd';

const { Text } = Typography;

export interface CountInfoProps {
  count: number;
}

export const CountInfo: React.FC<CountInfoProps> = ({ count }) => (
  <Text type="secondary" style={{ fontSize: 12 }}>
    共 {count} 条
  </Text>
);
