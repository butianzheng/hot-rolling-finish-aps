import { useCallback, useMemo, useState } from 'react';
import type { Dispatch, SetStateAction } from 'react';
import { useQuery } from '@tanstack/react-query';

import { materialApi } from '../../../api/tauri';
import type { Material as InspectorMaterial } from '../../../components/MaterialInspector';
import type { MaterialPoolMaterial } from '../../../components/workbench/MaterialPool';
import { normalizeSchedState } from '../../../utils/schedState';

type IpcMaterialDetail = Awaited<ReturnType<typeof materialApi.getMaterialDetail>>;

export function useWorkbenchInspector(params: { materials: MaterialPoolMaterial[] }): {
  inspectorOpen: boolean;
  setInspectorOpen: Dispatch<SetStateAction<boolean>>;
  inspectedMaterialId: string | null;
  setInspectedMaterialId: Dispatch<SetStateAction<string | null>>;
  inspectedMaterial: InspectorMaterial | null;
  openInspector: (materialId: string) => void;
} {
  const { materials } = params;

  const [inspectorOpen, setInspectorOpen] = useState(false);
  const [inspectedMaterialId, setInspectedMaterialId] = useState<string | null>(null);

  const materialDetailQuery = useQuery({
    queryKey: ['materialDetail', inspectedMaterialId],
    enabled: !!inspectedMaterialId,
    queryFn: async () => {
      if (!inspectedMaterialId) return null;
      return materialApi.getMaterialDetail(inspectedMaterialId);
    },
    staleTime: 30 * 1000,
  });

  const inspectedMaterial = useMemo<InspectorMaterial | null>(() => {
    if (!inspectedMaterialId) return null;
    const fromList = materials.find((m) => m.material_id === inspectedMaterialId) || null;
    const detail: IpcMaterialDetail | null = materialDetailQuery.data ?? null;
    const master = detail?.master ?? null;
    const state = detail?.state ?? null;

    const sched_state = String(state?.sched_state ?? fromList?.sched_state ?? '').trim();
    const normalizedSched = normalizeSchedState(sched_state);
    const is_mature =
      normalizedSched === 'PENDING_MATURE'
        ? false
        : normalizedSched === 'READY' || normalizedSched === 'FORCE_RELEASE' || normalizedSched === 'SCHEDULED'
          ? true
          : undefined;

    const machineFromMaster = String(
      master?.next_machine_code ?? master?.current_machine_code ?? master?.rework_machine_code ?? ''
    ).trim();

    return {
      material_id: String(master?.material_id ?? state?.material_id ?? fromList?.material_id ?? inspectedMaterialId).trim(),
      machine_code: String(fromList?.machine_code ?? machineFromMaster ?? '').trim(),
      weight_t: Number(fromList?.weight_t ?? master?.weight_t ?? 0),
      steel_mark: String(fromList?.steel_mark ?? master?.steel_mark ?? '').trim(),
      sched_state,
      urgent_level: String(state?.urgent_level ?? fromList?.urgent_level ?? '').trim(),
      lock_flag: Boolean(state?.lock_flag ?? fromList?.lock_flag ?? false),
      manual_urgent_flag: Boolean(state?.manual_urgent_flag ?? fromList?.manual_urgent_flag ?? false),
      is_mature,
      temp_issue: false,
      urgent_reason: state?.urgent_reason ? String(state.urgent_reason) : undefined,
      eligibility_reason: undefined,
      priority_reason: undefined,
    };
  }, [inspectedMaterialId, materialDetailQuery.data, materials]);

  const openInspector = useCallback((materialId: string) => {
    const id = String(materialId || '').trim();
    if (!id) return;
    setInspectedMaterialId(id);
    setInspectorOpen(true);
  }, []);

  return {
    inspectorOpen,
    setInspectorOpen,
    inspectedMaterialId,
    setInspectedMaterialId,
    inspectedMaterial,
    openInspector,
  };
}
