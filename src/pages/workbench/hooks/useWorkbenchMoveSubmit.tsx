import { useCallback, useState } from 'react';
import type { Dispatch, SetStateAction } from 'react';
import type { Dayjs } from 'dayjs';
import { Alert, Modal, Space, Table, message } from 'antd';

import { planApi } from '../../../api/tauri';
import { formatDate } from '../../../utils/formatters';
import { getErrorMessage } from '../../../utils/errorUtils';
import type { MoveItemResultRow, MoveSeqMode, MoveValidationMode } from '../types';

type IpcPlanItem = Awaited<ReturnType<typeof planApi.listPlanItems>>[number];
type IpcMoveItemsResponse = Awaited<ReturnType<typeof planApi.moveItems>>;

export function useWorkbenchMoveSubmit(params: {
  activeVersionId: string | null;
  operator: string | null;
  moveTargetMachine: string | null;
  moveTargetDate: Dayjs | null;
  moveReason: string;
  moveSeqMode: MoveSeqMode;
  moveStartSeq: number;
  moveValidationMode: MoveValidationMode;
  planItems: IpcPlanItem[];
  selectedMaterialIds: string[];
  setMoveModalOpen: Dispatch<SetStateAction<boolean>>;
  setMoveReason: Dispatch<SetStateAction<string>>;
  setSelectedMaterialIds: Dispatch<SetStateAction<string[]>>;
  bumpRefreshSignal: () => void;
  materialsRefetch: () => void;
  planItemsRefetch: () => void;
}): {
  moveSubmitting: boolean;
  submitMove: () => Promise<void>;
} {
  const {
    activeVersionId,
    operator,
    moveTargetMachine,
    moveTargetDate,
    moveReason,
    moveSeqMode,
    moveStartSeq,
    moveValidationMode,
    planItems,
    selectedMaterialIds,
    setMoveModalOpen,
    setMoveReason,
    setSelectedMaterialIds,
    bumpRefreshSignal,
    materialsRefetch,
    planItemsRefetch,
  } = params;

  const [moveSubmitting, setMoveSubmitting] = useState(false);

  const submitMove = useCallback(async () => {
    if (!activeVersionId) {
      message.warning('请先激活一个版本');
      return;
    }
    if (!moveTargetMachine) {
      message.warning('请选择目标机组');
      return;
    }
    if (!moveTargetDate || !moveTargetDate.isValid()) {
      message.warning('请选择目标日期');
      return;
    }
    const reason = moveReason.trim();
    if (!reason) {
      message.warning('请输入移动原因');
      return;
    }

    setMoveSubmitting(true);
    try {
      const targetDate = formatDate(moveTargetDate);

      let planItemsRaw: IpcPlanItem[] = planItems ?? [];
      if (planItemsRaw.length === 0) {
        // 避免由于 Query 未命中导致误判“未排入”
        const fetched = await planApi.listPlanItems(activeVersionId);
        planItemsRaw = fetched;
      }

      const byId = new Map<string, IpcPlanItem>();
      planItemsRaw.forEach((it) => {
        const id = String(it.material_id ?? '').trim();
        if (id) byId.set(id, it);
      });

      const eligible = selectedMaterialIds.filter((id) => byId.has(id));
      const missing = selectedMaterialIds.filter((id) => !byId.has(id));

      if (eligible.length === 0) {
        message.error('所选物料不在当前版本排程中，无法移动');
        return;
      }

      const ordered = [...eligible].sort((a, b) => {
        const ia = byId.get(a);
        const ib = byId.get(b);
        const da = String(ia?.plan_date ?? '');
        const db = String(ib?.plan_date ?? '');
        if (da !== db) return da.localeCompare(db);
        const ma = String(ia?.machine_code ?? '');
        const mb = String(ib?.machine_code ?? '');
        if (ma !== mb) return ma.localeCompare(mb);
        return Number(ia?.seq_no ?? 0) - Number(ib?.seq_no ?? 0);
      });

      let startSeq = Math.max(1, Math.floor(Number(moveStartSeq || 1)));
      if (moveSeqMode === 'APPEND') {
        const maxSeq = planItemsRaw
          .filter((it) => String(it.machine_code ?? '') === moveTargetMachine && String(it.plan_date ?? '') === targetDate)
          .reduce((max: number, it) => Math.max(max, Number(it.seq_no ?? 0)), 0);
        startSeq = Math.max(1, maxSeq + 1);
      }

      const moves = ordered.map((id, idx) => ({
        material_id: id,
        to_date: targetDate,
        to_seq: startSeq + idx,
        to_machine: moveTargetMachine,
      }));

      const actualOperator = operator || 'admin';
      const res: IpcMoveItemsResponse = await planApi.moveItems(
        activeVersionId,
        moves,
        moveValidationMode,
        actualOperator,
        reason
      );

      setMoveModalOpen(false);
      setMoveReason('');
      setSelectedMaterialIds([]);
      bumpRefreshSignal();
      materialsRefetch();
      planItemsRefetch();

      const failedCount = Number(res?.failed_count ?? 0);
      if (failedCount > 0) {
        const results: MoveItemResultRow[] = (res.results ?? []).map((r) => ({
          material_id: String(r?.material_id ?? ''),
          success: Boolean(r?.success),
          from_machine: r?.from_machine == null ? null : String(r.from_machine),
          from_date: r?.from_date == null ? null : String(r.from_date),
          to_machine: String(r?.to_machine ?? ''),
          to_date: String(r?.to_date ?? ''),
          error: r?.error == null ? null : String(r.error),
          violation_type: r?.violation_type == null ? null : String(r.violation_type),
        }));
        Modal.info({
          title: '移动完成（部分失败）',
          width: 920,
          content: (
            <Space direction="vertical" style={{ width: '100%' }} size={12}>
              <Alert type="warning" showIcon message={String(res?.message || '移动完成')} />
              {missing.length > 0 && (
                <Alert type="info" showIcon message={`有 ${missing.length} 个物料不在当前版本排程中，已跳过`} />
              )}
              <Table<MoveItemResultRow>
                size="small"
                rowKey={(r) => r.material_id}
                pagination={false}
                dataSource={results}
                columns={[
                  { title: '物料', dataIndex: 'material_id', width: 160 },
                  {
                    title: '结果',
                    dataIndex: 'success',
                    width: 80,
                    render: (v) => (v ? '成功' : '失败'),
                  },
                  {
                    title: '原位置',
                    key: 'from',
                    width: 220,
                    render: (_, r) => `${r.from_machine || '-'}/${r.from_date || '-'}`,
                  },
                  {
                    title: '目标位置',
                    key: 'to',
                    width: 220,
                    render: (_, r) => `${r.to_machine || '-'}/${r.to_date || '-'}`,
                  },
                  { title: '原因', dataIndex: 'error' },
                ]}
                scroll={{ y: 320 }}
              />
            </Space>
          ),
        });
      } else {
        message.success(String(res?.message || '移动完成'));
        if (missing.length > 0) {
          message.info(`有 ${missing.length} 个物料不在当前版本排程中，已跳过`);
        }
      }
    } catch (e: unknown) {
      console.error('[Workbench] moveItems failed:', e);
      message.error(getErrorMessage(e) || '移动失败');
    } finally {
      setMoveSubmitting(false);
    }
  }, [
    activeVersionId,
    bumpRefreshSignal,
    materialsRefetch,
    moveReason,
    moveSeqMode,
    moveStartSeq,
    moveTargetDate,
    moveTargetMachine,
    moveValidationMode,
    operator,
    planItems,
    planItemsRefetch,
    selectedMaterialIds,
    setMoveModalOpen,
    setMoveReason,
    setSelectedMaterialIds,
  ]);

  return { moveSubmitting, submitMove };
}

