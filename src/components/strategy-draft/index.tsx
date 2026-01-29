/**
 * 策略草案对比主组件
 * 协调各子组件和工作流状态
 *
 * 重构后：1710 行 → ~120 行 (-93%)
 */

import React from 'react';
import { Col, Row } from 'antd';
import { useStrategyDraftComparison } from '../../hooks/useStrategyDraftComparison';
import { StrategyDraftControls } from './StrategyDraftControls';
import { KpiOverviewCard } from './KpiOverviewCard';
import { StrategyDraftCard } from './StrategyDraftCard';
import { StrategyDraftDetailDrawer } from './StrategyDraftDetailDrawer';
import MaterialDetailModal from './MaterialDetailModal';
import { PostPublishModal } from './PostPublishModal';

const StrategyDraftComparison: React.FC = () => {
  const workflow = useStrategyDraftComparison();

  return (
    <div style={{ padding: 12 }}>
      {/* 控制卡片 */}
      <StrategyDraftControls
        activeVersionId={workflow.activeVersionId}
        range={workflow.range}
        headerHint={workflow.headerHint}
        strategies={workflow.strategies}
        selectedStrategies={workflow.selectedStrategies}
        strategyTitleMap={workflow.strategyTitleMap}
        canGenerate={workflow.canGenerate}
        isGenerating={workflow.isGenerating}
        onRangeChange={workflow.setRange}
        onSelectedStrategiesChange={workflow.setSelectedStrategies}
        onGenerate={workflow.handleGenerate}
        onNavigateSettings={() => workflow.navigate('/settings?tab=strategy')}
        onNavigateHistorical={() => workflow.navigate('/comparison?tab=historical')}
        onNavigateWorkbench={() => workflow.navigate('/workbench')}
      />

      {/* KPI 总览 */}
      {workflow.hasAnyDraft && workflow.selectedStrategyKeysInOrder.length > 0 && (
        <KpiOverviewCard
          selectedStrategyKeysInOrder={workflow.selectedStrategyKeysInOrder}
          draftsByStrategy={workflow.draftsByStrategy}
          strategyTitleMap={workflow.strategyTitleMap}
          recommendation={workflow.recommendation}
          rangeDays={workflow.rangeDays}
        />
      )}

      {/* 策略卡片网格 */}
      <Row gutter={[12, 12]}>
        {workflow.strategies
          .filter((s) => workflow.selectedStrategies.includes(s.key))
          .map((s) => (
            <Col key={s.key} xs={24} sm={12} lg={6}>
              <StrategyDraftCard
                strategy={s}
                draft={workflow.draftsByStrategy[s.key]}
                strategyTitleMap={workflow.strategyTitleMap}
                activeVersionId={workflow.activeVersionId}
                publishingDraftId={workflow.publishingDraftId}
                onApply={workflow.handleApply}
                onOpenDetail={workflow.openDetail}
              />
            </Col>
          ))}
      </Row>

      {/* 发布后弹窗 */}
      <PostPublishModal
        open={workflow.postPublishOpen}
        createdVersionId={workflow.createdVersionId}
        postActionLoading={workflow.postActionLoading}
        onClose={workflow.closePostPublish}
        onGoHistorical={() => {
          workflow.closePostPublish();
          workflow.navigate('/comparison?tab=historical');
        }}
        onSwitch={workflow.handlePostSwitch}
        onActivate={workflow.handlePostActivate}
      />

      {/* 变更明细抽屉 */}
      <StrategyDraftDetailDrawer
        open={workflow.detailOpen}
        loading={workflow.detailLoading}
        draft={workflow.detailDraft}
        detailResp={workflow.detailResp}
        detailItems={workflow.detailItems}
        filter={workflow.detailFilter}
        search={workflow.detailSearch}
        strategyTitleMap={workflow.strategyTitleMap}
        squeezedHintCache={workflow.squeezedHintCache}
        range={workflow.range}
        onClose={workflow.closeDetail}
        onFilterChange={workflow.setDetailFilter}
        onSearchChange={workflow.setDetailSearch}
        onOpenMaterialDetail={workflow.openMaterialDetail}
        onEnsureSqueezedHint={workflow.ensureSqueezedHint}
      />

      {/* 物料详情弹窗 */}
      <MaterialDetailModal
        open={workflow.materialModalOpen}
        loading={workflow.materialModalLoading}
        context={workflow.materialModalContext}
        data={workflow.materialModalData}
        error={workflow.materialModalError}
        logsLoading={workflow.materialModalLogsLoading}
        logsError={workflow.materialModalLogsError}
        logs={workflow.materialModalLogs}
        range={workflow.range}
        onClose={workflow.closeMaterialModal}
        onGoWorkbench={(id) => {
          workflow.closeMaterialModal();
          workflow.navigate(`/workbench?material_id=${encodeURIComponent(id)}`);
        }}
      />
    </div>
  );
};

export default StrategyDraftComparison;
