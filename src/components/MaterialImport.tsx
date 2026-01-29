/**
 * 材料导入主组件
 * 协调各子组件和工作流状态
 *
 * 重构后：1028 行 → ~180 行 (-82%)
 */

import React from 'react';
import { Alert, Tabs, Typography } from 'antd';
import { useNavigate } from 'react-router-dom';
import { useCurrentUser, useGlobalActions } from '../stores/use-global-store';
import { useImportWorkflow } from '../hooks/useImportWorkflow';
import { safeWriteImportHistory } from '../utils/importHistoryStorage';
import { ImportTabContent } from './material-import/ImportTabContent';
import { ConflictsTabContent } from './material-import/ConflictsTabContent';
import { HistoryTabContent } from './material-import/HistoryTabContent';
import { RawDataModal } from './material-import/RawDataModal';

const { Title, Paragraph } = Typography;

const MaterialImport: React.FC = () => {
  const navigate = useNavigate();
  const currentUser = useCurrentUser();
  const { setImporting } = useGlobalActions();

  const isTauriRuntime = typeof window !== 'undefined' && !!(window as any).__TAURI__;

  // 使用 Hook 获取所有状态和方法
  const workflow = useImportWorkflow();

  // 处理冲突解决
  const handleResolveConflict = async (
    conflictId: string,
    action: 'KEEP_EXISTING' | 'OVERWRITE' | 'MERGE',
  ) => {
    await workflow.handleResolveConflict(conflictId, action, { currentUser: currentUser || 'admin' });
  };

  // 处理查看冲突（从历史记录跳转）
  const handleViewConflicts = async (batchId: string) => {
    workflow.setConflictBatchId(batchId);
    workflow.setConflictStatus('OPEN');
    workflow.setActiveTab('conflicts');
    await workflow.loadConflicts({ status: 'OPEN', batchId, page: 1 });
  };

  // 刷新历史记录
  const handleRefreshHistory = () => {
    window.location.reload();
  };

  // 清空历史记录
  const handleClearHistory = () => {
    safeWriteImportHistory([]);
    window.location.reload();
  };

  // 复制 ID 到剪贴板
  const handleCopyId = (id: string) => {
    navigator.clipboard.writeText(id);
  };

  return (
    <div style={{ padding: 24 }}>
      <Title level={2} style={{ marginBottom: 0 }}>
        材料导入
      </Title>
      <Paragraph type="secondary" style={{ marginTop: 8 }}>
        当前导入通道基于后端 MaterialImporter：CSV 解析 → 字段映射 → 清洗/派生 → DQ 校验 → 冲突入队 →
        落库。
      </Paragraph>

      {!isTauriRuntime && (
        <Alert
          type="warning"
          showIcon
          message="当前运行环境不支持材料导入"
          description="材料导入依赖 Tauri 桌面端的文件选择与本地文件读取能力，请在 Tauri 窗口中使用该功能。"
          style={{ marginBottom: 16 }}
        />
      )}

      <Tabs
        activeKey={workflow.activeTab}
        onChange={(k) => {
          const key = k as 'import' | 'conflicts' | 'history';
          workflow.setActiveTab(key);
          if (key === 'conflicts') {
            workflow.loadConflicts({ page: 1 }).catch(() => void 0);
          }
        }}
        items={[
          {
            key: 'import',
            label: '导入',
            children: (
              <ImportTabContent
                isTauriRuntime={isTauriRuntime}
                currentUser={currentUser || 'admin'}
                selectedFilePath={workflow.selectedFilePath}
                previewHeaders={workflow.previewHeaders}
                previewRows={workflow.previewRows}
                previewTotalRows={workflow.previewTotalRows}
                previewLoading={workflow.previewLoading}
                missingHeaders={workflow.missingHeaders}
                batchId={workflow.batchId}
                onBatchIdChange={workflow.setBatchId}
                mappingProfileId={workflow.mappingProfileId}
                onMappingProfileIdChange={workflow.setMappingProfileId}
                importLoading={workflow.importLoading}
                importResult={workflow.importResult}
                dqStats={workflow.dqStats}
                onSelectFile={() => workflow.handleSelectFile(isTauriRuntime)}
                onImport={async () => {
                  await workflow.doImport({
                    currentUser: currentUser || 'admin',
                    setImporting,
                  });
                }}
                onRefreshPreview={workflow.loadPreview}
                onNavigateToWorkbench={() => navigate('/workbench')}
              />
            ),
          },
          {
            key: 'conflicts',
            label: '冲突处理',
            children: (
              <ConflictsTabContent
                conflictStatus={workflow.conflictStatus}
                onStatusChange={workflow.setConflictStatus}
                conflictBatchId={workflow.conflictBatchId}
                onBatchIdChange={workflow.setConflictBatchId}
                conflicts={workflow.conflicts}
                conflictPagination={workflow.conflictPagination}
                conflictsLoading={workflow.conflictsLoading}
                onLoadConflicts={workflow.loadConflicts}
                onResolveConflict={handleResolveConflict}
                onViewRawData={(title, content) =>
                  workflow.setRawModal({ open: true, title, content })
                }
              />
            ),
          },
          {
            key: 'history',
            label: '导入历史',
            children: (
              <HistoryTabContent
                importHistory={workflow.importHistory}
                onRefresh={handleRefreshHistory}
                onClearHistory={handleClearHistory}
                onViewConflicts={handleViewConflicts}
                onCopyId={handleCopyId}
              />
            ),
          },
        ]}
      />

      <RawDataModal
        open={workflow.rawModal.open}
        title={workflow.rawModal.title}
        content={workflow.rawModal.content}
        onClose={() => workflow.setRawModal({ open: false, title: '', content: '' })}
      />
    </div>
  );
};

export default MaterialImport;
