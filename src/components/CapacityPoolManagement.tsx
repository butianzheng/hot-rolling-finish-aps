import React, { useState, useEffect } from 'react';
import {
  Card,
  Table,
  Button,
  Space,
  DatePicker,
  Select,
  Modal,
  InputNumber,
  Input,
  message,
  Row,
  Col,
  Statistic,
} from 'antd';
import { ReloadOutlined, EditOutlined } from '@ant-design/icons';
import type { ColumnsType } from 'antd/es/table';
import dayjs, { Dayjs } from 'dayjs';
import { useNavigate } from 'react-router-dom';
import { capacityApi } from '../api/tauri';
import { useActiveVersionId } from '../stores/use-global-store';
import { formatCapacity, formatPercent, formatDate } from '../utils/formatters';
import { tableEmptyConfig } from './CustomEmpty';
import NoActiveVersionGuide from './NoActiveVersionGuide';

const { RangePicker } = DatePicker;
const { Option } = Select;

interface CapacityPool {
  machine_code: string;
  plan_date: string;
  target_capacity_t: number;
  limit_capacity_t: number;
  used_capacity_t: number;
  available_capacity_t: number;
}

interface CapacityPoolManagementProps {
  onNavigateToPlan?: () => void;
}

const CapacityPoolManagement: React.FC<CapacityPoolManagementProps> = ({ onNavigateToPlan }) => {
  const navigate = useNavigate();
  const [loading, setLoading] = useState(false);
  const [capacityPools, setCapacityPools] = useState<CapacityPool[]>([]);
  const [selectedMachines, setSelectedMachines] = useState<string[]>(['H031', 'H032', 'H033', 'H034']);
  const [dateRange, setDateRange] = useState<[Dayjs, Dayjs]>([
    dayjs(),
    dayjs().add(7, 'day'),
  ]);
  const [editModalVisible, setEditModalVisible] = useState(false);
  const [editingPool, setEditingPool] = useState<CapacityPool | null>(null);
  const [targetCapacity, setTargetCapacity] = useState(0);
  const [limitCapacity, setLimitCapacity] = useState(0);
  const [updateReason, setUpdateReason] = useState('');
  const activeVersionId = useActiveVersionId();
  const navigateToPlan = onNavigateToPlan || (() => navigate('/plan'));

  const loadCapacityPools = async () => {
    if (!dateRange) {
      message.warning('请选择日期范围');
      return;
    }

    setLoading(true);
    try {
      const result = await capacityApi.getCapacityPools(
        selectedMachines,
        formatDate(dateRange[0]),
        formatDate(dateRange[1]),
        activeVersionId || undefined
      );

      // 后端返回的是 CapacityPool（不含 available_capacity_t），这里做一次兼容归一化，避免渲染期崩溃。
      const normalized: CapacityPool[] = (Array.isArray(result) ? result : []).map((row: any) => {
        const target = Number(row?.target_capacity_t ?? 0);
        const limit = Number(row?.limit_capacity_t ?? 0);
        const used = Number(row?.used_capacity_t ?? 0);
        const available = Math.max(limit - used, 0);

        return {
          machine_code: String(row?.machine_code ?? ''),
          plan_date: String(row?.plan_date ?? ''),
          target_capacity_t: Number.isFinite(target) ? target : 0,
          limit_capacity_t: Number.isFinite(limit) ? limit : 0,
          used_capacity_t: Number.isFinite(used) ? used : 0,
          available_capacity_t: available,
        };
      });

      setCapacityPools(normalized);
      message.success(`成功加载 ${normalized.length} 条产能池数据`);
    } catch (error: any) {
      console.error('加载产能池失败:', error);
    } finally {
      setLoading(false);
    }
  };

  const handleEdit = (record: CapacityPool) => {
    setEditingPool(record);
    setTargetCapacity(record.target_capacity_t);
    setLimitCapacity(record.limit_capacity_t);
    setUpdateReason('');
    setEditModalVisible(true);
  };

  const handleUpdate = async () => {
    if (!editingPool) return;
    if (!updateReason.trim()) {
      message.warning('请输入调整原因');
      return;
    }

    setLoading(true);
    try {
      await capacityApi.updateCapacityPool(
        editingPool.machine_code,
        editingPool.plan_date,
        targetCapacity,
        limitCapacity,
        updateReason
      );
      message.success('产能池更新成功');
      setEditModalVisible(false);
      await loadCapacityPools();
    } catch (error: any) {
      console.error('更新产能池失败:', error);
    } finally {
      setLoading(false);
    }
  };

  const columns: ColumnsType<CapacityPool> = [
    {
      title: '机组',
      dataIndex: 'machine_code',
      key: 'machine_code',
      width: 100,
      fixed: 'left',
    },
    {
      title: '日期',
      dataIndex: 'plan_date',
      key: 'plan_date',
      width: 120,
    },
    {
      title: '目标产能(吨)',
      dataIndex: 'target_capacity_t',
      key: 'target_capacity_t',
      width: 120,
      render: (value: number) => formatCapacity(value),
    },
    {
      title: '极限产能(吨)',
      dataIndex: 'limit_capacity_t',
      key: 'limit_capacity_t',
      width: 120,
      render: (value: number) => formatCapacity(value),
    },
    {
      title: '已用产能(吨)',
      dataIndex: 'used_capacity_t',
      key: 'used_capacity_t',
      width: 120,
      render: (value: number) => formatCapacity(value),
    },
    {
      title: '剩余产能(吨)',
      dataIndex: 'available_capacity_t',
      key: 'available_capacity_t',
      width: 120,
      render: (value: number) => (
        <span style={{ color: value < 100 ? '#cf1322' : '#52c41a' }}>
          {formatCapacity(value)}
        </span>
      ),
    },
    {
      title: '利用率',
      key: 'utilization',
      width: 100,
      render: (_, record) => {
        const target = record.target_capacity_t || 0;
        const used = record.used_capacity_t || 0;
        const rate = target > 0 ? (used / target) * 100 : 0;
        return (
          <span style={{ color: rate > 100 ? '#cf1322' : rate > 90 ? '#fa8c16' : '#52c41a' }}>
            {formatPercent(rate)}
          </span>
        );
      },
    },
    {
      title: '操作',
      key: 'action',
      width: 100,
      fixed: 'right',
      render: (_, record) => (
        <Button
          type="link"
          size="small"
          icon={<EditOutlined />}
          onClick={() => handleEdit(record)}
        >
          调整
        </Button>
      ),
    },
  ];

  useEffect(() => {
    if (activeVersionId) {
      loadCapacityPools();
    }
  }, [activeVersionId]);

  const totalStats = capacityPools.reduce(
    (acc, pool) => ({
      totalTarget: acc.totalTarget + pool.target_capacity_t,
      totalUsed: acc.totalUsed + pool.used_capacity_t,
      totalAvailable: acc.totalAvailable + pool.available_capacity_t,
    }),
    { totalTarget: 0, totalUsed: 0, totalAvailable: 0 }
  );

  // 没有激活版本时显示引导
  if (!activeVersionId) {
    return (
      <NoActiveVersionGuide
        title="尚无激活的排产版本"
        description="产能池管理需要一个激活的排产版本作为基础"
        onNavigateToPlan={navigateToPlan}
      />
    );
  }

  return (
    <div style={{ padding: '24px' }}>
      <Row justify="space-between" align="middle" style={{ marginBottom: 16 }}>
        <Col>
          <h2 style={{ margin: 0 }}>产能池管理</h2>
        </Col>
        <Col>
          <Button icon={<ReloadOutlined />} onClick={loadCapacityPools}>
            刷新
          </Button>
        </Col>
      </Row>

      {/* 统计卡片 */}
      <Row gutter={16} style={{ marginBottom: 16 }}>
        <Col span={8}>
          <Card>
            <Statistic
              title="总目标产能"
              value={formatCapacity(totalStats.totalTarget)}
              suffix="吨"
            />
          </Card>
        </Col>
        <Col span={8}>
          <Card>
            <Statistic
              title="总已用产能"
              value={formatCapacity(totalStats.totalUsed)}
              suffix="吨"
              valueStyle={{ color: '#1890ff' }}
            />
          </Card>
        </Col>
        <Col span={8}>
          <Card>
            <Statistic
              title="总剩余产能"
              value={formatCapacity(totalStats.totalAvailable)}
              suffix="吨"
              valueStyle={{
                color: totalStats.totalAvailable < 500 ? '#cf1322' : '#52c41a',
              }}
            />
          </Card>
        </Col>
      </Row>

      {/* 筛选栏 */}
      <Card style={{ marginBottom: 16 }}>
        <Space wrap>
          <Select
            mode="multiple"
            style={{ width: 300 }}
            placeholder="选择机组"
            value={selectedMachines}
            onChange={setSelectedMachines}
          >
            <Option value="H031">H031</Option>
            <Option value="H032">H032</Option>
            <Option value="H033">H033</Option>
            <Option value="H034">H034</Option>
          </Select>

          <RangePicker
            value={dateRange}
            onChange={(dates) => dates && setDateRange(dates as [Dayjs, Dayjs])}
            format="YYYY-MM-DD"
          />

          <Button type="primary" onClick={loadCapacityPools} loading={loading}>
            查询
          </Button>
        </Space>
      </Card>

      {/* 产能池表格 */}
      <Card>
        <Table
          columns={columns}
          dataSource={capacityPools}
          loading={loading}
          rowKey={(record) => `${record.machine_code}-${record.plan_date}`}
          locale={tableEmptyConfig}
          pagination={{
            pageSize: 20,
            showSizeChanger: true,
            showTotal: (total) => `共 ${total} 条记录`,
          }}
          scroll={{ x: 1000 }}
          size="small"
        />
      </Card>

      {/* 编辑模态框 */}
      <Modal
        title="调整产能池"
        open={editModalVisible}
        onOk={handleUpdate}
        onCancel={() => setEditModalVisible(false)}
        confirmLoading={loading}
      >
        {editingPool && (
          <Space direction="vertical" style={{ width: '100%' }}>
            <div>
              <strong>机组:</strong> {editingPool.machine_code}
            </div>
            <div>
              <strong>日期:</strong> {editingPool.plan_date}
            </div>
            <div>
              <label>目标产能(吨):</label>
              <InputNumber
                style={{ width: '100%', marginTop: 8 }}
                min={0}
                max={10000}
                value={targetCapacity}
                onChange={(val) => setTargetCapacity(val || 0)}
              />
            </div>
            <div>
              <label>极限产能(吨):</label>
              <InputNumber
                style={{ width: '100%', marginTop: 8 }}
                min={0}
                max={10000}
                value={limitCapacity}
                onChange={(val) => setLimitCapacity(val || 0)}
              />
            </div>
            <div>
              <label>调整原因(必填):</label>
              <Input.TextArea
                style={{ marginTop: 8 }}
                placeholder="请输入调整原因"
                value={updateReason}
                onChange={(e) => setUpdateReason(e.target.value)}
                rows={3}
              />
            </div>
          </Space>
        )}
      </Modal>
    </div>
  );
};

export default CapacityPoolManagement;
