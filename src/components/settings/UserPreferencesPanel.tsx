import React from 'react';
import { Button, Card, Form, InputNumber, Select, Space, Switch } from 'antd';
import { useGlobalActions, useGlobalStore, useUserPreferences, type WorkbenchViewMode } from '../../stores/use-global-store';
import type { StrategyType, UserPreferences } from '../../types/preferences';
import { useTheme } from '../../theme';

const DEFAULT_PREFERENCES: UserPreferences = {
  defaultTheme: 'dark',
  autoRefreshInterval: 30_000,
  sidebarCollapsed: false,
  defaultStrategy: 'balanced',
};

const STRATEGY_OPTIONS: Array<{ value: StrategyType; label: string }> = [
  { value: 'balanced', label: '均衡方案' },
  { value: 'urgent_first', label: '紧急优先' },
  { value: 'capacity_first', label: '产能优先' },
  { value: 'cold_stock_first', label: '冷料消化' },
  { value: 'manual', label: '手动调整' },
];

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
            value={preferences.defaultStrategy}
            style={{ width: 200 }}
            onChange={(val) => updateUserPreferences({ defaultStrategy: val })}
            options={STRATEGY_OPTIONS}
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
