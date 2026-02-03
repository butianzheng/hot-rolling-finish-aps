import type { MoveSeqMode } from '../types';
import type { IpcPlanItem } from './planItems';

export type MoveItemRequest = {
  material_id: string;
  to_date: string;
  to_seq: number;
  to_machine: string;
};

export function computeMoveStartSeq(params: {
  moveSeqMode: MoveSeqMode;
  moveStartSeq: number;
  planItems: IpcPlanItem[];
  moveTargetMachine: string;
  targetDate: string;
}): number {
  const { moveSeqMode, moveStartSeq, planItems, moveTargetMachine, targetDate } = params;

  let startSeq = Math.max(1, Math.floor(Number(moveStartSeq || 1)));
  if (moveSeqMode !== 'APPEND') return startSeq;

  const maxSeq = (planItems ?? [])
    .filter((it) => String(it.machine_code ?? '') === moveTargetMachine && String(it.plan_date ?? '') === targetDate)
    .reduce((max: number, it) => Math.max(max, Number(it.seq_no ?? 0)), 0);
  startSeq = Math.max(1, maxSeq + 1);
  return startSeq;
}

export function buildMoveRequests(params: {
  orderedMaterialIds: string[];
  targetMachine: string;
  targetDate: string;
  startSeq: number;
}): MoveItemRequest[] {
  const { orderedMaterialIds, targetMachine, targetDate, startSeq } = params;
  return (orderedMaterialIds ?? []).map((id, idx) => ({
    material_id: id,
    to_date: targetDate,
    to_seq: startSeq + idx,
    to_machine: targetMachine,
  }));
}

export function validateMoveSubmitParams(params: {
  activeVersionId: string | null;
  moveTargetMachine: string | null;
  moveTargetDateValid: boolean;
  moveReason: string;
}): string | null {
  const { activeVersionId, moveTargetMachine, moveTargetDateValid, moveReason } = params;
  if (!activeVersionId) return '请先激活一个版本';
  if (!moveTargetMachine) return '请选择目标机组';
  if (!moveTargetDateValid) return '请选择目标日期';
  if (!String(moveReason || '').trim()) return '请输入移动原因';
  return null;
}
