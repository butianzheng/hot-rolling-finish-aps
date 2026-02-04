/**
 * 风险快照分析 - 主组件
 *
 * 重构后：638 行 → ~140 行 (-78%)
 */

import React, { useMemo } from 'react';
import { Alert, Button, Card, Table, Tabs } from 'antd';
import {
  LineChartOutlined,
  PieChartOutlined,
  BarChartOutlined,
  WarningOutlined,
} from '@ant-design/icons';
import { useNavigate } from 'react-router-dom';
import NoActiveVersionGuide from '../NoActiveVersionGuide';

import { useRiskSnapshotCharts } from './useRiskSnapshotCharts';
import { createRiskSnapshotColumns } from './riskSnapshotColumns';
import { FilterBar } from './FilterBar';
import { RiskMetricsCards } from './RiskMetricsCards';
import { TrendChart } from './TrendChart';
import { DistributionChart } from './DistributionChart';
import type { RiskSnapshotChartsProps } from './types';

const { TabPane } = Tabs;

const RiskSnapshotCharts: React.FC<RiskSnapshotChartsProps> = ({ onNavigateToPlan }) => {
  const navigate = useNavigate();
  const navigateToPlan = onNavigateToPlan || (() => navigate('/comparison'));

  const state = useRiskSnapshotCharts();

  // 表格列配置
  const columns = useMemo(
    () => createRiskSnapshotColumns({ mostRiskyDate: state.mostRiskyDate }),
    [state.mostRiskyDate]
  );

  // 没有激活版本时显示引导
  if (!state.activeVersionId) {
    return (
      <NoActiveVersionGuide
        title="尚无激活的排产版本"
        description="风险分析需要一个激活的排产版本作为基础"
        onNavigateToPlan={navigateToPlan}
      />
    );
  }

  return (
    <div style={{ padding: '24px' }}>
      {/* 标题和操作栏 */}
      <FilterBar
        versionOptions={state.versionOptions}
        selectedVersion={state.selectedVersion}
        onVersionChange={state.setSelectedVersion}
        dateRange={state.dateRange}
        onDateRangeChange={state.setDateRange}
        onRefresh={state.refresh}
        riskSnapshots={state.riskSnapshots}
      />

      {/* 最危险日期提醒 */}
      {state.mostRiskyDate && (
        <Alert
          message="风险预警"
          description={`最危险日期: ${state.mostRiskyDate}，请重点关注该日期的排产情况`}
          type="warning"
          showIcon
          icon={<WarningOutlined />}
          style={{ marginBottom: 16 }}
          action={
            <Button
              size="small"
              type="primary"
              danger
              onClick={() => state.setActiveTab('details')}
            >
              查看详情
            </Button>
          }
        />
      )}

      {/* 风险指标卡片 */}
      <RiskMetricsCards
        riskSnapshots={state.riskSnapshots}
        mostRiskyDate={state.mostRiskyDate}
      />

      {/* 图表标签页 */}
      <Card>
        <Tabs activeKey={state.activeTab} onChange={state.setActiveTab}>
          <TabPane
            tab={
              <span>
                <LineChartOutlined />
                风险趋势
              </span>
            }
            key="trend"
          >
            <div>
              <h3>风险分数趋势图</h3>
              <p style={{ color: '#8c8c8c' }}>
                显示各日期的风险分数变化趋势，分数越高风险越大
              </p>
              <TrendChart riskSnapshots={state.riskSnapshots} />
            </div>
          </TabPane>

          <TabPane
            tab={
              <span>
                <PieChartOutlined />
                风险分布
              </span>
            }
            key="distribution"
          >
            <div>
              <h3>风险等级分布</h3>
              <p style={{ color: '#8c8c8c' }}>
                显示不同风险等级的天数分布情况
              </p>
              <DistributionChart riskSnapshots={state.riskSnapshots} />
            </div>
          </TabPane>

          <TabPane
            tab={
              <span>
                <BarChartOutlined />
                详细数据
              </span>
            }
            key="details"
          >
            <div>
              <h3>风险快照详情</h3>
              <p style={{ color: '#8c8c8c' }}>
                显示每日的详细风险指标数据
              </p>
              <Table
                columns={columns}
                dataSource={state.riskSnapshots}
                loading={state.loading}
                pagination={false}
                rowKey="planDate"
                size="small"
                rowClassName={(record) =>
                  record.planDate === state.mostRiskyDate ? 'most-risky-row' : ''
                }
              />
              <style>{`
                .most-risky-row {
                  background-color: #fff1f0 !important;
                  font-weight: bold;
                }
                .most-risky-row:hover {
                  background-color: #ffccc7 !important;
                }
              `}</style>
            </div>
          </TabPane>
        </Tabs>
      </Card>
    </div>
  );
};

export default RiskSnapshotCharts;
