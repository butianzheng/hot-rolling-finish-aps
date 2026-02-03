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
import { materialApi, pathRuleApi, planApi } from '../api/tauri';
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
import { getErrorMessage } from '../utils/errorUtils';
import type { PlanItemStatusFilter } from '../utils/planItemStatus';
import MaterialPool, { type MaterialPoolMaterial, type MaterialPoolSelection } from '../components/workbench/MaterialPool';
import ScheduleCardView from '../components/workbench/ScheduleCardView';
import ScheduleGanttView from '../components/workbench/ScheduleGanttView';
import BatchOperationToolbar from '../components/workbench/BatchOperationToolbar';
import OneClickOptimizeMenu from '../components/workbench/OneClickOptimizeMenu';
import DailyRhythmManagerModal from '../components/workbench/DailyRhythmManagerModal';
import PathOverrideConfirmModal from '../components/path-override-confirm/PathOverrideConfirmModal';
import PathOverridePendingCenterModal from '../components/path-override-confirm/PathOverridePendingCenterModal';
import RollCycleAnchorCard from '../components/roll-cycle-anchor/RollCycleAnchorCard';
import { CapacityTimelineContainer } from '../components/CapacityTimelineContainer';
import { MaterialInspector } from '../components/MaterialInspector';
import { RedLineGuard, createFrozenZoneViolation, createMaturityViolation } from '../components/guards/RedLineGuard';
import type { RedLineViolation } from '../components/guards/RedLineGuard';
import DecisionFlowGuide from '../components/flow/DecisionFlowGuide';
import { DEFAULT_MOVE_REASON, QUICK_MOVE_REASONS } from './workbench/constants';
import { useWorkbenchAutoDateRange } from './workbench/hooks/useWorkbenchAutoDateRange';
import { useWorkbenchDeepLink } from './workbench/hooks/useWorkbenchDeepLink';
import { useWorkbenchMoveModal } from './workbench/hooks/useWorkbenchMoveModal';
import type { ConditionLockFilter, MoveImpactRow, MoveSeqMode, MoveValidationMode, WorkbenchDateRangeMode } from './workbench/types';
import { extractForceReleaseViolations } from './workbench/utils';

const PlanItemVisualization = React.lazy(() => import('../components/PlanItemVisualization'));

type IpcMaterialWithState = Awaited<ReturnType<typeof materialApi.listMaterials>>[number];
type IpcMaterialDetail = Awaited<ReturnType<typeof materialApi.getMaterialDetail>>;
type IpcImpactSummary = Awaited<ReturnType<typeof materialApi.batchForceRelease>>;
type IpcPathOverridePendingSummary = Awaited<ReturnType<typeof pathRuleApi.listPathOverridePendingSummary>>[number];

const PlanningWorkbench: React.FC = () => {
  const navigate = useNavigate();
  const [searchParams] = useSearchParams();
  const activeVersionId = useActiveVersionId();
  const currentUser = useCurrentUser();
  const adminOverrideMode = useAdminOverrideMode();
  const workbenchViewMode = useGlobalStore((state) => state.workbenchViewMode);
  const workbenchFilters = useGlobalStore((state) => state.workbenchFilters);
  const preferences = useUserPreferences();
  const { setRecalculating, setActiveVersion, setWorkbenchViewMode, setWorkbenchFilters } = useGlobalActions();
  const [refreshSignal, setRefreshSignal] = useState(0);
  const bumpRefreshSignal = React.useCallback(() => setRefreshSignal((v) => v + 1), []);

  const [pathOverrideModalOpen, setPathOverrideModalOpen] = useState(false);
  const [pathOverrideCenterOpen, setPathOverrideCenterOpen] = useState(false);

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

  const [rhythmModalOpen, setRhythmModalOpen] = useState(false);

  const { deepLinkContext, deepLinkContextLabel } = useWorkbenchDeepLink({
    searchParams,
    globalMachineCode: workbenchFilters.machineCode,
    setPoolSelection,
    setWorkbenchFilters,
    setWorkbenchViewMode,
    setDateRangeMode,
    setWorkbenchDateRange,
    setInspectorOpen,
    setInspectedMaterialId,
  });

  // P2-2 修复：queryKey 包含筛选参数，避免缓存污染
  // 注意：暂保留 limit=1000 硬编码，待后续实施 useInfiniteQuery 分页优化
  const materialQueryParams = useMemo(() => ({
    machine_code: poolSelection.machineCode && poolSelection.machineCode !== 'all'
      ? poolSelection.machineCode
      : undefined,
    limit: 1000,
    offset: 0,
  }), [poolSelection.machineCode]);

  const materialsQuery = useQuery({
    queryKey: ['materials', materialQueryParams],
    queryFn: async () => {
      return materialApi.listMaterials(materialQueryParams);
    },
    staleTime: 30 * 1000,
  });

  const materials = useMemo<MaterialPoolMaterial[]>(() => {
    const raw: IpcMaterialWithState[] = materialsQuery.data ?? [];
    return raw.map((m) => {
      const sched = normalizeSchedState(m.sched_state);
      const is_mature =
        sched === 'PENDING_MATURE'
          ? false
          : sched === 'READY' || sched === 'FORCE_RELEASE' || sched === 'SCHEDULED'
            ? true
            : undefined;

      return {
        material_id: String(m.material_id ?? '').trim(),
        machine_code: String(m.machine_code ?? '').trim(),
        weight_t: Number(m.weight_t ?? 0),
        steel_mark: String(m.steel_mark ?? '').trim(),
        sched_state: String(m.sched_state ?? '').trim(),
        urgent_level: String(m.urgent_level ?? '').trim(),
        lock_flag: Boolean(m.lock_flag),
        manual_urgent_flag: Boolean(m.manual_urgent_flag),
        is_mature,
      };
    });
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

  const planItemsQuery = useQuery({
    queryKey: ['planItems', activeVersionId],
    enabled: !!activeVersionId,
    queryFn: async () => {
      if (!activeVersionId) return [];
      return planApi.listPlanItems(activeVersionId);
    },
    staleTime: 30 * 1000,
  });

  React.useEffect(() => {
    if (!activeVersionId) return;
    if (refreshSignal == null) return;
    planItemsQuery.refetch();
  }, [activeVersionId, refreshSignal, planItemsQuery.refetch]);

  const defaultPlanDate = useMemo(() => formatDate(dayjs()), []);
  const pathOverrideContext = useMemo(() => {
    const machine = String(scheduleFocus?.machine || poolSelection.machineCode || '').trim();
    const date = String(scheduleFocus?.date || defaultPlanDate).trim();
    return {
      machineCode: machine || null,
      planDate: date || null,
    };
  }, [defaultPlanDate, poolSelection.machineCode, scheduleFocus?.date, scheduleFocus?.machine]);

  const pathOverridePendingQuery = useQuery({
    queryKey: [
      'pathOverridePending',
      activeVersionId,
      pathOverrideContext.machineCode,
      pathOverrideContext.planDate,
      refreshSignal,
    ],
    enabled: !!activeVersionId && !!pathOverrideContext.machineCode && !!pathOverrideContext.planDate,
    queryFn: async () => {
      if (!activeVersionId || !pathOverrideContext.machineCode || !pathOverrideContext.planDate) return [];
      return pathRuleApi.listPathOverridePending({
        versionId: activeVersionId,
        machineCode: pathOverrideContext.machineCode,
        planDate: pathOverrideContext.planDate,
      });
    },
    staleTime: 15 * 1000,
  });

  const pathOverridePendingCount = useMemo(() => {
    return pathOverridePendingQuery.data?.length ?? 0;
  }, [pathOverridePendingQuery.data]);

  const recalcAfterPathOverride = async (baseDate: string) => {
    if (!activeVersionId) return;
    const base = String(baseDate || '').trim() || defaultPlanDate;
    setRecalculating(true);
    try {
      const res = await planApi.recalcFull(
        activeVersionId,
        base,
        undefined,
        currentUser || 'admin',
        preferences.defaultStrategy || 'balanced'
      );
      const nextVersionId = String(res?.version_id ?? '').trim();
      if (nextVersionId) {
        setActiveVersion(nextVersionId);
        message.success(`已重算并切换到新版本：${nextVersionId}`);
      } else {
        message.success(String(res?.message || '重算完成'));
      }
      setRefreshSignal((v) => v + 1);
      materialsQuery.refetch();
    } catch (e: unknown) {
      console.error('[Workbench] recalcAfterPathOverride failed:', e);
      message.error(getErrorMessage(e) || '重算失败');
    } finally {
      setRecalculating(false);
    }
  };

  // AUTO 日期范围（基于当前机组的排程数据）
  const { autoDateRange, applyWorkbenchDateRange, resetWorkbenchDateRangeToAuto } = useWorkbenchAutoDateRange({
    planItems: planItemsQuery.data || [],
    machineCode: poolSelection.machineCode,
    dateRangeMode,
    setDateRangeMode,
    setWorkbenchDateRange,
  });

  // 路径规则：跨日期/跨机组待确认汇总（基于当前 AUTO 日期范围）
  const pathOverrideSummaryRange = useMemo(() => {
    return {
      from: formatDate(autoDateRange[0]),
      to: formatDate(autoDateRange[1]),
    };
  }, [autoDateRange]);

  const pathOverrideSummaryQuery = useQuery({
    queryKey: [
      'pathOverridePendingSummary',
      activeVersionId,
      pathOverrideSummaryRange.from,
      pathOverrideSummaryRange.to,
      refreshSignal,
    ],
    enabled: !!activeVersionId,
    queryFn: async () => {
      if (!activeVersionId) return [];
      return pathRuleApi.listPathOverridePendingSummary({
        versionId: activeVersionId,
        planDateFrom: pathOverrideSummaryRange.from,
        planDateTo: pathOverrideSummaryRange.to,
      });
    },
    staleTime: 15 * 1000,
  });

  const pathOverridePendingTotalCount = useMemo(() => {
    const list: IpcPathOverridePendingSummary[] = pathOverrideSummaryQuery.data ?? [];
    return list.reduce((sum, r) => sum + Number(r.pending_count ?? 0), 0);
  }, [pathOverrideSummaryQuery.data]);

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

  const {
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
  } = useWorkbenchMoveModal({
    activeVersionId,
    operator: currentUser || 'admin',
    deepLinkDate: deepLinkContext?.date || null,
    poolMachineCode: poolSelection.machineCode,
    machineOptions,
    defaultStrategy: preferences.defaultStrategy,
    workbenchDateRange,
    planItems: planItemsQuery.data ?? [],
    planItemsRefetch: planItemsQuery.refetch,
    selectedMaterialIds,
    setSelectedMaterialIds,
    bumpRefreshSignal,
    materialsRefetch: materialsQuery.refetch,
  });

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
        const res: IpcImpactSummary = await materialApi.batchForceRelease(materialIds, operator, trimmed, mode);

        message.success(String(res?.message || '强制放行完成'));

        const violations = extractForceReleaseViolations(res?.details);
        if (violations.length > 0) {
          const rows = violations.map((v, idx: number) => ({
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
              <Button onClick={() => setRhythmModalOpen(true)}>每日节奏</Button>
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
                    { key: 'path_rule', label: '路径规则（v0.6）' },
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

        <DailyRhythmManagerModal
          open={rhythmModalOpen}
          onClose={() => setRhythmModalOpen(false)}
          versionId={activeVersionId}
          machineOptions={machineOptions}
          defaultMachineCode={scheduleFocus?.machine || poolSelection.machineCode || machineOptions[0] || null}
          defaultPlanDate={scheduleFocus?.date || formatDate(dayjs())}
          operator={currentUser || 'system'}
        />

        <PathOverrideConfirmModal
          open={pathOverrideModalOpen}
          onClose={() => setPathOverrideModalOpen(false)}
          versionId={activeVersionId}
          machineCode={pathOverrideContext.machineCode}
          planDate={pathOverrideContext.planDate}
          operator={currentUser || 'system'}
          onConfirmed={async ({ confirmedCount, autoRecalc }) => {
            if (confirmedCount <= 0) return;
            pathOverridePendingQuery.refetch();
            pathOverrideSummaryQuery.refetch();
            if (autoRecalc) {
              setPathOverrideModalOpen(false);
              await recalcAfterPathOverride(pathOverrideContext.planDate || defaultPlanDate);
            }
          }}
        />

        <PathOverridePendingCenterModal
          open={pathOverrideCenterOpen}
          onClose={() => setPathOverrideCenterOpen(false)}
          versionId={activeVersionId}
          planDateFrom={pathOverrideSummaryRange.from}
          planDateTo={pathOverrideSummaryRange.to}
          machineOptions={machineOptions}
          operator={currentUser || 'system'}
          onConfirmed={async ({ confirmedCount, autoRecalc, recalcBaseDate }) => {
            if (confirmedCount <= 0) return;
            pathOverridePendingQuery.refetch();
            pathOverrideSummaryQuery.refetch();
            if (autoRecalc) {
              setPathOverrideCenterOpen(false);
              await recalcAfterPathOverride(recalcBaseDate || defaultPlanDate);
            }
          }}
        />

        {pathOverridePendingTotalCount > 0 && activeVersionId ? (
          <Alert
            type="warning"
            showIcon
            message={`路径规则待确认（跨日期/跨机组）：${pathOverridePendingTotalCount} 条`}
            description={`范围 ${pathOverrideSummaryRange.from} ~ ${pathOverrideSummaryRange.to}（确认后建议重算生成新版本）`}
            action={
              <Space>
                <Button
                  size="small"
                  type="primary"
                  icon={<InfoCircleOutlined />}
                  loading={pathOverrideSummaryQuery.isFetching}
                  onClick={() => setPathOverrideCenterOpen(true)}
                >
                  待确认中心
                </Button>
              </Space>
            }
          />
        ) : null}

        {pathOverridePendingCount > 0 && pathOverrideContext.machineCode && pathOverrideContext.planDate ? (
          <Alert
            type="warning"
            showIcon
            message={`路径规则待确认：${pathOverridePendingCount} 条`}
            description={`机组 ${pathOverrideContext.machineCode} · 日期 ${pathOverrideContext.planDate}（确认后建议重算生成新版本）`}
            action={
              <Space>
                <Button
                  size="small"
                  type="primary"
                  icon={<InfoCircleOutlined />}
                  loading={pathOverridePendingQuery.isFetching}
                  onClick={() => setPathOverrideModalOpen(true)}
                >
                  去确认
                </Button>
              </Space>
            }
          />
        ) : null}

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
            <RollCycleAnchorCard
              versionId={activeVersionId}
              machineCode={poolSelection.machineCode}
              operator={currentUser || 'system'}
              refreshSignal={refreshSignal}
              onAfterReset={() => {
                setRefreshSignal((v) => v + 1);
                pathOverridePendingQuery.refetch();
                message.info('已重置换辊周期：建议执行“一键优化/重算”以刷新排程结果');
              }}
            />

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
                  <Button
                    size="small"
                    icon={<InfoCircleOutlined />}
                    type={pathOverridePendingCount > 0 ? 'primary' : 'default'}
                    danger={pathOverridePendingCount > 0}
                    disabled={!pathOverrideContext.machineCode}
                    loading={pathOverridePendingQuery.isFetching}
                    onClick={() => setPathOverrideModalOpen(true)}
                  >
                    路径待确认{pathOverridePendingCount > 0 ? ` ${pathOverridePendingCount}` : ''}
                  </Button>
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
                onChange={(v) => setMoveValidationMode(v as MoveValidationMode)}
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
          material={inspectedMaterial}
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
