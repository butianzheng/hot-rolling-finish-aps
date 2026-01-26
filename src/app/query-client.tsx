// ==========================================
// TanStack Query 配置
// ==========================================
// 职责: 配置全局QueryClient和QueryClientProvider
// ==========================================

import React from 'react';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { message } from 'antd';

// ==========================================
// QueryClient 实例
// ==========================================

/**
 * 全局QueryClient实例
 */
export const queryClient = new QueryClient({
  defaultOptions: {
    queries: {
      // 默认配置
      staleTime: 5 * 60 * 1000, // 5分钟
      gcTime: 10 * 60 * 1000, // 10分钟（原cacheTime）
      refetchOnWindowFocus: true,
      refetchOnReconnect: true,
      retry: 2,
      retryDelay: (attemptIndex) => Math.min(1000 * 2 ** attemptIndex, 30000),

      // 全局错误处理
      throwOnError: false, // 不自动抛出错误到Error Boundary
    },
    mutations: {
      // Mutation默认配置
      retry: 1,
      retryDelay: 1000,

      // 全局错误处理
      onError: (error: any) => {
        console.error('[Mutation Error]', error);
        message.error(error?.message || '操作失败，请重试');
      },
    },
  },
});

// ==========================================
// Provider 组件
// ==========================================

/**
 * QueryClientProvider 包装组件
 */
export const QueryProvider: React.FC<{ children: React.ReactNode }> = ({ children }) => {
  return <QueryClientProvider client={queryClient}>{children}</QueryClientProvider>;
};
