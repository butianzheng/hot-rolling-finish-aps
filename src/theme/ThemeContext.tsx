// ==========================================
// 主题上下文和 Hook
// ==========================================
// 提供主题切换功能和当前主题状态
// ==========================================

import React, { createContext, useContext, useState, useEffect } from 'react';
import { ConfigProvider } from 'antd';
import type { ThemeConfig } from 'antd';
import { darkTheme } from './darkTheme';
import { lightTheme } from './lightTheme';

// ==========================================
// 主题类型定义
// ==========================================
export type ThemeMode = 'dark' | 'light';

interface ThemeContextType {
  theme: ThemeMode;
  toggleTheme: () => void;
  setTheme: (theme: ThemeMode) => void;
}

// ==========================================
// 创建上下文
// ==========================================
const ThemeContext = createContext<ThemeContextType | undefined>(undefined);

// ==========================================
// 主题提供者组件
// ==========================================
interface ThemeProviderProps {
  children: React.ReactNode;
  defaultTheme?: ThemeMode;
}

export const ThemeProvider: React.FC<ThemeProviderProps> = ({
  children,
  defaultTheme = 'dark', // 默认暗色模式（适合控制室）
}) => {
  // 从 localStorage 读取保存的主题，如果没有则使用默认主题
  const [theme, setThemeState] = useState<ThemeMode>(() => {
    const savedTheme = localStorage.getItem('theme') as ThemeMode | null;
    return savedTheme || defaultTheme;
  });

  // 当主题改变时，保存到 localStorage
  useEffect(() => {
    localStorage.setItem('theme', theme);
    // 更新 document 的 data-theme 属性，方便 CSS 使用
    document.documentElement.setAttribute('data-theme', theme);
  }, [theme]);

  // 切换主题
  const toggleTheme = () => {
    setThemeState((prev) => (prev === 'dark' ? 'light' : 'dark'));
  };

  // 设置主题
  const setTheme = (newTheme: ThemeMode) => {
    setThemeState(newTheme);
  };

  // 根据当前主题选择配置
  const themeConfig: ThemeConfig = theme === 'dark' ? darkTheme : lightTheme;

  return (
    <ThemeContext.Provider value={{ theme, toggleTheme, setTheme }}>
      <ConfigProvider theme={themeConfig}>{children}</ConfigProvider>
    </ThemeContext.Provider>
  );
};

// ==========================================
// 自定义 Hook
// ==========================================
export const useTheme = (): ThemeContextType => {
  const context = useContext(ThemeContext);
  if (!context) {
    throw new Error('useTheme must be used within a ThemeProvider');
  }
  return context;
};

// ==========================================
// 导出主题配置（供其他地方直接使用）
// ==========================================
export { darkTheme, lightTheme };
