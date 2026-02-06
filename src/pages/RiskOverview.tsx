import React, { useCallback, useEffect, useMemo } from 'react';
import { Alert, Button, Space, Tag } from 'antd';
import { useNavigate, useSearchParams } from 'react-router-dom';
import ErrorBoundary from '../components/ErrorBoundary';
import NoActiveVersionGuide from '../components/NoActiveVersionGuide';
import DecisionFlowGuide from '../components/flow/DecisionFlowGuide';
import KPIBand from '../components/overview/KPIBand';
import DimensionTabs, { DimensionTabKey } from '../components/overview/DimensionTabs';
import DrilldownDrawer from '../components/overview/DrilldownDrawer';
import { useOnlineStatus } from '../hooks/useOnlineStatus';
import { useRiskOverviewData } from '../hooks/useRiskOverviewData';
import type { DrilldownSpec, RiskProblem, WorkbenchTabKey } from '../hooks/useRiskOverviewData';
import { useActiveVersionId, useGlobalActions } from '../stores/use-global-store';
import type { AgeBin, PressureLevel, UrgencyLevel } from '../types/decision';

function normalizeDimensionTabKey(value: string | null): DimensionTabKey {
  const aliases: Record<string, DimensionTabKey> = {
    issues: 'issues',
    orders: 'orders',
    capacity: 'capacity',
    inventory: 'inventory',
    roll: 'roll',
  };

  if (!value) return 'issues';
  return aliases[value] || 'issues';
}

const DRILLDOWN_KEYS = {
  kind: 'dd',
  urgency: 'urgency',
  machine: 'machine',
  date: 'date',
  ageBin: 'age',
  pressure: 'pressure',
} as const;

const URGENCY_LEVELS: UrgencyLevel[] = ['L0', 'L1', 'L2', 'L3'];
const AGE_BINS: AgeBin[] = ['0-7', '8-14', '15-30', '30+'];
const PRESSURE_LEVELS: PressureLevel[] = ['LOW', 'MEDIUM', 'HIGH', 'CRITICAL'];

function parseDrilldownSpec(params: URLSearchParams): DrilldownSpec | null {
  const kind = params.get(DRILLDOWN_KEYS.kind);
  if (!kind) return null;

  if (kind === 'orders') {
    const raw = params.get(DRILLDOWN_KEYS.urgency);
    const urgency = raw && URGENCY_LEVELS.includes(raw as UrgencyLevel) ? (raw as UrgencyLevel) : undefined;
    return urgency ? { kind: 'orders', urgency } : { kind: 'orders' };
  }

  if (kind === 'coldStock') {
    const machineCode = params.get(DRILLDOWN_KEYS.machine) || undefined;
    const ageRaw = params.get(DRILLDOWN_KEYS.ageBin);
    const ageBin = ageRaw && AGE_BINS.includes(ageRaw as AgeBin) ? (ageRaw as AgeBin) : undefined;
    const pressureRaw = params.get(DRILLDOWN_KEYS.pressure);
    const pressureLevel =
      pressureRaw && PRESSURE_LEVELS.includes(pressureRaw.toUpperCase() as PressureLevel)
        ? (pressureRaw.toUpperCase() as PressureLevel)
        : undefined;
    return machineCode || ageBin || pressureLevel
      ? { kind: 'coldStock', machineCode, ageBin, pressureLevel }
      : { kind: 'coldStock' };
  }

  if (kind === 'roll') {
    const machineCode = params.get(DRILLDOWN_KEYS.machine) || undefined;
    return machineCode ? { kind: 'roll', machineCode } : { kind: 'roll' };
  }

  if (kind === 'risk') {
    const planDate = params.get(DRILLDOWN_KEYS.date) || undefined;
    return planDate ? { kind: 'risk', planDate } : { kind: 'risk' };
  }

  if (kind === 'bottleneck') {
    const machineCode = params.get(DRILLDOWN_KEYS.machine) || undefined;
    const planDate = params.get(DRILLDOWN_KEYS.date) || undefined;
    return machineCode || planDate ? { kind: 'bottleneck', machineCode, planDate } : { kind: 'bottleneck' };
  }

  if (kind === 'capacityOpportunity') {
    const machineCode = params.get(DRILLDOWN_KEYS.machine) || undefined;
    const planDate = params.get(DRILLDOWN_KEYS.date) || undefined;
    return machineCode || planDate ? { kind: 'capacityOpportunity', machineCode, planDate } : { kind: 'capacityOpportunity' };
  }

  return null;
}

function writeDrilldownParams(next: URLSearchParams, spec: DrilldownSpec) {
  next.set(DRILLDOWN_KEYS.kind, spec.kind);
  next.delete(DRILLDOWN_KEYS.urgency);
  next.delete(DRILLDOWN_KEYS.machine);
  next.delete(DRILLDOWN_KEYS.date);
  next.delete(DRILLDOWN_KEYS.ageBin);
  next.delete(DRILLDOWN_KEYS.pressure);

  if (spec.kind === 'orders' && spec.urgency) {
    next.set(DRILLDOWN_KEYS.urgency, spec.urgency);
  }
  if (spec.kind === 'coldStock') {
    if (spec.machineCode) next.set(DRILLDOWN_KEYS.machine, spec.machineCode);
    if (spec.ageBin) next.set(DRILLDOWN_KEYS.ageBin, spec.ageBin);
    if (spec.pressureLevel) next.set(DRILLDOWN_KEYS.pressure, spec.pressureLevel);
  }
  if (spec.kind === 'roll' && spec.machineCode) {
    next.set(DRILLDOWN_KEYS.machine, spec.machineCode);
  }
  if (spec.kind === 'risk' && spec.planDate) {
    next.set(DRILLDOWN_KEYS.date, spec.planDate);
  }
  if (spec.kind === 'bottleneck') {
    if (spec.machineCode) next.set(DRILLDOWN_KEYS.machine, spec.machineCode);
    if (spec.planDate) next.set(DRILLDOWN_KEYS.date, spec.planDate);
  }
  if (spec.kind === 'capacityOpportunity') {
    if (spec.machineCode) next.set(DRILLDOWN_KEYS.machine, spec.machineCode);
    if (spec.planDate) next.set(DRILLDOWN_KEYS.date, spec.planDate);
  }
}

function clearDrilldownParams(next: URLSearchParams) {
  next.delete(DRILLDOWN_KEYS.kind);
  next.delete(DRILLDOWN_KEYS.urgency);
  next.delete(DRILLDOWN_KEYS.machine);
  next.delete(DRILLDOWN_KEYS.date);
  next.delete(DRILLDOWN_KEYS.ageBin);
  next.delete(DRILLDOWN_KEYS.pressure);
}

const RiskOverview: React.FC = () => {
  const navigate = useNavigate();
  const [searchParams, setSearchParams] = useSearchParams();
  const isOnline = useOnlineStatus();
  const activeVersionId = useActiveVersionId();
  const { setWorkbenchFilters, setWorkbenchViewMode } = useGlobalActions();
  const data = useRiskOverviewData(activeVersionId);

  // C1修复：过滤出高压力和严重压力的冷坨数据，避免DrilldownDrawer显示无关数据
  const severeColdStockBuckets = useMemo(() =>
    data.coldStockBuckets.filter(b =>
      b.pressureLevel === 'HIGH' || b.pressureLevel === 'CRITICAL'
    ), [data.coldStockBuckets]);

  const activeTab = useMemo(
    () => normalizeDimensionTabKey(searchParams.get('tab')),
    [searchParams]
  );

  const setTab = (key: DimensionTabKey) => {
    const next = new URLSearchParams(searchParams);
    next.set('tab', key);
    setSearchParams(next, { replace: true });
  };

  const drawerSpec = useMemo(() => parseDrilldownSpec(searchParams), [searchParams]);
  const drawerOpen = !!drawerSpec;

  // When a drilldown deep-link is opened without an explicit tab, choose the most relevant dimension tab.
  useEffect(() => {
    if (!drawerSpec) return;
    if (searchParams.get('tab')) return;

    const impliedTab: DimensionTabKey =
      drawerSpec.kind === 'orders'
        ? 'orders'
        : drawerSpec.kind === 'coldStock'
          ? 'inventory'
          : drawerSpec.kind === 'roll'
            ? 'roll'
            : drawerSpec.kind === 'risk' || drawerSpec.kind === 'bottleneck' || drawerSpec.kind === 'capacityOpportunity'
              ? 'capacity'
              : 'issues';

    const next = new URLSearchParams(searchParams);
    next.set('tab', impliedTab);
    setSearchParams(next, { replace: true });
  }, [drawerSpec, searchParams, setSearchParams]);

  const openDrilldown = (spec: DrilldownSpec) => {
    const next = new URLSearchParams(searchParams);
    writeDrilldownParams(next, spec);
    setSearchParams(next);
  };

  const closeDrilldown = () => {
    const next = new URLSearchParams(searchParams);
    clearDrilldownParams(next);
    // Closing via UI should not pollute history (back button shouldn't re-open the drawer).
    setSearchParams(next, { replace: true });
  };

  const goWorkbenchWith = useCallback((opts: {
    workbenchTab?: WorkbenchTabKey;
    machineCode?: string | null;
    urgencyLevel?: string | null;
    planDate?: string | null;
    context?: string;
  }) => {
    // 从风险概览进入工作台，默认回到"全局视角"，并带上当前问题的关键信息（如机组、紧急度）。
    setWorkbenchFilters({
      machineCode: opts.machineCode ?? null,
      urgencyLevel: opts.urgencyLevel ?? null,
      lockStatus: 'ALL',
    });

    // 轻量联动：从不同问题入口进入工作台时，默认切到更匹配的视图。
    // - risk/bottleneck/capacityOpportunity：默认甘特图（便于定位到日期列并打开同日明细）
    // - visualization/roll：倾向甘特图
    // - materials：倾向卡片
    const context = String(opts.context || '').trim();
    const isCellContext = context === 'risk' || context === 'bottleneck' || context === 'capacityOpportunity';
    if (opts.workbenchTab === 'materials') {
      setWorkbenchViewMode('CARD');
    } else if (opts.workbenchTab === 'capacity' || opts.workbenchTab === 'visualization' || isCellContext) {
      setWorkbenchViewMode('GANTT');
    }

    // 构建深链接URL参数（第三阶段：风险概览深链接）
    const params = new URLSearchParams();
    if (opts.machineCode) params.set('machine', opts.machineCode);
    if (opts.urgencyLevel) params.set('urgency', opts.urgencyLevel);
    if (opts.planDate) params.set('date', opts.planDate);
    if (opts.context) params.set('context', opts.context);

    // 风险日/瓶颈点/机会点：定位到日期列并自动打开该单元格明细（提升问题直达效率）
    if (isCellContext) {
      params.set('focus', 'gantt');
      if (opts.machineCode && opts.planDate) {
        params.set('openCell', '1');
      }
    }

    const url = params.toString() ? `/workbench?${params.toString()}` : '/workbench';
    navigate(url);
  }, [navigate, setWorkbenchFilters, setWorkbenchViewMode]);

  const goWorkbench = (problem: RiskProblem) => {
    const urgency =
      problem.drilldown.kind === 'orders' && problem.drilldown.urgency
        ? problem.drilldown.urgency
        : null;

    // 提取planDate（日期信息）从不同类型的drilldown
    const planDate =
      problem.drilldown.kind === 'risk' && problem.drilldown.planDate
        ? problem.drilldown.planDate
        : problem.drilldown.kind === 'bottleneck' && problem.drilldown.planDate
        ? problem.drilldown.planDate
        : problem.drilldown.kind === 'capacityOpportunity' && problem.drilldown.planDate
        ? problem.drilldown.planDate
        : null;

    // 构建上下文字符串（用于显示来源提示）
    const context = problem.drilldown.kind;

    goWorkbenchWith({
      workbenchTab: problem.workbenchTab,
      machineCode: problem.workbenchMachineCode ?? null,
      urgencyLevel: urgency,
      planDate,
      context,
    });
  };

  if (!activeVersionId) {
    return (
      <NoActiveVersionGuide
        title="尚无激活的排产版本"
        description="风险概览需要一个激活的排产版本作为基础"
        onNavigateToImport={() => navigate('/import')}
        onNavigateToPlan={() => navigate('/comparison')}
      />
    );
  }

  const drawerError = drawerSpec ? data.errorByKind[drawerSpec.kind] : undefined;
  const drawerLoading = drawerSpec ? data.loadingByKind[drawerSpec.kind] : false;

  const problemCounts = useMemo(() => {
    const out = { P0: 0, P1: 0, P2: 0, P3: 0 } as Record<string, number>;
    (data.problems || []).forEach((p) => {
      const key = String(p?.severity || '');
      if (!key) return;
      out[key] = (out[key] || 0) + 1;
    });
    return out;
  }, [data.problems]);

  const recommendedProblem = useMemo(() => {
    return (
      data.problems.find((p) => p.severity === 'P0') ||
      data.problems.find((p) => p.severity === 'P1') ||
      null
    );
  }, [data.problems]);

  return (
    <ErrorBoundary>
      <Space direction="vertical" size={12} style={{ width: '100%' }}>
        {!isOnline && (
          <Alert type="warning" showIcon message="当前处于离线状态，数据可能无法刷新" />
        )}

        {data.errors.length > 0 && (
          <Alert
            type="error"
            showIcon
            message="部分数据加载失败"
            action={
              <Button size="small" onClick={data.refetchAll}>
                重试
              </Button>
            }
          />
        )}

        <DecisionFlowGuide
          stage="overview"
          title={
            recommendedProblem
              ? `下一步：去工作台处理「${recommendedProblem.title}」`
              : '下一步：去工作台处理关键物料'
          }
          tags={
            <Space wrap size={6}>
              {problemCounts.P0 > 0 ? <Tag color="red">P0 {problemCounts.P0}</Tag> : null}
              {problemCounts.P1 > 0 ? <Tag color="orange">P1 {problemCounts.P1}</Tag> : null}
            </Space>
          }
          description="处理完后建议到「策略草案对比」生成多方案预览，选择推荐方案发布并激活。"
          primaryAction={{
            label: '去工作台',
            onClick: () => {
              if (recommendedProblem) {
                goWorkbench(recommendedProblem);
                return;
              }
              navigate('/workbench');
            },
          }}
        />

        <KPIBand
          loading={data.loadingByKind.kpi}
          kpi={data.kpi}
          onOpenDrilldown={openDrilldown}
          onGoWorkbench={goWorkbenchWith}
        />

        <DimensionTabs
          activeKey={activeTab}
          onChange={setTab}
          loading={data.isLoading}
          problems={data.problems}
          onOpenDrilldown={openDrilldown}
          onGoWorkbench={goWorkbench}
        />

        <DrilldownDrawer
          open={drawerOpen}
          onClose={closeDrilldown}
          spec={drawerSpec}
          loading={drawerLoading}
          error={drawerError}
          onRetry={data.refetchAll}
          onGoWorkbench={goWorkbenchWith}
          riskDays={data.riskDays}
          bottlenecks={data.bottlenecks}
          orderFailures={data.orderFailures}
          coldStockBuckets={severeColdStockBuckets}
          rollAlerts={data.rollAlerts}
          capacityOpportunities={data.capacityOpportunities}
        />
      </Space>
    </ErrorBoundary>
  );
};

export default RiskOverview;
