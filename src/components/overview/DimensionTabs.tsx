import React, { useMemo } from 'react';
import { Badge, Space, Tabs } from 'antd';
import {
  BarChartOutlined,
  ExclamationCircleOutlined,
  InboxOutlined,
  ProfileOutlined,
  ToolOutlined,
} from '@ant-design/icons';
import PageSkeleton from '../PageSkeleton';
import ProblemList from './ProblemList';
import type { DrilldownSpec, RiskProblem } from '../../hooks/useRiskOverviewData';

const D1RiskHeatmap = React.lazy(() => import('../../pages/DecisionBoard/D1RiskHeatmap'));
const D2OrderFailure = React.lazy(() => import('../../pages/DecisionBoard/D2OrderFailure'));
const D3ColdStock = React.lazy(() => import('../../pages/DecisionBoard/D3ColdStock'));
const D4Bottleneck = React.lazy(() => import('../../pages/DecisionBoard/D4Bottleneck'));
const D5RollCampaign = React.lazy(() => import('../../pages/DecisionBoard/D5RollCampaign'));
const D6CapacityOpportunity = React.lazy(() => import('../../pages/DecisionBoard/D6CapacityOpportunity'));

export type DimensionTabKey = 'issues' | 'orders' | 'capacity' | 'inventory' | 'roll';

interface DimensionTabsProps {
  activeKey: DimensionTabKey;
  onChange: (key: DimensionTabKey) => void;
  loading?: boolean;
  problems: RiskProblem[];
  onOpenDrilldown: (spec: DrilldownSpec) => void;
  onGoWorkbench: (problem: RiskProblem) => void;
}

const DimensionTabs: React.FC<DimensionTabsProps> = ({
  activeKey,
  onChange,
  loading,
  problems,
  onOpenDrilldown,
  onGoWorkbench,
}) => {
  const counts = useMemo(() => {
    const out: Record<DimensionTabKey, number> = {
      issues: problems.length,
      orders: 0,
      capacity: 0,
      inventory: 0,
      roll: 0,
    };

    problems.forEach((p) => {
      switch (p.drilldown.kind) {
        case 'orders':
          out.orders += 1;
          break;
        case 'bottleneck':
        case 'risk':
          out.capacity += 1;
          break;
        case 'coldStock':
          out.inventory += 1;
          break;
        case 'roll':
          out.roll += 1;
          break;
        default:
          break;
      }
    });

    return out;
  }, [problems]);

  const makeLabel = (key: DimensionTabKey, icon: React.ReactNode, text: string) => (
    <Space size={8}>
      {icon}
      <span>{text}</span>
      {counts[key] > 0 ? <Badge count={counts[key]} size="small" overflowCount={99} /> : null}
    </Space>
  );

  return (
    <Tabs
      activeKey={activeKey}
      onChange={(k) => onChange(k as DimensionTabKey)}
      destroyInactiveTabPane
      items={[
        {
          key: 'issues',
          label: makeLabel('issues', <ExclamationCircleOutlined />, '问题汇总'),
          children: (
            <ProblemList
              loading={loading}
              problems={problems}
              onOpenDrilldown={onOpenDrilldown}
              onGoWorkbench={onGoWorkbench}
            />
          ),
        },
        {
          key: 'orders',
          label: makeLabel('orders', <ProfileOutlined />, '订单维度'),
          children: (
            <React.Suspense fallback={<PageSkeleton />}>
              <D2OrderFailure embedded onOpenDrilldown={onOpenDrilldown} />
            </React.Suspense>
          ),
        },
        {
          key: 'capacity',
          label: makeLabel('capacity', <BarChartOutlined />, '产能维度'),
          children: (
            <React.Suspense fallback={<PageSkeleton />}>
              <div style={{ display: 'flex', flexDirection: 'column', gap: 12 }}>
                <D1RiskHeatmap embedded onOpenDrilldown={onOpenDrilldown} />
                <D4Bottleneck embedded onOpenDrilldown={onOpenDrilldown} />
                <D6CapacityOpportunity embedded onOpenDrilldown={onOpenDrilldown} />
              </div>
            </React.Suspense>
          ),
        },
        {
          key: 'inventory',
          label: makeLabel('inventory', <InboxOutlined />, '库存维度'),
          children: (
            <React.Suspense fallback={<PageSkeleton />}>
              <D3ColdStock embedded onOpenDrilldown={onOpenDrilldown} />
            </React.Suspense>
          ),
        },
        {
          key: 'roll',
          label: makeLabel('roll', <ToolOutlined />, '换辊维度'),
          children: (
            <React.Suspense fallback={<PageSkeleton />}>
              <D5RollCampaign embedded onOpenDrilldown={onOpenDrilldown} />
            </React.Suspense>
          ),
        },
      ]}
    />
  );
};

export default React.memo(DimensionTabs);
