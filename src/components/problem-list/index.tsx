/**
 * ProblemList 问题列表主组件
 *
 * 重构后：421 行 → ~85 行 (-80%)
 */

import React, { useEffect, useMemo, useState } from 'react';
import { Card, Empty, Grid, Segmented, Space, Tag, Typography, theme } from 'antd';
import type { ProblemListProps, ProblemSeverity } from './types';
import { severityMeta } from './types';
import { ProblemCard } from './ProblemCard';

const { Text } = Typography;

const ProblemList: React.FC<ProblemListProps> = ({ loading, problems, onOpenDrilldown, onGoWorkbench }) => {
  const { token } = theme.useToken();
  const screens = Grid.useBreakpoint();
  const compact = !screens.md;

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
        </Space>
      }
    >
      {list.length === 0 ? (
        <Empty
          description={
            <Space direction="vertical" size={4}>
              <Text>当前无突出问题</Text>
              <Text type="secondary" style={{ fontSize: 12 }}>
                维度页仍可查看细节趋势（订单 / 产能 / 库存 / 换辊）
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
