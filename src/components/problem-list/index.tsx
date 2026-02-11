/**
 * ProblemList 问题列表主组件
 *
 * 重构后：421 行 → ~85 行 (-80%)
 */

import React, { useEffect, useMemo, useRef, useState } from 'react';
import { Card, Empty, Grid, Segmented, Space, Tag, Typography, theme } from 'antd';
import type { ProblemListProps, ProblemSeverity, RiskProblem } from './types';
import { severityMeta } from './types';
import { ProblemCard } from './ProblemCard';

const { Text } = Typography;

type ProblemSyncDelta = {
  resolved: number;
  improved: number;
  worsened: number;
  updated: number;
  added: number;
};

const SEVERITY_RANK: Record<ProblemSeverity, number> = {
  P0: 0,
  P1: 1,
  P2: 2,
  P3: 3,
};

function normalizeText(value: unknown): string {
  return String(value ?? '').trim();
}

const previousProblemSnapshots = new Map<string, RiskProblem[]>();

const ProblemList: React.FC<ProblemListProps> = ({
  loading,
  snapshotKey,
  problems,
  onOpenDrilldown,
  onGoWorkbench,
}) => {
  const { token } = theme.useToken();
  const screens = Grid.useBreakpoint();
  const compact = !screens.md;
  const previousProblemsRef = useRef<ProblemListProps['problems'] | null>(null);
  const [syncDelta, setSyncDelta] = useState<ProblemSyncDelta | null>(null);
  const snapshotScopeKey = normalizeText(snapshotKey) || 'default';

  const [scope, setScope] = useState<'ALL' | 'P0_P1'>('ALL');
  const list = useMemo(() => {
    if (scope === 'P0_P1') return problems.filter((p) => p.severity === 'P0' || p.severity === 'P1');
    return problems;
  }, [problems, scope]);

  const counts = useMemo(() => {
    const out: Record<ProblemSeverity, number> = { P0: 0, P1: 0, P2: 0, P3: 0 };
    problems.forEach((p) => {
      out[p.severity] = (out[p.severity] || 0) + 1;
    });
    return out;
  }, [problems]);

  const [entered, setEntered] = useState(false);
  useEffect(() => {
    setEntered(false);
    const id = requestAnimationFrame(() => setEntered(true));
    return () => cancelAnimationFrame(id);
  }, [scope, problems.length]);

  useEffect(() => {
    const prevFromScope = previousProblemSnapshots.get(snapshotScopeKey) ?? null;
    const prev = previousProblemsRef.current ?? prevFromScope;
    if (!prev) {
      previousProblemsRef.current = problems;
      previousProblemSnapshots.set(snapshotScopeKey, problems);
      return;
    }

    const prevById = new Map(prev.map((p) => [p.id, p]));
    const nextById = new Map(problems.map((p) => [p.id, p]));

    let resolved = 0;
    let improved = 0;
    let worsened = 0;
    let updated = 0;
    let added = 0;

    prevById.forEach((_prevProblem, id) => {
      if (!nextById.has(id)) resolved += 1;
    });

    nextById.forEach((nextProblem, id) => {
      const prevProblem = prevById.get(id);
      if (!prevProblem) {
        added += 1;
        return;
      }

      const prevRank = SEVERITY_RANK[prevProblem.severity];
      const nextRank = SEVERITY_RANK[nextProblem.severity];
      if (nextRank > prevRank) {
        improved += 1;
        return;
      }
      if (nextRank < prevRank) {
        worsened += 1;
        return;
      }

      const contentChanged =
        prevProblem.count !== nextProblem.count ||
        normalizeText(prevProblem.detail) !== normalizeText(nextProblem.detail) ||
        normalizeText(prevProblem.impact) !== normalizeText(nextProblem.impact) ||
        normalizeText(prevProblem.timeHint) !== normalizeText(nextProblem.timeHint) ||
        normalizeText(prevProblem.workbenchMachineCode) !== normalizeText(nextProblem.workbenchMachineCode) ||
        normalizeText(prevProblem.workbenchPlanDate) !== normalizeText(nextProblem.workbenchPlanDate) ||
        normalizeText(prevProblem.workbenchContext) !== normalizeText(nextProblem.workbenchContext) ||
        normalizeText(prevProblem.workbenchMaterialId) !== normalizeText(nextProblem.workbenchMaterialId) ||
        normalizeText(prevProblem.workbenchContractNo) !== normalizeText(nextProblem.workbenchContractNo);
      if (contentChanged) updated += 1;
    });

    setSyncDelta({ resolved, improved, worsened, updated, added });

    previousProblemsRef.current = problems;
    previousProblemSnapshots.set(snapshotScopeKey, problems);
  }, [problems, snapshotScopeKey]);

  return (
    <Card
      size="small"
      title="问题汇总"
      loading={loading}
      extra={
        <Space size={8} wrap>
          {(['P0', 'P1', 'P2'] as const).map((k) => {
            const n = counts[k];
            if (!n) return null;
            const meta = severityMeta(k);
            return (
              <Tag
                key={k}
                style={{
                  marginInlineEnd: 0,
                  borderColor: token.colorBorderSecondary,
                  background: token.colorFillQuaternary,
                  color: token.colorText,
                }}
              >
                <span style={{ color: meta.color, marginRight: 6, fontWeight: 700 }}>{k}</span>
                {n}
              </Tag>
            );
          })}

          <Segmented
            size="small"
            value={scope}
            onChange={(v) => setScope(v as 'ALL' | 'P0_P1')}
            options={[
              { label: '全部', value: 'ALL' },
              { label: '仅高优问题', value: 'P0_P1' },
            ]}
          />
          {syncDelta ? (
            (() => {
              const hasAny =
                syncDelta.resolved +
                  syncDelta.improved +
                  syncDelta.worsened +
                  syncDelta.updated +
                  syncDelta.added >
                0;
              return (
                <Tag
                  style={{ marginInlineEnd: 0 }}
                  color={
                    !hasAny
                      ? 'default'
                      : syncDelta.worsened > 0
                        ? 'error'
                        : syncDelta.resolved > 0 || syncDelta.improved > 0
                          ? 'success'
                          : 'processing'
                  }
                >
                  {hasAny
                    ? `同步:${syncDelta.resolved > 0 ? ` 已处理${syncDelta.resolved}` : ''}${syncDelta.improved > 0 ? ` 改善${syncDelta.improved}` : ''}${syncDelta.worsened > 0 ? ` 恶化${syncDelta.worsened}` : ''}${syncDelta.updated > 0 ? ` 更新${syncDelta.updated}` : ''}${syncDelta.added > 0 ? ` 新增${syncDelta.added}` : ''}`
                    : '同步: 无变化'}
                </Tag>
              );
            })()
          ) : null}
        </Space>
      }
    >
      {list.length === 0 ? (
        <Empty
          description={
            <Space direction="vertical" size={4}>
              <Text>当前无突出问题</Text>
              <Text type="secondary" style={{ fontSize: 12 }}>
                维度页仍可查看细节趋势（材料 / 产能 / 库存 / 换辊）
              </Text>
            </Space>
          }
        />
      ) : (
        <div style={{ display: 'flex', flexDirection: 'column', gap: 10 }}>
          {list.map((p, idx) => (
            <ProblemCard
              key={p.id}
              problem={p}
              index={idx}
              entered={entered}
              compact={compact}
              onOpenDrilldown={onOpenDrilldown}
              onGoWorkbench={onGoWorkbench}
            />
          ))}
        </div>
      )}
    </Card>
  );
};

export default React.memo(ProblemList);
