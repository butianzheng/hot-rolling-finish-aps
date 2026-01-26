import React, { useState, useEffect } from 'react';
import { Layout, Menu } from 'antd';
import type { MenuProps } from 'antd';
import { Outlet, useNavigate, useLocation } from 'react-router-dom';
import {
  AppstoreOutlined,
  DatabaseOutlined,
  DashboardOutlined,
  UploadOutlined,
  BarChartOutlined,
  LineChartOutlined,
  FileTextOutlined,
  MenuFoldOutlined,
  MenuUnfoldOutlined,
} from '@ant-design/icons';
import { LongTaskManager } from './components/LongTaskProgress';
import ErrorBoundary from './components/ErrorBoundary';
import UserSelector from './components/UserSelector';
import { ThemeToggle } from './components/ThemeToggle';
import { AdminOverrideToggle } from './components/AdminOverrideToggle';
import { GlobalKPIDisplay } from './components/GlobalKPIDisplay';
import { useTheme, LAYOUT } from './theme';
import type { GlobalKPI } from './types/kpi';
import { defaultKPI } from './types/kpi';
import { dashboardApi, materialApi, planApi } from './api/tauri';
import { decisionService } from './services/decision-service';
import { parseAlertLevel } from './types/decision';
import { useActiveVersionId, useGlobalActions } from './stores/use-global-store';
import { useEvent } from './api/eventBus';

const { Header, Content, Sider } = Layout;

type MenuItem = Required<MenuProps>['items'][number];

// 菜单配置（key对应路由路径）
const items: MenuItem[] = [
  {
    key: '/dashboard',
    icon: <DashboardOutlined />,
    label: '驾驶舱',
  },
  {
    key: '/decision',
    icon: <LineChartOutlined />,
    label: '决策看板',
    children: [
      {
        key: '/decision/risk-dashboard',
        icon: <DashboardOutlined />,
        label: '风险仪表盘',
      },
      {
        key: '/decision/d1-risk-heatmap',
        label: 'D1: 风险热力图',
      },
      {
        key: '/decision/d2-order-failure',
        label: 'D2: 订单看板',
      },
      {
        key: '/decision/d3-cold-stock',
        label: 'D3: 库龄分析',
      },
      {
        key: '/decision/d4-bottleneck',
        label: 'D4: 堵塞矩阵',
      },
      {
        key: '/decision/d5-roll-campaign',
        label: 'D5: 换辊警报',
      },
      {
        key: '/decision/d6-capacity-opportunity',
        label: 'D6: 容量机会',
      },
    ],
  },
  {
    key: '/material',
    icon: <DatabaseOutlined />,
    label: '材料管理',
  },
  {
    key: '/import',
    icon: <UploadOutlined />,
    label: '材料导入',
  },
  {
    key: '/plan',
    icon: <AppstoreOutlined />,
    label: '排产方案',
  },
  {
    key: '/visualization',
    icon: <BarChartOutlined />,
    label: '排产明细',
  },
  {
    key: '/capacity',
    icon: <DatabaseOutlined />,
    label: '产能池管理',
  },
  {
    key: '/config',
    icon: <AppstoreOutlined />,
    label: '配置管理',
  },
  {
    key: '/risk',
    icon: <LineChartOutlined />,
    label: '风险分析',
  },
  {
    key: '/logs',
    icon: <FileTextOutlined />,
    label: '操作日志',
  },
];

const App: React.FC = () => {
  const { theme } = useTheme();
  const navigate = useNavigate();
  const location = useLocation();
  const [collapsed, setCollapsed] = useState(false);
  const [globalKPI, setGlobalKPI] = useState<GlobalKPI>(defaultKPI);
  const activeVersionId = useActiveVersionId();
  const { setActiveVersion } = useGlobalActions();

  // 全局 KPI 数据（基于当前激活版本）
  const loadGlobalKPI = async (versionId: string) => {
    try {
      const [
        mostRiskyRes,
        coldStockRes,
        urgentL2,
        urgentL3,
        rollAlertsRes,
      ] = await Promise.all([
        dashboardApi.getMostRiskyDate(versionId),
        dashboardApi.getColdStockMaterials(versionId, 30),
        materialApi.listMaterialsByUrgentLevel('L2'),
        materialApi.listMaterialsByUrgentLevel('L3'),
        decisionService.getAllRollCampaignAlerts(versionId),
      ]);

      const most = mostRiskyRes?.items?.[0];
      const riskLevelRaw = String(most?.risk_level || '').toUpperCase();
      const riskLevel =
        riskLevelRaw === 'CRITICAL'
          ? 'critical'
          : riskLevelRaw === 'HIGH'
          ? 'high'
          : riskLevelRaw === 'MEDIUM'
          ? 'medium'
          : 'low';

      const urgentMaterials = [
        ...(Array.isArray(urgentL2) ? urgentL2 : []),
        ...(Array.isArray(urgentL3) ? urgentL3 : []),
      ];
      const blockedUrgentCount = urgentMaterials.filter((m: any) => m?.sched_state !== 'SCHEDULED').length;

      // 选取“最需要关注”的换辊警报点作为 Header KPI 展示
      const rollItems = rollAlertsRes?.items || [];
      const rollItem = rollItems.reduce<any | null>((best, cur) => {
        if (!best) return cur;
        const bestStatus = parseAlertLevel(String(best.alertLevel || ''));
        const curStatus = parseAlertLevel(String(cur.alertLevel || ''));
        const severityOrder: Record<string, number> = {
          HARD_STOP: 3,
          WARNING: 2,
          SUGGEST: 1,
          NORMAL: 0,
        };
        const bestScore = severityOrder[bestStatus] ?? 0;
        const curScore = severityOrder[curStatus] ?? 0;
        if (curScore !== bestScore) return curScore > bestScore ? cur : best;

        const bestUtil = best.hardLimitT > 0 ? best.currentTonnageT / best.hardLimitT : 0;
        const curUtil = cur.hardLimitT > 0 ? cur.currentTonnageT / cur.hardLimitT : 0;
        return curUtil > bestUtil ? cur : best;
      }, null);

      const rollStatusRaw = rollItem ? parseAlertLevel(String(rollItem.alertLevel || '')) : 'NORMAL';
      const rollStatus =
        rollStatusRaw === 'HARD_STOP'
          ? 'critical'
          : rollStatusRaw === 'WARNING' || rollStatusRaw === 'SUGGEST'
          ? 'warning'
          : 'healthy';

      const coldStockCount =
        typeof coldStockRes?.summary?.total_cold_stock_count === 'number'
          ? coldStockRes.summary.total_cold_stock_count
          : (coldStockRes?.items || []).reduce((sum: number, b: any) => sum + (Number(b?.count) || 0), 0);

      setGlobalKPI({
        mostRiskyDate: most?.plan_date,
        riskLevel,
        urgentOrdersCount: urgentMaterials.length,
        blockedUrgentCount,
        capacityUtilization: typeof most?.capacity_util_pct === 'number' ? most.capacity_util_pct : 0,
        coldStockCount,
        rollCampaignProgress: rollItem?.currentTonnageT ?? 0,
        rollChangeThreshold: rollItem?.hardLimitT ?? 1500,
        rollStatus,
      });
    } catch (e) {
      console.error('[GlobalKPI] load failed:', e);
      setGlobalKPI(defaultKPI);
    }
  };

  useEffect(() => {
    if (!activeVersionId) {
      setGlobalKPI(defaultKPI);
      return;
    }
    loadGlobalKPI(activeVersionId);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [activeVersionId]);

  // 关键事件触发时刷新 KPI（联动：导入/重算/移单后 Header 指标应同步变化）
  useEvent('plan_updated', () => {
    if (activeVersionId) loadGlobalKPI(activeVersionId);
  });
  useEvent('risk_snapshot_updated', () => {
    if (activeVersionId) loadGlobalKPI(activeVersionId);
  });
  useEvent('material_state_changed', () => {
    if (activeVersionId) loadGlobalKPI(activeVersionId);
  });

  // 启动时自动回填“最近激活版本”，避免已有激活版本但全局状态为空导致各页面不可用。
  useEffect(() => {
    const isTauriRuntime = typeof window !== 'undefined' && !!(window as any).__TAURI__;
    if (!isTauriRuntime || activeVersionId) return;

    (async () => {
      try {
        const latest = await planApi.getLatestActiveVersionId();
        if (latest) {
          setActiveVersion(latest);
        }
      } catch {
        // 错误已由 IpcClient 统一处理，这里避免再次打断渲染。
      }
    })();
  }, [activeVersionId, setActiveVersion]);

  // 处理菜单点击 - 使用路由导航
  const handleMenuClick: MenuProps['onClick'] = (e) => {
    navigate(e.key);
  };

  const toggleCollapsed = () => {
    setCollapsed(!collapsed);
  };

  // 根据当前路由路径计算选中的菜单项
  const selectedKeys = [location.pathname];
  // 如果是决策看板的子路由，也要选中父菜单
  const openKeys = location.pathname.startsWith('/decision') ? ['/decision'] : [];

  return (
    <ErrorBoundary>
      <Layout style={{ minHeight: '100vh' }}>
          {/* ==========================================
              自定义 Header - Tauri 拖拽区域
              ========================================== */}
          <Header
            data-tauri-drag-region
            style={{
              display: 'flex',
              alignItems: 'center',
              justifyContent: 'space-between',
              padding: '0 24px',
              height: LAYOUT.HEADER_HEIGHT,
              position: 'sticky',
              top: 0,
              zIndex: 1000,
              boxShadow: theme === 'dark'
                ? '0 2px 8px rgba(0, 0, 0, 0.45)'
                : '0 2px 8px rgba(0, 0, 0, 0.15)',
            }}
          >
            {/* 左侧：Logo + 折叠按钮 */}
            <div style={{ display: 'flex', alignItems: 'center', gap: 16 }}>
              <div
                onClick={toggleCollapsed}
                style={{
                  color: 'rgba(255, 255, 255, 0.85)',
                  fontSize: 18,
                  cursor: 'pointer',
                  transition: 'color 0.3s',
                }}
                onMouseEnter={(e) => (e.currentTarget.style.color = '#1677ff')}
                onMouseLeave={(e) => (e.currentTarget.style.color = 'rgba(255, 255, 255, 0.85)')}
              >
                {collapsed ? <MenuUnfoldOutlined /> : <MenuFoldOutlined />}
              </div>
              <div style={{ color: 'white', fontSize: 18, fontWeight: 'bold' }}>
                热轧精整排产系统
              </div>
            </div>

            {/* 中间：全局 KPI */}
            <GlobalKPIDisplay kpi={globalKPI} />

            {/* 右侧:管理员覆盖模式 + 主题切换 + 用户选择器 */}
            <div style={{ display: 'flex', alignItems: 'center', gap: 16 }}>
              <AdminOverrideToggle />
              <ThemeToggle />
              <UserSelector />
            </div>
          </Header>

          <Layout>
            {/* ==========================================
                可折叠侧边栏 - IDE 风格
                ========================================== */}
            <Sider
              width={LAYOUT.SIDEBAR_WIDTH}
              collapsedWidth={LAYOUT.SIDEBAR_COLLAPSED_WIDTH}
              collapsed={collapsed}
              trigger={null}
              style={{
                overflow: 'auto',
                height: `calc(100vh - ${LAYOUT.HEADER_HEIGHT}px)`,
                position: 'sticky',
                top: LAYOUT.HEADER_HEIGHT,
                left: 0,
                transition: 'all 0.2s',
              }}
            >
              <Menu
                mode="inline"
                selectedKeys={selectedKeys}
                defaultOpenKeys={openKeys}
                onClick={handleMenuClick}
                style={{
                  height: '100%',
                  borderRight: 0,
                  paddingTop: 8,
                }}
                items={items}
              />
            </Sider>

            {/* ==========================================
                主内容区域 - 使用 React Router Outlet
                ========================================== */}
            <Layout style={{ padding: '16px' }}>
              <Content
                style={{
                  padding: 24,
                  margin: 0,
                  minHeight: 280,
                  borderRadius: 8,
                  overflow: 'auto',
                }}
              >
                <Outlet />
              </Content>
            </Layout>
          </Layout>
      </Layout>
      <LongTaskManager />
    </ErrorBoundary>
  );
};

export default App;
