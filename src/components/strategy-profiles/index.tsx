/**
 * 策略配置面板主组件
 *
 * 重构后：432 行 → ~50 行 (-88%)
 */

import React from 'react';
import { Alert, Space } from 'antd';
import { useStrategyProfiles } from './useStrategyProfiles';
import { PresetsCard } from './PresetsCard';
import { CustomProfilesTable } from './CustomProfilesTable';
import { StrategyFormModal } from './StrategyFormModal';

const StrategyProfilesPanel: React.FC = () => {
  const workflow = useStrategyProfiles();

  return (
    <Space direction="vertical" size={12} style={{ width: '100%' }}>
      <Alert
        type="info"
        showIcon
        message="策略配置说明"
        description={
          <div>
            <div>自定义策略用于沉淀"可复用的策略模板"（复制预设 → 调参 → 保存）。</div>
            <div>注意：当前版本自定义策略的参数仅做保存与展示；草案试算仍按"基于预设策略"执行（参数将在后续引擎支持后生效）。</div>
          </div>
        }
      />

      <PresetsCard
        presets={workflow.presets}
        loading={workflow.loading}
        onRefresh={workflow.loadAll}
        onCopyPreset={workflow.openCopyFromPreset}
        onCreateNew={() => workflow.openCreate('balanced')}
      />

      <CustomProfilesTable
        profiles={workflow.customProfiles}
        loading={workflow.loading}
        onEdit={workflow.openEdit}
        onCopy={workflow.openCopyFromCustom}
        onNavigateToDraft={(key) => workflow.navigate(`/comparison?tab=draft&strategies=${encodeURIComponent(key)}`)}
      />

      <StrategyFormModal
        open={workflow.modalOpen}
        mode={workflow.modalMode}
        saving={workflow.saving}
        form={workflow.form}
        baseStrategyOptions={workflow.baseStrategyOptions}
        onSave={workflow.handleSave}
        onCancel={workflow.closeModal}
      />
    </Space>
  );
};

export default StrategyProfilesPanel;
