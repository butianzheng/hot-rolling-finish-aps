/**
 * 版本对比模态框组件
 * 显示两个版本之间的详细对比结果（8个卡片）
 */

import React, { useMemo } from 'react';
import {
  Alert,
  Button,
  Card,
  Descriptions,
  Input,
  Modal,
  Select,
  Space,
  Table,
  Tag,
  Typography,
} from 'antd';
import type { EChartsOption } from 'echarts';
import type { BackendVersionComparisonResult, VersionDiff } from '../../types/comparison';
import { LocalCapacityDeltaRow, LocalVersionDiffSummary } from './types';

const LazyECharts = React.lazy(() => import('echarts-for-react'));

const Chart: React.FC<{ option: EChartsOption; height: number }> = ({ option, height }) => {
  return (
    <React.Suspense
      fallback={
        <div
          style={{
            height,
            width: '100%',
            background: '#fafafa',
            border: '1px dashed #d9d9d9',
            borderRadius: 6,
            display: 'flex',
            alignItems: 'center',
            justifyContent: 'center',
            color: '#8c8c8c',
            fontSize: 12,
          }}
        >
          图表加载中…
        </div>
      }
    >
      <LazyECharts option={option} style={{ height, width: '100%' }} notMerge lazyUpdate />
    </React.Suspense>
  );
};

export interface VersionComparisonModalProps {
  // 显示状态
  open: boolean;
  onClose: () => void;

  // 后端对比结果
  compareResult: BackendVersionComparisonResult | null;
  compareKpiRows: Array<{ key: string; metric: string; a: string; b: string; delta: string }>;
  compareKpiLoading?: boolean;
  compareKpiError?: Error | null;

  // 本地差异计算
  localDiffResult: {
    diffs: VersionDiff[];
    summary: LocalVersionDiffSummary;
  } | null;
  loadLocalCompareDetail: boolean;
  planItemsLoading?: boolean;
  planItemsErrorA?: Error | null;
  planItemsErrorB?: Error | null;

  // 产能分析
  localCapacityRows: {
    rows: LocalCapacityDeltaRow[];
    overflowRows: LocalCapacityDeltaRow[];
    totalA: number;
    totalB: number;
    dateFrom: string | null;
    dateTo: string | null;
    machines: string[];
  } | null;
  localCapacityRowsBase: {
    rows: LocalCapacityDeltaRow[];
    totalA: number;
    totalB: number;
    dateFrom: string | null;
    dateTo: string | null;
    machines: string[];
  } | null;
  capacityPoolsErrorA?: Error | null;
  capacityPoolsErrorB?: Error | null;
  showAllCapacityRows?: boolean;

  // 回顾性笔记
  retrospectiveNote?: string;
  retrospectiveSavedAt?: string | null;

  // 搜索和过滤
  diffSearchText?: string;
  diffTypeFilter?: 'ALL' | 'ADDED' | 'REMOVED' | 'MOVED' | 'MODIFIED';

  // 图表选项
  diffSummaryChartOption?: EChartsOption | null;
  capacityTrendOption?: EChartsOption | null;
  riskTrendOption?: EChartsOption | null;

  // 回调
  onActivateVersion?: (versionId: string) => Promise<void>;
  onLoadLocalCompareDetail?: () => void;
  onToggleShowAllCapacityRows?: () => void;
  onRetrospectiveNoteChange?: (note: string) => void;
  onRetrospectiveNoteSave?: () => void;
  onDiffSearchChange?: (text: string) => void;
  onDiffTypeFilterChange?: (type: 'ALL' | 'ADDED' | 'REMOVED' | 'MOVED' | 'MODIFIED') => void;
  onExportDiffs?: (format: 'csv' | 'json') => Promise<void>;
  onExportCapacity?: (format: 'csv' | 'json') => Promise<void>;
  onExportReport?: (format: 'json' | 'markdown' | 'html') => Promise<void>;
}

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

  return (
    <Modal
      title="版本对比结果"
      open={open}
      onCancel={onClose}
      footer={[
        compareResult ? (
          <Button
            key="activateA"
            onClick={() => onActivateVersion?.(compareResult.version_id_a)}
          >
            回滚到版本A
          </Button>
        ) : null,
        compareResult ? (
          <Button
            key="activateB"
            type="primary"
            onClick={() => onActivateVersion?.(compareResult.version_id_b)}
          >
            回滚到版本B
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
              <Descriptions.Item label="版本A">{compareResult.version_id_a}</Descriptions.Item>
              <Descriptions.Item label="版本B">{compareResult.version_id_b}</Descriptions.Item>
              <Descriptions.Item label="移动数量">{compareResult.moved_count}</Descriptions.Item>
              <Descriptions.Item label="新增数量">{compareResult.added_count}</Descriptions.Item>
              <Descriptions.Item label="删除数量">{compareResult.removed_count}</Descriptions.Item>
              <Descriptions.Item label="挤出数量">{compareResult.squeezed_out_count}</Descriptions.Item>
            </Descriptions>
          </Card>

          {/* KPI 对比卡片 */}
          <Card title="KPI 总览（后端聚合）" size="small">
            {compareKpiLoading ? (
              <Alert type="info" showIcon message="正在计算 KPI…" />
            ) : compareKpiError ? (
              <Alert
                type="error"
                showIcon
                message="KPI 计算失败"
                description={String((compareKpiError as any)?.message || compareKpiError)}
              />
            ) : !compareKpiRows || compareKpiRows.length === 0 ? (
              <Alert type="info" showIcon message="暂无 KPI 数据" />
            ) : (
              <Table
                size="small"
                pagination={false}
                rowKey={(r) => String((r as any).key)}
                dataSource={compareKpiRows}
                columns={[
                  { title: '指标', dataIndex: 'metric', width: 180 },
                  { title: '版本A', dataIndex: 'a', width: 160 },
                  { title: '版本B', dataIndex: 'b', width: 160 },
                  { title: 'Δ(B-A)', dataIndex: 'delta' },
                ]}
              />
            )}
          </Card>

          {/* 物料变更明细卡片 */}
          <Card
            title="物料变更明细（本地计算）"
            size="small"
            extra={
              <Space>
                <Button
                  size="small"
                  onClick={() => onLoadLocalCompareDetail?.()}
                  disabled={loadLocalCompareDetail}
                >
                  {loadLocalCompareDetail ? '已加载明细' : '加载明细'}
                </Button>
                <Button size="small" onClick={() => onExportDiffs?.('csv')} disabled={!localDiffResult}>
                  导出差异(CSV)
                </Button>
                <Button size="small" onClick={() => onExportDiffs?.('json')} disabled={!localDiffResult}>
                  导出差异(JSON)
                </Button>
              </Space>
            }
          >
            {!loadLocalCompareDetail ? (
              <Alert
                type="info"
                showIcon
                message="为提升性能，默认不加载全量排产明细"
                description="点击右上角「加载明细」后，将拉取两个版本的 plan_item 用于本地计算：变更明细/产能变化等。"
              />
            ) : planItemsLoading ? (
              <Alert type="info" showIcon message="正在加载排产明细，用于计算差异…" />
            ) : planItemsErrorA || planItemsErrorB ? (
              <Alert
                type="error"
                showIcon
                message="排产明细加载失败，无法生成本地差异"
                description={String((planItemsErrorA as any)?.message || planItemsErrorA || (planItemsErrorB as any)?.message || planItemsErrorB)}
              />
            ) : !localDiffResult ? (
              <Alert type="info" showIcon message="暂无差异数据" />
            ) : (
              <Space direction="vertical" style={{ width: '100%' }} size={10}>
                <Space wrap>
                  <Tag color="blue">总变更 {localDiffResult.summary.totalChanges}</Tag>
                  <Tag color="green">新增 {localDiffResult.summary.addedCount}</Tag>
                  <Tag color="red">删除 {localDiffResult.summary.removedCount}</Tag>
                  <Tag color="gold">移动 {localDiffResult.summary.movedCount}</Tag>
                  <Tag color="purple">修改 {localDiffResult.summary.modifiedCount}</Tag>
                </Space>

                {diffSummaryChartOption ? (
                  <Chart option={diffSummaryChartOption} height={180} />
                ) : (
                  <Typography.Text type="secondary" style={{ fontSize: 12 }}>
                    暂无可视化统计
                  </Typography.Text>
                )}

                <Space wrap>
                  <Input
                    placeholder="搜索物料/From/To…"
                    value={diffSearchText}
                    onChange={(e) => onDiffSearchChange?.(e.target.value)}
                    style={{ width: 260 }}
                    allowClear
                  />
                  <Select
                    value={diffTypeFilter}
                    style={{ width: 220 }}
                    onChange={(v) => onDiffTypeFilterChange?.(v)}
                    options={[
                      { value: 'ALL', label: '全部类型' },
                      { value: 'MOVED', label: '移动' },
                      { value: 'MODIFIED', label: '修改' },
                      { value: 'ADDED', label: '新增' },
                      { value: 'REMOVED', label: '删除' },
                    ]}
                  />
                  <Typography.Text type="secondary" style={{ fontSize: 12 }}>
                    显示 {filteredDiffs.length} / {localDiffResult.diffs.length}
                  </Typography.Text>
                </Space>

                <Table<VersionDiff>
                  size="small"
                  rowKey={(r) => r.materialId}
                  pagination={false}
                  dataSource={filteredDiffs}
                  columns={[
                    {
                      title: '类型',
                      dataIndex: 'changeType',
                      width: 90,
                      render: (v) => {
                        const color = v === 'REMOVED' ? 'red' : v === 'ADDED' ? 'green' : v === 'MOVED' ? 'gold' : 'purple';
                        return <Tag color={color}>{v}</Tag>;
                      },
                    },
                    {
                      title: '物料',
                      dataIndex: 'materialId',
                      width: 200,
                      render: (v) => (
                        <Typography.Text code copyable>
                          {String(v)}
                        </Typography.Text>
                      ),
                    },
                    {
                      title: 'From',
                      key: 'from',
                      width: 260,
                      render: (_, r) => {
                        const s = r.previousState;
                        return s ? `${s.machine_code}/${s.plan_date}/序${s.seq_no}` : '-';
                      },
                    },
                    {
                      title: 'To',
                      key: 'to',
                      width: 260,
                      render: (_, r) => {
                        const s = r.currentState;
                        return s ? `${s.machine_code}/${s.plan_date}/序${s.seq_no}` : '-';
                      },
                    },
                    {
                      title: '紧急',
                      key: 'urgent',
                      width: 90,
                      render: (_, r) => {
                        const u = r.currentState?.urgent_level ?? r.previousState?.urgent_level ?? '';
                        return u ? <Tag>{u}</Tag> : '-';
                      },
                    },
                    {
                      title: '重量',
                      key: 'weight',
                      width: 90,
                      render: (_, r) => {
                        const w = r.currentState?.weight_t ?? r.previousState?.weight_t ?? null;
                        if (w == null || !Number.isFinite(Number(w))) return '-';
                        return `${Number(w).toFixed(3)}t`;
                      },
                    },
                  ]}
                  virtual
                  scroll={{ y: 320 }}
                />
              </Space>
            )}
          </Card>

          {/* 产能变化卡片 */}
          <Card
            title="产能变化（本地计算）"
            size="small"
            extra={
              <Space>
                <Button
                  size="small"
                  onClick={() => onToggleShowAllCapacityRows?.()}
                  disabled={!localCapacityRowsBase}
                >
                  {showAllCapacityRows ? '仅看变化' : '查看全量'}
                </Button>
                <Button size="small" onClick={() => onExportCapacity?.('csv')} disabled={!localCapacityRows}>
                  导出产能(CSV)
                </Button>
                <Button size="small" onClick={() => onExportCapacity?.('json')} disabled={!localCapacityRows}>
                  导出产能(JSON)
                </Button>
              </Space>
            }
          >
            {!loadLocalCompareDetail ? (
              <Alert
                type="info"
                showIcon
                message="未加载排产明细"
                description="点击上方「物料变更明细」区域右上角的「加载明细」，即可生成本地产能变化分析。"
              />
            ) : !localCapacityRowsBase ? (
              <Alert type="info" showIcon message="暂无产能差异数据" />
            ) : (
              <Space direction="vertical" style={{ width: '100%' }} size={10}>
                {capacityTrendOption ? (
                  <Chart option={capacityTrendOption} height={220} />
                ) : (
                  <Typography.Text type="secondary" style={{ fontSize: 12 }}>
                    暂无产能趋势图
                  </Typography.Text>
                )}

                <Space wrap>
                  <Tag color="blue">总量A {localCapacityRowsBase.totalA.toFixed(1)}t</Tag>
                  <Tag color="blue">总量B {localCapacityRowsBase.totalB.toFixed(1)}t</Tag>
                  <Tag
                    color={
                      localCapacityRowsBase.totalB - localCapacityRowsBase.totalA >= 0
                        ? 'green'
                        : 'red'
                    }
                  >
                    Δ {(localCapacityRowsBase.totalB - localCapacityRowsBase.totalA).toFixed(1)}t
                  </Tag>
                  {localCapacityRows ? (
                    <Tag color={localCapacityRows.overflowRows.length > 0 ? 'red' : 'green'}>
                      预计超上限 {localCapacityRows.overflowRows.length}
                    </Tag>
                  ) : null}
                  <Typography.Text type="secondary" style={{ fontSize: 12 }}>
                    {localCapacityRowsBase.dateFrom || '-'} ~ {localCapacityRowsBase.dateTo || '-'} · 机组{' '}
                    {localCapacityRowsBase.machines.length}
                  </Typography.Text>
                </Space>

                {capacityPoolsErrorA || capacityPoolsErrorB ? (
                  <Alert
                    type="warning"
                    showIcon
                    message="产能池加载失败（仍可查看吨位差异）"
                    description={String(
                      (capacityPoolsErrorA as any)?.message ||
                        capacityPoolsErrorA ||
                        (capacityPoolsErrorB as any)?.message ||
                        capacityPoolsErrorB
                    )}
                  />
                ) : null}

                <Table<LocalCapacityDeltaRow>
                  size="small"
                  rowKey={(r) => `${r.machine_code}__${r.date}`}
                  pagination={false}
                  dataSource={localCapacityRows?.rows ?? localCapacityRowsBase.rows}
                  columns={[
                    { title: '日期', dataIndex: 'date', width: 120 },
                    { title: '机组', dataIndex: 'machine_code', width: 90 },
                    {
                      title: 'A已用(t)',
                      dataIndex: 'used_a',
                      width: 110,
                      render: (v: number) => v.toFixed(1),
                    },
                    {
                      title: 'B已用(t)',
                      dataIndex: 'used_b',
                      width: 110,
                      render: (v: number, r) => {
                        const threshold = r.limit_b ?? r.target_b ?? null;
                        const over = threshold != null && v > threshold + 1e-9;
                        return <span style={{ color: over ? '#cf1322' : undefined }}>{v.toFixed(1)}</span>;
                      },
                    },
                    {
                      title: 'Δ(t)',
                      dataIndex: 'delta',
                      width: 110,
                      render: (v: number) => (
                        <span style={{ color: v > 1e-9 ? '#3f8600' : v < -1e-9 ? '#cf1322' : undefined }}>
                          {v.toFixed(1)}
                        </span>
                      ),
                    },
                    {
                      title: 'B目标/上限',
                      key: 'capB',
                      width: 160,
                      render: (_, r) => {
                        const target = r.target_b;
                        const limit = r.limit_b;
                        if (target == null && limit == null) return '-';
                        const t = target == null ? '-' : target.toFixed(1);
                        const l = limit == null ? '-' : limit.toFixed(1);
                        return `${t} / ${l}`;
                      },
                    },
                  ]}
                  virtual
                  scroll={{ y: 320 }}
                />
              </Space>
            )}
          </Card>

          {/* 配置变化卡片 */}
          <Card title="配置变化" size="small">
            {compareResult.config_changes && compareResult.config_changes.length > 0 ? (
              <Table
                size="small"
                pagination={false}
                rowKey={(r) => r.key}
                dataSource={compareResult.config_changes}
                columns={[
                  { title: 'Key', dataIndex: 'key', width: 220 },
                  {
                    title: '版本A',
                    dataIndex: 'value_a',
                    render: (v) => (v == null ? '-' : String(v)),
                  },
                  {
                    title: '版本B',
                    dataIndex: 'value_b',
                    render: (v) => (v == null ? '-' : String(v)),
                  },
                ]}
                scroll={{ y: 240 }}
              />
            ) : (
              <Alert type="success" showIcon message="无配置变化" />
            )}
          </Card>

          {/* 风险/产能变化卡片 */}
          <Card title="风险/产能变化" size="small">
            <Space direction="vertical" style={{ width: '100%' }} size={10}>
              {compareResult.risk_delta ? (
                <Space direction="vertical" style={{ width: '100%' }} size={10}>
                  {riskTrendOption ? <Chart option={riskTrendOption} height={220} /> : null}
                  <Table
                    size="small"
                    pagination={false}
                    rowKey={(r) => `${r.date}`}
                    dataSource={compareResult.risk_delta}
                    columns={[
                      { title: '日期', dataIndex: 'date', width: 120 },
                      { title: 'A风险', dataIndex: 'risk_score_a', width: 120 },
                      { title: 'B风险', dataIndex: 'risk_score_b', width: 120 },
                      { title: 'Δ', dataIndex: 'risk_score_delta' },
                    ]}
                    scroll={{ y: 200 }}
                  />
                </Space>
              ) : (
                <Alert
                  type="info"
                  showIcon
                  message="风险变化对比暂不可用"
                  description="后端 compare_versions 当前未返回 risk_delta（待 RiskSnapshotRepository 支持）。"
                />
              )}

              {compareResult.capacity_delta ? (
                <Table
                  size="small"
                  pagination={false}
                  rowKey={(r) => `${r.machine_code}__${r.date}`}
                  dataSource={compareResult.capacity_delta}
                  columns={[
                    { title: '机组', dataIndex: 'machine_code', width: 90 },
                    { title: '日期', dataIndex: 'date', width: 120 },
                    { title: 'A已用', dataIndex: 'used_capacity_a', width: 120 },
                    { title: 'B已用', dataIndex: 'used_capacity_b', width: 120 },
                    { title: 'Δ', dataIndex: 'capacity_delta' },
                  ]}
                  scroll={{ y: 200 }}
                />
              ) : (
                <Alert
                  type="info"
                  showIcon
                  message="产能变化对比暂不可用"
                  description="后端 compare_versions 当前未返回 capacity_delta（待 CapacityPoolRepository 支持）。"
                />
              )}
            </Space>
          </Card>

          {/* 复盘总结卡片 */}
          <Card
            title="复盘总结"
            size="small"
            extra={
              <Space>
                <Button size="small" onClick={() => onRetrospectiveNoteSave?.()}>
                  保存总结
                </Button>
                <Button size="small" onClick={() => onExportReport?.('json')}>
                  导出报告(JSON)
                </Button>
                <Button size="small" onClick={() => onExportReport?.('markdown')} disabled={!compareResult}>
                  导出报告(MD)
                </Button>
                <Button size="small" onClick={() => onExportReport?.('html')} disabled={!compareResult}>
                  导出报告(HTML)
                </Button>
              </Space>
            }
          >
            <Space direction="vertical" style={{ width: '100%' }} size={8}>
              <Input.TextArea
                rows={5}
                value={retrospectiveNote}
                onChange={(e) => onRetrospectiveNoteChange?.(e.target.value)}
                placeholder="记录本次决策要点、代价与后续关注项（本地保存，不会写入数据库）。"
              />
              <Alert
                type="info"
                showIcon
                message={
                  retrospectiveSavedAt ? `已保存（本地）：${retrospectiveSavedAt}` : '未保存（本地）'
                }
              />
            </Space>
          </Card>
        </Space>
      )}
    </Modal>
  );
};

export default VersionComparisonModal;
