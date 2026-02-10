/**
 * 版本对比模态框组件
 *
 * 重构后：666 行 → ~120 行 (-82%)
 */

import React, { useMemo } from 'react';
import { Alert, Button, Card, Descriptions, Modal, Space, Table } from 'antd';
import type { VersionComparisonModalProps } from './types';
import { KpiCompareCard } from './KpiCompareCard';
import { MaterialDiffCard } from './MaterialDiffCard';
import { CapacityDeltaCard } from './CapacityDeltaCard';
import { RetrospectiveCard } from './RetrospectiveCard';
import { Chart } from './Chart';

export const VersionComparisonModal: React.FC<VersionComparisonModalProps> = ({
  open,
  onClose,
  compareResult,
  compareKpiRows,
  compareKpiLoading,
  compareKpiError,
  localDiffResult,
  loadLocalCompareDetail,
  planItemsLoading,
  planItemsErrorA,
  planItemsErrorB,
  localCapacityRows,
  localCapacityRowsBase,
  capacityPoolsErrorA,
  capacityPoolsErrorB,
  showAllCapacityRows = false,
  retrospectiveNote = '',
  retrospectiveSavedAt,
  diffSearchText = '',
  diffTypeFilter = 'ALL',
  diffSummaryChartOption,
  capacityTrendOption,
  riskTrendOption,
  onActivateVersion,
  onLoadLocalCompareDetail,
  onToggleShowAllCapacityRows,
  onRetrospectiveNoteChange,
  onRetrospectiveNoteSave,
  onDiffSearchChange,
  onDiffTypeFilterChange,
  onExportDiffs,
  onExportCapacity,
  onExportReport,
}) => {
  // 筛选差异
  const filteredDiffs = useMemo(() => {
    if (!localDiffResult) return [];
    let diffs = localDiffResult.diffs;

    if (diffTypeFilter !== 'ALL') {
      diffs = diffs.filter((d) => d.changeType === diffTypeFilter);
    }

    if (diffSearchText) {
      const search = diffSearchText.toLowerCase();
      diffs = diffs.filter((d) => {
        const id = String(d.materialId).toLowerCase();
        const from = d.previousState ? `${d.previousState.machine_code}/${d.previousState.plan_date}` : '';
        const to = d.currentState ? `${d.currentState.machine_code}/${d.currentState.plan_date}` : '';
        return id.includes(search) || from.includes(search) || to.includes(search);
      });
    }

    return diffs;
  }, [localDiffResult, diffTypeFilter, diffSearchText]);

  const hasBackendRiskDelta = Array.isArray(compareResult?.risk_delta) && compareResult.risk_delta.length > 0;
  const hasBackendCapacityDelta =
    Array.isArray(compareResult?.capacity_delta) && compareResult.capacity_delta.length > 0;

  return (
    <Modal
      title="版本对比结果"
      open={open}
      onCancel={onClose}
      footer={[
        compareResult ? (
          <Button key="activateA" onClick={() => onActivateVersion?.(compareResult.version_id_a)}>
            回滚到版本甲
          </Button>
        ) : null,
        compareResult ? (
          <Button key="activateB" type="primary" onClick={() => onActivateVersion?.(compareResult.version_id_b)}>
            回滚到版本乙
          </Button>
        ) : null,
        <Button key="close" onClick={onClose}>
          关闭
        </Button>,
      ]}
      width={1100}
      bodyStyle={{ maxHeight: 600, overflow: 'auto' }}
    >
      {compareResult && (
        <Space direction="vertical" style={{ width: '100%' }}>
          <Alert type="info" showIcon message={compareResult.message} />

          {/* 对比摘要卡片 */}
          <Card title="对比摘要" size="small">
            <Descriptions size="small" column={2} bordered>
              <Descriptions.Item label="版本甲">{compareResult.version_id_a}</Descriptions.Item>
              <Descriptions.Item label="版本乙">{compareResult.version_id_b}</Descriptions.Item>
              <Descriptions.Item label="移动数量">{compareResult.moved_count}</Descriptions.Item>
              <Descriptions.Item label="新增数量">{compareResult.added_count}</Descriptions.Item>
              <Descriptions.Item label="删除数量">{compareResult.removed_count}</Descriptions.Item>
              <Descriptions.Item label="挤出数量">{compareResult.squeezed_out_count}</Descriptions.Item>
            </Descriptions>
          </Card>

          {/* KPI 对比卡片 */}
          <KpiCompareCard loading={compareKpiLoading} error={compareKpiError} rows={compareKpiRows} />

          {/* 物料变更明细卡片 */}
          <MaterialDiffCard
            loadLocalCompareDetail={loadLocalCompareDetail}
            planItemsLoading={planItemsLoading}
            planItemsError={planItemsErrorA || planItemsErrorB}
            localDiffResult={localDiffResult}
            filteredDiffs={filteredDiffs}
            diffSearchText={diffSearchText}
            diffTypeFilter={diffTypeFilter}
            diffSummaryChartOption={diffSummaryChartOption}
            onLoadLocalCompareDetail={onLoadLocalCompareDetail}
            onDiffSearchChange={onDiffSearchChange}
            onDiffTypeFilterChange={onDiffTypeFilterChange}
            onExportDiffs={onExportDiffs}
          />

          {/* 产能变化卡片 */}
          <CapacityDeltaCard
            loadLocalCompareDetail={loadLocalCompareDetail}
            localCapacityRows={localCapacityRows}
            localCapacityRowsBase={localCapacityRowsBase}
            capacityPoolsError={capacityPoolsErrorA || capacityPoolsErrorB}
            showAllCapacityRows={showAllCapacityRows}
            capacityTrendOption={capacityTrendOption}
            onToggleShowAllCapacityRows={onToggleShowAllCapacityRows}
            onExportCapacity={onExportCapacity}
          />

          {/* 配置变化卡片 */}
          <Card title="配置变化" size="small">
            {compareResult.config_changes && compareResult.config_changes.length > 0 ? (
              <Table
                size="small"
                pagination={false}
                rowKey={(r) => r.key}
                dataSource={compareResult.config_changes}
                columns={[
                  { title: '配置键', dataIndex: 'key', width: 220 },
                  { title: '版本甲', dataIndex: 'value_a', render: (v) => (v == null ? '-' : String(v)) },
                  { title: '版本乙', dataIndex: 'value_b', render: (v) => (v == null ? '-' : String(v)) },
                ]}
                scroll={{ y: 240 }}
              />
            ) : (
              <Alert type="success" showIcon message="无配置变化" />
            )}
          </Card>

          {/* 风险/产能变化卡片 */}
          <Card title="风险/产能变化（后端口径）" size="small">
            <Space direction="vertical" style={{ width: '100%' }} size={10}>
              {hasBackendRiskDelta ? (
                <Space direction="vertical" style={{ width: '100%' }} size={10}>
                  {riskTrendOption ? <Chart option={riskTrendOption} height={220} /> : null}
                  <Table
                    size="small"
                    pagination={false}
                    rowKey={(r) => `${r.date}`}
                    dataSource={compareResult.risk_delta ?? []}
                    columns={[
                      { title: '日期', dataIndex: 'date', width: 120 },
                      { title: '版本甲风险', dataIndex: 'risk_score_a', width: 120 },
                      { title: '版本乙风险', dataIndex: 'risk_score_b', width: 120 },
                      { title: '变化值', dataIndex: 'risk_score_delta' },
                    ]}
                    scroll={{ y: 200 }}
                  />
                </Space>
              ) : (
                <Alert
                  type="info"
                  showIcon
                  message="风险变化明细暂不可用"
                  description="当前对比版本缺少风险快照数据。可结合物料变更明细和本地补算卡片辅助判断。"
                />
              )}

              {hasBackendCapacityDelta ? (
                <Table
                  size="small"
                  pagination={false}
                  rowKey={(r) => `${r.machine_code}__${r.date}`}
                  dataSource={compareResult.capacity_delta ?? []}
                  columns={[
                    { title: '机组', dataIndex: 'machine_code', width: 90 },
                    { title: '日期', dataIndex: 'date', width: 120 },
                    { title: '版本甲已用', dataIndex: 'used_capacity_a', width: 120 },
                    { title: '版本乙已用', dataIndex: 'used_capacity_b', width: 120 },
                    { title: '变化值', dataIndex: 'capacity_delta' },
                  ]}
                  scroll={{ y: 200 }}
                />
              ) : (
                <Alert
                  type="info"
                  showIcon
                  message="产能变化明细暂不可用"
                  description="当前对比版本缺少产能池快照数据。可在「产能变化（后端优先，缺失时本地补算）」卡片查看本地补算结果。"
                />
              )}
            </Space>
          </Card>

          {/* 复盘总结卡片 */}
          <RetrospectiveCard
            compareResult={compareResult}
            retrospectiveNote={retrospectiveNote}
            retrospectiveSavedAt={retrospectiveSavedAt}
            onRetrospectiveNoteChange={onRetrospectiveNoteChange}
            onRetrospectiveNoteSave={onRetrospectiveNoteSave}
            onExportReport={onExportReport}
          />
        </Space>
      )}
    </Modal>
  );
};

export default VersionComparisonModal;
