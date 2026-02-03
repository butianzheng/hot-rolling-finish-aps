import { useCallback, useEffect, useMemo, useState } from 'react';
import type { Dispatch, SetStateAction } from 'react';
import dayjs from 'dayjs';
import { Alert, Modal, Space, Table, message } from 'antd';
import { useQuery } from '@tanstack/react-query';
import { capacityApi, planApi } from '../../../api/tauri';
import { formatDate } from '../../../utils/formatters';
import { getErrorMessage } from '../../../utils/errorUtils';
import { DEFAULT_MOVE_REASON } from '../constants';
import type { MoveImpactRow, MoveItemResultRow, MoveSeqMode, MoveValidationMode } from '../types';

type IpcPlanItem = Awaited<ReturnType<typeof planApi.listPlanItems>>[number];
type IpcCapacityPool = Awaited<ReturnType<typeof capacityApi.getCapacityPools>>[number];
type IpcMoveItemsResponse = Awaited<ReturnType<typeof planApi.moveItems>>;

export type SelectedPlanItemStats = {
  inPlan: number;
  frozenInPlan: number;
  outOfPlan: number;
};

export type MoveImpactPreview = {
  rows: MoveImpactRow[];
  overflowRows: MoveImpactRow[];
  loading: boolean;
};

export type MoveRecommendSummary = {
  machine: string;
  date: string;
  overLimitCount: number;
  unknownCount: number;
  totalOverT: number;
  maxUtilPct: number;
};

export function useWorkbenchMoveModal(params: {
  activeVersionId: string | null;
  operator: string | null;
  deepLinkDate: string | null;
  poolMachineCode: string | null;
  machineOptions: string[];
  defaultStrategy: string | null | undefined;
  workbenchDateRange: [dayjs.Dayjs, dayjs.Dayjs];
  planItems: IpcPlanItem[];
  planItemsRefetch: () => void;
  selectedMaterialIds: string[];
  setSelectedMaterialIds: Dispatch<SetStateAction<string[]>>;
  bumpRefreshSignal: () => void;
  materialsRefetch: () => void;
}): {
  moveModalOpen: boolean;
  setMoveModalOpen: Dispatch<SetStateAction<boolean>>;
  moveTargetMachine: string | null;
  setMoveTargetMachine: Dispatch<SetStateAction<string | null>>;
  moveTargetDate: dayjs.Dayjs | null;
  setMoveTargetDate: Dispatch<SetStateAction<dayjs.Dayjs | null>>;
  moveSeqMode: MoveSeqMode;
  setMoveSeqMode: Dispatch<SetStateAction<MoveSeqMode>>;
  moveStartSeq: number;
  setMoveStartSeq: Dispatch<SetStateAction<number>>;
  moveValidationMode: MoveValidationMode;
  setMoveValidationMode: Dispatch<SetStateAction<MoveValidationMode>>;
  moveSubmitting: boolean;
  moveReason: string;
  setMoveReason: Dispatch<SetStateAction<string>>;
  moveRecommendLoading: boolean;
  moveRecommendSummary: MoveRecommendSummary | null;
  strategyLabel: string;
  selectedPlanItemStats: SelectedPlanItemStats;
  moveImpactPreview: MoveImpactPreview | null;
  recommendMoveTarget: () => Promise<void>;
  openMoveModal: () => void;
  openMoveModalAt: (targetMachine: string, targetDate: string) => void;
  openMoveModalWithRecommend: () => void;
  submitMove: () => Promise<void>;
} {
  const {
    activeVersionId,
    operator,
    deepLinkDate,
    poolMachineCode,
    machineOptions,
    defaultStrategy,
    workbenchDateRange,
    planItems,
    planItemsRefetch,
    selectedMaterialIds,
    setSelectedMaterialIds,
    bumpRefreshSignal,
    materialsRefetch,
  } = params;

  const [moveModalOpen, setMoveModalOpen] = useState(false);
  const [moveTargetMachine, setMoveTargetMachine] = useState<string | null>(null);
  const [moveTargetDate, setMoveTargetDate] = useState<dayjs.Dayjs | null>(dayjs());
  const [moveSeqMode, setMoveSeqMode] = useState<MoveSeqMode>('APPEND');
  const [moveStartSeq, setMoveStartSeq] = useState<number>(1);
  const [moveValidationMode, setMoveValidationMode] = useState<MoveValidationMode>('AUTO_FIX');
  const [moveSubmitting, setMoveSubmitting] = useState(false);
  const [moveReason, setMoveReason] = useState<string>('');
  const [moveRecommendLoading, setMoveRecommendLoading] = useState(false);
  const [moveRecommendSummary, setMoveRecommendSummary] = useState<MoveRecommendSummary | null>(null);
  const [autoRecommendOnOpen, setAutoRecommendOnOpen] = useState(false);

  const strategyLabel = useMemo(() => {
    const v = String(defaultStrategy || 'balanced');
    if (v === 'urgent_first') return '紧急优先';
    if (v === 'capacity_first') return '产能优先';
    if (v === 'cold_stock_first') return '冷坯消化';
    if (v === 'manual') return '手动调整';
    return '均衡方案';
  }, [defaultStrategy]);

  const planItemById = useMemo(() => {
    const map = new Map<string, IpcPlanItem>();
    const raw = planItems ?? [];
    raw.forEach((it) => {
      const id = String(it.material_id ?? '').trim();
      if (id) map.set(id, it);
    });
    return map;
  }, [planItems]);

  const selectedPlanItemStats = useMemo<SelectedPlanItemStats>(() => {
    let inPlan = 0;
    let frozenInPlan = 0;
    selectedMaterialIds.forEach((id) => {
      const it = planItemById.get(id);
      if (!it) return;
      inPlan += 1;
      if (it?.locked_in_plan === true) frozenInPlan += 1;
    });
    return { inPlan, frozenInPlan, outOfPlan: selectedMaterialIds.length - inPlan };
  }, [planItemById, selectedMaterialIds]);

  const moveImpactBase = useMemo(() => {
    if (!moveModalOpen) return null;
    if (!moveTargetMachine) return null;
    if (!moveTargetDate || !moveTargetDate.isValid()) return null;

    const targetDate = formatDate(moveTargetDate);
    const raw = planItems ?? [];

    const tonnageMap = new Map<string, number>();
    raw.forEach((it) => {
      const machine = String(it.machine_code ?? '').trim();
      const date = String(it.plan_date ?? '').trim();
      if (!machine || !date) return;
      const weight = Number(it.weight_t ?? 0);
      if (!Number.isFinite(weight) || weight <= 0) return;
      const key = `${machine}__${date}`;
      tonnageMap.set(key, (tonnageMap.get(key) ?? 0) + weight);
    });

    const byId = new Map<string, IpcPlanItem>();
    raw.forEach((it) => {
      const id = String(it.material_id ?? '').trim();
      if (id) byId.set(id, it);
    });

    const deltaMap = new Map<string, number>();
    selectedMaterialIds.forEach((id) => {
      const it = byId.get(id);
      if (!it) return;
      const fromMachine = String(it.machine_code ?? '').trim();
      const fromDate = String(it.plan_date ?? '').trim();
      if (!fromMachine || !fromDate) return;
      const weight = Number(it.weight_t ?? 0);
      if (!Number.isFinite(weight) || weight <= 0) return;

      const fromKey = `${fromMachine}__${fromDate}`;
      const toKey = `${moveTargetMachine}__${targetDate}`;
      if (fromKey === toKey) return;
      deltaMap.set(fromKey, (deltaMap.get(fromKey) ?? 0) - weight);
      deltaMap.set(toKey, (deltaMap.get(toKey) ?? 0) + weight);
    });

    const affectedKeys = Array.from(deltaMap.entries())
      .filter(([, delta]) => Number.isFinite(delta) && Math.abs(delta) > 1e-9)
      .map(([key]) => key);

    if (affectedKeys.length === 0) {
      return {
        targetDate,
        affectedMachines: [moveTargetMachine],
        dateFrom: targetDate,
        dateTo: targetDate,
        rows: [] as MoveImpactRow[],
      };
    }

    const dates = affectedKeys.map((k) => k.split('__')[1]).filter(Boolean).sort();
    const dateFrom = dates[0] || targetDate;
    const dateTo = dates[dates.length - 1] || targetDate;
    const affectedMachines = Array.from(
      new Set(affectedKeys.map((k) => k.split('__')[0]).filter(Boolean))
    ).sort();

    const rows: MoveImpactRow[] = affectedKeys
      .map((key) => {
        const [machine, date] = key.split('__');
        const before = tonnageMap.get(key) ?? 0;
        const delta = deltaMap.get(key) ?? 0;
        const after = before + delta;
        return {
          machine_code: machine,
          date,
          before_t: before,
          delta_t: delta,
          after_t: after,
          target_capacity_t: null,
          limit_capacity_t: null,
        };
      })
      .sort((a, b) =>
        a.date === b.date ? a.machine_code.localeCompare(b.machine_code) : a.date.localeCompare(b.date)
      );

    return { targetDate, affectedMachines, dateFrom, dateTo, rows };
  }, [moveModalOpen, moveTargetMachine, moveTargetDate, planItems, selectedMaterialIds]);

  const moveImpactCapacityQuery = useQuery({
    queryKey: [
      'moveImpactCapacity',
      activeVersionId,
      moveImpactBase?.affectedMachines.join(',') || '',
      moveImpactBase?.dateFrom || '',
      moveImpactBase?.dateTo || '',
    ],
    enabled:
      !!activeVersionId &&
      !!moveModalOpen &&
      !!moveImpactBase &&
      moveImpactBase.affectedMachines.length > 0 &&
      !!moveImpactBase.dateFrom &&
      !!moveImpactBase.dateTo,
    queryFn: async () => {
      if (!activeVersionId || !moveImpactBase) return [];
      return capacityApi.getCapacityPools(
        moveImpactBase.affectedMachines,
        moveImpactBase.dateFrom,
        moveImpactBase.dateTo,
        activeVersionId
      );
    },
    staleTime: 30 * 1000,
  });

  const moveImpactPreview = useMemo<MoveImpactPreview | null>(() => {
    if (!moveImpactBase) return null;
    const pools: IpcCapacityPool[] = moveImpactCapacityQuery.data ?? [];
    const poolMap = new Map<string, { target: number | null; limit: number | null }>();
    pools.forEach((p) => {
      const machine = String(p.machine_code ?? '').trim();
      const date = String(p.plan_date ?? '').trim();
      if (!machine || !date) return;
      const target = Number(p.target_capacity_t ?? 0);
      const limit = Number(p.limit_capacity_t ?? 0);
      poolMap.set(`${machine}__${date}`, {
        target: Number.isFinite(target) && target > 0 ? target : null,
        limit: Number.isFinite(limit) && limit > 0 ? limit : null,
      });
    });

    const rows = moveImpactBase.rows.map((r) => {
      const cap = poolMap.get(`${r.machine_code}__${r.date}`);
      return {
        ...r,
        target_capacity_t: cap?.target ?? null,
        limit_capacity_t: cap?.limit ?? cap?.target ?? null,
      };
    });

    const overflowRows = rows.filter((r) => {
      if (r.limit_capacity_t == null) return false;
      return r.after_t > r.limit_capacity_t + 1e-9;
    });

    return { rows, overflowRows, loading: moveImpactCapacityQuery.isLoading };
  }, [moveImpactBase, moveImpactCapacityQuery.data, moveImpactCapacityQuery.isLoading]);

  const recommendMoveTarget = useCallback(async () => {
    if (!activeVersionId) {
      message.warning('请先激活一个版本');
      return;
    }
    const targetMachine = String(moveTargetMachine || '').trim();
    if (!targetMachine) {
      message.warning('请先选择目标机组');
      return;
    }

    // 仅基于“可移动”的已选物料做推荐（AUTO_FIX 模式下，冻结项会被跳过）
    let planItemsRaw: IpcPlanItem[] = planItems ?? [];
    if (planItemsRaw.length === 0) {
      const fetched = await planApi.listPlanItems(activeVersionId);
      planItemsRaw = fetched;
    }

    const byId = new Map<string, IpcPlanItem>();
    const tonnageMap = new Map<string, number>();
    planItemsRaw.forEach((it) => {
      const id = String(it.material_id ?? '').trim();
      if (id) byId.set(id, it);
      const machine = String(it.machine_code ?? '').trim();
      const date = String(it.plan_date ?? '').trim();
      if (!machine || !date) return;
      const weight = Number(it.weight_t ?? 0);
      if (!Number.isFinite(weight) || weight <= 0) return;
      const key = `${machine}__${date}`;
      tonnageMap.set(key, (tonnageMap.get(key) ?? 0) + weight);
    });

    const movable = selectedMaterialIds
      .map((id) => byId.get(id))
      .filter((it): it is IpcPlanItem => Boolean(it))
      .filter((it) => !(moveValidationMode === 'AUTO_FIX' && it.locked_in_plan === true))
      .map((it) => ({
        material_id: String(it.material_id ?? '').trim(),
        from_machine: String(it.machine_code ?? '').trim(),
        from_date: String(it.plan_date ?? '').trim(),
        weight_t: Number(it.weight_t ?? 0),
      }))
      .filter((it) => it.material_id && it.from_machine && it.from_date && Number.isFinite(it.weight_t) && it.weight_t > 0);

    if (movable.length === 0) {
      message.warning('所选物料在当前版本中不可移动（可能均为冻结或不在排程）');
      return;
    }

    const totalWeight = movable.reduce((sum, it) => sum + it.weight_t, 0);
    const deltaBase = new Map<string, number>();
    movable.forEach((it) => {
      const fromKey = `${it.from_machine}__${it.from_date}`;
      deltaBase.set(fromKey, (deltaBase.get(fromKey) ?? 0) - it.weight_t);
    });

    const focus =
      moveTargetDate && moveTargetDate.isValid() ? moveTargetDate.startOf('day') : dayjs().startOf('day');
    const rangeStart = workbenchDateRange[0].startOf('day');
    const rangeEnd = workbenchDateRange[1].startOf('day');
    const spanDays = rangeEnd.diff(rangeStart, 'day');
    const candidates: string[] = [];

    // 默认最多评估 31 天（围绕焦点日期）
    const radius = 15;
    if (spanDays <= radius * 2) {
      for (let i = 0; i <= spanDays; i += 1) {
        candidates.push(rangeStart.add(i, 'day').format('YYYY-MM-DD'));
      }
    } else {
      for (let offset = -radius; offset <= radius; offset += 1) {
        const d = focus.add(offset, 'day');
        if (d.isBefore(rangeStart) || d.isAfter(rangeEnd)) continue;
        candidates.push(d.format('YYYY-MM-DD'));
      }
    }

    if (candidates.length === 0) {
      message.warning('当前日期范围过窄，无法推荐（可先调整范围）');
      return;
    }

    const affectedMachines = Array.from(new Set<string>([targetMachine, ...movable.map((it) => it.from_machine)])).sort();

    const originDates = movable.map((it) => it.from_date).filter(Boolean).sort();
    const candidateDates = [...candidates].sort();
    const dateFrom = [originDates[0], candidateDates[0]].filter(Boolean).sort()[0] || candidateDates[0];
    const dateTo =
      [originDates[originDates.length - 1], candidateDates[candidateDates.length - 1]]
        .filter(Boolean)
        .sort()
        .slice(-1)[0] || candidateDates[candidateDates.length - 1];

    setMoveRecommendLoading(true);
    try {
      const pools = await capacityApi.getCapacityPools(affectedMachines, dateFrom, dateTo, activeVersionId);
      const poolMap = new Map<string, { target: number | null; limit: number | null }>();
      pools.forEach((p: IpcCapacityPool) => {
        const machine = String(p.machine_code ?? '').trim();
        const date = String(p.plan_date ?? '').trim();
        if (!machine || !date) return;
        const target = Number(p.target_capacity_t ?? 0);
        const limit = Number(p.limit_capacity_t ?? 0);
        poolMap.set(`${machine}__${date}`, {
          target: Number.isFinite(target) && target > 0 ? target : null,
          limit: Number.isFinite(limit) && limit > 0 ? limit : null,
        });
      });

      const scored = candidates
        .map((date) => {
          const deltaMap = new Map<string, number>(deltaBase);
          const toKey = `${targetMachine}__${date}`;
          deltaMap.set(toKey, (deltaMap.get(toKey) ?? 0) + totalWeight);

          // 过滤掉无变化的 key
          const keys = Array.from(deltaMap.entries()).filter(([, d]) => Number.isFinite(d) && Math.abs(d) > 1e-9);
          if (keys.length === 0) return null;

          let overLimitCount = 0;
          let unknownCount = 0;
          let totalOverT = 0;
          let maxUtilPct = 0;

          keys.forEach(([key, delta]) => {
            const before = tonnageMap.get(key) ?? 0;
            const after = before + delta;
            const cap = poolMap.get(key);
            const limit = cap?.limit ?? cap?.target ?? null;
            if (limit == null || limit <= 0) {
              unknownCount += 1;
              return;
            }
            const pct = (after / limit) * 100;
            if (pct > maxUtilPct) maxUtilPct = pct;
            if (after > limit + 1e-9) {
              overLimitCount += 1;
              totalOverT += after - limit;
            }
          });

          const distance = Math.abs(dayjs(date).diff(focus, 'day'));
          return {
            date,
            overLimitCount,
            unknownCount,
            totalOverT,
            maxUtilPct,
            distance,
          };
        })
        .filter(Boolean) as Array<{
        date: string;
        overLimitCount: number;
        unknownCount: number;
        totalOverT: number;
        maxUtilPct: number;
        distance: number;
      }>;

      if (scored.length === 0) {
        message.warning('暂无可推荐的位置（可能全为无变化/未知容量）');
        return;
      }

      const strategy = String(defaultStrategy || 'balanced');
      scored.sort((a, b) => {
        if (a.overLimitCount !== b.overLimitCount) return a.overLimitCount - b.overLimitCount;
        if (a.unknownCount !== b.unknownCount) return a.unknownCount - b.unknownCount;

        // 策略差异（尽量贴合“当前方案策略”的偏好）
        if (strategy === 'capacity_first') {
          if (a.maxUtilPct !== b.maxUtilPct) return a.maxUtilPct - b.maxUtilPct;
          if (a.totalOverT !== b.totalOverT) return a.totalOverT - b.totalOverT;
          if (a.distance !== b.distance) return a.distance - b.distance;
        } else {
          if (a.totalOverT !== b.totalOverT) return a.totalOverT - b.totalOverT;
          if (a.distance !== b.distance) return a.distance - b.distance;
          if (a.maxUtilPct !== b.maxUtilPct) return a.maxUtilPct - b.maxUtilPct;
        }

        if (strategy === 'urgent_first') return a.date.localeCompare(b.date); // 越早越好
        if (strategy === 'cold_stock_first') return b.date.localeCompare(a.date); // 越晚越好
        return a.date.localeCompare(b.date);
      });

      const best = scored[0];
      setMoveTargetDate(dayjs(best.date));
      setMoveRecommendSummary({
        machine: targetMachine,
        date: best.date,
        overLimitCount: best.overLimitCount,
        unknownCount: best.unknownCount,
        totalOverT: best.totalOverT,
        maxUtilPct: best.maxUtilPct,
      });

      message.success(`推荐位置：${targetMachine} / ${best.date}（策略：${strategyLabel}）`);
    } catch (error: unknown) {
      console.error('推荐位置失败:', error);
      message.error(`推荐位置失败: ${getErrorMessage(error)}`);
    } finally {
      setMoveRecommendLoading(false);
    }
  }, [
    activeVersionId,
    defaultStrategy,
    moveTargetDate,
    moveTargetMachine,
    moveValidationMode,
    planItems,
    selectedMaterialIds,
    strategyLabel,
    workbenchDateRange,
  ]);

  useEffect(() => {
    if (!moveModalOpen) return;
    if (!autoRecommendOnOpen) return;
    setAutoRecommendOnOpen(false);
    recommendMoveTarget();
  }, [autoRecommendOnOpen, moveModalOpen, recommendMoveTarget]);

  const openMoveModal = useCallback(() => {
    if (selectedMaterialIds.length === 0) {
      message.warning('请先选择物料');
      return;
    }

    const fallbackMachine = poolMachineCode || machineOptions[0] || null;
    const focusDate = deepLinkDate ? dayjs(deepLinkDate) : dayjs();
    setMoveTargetMachine(fallbackMachine);
    setMoveTargetDate(focusDate.isValid() ? focusDate : dayjs());
    setMoveSeqMode('APPEND');
    setMoveStartSeq(1);
    setMoveValidationMode('AUTO_FIX');
    setMoveReason(DEFAULT_MOVE_REASON);
    setMoveRecommendSummary(null);
    setMoveModalOpen(true);
  }, [deepLinkDate, machineOptions, poolMachineCode, selectedMaterialIds.length]);

  const openMoveModalAt = useCallback(
    (targetMachine: string, targetDate: string) => {
      if (selectedMaterialIds.length === 0) {
        message.warning('请先选择物料');
        return;
      }

      const machine = String(targetMachine || '').trim() || poolMachineCode || machineOptions[0] || null;
      const date = dayjs(targetDate);

      setMoveTargetMachine(machine);
      setMoveTargetDate(date.isValid() ? date : dayjs());
      setMoveSeqMode('APPEND');
      setMoveStartSeq(1);
      setMoveValidationMode('AUTO_FIX');
      setMoveReason(DEFAULT_MOVE_REASON);
      setMoveRecommendSummary(null);
      setMoveModalOpen(true);
    },
    [machineOptions, poolMachineCode, selectedMaterialIds.length]
  );

  const openMoveModalWithRecommend = useCallback(() => {
    if (selectedMaterialIds.length === 0) {
      message.warning('请先选择物料');
      return;
    }
    openMoveModal();
    setAutoRecommendOnOpen(true);
  }, [openMoveModal, selectedMaterialIds.length]);

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
    setSelectedMaterialIds,
  ]);

  return {
    moveModalOpen,
    setMoveModalOpen,
    moveTargetMachine,
    setMoveTargetMachine,
    moveTargetDate,
    setMoveTargetDate,
    moveSeqMode,
    setMoveSeqMode,
    moveStartSeq,
    setMoveStartSeq,
    moveValidationMode,
    setMoveValidationMode,
    moveSubmitting,
    moveReason,
    setMoveReason,
    moveRecommendLoading,
    moveRecommendSummary,
    strategyLabel,
    selectedPlanItemStats,
    moveImpactPreview,
    recommendMoveTarget,
    openMoveModal,
    openMoveModalAt,
    openMoveModalWithRecommend,
    submitMove,
  };
}
