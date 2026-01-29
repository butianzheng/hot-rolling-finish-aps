import React, { useMemo, useState } from 'react';
import { Badge, Card, Col, Row, Skeleton, Space, Statistic, Tag, Typography, theme } from 'antd';
import type { GlobalKPI } from '../../types/kpi';
import type { DrilldownSpec, WorkbenchTabKey } from '../../hooks/useRiskOverviewData';

const { Text } = Typography;

function getRiskMeta(level: GlobalKPI['riskLevel']) {
  switch (level) {
    case 'critical':
      return { label: '严重', color: '#ff4d4f' };
    case 'high':
      return { label: '高', color: '#faad14' };
    case 'medium':
      return { label: '中', color: '#1677ff' };
    default:
      return { label: '低', color: '#52c41a' };
  }
}

function getRollMeta(status: GlobalKPI['rollStatus']) {
  switch (status) {
    case 'critical':
      return { label: '硬停止', color: '#ff4d4f' };
    case 'warning':
      return { label: '预警', color: '#faad14' };
    default:
      return { label: '正常', color: '#52c41a' };
  }
}

interface KPIBandProps {
  loading?: boolean;
  kpi: GlobalKPI | null;
  onOpenDrilldown?: (spec: DrilldownSpec) => void;
  onGoWorkbench?: (opts: {
    workbenchTab?: WorkbenchTabKey;
    machineCode?: string | null;
    urgencyLevel?: string | null;
  }) => void;
}

const KPIBand: React.FC<KPIBandProps> = ({ loading, kpi, onOpenDrilldown, onGoWorkbench }) => {
  const { token } = theme.useToken();
  const [hovered, setHovered] = useState<string | null>(null);

  const safe = kpi ?? {
    urgentOrdersCount: 0,
    blockedUrgentCount: 0,
    urgentBreakdown: {
      L2: { total: 0, blocked: 0 },
      L3: { total: 0, blocked: 0 },
    },
    capacityUtilization: 0,
    coldStockCount: 0,
    rollCampaignProgress: 0,
    rollChangeThreshold: 1500,
    rollStatus: 'healthy' as const,
    riskLevel: 'low' as const,
  };

  const riskMeta = useMemo(() => getRiskMeta(safe.riskLevel), [safe.riskLevel]);
  const rollMeta = useMemo(() => getRollMeta(safe.rollStatus), [safe.rollStatus]);
  const urgentL3Total = safe.urgentBreakdown?.L3?.total ?? 0;
  const urgentL3Blocked = safe.urgentBreakdown?.L3?.blocked ?? 0;
  const urgentL2Total = safe.urgentBreakdown?.L2?.total ?? 0;
  const urgentL2Blocked = safe.urgentBreakdown?.L2?.blocked ?? 0;
  const urgentDefaultLevel = urgentL3Total > 0 ? 'L3' : urgentL2Total > 0 ? 'L2' : null;

  if (loading) {
    return (
      <Row gutter={12} style={{ marginBottom: 12 }}>
        {Array.from({ length: 5 }).map((_, idx) => (
          <Col key={idx} span={24} md={12} xl={4}>
            <Card size="small">
              <Skeleton active paragraph={{ rows: 1 }} />
            </Card>
          </Col>
        ))}
      </Row>
    );
  }

  const clickWrapProps = (id: string, onClick?: () => void) => ({
    role: onClick ? 'button' : undefined,
    tabIndex: onClick ? 0 : undefined,
    onClick,
    onKeyDown: onClick
      ? (e: React.KeyboardEvent) => {
          if (e.key === 'Enter' || e.key === ' ') {
            e.preventDefault();
            onClick();
          }
        }
      : undefined,
    onMouseEnter: onClick ? () => setHovered(id) : undefined,
    onMouseLeave: onClick ? () => setHovered((cur) => (cur === id ? null : cur)) : undefined,
    style: {
      cursor: onClick ? 'pointer' : 'default',
      transition: 'transform 160ms ease, box-shadow 160ms ease',
      transform: hovered === id ? 'translateY(-2px)' : 'translateY(0)',
      boxShadow: hovered === id ? token.boxShadowSecondary : 'none',
      borderRadius: token.borderRadiusLG,
      outline: 'none',
    } as React.CSSProperties,
  });

  const open = (spec: DrilldownSpec) => {
    if (!onOpenDrilldown) return;
    onOpenDrilldown(spec);
  };

  return (
    <Row gutter={12} style={{ marginBottom: 12 }}>
      <Col span={24} md={12} xl={4}>
        <div {...clickWrapProps('risk', onOpenDrilldown ? () => open({ kind: 'risk' }) : undefined)}>
          <Card size="small">
          <Space direction="vertical" size={2}>
            <Space size={8}>
              <Badge color={riskMeta.color} />
              <Text type="secondary">整体风险</Text>
            </Space>
            <Statistic value={riskMeta.label} valueStyle={{ color: riskMeta.color, fontWeight: 600 }} />
            {safe.mostRiskyDate && (
              <Text type="secondary" style={{ fontSize: 12 }}>
                {safe.mostRiskyDate}
              </Text>
            )}
            {onOpenDrilldown && (
              <Text type="secondary" style={{ fontSize: 12 }}>
                点击查看风险下钻
              </Text>
            )}
          </Space>
          </Card>
        </div>
      </Col>

      <Col span={24} md={12} xl={5}>
        <div
          {...clickWrapProps(
            'urgent',
            onGoWorkbench
              ? () =>
                  onGoWorkbench({
                    workbenchTab: 'visualization',
                    urgencyLevel: urgentDefaultLevel,
                  })
              : onOpenDrilldown
                ? () => open({ kind: 'orders' })
                : undefined
          )}
        >
          <Card size="small">
          <Space direction="vertical" size={2}>
            <Text type="secondary">紧急物料</Text>
            <Statistic value={safe.urgentOrdersCount} suffix="个" valueStyle={{ fontWeight: 600 }} />
            {safe.blockedUrgentCount > 0 && (
              <Text style={{ color: '#ff4d4f', fontSize: 12 }}>{safe.blockedUrgentCount} 个阻塞</Text>
            )}
            {(urgentL3Total > 0 || urgentL2Total > 0) && (
              <Space size={6} wrap>
                {urgentL3Total > 0 ? (
                  <Tag
                    color="red"
                    style={{ marginInlineEnd: 0, cursor: onGoWorkbench ? 'pointer' : 'default' }}
                    onClick={
                      onGoWorkbench
                        ? (e) => {
                            e.stopPropagation();
                            onGoWorkbench({ workbenchTab: 'visualization', urgencyLevel: 'L3' });
                          }
                        : undefined
                    }
                  >
                    L3 {urgentL3Total}
                    {urgentL3Blocked > 0 ? ` (阻${urgentL3Blocked})` : ''}
                  </Tag>
                ) : null}

                {urgentL2Total > 0 ? (
                  <Tag
                    color="orange"
                    style={{ marginInlineEnd: 0, cursor: onGoWorkbench ? 'pointer' : 'default' }}
                    onClick={
                      onGoWorkbench
                        ? (e) => {
                            e.stopPropagation();
                            onGoWorkbench({ workbenchTab: 'visualization', urgencyLevel: 'L2' });
                          }
                        : undefined
                    }
                  >
                    L2 {urgentL2Total}
                    {urgentL2Blocked > 0 ? ` (阻${urgentL2Blocked})` : ''}
                  </Tag>
                ) : null}
              </Space>
            )}
            {onGoWorkbench && (
              <Text type="secondary" style={{ fontSize: 12 }}>
                {urgentDefaultLevel ? `点击进入工作台（${urgentDefaultLevel}）` : '点击进入工作台'}
              </Text>
            )}
          </Space>
          </Card>
        </div>
      </Col>

      <Col span={24} md={12} xl={5}>
        <div
          {...clickWrapProps('util', onOpenDrilldown ? () => open({ kind: 'bottleneck' }) : undefined)}
        >
          <Card size="small">
          <Space direction="vertical" size={2}>
            <Text type="secondary">利用率</Text>
            <Statistic
              value={Number.isFinite(safe.capacityUtilization) ? safe.capacityUtilization : 0}
              precision={1}
              suffix="%"
              valueStyle={{ fontWeight: 600 }}
            />
            {onOpenDrilldown && (
              <Text type="secondary" style={{ fontSize: 12 }}>
                点击查看堵塞下钻
              </Text>
            )}
          </Space>
          </Card>
        </div>
      </Col>

      <Col span={24} md={12} xl={5}>
        <div
          {...clickWrapProps('cold', onOpenDrilldown ? () => open({ kind: 'coldStock' }) : undefined)}
        >
          <Card size="small">
          <Space direction="vertical" size={2}>
            <Text type="secondary">冷坨压力</Text>
            <Statistic value={safe.coldStockCount} suffix="件" valueStyle={{ fontWeight: 600 }} />
            {onOpenDrilldown && (
              <Text type="secondary" style={{ fontSize: 12 }}>
                点击查看库存下钻
              </Text>
            )}
          </Space>
          </Card>
        </div>
      </Col>

      <Col span={24} md={12} xl={5}>
        <div {...clickWrapProps('roll', onOpenDrilldown ? () => open({ kind: 'roll' }) : undefined)}>
          <Card size="small">
          <Space direction="vertical" size={2}>
            <Space size={8}>
              <Badge color={rollMeta.color} />
              <Text type="secondary">换辊</Text>
            </Space>
            <Statistic value={rollMeta.label} valueStyle={{ color: rollMeta.color, fontWeight: 600 }} />
            <Text type="secondary" style={{ fontSize: 12 }}>
              {safe.rollCampaignProgress}t / {safe.rollChangeThreshold}t
            </Text>
            {onOpenDrilldown && (
              <Text type="secondary" style={{ fontSize: 12 }}>
                点击查看换辊下钻
              </Text>
            )}
          </Space>
          </Card>
        </div>
      </Col>
    </Row>
  );
};

export default React.memo(KPIBand);
