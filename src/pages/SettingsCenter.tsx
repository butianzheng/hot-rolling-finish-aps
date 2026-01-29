import React, { useMemo } from 'react';
import { Tabs } from 'antd';
import { useSearchParams } from 'react-router-dom';
import ErrorBoundary from '../components/ErrorBoundary';
import PageSkeleton from '../components/PageSkeleton';
import UserPreferencesPanel from '../components/settings/UserPreferencesPanel';

const ConfigManagement = React.lazy(() => import('../components/ConfigManagement'));
const ActionLogQuery = React.lazy(() => import('../components/ActionLogQuery'));
const CapacityPoolManagement = React.lazy(() => import('../components/CapacityPoolManagement'));
const MaterialManagement = React.lazy(() => import('../components/MaterialManagement'));
const StrategyProfilesPanel = React.lazy(() => import('../components/settings/StrategyProfilesPanel'));

const TAB_KEYS = ['system', 'machine', 'materials', 'strategy', 'logs', 'preferences'] as const;
type TabKey = (typeof TAB_KEYS)[number];

function normalizeTabKey(value: string | null): TabKey {
  if (value && (TAB_KEYS as readonly string[]).includes(value)) return value as TabKey;
  return 'system';
}

const SettingsCenter: React.FC = () => {
  const [searchParams, setSearchParams] = useSearchParams();
  const activeKey = useMemo(() => normalizeTabKey(searchParams.get('tab')), [searchParams]);

  const handleTabChange = (key: string) => {
    const next = new URLSearchParams(searchParams);
    next.set('tab', key);
    setSearchParams(next, { replace: true });
  };

  return (
    <ErrorBoundary>
      <Tabs
        destroyInactiveTabPane
        activeKey={activeKey}
        onChange={handleTabChange}
        items={[
          {
            key: 'system',
            label: '系统配置',
            children: (
              <React.Suspense fallback={<PageSkeleton />}>
                <ConfigManagement />
              </React.Suspense>
            ),
          },
          {
            key: 'machine',
            label: '机组配置',
            children: (
              <React.Suspense fallback={<PageSkeleton />}>
                <CapacityPoolManagement />
              </React.Suspense>
            ),
          },
          {
            key: 'materials',
            label: '物料管理',
            children: (
              <React.Suspense fallback={<PageSkeleton />}>
                <MaterialManagement />
              </React.Suspense>
            ),
          },
          {
            key: 'strategy',
            label: '策略配置',
            children: (
              <React.Suspense fallback={<PageSkeleton />}>
                <StrategyProfilesPanel />
              </React.Suspense>
            ),
          },
          {
            key: 'logs',
            label: '操作日志',
            children: (
              <React.Suspense fallback={<PageSkeleton />}>
                <ActionLogQuery />
              </React.Suspense>
            ),
          },
          {
            key: 'preferences',
            label: '用户偏好',
            children: <UserPreferencesPanel />,
          },
        ]}
      />
    </ErrorBoundary>
  );
};

export default SettingsCenter;
