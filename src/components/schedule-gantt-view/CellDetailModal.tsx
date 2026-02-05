/**
 * 单元格详情弹窗
 */

import React, { useMemo } from 'react';
import { Alert, Button, Modal, Tag, Typography } from 'antd';
import { FONT_FAMILIES } from '../../theme';
import { formatCapacity, formatPercent, formatWeight } from '../../utils/formatters';
import type { PlanItemRow, CapacityData, CellDetail } from './types';
import { getUrgentTagColor } from './utils';
import type { PlanItemStatusFilter } from '../../utils/planItemStatus';
import { PLAN_ITEM_STATUS_FILTER_META, matchPlanItemStatusFilter } from '../../utils/planItemStatus';

const { Text } = Typography;

interface CellDetailModalProps {
  cellDetail: CellDetail;
  itemsByMachineDate: Map<string, Map<string, PlanItemRow[]>>;
  capacityByMachineDate: Map<string, CapacityData>;
  selectedSet: Set<string>;
  selectedMaterialIds: string[];
  onClose: () => void;
  onToggleSelection: (id: string, checked: boolean) => void;
  onInspectMaterialId?: (id: string) => void;
  onRequestMoveToCell?: (machine: string, date: string) => void;
  viewUrgentFilter?: string | null;
  viewStatusFilter?: PlanItemStatusFilter;
  onNavigateToMatrix?: (machine: string, date: string) => void;
}

export const CellDetailModal: React.FC<CellDetailModalProps> = ({
  cellDetail,
  itemsByMachineDate,
  capacityByMachineDate,
  selectedSet,
  selectedMaterialIds,
  onClose,
  onToggleSelection,
  onInspectMaterialId,
  onRequestMoveToCell,
  viewUrgentFilter,
  viewStatusFilter,
  onNavigateToMatrix,
}) => {
  const cellDetailItems = useMemo(() => {
    if (!cellDetail) return [];
    const byDate = itemsByMachineDate.get(cellDetail.machine);
    const list = byDate?.get(cellDetail.date) ?? [];
    let sorted = [...list].sort((a, b) => a.seq_no - b.seq_no);
    if (viewStatusFilter && viewStatusFilter !== 'ALL') {
      sorted = sorted.filter((it) => matchPlanItemStatusFilter(it, viewStatusFilter));
    }
    return sorted;
  }, [cellDetail, itemsByMachineDate, viewStatusFilter]);

  const cellCapacity = useMemo(() => {
    if (!cellDetail) return null;
    return capacityByMachineDate.get(`${cellDetail.machine}__${cellDetail.date}`) ?? null;
  }, [capacityByMachineDate, cellDetail]);

  return (
    <Modal
      title={cellDetail ? `同日明细：${cellDetail.machine} / ${cellDetail.date}` : '同日明细'}
      open={!!cellDetail}
      onCancel={onClose}
      footer={
        cellDetail
          ? [
              onNavigateToMatrix ? (
                <Button
                  key="matrix"
                  onClick={() => {
                    onNavigateToMatrix(cellDetail.machine, cellDetail.date);
                    onClose();
                  }}
                >
                  在矩阵查看
                </Button>
              ) : null,
              onRequestMoveToCell && selectedMaterialIds.length > 0 ? (
                <Button
                  key="move"
                  type="primary"
                  onClick={() => onRequestMoveToCell(cellDetail.machine, cellDetail.date)}
                >
                  移动已选({selectedMaterialIds.length})到这里
                </Button>
              ) : null,
              <Button key="close" onClick={onClose}>
                关闭
              </Button>,
            ].filter(Boolean)
          : null
      }
      width={900}
      destroyOnClose
    >
      {((viewUrgentFilter && viewUrgentFilter !== 'all') ||
        (viewStatusFilter && viewStatusFilter !== 'ALL')) ? (
        <Text type="secondary" style={{ fontSize: 12, display: 'block', marginBottom: 12 }}>
          甘特图当前筛选：
          {viewUrgentFilter && viewUrgentFilter !== 'all' ? ` ${String(viewUrgentFilter).toUpperCase()}` : ' 全部紧急度'}
          {viewStatusFilter && viewStatusFilter !== 'ALL'
            ? ` · ${PLAN_ITEM_STATUS_FILTER_META[viewStatusFilter].label}`
            : ''}
          （同日明细不受紧急度筛选影响；状态筛选在明细中生效）
        </Text>
      ) : null}
      {cellCapacity ? (
        <Alert
          type={
            cellCapacity.limit > 0 && cellCapacity.used > cellCapacity.limit
              ? 'error'
              : cellCapacity.target > 0 && cellCapacity.used > cellCapacity.target
              ? 'warning'
              : 'info'
          }
          showIcon
          message={`产能 ${formatCapacity(cellCapacity.used)} / 目标 ${formatCapacity(cellCapacity.target)} / 上限 ${formatCapacity(
            cellCapacity.limit
          )}（利用率 ${formatPercent(
            (cellCapacity.used / Math.max(cellCapacity.limit || cellCapacity.target || 0, 1)) * 100
          )}）`}
          style={{ marginBottom: 12 }}
        />
      ) : (
        <Text type="secondary" style={{ fontSize: 12, display: 'block', marginBottom: 12 }}>
          暂无产能池数据
        </Text>
      )}
      {cellDetailItems.length === 0 ? (
        <Alert type="info" showIcon message="该单元格暂无数据" />
      ) : (
        <div style={{ maxHeight: 560, overflow: 'auto' }}>
          <table style={{ width: '100%', borderCollapse: 'collapse' }}>
            <thead>
              <tr style={{ textAlign: 'left', borderBottom: '1px solid #f0f0f0' }}>
                <th style={{ width: 60, padding: '8px 6px' }}>选择</th>
                <th style={{ width: 180, padding: '8px 6px' }}>物料</th>
                <th style={{ width: 80, padding: '8px 6px' }}>序号</th>
                <th style={{ width: 90, padding: '8px 6px' }}>紧急</th>
                <th style={{ width: 120, padding: '8px 6px' }}>重量</th>
                <th style={{ width: 110, padding: '8px 6px' }}>厚度(mm)</th>
                <th style={{ width: 110, padding: '8px 6px' }}>宽度(mm)</th>
                <th style={{ width: 90, padding: '8px 6px' }}>冻结</th>
                <th style={{ width: 120, padding: '8px 6px' }}>操作</th>
              </tr>
            </thead>
            <tbody>
              {cellDetailItems.map((it) => {
                const checked = selectedSet.has(it.material_id);
                const urgent = String(it.urgent_level || 'L0');
                const urgentColor = getUrgentTagColor(urgent);
                const thickness = it.thickness_mm;
                const width = it.width_mm;

                return (
                  <tr key={it.material_id} style={{ borderBottom: '1px solid #f5f5f5' }}>
                    <td style={{ padding: '8px 6px' }}>
                      <input
                        type="checkbox"
                        checked={checked}
                        onChange={(e) => onToggleSelection(it.material_id, e.target.checked)}
                      />
                    </td>
                    <td style={{ padding: '8px 6px', fontFamily: FONT_FAMILIES.MONOSPACE }}>
                      {it.material_id}
                    </td>
                    <td style={{ padding: '8px 6px' }}>{it.seq_no}</td>
                    <td style={{ padding: '8px 6px' }}>
                      <Tag color={urgentColor}>{urgent}</Tag>
                    </td>
                    <td style={{ padding: '8px 6px' }}>{formatWeight(it.weight_t)}</td>
                    <td style={{ padding: '8px 6px', fontFamily: FONT_FAMILIES.MONOSPACE }}>
                      {thickness == null || !Number.isFinite(Number(thickness)) ? '-' : Number(thickness).toFixed(2)}
                    </td>
                    <td style={{ padding: '8px 6px', fontFamily: FONT_FAMILIES.MONOSPACE }}>
                      {width == null || !Number.isFinite(Number(width)) ? '-' : Number(width).toFixed(2)}
                    </td>
                    <td style={{ padding: '8px 6px' }}>{it.locked_in_plan ? '是' : '否'}</td>
                    <td style={{ padding: '8px 6px' }}>
                      <Button size="small" onClick={() => onInspectMaterialId?.(it.material_id)}>
                        查看
                      </Button>
                    </td>
                  </tr>
                );
              })}
            </tbody>
          </table>
        </div>
      )}
    </Modal>
  );
};
