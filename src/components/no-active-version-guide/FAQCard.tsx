/**
 * 常见问题卡片组件
 */

import React from 'react';
import { Card, Space, theme } from 'antd';
import { FAQ_ITEMS } from './types';

export const FAQCard: React.FC = () => {
  const { token } = theme.useToken();

  return (
    <Card
      title={<div>❓ 常见问题</div>}
      variant="borderless"
      style={{ marginTop: 24 }}
    >
      <Space direction="vertical" size="small" style={{ width: '100%' }}>
        {FAQ_ITEMS.map((item, index) => (
          <div key={index}>
            <p style={{ marginBottom: 4 }}>
              <strong>Q: {item.question}</strong>
            </p>
            <p style={{ margin: '0 0 8px 16px', color: token.colorTextSecondary }}>
              A: {item.answer}
            </p>
          </div>
        ))}
      </Space>
    </Card>
  );
};

export default FAQCard;
