/**
 * 产能池管理日历视图 - 主容器
 * 职责：组件编排、状态管理、布局协调
 */

import React, { useState } from 'react';
import { Row, Col, Select, Card, Space } from 'antd';
import dayjs from 'dayjs';
import { useActiveVersionId } from '../../stores/use-global-store';
import MachineConfigPanel from './MachineConfigPanel';
import CalendarViewSwitcher from './CalendarViewSwitcher';
import CapacityCalendar from './CapacityCalendar';
import CapacityDetailDrawer from './CapacityDetailDrawer';
import BatchAdjustModal from './BatchAdjustModal';
import GlobalStatisticsCards from './GlobalStatisticsCards';
import { useGlobalCapacityStats } from '../../hooks/useGlobalCapacityStats';
import type { ViewMode } from './types';
import type { CapacityPoolCalendarData } from '../../api/ipcSchemas/machineConfigSchemas';

// 可用机组列表（与后端配置保持一致）
const AVAILABLE_MACHINES = ['H031', 'H032', 'H033', 'H034'];

export const CapacityPoolManagementV2: React.FC = () => {
  const currentVersionId = useActiveVersionId();

  // ========== 全局状态 ==========
  const [selectedMachines, setSelectedMachines] = useState<string[]>([AVAILABLE_MACHINES[0]]);
  const [viewMode, setViewMode] = useState<ViewMode>('day');
  const [dateRange, setDateRange] = useState<{ dateFrom: string; dateTo: string }>(() => {
    const today = dayjs();
    return {
      dateFrom: today.format('YYYY-MM-DD'),
      dateTo: today.add(29, 'day').format('YYYY-MM-DD'),
    };
  });

  // ========== 详情抽屉状态 ==========
  const [detailDrawerOpen, setDetailDrawerOpen] = useState(false);
  const [selectedDateData] = useState<CapacityPoolCalendarData | null>(null);

  // ========== 批量调整状态 ==========
  const [batchAdjustModalOpen, setBatchAdjustModalOpen] = useState(false);
  const [selectedDates, setSelectedDates] = useState<string[]>([]);

  // ========== 刷新标记 ==========
  const [refreshKey, setRefreshKey] = useState(0);

  // ========== 全局产能统计 ==========
  const { globalStats, loading: statsLoading } = useGlobalCapacityStats(
    currentVersionId || '',
    selectedMachines,
    dateRange.dateFrom,
    dateRange.dateTo
  );

  // ========== 事件处理 ==========
  const handleDateRangeChange = (dateFrom: string, dateTo: string) => {
    setDateRange({ dateFrom, dateTo });
  };

  const handleConfigUpdated = () => {
    // 配置更新后，刷新日历数据
    setRefreshKey((prev) => prev + 1);
  };

  const handleCapacityUpdated = () => {
    // 产能调整后，刷新日历数据
    setRefreshKey((prev) => prev + 1);
    setDetailDrawerOpen(false);
  };

  const handleBatchAdjustUpdated = () => {
    // 批量调整后，刷新日历数据
    setRefreshKey((prev) => prev + 1);
    setBatchAdjustModalOpen(false);
    setSelectedDates([]);
  };

  const handleMachineSelectionChange = (machines: string[]) => {
    setSelectedMachines(machines.length > 0 ? machines : [AVAILABLE_MACHINES[0]]);
  };

  return (
    <div style={{ padding: 16, height: 'calc(100vh - 120px)', overflow: 'auto' }}>
      <Space direction="vertical" style={{ width: '100%' }} size={16}>
        {/* 顶部控制区 */}
        <Card size="small" bodyStyle={{ padding: '8px 12px' }}>
          <Row gutter={12} align="middle">
            <Col flex="100px">
              <span style={{ fontSize: 13, fontWeight: 500 }}>机组选择</span>
            </Col>
            <Col flex="auto">
              <Select
                mode="multiple"
                style={{ width: '100%' }}
                placeholder="请选择一个或多个机组"
                value={selectedMachines}
                onChange={handleMachineSelectionChange}
                maxTagCount={3}
                size="small"
                options={AVAILABLE_MACHINES.map((code) => ({
                  label: code,
                  value: code,
                }))}
              />
            </Col>
            <Col>
              <CalendarViewSwitcher
                viewMode={viewMode}
                onViewModeChange={setViewMode}
                dateRange={dateRange}
                onDateRangeChange={handleDateRangeChange}
              />
            </Col>
          </Row>
        </Card>

        {/* 全局产能统计 */}
        <GlobalStatisticsCards stats={globalStats} loading={statsLoading} />

        {/* 主内容区 */}
        <Row gutter={16}>
          {/* 左侧：批量配置管理面板 */}
          <Col span={6}>
            <div style={{ position: 'sticky', top: 0 }}>
              <MachineConfigPanel
                versionId={currentVersionId || ''}
                onConfigApplied={handleConfigUpdated}
              />
            </div>
          </Col>

          {/* 右侧：日历视图区 */}
          <Col span={18}>
            <Space direction="vertical" style={{ width: '100%' }} size={12}>
              {selectedMachines.map((machine) => (
                <CapacityCalendar
                  key={`${machine}-${refreshKey}`}
                  versionId={currentVersionId || ''}
                  machineCode={machine}
                  dateFrom={dateRange.dateFrom}
                  dateTo={dateRange.dateTo}
                  viewMode={viewMode}
                />
              ))}
            </Space>
          </Col>
        </Row>
      </Space>

      {/* 详情抽屉 */}
      <CapacityDetailDrawer
        open={detailDrawerOpen}
        onClose={() => setDetailDrawerOpen(false)}
        versionId={currentVersionId || ''}
        data={selectedDateData}
        onUpdated={handleCapacityUpdated}
      />

      {/* 批量调整模态框 */}
      <BatchAdjustModal
        open={batchAdjustModalOpen}
        onClose={() => setBatchAdjustModalOpen(false)}
        versionId={currentVersionId || ''}
        machineCode={selectedMachines[0] || AVAILABLE_MACHINES[0]}
        selectedDates={selectedDates}
        onUpdated={handleBatchAdjustUpdated}
      />
    </div>
  );
};

export default CapacityPoolManagementV2;
