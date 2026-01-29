/**
 * KPI 总览卡片
 * 策略草案并排对比展示
 */

import React, { useMemo } from 'react';
import { Alert, Card, Space, Tag, Typography } from 'antd';
import type { StrategyDraftSummary, StrategyKey } from '../../types/strategy-draft';
import { formatPercent, formatTon, getMaturityRate, isSameNumber } from '../../utils/strategyDraftFormatters';

const { Text } = Typography;

export interface KpiOverviewCardProps {
  selectedStrategyKeysInOrder: StrategyKey[];
  draftsByStrategy: Partial<Record<StrategyKey, StrategyDraftSummary>>;
  strategyTitleMap: Partial<Record<StrategyKey, string>>;
  recommendation: StrategyDraftSummary | null;
  rangeDays: number;
}

type RowDef = {
  key: string;
  label: string;
  better?: 'min' | 'max';
  getScore?: (d: StrategyDraftSummary) => number;
  render: (d: StrategyDraftSummary) => React.ReactNode;
};

export const KpiOverviewCard: React.FC<KpiOverviewCardProps> = ({
  selectedStrategyKeysInOrder,
  draftsByStrategy,
  strategyTitleMap,
  recommendation,
  rangeDays,
}) => {
  const machineDaysTotal = Math.max(1, rangeDays) * 3; // backend 目前固定 3 条机组

  const overviewRows: RowDef[] = useMemo(
    () => [
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
    ],
    [machineDaysTotal]
  );

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

  return (
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
                成熟/未成熟 {recommendation.mature_count}/{recommendation.immature_count}，挤出{' '}
                {recommendation.squeezed_out_count}。 （仍建议人工复核关键订单/冻结区/风险点）
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
  );
};

export default KpiOverviewCard;
