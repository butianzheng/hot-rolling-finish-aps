/**
 * 刷新控制栏组件
 */

import React from 'react';
import { Button, Card, Select, Space, Switch, Tag, Tooltip } from 'antd';
import { CloseCircleOutlined, ReloadOutlined } from '@ant-design/icons';
import { REFRESH_INTERVAL_OPTIONS } from './types';

export interface RefreshControlBarProps {
  loading: boolean;
  autoRefreshEnabled: boolean;
  onAutoRefreshChange: (enabled: boolean) => void;
  refreshInterval: number;
  onRefreshIntervalChange: (interval: number) => void;
  lastRefreshTime: Date | null;
  nextRefreshCountdown: number;
  onManualRefresh: () => void;
}

// 格式化时间为 HH:mm:ss
function formatTime(date: Date | null): string {
  if (!date) return '从未刷新';
  return date.toLocaleTimeString('zh-CN', { hour12: false });
}

export const RefreshControlBar: React.FC<RefreshControlBarProps> = ({
  loading,
  autoRefreshEnabled,
  onAutoRefreshChange,
  refreshInterval,
  onRefreshIntervalChange,
  lastRefreshTime,
  nextRefreshCountdown,
  onManualRefresh,
}) => {
  return (
    <Card style={{ marginBottom: 16 }}>
      <Space wrap align="center" size="large">
        {/* 手动刷新按钮 */}
        <Button
          type="primary"
          icon={<ReloadOutlined />}
          onClick={onManualRefresh}
          loading={loading}
        >
          手动刷新
        </Button>

        {/* 自动刷新开关 */}
        <Space>
          <span>自动刷新:</span>
          <Switch
            checked={autoRefreshEnabled}
            onChange={onAutoRefreshChange}
            size="small"
          />
        </Space>

        {/* 刷新间隔选择 */}
        {autoRefreshEnabled && (
          <Space>
            <span>间隔:</span>
            <Select
              value={refreshInterval}
              onChange={onRefreshIntervalChange}
              style={{ width: 120 }}
              size="small"
            >
              {REFRESH_INTERVAL_OPTIONS.map((opt) => (
                <Select.Option key={opt.value} value={opt.value}>
                  {opt.label}
                </Select.Option>
              ))}
            </Select>
          </Space>
        )}

        {/* 刷新状态显示 */}
        <Space>
          {autoRefreshEnabled ? (
            <>
              <Tag color="green" icon={<ReloadOutlined />}>
                自动刷新中
              </Tag>
              {nextRefreshCountdown > 0 && (
                <Tooltip title="下次自动刷新倒计时">
                  <span style={{ color: '#666' }}>
                    {nextRefreshCountdown}s 后刷新
                  </span>
                </Tooltip>
              )}
            </>
          ) : (
            <Tag color="red" icon={<CloseCircleOutlined />}>
              自动刷新关闭
            </Tag>
          )}
        </Space>

        {/* 最后刷新时间 */}
        <Space>
          <span style={{ color: '#666' }}>
            最后刷新: <strong>{formatTime(lastRefreshTime)}</strong>
          </span>
        </Space>
      </Space>
    </Card>
  );
};

export default RefreshControlBar;
