// ==========================================
// D2决策：材料失败看板页面（材料维度）
// ==========================================

import React, { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import {
  Alert,
  Button,
  Card,
  Col,
  Descriptions,
  Empty,
  Row,
  Select,
  Space,
  Spin,
  Statistic,
  Table,
  Tag,
  Typography,
} from 'antd';
import { ClockCircleOutlined, ExclamationCircleOutlined, WarningOutlined } from '@ant-design/icons';
import type { ColumnsType } from 'antd/es/table';
import { useNavigate, useSearchParams } from 'react-router-dom';

import { EmptyState } from '../../components/EmptyState';
import { useMaterialFailureSet } from '../../hooks/queries/use-decision-queries';
import type { DrilldownSpec } from '../../hooks/useRiskOverviewData';
import { useActiveVersionId } from '../../stores/use-global-store';
import type {
  FailType,
  MaterialFailure,
  MaterialFailureContractAggregate,
  MaterialFailureProblemScope,
  UrgencyLevel,
} from '../../types/decision';
import {
  FAIL_TYPE_LABELS,
  URGENCY_LEVEL_LABELS,
  getFailTypeColor,
  getUrgencyLevelColor,
} from '../../types/decision';
import { formatNumber, formatWeight } from '../../utils/formatters';

const { Text } = Typography;

type UrgencyFilter = UrgencyLevel | 'ALL';
type FailTypeFilter = FailType | 'ALL';

interface D2OrderFailureProps {
  embedded?: boolean;
  onOpenDrilldown?: (spec: DrilldownSpec) => void;
}

type ContractGroupRow = {
  contractNo: string;
  materialCount: number;
  unscheduledCount: number;
  overdueCount: number;
  earliestDueDate: string;
  maxUrgencyLevel: UrgencyLevel;
  representativeMaterialId: string;
  representative?: MaterialFailure;
  materials: MaterialFailure[];
};

function materialRowKey(row: MaterialFailure): string {
  return `${row.contractNo || ''}::${row.materialId}`;
}

function urgencyRank(level: string): number {
  if (level === 'L3') return 3;
  if (level === 'L2') return 2;
  if (level === 'L1') return 1;
  return 0;
}

function sortContractGroups(a: ContractGroupRow, b: ContractGroupRow): number {
  if (a.unscheduledCount !== b.unscheduledCount) return b.unscheduledCount - a.unscheduledCount;
  if (a.overdueCount !== b.overdueCount) return b.overdueCount - a.overdueCount;
  if (a.earliestDueDate !== b.earliestDueDate) return a.earliestDueDate.localeCompare(b.earliestDueDate);
  return a.contractNo.localeCompare(b.contractNo);
}

function buildContractGroups(rows: MaterialFailure[]): ContractGroupRow[] {
  const map = new Map<string, ContractGroupRow>();

  rows.forEach((row) => {
    const contractNo = String(row.contractNo || '').trim();
    if (!contractNo) return;

    const current = map.get(contractNo);
    if (!current) {
      map.set(contractNo, {
        contractNo,
        materialCount: 1,
        unscheduledCount: row.isScheduled ? 0 : 1,
        overdueCount: row.daysToDue < 0 ? 1 : 0,
        earliestDueDate: row.dueDate,
        maxUrgencyLevel: row.urgencyLevel,
        representativeMaterialId: row.materialId,
        representative: row,
        materials: [row],
      });
      return;
    }

    current.materialCount += 1;
    if (!row.isScheduled) current.unscheduledCount += 1;
    if (row.daysToDue < 0) current.overdueCount += 1;
    if (row.dueDate < current.earliestDueDate) current.earliestDueDate = row.dueDate;
    if (urgencyRank(row.urgencyLevel) > urgencyRank(current.maxUrgencyLevel)) {
      current.maxUrgencyLevel = row.urgencyLevel;
    }

    const rep = current.representative;
    if (rep) {
      const shouldReplace =
        (row.isScheduled ? 0 : 1) > (rep.isScheduled ? 0 : 1) ||
        ((row.isScheduled ? 0 : 1) === (rep.isScheduled ? 0 : 1) &&
          (urgencyRank(row.urgencyLevel) > urgencyRank(rep.urgencyLevel) ||
            (urgencyRank(row.urgencyLevel) === urgencyRank(rep.urgencyLevel) &&
              (row.dueDate < rep.dueDate ||
                (row.dueDate === rep.dueDate && row.materialId < rep.materialId)))));

      if (shouldReplace) {
        current.representative = row;
        current.representativeMaterialId = row.materialId;
      }
    }

    current.materials.push(row);
  });

  return [...map.values()].sort(sortContractGroups);
}

function buildContractGroupsFromAggregates(
  aggregates: MaterialFailureContractAggregate[],
  rowsByContract: Map<string, MaterialFailure[]>,
): ContractGroupRow[] {
  return [...aggregates]
    .map((agg) => {
      const materials = rowsByContract.get(agg.contractNo) ?? [];
      const representative =
        materials.find((m) => m.materialId === agg.representativeMaterialId) || materials[0];
      return {
        contractNo: agg.contractNo,
        materialCount: agg.materialCount,
        unscheduledCount: agg.unscheduledCount,
        overdueCount: agg.overdueCount,
        earliestDueDate: agg.earliestDueDate,
        maxUrgencyLevel: agg.maxUrgencyLevel,
        representativeMaterialId: agg.representativeMaterialId,
        representative,
        materials,
      };
    })
    .sort(sortContractGroups);
}

export const D2OrderFailure: React.FC<D2OrderFailureProps> = ({ embedded, onOpenDrilldown }) => {
  const versionId = useActiveVersionId();
  const navigate = useNavigate();
  const [searchParams] = useSearchParams();

  const [selectedUrgency, setSelectedUrgency] = useState<UrgencyFilter>('ALL');
  const [selectedFailType, setSelectedFailType] = useState<FailTypeFilter>('ALL');
  const [problemScope, setProblemScope] = useState<MaterialFailureProblemScope>('UNSCHEDULED_ONLY');
  const [selectedMaterial, setSelectedMaterial] = useState<MaterialFailure | null>(null);
  const [expandedContractKeys, setExpandedContractKeys] = useState<string[]>([]);
  const [contractPage, setContractPage] = useState(1);
  const [materialPageByContract, setMaterialPageByContract] = useState<Record<string, number>>({});
  const [highlightMaterialKey, setHighlightMaterialKey] = useState<string | null>(null);
  const autoLocatedRef = useRef<string>('');
  const contractPageSize = embedded ? 8 : 15;
  const materialPageSize = embedded ? 6 : 10;

  const { data, isLoading, error } = useMaterialFailureSet(
    {
      versionId: versionId || '',
      limit: 5000,
      problemScope,
      onlyUnscheduled: problemScope === 'UNSCHEDULED_ONLY' ? true : undefined,
    },
    { enabled: !!versionId },
  );

  useEffect(() => {
    const urgency = String(searchParams.get('urgency') || '').trim();
    if (urgency && ['L0', 'L1', 'L2', 'L3'].includes(urgency)) {
      setSelectedUrgency(urgency as UrgencyLevel);
    }

    const failType = String(searchParams.get('failType') || '').trim();
    if (
      failType &&
      ['Overdue', 'NearDueImpossible', 'CapacityShortage', 'StructureConflict', 'ColdStockNotReady', 'Other'].includes(
        failType,
      )
    ) {
      setSelectedFailType(failType as FailType);
    }
  }, [searchParams]);

  const targetMaterialId = useMemo(() => String(searchParams.get('material_id') || '').trim(), [searchParams]);
  const targetContractNo = useMemo(() => String(searchParams.get('contract_no') || '').trim(), [searchParams]);

  const allMaterials = data?.items ?? [];

  const filteredMaterials = useMemo(() => {
    let rows = allMaterials;
    if (selectedUrgency !== 'ALL') {
      rows = rows.filter((r) => r.urgencyLevel === selectedUrgency);
    }
    if (selectedFailType !== 'ALL') {
      rows = rows.filter((r) => r.failType === selectedFailType);
    }
    return rows;
  }, [allMaterials, selectedFailType, selectedUrgency]);

  const hasFilter = selectedUrgency !== 'ALL' || selectedFailType !== 'ALL';

  const materialRowsByContract = useMemo(() => {
    const map = new Map<string, MaterialFailure[]>();
    filteredMaterials.forEach((row) => {
      const key = String(row.contractNo || '').trim();
      if (!key) return;
      const list = map.get(key);
      if (list) {
        list.push(row);
      } else {
        map.set(key, [row]);
      }
    });
    return map;
  }, [filteredMaterials]);

  const groupedContracts = useMemo(() => {
    if (!hasFilter && Array.isArray(data?.contractAggregates) && data.contractAggregates.length > 0) {
      return buildContractGroupsFromAggregates(data.contractAggregates, materialRowsByContract);
    }
    return buildContractGroups(filteredMaterials);
  }, [data?.contractAggregates, filteredMaterials, hasFilter, materialRowsByContract]);

  const locateMaterial = useCallback(
    (row: MaterialFailure) => {
      const contractNo = String(row.contractNo || '').trim();
      if (!contractNo) {
        setHighlightMaterialKey(materialRowKey(row));
        setSelectedMaterial(row);
        return;
      }

      const contractIndex = groupedContracts.findIndex((it) => it.contractNo === contractNo);
      if (contractIndex >= 0) {
        const targetContract = groupedContracts[contractIndex];
        setContractPage(Math.floor(contractIndex / contractPageSize) + 1);
        setExpandedContractKeys((prev) => (prev.includes(contractNo) ? prev : [...prev, contractNo]));

        const materialIndex = targetContract.materials.findIndex((it) => it.materialId === row.materialId);
        if (materialIndex >= 0) {
          setMaterialPageByContract((prev) => ({
            ...prev,
            [contractNo]: Math.floor(materialIndex / materialPageSize) + 1,
          }));
        }
      }

      setHighlightMaterialKey(materialRowKey(row));
      setSelectedMaterial(row);
    },
    [contractPageSize, groupedContracts, materialPageSize],
  );

  useEffect(() => {
    const validContractSet = new Set(groupedContracts.map((it) => it.contractNo));
    setExpandedContractKeys((prev) => prev.filter((it) => validContractSet.has(it)));
    setMaterialPageByContract((prev) => {
      const next: Record<string, number> = {};
      Object.entries(prev).forEach(([contractNo, page]) => {
        if (validContractSet.has(contractNo)) {
          next[contractNo] = page;
        }
      });
      return next;
    });
    const maxPage = Math.max(1, Math.ceil(groupedContracts.length / contractPageSize));
    setContractPage((prev) => Math.min(prev, maxPage));
  }, [contractPageSize, groupedContracts]);

  useEffect(() => {
    if (!targetMaterialId && !targetContractNo) {
      autoLocatedRef.current = '';
      return;
    }
    if (!groupedContracts.length) return;

    const locateKey = `${targetContractNo}::${targetMaterialId}`;
    if (autoLocatedRef.current === locateKey) return;

    let target: MaterialFailure | undefined;
    if (targetMaterialId) {
      target = filteredMaterials.find(
        (it) =>
          it.materialId === targetMaterialId &&
          (!targetContractNo || String(it.contractNo || '').trim() === targetContractNo),
      );
    }
    if (!target && targetContractNo) {
      const contract = groupedContracts.find((it) => it.contractNo === targetContractNo);
      target = contract?.materials[0] || contract?.representative;
    }

    if (target) {
      locateMaterial(target);
      autoLocatedRef.current = locateKey;
    }
  }, [filteredMaterials, groupedContracts, locateMaterial, targetContractNo, targetMaterialId]);

  const globalSummary = data?.summary;

  const viewSummary = useMemo(() => {
    const contractSet = new Set(filteredMaterials.map((r) => r.contractNo).filter(Boolean));
    const overdueMaterials = filteredMaterials.filter((r) => r.daysToDue < 0).length;
    const unscheduledMaterials = filteredMaterials.filter((r) => !r.isScheduled).length;
    const totalUnscheduledWeightT = filteredMaterials.reduce((sum, r) => sum + Number(r.unscheduledWeightT || 0), 0);
    return {
      totalFailedMaterials: filteredMaterials.length,
      totalFailedContracts: contractSet.size,
      overdueMaterials,
      unscheduledMaterials,
      totalUnscheduledWeightT,
    };
  }, [filteredMaterials]);

  const displaySummary = hasFilter
    ? viewSummary
    : {
        totalFailedMaterials: globalSummary?.totalFailedMaterials ?? viewSummary.totalFailedMaterials,
        totalFailedContracts: globalSummary?.totalFailedContracts ?? viewSummary.totalFailedContracts,
        overdueMaterials: globalSummary?.overdueMaterials ?? viewSummary.overdueMaterials,
        unscheduledMaterials: globalSummary?.unscheduledMaterials ?? viewSummary.unscheduledMaterials,
        totalUnscheduledWeightT: globalSummary?.totalUnscheduledWeightT ?? viewSummary.totalUnscheduledWeightT,
      };

  const isMaterialRowsTruncated = Number(data?.totalCount || 0) > allMaterials.length;

  const goWorkbenchByMaterial = useCallback(
    (row: MaterialFailure) => {
      const params = new URLSearchParams();
      params.set('context', 'orders');
      params.set('focus', 'matrix');
      params.set('material_id', row.materialId);
      if (row.contractNo) params.set('contract_no', row.contractNo);
      navigate(`/workbench?${params.toString()}`);
    },
    [navigate],
  );

  const materialColumns: ColumnsType<MaterialFailure> = [
    { title: '材料号', dataIndex: 'materialId', key: 'materialId', width: 160, ellipsis: true },
    {
      title: '状态',
      key: 'status',
      width: 90,
      render: (_, row) => (
        <Tag color={row.isScheduled ? 'blue' : 'orange'}>{row.isScheduled ? '已排' : '未排'}</Tag>
      ),
    },
    {
      title: '紧急',
      dataIndex: 'urgencyLevel',
      key: 'urgencyLevel',
      width: 90,
      render: (v: UrgencyLevel) => <Tag color={getUrgencyLevelColor(v)}>{URGENCY_LEVEL_LABELS[v]}</Tag>,
    },
    { title: '交期', dataIndex: 'dueDate', key: 'dueDate', width: 110 },
    { title: '剩余天数', dataIndex: 'daysToDue', key: 'daysToDue', width: 90 },
    {
      title: '失败类型',
      dataIndex: 'failType',
      key: 'failType',
      width: 130,
      render: (v: FailType) => <Tag color={getFailTypeColor(v)}>{FAIL_TYPE_LABELS[v] || v}</Tag>,
    },
    { title: '机组', dataIndex: 'machineCode', key: 'machineCode', width: 90 },
    {
      title: '未排（吨）',
      dataIndex: 'unscheduledWeightT',
      key: 'unscheduledWeightT',
      width: 120,
      render: (v: number) => formatWeight(v),
    },
    {
      title: '操作',
      key: 'action',
      width: 180,
      render: (_, row) => (
        <Space size={8}>
          <Button
            size="small"
            type="primary"
            onClick={() => {
              locateMaterial(row);
              goWorkbenchByMaterial(row);
            }}
          >
            处理
          </Button>
          {embedded && onOpenDrilldown ? (
            <Button size="small" onClick={() => onOpenDrilldown({ kind: 'orders', urgency: row.urgencyLevel })}>
              下钻
            </Button>
          ) : null}
          <Button size="small" onClick={() => locateMaterial(row)}>
            详情
          </Button>
        </Space>
      ),
    },
  ];

  const contractColumns: ColumnsType<ContractGroupRow> = [
    { title: '合同号', dataIndex: 'contractNo', key: 'contractNo', width: 160, ellipsis: true },
    { title: '材料数', dataIndex: 'materialCount', key: 'materialCount', width: 90 },
    { title: '未排数', dataIndex: 'unscheduledCount', key: 'unscheduledCount', width: 90 },
    { title: '逾期数', dataIndex: 'overdueCount', key: 'overdueCount', width: 90 },
    { title: '最早交期', dataIndex: 'earliestDueDate', key: 'earliestDueDate', width: 120 },
    {
      title: '最高紧急',
      dataIndex: 'maxUrgencyLevel',
      key: 'maxUrgencyLevel',
      width: 100,
      render: (v: UrgencyLevel) => <Tag color={getUrgencyLevelColor(v)}>{URGENCY_LEVEL_LABELS[v]}</Tag>,
    },
    {
      title: '代表材料',
      dataIndex: 'representativeMaterialId',
      key: 'representativeMaterialId',
      width: 170,
      ellipsis: true,
    },
    {
      title: '操作',
      key: 'action',
      width: 180,
      render: (_, row) => (
        <Space size={8}>
          <Button
            size="small"
            type="primary"
            disabled={!row.representative}
            onClick={() => {
              if (!row.representative) return;
              locateMaterial(row.representative);
              goWorkbenchByMaterial(row.representative);
            }}
          >
            处理
          </Button>
          {embedded && onOpenDrilldown ? (
            <Button
              size="small"
              disabled={!row.representative}
              onClick={() =>
                row.representative &&
                onOpenDrilldown({ kind: 'orders', urgency: row.representative.urgencyLevel })
              }
            >
              下钻
            </Button>
          ) : null}
          <Button
            size="small"
            disabled={!row.representative}
            onClick={() => row.representative && locateMaterial(row.representative)}
          >
            详情
          </Button>
        </Space>
      ),
    },
  ];

  if (isLoading) {
    return (
      <div style={{ textAlign: 'center', padding: embedded ? '40px 0' : '100px 0' }}>
        <Spin size="large" tip="正在加载材料失败数据...">
          <div style={{ minHeight: 80 }} />
        </Spin>
      </div>
    );
  }

  if (error) {
    return (
      <Alert
        message="数据加载失败"
        description={error.message || '未知错误'}
        type="error"
        showIcon
        style={{ margin: embedded ? 0 : '20px' }}
      />
    );
  }

  if (!versionId) {
    return (
      <Alert
        message="未选择排产版本"
        description="请先在主界面选择一个排产版本"
        type="warning"
        showIcon
        style={{ margin: embedded ? 0 : '20px' }}
      />
    );
  }

  return (
    <div style={{ padding: embedded ? 0 : 24 }}>
      {!embedded ? (
        <div style={{ marginBottom: 16 }}>
          <h2>D2决策：材料失败看板</h2>
          <p style={{ color: '#8c8c8c', marginBottom: 12 }}>材料维度展示失败清单，按合同聚合并支持展开到材料明细。</p>
        </div>
      ) : null}

      <Space wrap style={{ marginBottom: 12 }}>
        <Space>
          <span>紧急度：</span>
          <Select
            size={embedded ? 'small' : 'middle'}
            value={selectedUrgency}
            onChange={setSelectedUrgency}
            style={{ width: 150 }}
            options={[
              { label: '全部', value: 'ALL' },
              { label: URGENCY_LEVEL_LABELS.L3, value: 'L3' },
              { label: URGENCY_LEVEL_LABELS.L2, value: 'L2' },
              { label: URGENCY_LEVEL_LABELS.L1, value: 'L1' },
              { label: URGENCY_LEVEL_LABELS.L0, value: 'L0' },
            ]}
          />
        </Space>

        <Space>
          <span>失败类型：</span>
          <Select
            size={embedded ? 'small' : 'middle'}
            value={selectedFailType}
            onChange={setSelectedFailType}
            style={{ width: 190 }}
            options={[
              { label: '全部', value: 'ALL' },
              { label: FAIL_TYPE_LABELS.Overdue, value: 'Overdue' },
              { label: FAIL_TYPE_LABELS.NearDueImpossible, value: 'NearDueImpossible' },
              { label: FAIL_TYPE_LABELS.CapacityShortage, value: 'CapacityShortage' },
              { label: FAIL_TYPE_LABELS.StructureConflict, value: 'StructureConflict' },
              { label: FAIL_TYPE_LABELS.ColdStockNotReady, value: 'ColdStockNotReady' },
              { label: FAIL_TYPE_LABELS.Other, value: 'Other' },
            ]}
          />
        </Space>

        <Space>
          <span>问题范围：</span>
          <Select
            size={embedded ? 'small' : 'middle'}
            value={problemScope}
            onChange={(v) => setProblemScope(v as MaterialFailureProblemScope)}
            style={{ width: 200 }}
            options={[
              { label: '精准问题材料（默认）', value: 'UNSCHEDULED_ONLY' },
              { label: '临期关键窗口', value: 'DUE_WINDOW_CRITICAL' },
            ]}
          />
        </Space>
      </Space>

      <Row gutter={embedded ? 12 : 16} style={{ marginBottom: 12 }}>
        <Col span={6}>
          <Card size={embedded ? 'small' : undefined}>
            <Statistic
              title={hasFilter ? '失败材料（当前/全局）' : '失败材料总数'}
              value={
                hasFilter
                  ? `${viewSummary.totalFailedMaterials}/${globalSummary?.totalFailedMaterials ?? displaySummary.totalFailedMaterials}`
                  : displaySummary.totalFailedMaterials
              }
              prefix={<ExclamationCircleOutlined />}
            />
          </Card>
        </Col>
        <Col span={6}>
          <Card size={embedded ? 'small' : undefined}>
            <Statistic
              title={hasFilter ? '涉及合同（当前/全局）' : '涉及合同数'}
              value={
                hasFilter
                  ? `${viewSummary.totalFailedContracts}/${globalSummary?.totalFailedContracts ?? displaySummary.totalFailedContracts}`
                  : displaySummary.totalFailedContracts
              }
              prefix={<WarningOutlined />}
            />
          </Card>
        </Col>
        <Col span={6}>
          <Card size={embedded ? 'small' : undefined}>
            <Statistic
              title={hasFilter ? '未排材料（当前/全局）' : '未排材料数'}
              value={
                hasFilter
                  ? `${viewSummary.unscheduledMaterials}/${globalSummary?.unscheduledMaterials ?? displaySummary.unscheduledMaterials}`
                  : displaySummary.unscheduledMaterials
              }
            />
          </Card>
        </Col>
        <Col span={6}>
          <Card size={embedded ? 'small' : undefined}>
            <Statistic
              title={hasFilter ? '逾期材料（当前/全局）' : '逾期材料数'}
              value={
                hasFilter
                  ? `${viewSummary.overdueMaterials}/${globalSummary?.overdueMaterials ?? displaySummary.overdueMaterials}`
                  : displaySummary.overdueMaterials
              }
              prefix={<ClockCircleOutlined />}
            />
          </Card>
        </Col>
      </Row>

      {isMaterialRowsTruncated ? (
        <Alert
          showIcon
          type="info"
          style={{ marginBottom: 12 }}
          message="材料明细已按分页限制截断，合同聚合为全量统计"
          description="如需查看缺失合同的材料明细，请增加筛选条件后再展开。"
        />
      ) : null}

      <Card
        size={embedded ? 'small' : undefined}
        title={`合同聚合（${groupedContracts.length}）`}
        extra={
          <Text type="secondary">
            未排重量 {formatWeight(hasFilter ? viewSummary.totalUnscheduledWeightT : displaySummary.totalUnscheduledWeightT)}
          </Text>
        }
      >
        <Table
          rowKey={(r) => r.contractNo}
          size="small"
          columns={contractColumns}
          dataSource={groupedContracts}
          expandable={{
            expandedRowKeys: expandedContractKeys,
            onExpandedRowsChange: (keys) => setExpandedContractKeys(keys.map((it) => String(it))),
            expandedRowRender: (row) =>
              row.materials.length > 0 ? (
                <Table
                  rowKey={(r) => materialRowKey(r)}
                  size="small"
                  columns={materialColumns}
                  dataSource={row.materials}
                  onRow={(record) => ({
                    style:
                      materialRowKey(record) === highlightMaterialKey
                        ? { backgroundColor: '#e6f4ff' }
                        : undefined,
                  })}
                  pagination={{
                    current: materialPageByContract[row.contractNo] ?? 1,
                    pageSize: materialPageSize,
                    hideOnSinglePage: row.materials.length <= materialPageSize,
                    onChange: (page) =>
                      setMaterialPageByContract((prev) => ({
                        ...prev,
                        [row.contractNo]: page,
                      })),
                  }}
                />
              ) : (
                <Empty
                  image={Empty.PRESENTED_IMAGE_SIMPLE}
                  description="该合同明细未在当前批次中加载，可先缩小范围后重试。"
                />
              ),
          }}
          pagination={{
            current: contractPage,
            pageSize: contractPageSize,
            onChange: (page) => setContractPage(page),
          }}
          locale={{ emptyText: <EmptyState type="order" style={{ padding: '24px 0' }} /> }}
        />
      </Card>

      {!embedded && selectedMaterial ? (
        <Card title={`材料详情: ${selectedMaterial.materialId}`} style={{ marginTop: 16 }}>
          <Descriptions size="small" bordered column={3}>
            <Descriptions.Item label="合同号">{selectedMaterial.contractNo}</Descriptions.Item>
            <Descriptions.Item label="紧急等级">
              <Tag color={getUrgencyLevelColor(selectedMaterial.urgencyLevel)}>
                {URGENCY_LEVEL_LABELS[selectedMaterial.urgencyLevel]}
              </Tag>
            </Descriptions.Item>
            <Descriptions.Item label="失败类型">
              <Tag color={getFailTypeColor(selectedMaterial.failType)}>
                {FAIL_TYPE_LABELS[selectedMaterial.failType] || selectedMaterial.failType}
              </Tag>
            </Descriptions.Item>
            <Descriptions.Item label="机组">{selectedMaterial.machineCode}</Descriptions.Item>
            <Descriptions.Item label="交期">{selectedMaterial.dueDate}</Descriptions.Item>
            <Descriptions.Item label="剩余天数">{selectedMaterial.daysToDue}</Descriptions.Item>
            <Descriptions.Item label="重量">{formatWeight(selectedMaterial.weightT)}</Descriptions.Item>
            <Descriptions.Item label="未排重量">{formatWeight(selectedMaterial.unscheduledWeightT)}</Descriptions.Item>
            <Descriptions.Item label="合同完成率">{formatNumber(selectedMaterial.completionRate, 2)}%</Descriptions.Item>
          </Descriptions>
        </Card>
      ) : null}
    </div>
  );
};

export default D2OrderFailure;
