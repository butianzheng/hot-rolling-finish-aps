/**
 * 机组产能配置面板（增强版）
 * 职责：
 * - 独立的机组筛选（支持多选）
 * - 日期范围选择（单日/月/季/年/自定义）
 * - 批量应用配置到产能池
 */

import React, { useState, useMemo } from 'react';
import {
  Form,
  Input,
  InputNumber,
  Button,
  Space,
  Select,
  DatePicker,
  Card,
  Alert,
  Statistic,
  Row,
  Col,
  Tag,
  Modal,
  Table,
  message,
  Collapse,
  Typography,
} from 'antd';
import {
  CheckCircleOutlined,
  HistoryOutlined,
  ThunderboltOutlined,
  CalendarOutlined,
  SettingOutlined,
} from '@ant-design/icons';
import dayjs, { Dayjs } from 'dayjs';
import { useMachineConfig } from '../../hooks/useMachineConfig';

const { RangePicker } = DatePicker;
const { Text } = Typography;

// 可用机组列表
const AVAILABLE_MACHINES = ['H031', 'H032', 'H033', 'H034'];

// 日期范围快捷选项
type DateRangePreset = {
  label: string;
  getValue: () => [Dayjs, Dayjs];
};

const DATE_RANGE_PRESETS: DateRangePreset[] = [
  {
    label: '今天',
    getValue: () => [dayjs(), dayjs()],
  },
  {
    label: '本月',
    getValue: () => [dayjs().startOf('month'), dayjs().endOf('month')],
  },
  {
    label: '本季度',
    getValue: () => [dayjs().startOf('quarter'), dayjs().endOf('quarter')],
  },
  {
    label: '本年度',
    getValue: () => [dayjs().startOf('year'), dayjs().endOf('year')],
  },
  {
    label: '未来30天',
    getValue: () => [dayjs(), dayjs().add(30, 'day')],
  },
  {
    label: '未来90天',
    getValue: () => [dayjs(), dayjs().add(90, 'day')],
  },
];

export interface MachineConfigPanelProps {
  versionId: string;
  onConfigApplied?: () => void; // 批量应用成功回调
}

export const MachineConfigPanel: React.FC<MachineConfigPanelProps> = ({
  versionId,
  onConfigApplied,
}) => {
  const [form] = Form.useForm();

  // ========== 状态 ==========
  const [selectedMachines, setSelectedMachines] = useState<string[]>([AVAILABLE_MACHINES[0]]);
  const [dateRange, setDateRange] = useState<[Dayjs, Dayjs]>([dayjs(), dayjs().add(29, 'day')]);
  const [historyModalOpen, setHistoryModalOpen] = useState(false);
  const [historyMachine, setHistoryMachine] = useState<string>('');

  const {
    configs,
    configsLoading,
    updateConfig,
    updateConfigLoading,
    applyConfigToDates,
    applyConfigLoading,
    getConfigHistory,
    configHistory,
    clearConfigHistory,
  } = useMachineConfig(versionId);

  // ========== 当前选中机组的配置 ==========
  const selectedConfigs = useMemo(() => {
    return selectedMachines.map((machineCode) => {
      const config = configs.find((c) => c.machine_code === machineCode);
      return {
        machineCode,
        config,
      };
    });
  }, [selectedMachines, configs]);

  // ========== 计算日期范围统计 ==========
  const dateRangeStats = useMemo(() => {
    const [from, to] = dateRange;
    const days = to.diff(from, 'day') + 1;
    return {
      days,
      machines: selectedMachines.length,
      totalRecords: days * selectedMachines.length,
    };
  }, [dateRange, selectedMachines]);

  // ========== 事件处理 ==========
  const handleQuickSelectPreset = (preset: DateRangePreset) => {
    setDateRange(preset.getValue());
  };

  const handleBatchApply = async () => {
    try {
      const values = await form.validateFields();
      const [from, to] = dateRange;

      // 确认对话框
      Modal.confirm({
        title: '确认批量应用配置',
        content: (
          <div>
            <p>
              <strong>将应用以下配置：</strong>
            </p>
            <ul>
              <li>目标产能: {values.default_daily_target_t.toFixed(3)} 吨/天</li>
              <li>极限产能: {values.default_daily_limit_pct.toFixed(1)}%</li>
            </ul>
            <p>
              <strong>应用范围：</strong>
            </p>
            <ul>
              <li>机组: {selectedMachines.join(', ')}</li>
              <li>
                日期范围: {from.format('YYYY-MM-DD')} ~ {to.format('YYYY-MM-DD')}
              </li>
              <li>
                <Tag color="blue">共 {dateRangeStats.totalRecords} 条记录</Tag>
              </li>
            </ul>
            <Alert
              type="warning"
              message="注意：已有产能数据的记录将被跳过，仅更新空记录"
              style={{ marginTop: 12 }}
            />
          </div>
        ),
        okText: '确认应用',
        cancelText: '取消',
        onOk: async () => {
          // 为每个机组应用配置
          const promises = selectedMachines.map((machineCode) =>
            applyConfigToDates({
              version_id: versionId,
              machine_code: machineCode,
              date_from: from.format('YYYY-MM-DD'),
              date_to: to.format('YYYY-MM-DD'),
              default_daily_target_t: values.default_daily_target_t,
              default_daily_limit_pct: values.default_daily_limit_pct / 100, // 转换为小数
              reason: values.reason,
              operator: 'system', // TODO: 从用户上下文获取
            })
          );

          const results = await Promise.all(promises);

          // 汇总结果
          const totalUpdated = results.reduce((sum, r) => sum + r.updated_count, 0);
          const totalSkipped = results.reduce((sum, r) => sum + r.skipped_count, 0);

          message.success(
            `批量应用成功！更新 ${totalUpdated} 条记录，跳过 ${totalSkipped} 条已有数据的记录`
          );

          form.resetFields(['reason']); // 清空原因字段
          onConfigApplied?.();
        },
      });
    } catch (e: any) {
      console.error('[MachineConfigPanel] batch apply failed:', e);
    }
  };

  const handleUpdateMachineConfig = async () => {
    try {
      const values = await form.validateFields();

      // 为每个选中的机组更新配置
      const promises = selectedMachines.map((machineCode) =>
        updateConfig({
          version_id: versionId,
          machine_code: machineCode,
          default_daily_target_t: values.default_daily_target_t,
          default_daily_limit_pct: values.default_daily_limit_pct / 100, // 转换为小数
          effective_date: null, // 立即生效
          reason: values.reason,
          operator: 'system', // TODO: 从用户上下文获取
        })
      );

      await Promise.all(promises);

      message.success(`已为 ${selectedMachines.length} 个机组更新默认配置`);
      form.resetFields(['reason']);
    } catch (e: any) {
      console.error('[MachineConfigPanel] update config failed:', e);
    }
  };

  const handleShowHistory = async (machineCode: string) => {
    setHistoryMachine(machineCode);
    setHistoryModalOpen(true);
    await getConfigHistory(machineCode);
  };

  const handleCloseHistory = () => {
    setHistoryModalOpen(false);
    setHistoryMachine('');
    clearConfigHistory();
  };

  // 历史记录列
  const historyColumns = [
    {
      title: '版本ID',
      dataIndex: 'version_id',
      width: 120,
      ellipsis: true,
    },
    {
      title: '目标产能(t/天)',
      dataIndex: 'default_daily_target_t',
      width: 140,
      render: (v: number) => v.toFixed(3),
    },
    {
      title: '极限产能',
      dataIndex: 'default_daily_limit_pct',
      width: 110,
      render: (v: number) => `${(v * 100).toFixed(1)}%`,
    },
    {
      title: '操作人',
      dataIndex: 'created_by',
      width: 100,
    },
    {
      title: '创建时间',
      dataIndex: 'created_at',
      width: 180,
    },
    {
      title: '原因',
      dataIndex: 'reason',
      ellipsis: true,
      render: (v: string | null) => v || '-',
    },
  ];

  return (
    <>
      <Space direction="vertical" style={{ width: '100%' }} size={12}>
        {/* 机组选择卡片 */}
        <Card
          title={
            <Space>
              <SettingOutlined />
              <span>机组选择</span>
            </Space>
          }
          size="small"
          loading={configsLoading}
          bodyStyle={{ padding: 12 }}
        >
          <Select
            mode="multiple"
            style={{ width: '100%', marginBottom: 12 }}
            placeholder="选择一个或多个机组"
            value={selectedMachines}
            onChange={setSelectedMachines}
            options={AVAILABLE_MACHINES.map((code) => ({
              label: code,
              value: code,
            }))}
            maxTagCount="responsive"
            size="small"
          />

          {/* 当前配置预览 - 使用折叠面板 */}
          {selectedConfigs.length > 0 && (
            <Collapse
              size="small"
              items={[
                {
                  key: 'current-config',
                  label: (
                    <Text type="secondary" style={{ fontSize: 12 }}>
                      当前配置 ({selectedMachines.length}个机组)
                    </Text>
                  ),
                  children: (
                    <Space direction="vertical" style={{ width: '100%' }} size={4}>
                      {selectedConfigs.map(({ machineCode, config }) => (
                        <div
                          key={machineCode}
                          style={{
                            display: 'flex',
                            justifyContent: 'space-between',
                            alignItems: 'center',
                            padding: '4px 8px',
                            background: '#fafafa',
                            borderRadius: 4,
                            fontSize: 12,
                          }}
                        >
                          <Text strong style={{ fontSize: 12 }}>
                            {machineCode}
                          </Text>
                          {config ? (
                            <Space size={4}>
                              <Tag color="blue" style={{ margin: 0, fontSize: 11 }}>
                                {config.default_daily_target_t.toFixed(3)}t
                              </Tag>
                              <Tag color="orange" style={{ margin: 0, fontSize: 11 }}>
                                {(config.default_daily_limit_pct * 100).toFixed(1)}%
                              </Tag>
                              <Button
                                type="link"
                                size="small"
                                icon={<HistoryOutlined />}
                                onClick={() => handleShowHistory(machineCode)}
                                style={{ padding: '0 4px', fontSize: 11 }}
                              >
                                历史
                              </Button>
                            </Space>
                          ) : (
                            <Tag color="default" style={{ margin: 0, fontSize: 11 }}>
                              未配置
                            </Tag>
                          )}
                        </div>
                      ))}
                    </Space>
                  ),
                },
              ]}
            />
          )}
        </Card>

        {/* 日期范围卡片 */}
        <Card
          title={
            <Space>
              <CalendarOutlined />
              <span>日期范围</span>
            </Space>
          }
          size="small"
          bodyStyle={{ padding: 12 }}
        >
          {/* 日期范围选择器 */}
          <RangePicker
            style={{ width: '100%', marginBottom: 8 }}
            value={dateRange}
            onChange={(dates) => {
              if (dates && dates[0] && dates[1]) {
                setDateRange([dates[0], dates[1]]);
              }
            }}
            format="YYYY-MM-DD"
            allowClear={false}
            size="small"
          />

          {/* 快捷选择 */}
          <Space wrap size={[4, 4]} style={{ marginBottom: 8 }}>
            {DATE_RANGE_PRESETS.map((preset) => (
              <Button
                key={preset.label}
                size="small"
                onClick={() => handleQuickSelectPreset(preset)}
                style={{ fontSize: 11 }}
              >
                {preset.label}
              </Button>
            ))}
          </Space>

          {/* 统计信息 */}
          <Row gutter={8}>
            <Col span={8}>
              <Card size="small" bodyStyle={{ padding: '6px 8px', textAlign: 'center' }}>
                <Statistic
                  title={<span style={{ fontSize: 11 }}>天数</span>}
                  value={dateRangeStats.days}
                  valueStyle={{ fontSize: 14, fontWeight: 600 }}
                />
              </Card>
            </Col>
            <Col span={8}>
              <Card size="small" bodyStyle={{ padding: '6px 8px', textAlign: 'center' }}>
                <Statistic
                  title={<span style={{ fontSize: 11 }}>机组</span>}
                  value={dateRangeStats.machines}
                  valueStyle={{ fontSize: 14, fontWeight: 600 }}
                />
              </Card>
            </Col>
            <Col span={8}>
              <Card size="small" bodyStyle={{ padding: '6px 8px', textAlign: 'center' }}>
                <Statistic
                  title={<span style={{ fontSize: 11 }}>记录</span>}
                  value={dateRangeStats.totalRecords}
                  valueStyle={{ fontSize: 14, fontWeight: 600, color: '#1890ff' }}
                />
              </Card>
            </Col>
          </Row>
        </Card>

        {/* 配置表单卡片 */}
        <Card
          title={
            <Space>
              <ThunderboltOutlined />
              <span>产能配置</span>
            </Space>
          }
          size="small"
          bodyStyle={{ padding: 12 }}
        >
          <Form
            form={form}
            layout="vertical"
            size="small"
            initialValues={{
              default_daily_target_t: 1200,
              default_daily_limit_pct: 105,
            }}
          >
            <Form.Item
              label={<span style={{ fontSize: 12 }}>目标产能 (吨/天)</span>}
              name="default_daily_target_t"
              rules={[
                { required: true, message: '请输入目标产能' },
                { type: 'number', min: 100, message: '目标产能必须 ≥ 100' },
              ]}
              style={{ marginBottom: 12 }}
            >
              <InputNumber
                style={{ width: '100%' }}
                precision={3}
                step={10}
                placeholder="例如: 1200.000"
                size="small"
              />
            </Form.Item>

            <Form.Item
              label={<span style={{ fontSize: 12 }}>极限产能 (%)</span>}
              name="default_daily_limit_pct"
              rules={[
                { required: true, message: '请输入极限产能百分比' },
                { type: 'number', min: 100, message: '极限产能必须 ≥ 100%' },
              ]}
              tooltip="相对于目标产能的百分比，通常为 105%-120%"
              style={{ marginBottom: 12 }}
            >
              <InputNumber
                style={{ width: '100%' }}
                precision={1}
                step={1}
                placeholder="例如: 105.0"
                addonAfter="%"
                size="small"
              />
            </Form.Item>

            <Form.Item
              label={<span style={{ fontSize: 12 }}>配置原因</span>}
              name="reason"
              rules={[{ required: true, message: '请填写配置原因（审计要求）' }]}
              style={{ marginBottom: 12 }}
            >
              <Input.TextArea
                rows={2}
                placeholder="例如：根据设备检修后产能提升调整"
                style={{ fontSize: 12 }}
              />
            </Form.Item>

            <Form.Item style={{ marginBottom: 0 }}>
              <Space direction="vertical" style={{ width: '100%' }} size={6}>
                <Button
                  type="primary"
                  icon={<CalendarOutlined />}
                  onClick={handleBatchApply}
                  loading={applyConfigLoading}
                  block
                  size="small"
                >
                  批量应用到日期范围
                </Button>
                <Button
                  icon={<CheckCircleOutlined />}
                  onClick={handleUpdateMachineConfig}
                  loading={updateConfigLoading}
                  block
                  size="small"
                >
                  更新机组默认配置
                </Button>
              </Space>
            </Form.Item>
          </Form>

          <Alert
            type="info"
            message={<span style={{ fontSize: 11 }}>操作说明</span>}
            description={
              <ul style={{ margin: '4px 0', paddingLeft: 16, fontSize: 11, lineHeight: 1.6 }}>
                <li>批量应用：将配置应用到指定日期范围的产能池记录</li>
                <li>更新默认配置：更新机组默认值（用于新建记录）</li>
              </ul>
            }
            style={{ marginTop: 12 }}
          />
        </Card>
      </Space>

      {/* 历史记录模态框 */}
      <Modal
        title={`${historyMachine} 配置历史（跨版本）`}
        open={historyModalOpen}
        onCancel={handleCloseHistory}
        footer={null}
        width={1000}
      >
        <Table
          dataSource={configHistory}
          columns={historyColumns}
          rowKey="config_id"
          size="small"
          pagination={{ pageSize: 10 }}
          scroll={{ x: 800 }}
        />
      </Modal>
    </>
  );
};

export default MachineConfigPanel;
