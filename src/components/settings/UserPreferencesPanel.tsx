import React, { useMemo } from 'react';
import { Button, Card, Form, InputNumber, Select, Space, Switch } from 'antd';
import { useQuery } from '@tanstack/react-query';
import { configApi } from '../../api/tauri';
import { useGlobalActions, useGlobalStore, useUserPreferences, type WorkbenchViewMode } from '../../stores/use-global-store';
import type { StrategyType, UserPreferences } from '../../types/preferences';
import { BUILTIN_STRATEGY_OPTIONS, normalizeStrategyKey } from '../../types/strategy';
import { useTheme } from '../../theme';

const DEFAULT_PREFERENCES: UserPreferences = {
  defaultTheme: 'dark',
  autoRefreshInterval: 30_000,
  sidebarCollapsed: false,
  defaultStrategy: 'balanced',
};

const WORKBENCH_VIEW_OPTIONS: Array<{ value: WorkbenchViewMode; label: string }> = [
  { value: 'MATRIX', label: '矩阵' },
  { value: 'CALENDAR', label: '日历' },
  { value: 'CARD', label: '卡片' },
];

const UserPreferencesPanel: React.FC = () => {
  const preferences = useUserPreferences();
  const workbenchViewMode = useGlobalStore((state) => state.workbenchViewMode);
  const { updateUserPreferences, setWorkbenchViewMode } = useGlobalActions();
  const { setTheme } = useTheme();

  const customStrategyQuery = useQuery({
    queryKey: ['custom-strategies', 'user-preferences'],
    queryFn: () => configApi.listCustomStrategies(),
    staleTime: 60_000,
  });

  const strategyOptions = useMemo<Array<{ value: StrategyType; label: string }>>(
    () => [
      ...BUILTIN_STRATEGY_OPTIONS,
      ...(customStrategyQuery.data || []).map((profile) => ({
        value: `custom:${String(profile.strategy_id || '').trim()}` as StrategyType,
        label: `自定义 · ${String(profile.title || profile.strategy_id || '').trim()}`,
      })),
    ],
    [customStrategyQuery.data],
  );

  const strategyOptionValues = useMemo(
    () => new Set(strategyOptions.map((option) => option.value)),
    [strategyOptions],
  );

  const selectedStrategy = normalizeStrategyKey(preferences.defaultStrategy) as StrategyType;
  const strategyValue: StrategyType = strategyOptionValues.has(selectedStrategy)
    ? selectedStrategy
    : 'balanced';

  return (
    <Card>
      <Form layout="vertical">
        <Form.Item label="侧边栏折叠">
          <Switch
            checked={preferences.sidebarCollapsed}
            onChange={(checked) => updateUserPreferences({ sidebarCollapsed: checked })}
          />
        </Form.Item>

        <Form.Item label="自动刷新间隔（秒）">
          <InputNumber
            min={5}
            max={3600}
            value={Math.round(preferences.autoRefreshInterval / 1000)}
            onChange={(val) =>
              updateUserPreferences({ autoRefreshInterval: (Number(val) || 30) * 1000 })
            }
          />
        </Form.Item>

        <Form.Item label="默认主题">
          <Select
            value={preferences.defaultTheme}
            style={{ width: 160 }}
            onChange={(val) => {
              setTheme(val);
              updateUserPreferences({ defaultTheme: val });
            }}
            options={[
              { value: 'dark', label: '暗色' },
              { value: 'light', label: '亮色' },
            ]}
          />
        </Form.Item>

        <Form.Item label="默认策略">
          <Select
            value={strategyValue}
            style={{ width: 200 }}
            onChange={(val) => updateUserPreferences({ defaultStrategy: normalizeStrategyKey(val) as StrategyType })}
            options={strategyOptions}
            loading={customStrategyQuery.isLoading}
          />
        </Form.Item>

        <Form.Item label="工作台默认视图">
          <Select
            value={workbenchViewMode}
            style={{ width: 200 }}
            onChange={(val) => setWorkbenchViewMode(val)}
            options={WORKBENCH_VIEW_OPTIONS}
          />
        </Form.Item>

        <Space>
          <Button
            onClick={() => {
              setTheme(DEFAULT_PREFERENCES.defaultTheme);
              updateUserPreferences(DEFAULT_PREFERENCES);
              setWorkbenchViewMode('MATRIX');
            }}
          >
            恢复默认
          </Button>
        </Space>
      </Form>
    </Card>
  );
};

export default UserPreferencesPanel;
