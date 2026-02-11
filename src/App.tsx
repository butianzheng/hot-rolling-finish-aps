import React, { useEffect } from 'react';
import { useQueryClient } from '@tanstack/react-query';
import { Layout, Menu, Tag, Tooltip } from 'antd';
import type { MenuProps } from 'antd';
import { Outlet, useNavigate, useLocation } from 'react-router-dom';
import {
  AppstoreOutlined,
  DashboardOutlined,
  SettingOutlined,
  SwapOutlined,
  UploadOutlined,
  MenuFoldOutlined,
  MenuUnfoldOutlined,
  SyncOutlined,
} from '@ant-design/icons';
import { LongTaskManager } from './components/LongTaskProgress';
import ErrorBoundary from './components/ErrorBoundary';
import UserSelector from './components/UserSelector';
import { ThemeToggle } from './components/ThemeToggle';
import { AdminOverrideToggle } from './components/AdminOverrideToggle';
import { GlobalKPIDisplay } from './components/GlobalKPIDisplay';
import { DecisionRefreshTag } from './components/DecisionRefreshTag';
import { useTheme, LAYOUT } from './theme';
import { defaultKPI } from './types/kpi';
import { IpcClient } from './api/ipcClient';
import { planApi } from './api/tauri';
import {
  useActiveVersionId,
  useGlobalActions,
  useGlobalStore,
  useIsImporting,
  useIsRecalculating,
  useUserPreferences,
} from './stores/use-global-store';
import { useEvent } from './api/eventBus';
import { useGlobalKPI } from './hooks/useGlobalKPI';
import { useOnlineStatus } from './hooks/useOnlineStatus';
import { useVersionSwitchInvalidation } from './hooks/useVersionSwitchInvalidation';
import { useStalePlanRevBootstrap } from './hooks/useStalePlanRevBootstrap';
import { decisionQueryKeys } from './hooks/queries/use-decision-queries';
import { bootstrapFrontendRuntimeConfig } from './services/frontendRuntimeConfig';
import { reportFrontendError } from './utils/telemetry';

const { Header, Content, Sider } = Layout;

type MenuItem = Required<MenuProps>['items'][number];

// 菜单配置（key对应路由路径）
const items: MenuItem[] = [
  {
    key: '/overview',
    icon: <DashboardOutlined />,
    label: '风险概览',
  },
  {
    key: '/workbench',
    icon: <AppstoreOutlined />,
    label: '计划工作台',
  },
  {
    key: '/comparison',
    icon: <SwapOutlined />,
    label: '版本对比',
  },
  {
    key: '/import',
    icon: <UploadOutlined />,
    label: '数据导入',
  },
  { type: 'divider' },
  {
    key: '/settings',
    icon: <SettingOutlined />,
    label: '设置中心',
  },
];

export function shouldBackfillPlanContextFromRunEvent(params: {
  hadTrackedRun: boolean;
  incomingVersionId: string;
  incomingPlanRev: number;
  currentVersionId: string | null;
  currentPlanRev: number | null;
}): boolean {
  if (params.hadTrackedRun) return false;

  const versionId = String(params.incomingVersionId || '').trim();
  if (!versionId) return false;

  if (!Number.isFinite(Number(params.incomingPlanRev))) return false;

  const currentVersionId = String(params.currentVersionId || '').trim();
  if (currentVersionId && currentVersionId !== versionId) {
    return false;
  }

  const currentPlanRev = Number(params.currentPlanRev);
  if (Number.isFinite(currentPlanRev) && params.incomingPlanRev < currentPlanRev) {
    return false;
  }

  return true;
}

const App: React.FC = () => {
  const queryClient = useQueryClient();
  const { theme } = useTheme();
  const navigate = useNavigate();
  const location = useLocation();
  const activeVersionId = useActiveVersionId();
  const [effectiveVersionId, setEffectiveVersionId] = React.useState<string | null>(null);
  const isImporting = useIsImporting();
  const isRecalculating = useIsRecalculating();
  const {
    setActiveVersion,
    setPlanContext,
    markLatestRunDone,
    expireLatestRunIfNeeded,
    updateUserPreferences,
  } = useGlobalActions();
  const { sidebarCollapsed, autoRefreshInterval } = useUserPreferences();
  const isOnline = useOnlineStatus();

  const { data: globalKPIData, refetch: refetchGlobalKPI } = useGlobalKPI(activeVersionId);
  const globalKPI = globalKPIData ?? defaultKPI;

  // 监听版本切换，自动失效决策数据缓存
  useVersionSwitchInvalidation();

  // 全局注册：统一处理 STALE_PLAN_REV
  useStalePlanRevBootstrap();

  const refreshEffectiveVersion = React.useCallback(async () => {
    try {
      const latest = await IpcClient.call<string | null>(
        'get_latest_active_version_id',
        {},
        { showError: false }
      );
      setEffectiveVersionId(String(latest || '').trim() || null);
    } catch {
      // ignore
    }
  }, []);

  useEffect(() => {
    if (!isOnline) return;
    void refreshEffectiveVersion();
  }, [activeVersionId, isOnline, refreshEffectiveVersion]);

  const workingVersionLabel = React.useMemo(() => {
    if (!activeVersionId) return '工作: 未选择';
    const text = String(activeVersionId);
    if (text.length <= 16) return `工作: ${text}`;
    return `工作: ${text.slice(0, 8)}…${text.slice(-4)}`;
  }, [activeVersionId]);

  const effectiveVersionLabel = React.useMemo(() => {
    if (!effectiveVersionId) return '生效: 无';
    const text = String(effectiveVersionId);
    if (text.length <= 16) return `生效: ${text}`;
    return `生效: ${text.slice(0, 8)}…${text.slice(-4)}`;
  }, [effectiveVersionId]);

  const versionDiverged = React.useMemo(() => {
    if (!activeVersionId || !effectiveVersionId) return false;
    return activeVersionId !== effectiveVersionId;
  }, [activeVersionId, effectiveVersionId]);

  const invalidateDecisionViews = React.useCallback(() => {
    // 决策相关页面（风险概览/问题列表）默认 staleTime 较长，必须在关键事件后主动失效。
    void queryClient.invalidateQueries({ queryKey: decisionQueryKeys.all });
    void queryClient.invalidateQueries({ queryKey: ['decisionRefreshStatus'] });
    void queryClient.invalidateQueries({ queryKey: ['globalKpi'] });
  }, [queryClient]);

  // 关键事件触发时刷新 KPI + 失效决策缓存（联动：导入/重算/移单后风险视图应同步变化）
  useEvent('plan_updated', (payload: unknown) => {
    void refreshEffectiveVersion();
    expireLatestRunIfNeeded();

    const raw = payload && typeof payload === 'object'
      ? payload as Record<string, unknown>
      : null;

    const runId = String(raw?.run_id || '').trim();
    const versionId = String(raw?.version_id || '').trim();
    const planRevRaw = Number(raw?.plan_rev);
    const hasPlanRev = Number.isFinite(planRevRaw);

    if (runId) {
      const before = useGlobalStore.getState();
      const hadTrackedRun = !!before.latestRun.runId;

      markLatestRunDone(runId, {
        versionId: versionId || undefined,
        planRev: hasPlanRev ? planRevRaw : undefined,
      });

      // 刷新/重启后可能丢失 latestRun.runId：此时允许用事件里的 version_id+plan_rev 回填 PlanContext，
      // 但仅在“当前没有 run 跟踪上下文”且不会回退 plan_rev 时生效，避免旧事件覆盖新结果。
      if (!hadTrackedRun && versionId && hasPlanRev) {
        const current = useGlobalStore.getState();
        if (shouldBackfillPlanContextFromRunEvent({
          hadTrackedRun,
          incomingVersionId: versionId,
          incomingPlanRev: planRevRaw,
          currentVersionId: current.activeVersionId,
          currentPlanRev: current.activePlanRev,
        })) {
          setPlanContext({ versionId, planRev: planRevRaw });
        }
      }
    } else if (versionId && hasPlanRev) {
      const currentVersionId = useGlobalStore.getState().activeVersionId;
      if (!currentVersionId || currentVersionId === versionId) {
        setPlanContext({ versionId, planRev: planRevRaw });
      }
    }

    invalidateDecisionViews();
    if (activeVersionId) refetchGlobalKPI();
  });
  useEvent('risk_snapshot_updated', () => {
    invalidateDecisionViews();
    if (activeVersionId) refetchGlobalKPI();
  });
  useEvent('material_state_changed', () => {
    invalidateDecisionViews();
    if (activeVersionId) refetchGlobalKPI();
  });

  useEffect(() => {
    void bootstrapFrontendRuntimeConfig();
  }, []);

  // 轻量轮询：推进 latestRun TTL 过期状态，避免长驻页面卡在 RUNNING/PENDING
  useEffect(() => {
    expireLatestRunIfNeeded();

    const timer = window.setInterval(() => {
      expireLatestRunIfNeeded();
    }, 5_000);

    return () => {
      window.clearInterval(timer);
    };
  }, [expireLatestRunIfNeeded]);

  // 启动时自动回填“最近激活版本”：
  // - 若持久化的 activeVersionId 已失效（例如切换/重置数据库），静默降级到最近激活版本；
  // - 不弹出错误弹窗，避免干扰首次进入。
  useEffect(() => {
    const isTauriRuntime = typeof window !== 'undefined' && !!(window as any).__TAURI__;
    if (!isTauriRuntime) return;

    let cancelled = false;

    (async () => {
      if (activeVersionId) {
        try {
          // Validate persisted selection quietly.
          await IpcClient.call('list_plan_items', { version_id: activeVersionId }, { showError: false });
          return;
        } catch {
          // Fall through to auto-fill the latest active version.
        }
      }

      try {
        const latest = await IpcClient.call<string | null>(
          'get_latest_active_version_id',
          {},
          { showError: false }
        );
        if (!cancelled && latest) setActiveVersion(latest);
      } catch {
        // ignore
      }
    })();

    return () => {
      cancelled = true;
    };
  }, [activeVersionId, setActiveVersion]);

  // 同步当前工作版本的 plan_rev，作为查询一致性上下文
  useEffect(() => {
    if (!activeVersionId) {
      setPlanContext({ versionId: null, planRev: null });
      return;
    }

    let cancelled = false;
    (async () => {
      try {
        const detail = await planApi.getVersionDetail(activeVersionId);
        if (cancelled) return;
        setPlanContext({
          versionId: activeVersionId,
          planRev: typeof detail?.revision === 'number' ? detail.revision : null,
        });
      } catch {
        if (cancelled) return;
        setPlanContext({ versionId: activeVersionId, planRev: null });
      }
    })();

    return () => {
      cancelled = true;
    };
  }, [activeVersionId, setPlanContext]);

  // 全局错误捕获（补齐 ErrorBoundary 覆盖不到的场景：Promise rejection / 事件处理器异常等）
  useEffect(() => {
    const onError = (event: any) => {
      // 资源加载失败也会触发 error 事件，这里只采集 ErrorEvent。
      if (event && typeof event === 'object' && 'error' in event) {
        void reportFrontendError((event as ErrorEvent).error || (event as ErrorEvent).message, {
          source: 'window.error',
          filename: (event as ErrorEvent).filename,
          lineno: (event as ErrorEvent).lineno,
          colno: (event as ErrorEvent).colno,
        });
      }
    };

    const onUnhandledRejection = (event: PromiseRejectionEvent) => {
      void reportFrontendError(event.reason, { source: 'window.unhandledrejection' });
    };

    window.addEventListener('error', onError);
    window.addEventListener('unhandledrejection', onUnhandledRejection);
    return () => {
      window.removeEventListener('error', onError);
      window.removeEventListener('unhandledrejection', onUnhandledRejection);
    };
  }, []);

  // 全局 KPI 自动刷新（由“用户偏好”控制；离线/后台时不刷）
  useEffect(() => {
    if (!activeVersionId) return;
    const intervalMs = Number(autoRefreshInterval || 0);
    if (!Number.isFinite(intervalMs) || intervalMs <= 0) return;
    if (!isOnline) return;

    const timer = window.setInterval(() => {
      if (typeof document !== 'undefined' && document.hidden) return;
      refetchGlobalKPI();
    }, intervalMs);

    return () => window.clearInterval(timer);
  }, [activeVersionId, autoRefreshInterval, isOnline, refetchGlobalKPI]);

  // 处理菜单点击 - 使用路由导航
  const handleMenuClick: MenuProps['onClick'] = (e) => {
    navigate(e.key);
  };

  const toggleCollapsed = () => {
    updateUserPreferences({ sidebarCollapsed: !sidebarCollapsed });
  };

  // 根据当前路由路径计算选中的菜单项
  const selectedKeys = [location.pathname];
  const openKeys: string[] = [];

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
                {sidebarCollapsed ? <MenuUnfoldOutlined /> : <MenuFoldOutlined />}
              </div>
              <div style={{ color: 'white', fontSize: 18, fontWeight: 'bold' }}>
                热轧精整排产系统
              </div>
            </div>

            {/* 中间：全局 KPI */}
            <GlobalKPIDisplay kpi={globalKPI} />

            {/* 右侧:管理员覆盖模式 + 主题切换 + 用户选择器 */}
            <div style={{ display: 'flex', alignItems: 'center', gap: 16 }}>
              <Tooltip title={activeVersionId ? `当前工作版本: ${activeVersionId}` : '尚未选择工作版本'}>
                <Tag
                  color={activeVersionId ? 'blue' : 'orange'}
                  style={{ cursor: 'pointer', margin: 0 }}
                  onClick={() => navigate('/comparison')}
                >
                  {workingVersionLabel}
                </Tag>
              </Tooltip>
              <Tooltip title={effectiveVersionId ? `当前生效版本: ${effectiveVersionId}` : '尚无生效版本'}>
                <Tag
                  color={effectiveVersionId ? 'green' : 'default'}
                  style={{ cursor: 'pointer', margin: 0 }}
                  onClick={() => navigate('/comparison')}
                >
                  {effectiveVersionLabel}
                </Tag>
              </Tooltip>
              {versionDiverged ? (
                <Tooltip title={`当前仅切换到工作版本 ${activeVersionId}；系统生效版本仍为 ${effectiveVersionId}`}>
                  <Tag color="gold" style={{ margin: 0 }}>
                    工作≠生效
                  </Tag>
                </Tooltip>
              ) : null}
              {isImporting && (
                <Tooltip title="正在导入数据，请稍候…">
                  <Tag color="processing" style={{ margin: 0 }}>
                    <SyncOutlined spin style={{ marginRight: 6 }} />
                    导入中
                  </Tag>
                </Tooltip>
              )}
              {isRecalculating && (
                <Tooltip title="正在执行排产重算，请稍候…">
                  <Tag color="processing" style={{ margin: 0 }}>
                    <SyncOutlined spin style={{ marginRight: 6 }} />
                    重算中
                  </Tag>
                </Tooltip>
              )}
              <DecisionRefreshTag versionId={activeVersionId} />
              {!isOnline && (
                <Tooltip title="当前处于离线状态，数据可能无法刷新">
                  <Tag color="volcano" style={{ margin: 0 }}>
                    离线
                  </Tag>
                </Tooltip>
              )}
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
              collapsed={sidebarCollapsed}
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
