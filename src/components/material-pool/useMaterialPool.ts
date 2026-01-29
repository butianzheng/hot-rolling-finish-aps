/**
 * MaterialPool 状态管理 Hook
 */

import { useMemo, useState } from 'react';
import type {
  MaterialPoolFilters,
  MaterialPoolMaterial,
  MaterialPoolSelection,
  PoolRow,
  UrgencyBucket,
} from './types';
import { URGENCY_ORDER } from './types';
import { buildTreeData, normalizeUrgencyLevel, selectionToTreeKey } from './utils';
import { normalizeSchedState } from '../../utils/schedState';

interface UseMaterialPoolOptions {
  materials: MaterialPoolMaterial[];
  selection: MaterialPoolSelection;
  filters?: MaterialPoolFilters;
  selectedMaterialIds: string[];
  onSelectedMaterialIdsChange: (ids: string[]) => void;
}

export function useMaterialPool({
  materials,
  selection,
  filters,
  selectedMaterialIds,
  onSelectedMaterialIdsChange,
}: UseMaterialPoolOptions) {
  const [searchText, setSearchText] = useState('');
  const [groupByUrgency, setGroupByUrgency] = useState(true);
  const [collapsedUrgency, setCollapsedUrgency] = useState<Record<UrgencyBucket, boolean>>({
    L3: false,
    L2: false,
    L1: false,
    L0: false,
  });

  const treeData = useMemo(() => buildTreeData(materials), [materials]);
  const selectedTreeKey = selectionToTreeKey(selection);

  const filtered = useMemo(() => {
    let list = materials;
    if (selection.machineCode) {
      list = list.filter((m) => String(m.machine_code || '') === selection.machineCode);
    }
    if (selection.schedState) {
      const want = normalizeSchedState(selection.schedState);
      list = list.filter((m) => normalizeSchedState(m.sched_state) === want);
    }
    if (filters?.urgencyLevel) {
      const want = String(filters.urgencyLevel).toUpperCase();
      list = list.filter((m) => String(m.urgent_level || '').toUpperCase() === want);
    }
    if (filters?.lockStatus === 'LOCKED') {
      list = list.filter((m) => !!m.lock_flag);
    } else if (filters?.lockStatus === 'UNLOCKED') {
      list = list.filter((m) => !m.lock_flag);
    }
    if (searchText.trim()) {
      const q = searchText.trim().toLowerCase();
      list = list.filter((m) => String(m.material_id || '').toLowerCase().includes(q));
    }

    return [...list].sort((a, b) => {
      const ma = String(a.machine_code || '');
      const mb = String(b.machine_code || '');
      if (ma !== mb) return ma.localeCompare(mb);
      const ua = String(a.urgent_level || 'L0');
      const ub = String(b.urgent_level || 'L0');
      if (ua !== ub) return ub.localeCompare(ua);
      return String(a.material_id || '').localeCompare(String(b.material_id || ''));
    });
  }, [filters?.lockStatus, filters?.urgencyLevel, materials, searchText, selection.machineCode, selection.schedState]);

  const rows = useMemo<PoolRow[]>(() => {
    if (!groupByUrgency) {
      return filtered.map((m) => ({ type: 'material', material: m }));
    }

    const byLevel = new Map<UrgencyBucket, MaterialPoolMaterial[]>();
    for (const m of filtered) {
      const lvl = normalizeUrgencyLevel(m.urgent_level);
      const list = byLevel.get(lvl);
      if (list) list.push(m);
      else byLevel.set(lvl, [m]);
    }

    const out: PoolRow[] = [];
    for (const lvl of URGENCY_ORDER) {
      const list = byLevel.get(lvl);
      if (!list || list.length === 0) continue;
      const weight = list.reduce((sum, m) => sum + Number(m.weight_t || 0), 0);
      const collapsed = !!collapsedUrgency[lvl];
      out.push({ type: 'header', level: lvl, count: list.length, weight, collapsed });
      if (!collapsed) {
        list.forEach((m) => out.push({ type: 'material', material: m }));
      }
    }
    return out;
  }, [collapsedUrgency, filtered, groupByUrgency]);

  const selectedSet = useMemo(() => new Set(selectedMaterialIds), [selectedMaterialIds]);

  const toggleSelection = (materialId: string, checked: boolean) => {
    const next = new Set(selectedSet);
    if (checked) next.add(materialId);
    else next.delete(materialId);
    onSelectedMaterialIdsChange(Array.from(next));
  };

  const selectAllVisible = () => {
    const next = new Set(selectedSet);
    filtered.forEach((m) => next.add(m.material_id));
    onSelectedMaterialIdsChange(Array.from(next));
  };

  const clearSelection = () => onSelectedMaterialIdsChange([]);

  const toggleUrgencyCollapse = (level: UrgencyBucket) => {
    setCollapsedUrgency((prev) => ({ ...prev, [level]: !prev[level] }));
  };

  return {
    // 状态
    searchText,
    setSearchText,
    groupByUrgency,
    setGroupByUrgency,
    treeData,
    selectedTreeKey,
    filtered,
    rows,
    selectedSet,

    // 操作
    toggleSelection,
    selectAllVisible,
    clearSelection,
    toggleUrgencyCollapse,
  };
}
