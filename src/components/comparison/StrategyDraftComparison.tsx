import React, { useEffect, useMemo, useState } from 'react';
import { Alert, Button, Card, Checkbox, Col, DatePicker, Descriptions, Divider, Drawer, Empty, Input, Modal, Row, Segmented, Space, Spin, Table, Tag, Tooltip, Typography, message } from 'antd';
import type { Dayjs } from 'dayjs';
import dayjs from 'dayjs';
import { useNavigate, useSearchParams } from 'react-router-dom';
import { configApi, dashboardApi, materialApi, planApi } from '../../api/tauri';
import { useActiveVersionId, useCurrentUser, useGlobalActions } from '../../stores/use-global-store';

const { RangePicker } = DatePicker;
const { Text } = Typography;

type StrategyKey = string;

type StrategyPreset = {
  key: StrategyKey;
  title: string;
  description: string;
  kind?: 'preset' | 'custom';
  base_strategy?: string;
  strategy_id?: string;
  parameters?: any;
};

type CustomStrategyProfile = {
  strategy_id: string;
  title: string;
  description?: string | null;
  base_strategy: string;
  parameters?: any;
};

function makeCustomStrategyKey(strategyId: string): string {
  return `custom:${String(strategyId || '').trim()}`;
}

type StrategyDraftSummary = {
  draft_id: string;
  base_version_id: string;
  strategy: StrategyKey;
  plan_items_count: number;
  frozen_items_count: number;
  calc_items_count: number;
  mature_count: number;
  immature_count: number;
  total_capacity_used_t: number;
  overflow_days: number;
  moved_count: number;
  added_count: number;
  removed_count: number;
  squeezed_out_count: number;
  message: string;
};

type GenerateStrategyDraftsResponse = {
  base_version_id: string;
  plan_date_from: string;
  plan_date_to: string;
  drafts: StrategyDraftSummary[];
  message: string;
};

type ListStrategyDraftsResponse = GenerateStrategyDraftsResponse;

type ApplyStrategyDraftResponse = {
  version_id: string;
  success: boolean;
  message: string;
};

type StrategyDraftDiffItem = {
  material_id: string;
  change_type: 'MOVED' | 'ADDED' | 'SQUEEZED_OUT' | string;
  from_plan_date?: string | null;
  from_machine_code?: string | null;
  from_seq_no?: number | null;
  to_plan_date?: string | null;
  to_machine_code?: string | null;
  to_seq_no?: number | null;
  to_assign_reason?: string | null;
  to_urgent_level?: string | null;
  to_sched_state?: string | null;
  material_state_snapshot?: {
    sched_state?: string | null;
    urgent_level?: string | null;
    rush_level?: string | null;
    lock_flag?: boolean | null;
    force_release_flag?: boolean | null;
    manual_urgent_flag?: boolean | null;
    in_frozen_zone?: boolean | null;
    ready_in_days?: number | null;
    earliest_sched_date?: string | null;
    scheduled_date?: string | null;
    scheduled_machine_code?: string | null;
    seq_no?: number | null;
  } | null;
};

type GetStrategyDraftDetailResponse = {
  draft_id: string;
  base_version_id: string;
  plan_date_from: string;
  plan_date_to: string;
  strategy: StrategyKey;
  diff_items: StrategyDraftDiffItem[];
  diff_items_total: number;
  diff_items_truncated: boolean;
  message: string;
};

type MaterialDetailPayload = {
  master: any;
  state: any;
};

type ActionLogRow = {
  action_id: string;
  version_id: string;
  action_type: string;
  action_ts: string;
  actor: string;
  payload_json?: any;
  impact_summary_json?: any;
  machine_code?: string | null;
  date_range_start?: string | null;
  date_range_end?: string | null;
  detail?: string | null;
};

const FALLBACK_STRATEGIES: StrategyPreset[] = [
  { key: 'balanced', title: '均衡方案', description: '在交付/产能/库存之间保持均衡', kind: 'preset' },
  { key: 'urgent_first', title: '紧急优先', description: '优先保障 L3/L2 紧急订单', kind: 'preset' },
  { key: 'capacity_first', title: '产能优先', description: '优先提升产能利用率，减少溢出', kind: 'preset' },
  { key: 'cold_stock_first', title: '冷坨消化', description: '优先消化冷坨/压库物料', kind: 'preset' },
];

const MAX_DAYS = 60;

function clampRange(range: [Dayjs, Dayjs]): [Dayjs, Dayjs] {
  let [start, end] = range;
  start = start.startOf('day');
  end = end.startOf('day');
  if (end.isBefore(start)) {
    const tmp = start;
    start = end;
    end = tmp;
  }
  const days = end.diff(start, 'day') + 1;
  if (days > MAX_DAYS) {
    end = start.add(MAX_DAYS - 1, 'day');
  }
  return [start, end];
}

const StrategyDraftComparison: React.FC = () => {
  const navigate = useNavigate();
  const [searchParams] = useSearchParams();
  const activeVersionId = useActiveVersionId();
  const currentUser = useCurrentUser();
  const { setActiveVersion } = useGlobalActions();

  const [range, setRange] = useState<[Dayjs, Dayjs]>(() => [dayjs().startOf('day'), dayjs().add(6, 'day').startOf('day')]);
  const [strategies, setStrategies] = useState<StrategyPreset[]>(FALLBACK_STRATEGIES);
  const [selectedStrategies, setSelectedStrategies] = useState<StrategyKey[]>(FALLBACK_STRATEGIES.map((s) => s.key));
  const [querySelectionApplied, setQuerySelectionApplied] = useState(false);

  const [draftsByStrategy, setDraftsByStrategy] = useState<Partial<Record<StrategyKey, StrategyDraftSummary>>>({});
  const [isGenerating, setIsGenerating] = useState(false);
  const [publishingDraftId, setPublishingDraftId] = useState<string | null>(null);
  const [postPublishOpen, setPostPublishOpen] = useState(false);
  const [createdVersionId, setCreatedVersionId] = useState<string | null>(null);
  const [postActionLoading, setPostActionLoading] = useState<'switch' | 'activate' | null>(null);

  const [detailOpen, setDetailOpen] = useState(false);
  const [detailLoading, setDetailLoading] = useState(false);
  const [detailDraft, setDetailDraft] = useState<StrategyDraftSummary | null>(null);
  const [detailResp, setDetailResp] = useState<GetStrategyDraftDetailResponse | null>(null);
  const [detailFilter, setDetailFilter] = useState<'ALL' | 'MOVED' | 'ADDED' | 'SQUEEZED_OUT'>('ALL');
  const [detailSearch, setDetailSearch] = useState('');

  const [materialModalOpen, setMaterialModalOpen] = useState(false);
  const [materialModalLoading, setMaterialModalLoading] = useState(false);
  const [materialModalContext, setMaterialModalContext] = useState<StrategyDraftDiffItem | null>(null);
  const [materialModalData, setMaterialModalData] = useState<MaterialDetailPayload | null>(null);
  const [materialModalError, setMaterialModalError] = useState<string | null>(null);
  const [materialModalLogsLoading, setMaterialModalLogsLoading] = useState(false);
  const [materialModalLogsError, setMaterialModalLogsError] = useState<string | null>(null);
  const [materialModalLogs, setMaterialModalLogs] = useState<ActionLogRow[]>([]);

  const [squeezedHintCache, setSqueezedHintCache] = useState<
    Record<
      string,
      { status: 'loading' | 'ready' | 'error'; sections?: Array<{ title: string; lines: string[] }>; error?: string }
    >
  >({});

  const selectedStrategyKeysInOrder = useMemo(
    () => strategies.filter((s) => selectedStrategies.includes(s.key)).map((s) => s.key),
    [strategies, selectedStrategies],
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

  const formatTon = (v: any) => {
    const n = Number(v);
    if (!Number.isFinite(n)) return '—';
    return n.toFixed(1);
  };

  const formatPercent = (v: any) => {
    const n = Number(v);
    if (!Number.isFinite(n)) return '—';
    return `${(n * 100).toFixed(1)}%`;
  };

  const getMaturityRate = (d: StrategyDraftSummary) => {
    const denom = (d?.mature_count ?? 0) + (d?.immature_count ?? 0);
    if (!denom) return 0;
    return (d.mature_count ?? 0) / denom;
  };

  const recommendation = useMemo(() => {
    const candidates = selectedStrategyKeysInOrder
      .map((k) => draftsByStrategy[k])
      .filter((d): d is StrategyDraftSummary => Boolean(d?.draft_id));

    if (candidates.length < 2) return null;

    const sorted = [...candidates].sort((a, b) => {
      if ((a.overflow_days ?? 0) !== (b.overflow_days ?? 0)) return (a.overflow_days ?? 0) - (b.overflow_days ?? 0);
      if ((a.total_capacity_used_t ?? 0) !== (b.total_capacity_used_t ?? 0)) return (b.total_capacity_used_t ?? 0) - (a.total_capacity_used_t ?? 0);
      if ((a.squeezed_out_count ?? 0) !== (b.squeezed_out_count ?? 0)) return (a.squeezed_out_count ?? 0) - (b.squeezed_out_count ?? 0);
      return (b.mature_count ?? 0) - (a.mature_count ?? 0);
    });

    return sorted[0] || null;
  }, [draftsByStrategy, selectedStrategyKeysInOrder]);

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
          const desc = [
            `自定义策略（基于：${baseTitle}）`,
            p.description ? p.description : null,
          ]
            .filter(Boolean)
            .join(' · ');

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

  // 支持从外部页面（例如“设置中心 > 策略配置”）通过 URL 预选策略：
  // /comparison?tab=draft&strategies=balanced,custom:my_strategy_1
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

  const canGenerate = Boolean(activeVersionId) && selectedStrategies.length > 0 && !isGenerating;

  // 页面刷新/重启后：尝试从后端恢复“同基准版本 + 同日期范围”的最新草案
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
        // best-effort: 恢复失败不影响主流程（用户仍可重新生成）
      }
    })();

    return () => {
      cancelled = true;
    };
  }, [activeVersionId, range]);

  const handleGenerate = async () => {
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
  };

  const handleApply = async (draft: StrategyDraftSummary) => {
    if (!draft?.draft_id) return;

    Modal.confirm({
      title: '确认发布该草案？',
      content: '发布后将生成一个新的正式版本（会落库），可在历史版本中对比/激活。',
      okText: '发布并生成版本',
      cancelText: '取消',
      onOk: async () => {
        setPublishingDraftId(draft.draft_id);
        try {
          const resp = (await planApi.applyStrategyDraft(draft.draft_id, currentUser || 'admin')) as ApplyStrategyDraftResponse;
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
  };

  const hasAnyDraft = useMemo(() => Object.values(draftsByStrategy).some((d) => Boolean(d?.draft_id)), [draftsByStrategy]);

  const overviewRows = useMemo(() => {
    const machineDaysTotal = Math.max(1, rangeDays) * 3; // backend 目前固定 3 条机组：H032/H033/H034

    type RowDef = {
      key: string;
      label: string;
      better?: 'min' | 'max';
      getScore?: (d: StrategyDraftSummary) => number;
      render: (d: StrategyDraftSummary) => React.ReactNode;
    };

    const rows: RowDef[] = [
      {
        key: 'items',
        label: '排产项(冻结+新排)',
        render: (d) => `${d.plan_items_count} (${d.frozen_items_count}+${d.calc_items_count})`,
      },
      {
        key: 'capacity',
        label: '预计产量(t)',
        better: 'max',
        getScore: (d) => Number(d.total_capacity_used_t ?? 0),
        render: (d) => formatTon(d.total_capacity_used_t),
      },
      {
        key: 'overflow',
        label: '超限机组日',
        better: 'min',
        getScore: (d) => Number(d.overflow_days ?? 0),
        render: (d) => `${d.overflow_days} / ${machineDaysTotal}`,
      },
      {
        key: 'maturity',
        label: '成熟/未成熟(成熟率)',
        better: 'max',
        getScore: (d) => getMaturityRate(d),
        render: (d) => `${d.mature_count}/${d.immature_count} (${formatPercent(getMaturityRate(d))})`,
      },
      {
        key: 'squeezed',
        label: '挤出',
        better: 'min',
        getScore: (d) => Number(d.squeezed_out_count ?? 0),
        render: (d) => String(d.squeezed_out_count ?? 0),
      },
      {
        key: 'moved',
        label: '移动',
        better: 'min',
        getScore: (d) => Number(d.moved_count ?? 0),
        render: (d) => String(d.moved_count ?? 0),
      },
    ];

    return rows;
  }, [formatTon, formatPercent, getMaturityRate, rangeDays]);

  const kpiExtremaByRow = useMemo(() => {
    const result: Record<string, { best: number; worst: number } | null> = {};
    overviewRows.forEach((row) => {
      if (!row.getScore || !row.better) {
        result[row.key] = null;
        return;
      }
      const scores: number[] = selectedStrategyKeysInOrder
        .map((k) => draftsByStrategy[k])
        .filter((d): d is StrategyDraftSummary => Boolean(d?.draft_id))
        .map((d) => row.getScore!(d))
        .filter((n) => Number.isFinite(n));
      if (!scores.length) {
        result[row.key] = null;
        return;
      }
      const best = row.better === 'min' ? Math.min(...scores) : Math.max(...scores);
      const worst = row.better === 'min' ? Math.max(...scores) : Math.min(...scores);
      result[row.key] = { best, worst };
    });
    return result;
  }, [draftsByStrategy, overviewRows, selectedStrategyKeysInOrder]);

  const isSameNumber = (a: number, b: number) => Math.abs(a - b) < 1e-6;

  const formatPosition = (date?: string | null, machine?: string | null, seq?: number | null) => {
    const d = String(date ?? '').trim();
    const m = String(machine ?? '').trim();
    const s = seq == null ? '' : String(seq);
    if (!d && !m && !s) return '—';
    return `${m || '-'} / ${d || '-'} / #${s || '-'}`;
  };

  const formatBool = (v: any) => {
    if (v == null) return '—';
    return v ? '是' : '否';
  };

  const formatText = (v: any) => {
    if (v == null) return '—';
    const s = String(v).trim();
    return s ? s : '—';
  };

  const formatNumber = (v: any, digits = 2) => {
    const n = Number(v);
    if (!Number.isFinite(n)) return '—';
    return n.toFixed(digits);
  };

  const normalizeMaterialDetail = (raw: any): MaterialDetailPayload | null => {
    if (!raw) return null;
    if (Array.isArray(raw) && raw.length >= 2) {
      return { master: raw[0], state: raw[1] };
    }
    if (typeof raw === 'object') {
      // 兼容可能的后端形态（如果未来改为对象返回）
      const maybeMaster = (raw as any)?.master ?? (raw as any)?.material_master ?? (raw as any)?.[0];
      const maybeState = (raw as any)?.state ?? (raw as any)?.material_state ?? (raw as any)?.[1];
      if (maybeMaster && maybeState) return { master: maybeMaster, state: maybeState };
    }
    return null;
  };

  const prettyReason = (value: any) => {
    const raw = value == null ? '' : String(value).trim();
    if (!raw) return null;
    if ((raw.startsWith('{') && raw.endsWith('}')) || (raw.startsWith('[') && raw.endsWith(']'))) {
      try {
        return JSON.stringify(JSON.parse(raw), null, 2);
      } catch {
        return raw;
      }
    }
    return raw;
  };

  const renderSqueezedHintLine = (line: string, key: string) => {
    const text = String(line || '');
    const isBlocking = text.startsWith('窗口内不可') || text.includes('红线');
    return isBlocking ? (
      <Text key={key} type="danger" style={{ fontSize: 12 }}>
        {text}
      </Text>
    ) : (
      <Text key={key} style={{ fontSize: 12 }}>
        {text}
      </Text>
    );
  };

  const buildSqueezedOutHintSections = (state: any, windowStartDate: string, windowEndDate: string) => {
    const sections: Array<{ title: string; lines: string[] }> = [];
    const asBool = (v: any) => v === true || v === 1 || v === '1' || String(v).toLowerCase() === 'true';

    const notes: string[] = [
      `比较窗口：${windowStartDate} ~ ${windowEndDate}`,
      '挤出=草案窗口内未排入（若被排到窗口外，也会显示为挤出）',
      '说明：以下提示仅基于 material_state 字段（未额外查库）',
    ];

    sections.push({ title: '含义/范围', lines: notes });

    const key: string[] = [];
    const readyInDays = Number(state?.ready_in_days);
    if (Number.isFinite(readyInDays)) {
      if (readyInDays > 0) {
        const readyDate = dayjs().add(readyInDays, 'day').format('YYYY-MM-DD');
        if (dayjs(readyDate).isValid() && dayjs(windowEndDate).isValid() && dayjs(readyDate).isAfter(dayjs(windowEndDate), 'day')) {
          key.push(`窗口内不可适温：预计适温日 ${readyDate} > 窗口结束 ${windowEndDate}`);
        } else {
          key.push(`距适温：${readyInDays} 天（预计适温日：${readyDate}）`);
        }
      } else {
        key.push('已适温（ready_in_days <= 0）');
      }
    } else if (state?.is_mature != null) {
      key.push(asBool(state?.is_mature) ? '已适温（is_mature=1）' : '未适温（is_mature=0）');
    }

    const earliest = String(state?.earliest_sched_date ?? '').trim();
    if (earliest) {
      const end = dayjs(windowEndDate);
      const d = dayjs(earliest);
      if (d.isValid() && end.isValid() && d.isAfter(end, 'day')) {
        key.push(`窗口内不可排：最早可排 ${earliest} > 窗口结束 ${windowEndDate}`);
      } else {
        key.push(`最早可排：${earliest}`);
      }
    }

    if (key.length) sections.push({ title: '关键判断（窗口相关）', lines: key });

    const constraints: string[] = [];
    if (state?.in_frozen_zone != null) constraints.push(`冻结区：${asBool(state?.in_frozen_zone) ? '是' : '否'}${asBool(state?.in_frozen_zone) ? '（红线：冻结区不可变更）' : ''}`);
    if (state?.lock_flag != null) constraints.push(`锁定：${asBool(state?.lock_flag) ? '是' : '否'}`);
    if (state?.force_release_flag != null) constraints.push(`强制放行：${asBool(state?.force_release_flag) ? '是' : '否'}`);
    if (constraints.length) sections.push({ title: '限制/标志', lines: constraints });

    const urgency: string[] = [];
    const urgent = String(state?.urgent_level ?? '').trim();
    if (urgent) urgency.push(`紧急等级：${urgent}`);
    if (state?.manual_urgent_flag != null) urgency.push(`人工紧急：${asBool(state?.manual_urgent_flag) ? '是' : '否'}`);
    const rush = String(state?.rush_level ?? '').trim();
    if (rush) urgency.push(`催料等级：${rush}`);
    if (urgency.length) sections.push({ title: '紧急/优先', lines: urgency });

    const cur: string[] = [];
    const schedState = String(state?.sched_state ?? '').trim();
    if (schedState) cur.push(`排产状态：${schedState}`);
    const scheduledDate = String(state?.scheduled_date ?? '').trim();
    const scheduledMachine = String(state?.scheduled_machine_code ?? '').trim();
    const seqNo = state?.seq_no;
    if (scheduledDate || scheduledMachine) {
      cur.push(`已排（material_state）：${scheduledMachine || '-'} / ${scheduledDate || '-'} / #${seqNo ?? '-'}`);
    } else {
      cur.push('已排（material_state）：未排');
    }
    if (cur.length) sections.push({ title: '当前状态（material_state）', lines: cur });

    return sections;
  };

  const ensureSqueezedHint = async (materialId: string) => {
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
  };

  const openMaterialDetail = async (row: StrategyDraftDiffItem) => {
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
  };

  const detailItems = useMemo(() => {
    const items = Array.isArray(detailResp?.diff_items) ? detailResp!.diff_items : [];
    const q = detailSearch.trim().toLowerCase();
    return items.filter((it) => {
      if (detailFilter !== 'ALL' && String(it.change_type) !== detailFilter) return false;
      if (!q) return true;
      return String(it.material_id || '').toLowerCase().includes(q);
    });
  }, [detailResp, detailFilter, detailSearch]);

  const openDetail = async (draft: StrategyDraftSummary) => {
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
  };

  return (
    <div style={{ padding: 12 }}>
      <Card
        size="small"
        title={
          <Space>
            <span>策略草案对比</span>
            <Tag color="gold">草案</Tag>
          </Space>
        }
        extra={
          <Space>
            <Button size="small" onClick={() => navigate('/settings?tab=strategy')}>
              策略配置
            </Button>
            <Button size="small" onClick={() => navigate('/comparison?tab=historical')}>
              去历史版本对比
            </Button>
            <Button size="small" onClick={() => navigate('/workbench')}>
              返回工作台
            </Button>
          </Space>
        }
        style={{ marginBottom: 12 }}
      >
        <Space direction="vertical" style={{ width: '100%' }} size={10}>
          <Space wrap>
            <Text type="secondary">基准版本</Text>
            <Text code>{activeVersionId || '未激活'}</Text>
            {!activeVersionId && (
              <Button size="small" type="primary" onClick={() => navigate('/comparison?tab=historical')}>
                去激活版本
              </Button>
            )}
          </Space>

          <Space wrap>
            <Text type="secondary">计划范围</Text>
            <RangePicker
              value={range}
              onChange={(values) => {
                if (!values || !values[0] || !values[1]) return;
                const next = clampRange([values[0], values[1]]);
                if (next[1].diff(next[0], 'day') + 1 > MAX_DAYS) {
                  message.warning(`时间跨度过大，已限制为${MAX_DAYS}天`);
                }
                setRange(next);
              }}
              allowClear={false}
            />
            <Text type="secondary" style={{ fontSize: 12 }}>
              {headerHint}
            </Text>
          </Space>

          <Space wrap>
            <Text type="secondary">参与对比</Text>
            <Checkbox.Group
              value={selectedStrategies}
              onChange={(vals) => setSelectedStrategies(vals as StrategyKey[])}
              options={strategies.map((s) => ({
                value: s.key,
                label: (
                  <Space size={6}>
                    <span>{s.title}</span>
                    {s.kind === 'custom' ? <Tag color="gold">自定义</Tag> : null}
                    {s.kind === 'custom' && s.base_strategy ? (
                      <Tag color="blue">{strategyTitleMap[s.base_strategy] || s.base_strategy}</Tag>
                    ) : null}
                  </Space>
                ),
              }))}
            />
          </Space>

          <Space wrap>
            <Button
              type="primary"
              disabled={!canGenerate}
              loading={isGenerating}
              onClick={handleGenerate}
            >
              重新计算策略草案
            </Button>
            <Text type="secondary" style={{ fontSize: 12 }}>
              说明：草案为临时对象，不会写入数据库；确认后才生成正式版本。
            </Text>
            <Text type="secondary" style={{ fontSize: 12 }}>
              自定义策略：后端已支持“参数化排序”（仅影响等级内排序，不触碰冻结区/适温/产能硬约束）。
            </Text>
            <Text type="secondary" style={{ fontSize: 12 }}>
              KPI 说明：成熟/未成熟为计算过程统计；超限机组日按「机组×日期」统计。
            </Text>
          </Space>
        </Space>
      </Card>

      {hasAnyDraft && selectedStrategyKeysInOrder.length > 0 && (
        <Card size="small" title="KPI 总览（并排对比）" style={{ marginBottom: 12 }}>
          <Space direction="vertical" style={{ width: '100%' }} size={10}>
            {recommendation && (
              <Alert
                type={Number(recommendation.overflow_days ?? 0) > 0 ? 'warning' : 'info'}
                showIcon
                message={`建议优先考虑：${strategyTitleMap[recommendation.strategy] || recommendation.strategy}`}
                description={
                  <Text type="secondary">
                    超限机组日 {recommendation.overflow_days}，预计产量 {formatTon(recommendation.total_capacity_used_t)}t，
                    成熟/未成熟 {recommendation.mature_count}/{recommendation.immature_count}，挤出 {recommendation.squeezed_out_count}。
                    （仍建议人工复核关键订单/冻结区/风险点）
                  </Text>
                }
              />
            )}

            <div
              style={{
                display: 'grid',
                gridTemplateColumns: `160px repeat(${Math.max(1, selectedStrategyKeysInOrder.length)}, minmax(0, 1fr))`,
                gap: 8,
                alignItems: 'center',
              }}
            >
              <div />
              {selectedStrategyKeysInOrder.map((k) => {
                const title = strategyTitleMap[k] || k;
                const isRec = recommendation?.strategy === k;
                return (
                  <div key={`head-${k}`} style={{ display: 'flex', gap: 6, alignItems: 'center' }}>
                    <Text strong>{title}</Text>
                    {isRec ? <Tag color="blue">推荐</Tag> : null}
                    {Number(draftsByStrategy[k]?.overflow_days ?? 0) > 0 ? <Tag color="red">超限</Tag> : null}
                  </div>
                );
              })}

              {overviewRows.map((row) => {
                const extrema = kpiExtremaByRow[row.key];
                return (
                  <React.Fragment key={`row-${row.key}`}>
                    <Text type="secondary">{row.label}</Text>
                    {selectedStrategyKeysInOrder.map((k) => {
                      const draft = draftsByStrategy[k];
                      if (!draft?.draft_id) {
                        return (
                          <Text key={`cell-${row.key}-${k}`} type="secondary">
                            —
                          </Text>
                        );
                      }

                      const score = row.getScore ? row.getScore(draft) : null;
                      const isBest = extrema && score !== null ? isSameNumber(score, extrema.best) : false;
                      const isWorst = extrema && score !== null ? isSameNumber(score, extrema.worst) : false;

                      const cellStyle: React.CSSProperties = {
                        padding: '4px 6px',
                        borderRadius: 6,
                        background: isBest ? '#f6ffed' : isWorst ? '#fff2f0' : 'transparent',
                        border: isBest ? '1px solid #b7eb8f' : isWorst ? '1px solid #ffccc7' : '1px solid transparent',
                      };

                      return (
                        <div key={`cell-${row.key}-${k}`} style={cellStyle}>
                          <Text style={{ fontWeight: isBest ? 600 : 400 }}>{row.render(draft)}</Text>
                        </div>
                      );
                    })}
                  </React.Fragment>
                );
              })}
            </div>
          </Space>
        </Card>
      )}

      <Row gutter={[12, 12]}>
        {strategies.filter((s) => selectedStrategies.includes(s.key)).map((s) => {
          const draft = draftsByStrategy[s.key];
          const hasDraft = Boolean(draft?.draft_id);
          return (
          <Col key={s.key} xs={24} sm={12} lg={6}>
            <Card
              size="small"
              title={
                <Space size={6}>
                  <span>{s.title}</span>
                  {s.kind === 'custom' ? <Tag color="gold">自定义</Tag> : null}
                  {s.kind === 'custom' && s.base_strategy ? (
                    <Tag color="blue">{strategyTitleMap[s.base_strategy] || s.base_strategy}</Tag>
                  ) : null}
                </Space>
              }
              extra={
                hasDraft ? (
                  <Space size={6}>
                    <Tag color="green">已生成</Tag>
                    {Number(draft?.overflow_days ?? 0) > 0 ? <Tag color="red">超限</Tag> : null}
                  </Space>
                ) : (
                  <Tag color="default">未生成</Tag>
                )
              }
              style={{ height: '100%' }}
              actions={[
                <Button
                  key="select"
                  type="primary"
                  disabled={!hasDraft}
                  loading={publishingDraftId === draft?.draft_id}
                  onClick={() => draft && handleApply(draft)}
                >
                  选择该草案
                </Button>,
              ]}
            >
              <Space direction="vertical" style={{ width: '100%' }} size={8}>
                <Text type="secondary" style={{ fontSize: 12 }}>
                  {s.description}
                </Text>
                <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: 8 }}>
                  <div>
                    <Text type="secondary">排产项</Text>
                    <div style={{ fontWeight: 600 }}>{hasDraft ? draft?.plan_items_count : '—'}</div>
                  </div>
                  <div>
                    <Text type="secondary">预计产量(t)</Text>
                    <div style={{ fontWeight: 600 }}>{hasDraft ? formatTon(draft?.total_capacity_used_t) : '—'}</div>
                  </div>
                  <div>
                    <Text type="secondary">冻结</Text>
                    <div style={{ fontWeight: 600 }}>{hasDraft ? draft?.frozen_items_count : '—'}</div>
                  </div>
                  <div>
                    <Text type="secondary">新排</Text>
                    <div style={{ fontWeight: 600 }}>{hasDraft ? draft?.calc_items_count : '—'}</div>
                  </div>
                  <div>
                    <Text type="secondary">成熟/未成熟</Text>
                    <div style={{ fontWeight: 600 }}>
                      {hasDraft ? `${draft?.mature_count ?? 0}/${draft?.immature_count ?? 0}` : '—'}
                    </div>
                  </div>
                  <div>
                    <Text type="secondary">超限机组日</Text>
                    <div style={{ fontWeight: 600 }}>{hasDraft ? draft?.overflow_days : '—'}</div>
                  </div>
                </div>

                <Text type="secondary" style={{ fontSize: 12 }}>
                  变更：移动 {hasDraft ? draft?.moved_count : '—'} · 新增 {hasDraft ? draft?.added_count : '—'} · 挤出{' '}
                  {hasDraft ? draft?.squeezed_out_count : '—'}
                </Text>
                {hasDraft ? (
                  <Button
                    size="small"
                    type="link"
                    style={{ padding: 0, height: 'auto' }}
                    onClick={() => draft && openDetail(draft)}
                  >
                    查看变更明细
                  </Button>
                ) : null}

                {!activeVersionId ? (
                  <Alert type="info" showIcon message="未选择基准版本" description="请先激活一个版本再生成草案" />
                ) : hasDraft ? (
                  Number(draft?.overflow_days ?? 0) > 0 ? (
                    <Alert
                      type="warning"
                      showIcon
                      message="存在产能超限风险"
                      description={draft?.message || '可尝试“产能优先”或缩短窗口/调整产能配置后再生成'}
                    />
                  ) : (
                    <Alert
                      type="success"
                      showIcon
                      message="草案可用"
                      description={draft?.message || '可点击“选择该草案”生成正式版本'}
                    />
                  )
                ) : (
                  <Alert type="warning" showIcon message="尚未生成" description="点击“重新计算策略草案”后生成该策略的草案" />
                )}
              </Space>
            </Card>
          </Col>
        )})}
      </Row>

      <Modal
        title="草案已发布"
        open={postPublishOpen}
        onCancel={() => {
          setPostPublishOpen(false);
          setCreatedVersionId(null);
          setPostActionLoading(null);
        }}
        footer={[
          <Button
            key="later"
            onClick={() => {
              setPostPublishOpen(false);
              setCreatedVersionId(null);
              setPostActionLoading(null);
            }}
          >
            稍后
          </Button>,
          <Button
            key="historical"
            onClick={() => {
              setPostPublishOpen(false);
              setPostActionLoading(null);
              navigate('/comparison?tab=historical');
            }}
          >
            去历史版本对比
          </Button>,
          <Button
            key="switch"
            disabled={!createdVersionId}
            loading={postActionLoading === 'switch'}
            onClick={async () => {
              if (!createdVersionId) return;
              setPostActionLoading('switch');
              try {
                setActiveVersion(createdVersionId);
                message.success('已切换到新版本');
                setPostPublishOpen(false);
                navigate('/workbench');
              } finally {
                setPostActionLoading(null);
              }
            }}
          >
            去工作台继续
          </Button>,
          <Button
            key="activate"
            type="primary"
            disabled={!createdVersionId}
            loading={postActionLoading === 'activate'}
            onClick={async () => {
              if (!createdVersionId) return;
              setPostActionLoading('activate');
              try {
                await planApi.activateVersion(createdVersionId, currentUser || 'admin');
                setActiveVersion(createdVersionId);
                message.success('已激活并切换到新版本');
                setPostPublishOpen(false);
                navigate('/workbench');
              } finally {
                setPostActionLoading(null);
              }
            }}
          >
            去工作台并激活
          </Button>,
        ]}
      >
        <Space direction="vertical" style={{ width: '100%' }} size={10}>
          <Alert
            type="success"
            showIcon
            message="已生成正式版本"
            description={
              <Space direction="vertical" size={6}>
                <Text type="secondary">新版本ID</Text>
                <Text code>{createdVersionId || '-'}</Text>
                <Text type="secondary" style={{ fontSize: 12 }}>
                  你可以先切换到该版本继续在工作台做人工微调；也可以直接激活使其成为当前生效版本。
                </Text>
              </Space>
            }
          />
        </Space>
      </Modal>

      <Drawer
        title={
          <Space>
            <span>变更明细</span>
            {detailDraft ? <Tag color="blue">{strategyTitleMap[detailDraft.strategy] || detailDraft.strategy}</Tag> : null}
          </Space>
        }
        open={detailOpen}
        onClose={() => {
          setDetailOpen(false);
          setDetailLoading(false);
          setDetailDraft(null);
          setDetailResp(null);
        }}
        width={860}
        destroyOnClose
      >
        <Space direction="vertical" style={{ width: '100%' }} size={10}>
          {detailDraft ? (
            <div style={{ display: 'flex', flexWrap: 'wrap', gap: 8, alignItems: 'center' }}>
              <Text type="secondary">草案ID</Text>
              <Text code>{detailDraft.draft_id}</Text>
              <Text type="secondary">基准</Text>
              <Text code>{detailDraft.base_version_id}</Text>
              <Text type="secondary">窗口</Text>
              <Text code>
                {detailResp?.plan_date_from || '—'} ~ {detailResp?.plan_date_to || '—'}
              </Text>
              <Text type="secondary">移动</Text>
              <Text strong>{detailDraft.moved_count}</Text>
              <Text type="secondary">新增</Text>
              <Text strong>{detailDraft.added_count}</Text>
              <Text type="secondary">挤出</Text>
              <Text strong>{detailDraft.squeezed_out_count}</Text>
            </div>
          ) : null}

          {detailResp?.diff_items_truncated ? (
            <Alert
              type="warning"
              showIcon
              message="明细已截断"
              description={detailResp.message || `仅展示部分变更（${detailResp.diff_items.length}/${detailResp.diff_items_total}）`}
            />
          ) : null}

          <Space wrap>
            <Segmented
              value={detailFilter}
              onChange={(v) => setDetailFilter(v as any)}
              options={[
                { label: '全部', value: 'ALL' },
                { label: '移动', value: 'MOVED' },
                { label: '新增', value: 'ADDED' },
                { label: '挤出', value: 'SQUEEZED_OUT' },
              ]}
            />
            <Input
              allowClear
              placeholder="搜索 material_id"
              style={{ width: 240 }}
              value={detailSearch}
              onChange={(e) => setDetailSearch(e.target.value)}
            />
            <Text type="secondary" style={{ fontSize: 12 }}>
              {detailLoading ? '加载中…' : `共 ${detailItems.length} 条`}
            </Text>
          </Space>

          <Table
            size="small"
            rowKey={(r) => `${r.change_type}-${r.material_id}`}
            loading={detailLoading}
            pagination={{ pageSize: 20, showSizeChanger: true }}
            dataSource={detailItems}
            scroll={{ x: 980 }}
            columns={[
              {
                title: '变更',
                dataIndex: 'change_type',
                width: 90,
                render: (_: any, r: StrategyDraftDiffItem) => {
                  const t = String(r?.change_type || '');
                  const color = t === 'ADDED' ? 'green' : t === 'SQUEEZED_OUT' ? 'red' : 'blue';
                  const label = t === 'ADDED' ? '新增' : t === 'SQUEEZED_OUT' ? '挤出' : '移动';

                  if (t !== 'SQUEEZED_OUT') return <Tag color={color}>{label}</Tag>;

                  const id = String(r?.material_id || '').trim();
                  const cached = id ? squeezedHintCache[id] : undefined;
                  const snapshot = r?.material_state_snapshot ?? null;
                  const windowStart = range[0].format('YYYY-MM-DD');
                  const windowEnd = range[1].format('YYYY-MM-DD');
                  const snapshotSections = snapshot ? buildSqueezedOutHintSections(snapshot, windowStart, windowEnd) : [];

                  const titleNode = (
                    <Space direction="vertical" size={4}>
                      <Text strong style={{ fontSize: 12 }}>
                        挤出（窗口内未排入）
                      </Text>
                      <Text type="secondary" style={{ fontSize: 12 }}>
                        基线：{formatPosition(r.from_plan_date, r.from_machine_code, r.from_seq_no)}
                      </Text>
                      <Text type="secondary" style={{ fontSize: 12 }}>
                        提示：若草案把物料排到窗口外，也会显示为“挤出”
                      </Text>
                      {snapshotSections.length ? (
                        <Space direction="vertical" size={6}>
                          {snapshotSections
                            .filter((sec) => sec.lines && sec.lines.length)
                            .slice(0, 3)
                            .map((sec, secIdx) => (
                              <div key={`${sec.title}-${secIdx}`}>
                                <Text type="secondary" style={{ fontSize: 12 }}>
                                  {sec.title}
                                </Text>
                                <Space direction="vertical" size={2}>
                                  {sec.lines.slice(0, 4).map((line, idx) =>
                                    renderSqueezedHintLine(line, `${sec.title}-${idx}`)
                                  )}
                                </Space>
                              </div>
                            ))}
                        </Space>
                      ) : cached?.status === 'ready' ? (
                        (() => {
                          const sections = cached.sections || [];
                          let remaining = 10;
                          const nodes = sections
                            .map((sec) => {
                              if (remaining <= 0) return null;
                              const lines = sec.lines.slice(0, remaining);
                              remaining -= lines.length;
                              if (!lines.length) return null;
                              return (
                                <div key={sec.title}>
                                  <Text type="secondary" style={{ fontSize: 12 }}>
                                    {sec.title}
                                  </Text>
                                  <Space direction="vertical" size={2}>
                                    {lines.map((line, idx) => renderSqueezedHintLine(line, `${sec.title}-${idx}`))}
                                  </Space>
                                </div>
                              );
                            })
                            .filter(Boolean);

                          if (!nodes.length) {
                            return (
                              <Text type="secondary" style={{ fontSize: 12 }}>
                                暂无可提示的状态
                              </Text>
                            );
                          }
                          return <Space direction="vertical" size={6}>{nodes as any}</Space>;
                        })()
                      ) : cached?.status === 'error' ? (
                        <Text type="secondary" style={{ fontSize: 12 }}>
                          {cached.error || '加载失败'}
                        </Text>
                      ) : (
                        <Space>
                          <Spin size="small" />
                          <Text type="secondary" style={{ fontSize: 12 }}>
                            加载中…
                          </Text>
                        </Space>
                      )}
                      <Text type="secondary" style={{ fontSize: 12 }}>
                        点击 Material ID 可查看完整详情
                      </Text>
                    </Space>
                  );

                  return (
                    <Tooltip
                      title={titleNode}
                      onOpenChange={(open) => {
                        if (!open) return;
                        if (!id) return;
                        if (snapshot) return;
                        void ensureSqueezedHint(id);
                      }}
                    >
                      <Tag color={color}>{label}</Tag>
                    </Tooltip>
                  );
                },
              },
              {
                title: 'Material ID',
                dataIndex: 'material_id',
                width: 180,
                render: (_: any, r: StrategyDraftDiffItem) => (
                  <Button
                    type="link"
                    size="small"
                    style={{ padding: 0, height: 'auto' }}
                    onClick={() => openMaterialDetail(r)}
                  >
                    <Text code>{String(r?.material_id || '')}</Text>
                  </Button>
                ),
              },
              {
                title: 'From',
                key: 'from',
                width: 240,
                render: (_: any, r: StrategyDraftDiffItem) => (
                  <Text>{formatPosition(r.from_plan_date, r.from_machine_code, r.from_seq_no)}</Text>
                ),
              },
              {
                title: 'To',
                key: 'to',
                width: 240,
                render: (_: any, r: StrategyDraftDiffItem) => (
                  <Text>{formatPosition(r.to_plan_date, r.to_machine_code, r.to_seq_no)}</Text>
                ),
              },
              {
                title: '草案原因',
                key: 'reason',
                width: 220,
                render: (_: any, r: StrategyDraftDiffItem) => {
                  const reason = prettyReason(r.to_assign_reason);
                  const display = !reason
                    ? '—'
                    : reason.includes('\n')
                      ? `${reason.split('\n')[0]} …`
                      : reason;
                  return (
                    <Text
                      ellipsis={{
                        tooltip: reason ? (
                          <pre
                            style={{
                              margin: 0,
                              maxWidth: 640,
                              whiteSpace: 'pre-wrap',
                              fontSize: 12,
                            }}
                          >
                            {reason}
                          </pre>
                        ) : (
                          ''
                        ),
                      }}
                      copyable={reason ? { text: reason } : false}
                    >
                      {display}
                    </Text>
                  );
                },
              },
            ]}
          />
        </Space>
      </Drawer>

      <Modal
        title={
          <Space>
            <span>物料详情</span>
            {materialModalContext ? <Text code>{materialModalContext.material_id}</Text> : null}
            {materialModalContext?.change_type ? (
              <Tag color={String(materialModalContext.change_type) === 'ADDED' ? 'green' : String(materialModalContext.change_type) === 'SQUEEZED_OUT' ? 'red' : 'blue'}>
                {String(materialModalContext.change_type) === 'ADDED' ? '新增' : String(materialModalContext.change_type) === 'SQUEEZED_OUT' ? '挤出' : '移动'}
              </Tag>
            ) : null}
          </Space>
        }
        open={materialModalOpen}
        onCancel={() => {
          setMaterialModalOpen(false);
          setMaterialModalLoading(false);
          setMaterialModalContext(null);
          setMaterialModalData(null);
          setMaterialModalError(null);
          setMaterialModalLogsLoading(false);
          setMaterialModalLogsError(null);
          setMaterialModalLogs([]);
        }}
        footer={[
          <Button
            key="to-workbench"
            type="primary"
            disabled={!materialModalContext?.material_id}
            onClick={() => {
              const id = String(materialModalContext?.material_id ?? '').trim();
              if (!id) return;
              setMaterialModalOpen(false);
              setMaterialModalContext(null);
              setMaterialModalData(null);
              setMaterialModalError(null);
              setMaterialModalLogsLoading(false);
              setMaterialModalLogsError(null);
              setMaterialModalLogs([]);
              navigate(`/workbench?material_id=${encodeURIComponent(id)}`);
            }}
          >
            去工作台查看
          </Button>,
          <Button
            key="close"
            onClick={() => {
              setMaterialModalOpen(false);
              setMaterialModalLoading(false);
              setMaterialModalContext(null);
              setMaterialModalData(null);
              setMaterialModalError(null);
              setMaterialModalLogsLoading(false);
              setMaterialModalLogsError(null);
              setMaterialModalLogs([]);
            }}
          >
            关闭
          </Button>,
        ]}
        width={760}
        destroyOnClose
      >
        {materialModalLoading ? (
          <div style={{ padding: 24, textAlign: 'center' }}>
            <Spin tip="加载中…" />
          </div>
        ) : materialModalError ? (
          <Alert type="error" showIcon message="加载失败" description={materialModalError} />
        ) : materialModalData ? (
          <Space direction="vertical" style={{ width: '100%' }} size={12}>
            {materialModalContext ? (
              <Alert
                type="info"
                showIcon
                message="本次变更位置"
                description={
                  <Space direction="vertical" size={4}>
                    <Text type="secondary">From</Text>
                    <Text>{formatPosition(materialModalContext.from_plan_date, materialModalContext.from_machine_code, materialModalContext.from_seq_no)}</Text>
                    <Text type="secondary">To</Text>
                    <Text>
                      {String(materialModalContext.change_type) === 'SQUEEZED_OUT'
                        ? '未安排（挤出）'
                        : formatPosition(materialModalContext.to_plan_date, materialModalContext.to_machine_code, materialModalContext.to_seq_no)}
                    </Text>
                    <Text type="secondary">草案原因</Text>
                    {(() => {
                      const reason = prettyReason(materialModalContext.to_assign_reason);
                      if (!reason) return <Text>—</Text>;
                      if (reason.includes('\n')) {
                        return (
                          <pre
                            style={{
                              margin: 0,
                              padding: 12,
                              borderRadius: 6,
                              border: '1px solid #f0f0f0',
                              background: '#fafafa',
                              whiteSpace: 'pre-wrap',
                              fontSize: 12,
                            }}
                          >
                            {reason}
                          </pre>
                        );
                      }
                      return <Text>{reason}</Text>;
                    })()}
                    <Text type="secondary">草案快照</Text>
                    <Text>
                      {formatText(
                        [materialModalContext.to_urgent_level, materialModalContext.to_sched_state]
                          .map((v) => (v == null ? '' : String(v).trim()))
                          .filter(Boolean)
                          .join(' / ')
                      )}
                    </Text>
                    {String(materialModalContext.change_type) === 'SQUEEZED_OUT' ? (
                      <>
                        <Text type="secondary">挤出提示（基于物料状态，不做臆测）</Text>
                        {(() => {
                          const state = materialModalData?.state;
                          const windowStart = range[0].format('YYYY-MM-DD');
                          const windowEnd = range[1].format('YYYY-MM-DD');
                          const sections = state ? buildSqueezedOutHintSections(state, windowStart, windowEnd) : [];
                          if (!sections.length) return <Text>—</Text>;
                          return (
                            <Space direction="vertical" size={10} style={{ width: '100%' }}>
                              {sections.map((sec, secIdx) => (
                                <div key={`${sec.title}-${secIdx}`}>
                                  <Text type="secondary" style={{ fontSize: 12 }}>
                                    {sec.title}
                                  </Text>
                                  <Space direction="vertical" size={2} style={{ marginTop: 4 }}>
                                    {sec.lines.map((line, idx) => renderSqueezedHintLine(line, `${sec.title}-${idx}`))}
                                  </Space>
                                </div>
                              ))}
                            </Space>
                          );
                        })()}
                      </>
                    ) : null}
                  </Space>
                }
              />
            ) : null}

            <div>
              <Text strong>物料信息</Text>
              <Divider style={{ margin: '8px 0' }} />
              <Descriptions size="small" column={2} bordered>
                <Descriptions.Item label="材料号">
                  <Text code copyable>
                    {formatText(materialModalData.master?.material_id || materialModalData.state?.material_id)}
                  </Text>
                </Descriptions.Item>
                <Descriptions.Item label="钢种">
                  {formatText(materialModalData.master?.steel_mark)}
                </Descriptions.Item>
                <Descriptions.Item label="重量(t)">
                  {formatNumber(materialModalData.master?.weight_t, 3)}
                </Descriptions.Item>
                <Descriptions.Item label="交期">
                  {formatText(materialModalData.master?.due_date)}
                </Descriptions.Item>
                <Descriptions.Item label="下道机组">
                  {formatText(materialModalData.master?.next_machine_code || materialModalData.master?.current_machine_code)}
                </Descriptions.Item>
                <Descriptions.Item label="库存天数">
                  {formatText(materialModalData.state?.stock_age_days ?? materialModalData.master?.stock_age_days)}
                </Descriptions.Item>
              </Descriptions>
            </div>

            <div>
              <Text strong>状态/原因</Text>
              <Divider style={{ margin: '8px 0' }} />
              <Descriptions size="small" column={2} bordered>
                <Descriptions.Item label="排产状态">
                  {formatText(materialModalData.state?.sched_state)}
                </Descriptions.Item>
                <Descriptions.Item label="紧急等级">
                  {formatText(materialModalData.state?.urgent_level)}
                </Descriptions.Item>
                <Descriptions.Item label="锁定">
                  {formatBool(materialModalData.state?.lock_flag)}
                </Descriptions.Item>
                <Descriptions.Item label="人工紧急">
                  {formatBool(materialModalData.state?.manual_urgent_flag)}
                </Descriptions.Item>
                <Descriptions.Item label="强制放行">
                  {formatBool(materialModalData.state?.force_release_flag)}
                </Descriptions.Item>
                <Descriptions.Item label="距适温(天)">
                  {formatText(materialModalData.state?.ready_in_days)}
                </Descriptions.Item>
                <Descriptions.Item label="最早可排">
                  {formatText(materialModalData.state?.earliest_sched_date)}
                </Descriptions.Item>
                <Descriptions.Item label="冻结区">
                  {formatBool(materialModalData.state?.in_frozen_zone)}
                </Descriptions.Item>
                <Descriptions.Item label="已排日期">
                  {formatText(materialModalData.state?.scheduled_date)}
                </Descriptions.Item>
                <Descriptions.Item label="已排机组/序号">
                  {formatText(
                    materialModalData.state?.scheduled_machine_code
                      ? `${materialModalData.state.scheduled_machine_code} / #${materialModalData.state?.seq_no ?? '-'}`
                      : '—'
                  )}
                </Descriptions.Item>
              </Descriptions>

              <div style={{ marginTop: 12 }}>
                <Text type="secondary">紧急原因（urgent_reason）</Text>
                <div style={{ marginTop: 6 }}>
                  {prettyReason(materialModalData.state?.urgent_reason) ? (
                    <pre
                      style={{
                        margin: 0,
                        padding: 12,
                        borderRadius: 6,
                        border: '1px solid #f0f0f0',
                        background: '#fafafa',
                        whiteSpace: 'pre-wrap',
                        fontSize: 12,
                      }}
                    >
                      {prettyReason(materialModalData.state?.urgent_reason)}
                    </pre>
                  ) : (
                    <Empty description="暂无原因信息" image={Empty.PRESENTED_IMAGE_SIMPLE} />
                  )}
                </div>
              </div>

              <div style={{ marginTop: 12 }}>
                <Text type="secondary">最近相关操作（30天）</Text>
                <div style={{ marginTop: 6 }}>
                  {materialModalLogsLoading ? (
                    <div style={{ padding: 12, textAlign: 'center' }}>
                      <Spin size="small" tip="加载操作历史…" />
                    </div>
                  ) : materialModalLogsError ? (
                    <Alert type="warning" showIcon message="操作历史加载失败" description={materialModalLogsError} />
                  ) : materialModalLogs.length ? (
                    <Table
                      size="small"
                      rowKey="action_id"
                      pagination={false}
                      dataSource={materialModalLogs}
                      columns={[
                        {
                          title: '时间',
                          dataIndex: 'action_ts',
                          width: 160,
                          render: (v: string) => <Text>{String(v || '')}</Text>,
                        },
                        {
                          title: '类型',
                          dataIndex: 'action_type',
                          width: 160,
                          render: (v: string) => <Tag>{String(v || '')}</Tag>,
                        },
                        {
                          title: '操作人',
                          dataIndex: 'actor',
                          width: 120,
                          render: (v: string) => <Text>{String(v || '')}</Text>,
                        },
                        {
                          title: '详情',
                          dataIndex: 'detail',
                          render: (v: string) => (
                            <Text ellipsis={{ tooltip: v ? String(v) : '' }}>{v ? String(v) : '—'}</Text>
                          ),
                        },
                      ]}
                    />
                  ) : (
                    <Empty description="暂无相关操作" image={Empty.PRESENTED_IMAGE_SIMPLE} />
                  )}
                </div>
              </div>
            </div>
          </Space>
        ) : (
          <Empty description="暂无数据" />
        )}
      </Modal>
    </div>
  );
};

export default StrategyDraftComparison;
