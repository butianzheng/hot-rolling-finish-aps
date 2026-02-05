/**
 * 一键优化菜单组件
 *
 * 重构后：301 行 → ~55 行 (-82%)
 */

import React from 'react';
import { Button, Dropdown } from 'antd';
import { DownOutlined, ThunderboltOutlined } from '@ant-design/icons';
import type { OneClickOptimizeMenuProps, OptimizeMenuKey } from './types';
import { useOneClickOptimize } from './useOneClickOptimize';
import { PreviewModal } from './PreviewModal';
import { PostCreateModal } from './PostCreateModal';

const OneClickOptimizeMenu: React.FC<OneClickOptimizeMenuProps> = ({
  activeVersionId,
  operator,
  onBeforeExecute,
  onAfterExecute,
}) => {
  const workflow = useOneClickOptimize({
    activeVersionId,
    operator,
    onBeforeExecute,
    onAfterExecute,
  });

  return (
    <>
      <Dropdown
        disabled={!activeVersionId}
        menu={{
          onClick: ({ key }) => {
            const k = key as OptimizeMenuKey;
            if (k === 'preview' || k === 'execute') {
              workflow.openPreview();
              return;
            }
            if (k === 'balanced' || k === 'urgent_first' || k === 'capacity_first' || k === 'cold_stock_first') {
              workflow.openPreviewWithStrategy(k);
              return;
            }
          },
          items: [
            { key: 'preview', icon: <ThunderboltOutlined />, label: '预览（试算，不落库）' },
            { key: 'execute', icon: <ThunderboltOutlined />, label: '执行（重算，落库）' },
            { type: 'divider' },
            { key: 'balanced', label: '均衡方案' },
            { key: 'urgent_first', label: '紧急优先' },
            { key: 'capacity_first', label: '产能优先' },
            { key: 'cold_stock_first', label: '冷料消化' },
          ],
        }}
      >
        <Button icon={<ThunderboltOutlined />}>
          一键优化 <DownOutlined />
        </Button>
      </Dropdown>

      <PreviewModal
        open={workflow.previewOpen}
        strategyLabel={workflow.strategyLabel}
        strategy={workflow.strategy}
        baseDate={workflow.baseDate}
        windowDaysOverride={workflow.windowDaysOverride}
        simulateLoading={workflow.simulateLoading}
        executeLoading={workflow.executeLoading}
        simulateResult={workflow.simulateResult}
        activeVersionId={activeVersionId}
        onClose={workflow.closePreview}
        onExecute={workflow.runExecute}
        onSimulate={workflow.runSimulate}
        onBaseDateChange={workflow.changeBaseDate}
        onStrategyChange={workflow.changeStrategy}
        onWindowDaysOverrideChange={workflow.changeWindowDaysOverride}
      />

      <PostCreateModal
        open={workflow.postCreateOpen}
        createdVersionId={workflow.createdVersionId}
        postActionLoading={workflow.postActionLoading}
        onClose={workflow.closePostCreate}
        onSwitch={workflow.handleSwitch}
        onActivate={workflow.handleActivate}
      />
    </>
  );
};

export default React.memo(OneClickOptimizeMenu);
