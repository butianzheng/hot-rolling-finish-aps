// ==========================================
// React Router 路由配置
// ==========================================
// 采用专业路由管理，替代原有的switch-case方案
// 支持懒加载、嵌套路由、路由守卫
// ==========================================

import React from 'react';
import { createBrowserRouter, createHashRouter, Navigate } from 'react-router-dom';
import App from '../App';
import PageSkeleton from '../components/PageSkeleton';

// ==========================================
// 懒加载组件（按需加载，优化性能）
// ==========================================

// 新结构页面组件（Phase 1：先搭骨架，逐步迁移现有页面能力）
const RiskOverview = React.lazy(() => import('../pages/RiskOverview'));
const PlanningWorkbench = React.lazy(() => import('../pages/PlanningWorkbench'));
const VersionComparison = React.lazy(() => import('../pages/VersionComparison'));
const DataImport = React.lazy(() => import('../pages/DataImport'));
const SettingsCenter = React.lazy(() => import('../pages/SettingsCenter'));

// ==========================================
// 路由配置
// ==========================================

const routes = [
  {
    path: '/',
    element: <App />,
    children: [
      // 默认路由 - 重定向到风险概览
      {
        index: true,
        element: <Navigate to="/overview" replace />,
      },

      // ==========================================
      // 风险概览（合并 Dashboard + D1-D6）
      // ==========================================
      {
        path: 'overview',
        element: (
          <React.Suspense fallback={<PageSkeleton />}>
            <RiskOverview />
          </React.Suspense>
        ),
      },

      // ==========================================
      // 计划工作台（合并 Material + Plan + Visualization）
      // ==========================================
      {
        path: 'workbench',
        element: (
          <React.Suspense fallback={<PageSkeleton />}>
            <PlanningWorkbench />
          </React.Suspense>
        ),
      },

      // ==========================================
      // 版本对比（合并策略对比 + 复盘分析）
      // ==========================================
      {
        path: 'comparison',
        element: (
          <React.Suspense fallback={<PageSkeleton />}>
            <VersionComparison />
          </React.Suspense>
        ),
      },

      // ==========================================
      // 数据导入
      // ==========================================
      {
        path: 'import',
        element: (
          <React.Suspense fallback={<PageSkeleton />}>
            <DataImport />
          </React.Suspense>
        ),
      },

      // ==========================================
      // 设置中心（合并 Config + Logs + Machine + Preferences）
      // ==========================================
      {
        path: 'settings',
        element: (
          <React.Suspense fallback={<PageSkeleton />}>
            <SettingsCenter />
          </React.Suspense>
        ),
      },

      // ==========================================
      // 404 页面
      // ==========================================
      {
        path: '*',
        element: <div>404 - 页面未找到</div>,
      },
    ],
  },
];

// Tauri 打包场景下使用 HashRouter，避免深链/刷新导致资源 404（tauri:// 协议不支持 history 路由回退到 index.html）。
const isTauriRuntime = typeof window !== 'undefined' && !!(window as any).__TAURI__;
export const router = isTauriRuntime ? createHashRouter(routes) : createBrowserRouter(routes);

// ==========================================
// 路由键值映射（用于菜单高亮）
// ==========================================
export const ROUTE_KEYS = {
  OVERVIEW: '/overview',
  WORKBENCH: '/workbench',
  COMPARISON: '/comparison',
  IMPORT: '/import',
  SETTINGS: '/settings',
} as const;
