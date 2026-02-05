/**
 * KPIBand - 关键指标横幅组件
 *
 * 重构后：277 行 → ~130 行 (-53%)
 */

import React, { useMemo, useState } from 'react';
import { Badge, Col, Row, Skeleton, Space, Statistic, Tag, Typography, Card } from 'antd';
import type { DrilldownSpec } from '../../hooks/useRiskOverviewData';
import type { KPIBandProps } from './types';
import { getRiskMeta, getRollMeta, DEFAULT_KPI } from './types';
import { KPICardWrapper } from './KPICardWrapper';

const { Text } = Typography;

const KPIBand: React.FC<KPIBandProps> = ({ loading, kpi, onOpenDrilldown, onGoWorkbench }) => {
  const [hovered, setHovered] = useState<string | null>(null);

  const safe = kpi ?? DEFAULT_KPI;
  const riskMeta = useMemo(() => getRiskMeta(safe.riskLevel), [safe.riskLevel]);
  const rollMeta = useMemo(() => getRollMeta(safe.rollStatus), [safe.rollStatus]);

  const urgentL3Total = safe.urgentBreakdown?.L3?.total ?? 0;
  const urgentL3Blocked = safe.urgentBreakdown?.L3?.blocked ?? 0;
  const urgentL2Total = safe.urgentBreakdown?.L2?.total ?? 0;
  const urgentL2Blocked = safe.urgentBreakdown?.L2?.blocked ?? 0;
  const urgentDefaultLevel = urgentL3Total > 0 ? 'L3' : urgentL2Total > 0 ? 'L2' : null;

  const onEnter = (id: string) => setHovered(id);
  const onLeave = (id: string) => setHovered((cur) => (cur === id ? null : cur));
  const open = (spec: DrilldownSpec) => onOpenDrilldown?.(spec);

  if (loading) {
    return (
      <Row gutter={[12, 12]} style={{ marginBottom: 12 }}>
        {Array.from({ length: 5 }).map((_, idx) => (
          <Col key={idx} xs={24} sm={12} md={12} lg={8} xl={24 / 5} style={{ display: 'flex' }}>
            <Card size="small" style={{ width: '100%' }}>
              <Skeleton active paragraph={{ rows: 1 }} />
            </Card>
          </Col>
        ))}
      </Row>
    );
  }

  return (
    <Row gutter={[12, 12]} style={{ marginBottom: 12 }}>
      {/* 整体风险 */}
      <Col xs={24} sm={12} md={12} lg={8} xl={24 / 5} style={{ display: 'flex' }}>
        <KPICardWrapper
          id="risk"
          hovered={hovered}
          onClick={onOpenDrilldown ? () => open({ kind: 'risk' }) : undefined}
          onMouseEnter={onEnter}
          onMouseLeave={onLeave}
        >
          <Space direction="vertical" size={4} style={{ width: '100%' }}>
            <Space size={8}>
              <Badge color={riskMeta.color} />
              <Text type="secondary">整体风险</Text>
            </Space>
            <Statistic value={riskMeta.label} valueStyle={{ color: riskMeta.color, fontWeight: 600 }} />
            {safe.mostRiskyDate && (
              <Text type="secondary" style={{ fontSize: 12 }}>{safe.mostRiskyDate}</Text>
            )}
            {onOpenDrilldown && (
              <Text type="secondary" style={{ fontSize: 12 }}>点击查看风险下钻</Text>
            )}
          </Space>
        </KPICardWrapper>
      </Col>

      {/* 紧急物料 */}
      <Col xs={24} sm={12} md={12} lg={8} xl={24 / 5} style={{ display: 'flex' }}>
        <KPICardWrapper
          id="urgent"
          hovered={hovered}
          onClick={
            onGoWorkbench
              ? () => onGoWorkbench({ workbenchTab: 'visualization', urgencyLevel: urgentDefaultLevel })
              : onOpenDrilldown
                ? () => open({ kind: 'orders' })
                : undefined
          }
          onMouseEnter={onEnter}
          onMouseLeave={onLeave}
        >
          <Space direction="vertical" size={4} style={{ width: '100%' }}>
            <Text type="secondary">紧急物料</Text>
            <Statistic value={safe.urgentOrdersCount} suffix="个" valueStyle={{ fontWeight: 600 }} />
            {safe.blockedUrgentCount > 0 && (
              <Text style={{ color: '#ff4d4f', fontSize: 12 }}>{safe.blockedUrgentCount} 个阻塞</Text>
            )}
            {(urgentL3Total > 0 || urgentL2Total > 0) && (
              <Space size={6} wrap>
                {urgentL3Total > 0 && (
                  <Tag
                    color="red"
                    style={{ marginInlineEnd: 0, cursor: onGoWorkbench ? 'pointer' : 'default' }}
                    onClick={onGoWorkbench ? (e) => {
                      e.stopPropagation();
                      onGoWorkbench({ workbenchTab: 'visualization', urgencyLevel: 'L3' });
                    } : undefined}
                  >
                    L3 {urgentL3Total}{urgentL3Blocked > 0 ? ` (阻${urgentL3Blocked})` : ''}
                  </Tag>
                )}
                {urgentL2Total > 0 && (
                  <Tag
                    color="orange"
                    style={{ marginInlineEnd: 0, cursor: onGoWorkbench ? 'pointer' : 'default' }}
                    onClick={onGoWorkbench ? (e) => {
                      e.stopPropagation();
                      onGoWorkbench({ workbenchTab: 'visualization', urgencyLevel: 'L2' });
                    } : undefined}
                  >
                    L2 {urgentL2Total}{urgentL2Blocked > 0 ? ` (阻${urgentL2Blocked})` : ''}
                  </Tag>
                )}
              </Space>
            )}
            {onGoWorkbench && (
              <Text type="secondary" style={{ fontSize: 12 }}>
                {urgentDefaultLevel ? `点击进入工作台（${urgentDefaultLevel}）` : '点击进入工作台'}
              </Text>
            )}
          </Space>
        </KPICardWrapper>
      </Col>

      {/* 利用率 */}
      <Col xs={24} sm={12} md={12} lg={8} xl={24 / 5} style={{ display: 'flex' }}>
        <KPICardWrapper
          id="util"
          hovered={hovered}
          onClick={onOpenDrilldown ? () => open({ kind: 'bottleneck' }) : undefined}
          onMouseEnter={onEnter}
          onMouseLeave={onLeave}
        >
          <Space direction="vertical" size={4} style={{ width: '100%' }}>
            <Text type="secondary">利用率</Text>
            <Statistic
              value={Number.isFinite(safe.capacityUtilization) ? safe.capacityUtilization : 0}
              precision={1}
              suffix="%"
              valueStyle={{ fontWeight: 600 }}
            />
            {onOpenDrilldown && (
              <Text type="secondary" style={{ fontSize: 12 }}>点击查看堵塞下钻</Text>
            )}
          </Space>
        </KPICardWrapper>
      </Col>

      {/* 冷坨压力 */}
      <Col xs={24} sm={12} md={12} lg={8} xl={24 / 5} style={{ display: 'flex' }}>
        <KPICardWrapper
          id="cold"
          hovered={hovered}
          onClick={onOpenDrilldown ? () => open({ kind: 'coldStock' }) : undefined}
          onMouseEnter={onEnter}
          onMouseLeave={onLeave}
        >
          <Space direction="vertical" size={4} style={{ width: '100%' }}>
            <Text type="secondary">冷坨压力</Text>
            <Statistic value={safe.coldStockCount} suffix="件" valueStyle={{ fontWeight: 600 }} />
            {onOpenDrilldown && (
              <Text type="secondary" style={{ fontSize: 12 }}>点击查看库存下钻</Text>
            )}
          </Space>
        </KPICardWrapper>
      </Col>

      {/* 换辊 */}
      <Col xs={24} sm={12} md={12} lg={8} xl={24 / 5} style={{ display: 'flex' }}>
        <KPICardWrapper
          id="roll"
          hovered={hovered}
          onClick={onOpenDrilldown ? () => open({ kind: 'roll' }) : undefined}
          onMouseEnter={onEnter}
          onMouseLeave={onLeave}
        >
          <Space direction="vertical" size={4} style={{ width: '100%' }}>
            <Space size={8}>
              <Badge color={rollMeta.color} />
              <Text type="secondary">设备监控</Text>
            </Space>
            <Statistic value={rollMeta.label} valueStyle={{ color: rollMeta.color, fontWeight: 600 }} />
            <Text type="secondary" style={{ fontSize: 12 }}>
              {safe.rollCampaignProgress.toFixed(3)}t / {safe.rollChangeThreshold.toFixed(3)}t
            </Text>
            {onOpenDrilldown && (
              <Text type="secondary" style={{ fontSize: 12 }}>点击查看换辊监控</Text>
            )}
          </Space>
        </KPICardWrapper>
      </Col>
    </Row>
  );
};

export default React.memo(KPIBand);
