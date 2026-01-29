/**
 * 图例组件
 */

import React from 'react';
import { Space, Typography } from 'antd';
import { URGENCY_LEGEND_ITEMS, LINE_LEGEND_ITEMS } from './types';

const { Text } = Typography;

export const Legend: React.FC = () => {
  return (
    <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
      {/* 紧急等级图例 */}
      <Space size={16}>
        {URGENCY_LEGEND_ITEMS.map((item) => (
          <Space size={4} key={item.key}>
            <div
              style={{
                width: 12,
                height: 12,
                backgroundColor: item.color,
                borderRadius: 2,
              }}
            />
            <Text style={{ fontSize: 12 }}>{item.label}</Text>
          </Space>
        ))}
      </Space>

      {/* 线条图例 */}
      <Space size={16}>
        {LINE_LEGEND_ITEMS.map((item) => (
          <Space size={4} key={item.label}>
            <div
              style={{
                width: 2,
                height: 12,
                backgroundColor: item.color,
                opacity: item.opacity,
              }}
            />
            <Text style={{ fontSize: 12 }}>{item.label}</Text>
          </Space>
        ))}
      </Space>
    </div>
  );
};

export default Legend;
