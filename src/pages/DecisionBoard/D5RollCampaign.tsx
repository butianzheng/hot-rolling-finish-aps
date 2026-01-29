// ==========================================
// D5决策：轧制活动警报页面
// ==========================================
// 职责: 展示各机组换辊状态，监控换辊警报，提供换辊建议
// ==========================================

import React, { useMemo } from 'react';
import { Button, Card, Row, Col, Statistic, Tag, Spin, Alert, Progress, Descriptions, Space, Table } from 'antd';
import type { ColumnsType } from 'antd/es/table';
import {
  WarningOutlined,
  CheckCircleOutlined,
  ExclamationCircleOutlined,
  StopOutlined,
  ToolOutlined,
} from '@ant-design/icons';
import { useAllRollCampaignAlerts } from '../../hooks/queries/use-decision-queries';
import type { DrilldownSpec } from '../../hooks/useRiskOverviewData';
import { useActiveVersionId } from '../../stores/use-global-store';
import type { RollCampaignAlert } from '../../types/decision';
import {
  ROLL_STATUS_COLORS,
  ROLL_STATUS_LABELS,
  calculateUtilization,
  parseAlertLevel,
  type RollStatus,
} from '../../types/decision/d5-roll-campaign';

// ==========================================
// 主组件
// ==========================================

interface D5RollCampaignProps {
  embedded?: boolean;
  onOpenDrilldown?: (spec: DrilldownSpec) => void;
}

export const D5RollCampaign: React.FC<D5RollCampaignProps> = ({ embedded, onOpenDrilldown }) => {
  const versionId = useActiveVersionId();
  const openWithDrawer = !!embedded && !!onOpenDrilldown;

  // 获取换辊警报数据
  const { data, isLoading, error } = useAllRollCampaignAlerts(versionId);

  // 计算统计数据
  const stats = useMemo(() => {
    if (!data?.items || data.items.length === 0) {
      return {
        totalMachines: 0,
        normalCount: 0,
        suggestCount: 0,
        warningCount: 0,
        hardStopCount: 0,
        avgUtilization: 0,
      };
    }

    const totalMachines = data.items.length;
    const normalCount = data.items.filter((item) => parseAlertLevel(item.alertLevel) === 'NORMAL').length;
    const suggestCount = data.items.filter((item) => parseAlertLevel(item.alertLevel) === 'SUGGEST').length;
    const warningCount = data.items.filter((item) => parseAlertLevel(item.alertLevel) === 'WARNING').length;
    const hardStopCount = data.summary?.nearHardStopCount ?? 0;

    // 计算平均利用率（基于软限制）
    const totalUtilization = data.items.reduce(
      (sum, item) => sum + calculateUtilization(item.currentTonnageT, item.softLimitT),
      0
    );
    const avgUtilization = totalUtilization / totalMachines;

    return {
      totalMachines,
      normalCount,
      suggestCount,
      warningCount,
      hardStopCount,
      avgUtilization,
    };
  }, [data]);

  // 按状态分组
  const groupedByStatus = useMemo(() => {
    if (!data?.items) return { HARD_STOP: [], WARNING: [], SUGGEST: [], NORMAL: [] };

    const groups: Record<RollStatus, RollCampaignAlert[]> = {
      HARD_STOP: [],
      WARNING: [],
      SUGGEST: [],
      NORMAL: [],
    };

    data.items.forEach((item) => {
      const status = parseAlertLevel(item.alertLevel);
      groups[status].push(item);
    });

    return groups;
  }, [data]);

  // 表格列定义
  const columns: ColumnsType<RollCampaignAlert> = [
    {
      title: '机组',
      dataIndex: 'machineCode',
      key: 'machineCode',
      width: 100,
      fixed: 'left',
      render: (code: string) => <Tag color="blue">{code}</Tag>,
    },
    {
      title: '警报等级',
      dataIndex: 'alertLevel',
      key: 'alertLevel',
      width: 120,
      filters: [
        { text: '正常', value: 'NORMAL' },
        { text: '建议换辊', value: 'SUGGEST' },
        { text: '警告', value: 'WARNING' },
        { text: '硬停止', value: 'HARD_STOP' },
      ],
      onFilter: (value, record) => parseAlertLevel(record.alertLevel) === value,
      render: (alertLevel: string) => {
        const status = parseAlertLevel(alertLevel);
        return (
          <Tag
            color={ROLL_STATUS_COLORS[status]}
            icon={
              status === 'HARD_STOP' ? (
                <StopOutlined />
              ) : status === 'WARNING' ? (
                <WarningOutlined />
              ) : status === 'SUGGEST' ? (
                <ToolOutlined />
              ) : (
                <CheckCircleOutlined />
              )
            }
          >
            {ROLL_STATUS_LABELS[status]}
          </Tag>
        );
      },
    },
    {
      title: '当前累积吨位',
      dataIndex: 'currentTonnageT',
      key: 'currentTonnageT',
      width: 140,
      render: (weight: number) => `${weight.toFixed(1)}吨`,
      sorter: (a, b) => a.currentTonnageT - b.currentTonnageT,
    },
    {
      title: '软限制',
      dataIndex: 'softLimitT',
      key: 'softLimitT',
      width: 120,
      render: (threshold: number) => `${threshold.toFixed(0)}吨`,
    },
    {
      title: '硬限制',
      dataIndex: 'hardLimitT',
      key: 'hardLimitT',
      width: 120,
      render: (limit: number) => `${limit.toFixed(0)}吨`,
    },
    {
      title: '利用率',
      key: 'utilization',
      width: 180,
      render: (_, record) => {
        const utilization = calculateUtilization(
          record.currentTonnageT,
          record.softLimitT
        );
        return (
          <div style={{ width: '100%' }}>
            <Progress
              percent={utilization}
              size="small"
              strokeColor={
                utilization >= 100
                  ? '#ff4d4f'
                  : utilization >= 80
                  ? '#faad14'
                  : utilization >= 60
                  ? '#1677ff'
                  : '#52c41a'
              }
              status={utilization >= 100 ? 'exception' : utilization >= 80 ? 'normal' : 'success'}
            />
          </div>
        );
      },
      sorter: (a, b) =>
        calculateUtilization(a.currentTonnageT, a.softLimitT) -
        calculateUtilization(b.currentTonnageT, b.softLimitT),
    },
    {
      title: '剩余吨位',
      dataIndex: 'remainingTonnageT',
      key: 'remainingTonnageT',
      width: 120,
      render: (remaining: number) => (
        <span style={{ color: remaining <= 0 ? '#ff4d4f' : remaining < 500 ? '#faad14' : '#52c41a' }}>
          {remaining.toFixed(1)}吨
        </span>
      ),
      sorter: (a, b) => a.remainingTonnageT - b.remainingTonnageT,
    },
    {
      title: '预计硬停止日期',
      dataIndex: 'estimatedHardStopDate',
      key: 'estimatedHardStopDate',
      width: 140,
      render: (date: string | null) => date || '-',
    },
    {
      title: '警报消息',
      dataIndex: 'alertMessage',
      key: 'alertMessage',
      ellipsis: true,
      render: (message: string) => message || '-',
    },
  ];

  // ==========================================
  // 加载状态
  // ==========================================

  if (isLoading) {
    return (
      <div style={{ textAlign: 'center', padding: embedded ? '40px 0' : '100px 0' }}>
        <Spin size="large" tip="正在加载换辊警报数据...">
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
            <ToolOutlined style={{ marginRight: 8 }} />
            D5决策：轧制活动警报
          </h2>
          <p style={{ color: '#8c8c8c', marginBottom: 16 }}>
            监控各机组轧辊累积吨位，及时发现换辊需求，避免生产中断
          </p>
        </div>
      ) : null}

      {/* 警报总览 */}
      {stats.hardStopCount > 0 && (
        <Alert
          message={`紧急警告：${stats.hardStopCount} 台机组已达到强制换辊上限，必须立即处理！`}
          type="error"
          showIcon
          icon={<StopOutlined />}
          style={{ marginBottom: '16px' }}
        />
      )}

      {/* 统计卡片 */}
      <Row gutter={embedded ? 12 : 16} style={{ marginBottom: embedded ? 12 : 24 }}>
        <Col span={4}>
          <Card size={embedded ? 'small' : undefined}>
            <Statistic
              title="机组总数"
              value={stats.totalMachines}
              prefix={<ToolOutlined />}
            />
          </Card>
        </Col>
        <Col span={4}>
          <Card size={embedded ? 'small' : undefined}>
            <Statistic
              title="正常"
              value={stats.normalCount}
              prefix={<CheckCircleOutlined />}
              valueStyle={{ color: '#52c41a' }}
            />
          </Card>
        </Col>
        <Col span={4}>
          <Card size={embedded ? 'small' : undefined}>
            <Statistic
              title="建议换辊"
              value={stats.suggestCount}
              prefix={<ToolOutlined />}
              valueStyle={{ color: stats.suggestCount > 0 ? '#1677ff' : '#52c41a' }}
            />
          </Card>
        </Col>
        <Col span={4}>
          <Card size={embedded ? 'small' : undefined}>
            <Statistic
              title="警告"
              value={stats.warningCount}
              prefix={<WarningOutlined />}
              valueStyle={{ color: stats.warningCount > 0 ? '#faad14' : '#52c41a' }}
            />
          </Card>
        </Col>
        <Col span={4}>
          <Card size={embedded ? 'small' : undefined}>
            <Statistic
              title="硬停止"
              value={stats.hardStopCount}
              prefix={<StopOutlined />}
              valueStyle={{ color: stats.hardStopCount > 0 ? '#ff4d4f' : '#52c41a' }}
            />
          </Card>
        </Col>
        <Col span={4}>
          <Card size={embedded ? 'small' : undefined}>
            <Statistic
              title="平均利用率"
              value={stats.avgUtilization}
              precision={1}
              suffix="%"
              valueStyle={{
                color:
                  stats.avgUtilization >= 90
                    ? '#ff4d4f'
                    : stats.avgUtilization >= 70
                    ? '#faad14'
                    : '#52c41a',
              }}
            />
          </Card>
        </Col>
      </Row>

      {/* 严重警报卡片 */}
      {(groupedByStatus.HARD_STOP.length > 0 || groupedByStatus.WARNING.length > 0) && (
        <Card
          title={
            <Space>
              <ExclamationCircleOutlined style={{ color: '#ff4d4f' }} />
              <span>严重警报</span>
            </Space>
          }
          style={{ marginBottom: '24px' }}
        >
          <Row gutter={16}>
            {[...groupedByStatus.HARD_STOP, ...groupedByStatus.WARNING].map((alert) => (
              <Col key={alert.machineCode} span={8} style={{ marginBottom: '16px' }}>
                <SevereAlertCard alert={alert} />
              </Col>
            ))}
          </Row>
        </Card>
      )}

      {/* 完整表格 */}
      <Card
        title="机组换辊状态"
        style={{ marginBottom: '24px' }}
        extra={
          openWithDrawer ? (
            <Button size="small" onClick={() => onOpenDrilldown({ kind: 'roll' })}>
              打开下钻
            </Button>
          ) : undefined
        }
      >
        <Table<RollCampaignAlert>
          columns={columns}
          dataSource={data?.items || []}
          rowKey="machineCode"
          pagination={{
            pageSize: 20,
            showSizeChanger: true,
            showQuickJumper: true,
          }}
          scroll={{ x: 1400 }}
          onRow={
            openWithDrawer
              ? (record) => ({
                  onClick: () => onOpenDrilldown?.({ kind: 'roll', machineCode: record.machineCode }),
                  style: { cursor: 'pointer' },
                })
              : undefined
          }
        />
      </Card>
    </div>
  );
};

// ==========================================
// 严重警报卡片组件
// ==========================================

interface SevereAlertCardProps {
  alert: RollCampaignAlert;
}

const SevereAlertCard: React.FC<SevereAlertCardProps> = ({ alert }) => {
  const utilization = calculateUtilization(alert.currentTonnageT, alert.softLimitT);
  const status = parseAlertLevel(alert.alertLevel);

  return (
    <Card
      size="small"
      style={{
        borderLeft: `4px solid ${ROLL_STATUS_COLORS[status]}`,
      }}
    >
      <Space direction="vertical" style={{ width: '100%' }} size="small">
        {/* 机组和状态 */}
        <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
          <Tag color="blue" style={{ fontSize: '14px', fontWeight: 'bold' }}>
            {alert.machineCode}
          </Tag>
          <Tag
            color={ROLL_STATUS_COLORS[status]}
            icon={status === 'HARD_STOP' ? <StopOutlined /> : <WarningOutlined />}
          >
            {ROLL_STATUS_LABELS[status]}
          </Tag>
        </div>

        {/* 吨位信息 */}
        <Descriptions size="small" column={1} bordered>
          <Descriptions.Item label="当前累积">
            {alert.currentTonnageT.toFixed(1)}吨
          </Descriptions.Item>
          <Descriptions.Item label="软限制">
            {alert.softLimitT.toFixed(0)}吨
          </Descriptions.Item>
          <Descriptions.Item label="硬限制">
            {alert.hardLimitT.toFixed(0)}吨
          </Descriptions.Item>
        </Descriptions>

        {/* 利用率进度条 */}
        <div>
          <div style={{ fontSize: '12px', marginBottom: '4px' }}>利用率: {utilization}%</div>
          <Progress
            percent={utilization}
            size="small"
            strokeColor={utilization >= 100 ? '#ff4d4f' : '#faad14'}
            status={utilization >= 100 ? 'exception' : 'normal'}
          />
        </div>

        {/* 警报消息 */}
        {alert.alertMessage && (
          <div style={{ fontSize: '12px', color: '#8c8c8c' }}>
            <ExclamationCircleOutlined style={{ marginRight: '4px' }} />
            {alert.alertMessage}
          </div>
        )}

        {/* 预计硬停止日期 */}
        {alert.estimatedHardStopDate && (
          <div style={{ fontSize: '12px', color: '#1677ff' }}>
            预计硬停止: {alert.estimatedHardStopDate}
          </div>
        )}
      </Space>
    </Card>
  );
};

// ==========================================
// 默认导出（用于React.lazy）
// ==========================================
export default D5RollCampaign;
