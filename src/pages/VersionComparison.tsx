import React, { useMemo } from 'react';
import { Space, Tabs } from 'antd';
import { useNavigate, useSearchParams } from 'react-router-dom';
import ErrorBoundary from '../components/ErrorBoundary';
import PageSkeleton from '../components/PageSkeleton';
import StrategyDraftComparison from '../components/strategy-draft';
import DecisionFlowGuide from '../components/flow/DecisionFlowGuide';

const PlanManagement = React.lazy(() => import('../components/PlanManagement'));

const TAB_KEYS = ['historical', 'draft'] as const;
type TabKey = (typeof TAB_KEYS)[number];

function normalizeTabKey(value: string | null): TabKey {
  if (value && (TAB_KEYS as readonly string[]).includes(value)) return value as TabKey;
  return 'historical';
}

const VersionComparison: React.FC = () => {
  const navigate = useNavigate();
  const [searchParams, setSearchParams] = useSearchParams();
  const activeKey = useMemo(() => normalizeTabKey(searchParams.get('tab')), [searchParams]);

  const handleTabChange = (key: string) => {
    const next = new URLSearchParams(searchParams);
    next.set('tab', key);
    setSearchParams(next, { replace: true });
  };

  return (
    <ErrorBoundary>
      <Space direction="vertical" size={12} style={{ width: '100%' }}>
        <DecisionFlowGuide
          stage="comparison"
          title={
            activeKey === 'draft'
              ? '下一步：生成草案 -> 选择推荐 -> 发布后再决定是否激活'
              : '下一步：切到「策略草案对比」生成草案'
          }
          description={
            activeKey === 'draft'
              ? '在下方点击「重新计算策略草案」，查看指标/变更明细；发布后可选择仅切换到新版本继续微调，或直接激活为生效版本。'
              : '建议先进入草案对比：生成多策略草案并解释性对比，再发布为正式版本。'
          }
          primaryAction={{
            label: '去策略草案对比',
            disabled: activeKey === 'draft',
            onClick: () => handleTabChange('draft'),
          }}
          secondaryAction={{
            label: '回工作台',
            onClick: () => navigate('/workbench'),
          }}
        />

        <Tabs
          destroyInactiveTabPane
          activeKey={activeKey}
          onChange={handleTabChange}
          items={[
            {
              key: 'historical',
              label: '历史版本对比',
              children: (
                <React.Suspense fallback={<PageSkeleton />}>
                  <PlanManagement />
                </React.Suspense>
              ),
            },
            {
              key: 'draft',
              label: '策略草案对比',
              children: <StrategyDraftComparison />,
            },
          ]}
        />
      </Space>
    </ErrorBoundary>
  );
};

export default VersionComparison;
