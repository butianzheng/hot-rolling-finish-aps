import { planApi } from '../../../api/tauri';

export type IpcPlanItem = Awaited<ReturnType<typeof planApi.listPlanItems>>[number];

export async function loadPlanItemsIfEmpty(activeVersionId: string, planItems: IpcPlanItem[]): Promise<IpcPlanItem[]> {
  if ((planItems ?? []).length > 0) return planItems;
  // 避免由于 Query 未命中导致误判“未排入”
  return planApi.listPlanItems(activeVersionId);
}

export function buildPlanItemByIdMap(planItems: IpcPlanItem[]): Map<string, IpcPlanItem> {
  const byId = new Map<string, IpcPlanItem>();
  (planItems ?? []).forEach((it) => {
    const id = String(it.material_id ?? '').trim();
    if (id) byId.set(id, it);
  });
  return byId;
}

export function splitSelectedMaterialIds(
  selectedMaterialIds: string[],
  byId: Map<string, IpcPlanItem>
): { eligible: string[]; missing: string[] } {
  const eligible: string[] = [];
  const missing: string[] = [];
  (selectedMaterialIds ?? []).forEach((id) => {
    if (byId.has(id)) eligible.push(id);
    else missing.push(id);
  });
  return { eligible, missing };
}

export function sortMaterialIdsByPlan(materialIds: string[], byId: Map<string, IpcPlanItem>): string[] {
  return [...(materialIds ?? [])].sort((a, b) => {
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
}

