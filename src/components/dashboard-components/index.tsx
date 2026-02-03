/**
 * Dashboard - 主组件
 *
 * 重构后：399 行 → ~100 行 (-75%)
 */

import React, { useMemo } from 'react';
import { Card, Col, Row, Table } from 'antd';
import { useNavigate } from 'react-router-dom';
import NoActiveVersionGuide from '../NoActiveVersionGuide';

import { useDashboard } from './useDashboard';
import { createOrderFailureColumns, createColdStockColumns } from './dashboardColumns';
import { RefreshControlBar } from './RefreshControlBar';
import { StatisticsCards } from './StatisticsCards';

const Dashboard: React.FC = () => {
  const navigate = useNavigate();
  const state = useDashboard();

  // 表格列配置
  const orderFailureColumns = useMemo(() => createOrderFailureColumns(), []);
  const coldStockColumns = useMemo(() => createColdStockColumns(), []);

  // 无活动版本时显示引导
  if (!state.activeVersionId) {
    return (
      <NoActiveVersionGuide
        title="尚无激活的排产版本"
        description="驾驶舱需要一个激活的排产版本作为基础"
        onNavigateToPlan={() => navigate('/comparison')}
      />
    );
  }

  return (
    <div>
      {/* 刷新控制栏 */}
      <RefreshControlBar
        loading={state.loading}
        autoRefreshEnabled={state.autoRefreshEnabled}
        onAutoRefreshChange={state.setAutoRefreshEnabled}
        refreshInterval={state.refreshInterval}
        onRefreshIntervalChange={state.setRefreshInterval}
        lastRefreshTime={state.lastRefreshTime}
        nextRefreshCountdown={state.nextRefreshCountdown}
        onManualRefresh={state.manualRefresh}
      />

      {/* 统计卡片 */}
      <StatisticsCards
        orderFailures={state.orderFailures}
        orderFailureSummary={state.orderFailureSummary}
        coldStockBuckets={state.coldStockBuckets}
        coldStockSummary={state.coldStockSummary}
        mostCongestedPoint={state.mostCongestedPoint}
        onNavigate={navigate}
      />

      {/* 数据表格 */}
      <Row gutter={16}>
        <Col span={12}>
          <Card title="订单失败集合 (D2)" variant="borderless">
            <Table
              columns={orderFailureColumns}
              dataSource={state.orderFailures}
              rowKey={(r) => `${r.contractNo}-${r.dueDate}-${r.machineCode}`}
              loading={state.loading}
              pagination={{ pageSize: 5 }}
              size="small"
              onRow={(record) => ({
                onClick: () => {
                  const qs = new URLSearchParams({
                    contractNo: record.contractNo,
                    urgency: record.urgencyLevel,
                    failType: record.failType,
                  }).toString();
                  navigate(`/overview?tab=d2&${qs}`);
                },
              })}
            />
          </Card>
        </Col>
        <Col span={12}>
          <Card title="冷料压库分桶 (D3)" variant="borderless">
            <Table
              columns={coldStockColumns}
              dataSource={state.coldStockBuckets}
              rowKey={(r) => `${r.machineCode}-${r.ageBin}-${r.pressureLevel}`}
              loading={state.loading}
              pagination={{ pageSize: 5 }}
              size="small"
              onRow={(record) => ({
                onClick: () => {
                  const qs = new URLSearchParams({
                    machine: record.machineCode,
                    ageBin: record.ageBin,
                    pressureLevel: record.pressureLevel,
                  }).toString();
                  navigate(`/overview?tab=d3&${qs}`);
                },
              })}
            />
          </Card>
        </Col>
      </Row>
    </div>
  );
};

export default Dashboard;
