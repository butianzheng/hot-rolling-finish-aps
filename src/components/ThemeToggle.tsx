// ==========================================
// 主题切换按钮组件
// ==========================================
// 用于在 Header 中切换暗色/亮色主题
// ==========================================

import React from 'react';
import { Button, Tooltip } from 'antd';
import { BulbOutlined, BulbFilled } from '@ant-design/icons';
import { useTheme } from '../theme';
import { useGlobalActions } from '../stores/use-global-store';

export const ThemeToggle: React.FC = () => {
  const { theme, toggleTheme } = useTheme();
  const { updateUserPreferences } = useGlobalActions();

  return (
    <Tooltip title={theme === 'dark' ? '切换到亮色模式' : '切换到暗色模式'}>
      <Button
        type="text"
        icon={theme === 'dark' ? <BulbOutlined /> : <BulbFilled />}
        onClick={() => {
          const nextTheme = theme === 'dark' ? 'light' : 'dark';
          toggleTheme();
          updateUserPreferences({ defaultTheme: nextTheme });
        }}
        style={{
          color: theme === 'dark' ? 'rgba(255, 255, 255, 0.85)' : 'rgba(255, 255, 255, 0.85)',
          fontSize: 18,
        }}
      />
    </Tooltip>
  );
};
