/**
 * 材料失败集合内容（合同聚合 + 材料明细）
 */

import React, { useCallback, useEffect, useMemo, useState } from 'react';
import { Button, Empty, Space, Table, Tooltip } from 'antd';
import type { ColumnsType } from 'antd/es/table';
import type { MaterialFailure, MaterialFailureContractAggregate } from '../../../types/decision';
import { getFailTypeColor, getFailTypeLabel } from '../../../types/decision';
import { getUrgencyLevelColor, getUrgencyLevelLabel, TagWithColor, type WorkbenchCallback } from './shared';
import { formatWeight } from '../../../utils/formatters';

export interface OrdersContentProps {
  rows: MaterialFailure[];
  contractAggregates?: MaterialFailureContractAggregate[];
  urgencyFilter?: string | null;
  onGoWorkbench?: WorkbenchCallback;
  onViewDetail: (record: MaterialFailure) => void;
}

type ContractAggregateRow = {
  contractNo: string;
  materialCount: number;
  unscheduledCount: number;
  overdueCount: number;
  earliestDueDate: string;
  maxUrgencyLevel: MaterialFailure['urgencyLevel'];
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

function sortContractRows(a: ContractAggregateRow, b: ContractAggregateRow): number {
  if (a.unscheduledCount !== b.unscheduledCount) return b.unscheduledCount - a.unscheduledCount;
  if (a.overdueCount !== b.overdueCount) return b.overdueCount - a.overdueCount;
  if (a.earliestDueDate !== b.earliestDueDate) return a.earliestDueDate.localeCompare(b.earliestDueDate);
  return a.contractNo.localeCompare(b.contractNo);
}

function buildContractRowsFromMaterials(rows: MaterialFailure[]): ContractAggregateRow[] {
  const map = new Map<string, ContractAggregateRow>();

  rows.forEach((row) => {
    const key = String(row.contractNo || '').trim();
    if (!key) return;

    const existing = map.get(key);
    if (!existing) {
      map.set(key, {
        contractNo: key,
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

    existing.materialCount += 1;
    if (!row.isScheduled) existing.unscheduledCount += 1;
    if (row.daysToDue < 0) existing.overdueCount += 1;
    if (row.dueDate < existing.earliestDueDate) existing.earliestDueDate = row.dueDate;
    if (urgencyRank(row.urgencyLevel) > urgencyRank(existing.maxUrgencyLevel)) {
      existing.maxUrgencyLevel = row.urgencyLevel;
    }

    const currentRepresentative = existing.representative;
    if (currentRepresentative) {
      const shouldReplace =
        (row.isScheduled ? 0 : 1) > (currentRepresentative.isScheduled ? 0 : 1) ||
        ((row.isScheduled ? 0 : 1) === (currentRepresentative.isScheduled ? 0 : 1) &&
          (urgencyRank(row.urgencyLevel) > urgencyRank(currentRepresentative.urgencyLevel) ||
            (urgencyRank(row.urgencyLevel) === urgencyRank(currentRepresentative.urgencyLevel) &&
              (row.dueDate < currentRepresentative.dueDate ||
                (row.dueDate === currentRepresentative.dueDate &&
                  row.materialId < currentRepresentative.materialId)))));
      if (shouldReplace) {
        existing.representative = row;
        existing.representativeMaterialId = row.materialId;
      }
    }

    existing.materials.push(row);
  });

  return [...map.values()].sort(sortContractRows);
}

export const OrdersContent: React.FC<OrdersContentProps> = ({
  rows,
  contractAggregates,
  urgencyFilter,
  onGoWorkbench,
  onViewDetail,
}) => {
  const [expandedContractKeys, setExpandedContractKeys] = useState<string[]>([]);
  const [contractPage, setContractPage] = useState(1);
  const [materialPageByContract, setMaterialPageByContract] = useState<Record<string, number>>({});
  const [highlightMaterialKey, setHighlightMaterialKey] = useState<string | null>(null);
  const contractPageSize = 20;
  const materialPageSize = 10;

  const filteredRows = useMemo(
    () => (urgencyFilter ? rows.filter((o) => o.urgencyLevel === urgencyFilter) : rows),
    [rows, urgencyFilter],
  );

  const materialRowsByContract = useMemo(() => {
    const map = new Map<string, MaterialFailure[]>();
    filteredRows.forEach((row) => {
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
  }, [filteredRows]);

  const contractRows = useMemo<ContractAggregateRow[]>(() => {
    if (!urgencyFilter && Array.isArray(contractAggregates) && contractAggregates.length > 0) {
      return [...contractAggregates]
        .map((agg) => {
          const materials = materialRowsByContract.get(agg.contractNo) ?? [];
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
        .sort(sortContractRows);
    }
    return buildContractRowsFromMaterials(filteredRows);
  }, [contractAggregates, filteredRows, materialRowsByContract, urgencyFilter]);

  const locateMaterial = useCallback(
    (row: MaterialFailure) => {
      const contractNo = String(row.contractNo || '').trim();
      if (!contractNo) {
        setHighlightMaterialKey(materialRowKey(row));
        return;
      }

      const contractIndex = contractRows.findIndex((it) => it.contractNo === contractNo);
      if (contractIndex >= 0) {
        const targetContract = contractRows[contractIndex];
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
    },
    [contractRows],
  );

  useEffect(() => {
    const validContractSet = new Set(contractRows.map((it) => it.contractNo));
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
    const maxPage = Math.max(1, Math.ceil(contractRows.length / contractPageSize));
    setContractPage((prev) => Math.min(prev, maxPage));
  }, [contractRows]);

  const materialColumns: ColumnsType<MaterialFailure> = [
    { title: '材料号', dataIndex: 'materialId', key: 'materialId', width: 160, ellipsis: true },
    {
      title: '状态',
      key: 'status',
      width: 90,
      render: (_, r) => (
        <TagWithColor color={r.isScheduled ? '#1677ff' : '#fa8c16'}>
          {r.isScheduled ? '已排' : '未排'}
        </TagWithColor>
      ),
    },
    {
      title: '紧急',
      dataIndex: 'urgencyLevel',
      key: 'urgencyLevel',
      width: 90,
      render: (v: MaterialFailure['urgencyLevel']) => (
        <TagWithColor color={getUrgencyLevelColor(v)}>{getUrgencyLevelLabel(v)}</TagWithColor>
      ),
    },
    { title: '交期', dataIndex: 'dueDate', key: 'dueDate', width: 110 },
    { title: '剩余天数', dataIndex: 'daysToDue', key: 'daysToDue', width: 90 },
    {
      title: '失败类型',
      dataIndex: 'failType',
      key: 'failType',
      width: 130,
      render: (v: MaterialFailure['failType']) => (
        <TagWithColor color={getFailTypeColor(v)}>{getFailTypeLabel(v)}</TagWithColor>
      ),
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
      width: onGoWorkbench ? 160 : 90,
      render: (_, record) => (
        <Space size={8}>
          {onGoWorkbench ? (
            <Button
              size="small"
              type="primary"
              onClick={() => {
                locateMaterial(record);
                onGoWorkbench({
                  workbenchTab: 'materials',
                  machineCode: record.machineCode,
                  urgencyLevel: record.urgencyLevel,
                  planDate: record.dueDate,
                  context: 'orders',
                  materialId: record.materialId,
                  contractNo: record.contractNo,
                });
              }}
            >
              处理
            </Button>
          ) : null}
          <Button
            size="small"
            onClick={() => {
              locateMaterial(record);
              onViewDetail(record);
            }}
          >
            详情
          </Button>
        </Space>
      ),
    },
  ];

  const contractColumns: ColumnsType<ContractAggregateRow> = [
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
      render: (v: MaterialFailure['urgencyLevel']) => (
        <TagWithColor color={getUrgencyLevelColor(v)}>{getUrgencyLevelLabel(v)}</TagWithColor>
      ),
    },
    { title: '代表材料', dataIndex: 'representativeMaterialId', key: 'representativeMaterialId', width: 170, ellipsis: true },
    {
      title: '操作',
      key: 'action',
      width: onGoWorkbench ? 150 : 90,
      render: (_, row) => (
        <Space size={8}>
          {onGoWorkbench ? (
            <Tooltip
              title={
                row.representative
                  ? '处理代表材料'
                  : '代表材料未在当前加载批次，缩小过滤范围后可处理'
              }
            >
              <Button
                size="small"
                type="primary"
                disabled={!row.representative}
                onClick={() => {
                  if (!row.representative) return;
                  locateMaterial(row.representative);
                  onGoWorkbench({
                    workbenchTab: 'materials',
                    machineCode: row.representative.machineCode,
                    urgencyLevel: row.representative.urgencyLevel,
                    planDate: row.representative.dueDate,
                    context: 'orders',
                    materialId: row.representative.materialId,
                    contractNo: row.contractNo,
                  });
                }}
              >
                处理
              </Button>
            </Tooltip>
          ) : null}
          <Button
            size="small"
            disabled={!row.representative}
            onClick={() => {
              if (!row.representative) return;
              locateMaterial(row.representative);
              onViewDetail(row.representative);
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
      rowKey={(r) => r.contractNo}
      size="small"
      columns={contractColumns}
      dataSource={contractRows}
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
                  materialRowKey(record) === highlightMaterialKey ? { backgroundColor: '#e6f4ff' } : undefined,
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
              description="当前批次未加载该合同的材料明细，可先按机组/紧急度缩小范围"
            />
          ),
      }}
      pagination={{
        current: contractPage,
        pageSize: contractPageSize,
        onChange: (page) => setContractPage(page),
      }}
    />
  );
};

export default OrdersContent;
