// ==========================================
// D2决策：订单失败看板页面
// ==========================================
// 职责: 以看板（Kanban）方式展示紧急订单失败情况，按紧急度分组
// ==========================================

import React, { useEffect, useMemo, useState } from 'react';
import { Card, Row, Col, Statistic, Tag, Spin, Alert, Space, Select, Badge, Descriptions, Progress } from 'antd';
import {
  WarningOutlined,
  ThunderboltOutlined,
  ClockCircleOutlined,
  ExclamationCircleOutlined,
} from '@ant-design/icons';
import { useSearchParams } from 'react-router-dom';
import { useAllFailedOrders } from '../../hooks/queries/use-decision-queries';
import type { DrilldownSpec } from '../../hooks/useRiskOverviewData';
import { useActiveVersionId } from '../../stores/use-global-store';
import { EmptyState } from '../../components/EmptyState';
import type { OrderFailure, UrgencyLevel, FailType } from '../../types/decision';

// ==========================================
// 紧急度颜色映射
// ==========================================
const URGENCY_LEVEL_COLORS: Record<UrgencyLevel, string> = {
  L3: '#ff4d4f', // 超紧急 - 红色
  L2: '#faad14', // 紧急 - 橙色
  L1: '#1677ff', // 较紧急 - 蓝色
  L0: '#8c8c8c', // 正常 - 灰色
};

const URGENCY_LEVEL_LABELS: Record<UrgencyLevel, string> = {
  L3: '超紧急',
  L2: '紧急',
  L1: '较紧急',
  L0: '正常',
};

// ==========================================
// 失败类型颜色映射
// ==========================================
const FAIL_TYPE_COLORS: Record<FailType, string> = {
  Overdue: '#ff4d4f',
  NearDueImpossible: '#faad14',
  CapacityShortage: '#fa8c16',
  StructureConflict: '#1677ff',
  ColdStockNotReady: '#13c2c2',
  Other: '#8c8c8c',
};

const FAIL_TYPE_LABELS: Record<FailType, string> = {
  Overdue: '超期未完成',
  NearDueImpossible: '临期无法完成',
  CapacityShortage: '产能不足',
  StructureConflict: '结构冲突',
  ColdStockNotReady: '冷料未适温',
  Other: '其他',
};

/**
 * 阻塞因素类型中文翻译映射
 */
const FACTOR_TYPE_LABELS: Record<string, string> = {
  Capacity: '产能不足',
  Structure: '结构性堵塞',
  RollChange: '换辊影响',
  ColdStock: '冷料压库',
  Material: '物料约束',
  Temperature: '温度约束',
  Maturity: '适温约束',
  Mixed: '混合因素',
  CapacityShortage: '产能不足',
  StructureConflict: '结构冲突',
  ColdStockNotReady: '冷料未适温',
};

function getFactorTypeLabel(type: string): string {
  return FACTOR_TYPE_LABELS[type] || type;
}

const getFailTypeColor = (type: string) =>
  (FAIL_TYPE_COLORS as Record<string, string>)[type] || '#8c8c8c';
const getFailTypeLabel = (type: string) =>
  (FAIL_TYPE_LABELS as Record<string, string>)[type] || type;

// ==========================================
// 主组件
// ==========================================

interface D2OrderFailureProps {
  embedded?: boolean;
  onOpenDrilldown?: (spec: DrilldownSpec) => void;
}

export const D2OrderFailure: React.FC<D2OrderFailureProps> = ({ embedded, onOpenDrilldown }) => {
  const versionId = useActiveVersionId();
  const [searchParams] = useSearchParams();
  const [selectedUrgency, setSelectedUrgency] = useState<UrgencyLevel | 'ALL'>(() => {
    const urgency = searchParams.get('urgency');
    if (urgency && ['L0', 'L1', 'L2', 'L3'].includes(urgency)) {
      return urgency as UrgencyLevel;
    }
    return 'ALL';
  });
  const [selectedFailType, setSelectedFailType] = useState<FailType | 'ALL'>(() => {
    const failType = searchParams.get('failType');
    if (
      failType &&
      [
        'Overdue',
        'NearDueImpossible',
        'CapacityShortage',
        'StructureConflict',
        'ColdStockNotReady',
        'Other',
      ].includes(failType)
    ) {
      return failType as FailType;
    }
    return 'ALL';
  });
  const [selectedOrder, setSelectedOrder] = useState<OrderFailure | null>(null);

  const handleOrderClick = (order: OrderFailure) => {
    if (embedded && onOpenDrilldown) {
      setSelectedOrder(order);
      onOpenDrilldown({ kind: 'orders', urgency: order.urgencyLevel });
      return;
    }
    setSelectedOrder(order);
  };

  // 获取失败订单数据
  const { data, isLoading, error } = useAllFailedOrders(versionId);

  // 支持 Dashboard drill-down：通过 querystring 设置默认筛选/选中项
  useEffect(() => {
    const urgency = searchParams.get('urgency');
    if (urgency && ['L0', 'L1', 'L2', 'L3'].includes(urgency)) {
      setSelectedUrgency(urgency as UrgencyLevel);
    }
  }, [searchParams]);

  useEffect(() => {
    const failType = searchParams.get('failType');
    if (
      failType &&
      [
        'Overdue',
        'NearDueImpossible',
        'CapacityShortage',
        'StructureConflict',
        'ColdStockNotReady',
        'Other',
      ].includes(failType)
    ) {
      setSelectedFailType(failType as FailType);
    }
  }, [searchParams]);

  useEffect(() => {
    const contractNo = searchParams.get('contractNo');
    if (!contractNo || !data?.items) return;
    const found = data.items.find((o) => o.contractNo === contractNo);
    if (found) {
      setSelectedOrder(found);
    }
  }, [searchParams, data]);

  // 按紧急度分组
  const groupedOrders = useMemo(() => {
    if (!data?.items) return { L0: [], L1: [], L2: [], L3: [] };

    const groups: Record<UrgencyLevel, OrderFailure[]> = {
      L0: [],
      L1: [],
      L2: [],
      L3: [],
    };

    data.items.forEach((order) => {
      if (selectedFailType !== 'ALL' && order.failType !== selectedFailType) return;
      groups[order.urgencyLevel].push(order);
    });

    // 每个组内按到期日期排序
    Object.keys(groups).forEach((level) => {
      groups[level as UrgencyLevel].sort((a, b) => a.daysToDue - b.daysToDue);
    });

    return groups;
  }, [data]);

  // 过滤显示的订单
  const displayOrders = useMemo(() => {
    if (selectedUrgency === 'ALL') {
      return (['L3', 'L2', 'L1', 'L0'] as UrgencyLevel[]).flatMap((level) => groupedOrders[level]);
    }
    return groupedOrders[selectedUrgency];
  }, [groupedOrders, selectedUrgency]);

  // H7修复：区分全局统计和筛选视图统计，避免用户误解
  // 全局统计（基于所有订单，不受筛选影响）
  const globalStats = useMemo(() => {
    const allOrders = data?.items || [];
    if (allOrders.length === 0) {
      return {
        totalFailures: 0,
        overdueCount: 0,
        nearDueImpossibleCount: 0,
        avgCompletionRate: 0,
      };
    }

    const totalFailures = allOrders.length;
    const overdueCount = allOrders.filter((o) => o.failType === 'Overdue').length;
    const nearDueImpossibleCount = allOrders.filter((o) => o.failType === 'NearDueImpossible').length;
    const avgCompletionRate =
      allOrders.reduce((sum, o) => sum + o.completionRate, 0) / allOrders.length;

    return {
      totalFailures,
      overdueCount,
      nearDueImpossibleCount,
      avgCompletionRate: Math.round(avgCompletionRate),
    };
  }, [data?.items]);

  // 当前筛选视图统计（基于筛选后的订单）
  const viewStats = useMemo(() => {
    if (!displayOrders || displayOrders.length === 0) {
      return {
        totalFailures: 0,
        overdueCount: 0,
        nearDueImpossibleCount: 0,
        avgCompletionRate: 0,
      };
    }

    const totalFailures = displayOrders.length;
    const overdueCount = displayOrders.filter((o) => o.failType === 'Overdue').length;
    const nearDueImpossibleCount = displayOrders.filter((o) => o.failType === 'NearDueImpossible').length;
    const avgCompletionRate =
      displayOrders.reduce((sum, o) => sum + o.completionRate, 0) / displayOrders.length;

    return {
      totalFailures,
      overdueCount,
      nearDueImpossibleCount,
      avgCompletionRate: Math.round(avgCompletionRate),
    };
  }, [displayOrders]);

  // H7修复：判断是否有筛选条件，用于决定是否显示筛选提示
  const hasFilter = selectedUrgency !== 'ALL' || selectedFailType !== 'ALL';

  // ==========================================
  // 加载状态
  // ==========================================

  if (isLoading) {
    return (
      <div style={{ textAlign: 'center', padding: embedded ? '40px 0' : '100px 0' }}>
        <Spin size="large" tip="正在加载订单失败数据...">
          <div style={{ minHeight: 80 }} />
        </Spin>
      </div>
    );
  }

  // ==========================================
  // 错误状态
  // ==========================================

  if (error) {
    return (
      <Alert
        message="数据加载失败"
        description={error.message || '未知错误'}
        type="error"
        showIcon
        style={{ margin: embedded ? 0 : '20px' }}
      />
    );
  }

  if (!versionId) {
    return (
      <Alert
        message="未选择排产版本"
        description="请先在主界面选择一个排产版本"
        type="warning"
        showIcon
        style={{ margin: embedded ? 0 : '20px' }}
      />
    );
  }

  // ==========================================
  // 主界面
  // ==========================================

  return (
    <div style={{ padding: embedded ? 0 : 24 }}>
      {!embedded ? (
        <div style={{ marginBottom: 24 }}>
          <h2>
            <ThunderboltOutlined style={{ marginRight: 8 }} />
            D2决策：订单失败看板
          </h2>
          <p style={{ color: '#8c8c8c', marginBottom: 16 }}>
            展示紧急订单失败情况，按紧急度分组查看阻塞因素和推荐行动
          </p>

          <Space wrap>
            <Space>
              <span>紧急度筛选：</span>
              <Select
                value={selectedUrgency}
                onChange={setSelectedUrgency}
                style={{ width: 150 }}
                options={[
                  { label: '全部', value: 'ALL' },
                  { label: `${URGENCY_LEVEL_LABELS.L3} (${groupedOrders.L3.length})`, value: 'L3' },
                  { label: `${URGENCY_LEVEL_LABELS.L2} (${groupedOrders.L2.length})`, value: 'L2' },
                  { label: `${URGENCY_LEVEL_LABELS.L1} (${groupedOrders.L1.length})`, value: 'L1' },
                  { label: `${URGENCY_LEVEL_LABELS.L0} (${groupedOrders.L0.length})`, value: 'L0' },
                ]}
              />
            </Space>

            <Space>
              <span>失败类型筛选：</span>
              <Select
                value={selectedFailType}
                onChange={setSelectedFailType}
                style={{ width: 200 }}
                options={[
                  { label: '全部', value: 'ALL' },
                  { label: FAIL_TYPE_LABELS.Overdue, value: 'Overdue' },
                  { label: FAIL_TYPE_LABELS.NearDueImpossible, value: 'NearDueImpossible' },
                  { label: FAIL_TYPE_LABELS.CapacityShortage, value: 'CapacityShortage' },
                  { label: FAIL_TYPE_LABELS.StructureConflict, value: 'StructureConflict' },
                  { label: FAIL_TYPE_LABELS.ColdStockNotReady, value: 'ColdStockNotReady' },
                  { label: FAIL_TYPE_LABELS.Other, value: 'Other' },
                ]}
              />
            </Space>
          </Space>
        </div>
      ) : (
        <div style={{ marginBottom: 12 }}>
          <Space wrap>
            <Space>
              <span>紧急度：</span>
              <Select
                size="small"
                value={selectedUrgency}
                onChange={setSelectedUrgency}
                style={{ width: 150 }}
                options={[
                  { label: '全部', value: 'ALL' },
                  { label: `${URGENCY_LEVEL_LABELS.L3} (${groupedOrders.L3.length})`, value: 'L3' },
                  { label: `${URGENCY_LEVEL_LABELS.L2} (${groupedOrders.L2.length})`, value: 'L2' },
                  { label: `${URGENCY_LEVEL_LABELS.L1} (${groupedOrders.L1.length})`, value: 'L1' },
                  { label: `${URGENCY_LEVEL_LABELS.L0} (${groupedOrders.L0.length})`, value: 'L0' },
                ]}
              />
            </Space>

            <Space>
              <span>失败类型：</span>
              <Select
                size="small"
                value={selectedFailType}
                onChange={setSelectedFailType}
                style={{ width: 200 }}
                options={[
                  { label: '全部', value: 'ALL' },
                  { label: FAIL_TYPE_LABELS.Overdue, value: 'Overdue' },
                  { label: FAIL_TYPE_LABELS.NearDueImpossible, value: 'NearDueImpossible' },
                  { label: FAIL_TYPE_LABELS.CapacityShortage, value: 'CapacityShortage' },
                  { label: FAIL_TYPE_LABELS.StructureConflict, value: 'StructureConflict' },
                  { label: FAIL_TYPE_LABELS.ColdStockNotReady, value: 'ColdStockNotReady' },
                  { label: FAIL_TYPE_LABELS.Other, value: 'Other' },
                ]}
              />
            </Space>
          </Space>
        </div>
      )}

      {/* H7修复：统计卡片始终显示全局数据，避免筛选后的误导 */}
      {/* 当有筛选时，在标题中添加"（全局）"说明，并在下方显示当前筛选视图的统计 */}
      <Row gutter={embedded ? 12 : 16} style={{ marginBottom: embedded ? 12 : 24 }}>
        <Col span={6}>
          <Card size={embedded ? 'small' : undefined}>
            <Statistic
              title={hasFilter ? "失败订单总数（全局）" : "失败订单总数"}
              value={globalStats.totalFailures}
              prefix={<ExclamationCircleOutlined />}
              valueStyle={{ color: globalStats.totalFailures > 0 ? '#ff4d4f' : '#52c41a' }}
            />
            {hasFilter && (
              <div style={{ fontSize: 12, color: '#8c8c8c', marginTop: 8 }}>
                当前筛选: {viewStats.totalFailures}
              </div>
            )}
          </Card>
        </Col>
        <Col span={6}>
          <Card size={embedded ? 'small' : undefined}>
            <Statistic
              title={hasFilter ? "超期未完成（全局）" : "超期未完成"}
              value={globalStats.overdueCount}
              prefix={<WarningOutlined />}
              valueStyle={{ color: globalStats.overdueCount > 0 ? '#ff4d4f' : '#52c41a' }}
            />
            {hasFilter && (
              <div style={{ fontSize: 12, color: '#8c8c8c', marginTop: 8 }}>
                当前筛选: {viewStats.overdueCount}
              </div>
            )}
          </Card>
        </Col>
        <Col span={6}>
          <Card size={embedded ? 'small' : undefined}>
            <Statistic
              title={hasFilter ? "临期无法完成（全局）" : "临期无法完成"}
              value={globalStats.nearDueImpossibleCount}
              prefix={<ClockCircleOutlined />}
              valueStyle={{ color: globalStats.nearDueImpossibleCount > 0 ? '#faad14' : '#52c41a' }}
            />
            {hasFilter && (
              <div style={{ fontSize: 12, color: '#8c8c8c', marginTop: 8 }}>
                当前筛选: {viewStats.nearDueImpossibleCount}
              </div>
            )}
          </Card>
        </Col>
        <Col span={6}>
          <Card size={embedded ? 'small' : undefined}>
            <Statistic
              title={hasFilter ? "平均完成率（全局）" : "平均完成率"}
              value={globalStats.avgCompletionRate}
              suffix="%"
              valueStyle={{
                color: globalStats.avgCompletionRate < 50 ? '#ff4d4f' : globalStats.avgCompletionRate < 80 ? '#faad14' : '#52c41a',
              }}
            />
            {hasFilter && (
              <div style={{ fontSize: 12, color: '#8c8c8c', marginTop: 8 }}>
                当前筛选: {viewStats.avgCompletionRate}%
              </div>
            )}
          </Card>
        </Col>
      </Row>

      {/* 看板视图 */}
      {selectedUrgency === 'ALL' ? (
        <Row gutter={16}>
          {(['L3', 'L2', 'L1', 'L0'] as UrgencyLevel[]).map((level) => (
            <Col key={level} span={6}>
              <Card
                title={
                  <Space>
                    <Tag color={URGENCY_LEVEL_COLORS[level]}>{URGENCY_LEVEL_LABELS[level]}</Tag>
                    <Badge count={groupedOrders[level].length} showZero />
                  </Space>
                }
                bodyStyle={{ padding: '12px', maxHeight: '70vh', overflowY: 'auto' }}
              >
                {groupedOrders[level].map((order) => (
                  <OrderCard
                    key={order.contractNo}
                    order={order}
                    onClick={() => handleOrderClick(order)}
                    isSelected={selectedOrder?.contractNo === order.contractNo}
                  />
                ))}
                {groupedOrders[level].length === 0 && (
                  <EmptyState type="order" style={{ padding: '20px 0', minHeight: '120px' }} />
                )}
              </Card>
            </Col>
          ))}
        </Row>
      ) : (
        <Card
          title={
            <Space>
              <Tag color={URGENCY_LEVEL_COLORS[selectedUrgency]}>
                {URGENCY_LEVEL_LABELS[selectedUrgency]}
              </Tag>
              <Badge count={displayOrders.length} showZero />
            </Space>
          }
        >
          <Row gutter={16}>
            {displayOrders.map((order) => (
              <Col key={order.contractNo} span={6} style={{ marginBottom: '16px' }}>
                <OrderCard
                  order={order}
                  onClick={() => handleOrderClick(order)}
                  isSelected={selectedOrder?.contractNo === order.contractNo}
                />
              </Col>
            ))}
          </Row>
          {displayOrders.length === 0 && (
            <EmptyState type="order" style={{ padding: '40px 0' }} />
          )}
        </Card>
      )}

      {/* 选中订单的详细信息 */}
      {!embedded && selectedOrder && (
        <Card
          title={`订单详情: ${selectedOrder.contractNo}`}
          style={{ marginTop: '24px' }}
          extra={
            <Space>
              <Tag color={URGENCY_LEVEL_COLORS[selectedOrder.urgencyLevel]}>
                {URGENCY_LEVEL_LABELS[selectedOrder.urgencyLevel]}
              </Tag>
              <Tag color={getFailTypeColor(selectedOrder.failType)}>
                {getFailTypeLabel(selectedOrder.failType)}
              </Tag>
            </Space>
          }
        >
          <Descriptions column={3} bordered size="small" style={{ marginBottom: '16px' }}>
            <Descriptions.Item label="到期日期">{selectedOrder.dueDate}</Descriptions.Item>
            <Descriptions.Item label="距离到期">
              {selectedOrder.daysToDue}天
            </Descriptions.Item>
            <Descriptions.Item label="机组">{selectedOrder.machineCode}</Descriptions.Item>
            <Descriptions.Item label="总重量">
              {selectedOrder.totalWeightT.toFixed(3)}吨
            </Descriptions.Item>
            <Descriptions.Item label="已排产重量">
              {selectedOrder.scheduledWeightT.toFixed(3)}吨
            </Descriptions.Item>
            <Descriptions.Item label="未排产重量">
              {selectedOrder.unscheduledWeightT.toFixed(3)}吨
            </Descriptions.Item>
          </Descriptions>

          <div style={{ marginBottom: '16px' }}>
            <h4>完成进度</h4>
            <Progress
              percent={Math.round(selectedOrder.completionRate)}
              status={selectedOrder.completionRate < 50 ? 'exception' : selectedOrder.completionRate < 80 ? 'normal' : 'success'}
            />
          </div>

          {/* 阻塞因素 */}
          {selectedOrder.blockingFactors && selectedOrder.blockingFactors.length > 0 && (
            <>
              <h4 style={{ marginTop: '16px', marginBottom: '8px' }}>阻塞因素</h4>
              <Space direction="vertical" style={{ width: '100%' }}>
                {selectedOrder.blockingFactors.map((factor, index) => (
                  <Card key={index} size="small" type="inner">
                    <Space direction="vertical" style={{ width: '100%' }}>
                      <div>
                        <Tag color="red">{getFactorTypeLabel(factor.factorType)}</Tag>
                        <span style={{ fontWeight: 'bold' }}>{factor.description}</span>
                      </div>
                      <div>影响权重: {(factor.impact * 100).toFixed(2)}%</div>
                      <div>受影响材料数: {factor.affectedMaterialCount}个</div>
                    </Space>
                  </Card>
                ))}
              </Space>
            </>
          )}

          {/* 失败原因 */}
          {selectedOrder.failureReasons && selectedOrder.failureReasons.length > 0 && (
            <>
              <h4 style={{ marginTop: '16px', marginBottom: '8px' }}>失败原因</h4>
              <ul>
                {selectedOrder.failureReasons.map((reason, index) => (
                  <li key={index}>{reason}</li>
                ))}
              </ul>
            </>
          )}

          {/* 推荐行动 */}
          {selectedOrder.recommendedActions && selectedOrder.recommendedActions.length > 0 && (
            <>
              <h4 style={{ marginTop: '16px', marginBottom: '8px' }}>推荐行动</h4>
              <ul>
                {selectedOrder.recommendedActions.map((action, index) => (
                  <li key={index} style={{ color: '#1677ff', fontWeight: 'bold' }}>
                    {action}
                  </li>
                ))}
              </ul>
            </>
          )}
        </Card>
      )}
    </div>
  );
};

// ==========================================
// 订单卡片组件
// ==========================================

interface OrderCardProps {
  order: OrderFailure;
  onClick: () => void;
  isSelected: boolean;
}

// M6修复：使用React.memo优化OrderCard，避免不必要的重渲染
const OrderCard: React.FC<OrderCardProps> = React.memo(
  ({ order, onClick, isSelected }) => {
    return (
      <Card
        size="small"
        hoverable
        onClick={onClick}
        style={{
          marginBottom: '12px',
          borderLeft: `4px solid ${URGENCY_LEVEL_COLORS[order.urgencyLevel]}`,
          backgroundColor: isSelected ? '#e6f7ff' : undefined,
          cursor: 'pointer',
        }}
      >
        <Space direction="vertical" style={{ width: '100%' }} size="small">
          {/* 订单号 */}
          <div style={{ fontWeight: 'bold', fontSize: '14px' }}>
            {order.contractNo}
          </div>

          {/* 失败类型 */}
          <Tag color={getFailTypeColor(order.failType)} style={{ margin: 0 }}>
            {getFailTypeLabel(order.failType)}
          </Tag>

          {/* 到期信息 */}
          <div style={{ fontSize: '12px', color: '#8c8c8c' }}>
            <ClockCircleOutlined style={{ marginRight: '4px' }} />
            {order.daysToDue}天后到期 ({order.dueDate})
          </div>

          {/* 完成率 */}
          <div>
            <div style={{ fontSize: '12px', marginBottom: '4px' }}>
              完成率: {order.completionRate.toFixed(0)}%
            </div>
            <Progress
              percent={Math.round(order.completionRate)}
              size="small"
              showInfo={false}
              strokeColor={
                order.completionRate < 50
                  ? '#ff4d4f'
                  : order.completionRate < 80
                  ? '#faad14'
                  : '#52c41a'
              }
            />
          </div>

          {/* 重量信息 */}
          <div style={{ fontSize: '12px' }}>
            未排产: <span style={{ color: '#ff4d4f', fontWeight: 'bold' }}>{order.unscheduledWeightT.toFixed(3)}</span>吨 / 总计: {order.totalWeightT.toFixed(3)}吨
          </div>

          {/* 机组 */}
          <Tag color="blue" style={{ margin: 0 }}>
            {order.machineCode}
          </Tag>
        </Space>
      </Card>
    );
  },
  // M6修复：自定义比较函数，仅在订单关键属性或选中状态变化时重新渲染
  (prev, next) =>
    prev.order.contractNo === next.order.contractNo &&
    prev.order.completionRate === next.order.completionRate &&
    prev.order.failType === next.order.failType &&
    prev.order.urgencyLevel === next.order.urgencyLevel &&
    prev.order.daysToDue === next.order.daysToDue &&
    prev.order.unscheduledWeightT === next.order.unscheduledWeightT &&
    prev.isSelected === next.isSelected
);

// ==========================================
// 默认导出（用于React.lazy）
// ==========================================
export default D2OrderFailure;
