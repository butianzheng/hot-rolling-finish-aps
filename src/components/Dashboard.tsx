import React, { useState, useCallback } from 'react';
import { Card, Row, Col, Statistic, Table, message, Button, Space, Switch, Select, Tag, Tooltip } from 'antd';
import type { ColumnsType } from 'antd/es/table';
import {
  WarningOutlined,
  ClockCircleOutlined,
  DatabaseOutlined,
  ThunderboltOutlined,
  ReloadOutlined,
  CloseCircleOutlined,
} from '@ant-design/icons';
import { dashboardApi } from '../api/tauri';
import { formatNumber, formatPercent } from '../utils/formatters';
import { useAutoRefresh } from '../hooks/useAutoRefresh';
import { useActiveVersionId } from '../stores/use-global-store';
import NoActiveVersionGuide from './NoActiveVersionGuide';
import { useNavigate } from 'react-router-dom';

// D2: OrderFailureDto (snake_case)
interface OrderFailureRow {
  contract_no: string;
  due_date: string;
  urgency_level: string;
  fail_type: string;
  completion_rate: number; // percent (0-100)
  total_weight_t: number;
  unscheduled_weight_t: number;
  machine_code: string;
}

interface OrderFailureSetResponse {
  items: OrderFailureRow[];
  summary?: {
    total_failures: number;
    total_unscheduled_weight_t: number;
  };
}

// D3: ColdStockBucketDto (snake_case)
interface ColdStockBucketRow {
  machine_code: string;
  age_bin: string;
  pressure_level: string;
  count: number;
  weight_t: number;
  avg_age_days: number;
  max_age_days: number;
}

interface ColdStockProfileResponse {
  items: ColdStockBucketRow[];
  summary?: {
    total_cold_stock_count: number;
    total_cold_stock_weight_t: number;
  };
}

// D4: BottleneckPointDto (snake_case)
interface BottleneckPointRow {
  machine_code: string;
  plan_date: string;
  bottleneck_score: number;
  bottleneck_level: string;
}

interface MachineBottleneckProfileResponse {
  items: BottleneckPointRow[];
}

const Dashboard: React.FC = () => {
  const navigate = useNavigate();
  const activeVersionId = useActiveVersionId();
  const [loading, setLoading] = useState(false);
  const [orderFailures, setOrderFailures] = useState<OrderFailureRow[]>([]);
  const [orderFailureSummary, setOrderFailureSummary] = useState<OrderFailureSetResponse['summary']>(undefined);
  const [coldStockBuckets, setColdStockBuckets] = useState<ColdStockBucketRow[]>([]);
  const [coldStockSummary, setColdStockSummary] = useState<ColdStockProfileResponse['summary']>(undefined);
  const [mostCongestedPoint, setMostCongestedPoint] = useState<BottleneckPointRow | null>(null);

  // 自动刷新配置
  const [autoRefreshEnabled, setAutoRefreshEnabled] = useState(true);
  const [refreshInterval, setRefreshInterval] = useState(30000); // 默认 30 秒

  const unsatisfiedColumns: ColumnsType<OrderFailureRow> = [
    {
      title: '合同号',
      dataIndex: 'contract_no',
      key: 'contract_no',
    },
    {
      title: '紧急等级',
      dataIndex: 'urgency_level',
      key: 'urgency_level',
    },
    {
      title: '交期',
      dataIndex: 'due_date',
      key: 'due_date',
    },
    {
      title: '失败类型',
      dataIndex: 'fail_type',
      key: 'fail_type',
    },
    {
      title: '完成率',
      dataIndex: 'completion_rate',
      key: 'completion_rate',
      render: (val: number) => {
        const n = typeof val === 'number' ? val : 0;
        const pct = n <= 1 ? n * 100 : n; // 兼容 0-1 与 0-100 两种口径
        return formatPercent(pct || 0);
      },
    },
  ];

  const coldStockColumns: ColumnsType<ColdStockBucketRow> = [
    {
      title: '机组',
      dataIndex: 'machine_code',
      key: 'machine_code',
    },
    {
      title: '库龄分桶',
      dataIndex: 'age_bin',
      key: 'age_bin',
    },
    {
      title: '压力等级',
      dataIndex: 'pressure_level',
      key: 'pressure_level',
    },
    {
      title: '数量',
      dataIndex: 'count',
      key: 'count',
      width: 80,
    },
    {
      title: '重量(吨)',
      dataIndex: 'weight_t',
      key: 'weight_t',
      render: (val: number) => formatNumber(val, 2),
    },
  ];

  // 加载Dashboard数据（使用useCallback确保稳定的函数引用）
  const loadDashboardData = useCallback(async () => {
    if (!activeVersionId) {
      return; // 没有活动版本时不加载数据
    }
    setLoading(true);
    try {
      // 加载未满足的紧急单
      const orderFailureSet = (await dashboardApi.getUnsatisfiedUrgentMaterials(
        activeVersionId
      )) as OrderFailureSetResponse;
      setOrderFailures(orderFailureSet?.items || []);
      setOrderFailureSummary(orderFailureSet?.summary);

      // 加载冷料（库存超过30天）
      const coldStockProfile = (await dashboardApi.getColdStockMaterials(
        activeVersionId,
        30
      )) as ColdStockProfileResponse;
      setColdStockBuckets(coldStockProfile?.items || []);
      setColdStockSummary(coldStockProfile?.summary);

      // 加载最拥堵机组
      const bottleneckProfile = (await dashboardApi.getMostCongestedMachine(
        activeVersionId
      )) as MachineBottleneckProfileResponse;
      const points = bottleneckProfile?.items || [];
      const most = points.reduce<BottleneckPointRow | null>((max, p) => {
        if (!max) return p;
        return p.bottleneck_score > max.bottleneck_score ? p : max;
      }, null);
      setMostCongestedPoint(most);
    } catch (error: any) {
      message.error(`加载失败: ${error.message || error}`);
    } finally {
      setLoading(false);
    }
  }, [activeVersionId]);

  // 使用自动刷新 Hook
  const { lastRefreshTime, nextRefreshCountdown, refresh: manualRefresh } = useAutoRefresh(
    loadDashboardData,
    refreshInterval,
    autoRefreshEnabled
  );

  // 格式化时间为 HH:mm:ss
  const formatTime = (date: Date | null) => {
    if (!date) return '从未刷新';
    return date.toLocaleTimeString('zh-CN', { hour12: false });
  };

  if (!activeVersionId) {
    return (
      <NoActiveVersionGuide
        title="尚无激活的排产版本"
        description="驾驶舱需要一个激活的排产版本作为基础"
        onNavigateToPlan={() => navigate('/plan')}
      />
    );
  }

  return (
    <div>
      {/* 刷新控制栏 */}
      <Card style={{ marginBottom: 16 }}>
        <Space wrap align="center" size="large">
          {/* 手动刷新按钮 */}
          <Button
            type="primary"
            icon={<ReloadOutlined />}
            onClick={manualRefresh}
            loading={loading}
          >
            手动刷新
          </Button>

          {/* 自动刷新开关 */}
          <Space>
            <span>自动刷新:</span>
            <Switch
              checked={autoRefreshEnabled}
              onChange={setAutoRefreshEnabled}
              size="small"
            />
          </Space>

          {/* 刷新间隔选择 */}
          {autoRefreshEnabled && (
            <Space>
              <span>间隔:</span>
              <Select
                value={refreshInterval}
                onChange={setRefreshInterval}
                style={{ width: 120 }}
                size="small"
              >
                <Select.Option value={10000}>10 秒</Select.Option>
                <Select.Option value={15000}>15 秒</Select.Option>
                <Select.Option value={30000}>30 秒</Select.Option>
                <Select.Option value={60000}>1 分钟</Select.Option>
                <Select.Option value={300000}>5 分钟</Select.Option>
              </Select>
            </Space>
          )}

          {/* 刷新状态显示 */}
          <Space>
            {autoRefreshEnabled ? (
              <>
                <Tag color="green" icon={<ReloadOutlined />}>
                  自动刷新中
                </Tag>
                {nextRefreshCountdown > 0 && (
                  <Tooltip title="下次自动刷新倒计时">
                    <span style={{ color: '#666' }}>
                      {nextRefreshCountdown}s 后刷新
                    </span>
                  </Tooltip>
                )}
              </>
            ) : (
              <Tag color="red" icon={<CloseCircleOutlined />}>
                自动刷新关闭
              </Tag>
            )}
          </Space>

          {/* 最后刷新时间 */}
          <Space>
            <span style={{ color: '#666' }}>
              最后刷新: <strong>{formatTime(lastRefreshTime)}</strong>
            </span>
          </Space>
        </Space>
      </Card>

      <Row gutter={16} style={{ marginBottom: 24 }}>
        <Col span={6}>
          <Card
            hoverable
            style={{ cursor: 'pointer' }}
            onClick={() => navigate('/decision/d2-order-failure')}
          >
            <Statistic
              title="未满足紧急单"
              value={orderFailureSummary?.total_failures ?? orderFailures.length}
              prefix={<WarningOutlined />}
              valueStyle={{ color: '#cf1322' }}
            />
          </Card>
        </Col>
        <Col span={6}>
          <Card
            hoverable
            style={{ cursor: 'pointer' }}
            onClick={() => navigate('/decision/d3-cold-stock')}
          >
            <Statistic
              title="冷料数量"
              value={coldStockSummary?.total_cold_stock_count ?? coldStockBuckets.reduce((sum, b) => sum + (b.count || 0), 0)}
              prefix={<ClockCircleOutlined />}
              valueStyle={{ color: '#faad14' }}
            />
          </Card>
        </Col>
        <Col span={6}>
          <Card hoverable>
            <Statistic
              title="冷料总重(吨)"
              value={formatNumber(
                coldStockSummary?.total_cold_stock_weight_t ??
                  coldStockBuckets.reduce((sum, b) => sum + (b.weight_t || 0), 0),
                2
              )}
              prefix={<DatabaseOutlined />}
            />
          </Card>
        </Col>
        <Col span={6}>
          <Card
            hoverable
            style={{ cursor: mostCongestedPoint ? 'pointer' : 'default' }}
            onClick={() => {
              if (!mostCongestedPoint) return;
              const qs = new URLSearchParams({
                machine: mostCongestedPoint.machine_code,
                date: mostCongestedPoint.plan_date,
              }).toString();
              navigate(`/decision/d4-bottleneck?${qs}`);
            }}
          >
            <Statistic
              title="最拥堵机组"
              value={mostCongestedPoint?.machine_code || '-'}
              prefix={<ThunderboltOutlined />}
              valueStyle={{ color: '#1890ff' }}
            />
          </Card>
        </Col>
      </Row>

      <Row gutter={16}>
        <Col span={12}>
          <Card title="订单失败集合 (D2)" variant="borderless">
            <Table
              columns={unsatisfiedColumns}
              dataSource={orderFailures}
              rowKey={(r) => `${r.contract_no}-${r.due_date}-${r.machine_code}`}
              loading={loading}
              pagination={{ pageSize: 5 }}
              size="small"
              onRow={(record) => ({
                onClick: () => {
                  const qs = new URLSearchParams({
                    contractNo: record.contract_no,
                    urgency: record.urgency_level,
                    failType: record.fail_type,
                  }).toString();
                  navigate(`/decision/d2-order-failure?${qs}`);
                },
              })}
            />
          </Card>
        </Col>
        <Col span={12}>
          <Card title="冷料压库分桶 (D3)" variant="borderless">
            <Table
              columns={coldStockColumns}
              dataSource={coldStockBuckets}
              rowKey={(r) => `${r.machine_code}-${r.age_bin}-${r.pressure_level}`}
              loading={loading}
              pagination={{ pageSize: 5 }}
              size="small"
              onRow={(record) => ({
                onClick: () => {
                  const qs = new URLSearchParams({
                    machine: record.machine_code,
                    ageBin: record.age_bin,
                    pressureLevel: record.pressure_level,
                  }).toString();
                  navigate(`/decision/d3-cold-stock?${qs}`);
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
