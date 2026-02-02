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
import {
  // 堵塞等级/类型
  BOTTLENECK_LEVEL_LABELS,
  BOTTLENECK_TYPE_LABELS,
  // 风险等级
  RISK_LEVEL_LABELS,
  // 紧急等级/失败类型
  URGENCY_LEVEL_LABELS,
  FAIL_TYPE_LABELS,
  // 压库等级/年龄分桶
  PRESSURE_LEVEL_LABELS,
  AGE_BIN_LABELS,
  // 机会类型
  OPPORTUNITY_TYPE_LABELS,
  // 换辊状态
  getAlertLevelLabel,
} from '../../../types/decision';

import { OrdersContent } from './OrdersContent';
import { ColdStockContent } from './ColdStockContent';
import { BottleneckContent } from './BottleneckContent';
import { RollAlertContent } from './RollAlertContent';
import { RiskDayContent } from './RiskDayContent';
import { CapacityOpportunityContent } from './CapacityOpportunityContent';

const { Text } = Typography;

/**
 * 字段名汉化映射表
 * 覆盖所有决策层数据类型的字段
 */
const FIELD_LABEL_MAP: Record<string, string> = {
  // 通用字段
  machineCode: '机组代码',
  planDate: '计划日期',
  reasons: '原因',
  recommendedActions: '推荐行动',

  // BottleneckPoint - 堵塞点
  bottleneckScore: '堵塞分数',
  bottleneckLevel: '堵塞等级',
  bottleneckTypes: '堵塞类型',
  capacityUtilPct: '容量利用率',
  pendingMaterialCount: '缺口材料数(≤当日)',
  pendingWeightT: '缺口重量(吨, ≤当日)',
  scheduledMaterialCount: '已排材料数',
  // scheduledWeightT 在 OrderFailure 部分已定义

  // DaySummary - 日期风险摘要
  riskScore: '风险分数',
  riskLevel: '风险等级',
  overloadWeightT: '超载重量(吨)',
  urgentFailureCount: '紧急失败数',
  topReasons: '主要原因',
  involvedMachines: '涉及机组',

  // OrderFailure - 订单失败
  contractNo: '合同号',
  dueDate: '交货日期',
  daysToDue: '到期天数',
  urgencyLevel: '紧急等级',
  failType: '失败类型',
  completionRate: '完成率',
  totalWeightT: '总重量(吨)',
  scheduledWeightT: '已排重量(吨)',
  unscheduledWeightT: '未排重量(吨)',
  blockingFactors: '阻塞因素',
  failureReasons: '失败原因',

  // ColdStockBucket - 冷料分桶
  ageBin: '库龄分桶',
  count: '数量',
  weightT: '重量(吨)',
  pressureScore: '压库分数',
  pressureLevel: '压库等级',
  avgAgeDays: '平均库龄(天)',
  maxAgeDays: '最大库龄(天)',
  structureGap: '结构性缺口',
  trend: '趋势',

  // RollCampaignAlert - 换辊警报
  campaignId: '轧制活动ID',
  campaignStartDate: '轧制活动开始日期',
  currentTonnageT: '当前吨位(吨)',
  softLimitT: '软限制(吨)',
  hardLimitT: '硬限制(吨)',
  remainingTonnageT: '剩余吨位(吨)',
  alertLevel: '警报等级',
  alertType: '警报类型',
  estimatedHardStopDate: '预计硬停日期',
  alertMessage: '警报消息',
  impactDescription: '影响描述',

  // CapacityOpportunity - 容量优化机会
  opportunityType: '机会类型',
  currentUtilPct: '当前利用率',
  targetCapacityT: '目标容量(吨)',
  usedCapacityT: '已用容量(吨)',
  opportunitySpaceT: '机会空间(吨)',
  optimizedUtilPct: '优化后利用率',
  sensitivity: '敏感性分析',
  description: '描述',
  potentialBenefits: '潜在收益',
};

/**
 * 获取字段的汉化标签
 */
function getFieldLabel(fieldName: string): string {
  return FIELD_LABEL_MAP[fieldName] || fieldName;
}

/**
 * 枚举值汉化映射表
 * 根据字段名对特定枚举值进行汉化
 */
const ENUM_VALUE_LABELS: Record<string, Record<string, string>> = {
  bottleneckLevel: BOTTLENECK_LEVEL_LABELS,
  bottleneckTypes: BOTTLENECK_TYPE_LABELS,
  riskLevel: RISK_LEVEL_LABELS,
  urgencyLevel: URGENCY_LEVEL_LABELS,
  failType: FAIL_TYPE_LABELS,
  pressureLevel: PRESSURE_LEVEL_LABELS,
  ageBin: AGE_BIN_LABELS,
  opportunityType: OPPORTUNITY_TYPE_LABELS,
};

/**
 * 格式化字段值，对枚举类型进行汉化
 */
function formatFieldValue(fieldName: string, value: unknown): string {
  if (value === null || value === undefined) {
    return '-';
  }

  // 处理数组类型
  if (Array.isArray(value)) {
    const labelMap = ENUM_VALUE_LABELS[fieldName];
    if (labelMap) {
      // 数组中每个枚举值都进行汉化
      return value.map((v) => labelMap[String(v)] || String(v)).join(', ');
    }
    // 普通数组，直接 join
    return value.map((v) => (typeof v === 'object' ? JSON.stringify(v) : String(v))).join(', ');
  }

  // 处理对象类型（如 trend、reasons 等）
  if (typeof value === 'object') {
    return JSON.stringify(value);
  }

  // 特殊处理 alertLevel 字段（需要通过 parseAlertLevel 转换）
  if (fieldName === 'alertLevel') {
    return getAlertLevelLabel(String(value));
  }

  // 处理枚举值
  const labelMap = ENUM_VALUE_LABELS[fieldName];
  if (labelMap) {
    return labelMap[String(value)] || String(value);
  }

  // 处理百分比字段
  if (fieldName.endsWith('Pct') || fieldName === 'completionRate') {
    return `${Number(value).toFixed(1)}%`;
  }

  return String(value);
}

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
              <Descriptions.Item key={k} label={getFieldLabel(k)}>
                {formatFieldValue(k, v)}
              </Descriptions.Item>
            ))}
          </Descriptions>
        ) : null}
      </Modal>
    </>
  );
};

export default React.memo(DrilldownDrawer);
