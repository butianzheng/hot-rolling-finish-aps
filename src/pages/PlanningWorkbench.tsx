import React, { useMemo, useState } from 'react';
import {
  Alert,
  Button,
  Card,
  DatePicker,
  Divider,
  Dropdown,
  Input,
  InputNumber,
  Modal,
  Segmented,
  Select,
  Space,
  Table,
  Tag,
  Typography,
  message,
} from 'antd';
import { DownOutlined, InfoCircleOutlined, ReloadOutlined, SettingOutlined } from '@ant-design/icons';
import { useNavigate, useSearchParams } from 'react-router-dom';
import dayjs from 'dayjs';
import { useQuery } from '@tanstack/react-query';
import ErrorBoundary from '../components/ErrorBoundary';
import PageSkeleton from '../components/PageSkeleton';
import NoActiveVersionGuide from '../components/NoActiveVersionGuide';
import { capacityApi, materialApi, planApi } from '../api/tauri';
import {
  useActiveVersionId,
  useAdminOverrideMode,
  useCurrentUser,
  useGlobalActions,
  useGlobalStore,
  useUserPreferences,
  type WorkbenchViewMode,
} from '../stores/use-global-store';
import { formatDate } from '../utils/formatters';
import { normalizeSchedState } from '../utils/schedState';
import type { PlanItemStatusFilter } from '../utils/planItemStatus';
import MaterialPool, { type MaterialPoolMaterial, type MaterialPoolSelection } from '../components/workbench/MaterialPool';
import ScheduleCardView from '../components/workbench/ScheduleCardView';
import ScheduleGanttView from '../components/workbench/ScheduleGanttView';
import BatchOperationToolbar from '../components/workbench/BatchOperationToolbar';
import OneClickOptimizeMenu from '../components/workbench/OneClickOptimizeMenu';
import { CapacityTimelineContainer } from '../components/CapacityTimelineContainer';
import { MaterialInspector } from '../components/MaterialInspector';
import { RedLineGuard, createFrozenZoneViolation, createMaturityViolation } from '../components/guards/RedLineGuard';
import type { RedLineViolation } from '../components/guards/RedLineGuard';
import DecisionFlowGuide from '../components/flow/DecisionFlowGuide';

const PlanItemVisualization = React.lazy(() => import('../components/PlanItemVisualization'));
type MoveSeqMode = 'APPEND' | 'START_SEQ';
type MoveValidationMode = 'AUTO_FIX' | 'STRICT';
type MoveItemResultRow = {
  material_id: string;
  success: boolean;
  from_machine: string | null;
  from_date: string | null;
  to_machine: string;
  to_date: string;
  error: string | null;
  violation_type: string | null;
};
type MoveImpactRow = {
  machine_code: string;
  date: string;
  before_t: number;
  delta_t: number;
  after_t: number;
  target_capacity_t: number | null;
  limit_capacity_t: number | null;
};
type ConditionLockFilter = 'ALL' | 'LOCKED' | 'UNLOCKED';

const DEFAULT_MOVE_REASON = '手工微调';
const QUICK_MOVE_REASONS: Array<{ label: string; value: string }> = [
  { label: '手工微调', value: '手工微调' },
  { label: '处理产能超限', value: '处理产能超限' },
  { label: '满足紧急订单', value: '满足紧急订单' },
  { label: '轧辊/工艺约束调整', value: '轧辊/工艺约束调整' },
  { label: '冷坯消化', value: '冷坯消化' },
];

const PlanningWorkbench: React.FC = () => {
  const navigate = useNavigate();
  const [searchParams] = useSearchParams();
  const activeVersionId = useActiveVersionId();
  const currentUser = useCurrentUser();
  const adminOverrideMode = useAdminOverrideMode();
  const workbenchViewMode = useGlobalStore((state) => state.workbenchViewMode);
  const workbenchFilters = useGlobalStore((state) => state.workbenchFilters);
  const preferences = useUserPreferences();
  const { setRecalculating } = useGlobalActions();
  const [refreshSignal, setRefreshSignal] = useState(0);

  const { setWorkbenchViewMode, setWorkbenchFilters } = useGlobalActions();

  const [poolSelection, setPoolSelection] = useState<MaterialPoolSelection>(() => ({
    machineCode: workbenchFilters.machineCode,
    schedState: null,
  }));
  const [selectedMaterialIds, setSelectedMaterialIds] = useState<string[]>([]);
  const [scheduleStatusFilter, setScheduleStatusFilter] = useState<PlanItemStatusFilter>('ALL');
  const [scheduleFocus, setScheduleFocus] = useState<{
    machine?: string;
    date: string;
    source?: string;
  } | null>(null);
  const [matrixFocusRequest, setMatrixFocusRequest] = useState<{
    machine?: string;
    date: string;
    nonce: number;
  } | null>(null);

  type WorkbenchDateRangeMode = 'AUTO' | 'PINNED' | 'MANUAL';
  const [dateRangeMode, setDateRangeMode] = useState<WorkbenchDateRangeMode>(() => {
    const d = searchParams.get('date');
    const focusDate = d ? dayjs(d) : null;
    return focusDate && focusDate.isValid() ? 'PINNED' : 'AUTO';
  });
  const [workbenchDateRange, setWorkbenchDateRange] = useState<[dayjs.Dayjs, dayjs.Dayjs]>(() => {
    const d = searchParams.get('date');
    const focusDate = d ? dayjs(d) : null;
    if (focusDate && focusDate.isValid()) {
      return [focusDate.subtract(3, 'day'), focusDate.add(3, 'day')];
    }
    return [dayjs().subtract(3, 'day'), dayjs().add(10, 'day')];
  });

  const [inspectorOpen, setInspectorOpen] = useState(false);
  const [inspectedMaterialId, setInspectedMaterialId] = useState<string | null>(null);

  const [conditionalSelectOpen, setConditionalSelectOpen] = useState(false);
  const [conditionMachine, setConditionMachine] = useState<string | null>('all');
  const [conditionSchedState, setConditionSchedState] = useState<string | null>('all');
  const [conditionUrgency, setConditionUrgency] = useState<string | null>('all');
  const [conditionLock, setConditionLock] = useState<ConditionLockFilter>('ALL');
  const [conditionSearch, setConditionSearch] = useState<string>('');

  const [moveModalOpen, setMoveModalOpen] = useState(false);
  const [moveTargetMachine, setMoveTargetMachine] = useState<string | null>(null);
  const [moveTargetDate, setMoveTargetDate] = useState<dayjs.Dayjs | null>(dayjs());
  const [moveSeqMode, setMoveSeqMode] = useState<MoveSeqMode>('APPEND');
  const [moveStartSeq, setMoveStartSeq] = useState<number>(1);
  const [moveValidationMode, setMoveValidationMode] = useState<MoveValidationMode>('AUTO_FIX');
  const [moveSubmitting, setMoveSubmitting] = useState(false);
  const [moveReason, setMoveReason] = useState<string>('');
  const [moveRecommendLoading, setMoveRecommendLoading] = useState(false);
  const [moveRecommendSummary, setMoveRecommendSummary] = useState<{
    machine: string;
    date: string;
    overLimitCount: number;
    unknownCount: number;
    totalOverT: number;
    maxUtilPct: number;
  } | null>(null);
  const [autoRecommendOnOpen, setAutoRecommendOnOpen] = useState(false);

  // 深链接：从“策略对比/变更明细”等页面跳转到工作台时，可携带 material_id 自动打开详情侧栏
  React.useEffect(() => {
    const materialId = searchParams.get('material_id');
    const id = String(materialId || '').trim();
    if (!id) return;
    setInspectedMaterialId(id);
    setInspectorOpen(true);
  }, [searchParams]);

  // 与全局筛选同步：允许其他页面（如风险下钻）回填机组筛选
  React.useEffect(() => {
    const nextMachine = workbenchFilters.machineCode ?? null;
    setPoolSelection((prev) => {
      if (prev.machineCode === nextMachine) return prev;
      return { machineCode: nextMachine, schedState: null };
    });
  }, [workbenchFilters.machineCode]);

  // 深链接：从风险概览跳转时，处理上下文参数（第三阶段）
  const [deepLinkContext, setDeepLinkContext] = React.useState<{
    machine?: string;
    date?: string;
    urgency?: string;
    context?: string;
    focus?: string;
    openCell?: boolean;
  } | null>(null);

  React.useEffect(() => {
    const machine = searchParams.get('machine');
    const date = searchParams.get('date');
    const urgency = searchParams.get('urgency');
    const context = searchParams.get('context');
    const focus = searchParams.get('focus');
    const openCell = searchParams.get('openCell');

    // 如果有深链接参数，保存到状态并应用
    if (machine || date || urgency || context || focus || openCell) {
      const openCellFlag = openCell === '1' || openCell === 'true';
      setDeepLinkContext({
        machine: machine || undefined,
        date: date || undefined,
        urgency: urgency || undefined,
        context: context || undefined,
        focus: focus || undefined,
        openCell: openCellFlag,
      });

      // 应用机组筛选
      if (machine) {
        setPoolSelection((prev) => {
          if (prev.machineCode === machine) return prev;
          return { machineCode: machine, schedState: null };
        });
        setWorkbenchFilters({ machineCode: machine });
      }

      // 应用紧急度筛选（扩展功能）
      if (urgency) {
        setWorkbenchFilters({ urgencyLevel: urgency });
      }

      // 深链接日期：默认聚焦前后各 3 天，并锁定范围，避免被自动范围覆盖
      if (date) {
        const focusDate = dayjs(date);
        if (focusDate.isValid()) {
          setWorkbenchDateRange([focusDate.subtract(3, 'day'), focusDate.add(3, 'day')]);
          setDateRangeMode('PINNED');
        }
      }

      // 深链接指定甘特图定位（风险日/瓶颈点等）
      if (focus === 'gantt' || openCellFlag) {
        setWorkbenchViewMode('GANTT');
      }

      // 显示来源提示
      const contextLabel =
        context === 'risk'
          ? '风险日'
          : context === 'bottleneck'
          ? '瓶颈点'
          : context === 'capacityOpportunity'
          ? '容量优化机会'
          : context === 'orders'
          ? '订单失败'
          : context === 'coldStock'
          ? '冷坨高压力'
          : context === 'roll'
          ? '换辊警报'
          : '';

      if (contextLabel) {
        const filterHints = [];
        if (machine) filterHints.push(`机组: ${machine}`);
        if (urgency) filterHints.push(`紧急度: ${urgency}`);
        if (date) filterHints.push(`日期: ${date}`);

        const filterInfo = filterHints.length > 0 ? `（${filterHints.join('、')}）` : '';
        message.info(`已从「${contextLabel}」跳转，自动应用相关筛选条件${filterInfo}`);
      }
    }
  }, [searchParams, setWorkbenchFilters, setWorkbenchViewMode]);

  const deepLinkContextLabel = useMemo(() => {
    const ctx = String(deepLinkContext?.context || '').trim();
    if (ctx === 'risk') return '风险日';
    if (ctx === 'bottleneck') return '瓶颈点';
    if (ctx === 'capacityOpportunity') return '容量优化机会';
    if (ctx === 'orders') return '订单失败';
    if (ctx === 'coldStock') return '冷坨高压力';
    if (ctx === 'roll') return '换辊警报';
    return '';
  }, [deepLinkContext?.context]);

  const materialsQuery = useQuery({
    queryKey: ['materials'],
    queryFn: async () => {
      const res = await materialApi.listMaterials({ limit: 1000, offset: 0 });
      return Array.isArray(res) ? res : [];
    },
    staleTime: 30 * 1000,
  });

  const materials = useMemo<MaterialPoolMaterial[]>(() => {
    const raw = Array.isArray(materialsQuery.data) ? materialsQuery.data : [];
    return raw.map((m: any) => ({
      material_id: String(m?.material_id ?? ''),
      machine_code: String(m?.machine_code ?? ''),
      weight_t: Number(m?.weight_t ?? 0),
      steel_mark: String(m?.steel_mark ?? ''),
      sched_state: String(m?.sched_state ?? ''),
      urgent_level: String(m?.urgent_level ?? ''),
      lock_flag: !!m?.lock_flag,
      manual_urgent_flag: !!m?.manual_urgent_flag,
      is_frozen: m?.is_frozen === true,
      is_mature: m?.is_mature === true ? true : m?.is_mature === false ? false : undefined,
      temp_issue: m?.temp_issue === true,
    }));
  }, [materialsQuery.data]);

  const materialDetailQuery = useQuery({
    queryKey: ['materialDetail', inspectedMaterialId],
    enabled: !!inspectedMaterialId,
    queryFn: async () => {
      if (!inspectedMaterialId) return null;
      return materialApi.getMaterialDetail(inspectedMaterialId);
    },
    staleTime: 30 * 1000,
  });

  const inspectedMaterial = useMemo(() => {
    if (!inspectedMaterialId) return null;
    const fromList = materials.find((m) => m.material_id === inspectedMaterialId) || null;
    const fromDetail = materialDetailQuery.data ? (materialDetailQuery.data as any) : null;
    const merged = { ...(fromList || {}), ...(fromDetail || {}) };

    // 保持与 MaterialInspector 的字段命名一致（snake_case -> camelCase 这里不做转换，只做兜底）
    return {
      material_id: String(merged?.material_id ?? inspectedMaterialId),
      machine_code: String(merged?.machine_code ?? ''),
      weight_t: Number(merged?.weight_t ?? 0),
      steel_mark: String(merged?.steel_mark ?? ''),
      sched_state: String(merged?.sched_state ?? ''),
      urgent_level: String(merged?.urgent_level ?? ''),
      lock_flag: !!merged?.lock_flag,
      manual_urgent_flag: !!merged?.manual_urgent_flag,
      is_frozen: merged?.is_frozen === true,
      is_mature: merged?.is_mature === true ? true : merged?.is_mature === false ? false : undefined,
      temp_issue: merged?.temp_issue === true,
      urgent_reason: merged?.urgent_reason ? String(merged.urgent_reason) : undefined,
      eligibility_reason: merged?.eligibility_reason ? String(merged.eligibility_reason) : undefined,
      priority_reason: merged?.priority_reason ? String(merged.priority_reason) : undefined,
    };
  }, [inspectedMaterialId, materialDetailQuery.data, materials]);

  const planItemsQuery = useQuery({
    queryKey: ['planItems', activeVersionId],
    enabled: !!activeVersionId,
    queryFn: async () => {
      if (!activeVersionId) return [];
      const res = await planApi.listPlanItems(activeVersionId);
      return Array.isArray(res) ? res : [];
    },
    staleTime: 30 * 1000,
  });

  React.useEffect(() => {
    if (!activeVersionId) return;
    if (refreshSignal == null) return;
    planItemsQuery.refetch();
  }, [activeVersionId, refreshSignal, planItemsQuery.refetch]);

  // AUTO 日期范围（基于当前机组的排程数据）
  const autoDateRange = useMemo<[dayjs.Dayjs, dayjs.Dayjs]>(() => {
    const filteredItems = (planItemsQuery.data || []).filter(
      (item: any) => !poolSelection.machineCode ||
                    poolSelection.machineCode === 'all' ||
                    item.machine_code === poolSelection.machineCode
    );

    if (filteredItems.length === 0) {
      // 默认日期范围：今天前 3 天到后 10 天
      return [dayjs().subtract(3, 'day'), dayjs().add(10, 'day')];
    }

    // 提取所有排程日期
    const dates = filteredItems
      .map((item: any) => dayjs(item.plan_date))
      .filter((d: dayjs.Dayjs) => d.isValid());

    if (dates.length === 0) {
      return [dayjs().subtract(3, 'day'), dayjs().add(10, 'day')];
    }

    // 找到最早和最晚的日期
    const sortedDates = dates.sort((a: dayjs.Dayjs, b: dayjs.Dayjs) => a.valueOf() - b.valueOf());
    const minDate = sortedDates[0].subtract(1, 'day'); // 前面留 1 天余量
    const maxDate = sortedDates[sortedDates.length - 1].add(3, 'day'); // 后面留 3 天余量

    return [minDate, maxDate];
  }, [planItemsQuery.data, poolSelection.machineCode]);

  React.useEffect(() => {
    if (dateRangeMode !== 'AUTO') return;
    setWorkbenchDateRange(autoDateRange);
  }, [autoDateRange, dateRangeMode]);

  const applyWorkbenchDateRange = (next: [dayjs.Dayjs, dayjs.Dayjs]) => {
    if (!next || !next[0] || !next[1]) return;
    let start = next[0].startOf('day');
    let end = next[1].startOf('day');
    if (end.isBefore(start)) {
      const tmp = start;
      start = end;
      end = tmp;
    }
    setWorkbenchDateRange([start, end]);
    const isAuto =
      formatDate(start) === formatDate(autoDateRange[0]) &&
      formatDate(end) === formatDate(autoDateRange[1]);
    setDateRangeMode(isAuto ? 'AUTO' : 'MANUAL');
  };

  const resetWorkbenchDateRangeToAuto = () => {
    setDateRangeMode('AUTO');
    setWorkbenchDateRange(autoDateRange);
  };

  const openInspector = (materialId: string) => {
    setInspectedMaterialId(materialId);
    setInspectorOpen(true);
  };

  const applyWorkbenchMachineCode = (machineCode: string | null) => {
    setPoolSelection((prev) => {
      if (prev.machineCode === machineCode) return prev;
      return { machineCode, schedState: null };
    });
    setWorkbenchFilters({ machineCode });
  };

  const ganttFocusedDate = deepLinkContext?.date || null;
  const ganttAutoOpenCell = useMemo(() => {
    if (!deepLinkContext?.openCell) return null;
    const machine = String(deepLinkContext.machine || poolSelection.machineCode || '').trim();
    const date = String(deepLinkContext.date || '').trim();
    if (!machine || !date) return null;
    return { machine, date };
  }, [deepLinkContext?.date, deepLinkContext?.machine, deepLinkContext?.openCell, poolSelection.machineCode]);

  const [ganttOpenCellRequest, setGanttOpenCellRequest] = useState<{
    machine: string;
    date: string;
    nonce: number;
    source?: string;
  } | null>(null);

  const openGanttCellDetail = (machine: string, date: string, source: string) => {
    const machineCode = String(machine || '').trim();
    const d = dayjs(date);
    if (!machineCode || !d.isValid()) return;
    const dateKey = formatDate(d);
    setWorkbenchViewMode('GANTT');
    setGanttOpenCellRequest({ machine: machineCode, date: dateKey, nonce: Date.now(), source });
    setScheduleFocus({ machine: machineCode, date: dateKey, source });
  };

  const selectedMaterials = useMemo(() => {
    const set = new Set(selectedMaterialIds);
    return materials.filter((m) => set.has(m.material_id));
  }, [materials, selectedMaterialIds]);

  const selectedTotalWeight = useMemo(() => {
    return selectedMaterials.reduce((sum, m) => sum + Number(m.weight_t || 0), 0);
  }, [selectedMaterials]);

  const machineOptions = useMemo(() => {
    const codes = new Set<string>();
    materials.forEach((m) => {
      const code = String(m.machine_code || '').trim();
      if (code) codes.add(code);
    });
    return Array.from(codes).sort();
  }, [materials]);

  const conditionalMatches = useMemo(() => {
    let list = materials;
    if (conditionMachine && conditionMachine !== 'all') {
      list = list.filter((m) => String(m.machine_code || '') === conditionMachine);
    }
    if (conditionSchedState && conditionSchedState !== 'all') {
      const want = normalizeSchedState(conditionSchedState);
      list = list.filter((m) => normalizeSchedState(m.sched_state) === want);
    }
    if (conditionUrgency && conditionUrgency !== 'all') {
      list = list.filter((m) => String(m.urgent_level || '') === conditionUrgency);
    }
    if (conditionLock === 'LOCKED') {
      list = list.filter((m) => !!m.lock_flag);
    } else if (conditionLock === 'UNLOCKED') {
      list = list.filter((m) => !m.lock_flag);
    }
    const q = conditionSearch.trim().toLowerCase();
    if (q) {
      list = list.filter((m) => {
        const id = String(m.material_id || '').toLowerCase();
        const steel = String(m.steel_mark || '').toLowerCase();
        return id.includes(q) || steel.includes(q);
      });
    }
    return [...list].sort((a, b) => String(a.material_id || '').localeCompare(String(b.material_id || '')));
  }, [conditionLock, conditionMachine, conditionSchedState, conditionSearch, conditionUrgency, materials]);

  const conditionalSummary = useMemo(() => {
    const count = conditionalMatches.length;
    const weight = conditionalMatches.reduce((sum, m) => sum + Number(m.weight_t || 0), 0);
    return { count, weight };
  }, [conditionalMatches]);

  const planItemById = useMemo(() => {
    const map = new Map<string, any>();
    const raw = Array.isArray(planItemsQuery.data) ? planItemsQuery.data : [];
    raw.forEach((it: any) => {
      const id = String(it?.material_id ?? '').trim();
      if (id) map.set(id, it);
    });
    return map;
  }, [planItemsQuery.data]);

  const selectedPlanItemStats = useMemo(() => {
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
    const raw = Array.isArray(planItemsQuery.data) ? planItemsQuery.data : [];

    const tonnageMap = new Map<string, number>();
    raw.forEach((it: any) => {
      const machine = String(it?.machine_code ?? '').trim();
      const date = String(it?.plan_date ?? '').trim();
      if (!machine || !date) return;
      const weight = Number(it?.weight_t ?? 0);
      if (!Number.isFinite(weight) || weight <= 0) return;
      const key = `${machine}__${date}`;
      tonnageMap.set(key, (tonnageMap.get(key) ?? 0) + weight);
    });

    const byId = new Map<string, any>();
    raw.forEach((it: any) => {
      const id = String(it?.material_id ?? '').trim();
      if (id) byId.set(id, it);
    });

    const deltaMap = new Map<string, number>();
    selectedMaterialIds.forEach((id) => {
      const it = byId.get(id);
      if (!it) return;
      const fromMachine = String(it?.machine_code ?? '').trim();
      const fromDate = String(it?.plan_date ?? '').trim();
      if (!fromMachine || !fromDate) return;
      const weight = Number(it?.weight_t ?? 0);
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
      .sort((a, b) => (a.date === b.date ? a.machine_code.localeCompare(b.machine_code) : a.date.localeCompare(b.date)));

    return { targetDate, affectedMachines, dateFrom, dateTo, rows };
  }, [moveModalOpen, moveTargetMachine, moveTargetDate, planItemsQuery.data, selectedMaterialIds]);

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
      const res = await capacityApi.getCapacityPools(
        moveImpactBase.affectedMachines,
        moveImpactBase.dateFrom,
        moveImpactBase.dateTo,
        activeVersionId
      );
      return Array.isArray(res) ? res : [];
    },
    staleTime: 30 * 1000,
  });

  const moveImpactPreview = useMemo(() => {
    if (!moveImpactBase) return null;
    const pools = Array.isArray(moveImpactCapacityQuery.data) ? moveImpactCapacityQuery.data : [];
    const poolMap = new Map<string, { target: number | null; limit: number | null }>();
    pools.forEach((p: any) => {
      const machine = String(p?.machine_code ?? '').trim();
      const date = String(p?.plan_date ?? '').trim();
      if (!machine || !date) return;
      const target = Number(p?.target_capacity_t ?? 0);
      const limit = Number(p?.limit_capacity_t ?? 0);
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

  const strategyLabel = useMemo(() => {
    const v = String(preferences.defaultStrategy || 'balanced');
    if (v === 'urgent_first') return '紧急优先';
    if (v === 'capacity_first') return '产能优先';
    if (v === 'cold_stock_first') return '冷坯消化';
    if (v === 'manual') return '手动调整';
    return '均衡方案';
  }, [preferences.defaultStrategy]);

  const recommendMoveTarget = React.useCallback(async () => {
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
    let planItemsRaw = Array.isArray(planItemsQuery.data) ? planItemsQuery.data : [];
    if (planItemsRaw.length === 0) {
      const fetched = await planApi.listPlanItems(activeVersionId);
      planItemsRaw = Array.isArray(fetched) ? fetched : [];
    }

    const byId = new Map<string, any>();
    const tonnageMap = new Map<string, number>();
    planItemsRaw.forEach((it: any) => {
      const id = String(it?.material_id ?? '').trim();
      if (id) byId.set(id, it);
      const machine = String(it?.machine_code ?? '').trim();
      const date = String(it?.plan_date ?? '').trim();
      if (!machine || !date) return;
      const weight = Number(it?.weight_t ?? 0);
      if (!Number.isFinite(weight) || weight <= 0) return;
      const key = `${machine}__${date}`;
      tonnageMap.set(key, (tonnageMap.get(key) ?? 0) + weight);
    });

    const movable = selectedMaterialIds
      .map((id) => byId.get(id))
      .filter(Boolean)
      .filter((it: any) => !(moveValidationMode === 'AUTO_FIX' && it?.locked_in_plan === true))
      .map((it: any) => ({
        material_id: String(it?.material_id ?? '').trim(),
        from_machine: String(it?.machine_code ?? '').trim(),
        from_date: String(it?.plan_date ?? '').trim(),
        weight_t: Number(it?.weight_t ?? 0),
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

    const focus = moveTargetDate && moveTargetDate.isValid() ? moveTargetDate.startOf('day') : dayjs().startOf('day');
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

    const affectedMachines = Array.from(
      new Set<string>([targetMachine, ...movable.map((it) => it.from_machine)])
    ).sort();

    const originDates = movable.map((it) => it.from_date).filter(Boolean).sort();
    const candidateDates = [...candidates].sort();
    const dateFrom = [originDates[0], candidateDates[0]].filter(Boolean).sort()[0] || candidateDates[0];
    const dateTo =
      [originDates[originDates.length - 1], candidateDates[candidateDates.length - 1]].filter(Boolean).sort().slice(-1)[0] ||
      candidateDates[candidateDates.length - 1];

    setMoveRecommendLoading(true);
    try {
      const pools = await capacityApi.getCapacityPools(affectedMachines, dateFrom, dateTo, activeVersionId);
      const poolMap = new Map<string, { target: number | null; limit: number | null }>();
      (Array.isArray(pools) ? pools : []).forEach((p: any) => {
        const machine = String(p?.machine_code ?? '').trim();
        const date = String(p?.plan_date ?? '').trim();
        if (!machine || !date) return;
        const target = Number(p?.target_capacity_t ?? 0);
        const limit = Number(p?.limit_capacity_t ?? 0);
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

      const strategy = String(preferences.defaultStrategy || 'balanced');
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
    } catch (error: any) {
      console.error('推荐位置失败:', error);
      message.error(`推荐位置失败: ${error?.message || error}`);
    } finally {
      setMoveRecommendLoading(false);
    }
  }, [
    activeVersionId,
    moveTargetMachine,
    moveTargetDate,
    moveValidationMode,
    planItemsQuery.data,
    preferences.defaultStrategy,
    strategyLabel,
    selectedMaterialIds,
    workbenchDateRange,
  ]);

  React.useEffect(() => {
    if (!moveModalOpen) return;
    if (!autoRecommendOnOpen) return;
    setAutoRecommendOnOpen(false);
    recommendMoveTarget();
  }, [autoRecommendOnOpen, moveModalOpen, recommendMoveTarget]);

  const checkRedLineViolations = (
    material: MaterialPoolMaterial,
    operation: 'lock' | 'unlock' | 'urgent_on' | 'urgent_off'
  ): RedLineViolation[] => {
    if (adminOverrideMode) return [];
    const violations: RedLineViolation[] = [];

    if (
      material.is_frozen === true &&
      (operation === 'lock' || operation === 'unlock' || operation === 'urgent_on' || operation === 'urgent_off')
    ) {
      violations.push(
        createFrozenZoneViolation([material.material_id], '该材料位于冻结区，不允许修改状态')
      );
    }

    if (material.is_mature === false && operation === 'urgent_on') {
      violations.push(createMaturityViolation([material.material_id], 1));
    }

    return violations;
  };

  const showRedLineModal = (violations: RedLineViolation[]) => {
    Modal.error({
      title: '工业红线保护',
      width: 700,
      content: (
        <Space direction="vertical" style={{ width: '100%' }} size={16}>
          <div style={{ maxHeight: 420, overflow: 'auto' }}>
            <RedLineGuard violations={violations} mode="detailed" />
          </div>
          {!adminOverrideMode && (
            <div
              style={{
                padding: 12,
                background: '#fff7e6',
                border: '1px solid #ffd591',
                borderRadius: 4,
              }}
            >
              <Space>
                <InfoCircleOutlined style={{ color: '#faad14' }} />
                <div>
                  <div style={{ fontWeight: 600, color: '#faad14' }}>提示</div>
                  <div style={{ fontSize: 12, color: '#8c8c8c', marginTop: 4 }}>
                    如需覆盖此保护，请启用“管理员覆盖模式”。
                  </div>
                </div>
              </Space>
            </div>
          )}
        </Space>
      ),
    });
  };

  const runMaterialOperation = (materialIds: string[], type: 'lock' | 'unlock' | 'urgent_on' | 'urgent_off') => {
    if (materialIds.length === 0) {
      message.warning('请先选择物料');
      return;
    }

    if (!adminOverrideMode) {
      const set = new Set(materialIds);
      const targets = materials.filter((m) => set.has(m.material_id));
      const violations: RedLineViolation[] = [];
      targets.forEach((m) => violations.push(...checkRedLineViolations(m, type)));
      if (violations.length > 0) {
        showRedLineModal(violations);
        return;
      }
    }

    let reason = '';
    Modal.confirm({
      title:
        type === 'lock'
          ? `锁定物料（${materialIds.length}）`
          : type === 'unlock'
          ? `解锁物料（${materialIds.length}）`
          : type === 'urgent_on'
          ? `设为紧急（${materialIds.length}）`
          : `取消紧急（${materialIds.length}）`,
      width: 520,
      content: (
        <Space direction="vertical" style={{ width: '100%' }} size={10}>
          <Typography.Text type="secondary">请输入操作原因（必填）</Typography.Text>
          <Input.TextArea rows={3} autoSize={{ minRows: 3, maxRows: 6 }} onChange={(e) => (reason = e.target.value)} />
        </Space>
      ),
      onOk: async () => {
        const trimmed = reason.trim();
        if (!trimmed) {
          message.warning('请输入操作原因');
          return Promise.reject(new Error('reason_required'));
        }

        const operator = currentUser || 'admin';
        const lockMode = adminOverrideMode ? 'AutoFix' : undefined;

        if (type === 'lock') {
          await materialApi.batchLockMaterials(materialIds, true, operator, trimmed, lockMode);
          message.success('锁定成功');
        } else if (type === 'unlock') {
          await materialApi.batchLockMaterials(materialIds, false, operator, trimmed, lockMode);
          message.success('解锁成功');
        } else if (type === 'urgent_on') {
          await materialApi.batchSetUrgent(materialIds, true, operator, trimmed);
          message.success('已设置紧急标志');
        } else {
          await materialApi.batchSetUrgent(materialIds, false, operator, trimmed);
          message.success('已取消紧急标志');
        }

        setRefreshSignal((v) => v + 1);
        materialsQuery.refetch();
        planItemsQuery.refetch();
      },
    });
  };

  const runForceReleaseOperation = (materialIds: string[]) => {
    if (materialIds.length === 0) {
      message.warning('请先选择物料');
      return;
    }

    const set = new Set(materialIds);
    const targets = materials.filter((m) => set.has(m.material_id));
    const totalWeight = targets.reduce((sum, m) => sum + Number(m.weight_t || 0), 0);
    const immatureCount = targets.filter((m) => m.is_mature === false).length;
    const unknownMaturityCount = targets.filter((m) => m.is_mature == null).length;
    const frozenCount = targets.filter((m) => m.is_frozen === true).length;

    let reason = '';
    let mode: 'AutoFix' | 'Strict' = 'AutoFix';

    Modal.confirm({
      title: `强制放行（${materialIds.length}）`,
      width: 560,
      content: (
        <Space direction="vertical" style={{ width: '100%' }} size={10}>
          <Alert
            type="info"
            showIcon
            message="说明"
            description="强制放行会将材料状态标记为“强制放行”，并写入操作日志；通常用于人工决策放行未适温材料。"
          />

          <Space wrap>
            <Tag color="blue">可识别 {targets.length}/{materialIds.length}</Tag>
            <Tag color="geekblue">总重 {totalWeight.toFixed(2)}t</Tag>
            {frozenCount > 0 ? <Tag color="purple">冻结区 {frozenCount}</Tag> : null}
            {immatureCount > 0 ? <Tag color="orange">未适温 {immatureCount}</Tag> : null}
            {unknownMaturityCount > 0 ? <Tag>适温未知 {unknownMaturityCount}</Tag> : null}
          </Space>

          {immatureCount > 0 ? (
            <Alert
              type="warning"
              showIcon
              message={`检测到 ${immatureCount} 个未适温材料`}
              description="AUTO_FIX：允许放行并记录警告；STRICT：将阻止操作。"
            />
          ) : null}

          <Space wrap>
            <span>校验模式</span>
            <Select
              defaultValue="AutoFix"
              style={{ width: 220 }}
              onChange={(v) => {
                mode = v as 'AutoFix' | 'Strict';
              }}
              options={[
                { value: 'AutoFix', label: 'AUTO_FIX（允许未适温）' },
                { value: 'Strict', label: 'STRICT（未适温则失败）' },
              ]}
            />
          </Space>

          <Typography.Text type="secondary" style={{ fontSize: 12 }}>
            请输入强制放行原因（必填）
          </Typography.Text>
          <Input.TextArea
            rows={3}
            autoSize={{ minRows: 3, maxRows: 6 }}
            onChange={(e) => {
              reason = e.target.value;
            }}
          />
        </Space>
      ),
      onOk: async () => {
        const trimmed = reason.trim();
        if (!trimmed) {
          message.warning('请输入操作原因');
          return Promise.reject(new Error('reason_required'));
        }

        const operator = currentUser || 'admin';
        const res: any = await materialApi.batchForceRelease(materialIds, operator, trimmed, mode);

        message.success(String(res?.message || '强制放行完成'));

        const violations = Array.isArray(res?.details?.violations) ? res.details.violations : [];
        if (violations.length > 0) {
          const rows = violations.map((v: any, idx: number) => ({
            key: `${String(v?.material_id ?? idx)}__${idx}`,
            material_id: String(v?.material_id ?? ''),
            violation_type: String(v?.violation_type ?? ''),
            reason: String(v?.reason ?? ''),
          }));

          Modal.info({
            title: '强制放行警告（未适温材料）',
            width: 820,
            content: (
              <Space direction="vertical" style={{ width: '100%' }} size={12}>
                <Alert
                  type="warning"
                  showIcon
                  message={`本次包含 ${violations.length} 个未适温材料（AUTO_FIX 模式允许）`}
                />
                <Table
                  size="small"
                  pagination={false}
                  dataSource={rows}
                  columns={[
                    { title: '材料', dataIndex: 'material_id', width: 180 },
                    { title: '类型', dataIndex: 'violation_type', width: 140 },
                    { title: '说明', dataIndex: 'reason' },
                  ]}
                  scroll={{ y: 260 }}
                />
              </Space>
            ),
          });
        }

        setSelectedMaterialIds([]);
        setRefreshSignal((v) => v + 1);
        materialsQuery.refetch();
        planItemsQuery.refetch();
      },
    });
  };

  const openMoveModal = () => {
    if (selectedMaterialIds.length === 0) {
      message.warning('请先选择物料');
      return;
    }

    const fallbackMachine = poolSelection.machineCode || machineOptions[0] || null;
    const focusDate = deepLinkContext?.date ? dayjs(deepLinkContext.date) : dayjs();
    setMoveTargetMachine(fallbackMachine);
    setMoveTargetDate(focusDate.isValid() ? focusDate : dayjs());
    setMoveSeqMode('APPEND');
    setMoveStartSeq(1);
    setMoveValidationMode('AUTO_FIX');
    setMoveReason(DEFAULT_MOVE_REASON);
    setMoveRecommendSummary(null);
    setMoveModalOpen(true);
  };

  const openMoveModalAt = (targetMachine: string, targetDate: string) => {
    if (selectedMaterialIds.length === 0) {
      message.warning('请先选择物料');
      return;
    }

    const machine = String(targetMachine || '').trim() || poolSelection.machineCode || machineOptions[0] || null;
    const date = dayjs(targetDate);

    setMoveTargetMachine(machine);
    setMoveTargetDate(date.isValid() ? date : dayjs());
    setMoveSeqMode('APPEND');
    setMoveStartSeq(1);
    setMoveValidationMode('AUTO_FIX');
    setMoveReason(DEFAULT_MOVE_REASON);
    setMoveRecommendSummary(null);
    setMoveModalOpen(true);
  };

  const openMoveModalWithRecommend = () => {
    if (selectedMaterialIds.length === 0) {
      message.warning('请先选择物料');
      return;
    }
    openMoveModal();
    setAutoRecommendOnOpen(true);
  };

  const submitMove = async () => {
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

      let planItemsRaw = Array.isArray(planItemsQuery.data) ? planItemsQuery.data : [];
      if (planItemsRaw.length === 0) {
        // 避免由于 Query 未命中导致误判“未排入”
        const fetched = await planApi.listPlanItems(activeVersionId);
        planItemsRaw = Array.isArray(fetched) ? fetched : [];
      }

      const byId = new Map<string, any>();
      planItemsRaw.forEach((it: any) => {
        const id = String(it?.material_id ?? '').trim();
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
          .filter(
            (it: any) =>
              String(it?.machine_code ?? '') === moveTargetMachine &&
              String(it?.plan_date ?? '') === targetDate
          )
          .reduce((max: number, it: any) => Math.max(max, Number(it?.seq_no ?? 0)), 0);
        startSeq = Math.max(1, maxSeq + 1);
      }

      const moves = ordered.map((id, idx) => ({
        material_id: id,
        to_date: targetDate,
        to_seq: startSeq + idx,
        to_machine: moveTargetMachine,
      }));

      const operator = currentUser || 'admin';
      const res: any = await planApi.moveItems(activeVersionId, moves, moveValidationMode, operator, reason);

      setMoveModalOpen(false);
      setMoveReason('');
      setSelectedMaterialIds([]);
      setRefreshSignal((v) => v + 1);
      materialsQuery.refetch();
      planItemsQuery.refetch();

      const failedCount = Number(res?.failed_count ?? 0);
      if (failedCount > 0) {
        const results: MoveItemResultRow[] = (Array.isArray(res?.results) ? res.results : []).map(
          (r: any) => ({
            material_id: String(r?.material_id ?? ''),
            success: !!r?.success,
            from_machine: r?.from_machine == null ? null : String(r.from_machine),
            from_date: r?.from_date == null ? null : String(r.from_date),
            to_machine: String(r?.to_machine ?? ''),
            to_date: String(r?.to_date ?? ''),
            error: r?.error == null ? null : String(r.error),
            violation_type: r?.violation_type == null ? null : String(r.violation_type),
          })
        );
        Modal.info({
          title: '移动完成（部分失败）',
          width: 920,
          content: (
            <Space direction="vertical" style={{ width: '100%' }} size={12}>
              <Alert type="warning" showIcon message={String(res?.message || '移动完成')} />
              {missing.length > 0 && (
                <Alert
                  type="info"
                  showIcon
                  message={`有 ${missing.length} 个物料不在当前版本排程中，已跳过`}
                />
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
    } catch (e: any) {
      console.error('[Workbench] moveItems failed:', e);
      message.error(e?.message || '移动失败');
    } finally {
      setMoveSubmitting(false);
    }
  };

  if (!activeVersionId) {
    return (
      <NoActiveVersionGuide
        title="尚无激活的排产版本"
        description="计划工作台需要一个激活的排产版本作为基础"
        onNavigateToImport={() => navigate('/import')}
        onNavigateToPlan={() => navigate('/comparison')}
      />
    );
  }

  return (
    <ErrorBoundary>
      <div style={{ height: '100%', display: 'flex', flexDirection: 'column', gap: 12 }}>
        <DecisionFlowGuide
          stage="workbench"
          title="下一步：去策略草案对比生成多方案预览"
          tags={
            <Space wrap size={6}>
              {workbenchFilters.machineCode ? <Tag color="blue">机组 {workbenchFilters.machineCode}</Tag> : null}
              {workbenchFilters.urgencyLevel ? <Tag color="volcano">紧急 {workbenchFilters.urgencyLevel}</Tag> : null}
              {workbenchFilters.lockStatus && workbenchFilters.lockStatus !== 'ALL' ? (
                <Tag color="geekblue">
                  {workbenchFilters.lockStatus === 'LOCKED' ? '仅锁定' : '仅未锁定'}
                </Tag>
              ) : null}
            </Space>
          }
          description="建议先在工作台处理 P0/P1 物料（移位/锁定/紧急/强制放行），再去草案对比选择推荐方案发布并激活。"
          primaryAction={{
            label: '去策略草案对比',
            onClick: () => navigate('/comparison?tab=draft'),
          }}
          secondaryAction={{
            label: '回风险概览',
            onClick: () => navigate('/overview'),
          }}
        />

        {/* 工具栏 */}
        <Card size="small">
          <Space
            wrap
            align="center"
            style={{ width: '100%', justifyContent: 'space-between' }}
          >
            <Space wrap>
              <Typography.Text type="secondary">当前版本</Typography.Text>
              <Typography.Text code>{activeVersionId || '-'}</Typography.Text>
              <Button
                size="small"
                icon={<ReloadOutlined />}
                onClick={() => {
                  setRefreshSignal((v) => v + 1);
                  materialsQuery.refetch();
                }}
              >
                刷新
              </Button>
            </Space>

            <Space wrap>
              <Button onClick={() => navigate('/comparison')}>版本管理</Button>
              <Button onClick={() => navigate('/comparison?tab=draft')}>生成策略对比方案</Button>
              <BatchOperationToolbar
                disabled={selectedMaterialIds.length === 0}
                onLock={() => runMaterialOperation(selectedMaterialIds, 'lock')}
                onUnlock={() => runMaterialOperation(selectedMaterialIds, 'unlock')}
                onSetUrgent={() => runMaterialOperation(selectedMaterialIds, 'urgent_on')}
                onClearUrgent={() => runMaterialOperation(selectedMaterialIds, 'urgent_off')}
                onForceRelease={() => runForceReleaseOperation(selectedMaterialIds)}
                onMove={openMoveModal}
                onConditional={() => {
                  setConditionMachine(poolSelection.machineCode || 'all');
                  setConditionSchedState('all');
                  setConditionUrgency('all');
                  setConditionLock('ALL');
                  setConditionSearch('');
                  setConditionalSelectOpen(true);
                }}
                onClear={() => setSelectedMaterialIds([])}
              />
              <Dropdown
                menu={{
                  onClick: ({ key }) => navigate(`/settings?tab=${key}`),
                  items: [
                    { key: 'materials', label: '物料管理（表格）' },
                    { key: 'machine', label: '机组配置（产能池）' },
                    { type: 'divider' },
                    { key: 'system', label: '系统配置' },
                    { key: 'logs', label: '操作日志' },
                  ],
                }}
              >
                <Button icon={<SettingOutlined />}>
                  设置/工具 <DownOutlined />
                </Button>
              </Dropdown>
              <OneClickOptimizeMenu
                activeVersionId={activeVersionId}
                operator={currentUser}
                onBeforeExecute={() => setRecalculating(true)}
                onAfterExecute={() => {
                  setRecalculating(false);
                  setRefreshSignal((v) => v + 1);
                  materialsQuery.refetch();
                  planItemsQuery.refetch();
                }}
              />
            </Space>
          </Space>
        </Card>

        {!materialsQuery.isLoading && !materialsQuery.error && materials.length === 0 ? (
          <Alert
            type="info"
            showIcon
            message="暂无物料数据"
            description="请先在“数据导入”导入材料CSV；导入成功后再返回工作台进行排程与干预。"
            action={
              <Button size="small" type="primary" onClick={() => navigate('/import')}>
                去导入
              </Button>
            }
          />
        ) : null}

        {!planItemsQuery.isLoading &&
        !planItemsQuery.error &&
        Array.isArray(planItemsQuery.data) &&
        planItemsQuery.data.length === 0 ? (
          <Alert
            type="info"
            showIcon
            message="当前版本暂无排程明细"
            description="可点击右上角“一键优化”执行重算生成排程，然后再使用矩阵/甘特图视图进行调整。"
          />
        ) : null}

        {/* 主体：左物料池 + 右视图 */}
        <div style={{ flex: 1, minHeight: 0, display: 'flex', gap: 12 }}>
          <div style={{ flex: '0 0 380px', minWidth: 320, minHeight: 0 }}>
            {deepLinkContext?.date ? (
              <div style={{ marginBottom: 8 }}>
                <Space wrap size={6}>
                  <Tag color="blue">
                    定位：{deepLinkContext.machine || poolSelection.machineCode || '全部机组'} / {deepLinkContext.date}
                  </Tag>
                  {deepLinkContextLabel ? <Tag>来源：{deepLinkContextLabel}</Tag> : null}
                  {dateRangeMode !== 'AUTO' ? (
                    <Button size="small" onClick={resetWorkbenchDateRangeToAuto}>
                      恢复自动范围
                    </Button>
                  ) : null}
                </Space>
              </div>
            ) : null}
            <MaterialPool
              materials={materials}
              loading={materialsQuery.isLoading}
              error={materialsQuery.error}
              onRetry={() => materialsQuery.refetch()}
              selection={poolSelection}
              onSelectionChange={(next) => {
                setPoolSelection(next);
                setWorkbenchFilters({ machineCode: next.machineCode });
              }}
              filters={{
                urgencyLevel: workbenchFilters.urgencyLevel,
                lockStatus: workbenchFilters.lockStatus,
              }}
              onFiltersChange={(next) => setWorkbenchFilters(next)}
              selectedMaterialIds={selectedMaterialIds}
              onSelectedMaterialIdsChange={setSelectedMaterialIds}
              onInspectMaterial={(m) => openInspector(m.material_id)}
            />
          </div>

          <div
            style={{
              flex: 1,
              minWidth: 0,
              minHeight: 0,
              display: 'flex',
              flexDirection: 'column',
              gap: 12,
            }}
          >
            <Card size="small" title="产能概览" bodyStyle={{ padding: 12 }}>
              <div style={{ maxHeight: 260, overflow: 'auto' }}>
                <CapacityTimelineContainer
                  machineCode={poolSelection.machineCode}
                  dateRange={workbenchDateRange}
                  onMachineCodeChange={applyWorkbenchMachineCode}
                  onDateRangeChange={applyWorkbenchDateRange}
                  onResetDateRange={resetWorkbenchDateRangeToAuto}
                  onOpenScheduleCell={(machine, date, _materialIds, options) => {
                    if (options?.statusFilter) {
                      setScheduleStatusFilter(options.statusFilter);
                    }
                    openGanttCellDetail(machine, date, 'capacity');
                  }}
                  selectedMaterialIds={selectedMaterialIds}
                  materials={materials}
                />
              </div>
            </Card>

            <Card
              size="small"
              title="排程视图"
              extra={
                <Space wrap size={8}>
                  <Select
                    size="small"
                    style={{ width: 148 }}
                    value={poolSelection.machineCode ?? 'all'}
                    onChange={(value) => applyWorkbenchMachineCode(value === 'all' ? null : (value as string))}
                    options={[
                      { label: '全部机组', value: 'all' },
                      ...machineOptions.map((code) => ({ label: code, value: code })),
                    ]}
                  />
                  {scheduleFocus?.date ? (
                    <Tag color="blue">
                      聚焦：
                      {String(scheduleFocus.machine || '').trim()
                        ? `${scheduleFocus.machine} / ${formatDate(scheduleFocus.date)}`
                        : poolSelection.machineCode && poolSelection.machineCode !== 'all'
                        ? `${poolSelection.machineCode} / ${formatDate(scheduleFocus.date)}`
                        : formatDate(scheduleFocus.date)}
                    </Tag>
                  ) : null}
                  <Segmented
                    value={workbenchViewMode}
                    options={[
                      { label: '矩阵', value: 'MATRIX' },
                      { label: '甘特图', value: 'GANTT' },
                      { label: '卡片', value: 'CARD' },
                    ]}
                    onChange={(value) => setWorkbenchViewMode(value as WorkbenchViewMode)}
                  />
                </Space>
              }
              style={{ flex: 1, minHeight: 0 }}
              bodyStyle={{
                height: '100%',
                minHeight: 0,
                display: 'flex',
                flexDirection: 'column',
              }}
            >
              <div style={{ flex: 1, minHeight: 0, height: '100%' }}>
                {workbenchViewMode === 'CARD' ? (
                  <ScheduleCardView
                    machineCode={poolSelection.machineCode}
                    urgentLevel={workbenchFilters.urgencyLevel}
                    dateRange={workbenchDateRange}
                    statusFilter={scheduleStatusFilter}
                    onStatusFilterChange={setScheduleStatusFilter}
                    refreshSignal={refreshSignal}
                    selectedMaterialIds={selectedMaterialIds}
                    onSelectedMaterialIdsChange={setSelectedMaterialIds}
                    onInspectMaterialId={(id) => openInspector(id)}
                  />
                ) : workbenchViewMode === 'GANTT' ? (
                  <ScheduleGanttView
                    machineCode={poolSelection.machineCode}
                    urgentLevel={workbenchFilters.urgencyLevel}
                    dateRange={workbenchDateRange}
                    suggestedDateRange={autoDateRange}
                    onDateRangeChange={applyWorkbenchDateRange}
                    focusedDate={ganttFocusedDate}
                    autoOpenCell={ganttOpenCellRequest || ganttAutoOpenCell}
                    statusFilter={scheduleStatusFilter}
                    onStatusFilterChange={setScheduleStatusFilter}
                    onFocusChange={setScheduleFocus}
                    focus={scheduleFocus}
                    onNavigateToMatrix={(machine, date) => {
                      setWorkbenchViewMode('MATRIX');
                      setMatrixFocusRequest({ machine, date, nonce: Date.now() });
                      setScheduleFocus({ machine, date, source: 'matrixJump' });
                    }}
                    planItems={planItemsQuery.data}
                    loading={planItemsQuery.isLoading}
                    error={planItemsQuery.error}
                    onRetry={() => planItemsQuery.refetch()}
                    selectedMaterialIds={selectedMaterialIds}
                    onSelectedMaterialIdsChange={setSelectedMaterialIds}
                    onInspectMaterialId={(id) => openInspector(id)}
                    onRequestMoveToCell={(machine, date) => openMoveModalAt(machine, date)}
                  />
                ) : (
                    <React.Suspense fallback={<PageSkeleton />}>
                      <PlanItemVisualization
                        machineCode={poolSelection.machineCode}
                        urgentLevel={workbenchFilters.urgencyLevel}
                        statusFilter={scheduleStatusFilter}
                        onStatusFilterChange={setScheduleStatusFilter}
                        focusRequest={matrixFocusRequest}
                        refreshSignal={refreshSignal}
                        selectedMaterialIds={selectedMaterialIds}
                        onSelectedMaterialIdsChange={setSelectedMaterialIds}
                      />
                    </React.Suspense>
                )}
              </div>
            </Card>
          </div>
        </div>

        {/* 状态栏 */}
        <Card size="small">
          <Space wrap align="center" style={{ width: '100%', justifyContent: 'space-between' }}>
            <Space wrap>
              <Typography.Text>
                已选: {selectedMaterialIds.length} 个物料
              </Typography.Text>
              <Typography.Text type="secondary">
                总重: {selectedTotalWeight.toFixed(2)}t
              </Typography.Text>
            </Space>

            <Space wrap>
              <Button
                disabled={selectedMaterialIds.length === 0}
                onClick={() => runMaterialOperation(selectedMaterialIds, 'lock')}
              >
                锁定
              </Button>
              <Button
                disabled={selectedMaterialIds.length === 0}
                onClick={() => runMaterialOperation(selectedMaterialIds, 'unlock')}
              >
                解锁
              </Button>
              <Button
                type="primary"
                danger
                disabled={selectedMaterialIds.length === 0}
                onClick={() => runMaterialOperation(selectedMaterialIds, 'urgent_on')}
              >
                设为紧急
              </Button>
              <Button
                disabled={selectedMaterialIds.length === 0}
                onClick={() => runMaterialOperation(selectedMaterialIds, 'urgent_off')}
              >
                取消紧急
              </Button>
              <Button
                danger
                disabled={selectedMaterialIds.length === 0}
                onClick={() => runForceReleaseOperation(selectedMaterialIds)}
              >
                强制放行
              </Button>
              <Button disabled={selectedMaterialIds.length === 0} onClick={openMoveModalWithRecommend}>
                最近可行
              </Button>
              <Button disabled={selectedMaterialIds.length === 0} onClick={openMoveModal}>
                移动到...
              </Button>
              <Button disabled={selectedMaterialIds.length === 0} onClick={() => setSelectedMaterialIds([])}>
                清空选择
              </Button>
            </Space>
          </Space>
        </Card>

        <Modal
          title="按条件选中..."
          open={conditionalSelectOpen}
          onCancel={() => setConditionalSelectOpen(false)}
          footer={[
            <Button key="close" onClick={() => setConditionalSelectOpen(false)}>
              关闭
            </Button>,
            <Dropdown
              key="apply"
              disabled={conditionalMatches.length === 0}
              menu={{
                onClick: ({ key }) => {
                  const ids = conditionalMatches.map((m) => m.material_id);
                  setConditionalSelectOpen(false);
                  if (key === 'lock') return runMaterialOperation(ids, 'lock');
                  if (key === 'unlock') return runMaterialOperation(ids, 'unlock');
                  if (key === 'urgent_on') return runMaterialOperation(ids, 'urgent_on');
                  if (key === 'urgent_off') return runMaterialOperation(ids, 'urgent_off');
                  if (key === 'force_release') return runForceReleaseOperation(ids);
                },
                items: [
                  { key: 'lock', label: `锁定命中（${conditionalMatches.length}）` },
                  { key: 'unlock', label: `解锁命中（${conditionalMatches.length}）` },
                  { type: 'divider' },
                  { key: 'urgent_on', label: `设为紧急（${conditionalMatches.length}）` },
                  { key: 'urgent_off', label: `取消紧急（${conditionalMatches.length}）` },
                  { type: 'divider' },
                  { key: 'force_release', label: `强制放行（${conditionalMatches.length}）` },
                ],
              }}
            >
              <Button disabled={conditionalMatches.length === 0}>
                对命中执行 <DownOutlined />
              </Button>
            </Dropdown>,
            <Button
              key="replace"
              type="primary"
              onClick={() => {
                setSelectedMaterialIds(conditionalMatches.map((m) => m.material_id));
                setConditionalSelectOpen(false);
              }}
              disabled={conditionalMatches.length === 0}
            >
              替换为这些物料
            </Button>,
            <Button
              key="merge"
              onClick={() => {
                const next = new Set(selectedMaterialIds);
                conditionalMatches.forEach((m) => next.add(m.material_id));
                setSelectedMaterialIds(Array.from(next));
                setConditionalSelectOpen(false);
              }}
              disabled={conditionalMatches.length === 0}
            >
              叠加到当前选择
            </Button>,
          ]}
          width={820}
        >
          <Space direction="vertical" style={{ width: '100%' }} size={12}>
            <Alert
              type="info"
              showIcon
              message="说明"
              description="先按条件筛选出物料集合，再“替换/叠加”为当前选择，随后可用工具栏/状态栏执行批量操作。"
            />
            <Space wrap>
              <span>机组</span>
              <Select
                value={conditionMachine}
                onChange={(v) => setConditionMachine(v)}
                style={{ width: 160 }}
                options={[{ label: '全部', value: 'all' }, ...machineOptions.map((m) => ({ label: m, value: m }))]}
                showSearch
                optionFilterProp="label"
              />
              <span>状态</span>
              <Select
                value={conditionSchedState}
                onChange={(v) => setConditionSchedState(v)}
                style={{ width: 160 }}
                options={[
                  { label: '全部', value: 'all' },
                  { label: '未成熟/冷料', value: 'PENDING_MATURE' },
                  { label: '待排/就绪', value: 'READY' },
                  { label: '强制放行', value: 'FORCE_RELEASE' },
                  { label: '已锁定', value: 'LOCKED' },
                  { label: '已排产', value: 'SCHEDULED' },
                  { label: '阻断', value: 'BLOCKED' },
                ]}
              />
              <span>紧急度</span>
              <Select
                value={conditionUrgency}
                onChange={(v) => setConditionUrgency(v)}
                style={{ width: 140 }}
                options={[
                  { label: '全部', value: 'all' },
                  { label: 'L3', value: 'L3' },
                  { label: 'L2', value: 'L2' },
                  { label: 'L1', value: 'L1' },
                  { label: 'L0', value: 'L0' },
                ]}
              />
              <span>锁定</span>
              <Select
                value={conditionLock}
                onChange={(v) => setConditionLock(v as ConditionLockFilter)}
                style={{ width: 140 }}
                options={[
                  { label: '全部', value: 'ALL' },
                  { label: '已锁', value: 'LOCKED' },
                  { label: '未锁', value: 'UNLOCKED' },
                ]}
              />
              <Input.Search
                placeholder="搜索材料号/钢种"
                allowClear
                value={conditionSearch}
                onChange={(e) => setConditionSearch(e.target.value)}
                style={{ width: 220 }}
              />
            </Space>

            <Card size="small">
              <Space wrap align="center" style={{ width: '100%', justifyContent: 'space-between' }}>
                <Space wrap>
                  <Typography.Text>命中 {conditionalSummary.count} 条</Typography.Text>
                  <Typography.Text type="secondary">总重 {conditionalSummary.weight.toFixed(2)}t</Typography.Text>
                </Space>
                {conditionalSummary.count > 2000 ? (
                  <Tag color="orange">命中较多，建议增加筛选条件</Tag>
                ) : null}
              </Space>
            </Card>

            <Table<MaterialPoolMaterial>
              size="small"
              rowKey={(r) => r.material_id}
              pagination={{ pageSize: 8, showSizeChanger: true }}
              dataSource={conditionalMatches}
              columns={[
                { title: '材料号', dataIndex: 'material_id', width: 160, render: (v) => <span style={{ fontFamily: 'monospace' }}>{String(v)}</span> },
                { title: '机组', dataIndex: 'machine_code', width: 90 },
                { title: '状态', dataIndex: 'sched_state', width: 120 },
                { title: '紧急度', dataIndex: 'urgent_level', width: 90, render: (v) => <Tag>{String(v || 'L0')}</Tag> },
                { title: '重量(t)', dataIndex: 'weight_t', width: 110, render: (v) => <span style={{ fontFamily: 'monospace' }}>{Number(v || 0).toFixed(2)}</span> },
                { title: '钢种', dataIndex: 'steel_mark', ellipsis: true },
              ]}
            />
          </Space>
        </Modal>

        <Modal
          title="移动到..."
          open={moveModalOpen}
          onCancel={() => setMoveModalOpen(false)}
          onOk={submitMove}
          okText="执行移动"
          confirmLoading={moveSubmitting}
          okButtonProps={{ disabled: selectedMaterialIds.length === 0 || !moveReason.trim() }}
        >
          <Space direction="vertical" style={{ width: '100%' }} size={12}>
            {planItemsQuery.isLoading ? (
              <Alert type="info" showIcon message="正在加载排程数据，用于校验/自动排队..." />
            ) : selectedPlanItemStats.outOfPlan > 0 ? (
              <Alert
                type="warning"
                showIcon
                message={`已选 ${selectedMaterialIds.length} 个，其中 ${selectedPlanItemStats.outOfPlan} 个不在当前版本排程中，将跳过`}
              />
            ) : null}

            {selectedPlanItemStats.frozenInPlan > 0 ? (
              <Alert
                type="warning"
                showIcon
                message={`检测到 ${selectedPlanItemStats.frozenInPlan} 个冻结排程项：STRICT 模式会失败，AUTO_FIX 模式会跳过`}
              />
            ) : null}

            <Space wrap align="center">
              <Button
                size="small"
                onClick={() => recommendMoveTarget()}
                loading={moveRecommendLoading}
                disabled={selectedMaterialIds.length === 0 || !moveTargetMachine}
              >
                推荐位置（最近可行）
              </Button>
              <Typography.Text type="secondary" style={{ fontSize: 12 }}>
                策略：{strategyLabel}
              </Typography.Text>
              {moveRecommendSummary ? (
                <Tag color={moveRecommendSummary.overLimitCount === 0 ? 'green' : 'orange'}>
                  推荐：{moveRecommendSummary.machine} {moveRecommendSummary.date}{' '}
                  {moveRecommendSummary.unknownCount > 0
                    ? `· 未知容量 ${moveRecommendSummary.unknownCount}`
                    : `· 超限 ${moveRecommendSummary.overLimitCount}`}
                </Tag>
              ) : null}
            </Space>

            <Space wrap>
              <span>目标机组</span>
              <Select
                style={{ minWidth: 180 }}
                value={moveTargetMachine}
                onChange={(v) => setMoveTargetMachine(v)}
                options={machineOptions.map((code) => ({ label: code, value: code }))}
                showSearch
                optionFilterProp="label"
                placeholder="请选择机组"
              />
            </Space>

            <Space wrap>
              <span>目标日期</span>
              <DatePicker
                value={moveTargetDate}
                onChange={(d) => setMoveTargetDate(d)}
                format="YYYY-MM-DD"
                allowClear={false}
              />
            </Space>

            <Space wrap>
              <span>排队方式</span>
              <Segmented
                value={moveSeqMode}
                options={[
                  { label: '追加到末尾', value: 'APPEND' },
                  { label: '指定起始序号', value: 'START_SEQ' },
                ]}
                onChange={(v) => setMoveSeqMode(v as MoveSeqMode)}
              />
              {moveSeqMode === 'START_SEQ' ? (
                <InputNumber
                  min={1}
                  precision={0}
                  value={moveStartSeq}
                  onChange={(v) => setMoveStartSeq(Number(v || 1))}
                  style={{ width: 140 }}
                />
              ) : null}
            </Space>

            <Space wrap>
              <span>校验模式</span>
              <Select
                value={moveValidationMode}
                style={{ width: 180 }}
                onChange={(v) => setMoveValidationMode(v)}
                options={[
                  { label: 'AUTO_FIX（跳过冻结）', value: 'AUTO_FIX' },
                  { label: 'STRICT（遇冻结失败）', value: 'STRICT' },
                ]}
              />
            </Space>

            <Typography.Text type="secondary" style={{ fontSize: 12 }}>
              请输入移动原因（必填，将写入操作日志）
            </Typography.Text>
            <Space wrap align="center">
              <span>快捷原因</span>
              <Select
                style={{ minWidth: 220 }}
                value={
                  QUICK_MOVE_REASONS.some((opt) => opt.value === moveReason.trim())
                    ? moveReason.trim()
                    : undefined
                }
                onChange={(v) => setMoveReason(String(v || DEFAULT_MOVE_REASON))}
                options={QUICK_MOVE_REASONS}
                placeholder="选择一个常用原因"
              />
              <Typography.Text type="secondary" style={{ fontSize: 12 }}>
                可在下方补充说明
              </Typography.Text>
            </Space>
            <Input.TextArea
              value={moveReason}
              onChange={(e) => setMoveReason(e.target.value)}
              rows={3}
              autoSize={{ minRows: 3, maxRows: 6 }}
              placeholder="例如：为满足L3紧急订单，调整到更早日期"
            />

            <Typography.Text type="secondary" style={{ fontSize: 12 }}>
              提示：当前后端的 move_items 不返回“影响预览”，执行后可通过风险概览/对比页观察变化。
            </Typography.Text>

            <Divider style={{ margin: '4px 0' }} />

            <Space direction="vertical" style={{ width: '100%' }} size={8}>
              <Typography.Text strong>影响预览（本地估算）</Typography.Text>
              {!moveImpactPreview ? (
                <Typography.Text type="secondary" style={{ fontSize: 12 }}>
                  暂无可用预览（请先选择目标机组/日期）。
                </Typography.Text>
              ) : moveImpactPreview.rows.length === 0 ? (
                <Alert
                  type="info"
                  showIcon
                  message="未检测到产能变化"
                  description="所选物料均在相同机组/日期内（仅可能改变顺序），不会引起产能占用变化。"
                />
              ) : (
                <>
                  {moveImpactPreview.loading ? (
                    <Alert type="info" showIcon message="正在加载产能池，用于评估超限风险..." />
                  ) : null}
                  {moveImpactPreview.overflowRows.length > 0 ? (
                    <Alert
                      type="warning"
                      showIcon
                      message={`警告：预计有 ${moveImpactPreview.overflowRows.length} 个机组/日期将超出限制产能`}
                      description="可尝试切换到其他日期/机组，或使用 AUTO_FIX 模式（冻结项将跳过）。"
                    />
                  ) : (
                    <Alert type="success" showIcon message="未发现超限风险（按当前估算）" />
                  )}
                  <Table<MoveImpactRow>
                    size="small"
                    pagination={false}
                    rowKey={(r) => `${r.machine_code}__${r.date}`}
                    dataSource={moveImpactPreview.rows}
                    columns={[
                      { title: '机组', dataIndex: 'machine_code', width: 90 },
                      { title: '日期', dataIndex: 'date', width: 120 },
                      {
                        title: '操作前(t)',
                        dataIndex: 'before_t',
                        width: 120,
                        render: (v) => <span style={{ fontFamily: 'monospace' }}>{Number(v).toFixed(1)}</span>,
                      },
                      {
                        title: '变化(t)',
                        dataIndex: 'delta_t',
                        width: 110,
                        render: (v) => {
                          const n = Number(v);
                          const color = n > 0 ? 'green' : n < 0 ? 'red' : 'default';
                          const label = `${n >= 0 ? '+' : ''}${n.toFixed(1)}`;
                          return <Tag color={color}>{label}</Tag>;
                        },
                      },
                      {
                        title: '操作后(t)',
                        dataIndex: 'after_t',
                        width: 120,
                        render: (v) => <span style={{ fontFamily: 'monospace' }}>{Number(v).toFixed(1)}</span>,
                      },
                      {
                        title: '目标/限制(t)',
                        key: 'cap',
                        render: (_, r) => {
                          const target = r.target_capacity_t;
                          const limit = r.limit_capacity_t;
                          if (target == null && limit == null) return <span>-</span>;
                          if (limit != null && target != null && Math.abs(limit - target) < 1e-9) {
                            return (
                              <span style={{ fontFamily: 'monospace' }}>
                                {target.toFixed(0)}
                              </span>
                            );
                          }
                          return (
                            <span style={{ fontFamily: 'monospace' }}>
                              {(target ?? 0).toFixed(0)} / {(limit ?? 0).toFixed(0)}
                            </span>
                          );
                        },
                      },
                      {
                        title: '风险',
                        key: 'risk',
                        width: 110,
                        render: (_, r) => {
                          const limit = r.limit_capacity_t;
                          if (limit == null || limit <= 0) return <Tag>未知</Tag>;
                          const pct = (r.after_t / limit) * 100;
                          if (pct > 100) return <Tag color="red">超限 {pct.toFixed(0)}%</Tag>;
                          if (pct > 90) return <Tag color="orange">偏高 {pct.toFixed(0)}%</Tag>;
                          return <Tag color="green">正常 {pct.toFixed(0)}%</Tag>;
                        },
                      },
                    ]}
                    scroll={{ y: 240 }}
                  />
                </>
              )}
            </Space>
          </Space>
        </Modal>

        {/* 物料 Inspector */}
        <MaterialInspector
          visible={inspectorOpen}
          material={inspectedMaterial as any}
          onClose={() => setInspectorOpen(false)}
          onLock={(id) => runMaterialOperation([id], 'lock')}
          onUnlock={(id) => runMaterialOperation([id], 'unlock')}
          onSetUrgent={(id) => runMaterialOperation([id], 'urgent_on')}
          onClearUrgent={(id) => runMaterialOperation([id], 'urgent_off')}
        />
      </div>
    </ErrorBoundary>
  );
};

export default PlanningWorkbench;
