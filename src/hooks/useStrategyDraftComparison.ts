/**
 * 策略草案对比状态管理 Hook
 * 集中管理所有状态和业务逻辑
 */

import { useCallback, useEffect, useMemo, useState } from 'react';
import { Modal, message } from 'antd';
import type { Dayjs } from 'dayjs';
import dayjs from 'dayjs';
import { useNavigate, useSearchParams } from 'react-router-dom';
import { configApi, dashboardApi, materialApi, planApi } from '../api/tauri';
import { useActiveVersionId, useCurrentUser, useGlobalActions } from '../stores/use-global-store';
import {
  FALLBACK_STRATEGIES,
  type ActionLogRow,
  type ApplyStrategyDraftResponse,
  type CustomStrategyProfile,
  type GenerateStrategyDraftsResponse,
  type GetStrategyDraftDetailResponse,
  type ListStrategyDraftsResponse,
  type MaterialDetailPayload,
  type SqueezedHintCache,
  type StrategyDraftDiffItem,
  type StrategyDraftSummary,
  type StrategyKey,
  type StrategyPreset,
  makeCustomStrategyKey,
} from '../types/strategy-draft';
import {
  buildSqueezedOutHintSections,
  clampRange,
  normalizeMaterialDetail,
} from '../utils/strategyDraftFormatters';

export interface UseStrategyDraftComparisonReturn {
  // 基础状态
  activeVersionId: string | null;
  currentUser: string | null;
  range: [Dayjs, Dayjs];
  setRange: (range: [Dayjs, Dayjs]) => void;
  strategies: StrategyPreset[];
  selectedStrategies: StrategyKey[];
  setSelectedStrategies: (keys: StrategyKey[]) => void;

  // 草案数据
  draftsByStrategy: Partial<Record<StrategyKey, StrategyDraftSummary>>;
  isGenerating: boolean;
  publishingDraftId: string | null;
  hasAnyDraft: boolean;

  // 发布后弹窗
  postPublishOpen: boolean;
  setPostPublishOpen: (open: boolean) => void;
  createdVersionId: string | null;
  postActionLoading: 'switch' | 'activate' | null;

  // 明细抽屉
  detailOpen: boolean;
  setDetailOpen: (open: boolean) => void;
  detailLoading: boolean;
  detailDraft: StrategyDraftSummary | null;
  detailResp: GetStrategyDraftDetailResponse | null;
  detailFilter: 'ALL' | 'MOVED' | 'ADDED' | 'SQUEEZED_OUT';
  setDetailFilter: (filter: 'ALL' | 'MOVED' | 'ADDED' | 'SQUEEZED_OUT') => void;
  detailSearch: string;
  setDetailSearch: (search: string) => void;
  detailItems: StrategyDraftDiffItem[];

  // 物料弹窗
  materialModalOpen: boolean;
  materialModalLoading: boolean;
  materialModalContext: StrategyDraftDiffItem | null;
  materialModalData: MaterialDetailPayload | null;
  materialModalError: string | null;
  materialModalLogsLoading: boolean;
  materialModalLogsError: string | null;
  materialModalLogs: ActionLogRow[];

  // 挤出提示缓存
  squeezedHintCache: SqueezedHintCache;

  // 计算属性
  selectedStrategyKeysInOrder: StrategyKey[];
  headerHint: string;
  rangeDays: number;
  strategyTitleMap: Partial<Record<StrategyKey, string>>;
  recommendation: StrategyDraftSummary | null;
  canGenerate: boolean;

  // 操作方法
  handleGenerate: () => Promise<void>;
  handleApply: (draft: StrategyDraftSummary) => void;
  openDetail: (draft: StrategyDraftSummary) => Promise<void>;
  closeDetail: () => void;
  openMaterialDetail: (row: StrategyDraftDiffItem) => Promise<void>;
  closeMaterialModal: () => void;
  ensureSqueezedHint: (materialId: string) => Promise<void>;
  handlePostSwitch: () => Promise<void>;
  handlePostActivate: () => Promise<void>;
  closePostPublish: () => void;

  // 导航
  navigate: ReturnType<typeof useNavigate>;
}

export function useStrategyDraftComparison(): UseStrategyDraftComparisonReturn {
  const navigate = useNavigate();
  const [searchParams] = useSearchParams();
  const activeVersionId = useActiveVersionId();
  const currentUser = useCurrentUser();
  const { setActiveVersion } = useGlobalActions();

  // ========== 基础状态 ==========
  const [range, setRangeInternal] = useState<[Dayjs, Dayjs]>(() => [
    dayjs().startOf('day'),
    dayjs().add(6, 'day').startOf('day'),
  ]);
  const [strategies, setStrategies] = useState<StrategyPreset[]>(FALLBACK_STRATEGIES);
  const [selectedStrategies, setSelectedStrategies] = useState<StrategyKey[]>(
    FALLBACK_STRATEGIES.map((s) => s.key)
  );
  const [querySelectionApplied, setQuerySelectionApplied] = useState(false);

  // ========== 草案状态 ==========
  const [draftsByStrategy, setDraftsByStrategy] = useState<Partial<Record<StrategyKey, StrategyDraftSummary>>>({});
  const [isGenerating, setIsGenerating] = useState(false);
  const [publishingDraftId, setPublishingDraftId] = useState<string | null>(null);

  // ========== 发布后弹窗 ==========
  const [postPublishOpen, setPostPublishOpen] = useState(false);
  const [createdVersionId, setCreatedVersionId] = useState<string | null>(null);
  const [postActionLoading, setPostActionLoading] = useState<'switch' | 'activate' | null>(null);

  // ========== 明细抽屉 ==========
  const [detailOpen, setDetailOpen] = useState(false);
  const [detailLoading, setDetailLoading] = useState(false);
  const [detailDraft, setDetailDraft] = useState<StrategyDraftSummary | null>(null);
  const [detailResp, setDetailResp] = useState<GetStrategyDraftDetailResponse | null>(null);
  const [detailFilter, setDetailFilter] = useState<'ALL' | 'MOVED' | 'ADDED' | 'SQUEEZED_OUT'>('ALL');
  const [detailSearch, setDetailSearch] = useState('');

  // ========== 物料弹窗 ==========
  const [materialModalOpen, setMaterialModalOpen] = useState(false);
  const [materialModalLoading, setMaterialModalLoading] = useState(false);
  const [materialModalContext, setMaterialModalContext] = useState<StrategyDraftDiffItem | null>(null);
  const [materialModalData, setMaterialModalData] = useState<MaterialDetailPayload | null>(null);
  const [materialModalError, setMaterialModalError] = useState<string | null>(null);
  const [materialModalLogsLoading, setMaterialModalLogsLoading] = useState(false);
  const [materialModalLogsError, setMaterialModalLogsError] = useState<string | null>(null);
  const [materialModalLogs, setMaterialModalLogs] = useState<ActionLogRow[]>([]);

  // ========== 挤出提示缓存 ==========
  const [squeezedHintCache, setSqueezedHintCache] = useState<SqueezedHintCache>({});

  // ========== 计算属性 ==========
  const selectedStrategyKeysInOrder = useMemo(
    () => strategies.filter((s) => selectedStrategies.includes(s.key)).map((s) => s.key),
    [strategies, selectedStrategies]
  );

  const headerHint = useMemo(() => {
    const [start, end] = range;
    const days = end.diff(start, 'day') + 1;
    return `${start.format('YYYY-MM-DD')} ~ ${end.format('YYYY-MM-DD')}（${days}天）`;
  }, [range]);

  const rangeDays = useMemo(() => {
    const [start, end] = range;
    return end.diff(start, 'day') + 1;
  }, [range]);

  const strategyTitleMap = useMemo(() => {
    const map: Partial<Record<StrategyKey, string>> = {};
    strategies.forEach((s) => {
      map[s.key] = s.title;
    });
    return map;
  }, [strategies]);

  const recommendation = useMemo(() => {
    const candidates = selectedStrategyKeysInOrder
      .map((k) => draftsByStrategy[k])
      .filter((d): d is StrategyDraftSummary => Boolean(d?.draft_id));

    if (candidates.length < 2) return null;

    const sorted = [...candidates].sort((a, b) => {
      if ((a.overflow_days ?? 0) !== (b.overflow_days ?? 0)) return (a.overflow_days ?? 0) - (b.overflow_days ?? 0);
      if ((a.total_capacity_used_t ?? 0) !== (b.total_capacity_used_t ?? 0))
        return (b.total_capacity_used_t ?? 0) - (a.total_capacity_used_t ?? 0);
      if ((a.squeezed_out_count ?? 0) !== (b.squeezed_out_count ?? 0))
        return (a.squeezed_out_count ?? 0) - (b.squeezed_out_count ?? 0);
      return (b.mature_count ?? 0) - (a.mature_count ?? 0);
    });

    return sorted[0] || null;
  }, [draftsByStrategy, selectedStrategyKeysInOrder]);

  const hasAnyDraft = useMemo(
    () => Object.values(draftsByStrategy).some((d) => Boolean(d?.draft_id)),
    [draftsByStrategy]
  );

  const canGenerate = Boolean(activeVersionId) && selectedStrategies.length > 0 && !isGenerating;

  const detailItems = useMemo(() => {
    const items = Array.isArray(detailResp?.diff_items) ? detailResp!.diff_items : [];
    const q = detailSearch.trim().toLowerCase();
    return items.filter((it) => {
      if (detailFilter !== 'ALL' && String(it.change_type) !== detailFilter) return false;
      if (!q) return true;
      return String(it.material_id || '').toLowerCase().includes(q);
    });
  }, [detailResp, detailFilter, detailSearch]);

  // ========== 设置日期范围（带限制） ==========
  const setRange = useCallback((newRange: [Dayjs, Dayjs]) => {
    const clamped = clampRange(newRange);
    if (clamped[1].diff(clamped[0], 'day') + 1 > 60) {
      message.warning('时间跨度过大，已限制为60天');
    }
    setRangeInternal(clamped);
  }, []);

  // ========== 加载策略预设 ==========
  useEffect(() => {
    let cancelled = false;
    (async () => {
      try {
        const [presetRes, customRes] = await Promise.all([
          planApi.getStrategyPresets().catch(() => null),
          configApi.listCustomStrategies().catch(() => null),
        ]);
        if (cancelled) return;

        const presetList: StrategyPreset[] = (Array.isArray(presetRes) ? presetRes : FALLBACK_STRATEGIES)
          .map((p: any) => ({
            key: String(p?.strategy ?? p?.key ?? ''),
            title: String(p?.title ?? ''),
            description: String(p?.description ?? ''),
            kind: 'preset' as const,
          }))
          .filter((p) => Boolean(p.key));

        const presetTitleByKey: Record<string, string> = {};
        presetList.forEach((p) => {
          presetTitleByKey[p.key] = p.title;
        });

        const profiles: CustomStrategyProfile[] = Array.isArray(customRes)
          ? customRes
              .map((p: any) => ({
                strategy_id: String(p?.strategy_id ?? ''),
                title: String(p?.title ?? ''),
                description: p?.description != null ? String(p.description) : null,
                base_strategy: String(p?.base_strategy ?? ''),
                parameters: p?.parameters ?? null,
              }))
              .filter((p) => p.strategy_id && p.title && p.base_strategy)
          : [];

        const customList: StrategyPreset[] = profiles.map((p) => {
          const key = makeCustomStrategyKey(p.strategy_id);
          const baseTitle = presetTitleByKey[p.base_strategy] || p.base_strategy;
          const desc = [`自定义策略（基于：${baseTitle}）`, p.description ? p.description : null].filter(Boolean).join(' · ');

          return {
            key,
            title: p.title,
            description: desc,
            kind: 'custom' as const,
            base_strategy: p.base_strategy,
            strategy_id: p.strategy_id,
            parameters: p.parameters,
          };
        });

        const nextStrategies = [...presetList, ...customList];

        if (nextStrategies.length) {
          setStrategies(nextStrategies);
          setSelectedStrategies((prev) => {
            const nextKeys = nextStrategies.map((s) => s.key);
            if (!prev.length) return nextKeys;
            const filtered = prev.filter((k) => nextKeys.includes(k));
            return filtered.length ? filtered : nextKeys;
          });
        }
      } catch {
        // fallback to hardcoded presets
      }
    })();
    return () => {
      cancelled = true;
    };
  }, []);

  // ========== URL 参数预选策略 ==========
  useEffect(() => {
    if (querySelectionApplied) return;
    const raw = String(searchParams.get('strategies') || '').trim();
    if (!raw) return;

    const requested = raw
      .split(',')
      .map((s) => String(s || '').trim())
      .filter(Boolean);
    if (!requested.length) return;

    const available = new Set(strategies.map((s) => s.key));
    const next = requested.filter((k) => available.has(k));
    if (!next.length) return;

    setSelectedStrategies(next);
    setQuerySelectionApplied(true);
  }, [querySelectionApplied, searchParams, strategies]);

  // ========== 恢复已有草案 ==========
  useEffect(() => {
    if (!activeVersionId) return;
    let cancelled = false;

    (async () => {
      const [start, end] = range;
      const plan_date_from = start.format('YYYY-MM-DD');
      const plan_date_to = end.format('YYYY-MM-DD');

      try {
        const resp = (await planApi.listStrategyDrafts({
          base_version_id: activeVersionId,
          plan_date_from,
          plan_date_to,
          status_filter: 'DRAFT',
          limit: 200,
        })) as ListStrategyDraftsResponse;

        if (cancelled) return;

        const next: Partial<Record<StrategyKey, StrategyDraftSummary>> = {};
        (resp?.drafts || []).forEach((d) => {
          next[String(d.strategy)] = d;
        });

        setDraftsByStrategy(next);
      } catch {
        // best-effort
      }
    })();

    return () => {
      cancelled = true;
    };
  }, [activeVersionId, range]);

  // ========== 操作方法 ==========
  const handleGenerate = useCallback(async () => {
    if (!activeVersionId) {
      message.warning('请先激活一个版本作为基准');
      return;
    }
    if (!selectedStrategies.length) {
      message.warning('请至少选择一种策略');
      return;
    }

    const [start, end] = range;
    const plan_date_from = start.format('YYYY-MM-DD');
    const plan_date_to = end.format('YYYY-MM-DD');

    setIsGenerating(true);
    try {
      const operator = currentUser || 'admin';
      const resp = (await planApi.generateStrategyDrafts({
        base_version_id: activeVersionId,
        plan_date_from,
        plan_date_to,
        strategies: selectedStrategies,
        operator,
      })) as GenerateStrategyDraftsResponse;

      const next: Partial<Record<StrategyKey, StrategyDraftSummary>> = {};
      (resp?.drafts || []).forEach((d) => {
        next[String(d.strategy)] = d;
      });

      setDraftsByStrategy(next);
      message.success(resp?.message || '策略草案生成完成');
    } finally {
      setIsGenerating(false);
    }
  }, [activeVersionId, currentUser, range, selectedStrategies]);

  const handleApply = useCallback(
    (draft: StrategyDraftSummary) => {
      if (!draft?.draft_id) return;

      Modal.confirm({
        title: '确认发布该草案？',
        content: '发布后将生成新的正式版本（会落库），默认不会自动激活；可在弹窗中选择仅切换或激活。',
        okText: '发布并生成版本',
        cancelText: '取消',
        onOk: async () => {
          setPublishingDraftId(draft.draft_id);
          try {
            const resp = (await planApi.applyStrategyDraft(
              draft.draft_id,
              currentUser || 'admin'
            )) as ApplyStrategyDraftResponse;
            message.success(resp.message || '已生成正式版本');
            const newVersionId = String(resp?.version_id ?? '').trim();
            if (newVersionId) {
              setCreatedVersionId(newVersionId);
              setPostPublishOpen(true);
            } else {
              navigate('/comparison?tab=historical');
            }
          } finally {
            setPublishingDraftId(null);
          }
        },
      });
    },
    [currentUser, navigate]
  );

  const openDetail = useCallback(async (draft: StrategyDraftSummary) => {
    if (!draft?.draft_id) return;
    setDetailDraft(draft);
    setDetailResp(null);
    setDetailFilter('ALL');
    setDetailSearch('');
    setSqueezedHintCache({});
    setDetailOpen(true);
    setDetailLoading(true);
    try {
      const resp = (await planApi.getStrategyDraftDetail(draft.draft_id)) as GetStrategyDraftDetailResponse;
      setDetailResp(resp);
    } catch (e: any) {
      message.error(e?.message || '加载变更明细失败');
    } finally {
      setDetailLoading(false);
    }
  }, []);

  const closeDetail = useCallback(() => {
    setDetailOpen(false);
    setDetailLoading(false);
    setDetailDraft(null);
    setDetailResp(null);
  }, []);

  const openMaterialDetail = useCallback(async (row: StrategyDraftDiffItem) => {
    const materialId = String(row?.material_id ?? '').trim();
    if (!materialId) return;

    setMaterialModalContext(row);
    setMaterialModalData(null);
    setMaterialModalError(null);
    setMaterialModalLogs([]);
    setMaterialModalLogsError(null);
    setMaterialModalOpen(true);
    setMaterialModalLoading(true);
    setMaterialModalLogsLoading(true);
    try {
      const endTime = dayjs().format('YYYY-MM-DD HH:mm:ss');
      const startTime = dayjs().subtract(30, 'day').format('YYYY-MM-DD HH:mm:ss');

      const [detailResp, logsResp] = await Promise.all([
        materialApi.getMaterialDetail(materialId),
        dashboardApi.listActionLogsByMaterial(materialId, startTime, endTime, 10).catch((e: any) => {
          setMaterialModalLogsError(e?.message || '加载失败');
          return [];
        }),
      ]);

      const normalized = normalizeMaterialDetail(detailResp);
      if (!normalized) {
        setMaterialModalError('未找到该物料（或无状态记录）');
        return;
      }
      setMaterialModalData(normalized);

      const logs = Array.isArray(logsResp) ? (logsResp as ActionLogRow[]) : [];
      setMaterialModalLogs(logs.slice(0, 10));
    } catch (e: any) {
      setMaterialModalError(e?.message || '加载物料详情失败');
    } finally {
      setMaterialModalLoading(false);
      setMaterialModalLogsLoading(false);
    }
  }, []);

  const closeMaterialModal = useCallback(() => {
    setMaterialModalOpen(false);
    setMaterialModalLoading(false);
    setMaterialModalContext(null);
    setMaterialModalData(null);
    setMaterialModalError(null);
    setMaterialModalLogsLoading(false);
    setMaterialModalLogsError(null);
    setMaterialModalLogs([]);
  }, []);

  const ensureSqueezedHint = useCallback(
    async (materialId: string) => {
      const id = String(materialId || '').trim();
      if (!id) return;

      setSqueezedHintCache((prev) => {
        const cur = prev[id];
        if (cur && (cur.status === 'loading' || cur.status === 'ready')) return prev;
        return { ...prev, [id]: { status: 'loading' } };
      });

      try {
        const raw = await materialApi.getMaterialDetail(id);
        const normalized = normalizeMaterialDetail(raw);
        if (!normalized) throw new Error('未找到该物料（或无状态记录）');

        const windowStart = range[0].format('YYYY-MM-DD');
        const windowEnd = range[1].format('YYYY-MM-DD');
        const sections = buildSqueezedOutHintSections(normalized.state, windowStart, windowEnd);

        setSqueezedHintCache((prev) => ({ ...prev, [id]: { status: 'ready', sections } }));
      } catch (e: any) {
        setSqueezedHintCache((prev) => ({ ...prev, [id]: { status: 'error', error: e?.message || '加载失败' } }));
      }
    },
    [range]
  );

  const handlePostSwitch = useCallback(async () => {
    if (!createdVersionId) return;
    setPostActionLoading('switch');
    try {
      setActiveVersion(createdVersionId);
      message.success('已切换到新版本（仅本地切换，未激活）');
      setPostPublishOpen(false);
      navigate('/workbench');
    } finally {
      setPostActionLoading(null);
    }
  }, [createdVersionId, navigate, setActiveVersion]);

  const handlePostActivate = useCallback(async () => {
    if (!createdVersionId) return;
    setPostActionLoading('activate');
    try {
      await planApi.activateVersion(createdVersionId, currentUser || 'admin');
      setActiveVersion(createdVersionId);
      message.success('已激活并切换到新版本（全局生效）');
      setPostPublishOpen(false);
      navigate('/workbench');
    } finally {
      setPostActionLoading(null);
    }
  }, [createdVersionId, currentUser, navigate, setActiveVersion]);

  const closePostPublish = useCallback(() => {
    setPostPublishOpen(false);
    setCreatedVersionId(null);
    setPostActionLoading(null);
  }, []);

  return {
    activeVersionId,
    currentUser,
    range,
    setRange,
    strategies,
    selectedStrategies,
    setSelectedStrategies,
    draftsByStrategy,
    isGenerating,
    publishingDraftId,
    hasAnyDraft,
    postPublishOpen,
    setPostPublishOpen,
    createdVersionId,
    postActionLoading,
    detailOpen,
    setDetailOpen,
    detailLoading,
    detailDraft,
    detailResp,
    detailFilter,
    setDetailFilter,
    detailSearch,
    setDetailSearch,
    detailItems,
    materialModalOpen,
    materialModalLoading,
    materialModalContext,
    materialModalData,
    materialModalError,
    materialModalLogsLoading,
    materialModalLogsError,
    materialModalLogs,
    squeezedHintCache,
    selectedStrategyKeysInOrder,
    headerHint,
    rangeDays,
    strategyTitleMap,
    recommendation,
    canGenerate,
    handleGenerate,
    handleApply,
    openDetail,
    closeDetail,
    openMaterialDetail,
    closeMaterialModal,
    ensureSqueezedHint,
    handlePostSwitch,
    handlePostActivate,
    closePostPublish,
    navigate,
  };
}
