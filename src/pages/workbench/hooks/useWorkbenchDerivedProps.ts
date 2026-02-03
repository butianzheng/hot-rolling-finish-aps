import { useMemo } from 'react';

import type { MaterialPoolFilters, MaterialPoolMaterial } from '../../../components/workbench/MaterialPool';

export function useWorkbenchDerivedProps(params: {
  materials: MaterialPoolMaterial[];
  selectedMaterialIds: string[];
  urgencyLevel: MaterialPoolFilters['urgencyLevel'];
  lockStatus: MaterialPoolFilters['lockStatus'];
}): {
  poolFilters: MaterialPoolFilters;
  selectedTotalWeight: number;
  machineOptions: string[];
} {
  const { materials, selectedMaterialIds, urgencyLevel, lockStatus } = params;

  const poolFilters = useMemo<MaterialPoolFilters>(() => {
    return { urgencyLevel, lockStatus };
  }, [lockStatus, urgencyLevel]);

  const weightById = useMemo(() => {
    const map = new Map<string, number>();
    (materials ?? []).forEach((m) => {
      const id = String(m.material_id ?? '').trim();
      if (!id) return;
      const w = Number(m.weight_t ?? 0);
      map.set(id, Number.isFinite(w) ? w : 0);
    });
    return map;
  }, [materials]);

  const selectedTotalWeight = useMemo(() => {
    return (selectedMaterialIds ?? []).reduce((sum, id) => sum + (weightById.get(id) ?? 0), 0);
  }, [selectedMaterialIds, weightById]);

  const machineOptions = useMemo(() => {
    const codes = new Set<string>();
    (materials ?? []).forEach((m) => {
      const code = String(m.machine_code ?? '').trim();
      if (code) codes.add(code);
    });
    return Array.from(codes).sort();
  }, [materials]);

  return { poolFilters, selectedTotalWeight, machineOptions };
}

