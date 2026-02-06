/**
 * Drilldown Drawer 主组件
 * 协调各子内容组件的展示
 *
 * 重构后：1113 行 → ~140 行 (-87%)
 */

import React, { useMemo, useState } from 'react';
import { Alert, Button, Descriptions, Drawer, Modal, Space, Tag, Typography } from 'antd';
import type { DrilldownSpec, WorkbenchTabKey } from '../../../hooks/useRiskOverviewData';
import type {
  BottleneckPoint,
  CapacityOpportunity,
  ColdStockBucket,
  DaySummary,
  OrderFailure,
  RollCampaignAlert,
  ReasonItem,
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
  // 压库等级/年龄分桶/结构缺口
  PRESSURE_LEVEL_LABELS,
  AGE_BIN_LABELS,
  STRUCTURE_GAP_LABELS,
  // 机会类型
  OPPORTUNITY_TYPE_LABELS,
  // 换辊状态
  ROLL_STATUS_LABELS,
  getAlertLevelLabel,
} from '../../../types/decision';

import { OrdersContent } from './OrdersContent';
import { ColdStockContent } from './ColdStockContent';
import { BottleneckContent } from './BottleneckContent';
import { RollAlertContent } from './RollAlertContent';
import { RiskDayContent } from './RiskDayContent';
import { CapacityOpportunityContent } from './CapacityOpportunityContent';
import { ReasonTable } from './shared';
import { formatNumber, formatWeight } from '../../../utils/formatters';

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
  pendingMaterialCount: '未排材料数（≤当日）',
  pendingWeightT: '未排重量（吨，≤当日）',
  scheduledMaterialCount: '已排材料数',

  // DaySummary - 日期风险摘要
  riskScore: '风险分数',
  riskLevel: '风险等级',
  overloadWeightT: '超载重量（吨）',
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
  totalWeightT: '总重量（吨）',
  scheduledWeightT: '已排重量（吨）',
  unscheduledWeightT: '未排重量（吨）',
  blockingFactors: '阻塞因素',
  failureReasons: '失败原因',
  factorType: '因素类型',
  impact: '影响权重',
  affectedMaterialCount: '受影响材料数',

  // ColdStockBucket - 冷料分桶
  ageBin: '库龄分桶',
  count: '数量',
  weightT: '重量（吨）',
  pressureScore: '压库分数',
  pressureLevel: '压库等级',
  avgAgeDays: '平均库龄（天）',
  maxAgeDays: '最大库龄（天）',
  structureGap: '结构性缺口',
  trend: '趋势',
  direction: '趋势方向',
  changeRatePct: '变化率',

  // RollCampaignAlert - 换辊警报
  campaignId: '轧制活动编号',
  campaignStartDate: '轧制活动开始日期',
  campaignStartAt: '周期起点时刻',
  plannedChangeAt: '计划换辊时刻',
  plannedDowntimeMinutes: '计划停机时长（分钟）',
  planedDowntimeMinutes: '计划停机时长（分钟）',
  currentTonnageT: '当前吨位（吨）',
  softLimitT: '软限制（吨）',
  hardLimitT: '硬限制（吨）',
  remainingTonnageT: '剩余吨位（吨）',
  alertLevel: '警报等级',
  alertType: '警报类型',
  estimatedHardStopDate: '预计硬停日期',
  estimatedSoftReachAt: '预计触达软限制时刻',
  estimatedHardReachAt: '预计触达硬限制时刻',
  alertMessage: '警报消息',
  impactDescription: '影响描述',

  // CapacityOpportunity - 容量优化机会
  opportunityType: '机会类型',
  currentUtilPct: '当前利用率',
  targetCapacityT: '目标容量（吨）',
  usedCapacityT: '已用容量（吨）',
  opportunitySpaceT: '机会空间（吨）',
  optimizedUtilPct: '优化后利用率',
  sensitivity: '敏感性分析',
  scenarios: '方案列表',
  bestScenarioIndex: '最优方案索引',
  name: '方案名称',
  adjustment: '调整策略',
  utilPct: '利用率',
  description: '描述',
  potentialBenefits: '潜在收益',
};

/**
 * 获取字段的汉化标签
 */
function getFieldLabel(fieldName: string): string {
  return FIELD_LABEL_MAP[fieldName] || '其他字段';
}

const ALERT_TYPE_LABELS: Record<string, string> = {
  HARD_LIMIT_EXCEEDED: '硬上限超限',
  SOFT_LIMIT_EXCEEDED: '软限制触达',
  NORMAL: '正常',
  CAPACITY_HIGH: '产能高负荷',
  CAPACITY_MEDIUM: '产能中负荷',
  CAPACITY_LOW: '产能低负荷',
};

const BLOCKING_FACTOR_LABELS: Record<string, string> = {
  COLD_STOCK: '冷料未适温',
  STRUCTURE_CONFLICT: '结构冲突',
  CAPACITY_SHORTAGE: '产能不足',
};

const TREND_DIRECTION_LABELS: Record<string, string> = {
  RISING: '上升',
  STABLE: '稳定',
  FALLING: '下降',
};

const DATE_ONLY_FIELDS = new Set<string>(['planDate', 'dueDate', 'campaignStartDate', 'estimatedHardStopDate']);
const DATE_TIME_FIELDS = new Set<string>([
  'campaignStartAt',
  'plannedChangeAt',
  'estimatedSoftReachAt',
  'estimatedHardReachAt',
]);

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
  structureGap: STRUCTURE_GAP_LABELS,
  status: ROLL_STATUS_LABELS,
  alertType: ALERT_TYPE_LABELS,
  factorType: BLOCKING_FACTOR_LABELS,
  direction: TREND_DIRECTION_LABELS,
};

function formatDateValue(rawValue: unknown, withTime: boolean): string {
  const text = String(rawValue || '').trim();
  if (!text) {
    return '-';
  }
  const normalized = text.replace('T', ' ').replace('Z', '');
  const dateMatch = normalized.match(/^(\d{4}-\d{2}-\d{2})/);
  if (!dateMatch) {
    return text;
  }
  if (!withTime) {
    return dateMatch[1];
  }
  const timeMatch = normalized.match(/^\d{4}-\d{2}-\d{2}\s(\d{2}:\d{2})(?::(\d{2}))?/);
  if (!timeMatch) {
    return `${dateMatch[1]} 00:00:00`;
  }
  const seconds = timeMatch[2] || '00';
  return `${dateMatch[1]} ${timeMatch[1]}:${seconds}`;
}

function formatObjectValue(value: Record<string, unknown>): string {
  const entries = Object.entries(value);
  if (entries.length === 0) {
    return '-';
  }
  return entries
    .map(([entryKey, entryValue]) => `${getFieldLabel(entryKey)}：${formatFieldValue(entryKey, entryValue)}`)
    .join('；');
}

/**
 * 格式化字段值，对枚举类型进行汉化
 */
function formatFieldValue(fieldName: string, value: unknown): string {
  if (value === null || value === undefined) {
    return '-';
  }

  if (Array.isArray(value)) {
    const labelMap = ENUM_VALUE_LABELS[fieldName];
    if (labelMap) {
      return value.map((item) => labelMap[String(item)] || '其他').join('、');
    }
    return value
      .map((item) => {
        if (item === null || item === undefined) {
          return '-';
        }
        if (typeof item === 'object') {
          return formatObjectValue(item as Record<string, unknown>);
        }
        return String(item);
      })
      .join('；');
  }

  if (typeof value === 'object') {
    return formatObjectValue(value as Record<string, unknown>);
  }

  if (fieldName === 'alertLevel') {
    return getAlertLevelLabel(String(value));
  }

  const labelMap = ENUM_VALUE_LABELS[fieldName];
  if (labelMap) {
    return labelMap[String(value)] || '其他';
  }

  if (DATE_TIME_FIELDS.has(fieldName) || fieldName.endsWith('At')) {
    return formatDateValue(value, true);
  }

  if (DATE_ONLY_FIELDS.has(fieldName) || fieldName.endsWith('Date')) {
    return formatDateValue(value, false);
  }

  if (fieldName.endsWith('Pct') || fieldName === 'completionRate' || fieldName === 'impact') {
    return `${formatNumber(Number(value), 2)}%`;
  }

  if (fieldName.endsWith('Score')) {
    return formatNumber(Number(value), 2);
  }

  if (fieldName.endsWith('Days')) {
    return formatNumber(Number(value), 2);
  }

  if (fieldName.endsWith('T')) {
    return formatWeight(Number(value));
  }

  return String(value);
}

/**
 * 渲染字段值（支持复杂类型，返回React节点）
 */
function renderFieldValue(fieldName: string, value: unknown): React.ReactNode {
  if (value === null || value === undefined) {
    return '-';
  }

  if (fieldName === 'reasons' || fieldName === 'topReasons' || fieldName === 'failureReasons') {
    if (Array.isArray(value) && value.length > 0) {
      const firstItem = value[0];
      if (firstItem && typeof firstItem === 'object' && 'code' in firstItem && 'msg' in firstItem) {
        return (
          <div style={{ marginTop: 8 }}>
            <ReasonTable reasons={value as ReasonItem[]} />
          </div>
        );
      }
      return (
        <Space direction="vertical" size={4} style={{ width: '100%' }}>
          {value.map((item, index) => (
            <Text key={index}>{String(item)}</Text>
          ))}
        </Space>
      );
    }
    return <Text type="secondary">暂无原因</Text>;
  }

  if (Array.isArray(value)) {
    const labelMap = ENUM_VALUE_LABELS[fieldName];
    if (labelMap) {
      return (
        <Space wrap>
          {value.map((item, index) => (
            <Tag key={index}>{labelMap[String(item)] || '其他'}</Tag>
          ))}
        </Space>
      );
    }
    return (
      <Space direction="vertical" size={4} style={{ width: '100%' }}>
        {value.map((item, index) => (
          <Text key={index}>{formatFieldValue(fieldName, item)}</Text>
        ))}
      </Space>
    );
  }

  if (typeof value === 'object') {
    return <Text style={{ whiteSpace: 'pre-wrap' }}>{formatFieldValue(fieldName, value)}</Text>;
  }

  return formatFieldValue(fieldName, value);
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
        width={880}
      >
        {detailRecord ? (
          <Descriptions size="small" column={1} bordered layout="vertical">
            {Object.entries(detailRecord).map(([k, v]) => (
              <Descriptions.Item key={k} label={getFieldLabel(k)}>
                {renderFieldValue(k, v)}
              </Descriptions.Item>
            ))}
          </Descriptions>
        ) : null}
      </Modal>
    </>
  );
};

export default React.memo(DrilldownDrawer);
