import React, { useEffect, useMemo, useState } from 'react';
import { Button, Card, Empty, Grid, Segmented, Space, Tag, Typography, theme } from 'antd';
import {
  BarChartOutlined,
  ClockCircleOutlined,
  DeploymentUnitOutlined,
  FireOutlined,
  InboxOutlined,
  InfoCircleOutlined,
  ThunderboltOutlined,
  WarningOutlined,
} from '@ant-design/icons';
import type { DrilldownSpec, ProblemSeverity, RiskProblem, WorkbenchTabKey } from '../../hooks/useRiskOverviewData';
import { FONT_FAMILIES } from '../../theme';

const { Text } = Typography;

function severityMeta(severity: ProblemSeverity) {
  switch (severity) {
    case 'P0':
      return {
        badge: 'P0',
        label: '立即处理',
        color: '#ff4d4f',
        glow: 'rgba(255, 77, 79, 0.18)',
        icon: <FireOutlined />,
      };
    case 'P1':
      return {
        badge: 'P1',
        label: '尽快处理',
        color: '#faad14',
        glow: 'rgba(250, 173, 20, 0.16)',
        icon: <WarningOutlined />,
      };
    case 'P2':
      return {
        badge: 'P2',
        label: '关注',
        color: '#1677ff',
        glow: 'rgba(22, 119, 255, 0.14)',
        icon: <InfoCircleOutlined />,
      };
    default:
      return {
        badge: 'P3',
        label: '观察',
        color: '#52c41a',
        glow: 'rgba(82, 196, 26, 0.14)',
        icon: <InfoCircleOutlined />,
      };
  }
}

function workbenchMeta(tab: WorkbenchTabKey | undefined) {
  switch (tab) {
    case 'materials':
      return { label: '物料', icon: <InboxOutlined /> };
    case 'capacity':
      return { label: '产能', icon: <BarChartOutlined /> };
    case 'visualization':
      return { label: '排程', icon: <DeploymentUnitOutlined /> };
    default:
      return null;
  }
}

interface ProblemListProps {
  loading?: boolean;
  problems: RiskProblem[];
  onOpenDrilldown: (spec: DrilldownSpec) => void;
  onGoWorkbench: (problem: RiskProblem) => void;
}

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
              { label: '仅 P0/P1', value: 'P0_P1' },
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
          {list.map((p, idx) => {
            const meta = severityMeta(p.severity);
            const wb = workbenchMeta(p.workbenchTab);

            const actionLabel =
              p.workbenchTab === 'materials'
                ? '去物料处理'
                : p.workbenchTab === 'capacity'
                  ? '去机组配置'
                  : '去排程处理';

            return (
              <div
                key={p.id}
                role="button"
                tabIndex={0}
                onClick={() => onOpenDrilldown(p.drilldown)}
                onKeyDown={(e) => {
                  if (e.key === 'Enter' || e.key === ' ') {
                    e.preventDefault();
                    onOpenDrilldown(p.drilldown);
                  }
                }}
                style={{
                  position: 'relative',
                  borderRadius: token.borderRadiusLG,
                  border: `1px solid ${token.colorBorderSecondary}`,
                  background: token.colorBgContainer,
                  overflow: 'hidden',
                  padding: 12,
                  cursor: 'pointer',
                  opacity: entered ? 1 : 0,
                  transform: entered ? 'translateY(0)' : 'translateY(6px)',
                  transition: 'opacity 220ms ease, transform 220ms ease, border-color 220ms ease',
                  transitionDelay: `${Math.min(idx, 6) * 45}ms`,
                }}
              >
                <div
                  aria-hidden
                  style={{
                    position: 'absolute',
                    inset: 0,
                    backgroundImage: `radial-gradient(520px circle at 0% 0%, ${meta.glow}, transparent 60%)`,
                    pointerEvents: 'none',
                  }}
                />

                <div
                  aria-hidden
                  style={{
                    position: 'absolute',
                    left: 0,
                    top: 0,
                    bottom: 0,
                    width: 6,
                    background: `linear-gradient(180deg, ${meta.color}, transparent)`,
                    opacity: 0.85,
                    pointerEvents: 'none',
                  }}
                />

                <div
                  style={{
                    display: 'flex',
                    alignItems: compact ? 'stretch' : 'flex-start',
                    gap: 12,
                    flexDirection: compact ? 'column' : 'row',
                  }}
                >
                  <div style={{ display: 'flex', gap: 12, alignItems: 'flex-start' }}>
                    <div
                      style={{
                        width: 52,
                        minWidth: 52,
                        height: 52,
                        borderRadius: token.borderRadiusLG,
                        background: token.colorFillQuaternary,
                        border: `1px solid ${token.colorBorderSecondary}`,
                        display: 'flex',
                        flexDirection: 'column',
                        alignItems: 'center',
                        justifyContent: 'center',
                        gap: 2,
                        color: meta.color,
                      }}
                    >
                      <Text style={{ color: meta.color, fontFamily: FONT_FAMILIES.MONOSPACE, fontWeight: 800 }}>
                        {meta.badge}
                      </Text>
                      <Text style={{ color: token.colorTextSecondary, fontSize: 11 }}>{meta.label}</Text>
                    </div>

                    <div style={{ flex: 1, minWidth: 0 }}>
                      <div
                        style={{
                          display: 'flex',
                          alignItems: 'baseline',
                          justifyContent: 'space-between',
                          gap: 12,
                          flexWrap: 'wrap',
                        }}
                      >
                        <Space size={10} wrap>
                          <Space size={6} align="center">
                            <span style={{ color: meta.color, display: 'inline-flex', alignItems: 'center' }}>
                              {meta.icon}
                            </span>
                            <Text strong style={{ fontSize: 14 }}>
                              {p.title}
                            </Text>
                          </Space>

                          {typeof p.count === 'number' ? (
                            <Tag
                              style={{
                                marginInlineEnd: 0,
                                borderColor: token.colorBorderSecondary,
                                background: token.colorFillQuaternary,
                                color: token.colorTextSecondary,
                                fontFamily: FONT_FAMILIES.MONOSPACE,
                              }}
                            >
                              {p.count}
                            </Tag>
                          ) : null}
                        </Space>

                        <Space size={6} wrap>
                          {p.workbenchMachineCode ? (
                            <Tag
                              style={{
                                marginInlineEnd: 0,
                                borderColor: token.colorBorderSecondary,
                                background: token.colorFillQuaternary,
                                color: token.colorText,
                                fontFamily: FONT_FAMILIES.MONOSPACE,
                              }}
                            >
                              {p.workbenchMachineCode}
                            </Tag>
                          ) : null}

                          {wb ? (
                            <Tag
                              style={{
                                marginInlineEnd: 0,
                                borderColor: token.colorBorderSecondary,
                                background: token.colorFillQuaternary,
                                color: token.colorTextSecondary,
                              }}
                            >
                              <Space size={6}>
                                {wb.icon}
                                <span>{wb.label}</span>
                              </Space>
                            </Tag>
                          ) : null}
                        </Space>
                      </div>

                      <div style={{ marginTop: 8 }}>
                        <Space size={8} wrap>
                          {p.detail ? (
                            <Tag
                              style={{
                                marginInlineEnd: 0,
                                maxWidth: compact ? 360 : 520,
                                borderColor: token.colorBorderSecondary,
                                background: token.colorFillTertiary,
                                color: token.colorTextSecondary,
                              }}
                            >
                              <Space size={6}>
                                <InfoCircleOutlined />
                                <Text
                                  style={{ color: token.colorTextSecondary, maxWidth: compact ? 300 : 460 }}
                                  ellipsis={{ tooltip: p.detail }}
                                >
                                  {p.detail}
                                </Text>
                              </Space>
                            </Tag>
                          ) : null}

                          {p.impact ? (
                            <Tag
                              style={{
                                marginInlineEnd: 0,
                                maxWidth: compact ? 360 : 520,
                                borderColor: token.colorBorderSecondary,
                                background: token.colorFillTertiary,
                                color: token.colorTextSecondary,
                              }}
                            >
                              <Space size={6}>
                                <ThunderboltOutlined />
                                <Text
                                  style={{ color: token.colorTextSecondary, maxWidth: compact ? 300 : 460 }}
                                  ellipsis={{ tooltip: p.impact }}
                                >
                                  {p.impact}
                                </Text>
                              </Space>
                            </Tag>
                          ) : null}

                          {p.timeHint ? (
                            <Tag
                              style={{
                                marginInlineEnd: 0,
                                maxWidth: compact ? 360 : 520,
                                borderColor: token.colorBorderSecondary,
                                background: token.colorFillTertiary,
                                color: token.colorTextSecondary,
                              }}
                            >
                              <Space size={6}>
                                <ClockCircleOutlined />
                                <Text
                                  style={{ color: token.colorTextSecondary, maxWidth: compact ? 300 : 460 }}
                                  ellipsis={{ tooltip: p.timeHint }}
                                >
                                  {p.timeHint}
                                </Text>
                              </Space>
                            </Tag>
                          ) : null}
                        </Space>
                      </div>
                    </div>
                  </div>

                  <Space
                    size={8}
                    style={{
                      marginLeft: compact ? 0 : 'auto',
                      justifyContent: compact ? 'flex-start' : 'flex-end',
                    }}
                    wrap
                  >
                    <Button
                      onClick={(e) => {
                        e.stopPropagation();
                        onOpenDrilldown(p.drilldown);
                      }}
                    >
                      详情
                    </Button>
                    <Button
                      type="primary"
                      onClick={(e) => {
                        e.stopPropagation();
                        onGoWorkbench(p);
                      }}
                    >
                      {actionLabel}
                    </Button>
                  </Space>
                </div>
              </div>
            );
          })}
        </div>
      )}
    </Card>
  );
};

export default React.memo(ProblemList);
