// ==========================================
// React Router 路由配置
// ==========================================
// 采用专业路由管理，替代原有的switch-case方案
// 支持懒加载、嵌套路由、路由守卫
// ==========================================

import React from 'react';
import { createBrowserRouter, createHashRouter, Navigate } from 'react-router-dom';
import App from '../App';

// ==========================================
// 懒加载组件（按需加载，优化性能）
// ==========================================

// 主要页面组件
const Dashboard = React.lazy(() => import('../components/Dashboard'));
const MaterialManagement = React.lazy(() => import('../components/MaterialManagement'));
const MaterialImport = React.lazy(() => import('../components/MaterialImport'));
const PlanManagement = React.lazy(() => import('../components/PlanManagement'));
const PlanItemVisualization = React.lazy(() => import('../components/PlanItemVisualization'));
const CapacityPoolManagement = React.lazy(() => import('../components/CapacityPoolManagement'));
const ConfigManagement = React.lazy(() => import('../components/ConfigManagement'));
const RiskSnapshotCharts = React.lazy(() => import('../components/RiskSnapshotCharts'));
const ActionLogQuery = React.lazy(() => import('../components/ActionLogQuery'));

// 决策看板组件（暂时使用现有组件，后续替换为新实现）
const RiskDashboard = React.lazy(() => import('../components/RiskDashboard'));

// 决策看板新组件（D1-D6）
const D1RiskHeatmap = React.lazy(() => import('../pages/DecisionBoard/D1RiskHeatmap'));
const D2OrderFailure = React.lazy(() => import('../pages/DecisionBoard/D2OrderFailure'));
const D3ColdStock = React.lazy(() => import('../pages/DecisionBoard/D3ColdStock'));
const D4Bottleneck = React.lazy(() => import('../pages/DecisionBoard/D4Bottleneck'));
const D5RollCampaign = React.lazy(() => import('../pages/DecisionBoard/D5RollCampaign'));
const D6CapacityOpportunity = React.lazy(() => import('../pages/DecisionBoard/D6CapacityOpportunity'));

// ==========================================
// 路由配置
// ==========================================

const routes = [
  {
    path: '/',
    element: <App />,
    children: [
      // 默认路由 - 重定向到驾驶舱
      {
        index: true,
        element: <Navigate to="/dashboard" replace />,
      },

      // ==========================================
      // 驾驶舱
      // ==========================================
      {
        path: 'dashboard',
        element: (
          <React.Suspense fallback={<div>加载中...</div>}>
            <Dashboard />
          </React.Suspense>
        ),
      },

      // ==========================================
      // 决策看板（D1-D6）
      // ==========================================
      {
        path: 'decision',
        children: [
          // 决策看板总览（暂时重定向到风险仪表盘）
          {
            index: true,
            element: <Navigate to="/decision/risk-dashboard" replace />,
          },
          // D1: 风险热力图
          {
            path: 'd1-risk-heatmap',
            element: (
              <React.Suspense fallback={<div>加载中...</div>}>
                <D1RiskHeatmap />
              </React.Suspense>
            ),
          },
          // D2: 订单看板
          {
            path: 'd2-order-failure',
            element: (
              <React.Suspense fallback={<div>加载中...</div>}>
                <D2OrderFailure />
              </React.Suspense>
            ),
          },
          // D3: 库龄分析
          {
            path: 'd3-cold-stock',
            element: (
              <React.Suspense fallback={<div>加载中...</div>}>
                <D3ColdStock />
              </React.Suspense>
            ),
          },
          // D4: 堵塞矩阵
          {
            path: 'd4-bottleneck',
            element: (
              <React.Suspense fallback={<div>加载中...</div>}>
                <D4Bottleneck />
              </React.Suspense>
            ),
          },
          // D5: 换辊警报
          {
            path: 'd5-roll-campaign',
            element: (
              <React.Suspense fallback={<div>加载中...</div>}>
                <D5RollCampaign />
              </React.Suspense>
            ),
          },
          // D6: 容量机会
          {
            path: 'd6-capacity-opportunity',
            element: (
              <React.Suspense fallback={<div>加载中...</div>}>
                <D6CapacityOpportunity />
              </React.Suspense>
            ),
          },
          // 现有的风险仪表盘（临时保留）
          {
            path: 'risk-dashboard',
            element: (
              <React.Suspense fallback={<div>加载中...</div>}>
                <RiskDashboard />
              </React.Suspense>
            ),
          },
        ],
      },

      // ==========================================
      // 材料管理
      // ==========================================
      {
        path: 'material',
        element: (
          <React.Suspense fallback={<div>加载中...</div>}>
            <MaterialManagement />
          </React.Suspense>
        ),
      },

      // ==========================================
      // 材料导入
      // ==========================================
      {
        path: 'import',
        element: (
          <React.Suspense fallback={<div>加载中...</div>}>
            <MaterialImport />
          </React.Suspense>
        ),
      },

      // ==========================================
      // 排产方案
      // ==========================================
      {
        path: 'plan',
        element: (
          <React.Suspense fallback={<div>加载中...</div>}>
            <PlanManagement />
          </React.Suspense>
        ),
      },

      // ==========================================
      // 排产明细可视化
      // ==========================================
      {
        path: 'visualization',
        element: (
          <React.Suspense fallback={<div>加载中...</div>}>
            <PlanItemVisualization />
          </React.Suspense>
        ),
      },

      // ==========================================
      // 产能池管理
      // ==========================================
      {
        path: 'capacity',
        element: (
          <React.Suspense fallback={<div>加载中...</div>}>
            <CapacityPoolManagement />
          </React.Suspense>
        ),
      },

      // ==========================================
      // 配置管理
      // ==========================================
      {
        path: 'config',
        element: (
          <React.Suspense fallback={<div>加载中...</div>}>
            <ConfigManagement />
          </React.Suspense>
        ),
      },

      // ==========================================
      // 风险分析
      // ==========================================
      {
        path: 'risk',
        element: (
          <React.Suspense fallback={<div>加载中...</div>}>
            <RiskSnapshotCharts />
          </React.Suspense>
        ),
      },

      // ==========================================
      // 操作日志
      // ==========================================
      {
        path: 'logs',
        element: (
          <React.Suspense fallback={<div>加载中...</div>}>
            <ActionLogQuery />
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
  DASHBOARD: '/dashboard',
  DECISION: '/decision',
  DECISION_D1: '/decision/d1-risk-heatmap',
  DECISION_D2: '/decision/d2-order-failure',
  DECISION_D3: '/decision/d3-cold-stock',
  DECISION_D4: '/decision/d4-bottleneck',
  DECISION_D5: '/decision/d5-roll-campaign',
  DECISION_D6: '/decision/d6-capacity-opportunity',
  DECISION_RISK_DASHBOARD: '/decision/risk-dashboard',
  MATERIAL: '/material',
  IMPORT: '/import',
  PLAN: '/plan',
  VISUALIZATION: '/visualization',
  CAPACITY: '/capacity',
  CONFIG: '/config',
  RISK: '/risk',
  LOGS: '/logs',
} as const;
