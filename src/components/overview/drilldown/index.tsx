/**
 * Drilldown Drawer 主组件
 * 协调各子内容组件的展示
 *
 * 重构后：1113 行 → ~140 行 (-87%)
 */

import React, { useMemo, useState } from 'react';
import { Alert, Button, Descriptions, Drawer, Modal, Space, Typography } from 'antd';
import type { DrilldownSpec, WorkbenchTabKey } from '../../../hooks/useRiskOverviewData';
import type {
  BottleneckPoint,
  CapacityOpportunity,
  ColdStockBucket,
  DaySummary,
  OrderFailure,
  RollCampaignAlert,
} from '../../../types/decision';

import { OrdersContent } from './OrdersContent';
import { ColdStockContent } from './ColdStockContent';
import { BottleneckContent } from './BottleneckContent';
import { RollAlertContent } from './RollAlertContent';
import { RiskDayContent } from './RiskDayContent';
import { CapacityOpportunityContent } from './CapacityOpportunityContent';

const { Text } = Typography;

interface DrilldownDrawerProps {
  open: boolean;
  onClose: () => void;
  spec: DrilldownSpec | null;
  loading?: boolean;
  error?: unknown;
  onRetry?: () => void;
  onGoWorkbench?: (opts: {
    workbenchTab?: WorkbenchTabKey;
    machineCode?: string | null;
    urgencyLevel?: string | null;
  }) => void;

  riskDays: DaySummary[];
  bottlenecks: BottleneckPoint[];
  orderFailures: OrderFailure[];
  coldStockBuckets: ColdStockBucket[];
  rollAlerts: RollCampaignAlert[];
  capacityOpportunities: CapacityOpportunity[];
}

function titleFor(spec: DrilldownSpec | null) {
  if (!spec) return '详情';
  switch (spec.kind) {
    case 'orders':
      return '订单失败集合';
    case 'coldStock':
      return '冷坨高压力';
    case 'bottleneck':
      return '堵塞矩阵';
    case 'roll':
      return '换辊警报';
    case 'risk':
      return '风险摘要';
    case 'capacityOpportunity':
      return '容量优化机会';
    default:
      return '详情';
  }
}

const DrilldownDrawer: React.FC<DrilldownDrawerProps> = ({
  open,
  onClose,
  spec,
  loading,
  error,
  onRetry,
  onGoWorkbench,
  riskDays,
  bottlenecks,
  orderFailures,
  coldStockBuckets,
  rollAlerts,
  capacityOpportunities,
}) => {
  const [detailOpen, setDetailOpen] = useState(false);
  const [detailRecord, setDetailRecord] = useState<any>(null);

  const handleViewDetail = (record: any) => {
    setDetailRecord(record);
    setDetailOpen(true);
  };

  const content = useMemo(() => {
    if (!spec) return null;

    if (spec.kind === 'orders') {
      return (
        <OrdersContent
          rows={orderFailures}
          urgencyFilter={spec.urgency}
          onGoWorkbench={onGoWorkbench}
          onViewDetail={handleViewDetail}
        />
      );
    }

    if (spec.kind === 'coldStock') {
      return (
        <ColdStockContent
          buckets={coldStockBuckets}
          machineCodeFilter={spec.machineCode}
          ageBinFilter={spec.ageBin}
          pressureLevelFilter={spec.pressureLevel}
          onGoWorkbench={onGoWorkbench}
          onViewDetail={handleViewDetail}
        />
      );
    }

    if (spec.kind === 'bottleneck') {
      return (
        <BottleneckContent
          bottlenecks={bottlenecks}
          machineCodeFilter={spec.machineCode}
          planDateFilter={spec.planDate}
          onGoWorkbench={onGoWorkbench}
          onViewDetail={handleViewDetail}
        />
      );
    }

    if (spec.kind === 'roll') {
      return (
        <RollAlertContent
          alerts={rollAlerts}
          machineCodeFilter={spec.machineCode}
          onGoWorkbench={onGoWorkbench}
          onViewDetail={handleViewDetail}
        />
      );
    }

    if (spec.kind === 'capacityOpportunity') {
      return (
        <CapacityOpportunityContent
          opportunities={capacityOpportunities}
          machineCodeFilter={spec.machineCode}
          planDateFilter={spec.planDate}
          onGoWorkbench={onGoWorkbench}
          onViewDetail={handleViewDetail}
        />
      );
    }

    if (spec.kind === 'risk') {
      return (
        <RiskDayContent
          riskDays={riskDays}
          planDateFilter={spec.planDate}
          onGoWorkbench={onGoWorkbench}
          onViewDetail={handleViewDetail}
        />
      );
    }

    return null;
  }, [spec, orderFailures, coldStockBuckets, bottlenecks, rollAlerts, riskDays, capacityOpportunities, onGoWorkbench]);

  return (
    <>
      <Drawer
        title={titleFor(spec)}
        open={open}
        width={900}
        onClose={onClose}
        destroyOnClose
        extra={
          onRetry ? (
            <Space>
              <Button onClick={onRetry}>重试</Button>
            </Space>
          ) : null
        }
      >
        {!spec ? (
          <Text type="secondary">请选择一项问题查看详情</Text>
        ) : error ? (
          <Alert
            type="error"
            showIcon
            message="数据加载失败"
            description={<Text type="secondary">{String((error as any)?.message || error)}</Text>}
            action={onRetry ? <Button onClick={onRetry}>重试</Button> : undefined}
          />
        ) : (
          <div style={{ opacity: loading ? 0.6 : 1 }}>{content}</div>
        )}
      </Drawer>

      <Modal
        title="详情"
        open={detailOpen}
        onCancel={() => setDetailOpen(false)}
        footer={<Button onClick={() => setDetailOpen(false)}>关闭</Button>}
        width={720}
      >
        {detailRecord ? (
          <Descriptions size="small" column={1} bordered>
            {Object.entries(detailRecord).map(([k, v]) => (
              <Descriptions.Item key={k} label={k}>
                {Array.isArray(v) ? v.join(', ') : typeof v === 'object' ? JSON.stringify(v) : String(v)}
              </Descriptions.Item>
            ))}
          </Descriptions>
        ) : null}
      </Modal>
    </>
  );
};

export default React.memo(DrilldownDrawer);
