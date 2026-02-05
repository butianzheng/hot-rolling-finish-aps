import React, { useMemo } from 'react';
import { Tabs } from 'antd';
import { useSearchParams } from 'react-router-dom';
import ErrorBoundary from '../components/ErrorBoundary';
import PageSkeleton from '../components/PageSkeleton';
import UserPreferencesPanel from '../components/settings/UserPreferencesPanel';

const ConfigManagement = React.lazy(() => import('../components/ConfigManagement'));
const ActionLogQuery = React.lazy(() => import('../components/ActionLogQuery'));
const CapacityPoolManagementV2 = React.lazy(() => import('../components/capacity-pool-management-v2'));
const MaterialManagement = React.lazy(() => import('../components/MaterialManagement'));
const StrategyProfilesPanel = React.lazy(() => import('../components/settings/StrategyProfilesPanel'));
const PathRuleConfigPanel = React.lazy(() => import('../components/settings/PathRuleConfigPanel'));
const RollCampaignManagementPanel = React.lazy(
  () => import('../components/settings/RollCampaignManagementPanel')
);
const RhythmPresetManagementPanel = React.lazy(
  () => import('../components/settings/RhythmPresetManagementPanel')
);

const TAB_KEYS = [
  'system',
  'capacity_calendar',
  'materials',
  'roll',
  'rhythm',
  'strategy',
  'path_rule',
  'logs',
  'preferences',
] as const;
type TabKey = (typeof TAB_KEYS)[number];

function normalizeTabKey(value: string | null): TabKey {
  if (value && (TAB_KEYS as readonly string[]).includes(value)) return value as TabKey;
  return 'system';
}

const SettingsCenter: React.FC = () => {
  const [searchParams, setSearchParams] = useSearchParams();
  const activeKey = useMemo(() => normalizeTabKey(searchParams.get('tab')), [searchParams]);

  // 提取上下文参数（用于跳转携带上下文）
  const contextParams = useMemo(() => ({
    machineCode: searchParams.get('machine_code') || undefined,
    planDate: searchParams.get('plan_date') || undefined,
  }), [searchParams]);

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
            key: 'capacity_calendar',
            label: '产能池管理',
            children: (
              <React.Suspense fallback={<PageSkeleton />}>
                <CapacityPoolManagementV2 />
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
            key: 'roll',
            label: '换辊管理',
            children: (
              <React.Suspense fallback={<PageSkeleton />}>
                <RollCampaignManagementPanel />
              </React.Suspense>
            ),
          },
          {
            key: 'rhythm',
            label: '节奏模板',
            children: (
              <React.Suspense fallback={<PageSkeleton />}>
                <RhythmPresetManagementPanel />
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
            key: 'path_rule',
            label: '路径规则',
            children: (
              <React.Suspense fallback={<PageSkeleton />}>
                <PathRuleConfigPanel
                  contextMachineCode={contextParams.machineCode}
                  contextPlanDate={contextParams.planDate}
                />
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
