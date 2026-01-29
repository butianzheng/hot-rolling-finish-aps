import React, { useMemo, useState } from 'react';
import { Alert, Button, Descriptions, Drawer, Modal, Space, Table, Tag, Typography, theme } from 'antd';
import type { ColumnsType } from 'antd/es/table';
import type { DrilldownSpec, WorkbenchTabKey } from '../../hooks/useRiskOverviewData';
import type {
  BottleneckPoint,
  CapacityOpportunity,
  ColdStockBucket,
  DaySummary,
  OrderFailure,
  ReasonItem,
  RollCampaignAlert,
} from '../../types/decision';
import {
  BOTTLENECK_LEVEL_COLORS,
  OPPORTUNITY_TYPE_COLORS,
  OPPORTUNITY_TYPE_LABELS,
  getBottleneckTypeColor,
  getBottleneckTypeLabel,
  getPressureLevelColor,
  getPressureLevelLabel,
  getRiskLevelColor,
  getRiskLevelLabel,
  getUrgencyLevelColor,
  getUrgencyLevelLabel,
  parseAlertLevel,
} from '../../types/decision';

const { Text } = Typography;

interface DrilldownDrawerProps {
  open: boolean;
  onClose: () => void;
  spec: DrilldownSpec | null;
  loading?: boolean;
  error?: unknown;
  onRetry?: () => void;
  onGoWorkbench?: (opts: {
    workbenchTab?: WorkbenchTabKey;
    machineCode?: string | null;
    urgencyLevel?: string | null;
  }) => void;

  riskDays: DaySummary[];
  bottlenecks: BottleneckPoint[];
  orderFailures: OrderFailure[];
  coldStockBuckets: ColdStockBucket[];
  rollAlerts: RollCampaignAlert[];
  capacityOpportunities: CapacityOpportunity[];
}

function titleFor(spec: DrilldownSpec | null) {
  if (!spec) return '详情';
  switch (spec.kind) {
    case 'orders':
      return '订单失败集合';
    case 'coldStock':
      return '冷坨高压力';
    case 'bottleneck':
      return '堵塞矩阵';
    case 'roll':
      return '换辊警报';
    case 'risk':
      return '风险摘要';
    case 'capacityOpportunity':
      return '容量优化机会';
    default:
      return '详情';
  }
}

function parseOpportunityType(typeStr: string) {
  const upper = String(typeStr || '').toUpperCase().replace(/-/g, '_');
  if ((Object.keys(OPPORTUNITY_TYPE_COLORS) as string[]).includes(upper)) return upper;
  return 'UNDERUTILIZED';
}

const DrilldownDrawer: React.FC<DrilldownDrawerProps> = ({
  open,
  onClose,
  spec,
  loading,
  error,
  onRetry,
  onGoWorkbench,
  riskDays,
  bottlenecks,
  orderFailures,
  coldStockBuckets,
  rollAlerts,
  capacityOpportunities,
}) => {
  const { token } = theme.useToken();
  const [detailOpen, setDetailOpen] = useState(false);
  const [detailRecord, setDetailRecord] = useState<any>(null);

  const content = useMemo(() => {
    if (!spec) return null;

    if (spec.kind === 'orders') {
      const rows = spec.urgency ? orderFailures.filter((o) => o.urgencyLevel === spec.urgency) : orderFailures;

      const columns: ColumnsType<OrderFailure> = [
        { title: '合同号', dataIndex: 'contractNo', key: 'contractNo', width: 140, ellipsis: true },
        {
          title: '紧急等级',
          dataIndex: 'urgencyLevel',
          key: 'urgencyLevel',
          width: 110,
          render: (v: OrderFailure['urgencyLevel']) => (
            <Tag color={getUrgencyLevelColor(v)}>{getUrgencyLevelLabel(v)}</Tag>
          ),
        },
        { title: '交期', dataIndex: 'dueDate', key: 'dueDate', width: 110 },
        {
          title: '完成率',
          dataIndex: 'completionRate',
          key: 'completionRate',
          width: 90,
          render: (v: number) => `${Number(v || 0).toFixed(1)}%`,
        },
        { title: '未排(吨)', dataIndex: 'unscheduledWeightT', key: 'unscheduledWeightT', width: 100 },
        { title: '机组', dataIndex: 'machineCode', key: 'machineCode', width: 90 },
        {
          title: '操作',
          key: 'action',
          width: onGoWorkbench ? 160 : 90,
          render: (_, record) => (
            <Space size={8}>
              {onGoWorkbench ? (
                <Button
                  size="small"
                  type="primary"
                  onClick={() =>
                    onGoWorkbench({
                      workbenchTab: 'visualization',
                      machineCode: record.machineCode,
                      urgencyLevel: record.urgencyLevel,
                    })
                  }
                >
                  处理
                </Button>
              ) : null}
              <Button
                size="small"
                onClick={() => {
                  setDetailRecord(record);
                  setDetailOpen(true);
                }}
              >
                详情
              </Button>
            </Space>
          ),
        },
      ];

      return (
        <Table
          rowKey={(r) => `${r.contractNo}-${r.dueDate}-${r.machineCode}`}
          size="small"
          columns={columns}
          dataSource={rows}
          pagination={{ pageSize: 20 }}
        />
      );
    }

    if (spec.kind === 'coldStock') {
      const levelOrder: Record<ColdStockBucket['pressureLevel'], number> = {
        CRITICAL: 3,
        HIGH: 2,
        MEDIUM: 1,
        LOW: 0,
      };

      const ageOrder: Record<ColdStockBucket['ageBin'], number> = {
        '30+': 3,
        '15-30': 2,
        '8-14': 1,
        '0-7': 0,
      };

      const rows = [...coldStockBuckets].sort((a, b) => {
        const sa = spec.machineCode && a.machineCode === spec.machineCode ? 0 : 1;
        const sb = spec.machineCode && b.machineCode === spec.machineCode ? 0 : 1;
        if (sa !== sb) return sa - sb;

        const la = levelOrder[a.pressureLevel] ?? 0;
        const lb = levelOrder[b.pressureLevel] ?? 0;
        if (la !== lb) return lb - la;
        if (a.pressureScore !== b.pressureScore) return b.pressureScore - a.pressureScore;
        const aa = ageOrder[a.ageBin] ?? 0;
        const ab = ageOrder[b.ageBin] ?? 0;
        if (aa !== ab) return ab - aa;
        if (a.maxAgeDays !== b.maxAgeDays) return b.maxAgeDays - a.maxAgeDays;
        if (a.machineCode !== b.machineCode) return a.machineCode.localeCompare(b.machineCode);
        return a.ageBin.localeCompare(b.ageBin);
      });

      const hasBucketFilter = !!(spec.ageBin || spec.pressureLevel);
      const selectedBucket = hasBucketFilter
        ? coldStockBuckets.find((b) => {
            if (spec.machineCode && b.machineCode !== spec.machineCode) return false;
            if (spec.ageBin && b.ageBin !== spec.ageBin) return false;
            if (spec.pressureLevel && b.pressureLevel !== spec.pressureLevel) return false;
            return true;
          }) || null
        : null;

      const selectedMachineBuckets = spec.machineCode
        ? coldStockBuckets.filter((b) => b.machineCode === spec.machineCode)
        : [];

      const machineSummary = spec.machineCode
        ? selectedMachineBuckets.reduce(
            (acc, cur) => {
              acc.count += Number(cur.count || 0);
              acc.weightT += Number(cur.weightT || 0);
              acc.highPressureCount +=
                cur.pressureLevel === 'HIGH' || cur.pressureLevel === 'CRITICAL' ? Number(cur.count || 0) : 0;
              acc.maxPressureScore = Math.max(acc.maxPressureScore, Number(cur.pressureScore || 0));
              acc.maxAgeDays = Math.max(acc.maxAgeDays, Number(cur.maxAgeDays || 0));
              return acc;
            },
            { count: 0, weightT: 0, highPressureCount: 0, maxPressureScore: 0, maxAgeDays: 0 }
          )
        : null;

      const worstBucketForMachine =
        !hasBucketFilter && spec.machineCode
          ? [...selectedMachineBuckets].sort((a, b) => {
              const la = levelOrder[a.pressureLevel] ?? 0;
              const lb = levelOrder[b.pressureLevel] ?? 0;
              if (la !== lb) return lb - la;
              if (a.pressureScore !== b.pressureScore) return b.pressureScore - a.pressureScore;
              const aa = ageOrder[a.ageBin] ?? 0;
              const ab = ageOrder[b.ageBin] ?? 0;
              if (aa !== ab) return ab - aa;
              return b.maxAgeDays - a.maxAgeDays;
            })[0] || null
          : null;

      const displayedBucket = selectedBucket || worstBucketForMachine;

      const reasonColumns: ColumnsType<ReasonItem> = [
        { title: '代码', dataIndex: 'code', key: 'code', width: 120, render: (v: string) => <Tag>{v}</Tag> },
        { title: '原因', dataIndex: 'msg', key: 'msg', ellipsis: true },
        {
          title: '权重',
          dataIndex: 'weight',
          key: 'weight',
          width: 90,
          render: (v: number) => `${(Number(v || 0) * 100).toFixed(1)}%`,
        },
        {
          title: '影响数',
          dataIndex: 'affectedCount',
          key: 'affectedCount',
          width: 90,
          render: (v?: number) => (typeof v === 'number' ? v : '-'),
        },
      ];

      const columns: ColumnsType<ColdStockBucket> = [
        { title: '机组', dataIndex: 'machineCode', key: 'machineCode', width: 90 },
        { title: '库龄', dataIndex: 'ageBin', key: 'ageBin', width: 90 },
        {
          title: '压力',
          dataIndex: 'pressureLevel',
          key: 'pressureLevel',
          width: 120,
          render: (v: ColdStockBucket['pressureLevel']) => (
            <Tag color={getPressureLevelColor(v)}>{getPressureLevelLabel(v)}</Tag>
          ),
        },
        {
          title: '分数',
          dataIndex: 'pressureScore',
          key: 'pressureScore',
          width: 90,
          render: (v: number) => (Number.isFinite(v) ? Number(v).toFixed(0) : '-'),
        },
        { title: '数量', dataIndex: 'count', key: 'count', width: 80 },
        { title: '重量(吨)', dataIndex: 'weightT', key: 'weightT', width: 100 },
        { title: '平均库龄', dataIndex: 'avgAgeDays', key: 'avgAgeDays', width: 100 },
        { title: '最大库龄', dataIndex: 'maxAgeDays', key: 'maxAgeDays', width: 100 },
        {
          title: '结构缺口',
          dataIndex: 'structureGap',
          key: 'structureGap',
          width: 180,
          ellipsis: true,
          render: (v: string) => {
            const s = String(v || '').trim();
            if (!s || s === 'NONE' || s === '无') return '-';
            return s;
          },
        },
        {
          title: '首因',
          dataIndex: 'reasons',
          key: 'reasons',
          width: 260,
          ellipsis: true,
          render: (reasons: ReasonItem[]) =>
            Array.isArray(reasons) && reasons.length > 0 ? String(reasons[0]?.msg || '-') : '-',
        },
        {
          title: '操作',
          key: 'action',
          width: onGoWorkbench ? 160 : 90,
          render: (_: unknown, record: ColdStockBucket) => (
            <Space size={8}>
              {onGoWorkbench ? (
                <Button
                  size="small"
                  type="primary"
                  onClick={() =>
                    onGoWorkbench({
                      workbenchTab: 'materials',
                      machineCode: record.machineCode,
                    })
                  }
                >
                  处理
                </Button>
              ) : null}
              <Button
                size="small"
                onClick={() => {
                  setDetailRecord(record);
                  setDetailOpen(true);
                }}
              >
                详情
              </Button>
            </Space>
          ),
        },
      ];

      return (
        <Space direction="vertical" size={12} style={{ width: '100%' }}>
          {machineSummary ? (
            <Space wrap align="center">
              <Tag color="blue">{spec.machineCode}</Tag>
              <Text type="secondary">
                冷坨 {machineSummary.count} 件 · {machineSummary.weightT.toFixed(1)}t · 高压 {machineSummary.highPressureCount} 件 · 峰值 {machineSummary.maxPressureScore.toFixed(0)}
              </Text>
              {onGoWorkbench ? (
                <Button
                  size="small"
                  type="primary"
                  onClick={() =>
                    onGoWorkbench({
                      workbenchTab: 'materials',
                      machineCode: spec.machineCode,
                    })
                  }
                >
                  去处理
                </Button>
              ) : null}
            </Space>
          ) : null}

          {displayedBucket ? (
            <>
              <Space wrap align="center">
                <Tag color={getPressureLevelColor(displayedBucket.pressureLevel)}>
                  {getPressureLevelLabel(displayedBucket.pressureLevel)} / {displayedBucket.pressureScore.toFixed(0)}
                </Tag>
                <Text strong>
                  {displayedBucket.machineCode} · {displayedBucket.ageBin}
                </Text>
                {!hasBucketFilter && spec.machineCode ? (
                  <Tag style={{ marginInlineEnd: 0 }}>最高压力桶</Tag>
                ) : null}
              </Space>

              <Descriptions column={4} bordered size="small">
                <Descriptions.Item label="数量">{displayedBucket.count}</Descriptions.Item>
                <Descriptions.Item label="重量">{displayedBucket.weightT.toFixed(1)}t</Descriptions.Item>
                <Descriptions.Item label="平均库龄">{displayedBucket.avgAgeDays.toFixed(1)}天</Descriptions.Item>
                <Descriptions.Item label="最大库龄">{displayedBucket.maxAgeDays}天</Descriptions.Item>
              </Descriptions>

              {String(displayedBucket.structureGap || '').trim() &&
              String(displayedBucket.structureGap || '').trim() !== 'NONE' &&
              String(displayedBucket.structureGap || '').trim() !== '无' ? (
                <div>
                  <Text strong>结构缺口</Text>
                  <div style={{ marginTop: 6, color: token.colorTextSecondary }}>
                    {String(displayedBucket.structureGap || '').trim()}
                  </div>
                </div>
              ) : null}

              {Array.isArray(displayedBucket.reasons) && displayedBucket.reasons.length > 0 ? (
                <Table
                  rowKey={(r) => r.code}
                  size="small"
                  columns={reasonColumns}
                  dataSource={displayedBucket.reasons}
                  pagination={false}
                />
              ) : (
                <Text type="secondary">暂无压库原因明细</Text>
              )}
            </>
          ) : null}

          <Table
            rowKey={(r) => `${r.machineCode}-${r.ageBin}-${r.pressureLevel}`}
            size="small"
            columns={columns}
            dataSource={rows}
            pagination={{ pageSize: 20 }}
            scroll={{ x: 1180 }}
            onRow={(record) => ({
              style:
                (spec.machineCode || spec.ageBin || spec.pressureLevel) &&
                (!spec.machineCode || record.machineCode === spec.machineCode) &&
                (!spec.ageBin || record.ageBin === spec.ageBin) &&
                (!spec.pressureLevel || record.pressureLevel === spec.pressureLevel)
                  ? { background: token.colorFillQuaternary }
                  : undefined,
            })}
          />
        </Space>
      );
    }

    if (spec.kind === 'bottleneck') {
      const rows = [...bottlenecks].sort((a, b) => b.bottleneckScore - a.bottleneckScore);
      const selectedPoint =
        spec.machineCode && spec.planDate
          ? bottlenecks.find((p) => p.machineCode === spec.machineCode && p.planDate === spec.planDate) || null
          : null;

      const reasonColumns: ColumnsType<ReasonItem> = [
        { title: '代码', dataIndex: 'code', key: 'code', width: 120, render: (v: string) => <Tag>{v}</Tag> },
        { title: '原因', dataIndex: 'msg', key: 'msg', ellipsis: true },
        {
          title: '权重',
          dataIndex: 'weight',
          key: 'weight',
          width: 90,
          render: (v: number) => `${(Number(v || 0) * 100).toFixed(1)}%`,
        },
        {
          title: '影响数',
          dataIndex: 'affectedCount',
          key: 'affectedCount',
          width: 90,
          render: (v?: number) => (typeof v === 'number' ? v : '-'),
        },
      ];

      const columns: ColumnsType<BottleneckPoint> = [
        { title: '机组', dataIndex: 'machineCode', key: 'machineCode', width: 90 },
        { title: '日期', dataIndex: 'planDate', key: 'planDate', width: 110 },
        {
          title: '等级',
          dataIndex: 'bottleneckLevel',
          key: 'bottleneckLevel',
          width: 110,
          render: (v: BottleneckPoint['bottleneckLevel']) => (
            <Tag color={BOTTLENECK_LEVEL_COLORS[v] || '#8c8c8c'}>{v}</Tag>
          ),
        },
        { title: '分数', dataIndex: 'bottleneckScore', key: 'bottleneckScore', width: 90 },
        {
          title: '利用率',
          dataIndex: 'capacityUtilPct',
          key: 'capacityUtilPct',
          width: 100,
          render: (v: number) => `${Number(v || 0).toFixed(1)}%`,
        },
        { title: '待排数', dataIndex: 'pendingMaterialCount', key: 'pendingMaterialCount', width: 90 },
        { title: '待排(吨)', dataIndex: 'pendingWeightT', key: 'pendingWeightT', width: 100 },
        {
          title: '操作',
          key: 'action',
          width: onGoWorkbench ? 160 : 90,
          render: (_, record) => (
            <Space size={8}>
              {onGoWorkbench ? (
                <Button
                  size="small"
                  type="primary"
                  onClick={() =>
                    onGoWorkbench({
                      workbenchTab: 'capacity',
                      machineCode: record.machineCode,
                    })
                  }
                >
                  处理
                </Button>
              ) : null}
              <Button
                size="small"
                onClick={() => {
                  setDetailRecord(record);
                  setDetailOpen(true);
                }}
              >
                详情
              </Button>
            </Space>
          ),
        },
      ];

      return (
        <Space direction="vertical" size={12} style={{ width: '100%' }}>
          {selectedPoint ? (
            <>
              <Space wrap align="center">
                <Tag color={BOTTLENECK_LEVEL_COLORS[selectedPoint.bottleneckLevel] || '#8c8c8c'}>
                  {selectedPoint.bottleneckLevel} / {selectedPoint.bottleneckScore.toFixed(1)}
                </Tag>
                <Text strong>
                  {selectedPoint.machineCode} · {selectedPoint.planDate}
                </Text>
                {onGoWorkbench ? (
                  <Button
                    size="small"
                    type="primary"
                    onClick={() => onGoWorkbench({ workbenchTab: 'capacity', machineCode: selectedPoint.machineCode })}
                  >
                    去处理
                  </Button>
                ) : null}
              </Space>

              <Descriptions column={4} bordered size="small">
                <Descriptions.Item label="堵塞分数">
                  {selectedPoint.bottleneckScore.toFixed(1)}
                </Descriptions.Item>
                <Descriptions.Item label="容量利用率">
                  {selectedPoint.capacityUtilPct.toFixed(1)}%
                </Descriptions.Item>
                <Descriptions.Item label="待排材料数">
                  {selectedPoint.pendingMaterialCount}
                </Descriptions.Item>
                <Descriptions.Item label="待排重量">
                  {selectedPoint.pendingWeightT.toFixed(1)}t
                </Descriptions.Item>
              </Descriptions>

              {Array.isArray(selectedPoint.bottleneckTypes) && selectedPoint.bottleneckTypes.length > 0 ? (
                <Space wrap>
                  {selectedPoint.bottleneckTypes.map((t) => (
                    <Tag key={t} color={getBottleneckTypeColor(t)} style={{ marginInlineEnd: 0 }}>
                      {getBottleneckTypeLabel(t)}
                    </Tag>
                  ))}
                </Space>
              ) : null}

              {Array.isArray(selectedPoint.reasons) && selectedPoint.reasons.length > 0 ? (
                <Table
                  rowKey={(r) => r.code}
                  size="small"
                  columns={reasonColumns}
                  dataSource={selectedPoint.reasons}
                  pagination={false}
                />
              ) : (
                <Text type="secondary">暂无堵塞原因明细</Text>
              )}

              {Array.isArray(selectedPoint.recommendedActions) && selectedPoint.recommendedActions.length > 0 ? (
                <div>
                  <Text strong>推荐行动</Text>
                  <div style={{ marginTop: 6 }}>
                    {selectedPoint.recommendedActions.map((a, idx) => (
                      <div key={`${idx}-${a}`} style={{ color: token.colorTextSecondary }}>
                        · {a}
                      </div>
                    ))}
                  </div>
                </div>
              ) : null}
            </>
          ) : null}

          <Table
            rowKey={(r) => `${r.machineCode}-${r.planDate}-${r.bottleneckLevel}`}
            size="small"
            columns={columns}
            dataSource={rows}
            pagination={{ pageSize: 20 }}
            onRow={(record) => ({
              style:
                spec.machineCode &&
                spec.planDate &&
                record.machineCode === spec.machineCode &&
                record.planDate === spec.planDate
                  ? { background: token.colorFillQuaternary }
                  : undefined,
            })}
          />
        </Space>
      );
    }

    if (spec.kind === 'roll') {
      const rows = [...rollAlerts].sort((a, b) => a.remainingTonnageT - b.remainingTonnageT);
      const selected = spec.machineCode ? rollAlerts.find((r) => r.machineCode === spec.machineCode) || null : null;

      const columns: ColumnsType<RollCampaignAlert> = [
        { title: '机组', dataIndex: 'machineCode', key: 'machineCode', width: 90 },
        {
          title: '等级',
          dataIndex: 'alertLevel',
          key: 'alertLevel',
          width: 120,
          render: (v: string) => {
            const status = parseAlertLevel(String(v || ''));
            const color =
              status === 'HARD_STOP' ? '#ff4d4f' : status === 'WARNING' ? '#faad14' : status === 'SUGGEST' ? '#1677ff' : '#52c41a';
            return <Tag color={color}>{status}</Tag>;
          },
        },
        { title: '当前吨位', dataIndex: 'currentTonnageT', key: 'currentTonnageT', width: 100 },
        { title: '硬上限', dataIndex: 'hardLimitT', key: 'hardLimitT', width: 100 },
        { title: '剩余(距硬上限)', dataIndex: 'remainingTonnageT', key: 'remainingTonnageT', width: 140 },
        { title: '开始日', dataIndex: 'campaignStartDate', key: 'campaignStartDate', width: 110 },
        { title: '预计硬停止', dataIndex: 'estimatedHardStopDate', key: 'estimatedHardStopDate', width: 120 },
        {
          title: '操作',
          key: 'action',
          width: onGoWorkbench ? 160 : 90,
          render: (_, record) => (
            <Space size={8}>
              {onGoWorkbench ? (
                <Button
                  size="small"
                  type="primary"
                  onClick={() =>
                    onGoWorkbench({
                      workbenchTab: 'visualization',
                      machineCode: record.machineCode,
                    })
                  }
                >
                  处理
                </Button>
              ) : null}
              <Button
                size="small"
                onClick={() => {
                  setDetailRecord(record);
                  setDetailOpen(true);
                }}
              >
                详情
              </Button>
            </Space>
          ),
        },
      ];

      return (
        <Space direction="vertical" size={12} style={{ width: '100%' }}>
          {selected ? (
            <Space wrap align="center">
              <Tag color="blue">{selected.machineCode}</Tag>
              <Tag color={parseAlertLevel(String(selected.alertLevel || '')) === 'HARD_STOP' ? '#ff4d4f' : '#8c8c8c'}>
                {parseAlertLevel(String(selected.alertLevel || ''))}
              </Tag>
              {onGoWorkbench ? (
                <Button
                  size="small"
                  type="primary"
                  onClick={() =>
                    onGoWorkbench({
                      workbenchTab: 'visualization',
                      machineCode: selected.machineCode,
                    })
                  }
                >
                  去处理
                </Button>
              ) : null}
            </Space>
          ) : null}

          <Table
            rowKey={(r) => `${r.machineCode}-${r.campaignId}`}
            size="small"
            columns={columns}
            dataSource={rows}
            pagination={{ pageSize: 20 }}
            onRow={(record) => ({
              style: spec.machineCode && record.machineCode === spec.machineCode ? { background: token.colorFillQuaternary } : undefined,
            })}
          />
        </Space>
      );
    }

    if (spec.kind === 'capacityOpportunity') {
      const rows = [...capacityOpportunities].sort((a, b) => b.opportunitySpaceT - a.opportunitySpaceT);
      const selected =
        spec.machineCode && spec.planDate
          ? capacityOpportunities.find((r) => r.machineCode === spec.machineCode && r.planDate === spec.planDate) || null
          : null;

      const columns: ColumnsType<CapacityOpportunity> = [
        { title: '日期', dataIndex: 'planDate', key: 'planDate', width: 110 },
        { title: '机组', dataIndex: 'machineCode', key: 'machineCode', width: 90 },
        {
          title: '类型',
          dataIndex: 'opportunityType',
          key: 'opportunityType',
          width: 140,
          render: (v: string) => {
            const t = parseOpportunityType(v);
            return <Tag color={OPPORTUNITY_TYPE_COLORS[t as keyof typeof OPPORTUNITY_TYPE_COLORS]}>{OPPORTUNITY_TYPE_LABELS[t as keyof typeof OPPORTUNITY_TYPE_LABELS] || t}</Tag>;
          },
        },
        {
          title: '当前利用率',
          dataIndex: 'currentUtilPct',
          key: 'currentUtilPct',
          width: 110,
          render: (v: number) => `${Number(v || 0).toFixed(1)}%`,
        },
        {
          title: '机会(吨)',
          dataIndex: 'opportunitySpaceT',
          key: 'opportunitySpaceT',
          width: 110,
          render: (v: number) => `${Number(v || 0).toFixed(1)}`,
        },
        {
          title: '描述',
          dataIndex: 'description',
          key: 'description',
          ellipsis: true,
        },
        {
          title: '首条建议',
          dataIndex: 'recommendedActions',
          key: 'recommendedActions',
          width: 260,
          ellipsis: true,
          render: (v: string[]) => (Array.isArray(v) && v.length > 0 ? v[0] : '-'),
        },
        {
          title: '操作',
          key: 'action',
          width: onGoWorkbench ? 160 : 90,
          render: (_, record) => (
            <Space size={8}>
              {onGoWorkbench ? (
                <Button
                  size="small"
                  type="primary"
                  onClick={() =>
                    onGoWorkbench({
                      workbenchTab: 'capacity',
                      machineCode: record.machineCode,
                    })
                  }
                >
                  处理
                </Button>
              ) : null}
              <Button
                size="small"
                onClick={() => {
                  setDetailRecord(record);
                  setDetailOpen(true);
                }}
              >
                详情
              </Button>
            </Space>
          ),
        },
      ];

      return (
        <Space direction="vertical" size={12} style={{ width: '100%' }}>
          {selected ? (
            <>
              <Space wrap align="center">
                <Tag color="blue">{selected.machineCode}</Tag>
                <Text strong>
                  {selected.planDate} · 机会 {selected.opportunitySpaceT.toFixed(1)}t
                </Text>
                {onGoWorkbench ? (
                  <Button
                    size="small"
                    type="primary"
                    onClick={() =>
                      onGoWorkbench({
                        workbenchTab: 'capacity',
                        machineCode: selected.machineCode,
                      })
                    }
                  >
                    去处理
                  </Button>
                ) : null}
              </Space>

              <Descriptions column={4} bordered size="small">
                <Descriptions.Item label="当前利用率">
                  {selected.currentUtilPct.toFixed(1)}%
                </Descriptions.Item>
                <Descriptions.Item label="优化后利用率">
                  {selected.optimizedUtilPct.toFixed(1)}%
                </Descriptions.Item>
                <Descriptions.Item label="已用/目标">
                  {selected.usedCapacityT.toFixed(1)} / {selected.targetCapacityT.toFixed(0)}t
                </Descriptions.Item>
                <Descriptions.Item label="机会空间">
                  {selected.opportunitySpaceT.toFixed(1)}t
                </Descriptions.Item>
              </Descriptions>

              {selected.description ? (
                <div>
                  <Text strong>描述</Text>
                  <div style={{ marginTop: 6, color: token.colorTextSecondary }}>{selected.description}</div>
                </div>
              ) : null}

              {Array.isArray(selected.recommendedActions) && selected.recommendedActions.length > 0 ? (
                <div>
                  <Text strong>建议操作</Text>
                  <div style={{ marginTop: 6 }}>
                    {selected.recommendedActions.slice(0, 6).map((a, idx) => (
                      <div key={`${idx}-${a}`} style={{ color: token.colorTextSecondary }}>
                        · {a}
                      </div>
                    ))}
                  </div>
                </div>
              ) : null}

              {Array.isArray(selected.potentialBenefits) && selected.potentialBenefits.length > 0 ? (
                <div>
                  <Text strong>潜在收益</Text>
                  <div style={{ marginTop: 6 }}>
                    {selected.potentialBenefits.slice(0, 6).map((b, idx) => (
                      <div key={`${idx}-${b}`} style={{ color: token.colorTextSecondary }}>
                        · {b}
                      </div>
                    ))}
                  </div>
                </div>
              ) : null}
            </>
          ) : null}

          <Table
            rowKey={(r) => `${r.planDate}-${r.machineCode}`}
            size="small"
            columns={columns}
            dataSource={rows}
            pagination={{ pageSize: 20 }}
            onRow={(record) => ({
              style:
                spec.machineCode &&
                spec.planDate &&
                record.machineCode === spec.machineCode &&
                record.planDate === spec.planDate
                  ? { background: token.colorFillQuaternary }
                  : undefined,
            })}
            scroll={{ x: 1100 }}
          />
        </Space>
      );
    }

    if (spec.kind === 'risk') {
      const rows = [...riskDays].sort((a, b) => b.riskScore - a.riskScore);
      const selectedDay = spec.planDate ? riskDays.find((d) => d.planDate === spec.planDate) || null : null;

      const reasonColumns: ColumnsType<ReasonItem> = [
        { title: '代码', dataIndex: 'code', key: 'code', width: 120, render: (v: string) => <Tag>{v}</Tag> },
        { title: '原因', dataIndex: 'msg', key: 'msg', ellipsis: true },
        {
          title: '权重',
          dataIndex: 'weight',
          key: 'weight',
          width: 90,
          render: (v: number) => `${(Number(v || 0) * 100).toFixed(1)}%`,
        },
        {
          title: '影响数',
          dataIndex: 'affectedCount',
          key: 'affectedCount',
          width: 90,
          render: (v?: number) => (typeof v === 'number' ? v : '-'),
        },
      ];

      const columns: ColumnsType<DaySummary> = [
        { title: '日期', dataIndex: 'planDate', key: 'planDate', width: 110 },
        {
          title: '等级',
          dataIndex: 'riskLevel',
          key: 'riskLevel',
          width: 120,
          render: (v: DaySummary['riskLevel']) => (
            <Tag color={getRiskLevelColor(v)}>{getRiskLevelLabel(v)}</Tag>
          ),
        },
        { title: '分数', dataIndex: 'riskScore', key: 'riskScore', width: 90 },
        {
          title: '利用率',
          dataIndex: 'capacityUtilPct',
          key: 'capacityUtilPct',
          width: 100,
          render: (v: number) => `${Number(v || 0).toFixed(1)}%`,
        },
        { title: '超载(吨)', dataIndex: 'overloadWeightT', key: 'overloadWeightT', width: 100 },
        { title: '紧急失败', dataIndex: 'urgentFailureCount', key: 'urgentFailureCount', width: 90 },
        {
          title: '涉及机组',
          dataIndex: 'involvedMachines',
          key: 'involvedMachines',
          ellipsis: true,
          render: (v: string[]) => (Array.isArray(v) ? v.join(', ') : ''),
        },
        {
          title: '操作',
          key: 'action',
          width: onGoWorkbench ? 160 : 90,
          render: (_, record) => (
            <Space size={8}>
              {onGoWorkbench ? (
                <Button
                  size="small"
                  type="primary"
                  onClick={() =>
                    onGoWorkbench({
                      workbenchTab: 'capacity',
                      machineCode:
                        Array.isArray(record.involvedMachines) && record.involvedMachines.length > 0
                          ? record.involvedMachines[0]
                          : null,
                    })
                  }
                >
                  处理
                </Button>
              ) : null}
              <Button
                size="small"
                onClick={() => {
                  setDetailRecord(record);
                  setDetailOpen(true);
                }}
              >
                详情
              </Button>
            </Space>
          ),
        },
      ];

      return (
        <Space direction="vertical" size={12} style={{ width: '100%' }}>
          {selectedDay ? (
            <>
              <Space wrap align="center">
                <Tag color={getRiskLevelColor(selectedDay.riskLevel)}>
                  {getRiskLevelLabel(selectedDay.riskLevel)} / {selectedDay.riskScore.toFixed(1)}
                </Tag>
                <Text strong>{selectedDay.planDate}</Text>
                {onGoWorkbench ? (
                  <Button
                    size="small"
                    type="primary"
                    onClick={() =>
                      onGoWorkbench({
                        workbenchTab: 'capacity',
                        machineCode:
                          Array.isArray(selectedDay.involvedMachines) && selectedDay.involvedMachines.length > 0
                            ? selectedDay.involvedMachines[0]
                            : null,
                      })
                    }
                  >
                    去处理
                  </Button>
                ) : null}
              </Space>

              <Descriptions column={4} bordered size="small">
                <Descriptions.Item label="风险分数">
                  {selectedDay.riskScore.toFixed(1)}
                </Descriptions.Item>
                <Descriptions.Item label="容量利用率">
                  {selectedDay.capacityUtilPct.toFixed(1)}%
                </Descriptions.Item>
                <Descriptions.Item label="超载">
                  {selectedDay.overloadWeightT.toFixed(1)}t
                </Descriptions.Item>
                <Descriptions.Item label="紧急失败">
                  {selectedDay.urgentFailureCount}
                </Descriptions.Item>
              </Descriptions>

              {Array.isArray(selectedDay.involvedMachines) && selectedDay.involvedMachines.length > 0 ? (
                <Space wrap>
                  {selectedDay.involvedMachines.slice(0, 12).map((m) => (
                    <Tag key={m} style={{ marginInlineEnd: 0 }}>
                      {m}
                    </Tag>
                  ))}
                  {selectedDay.involvedMachines.length > 12 ? (
                    <Text type="secondary">等 {selectedDay.involvedMachines.length} 个机组</Text>
                  ) : null}
                </Space>
              ) : null}

              {Array.isArray(selectedDay.topReasons) && selectedDay.topReasons.length > 0 ? (
                <Table
                  rowKey={(r) => r.code}
                  size="small"
                  columns={reasonColumns}
                  dataSource={selectedDay.topReasons}
                  pagination={false}
                />
              ) : (
                <Text type="secondary">暂无风险原因明细</Text>
              )}
            </>
          ) : null}

          <Table
            rowKey={(r) => r.planDate}
            size="small"
            columns={columns}
            dataSource={rows}
            pagination={{ pageSize: 20 }}
            onRow={(record) => ({
              style: spec.planDate && record.planDate === spec.planDate ? { background: token.colorFillQuaternary } : undefined,
            })}
          />
        </Space>
      );
    }

    return null;
  }, [spec, orderFailures, coldStockBuckets, bottlenecks, rollAlerts, riskDays, capacityOpportunities, onGoWorkbench, token]);

  return (
    <>
      <Drawer
        title={titleFor(spec)}
        open={open}
        width={900}
        onClose={onClose}
        destroyOnClose
        extra={
          onRetry ? (
            <Space>
              <Button onClick={onRetry}>重试</Button>
            </Space>
          ) : null
        }
      >
        {!spec ? (
          <Text type="secondary">请选择一项问题查看详情</Text>
        ) : error ? (
          <Alert
            type="error"
            showIcon
            message="数据加载失败"
            description={<Text type="secondary">{String((error as any)?.message || error)}</Text>}
            action={onRetry ? <Button onClick={onRetry}>重试</Button> : undefined}
          />
        ) : (
          <div style={{ opacity: loading ? 0.6 : 1 }}>{content}</div>
        )}
      </Drawer>

      <Modal
        title="详情"
        open={detailOpen}
        onCancel={() => setDetailOpen(false)}
        footer={<Button onClick={() => setDetailOpen(false)}>关闭</Button>}
        width={720}
      >
        {detailRecord ? (
          <Descriptions size="small" column={1} bordered>
            {Object.entries(detailRecord).map(([k, v]) => (
              <Descriptions.Item key={k} label={k}>
                {Array.isArray(v) ? v.join(', ') : typeof v === 'object' ? JSON.stringify(v) : String(v)}
              </Descriptions.Item>
            ))}
          </Descriptions>
        ) : null}
      </Modal>
    </>
  );
};

export default React.memo(DrilldownDrawer);
