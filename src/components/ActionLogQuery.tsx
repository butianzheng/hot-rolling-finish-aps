import React, { useState, useEffect } from 'react';
import { useDebounce } from '../hooks/useDebounce';
import {
  Card,
  Table,
  Space,
  Button,
  Select,
  DatePicker,
  Input,
  Tag,
  Modal,
  Descriptions,
  Alert,
  message,
  Row,
  Col,
  Dropdown,
} from 'antd';
import {
  ReloadOutlined,
  DownloadOutlined,
  EyeOutlined,
  FilterOutlined,
} from '@ant-design/icons';
import type { ColumnsType } from 'antd/es/table';
import dayjs, { Dayjs } from 'dayjs';
import { dashboardApi } from '../api/tauri';
import { formatDateTime } from '../utils/formatters';
import { tableFilterEmptyConfig } from './CustomEmpty';
import { exportCSV, exportJSON } from '../utils/exportUtils';

const { Option } = Select;
const { RangePicker } = DatePicker;
const { Search } = Input;

// 操作日志类型
interface ActionLog {
  action_id: string;
  version_id: string;
  action_type: string;
  action_ts: string;
  actor: string;
  payload_json?: any;
  impact_summary_json?: any;
  machine_code?: string;
  date_range_start?: string;
  date_range_end?: string;
  detail?: string;
}

const ActionLogQuery: React.FC = () => {
  const [loading, setLoading] = useState(false);
  const [actionLogs, setActionLogs] = useState<ActionLog[]>([]);
  const [filteredLogs, setFilteredLogs] = useState<ActionLog[]>([]);
  const [selectedLog, setSelectedLog] = useState<ActionLog | null>(null);
  const [showDetailModal, setShowDetailModal] = useState(false);
  const [loadError, setLoadError] = useState<string | null>(null);

  // 筛选条件
  const [timeRange, setTimeRange] = useState<[Dayjs, Dayjs] | null>(null);
  const [selectedActionType, setSelectedActionType] = useState<string>('all');
  const [selectedActor, setSelectedActor] = useState<string>('all');
  const [selectedVersion, setSelectedVersion] = useState<string>('all');
  const [searchText, setSearchText] = useState('');

  // 防抖搜索文本（延迟 300ms）
  const debouncedSearchText = useDebounce(searchText, 300);

  // 操作类型映射
  const actionTypeLabels: Record<string, { text: string; color: string }> = {
    CREATE_PLAN: { text: '创建方案', color: 'blue' },
    DELETE_PLAN: { text: '删除方案', color: 'red' },
    CREATE_VERSION: { text: '创建版本', color: 'cyan' },
    DELETE_VERSION: { text: '删除版本', color: 'volcano' },
    ACTIVATE_VERSION: { text: '激活版本', color: 'green' },
    RECALC_FULL: { text: '一键重算', color: 'purple' },
    SIMULATE_RECALC: { text: '试算', color: 'geekblue' },
    // 材料操作（兼容旧/新 action_type 命名）
    BATCH_LOCK: { text: '批量锁定', color: 'orange' },
    LOCK_MATERIALS: { text: '批量锁定', color: 'orange' },
    BATCH_UNLOCK: { text: '批量解锁', color: 'lime' },
    UNLOCK_MATERIALS: { text: '批量解锁', color: 'lime' },
    BATCH_FORCE_RELEASE: { text: '批量强制放行', color: 'volcano' },
    FORCE_RELEASE: { text: '强制放行', color: 'volcano' },
    BATCH_SET_URGENT: { text: '设置紧急标志', color: 'red' },
    SET_URGENT: { text: '设置紧急标志', color: 'red' },

    // 排产操作
    MOVE_ITEMS: { text: '移动排产项', color: 'geekblue' },
    UPDATE_CONFIG: { text: '更新配置', color: 'gold' },
    BATCH_UPDATE_CONFIG: { text: '批量更新配置', color: 'gold' },
    RESTORE_CONFIG: { text: '恢复配置', color: 'gold' },
    UPDATE_CAPACITY_POOL: { text: '更新产能池', color: 'gold' },
    CREATE_ROLL_CAMPAIGN: { text: '创建换辊窗口', color: 'magenta' },
    CLOSE_ROLL_CAMPAIGN: { text: '结束换辊窗口', color: 'pink' },

    // 前端遥测/错误上报（写入 action_log，便于复用现有查询页）
    FRONTEND_ERROR: { text: '前端错误', color: 'red' },
    FRONTEND_WARN: { text: '前端告警', color: 'orange' },
    FRONTEND_INFO: { text: '前端日志', color: 'blue' },
    FRONTEND_DEBUG: { text: '前端调试', color: 'default' },
    FRONTEND_EVENT: { text: '前端事件', color: 'geekblue' },
  };

  // 加载操作日志
  const loadActionLogs = async (limit: number = 100) => {
    setLoading(true);
    setLoadError(null);
    try {
      let result;
      if (timeRange) {
        const [start, end] = timeRange;
        result = await dashboardApi.listActionLogs(
          formatDateTime(start),
          formatDateTime(end)
        );
      } else {
        result = await dashboardApi.getRecentActions(limit);
      }

      setActionLogs(result);
      setFilteredLogs(result);
      message.success(`成功加载 ${result.length} 条操作日志`);
    } catch (error: any) {
      console.error('加载操作日志失败:', error);
      const msg = String(error?.message || error || '加载失败');
      setLoadError(msg);
      message.error(`加载失败: ${msg}`);
    } finally {
      setLoading(false);
    }
  };

  // 筛选数据
  const filterData = () => {
    let filtered = [...actionLogs];

    // 按时间范围筛选
    if (timeRange) {
      const [start, end] = timeRange;
      filtered = filtered.filter((log) => {
        const logTime = dayjs(log.action_ts);
        return logTime.isAfter(start.subtract(1, 'second')) && logTime.isBefore(end.add(1, 'second'));
      });
    }

    // 按操作类型筛选
    if (selectedActionType !== 'all') {
      filtered = filtered.filter((log) => log.action_type === selectedActionType);
    }

    // 按操作人筛选
    if (selectedActor !== 'all') {
      filtered = filtered.filter((log) => log.actor === selectedActor);
    }

    // 按版本筛选
    if (selectedVersion !== 'all') {
      filtered = filtered.filter((log) => log.version_id === selectedVersion);
    }

    // 按搜索文本筛选（使用防抖后的搜索文本）
    if (debouncedSearchText) {
      const searchLower = debouncedSearchText.toLowerCase();
      filtered = filtered.filter(
        (log) =>
          log.action_id.toLowerCase().includes(searchLower) ||
          log.detail?.toLowerCase().includes(searchLower)
      );
    }

    setFilteredLogs(filtered);
  };

  // 查看详情
  const handleViewDetail = (log: ActionLog) => {
    setSelectedLog(log);
    setShowDetailModal(true);
  };

  // 表格列定义
  const columns: ColumnsType<ActionLog> = [
    {
      title: '操作时间',
      dataIndex: 'action_ts',
      key: 'action_ts',
      width: 180,
      sorter: (a, b) => a.action_ts.localeCompare(b.action_ts),
      defaultSortOrder: 'descend',
    },
    {
      title: '操作类型',
      dataIndex: 'action_type',
      key: 'action_type',
      width: 150,
      render: (type: string) => {
        const label = actionTypeLabels[type] || { text: type, color: 'default' };
        return <Tag color={label.color}>{label.text}</Tag>;
      },
    },
    {
      title: '操作人',
      dataIndex: 'actor',
      key: 'actor',
      width: 120,
    },
    {
      title: '版本ID',
      dataIndex: 'version_id',
      key: 'version_id',
      width: 120,
    },
    {
      title: '机组',
      dataIndex: 'machine_code',
      key: 'machine_code',
      width: 100,
      render: (code: string | null) => code || '-',
    },
    {
      title: '操作详情',
      dataIndex: 'detail',
      key: 'detail',
      ellipsis: true,
    },
    {
      title: '操作',
      key: 'action',
      width: 100,
      fixed: 'right',
      render: (_, record: ActionLog) => (
        <Button
          type="link"
          size="small"
          icon={<EyeOutlined />}
          onClick={() => handleViewDetail(record)}
        >
          详情
        </Button>
      ),
    },
  ];

  // 初始加载
  useEffect(() => {
    loadActionLogs();
  }, []);

  // 筛选条件变化时重新筛选（使用防抖后的搜索文本）
  useEffect(() => {
    filterData();
  }, [timeRange, selectedActionType, selectedActor, selectedVersion, debouncedSearchText]);

  // 获取唯一的操作人列表
  const getUniqueActors = () => {
    const actors = new Set(actionLogs.map((log) => log.actor));
    return Array.from(actors);
  };

  // 获取唯一的版本列表
  const getUniqueVersions = () => {
    const versions = new Set(actionLogs.map((log) => log.version_id));
    return Array.from(versions);
  };

  return (
    <div style={{ padding: '24px' }}>
      {/* 标题和操作栏 */}
      <Row justify="space-between" align="middle" style={{ marginBottom: 16 }}>
        <Col>
          <h2 style={{ margin: 0 }}>操作日志查询</h2>
        </Col>
        <Col>
          <Space>
            <Button icon={<ReloadOutlined />} onClick={() => loadActionLogs()}>
              刷新
            </Button>
            <Dropdown
              menu={{
                items: [
                  {
                    label: '导出为 CSV',
                    key: 'csv',
                    onClick: () => {
                      try {
                        const data = filteredLogs.map((log) => ({
                          操作时间: log.action_ts,
                          操作类型: actionTypeLabels[log.action_type]?.text || log.action_type,
                          操作人: log.actor,
                          版本ID: log.version_id,
                          机组: log.machine_code || '-',
                          操作详情: log.detail || '-',
                        }));
                        exportCSV(data, '操作日志');
                        message.success('导出成功');
                      } catch (error: any) {
                        message.error(`导出失败: ${error.message}`);
                      }
                    },
                  },
                  {
                    label: '导出为 JSON',
                    key: 'json',
                    onClick: () => {
                      try {
                        exportJSON(filteredLogs, '操作日志');
                        message.success('导出成功');
                      } catch (error: any) {
                        message.error(`导出失败: ${error.message}`);
                      }
                    },
                  },
                ],
              }}
            >
              <Button icon={<DownloadOutlined />}>导出</Button>
            </Dropdown>
          </Space>
        </Col>
      </Row>

      {/* 筛选栏 */}
      <Card style={{ marginBottom: 16 }}>
        {loadError && (
          <Alert
            type="error"
            showIcon
            message="操作日志加载失败"
            description={loadError}
            action={
              <Button size="small" onClick={() => loadActionLogs()}>
                重试
              </Button>
            }
            style={{ marginBottom: 12 }}
          />
        )}
        <Space wrap size="middle">
          <RangePicker
            showTime
            placeholder={['开始时间', '结束时间']}
            value={timeRange as any}
            onChange={(dates) => {
              if (dates && dates[0] && dates[1]) {
                setTimeRange([dates[0], dates[1]]);
              } else {
                setTimeRange(null);
              }
            }}
            format="YYYY-MM-DD HH:mm:ss"
            style={{ width: 400 }}
          />

          <Select
            style={{ width: 150 }}
            placeholder="操作类型"
            value={selectedActionType}
            onChange={setSelectedActionType}
          >
            <Option value="all">全部类型</Option>
            {Object.entries(actionTypeLabels).map(([key, value]) => (
              <Option key={key} value={key}>
                {value.text}
              </Option>
            ))}
          </Select>

          <Select
            style={{ width: 120 }}
            placeholder="操作人"
            value={selectedActor}
            onChange={setSelectedActor}
          >
            <Option value="all">全部操作人</Option>
            {getUniqueActors().map((actor) => (
              <Option key={actor} value={actor}>
                {actor}
              </Option>
            ))}
          </Select>

          <Select
            style={{ width: 120 }}
            placeholder="版本"
            value={selectedVersion}
            onChange={setSelectedVersion}
          >
            <Option value="all">全部版本</Option>
            {getUniqueVersions().map((version) => (
              <Option key={version} value={version}>
                {version}
              </Option>
            ))}
          </Select>

          <Search
            placeholder="搜索操作ID或详情"
            value={searchText}
            onChange={(e) => setSearchText(e.target.value)}
            style={{ width: 250 }}
            allowClear
          />

          <Button
            icon={<FilterOutlined />}
            onClick={() => {
              setTimeRange(null);
              setSelectedActionType('all');
              setSelectedActor('all');
              setSelectedVersion('all');
              setSearchText('');
            }}
          >
            清除筛选
          </Button>
        </Space>
      </Card>

      {/* 操作日志表格 */}
      <Card>
        <Table
          columns={columns}
          dataSource={filteredLogs}
          loading={loading}
          rowKey="action_id"
          locale={tableFilterEmptyConfig}
          virtual
          pagination={{
            pageSize: 20,
            showSizeChanger: true,
            showTotal: (total) => `共 ${total} 条记录`,
          }}
          scroll={{ x: 1200, y: 520 }}
          size="small"
        />
      </Card>

      {/* 详情模态框 */}
      <Modal
        title="操作日志详情"
        open={showDetailModal}
        onCancel={() => setShowDetailModal(false)}
        footer={[
          <Button key="close" onClick={() => setShowDetailModal(false)}>
            关闭
          </Button>,
        ]}
        width={800}
      >
        {selectedLog && (
          <div>
            <Descriptions bordered column={2} size="small">
              <Descriptions.Item label="操作ID" span={2}>
                {selectedLog.action_id}
              </Descriptions.Item>
              <Descriptions.Item label="操作时间" span={2}>
                {selectedLog.action_ts}
              </Descriptions.Item>
              <Descriptions.Item label="操作类型">
                <Tag color={actionTypeLabels[selectedLog.action_type]?.color || 'default'}>
                  {actionTypeLabels[selectedLog.action_type]?.text || selectedLog.action_type}
                </Tag>
              </Descriptions.Item>
              <Descriptions.Item label="操作人">
                {selectedLog.actor}
              </Descriptions.Item>
              <Descriptions.Item label="版本ID">
                {selectedLog.version_id}
              </Descriptions.Item>
              <Descriptions.Item label="机组">
                {selectedLog.machine_code || '-'}
              </Descriptions.Item>
              <Descriptions.Item label="操作详情" span={2}>
                {selectedLog.detail}
              </Descriptions.Item>
            </Descriptions>

            {/* Payload JSON */}
            {selectedLog.payload_json && (
              <Card title="操作参数 (Payload)" size="small" style={{ marginTop: 16 }}>
                <pre style={{ maxHeight: '200px', overflow: 'auto', fontSize: '12px' }}>
                  {JSON.stringify(selectedLog.payload_json, null, 2)}
                </pre>
              </Card>
            )}

            {/* Impact Summary JSON */}
            {selectedLog.impact_summary_json && (
              <Card title="影响摘要 (Impact Summary)" size="small" style={{ marginTop: 16 }}>
                <pre style={{ maxHeight: '200px', overflow: 'auto', fontSize: '12px' }}>
                  {JSON.stringify(selectedLog.impact_summary_json, null, 2)}
                </pre>
              </Card>
            )}
          </div>
        )}
      </Modal>
    </div>
  );
};

export default ActionLogQuery;
