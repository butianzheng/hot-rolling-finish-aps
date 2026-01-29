/**
 * 预设策略卡片组件
 */

import React from 'react';
import { Button, Card, Space } from 'antd';
import { PlusOutlined, ReloadOutlined } from '@ant-design/icons';
import type { StrategyPresetRow } from './types';
import { DEFAULT_PRESETS } from './types';

interface PresetsCardProps {
  presets: StrategyPresetRow[];
  loading: boolean;
  onRefresh: () => void;
  onCopyPreset: (preset: StrategyPresetRow) => void;
  onCreateNew: () => void;
}

export const PresetsCard: React.FC<PresetsCardProps> = ({
  presets,
  loading,
  onRefresh,
  onCopyPreset,
  onCreateNew,
}) => {
  const displayPresets = presets.length ? presets : DEFAULT_PRESETS;

  return (
    <Card
      size="small"
      title="预设策略（可复制为自定义）"
      extra={
        <Button size="small" icon={<ReloadOutlined />} loading={loading} onClick={onRefresh}>
          刷新
        </Button>
      }
    >
      <Space wrap>
        {displayPresets.map((p) => (
          <Button key={`preset-${p.strategy}`} onClick={() => onCopyPreset(p)}>
            复制：{p.title}
          </Button>
        ))}

        <Button type="primary" icon={<PlusOutlined />} onClick={onCreateNew}>
          新建自定义策略
        </Button>
      </Space>
    </Card>
  );
};
