/**
 * 通用卡片包装组件
 */

import React from 'react';
import { Card, theme } from 'antd';

interface KPICardWrapperProps {
  id: string;
  hovered: string | null;
  onClick?: () => void;
  onMouseEnter: (id: string) => void;
  onMouseLeave: (id: string) => void;
  children: React.ReactNode;
}

export const KPICardWrapper: React.FC<KPICardWrapperProps> = ({
  id,
  hovered,
  onClick,
  onMouseEnter,
  onMouseLeave,
  children,
}) => {
  const { token } = theme.useToken();
  const handleKeyDown = (e: React.KeyboardEvent) => {
    if (onClick && (e.key === 'Enter' || e.key === ' ')) {
      e.preventDefault();
      onClick();
    }
  };

  return (
    <div
      role={onClick ? 'button' : undefined}
      tabIndex={onClick ? 0 : undefined}
      onClick={onClick}
      onKeyDown={onClick ? handleKeyDown : undefined}
      onMouseEnter={onClick ? () => onMouseEnter(id) : undefined}
      onMouseLeave={onClick ? () => onMouseLeave(id) : undefined}
      style={{
        width: '100%',
        height: '100%',
        cursor: onClick ? 'pointer' : 'default',
        transition: 'transform 160ms ease, box-shadow 160ms ease',
        transform: hovered === id ? 'translateY(-2px)' : 'translateY(0)',
        boxShadow: hovered === id ? token.boxShadowSecondary : 'none',
        borderRadius: token.borderRadiusLG,
        outline: 'none',
      }}
    >
      <Card
        size="small"
        style={{
          height: '100%',
          minHeight: 120,
        }}
        bodyStyle={{
          height: '100%',
          display: 'flex',
          flexDirection: 'column',
          justifyContent: 'center',
          padding: 16,
        }}
      >
        {children}
      </Card>
    </div>
  );
};
