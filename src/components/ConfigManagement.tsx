import React, { useState, useEffect } from 'react';
import { useDebounce } from '../hooks/useDebounce';
import {
  Card,
  Table,
  Button,
  Space,
  Select,
  Modal,
  Input,
  message,
  Row,
  Col,
  Tag,
  Descriptions,
  Tooltip,
} from 'antd';
import {
  ReloadOutlined,
  EditOutlined,
  DownloadOutlined,
  UploadOutlined,
  SettingOutlined,
} from '@ant-design/icons';
import type { ColumnsType } from 'antd/es/table';
import { configApi } from '../api/tauri';
import { useCurrentUser } from '../stores/use-global-store';
import { save, open } from '@tauri-apps/api/dialog';
import { writeTextFile, readTextFile } from '@tauri-apps/api/fs';
import { tableEmptyConfig } from './CustomEmpty';

const { Option } = Select;

interface ConfigItem {
  scope_id: string;
  scope_type: string;
  key: string;
  value: string;
  updated_at?: string;
}

const ConfigManagement: React.FC = () => {
  const [loading, setLoading] = useState(false);
  const [configs, setConfigs] = useState<ConfigItem[]>([]);
  const [filteredConfigs, setFilteredConfigs] = useState<ConfigItem[]>([]);
  const [selectedScopeType, setSelectedScopeType] = useState<string>('all');
  const [searchText, setSearchText] = useState<string>('');
  const [editModalVisible, setEditModalVisible] = useState(false);
  const [editingConfig, setEditingConfig] = useState<ConfigItem | null>(null);
  const [editValue, setEditValue] = useState('');
  const [updateReason, setUpdateReason] = useState('');
  const currentUser = useCurrentUser();

  // 防抖搜索文本（延迟 300ms）
  const debouncedSearchText = useDebounce(searchText, 300);

  const loadConfigs = async () => {
    setLoading(true);
    try {
      const result = await configApi.listConfigs();
      setConfigs(result);
      setFilteredConfigs(result);
      message.success(`成功加载 ${result.length} 条配置`);
    } catch (error: any) {
      console.error('加载配置失败:', error);
    } finally {
      setLoading(false);
    }
  };

  const handleEdit = (record: ConfigItem) => {
    setEditingConfig(record);
    setEditValue(record.value);
    setUpdateReason('');
    setEditModalVisible(true);
  };

  const handleUpdate = async () => {
    if (!editingConfig) return;
    if (!updateReason.trim()) {
      message.warning('请输入修改原因');
      return;
    }

    setLoading(true);
    try {
      await configApi.updateConfig(
        editingConfig.scope_id,
        editingConfig.key,
        editValue,
        currentUser,
        updateReason
      );
      message.success('配置更新成功');
      setEditModalVisible(false);
      await loadConfigs();
    } catch (error: any) {
      console.error('更新配置失败:', error);
    } finally {
      setLoading(false);
    }
  };

  const handleExportSnapshot = async () => {
    try {
      const snapshot = await configApi.getConfigSnapshot();
      const filePath = await save({
        defaultPath: `config_snapshot_${Date.now()}.json`,
        filters: [{ name: 'JSON', extensions: ['json'] }],
      });

      if (filePath) {
        await writeTextFile(filePath, JSON.stringify(snapshot, null, 2));
        message.success('配置快照导出成功');
      }
    } catch (error: any) {
      console.error('导出快照失败:', error);
    }
  };

  const handleImportSnapshot = async () => {
    try {
      const filePath = await open({
        multiple: false,
        filters: [{ name: 'JSON', extensions: ['json'] }],
      });

      if (filePath && typeof filePath === 'string') {
        const content = await readTextFile(filePath);

        Modal.confirm({
          title: '确认恢复配置',
          content: '此操作将覆盖当前所有配置，是否继续？',
          onOk: async () => {
            setLoading(true);
            try {
              await configApi.restoreFromSnapshot(
                content,
                currentUser,
                '从快照恢复配置'
              );
              message.success('配置恢复成功');
              await loadConfigs();
            } catch (error: any) {
              console.error('恢复配置失败:', error);
            } finally {
              setLoading(false);
            }
          },
        });
      }
    } catch (error: any) {
      console.error('导入快照失败:', error);
    }
  };

  const filterData = () => {
    let filtered = [...configs];

    // 按作用域类型筛选
    if (selectedScopeType !== 'all') {
      filtered = filtered.filter((item) => item.scope_type === selectedScopeType);
    }

    // 按搜索文本筛选（配置键或值）
    if (debouncedSearchText) {
      const searchLower = debouncedSearchText.toLowerCase();
      filtered = filtered.filter(
        (item) =>
          item.key.toLowerCase().includes(searchLower) ||
          item.value.toLowerCase().includes(searchLower) ||
          item.scope_id.toLowerCase().includes(searchLower)
      );
    }

    setFilteredConfigs(filtered);
  };

  useEffect(() => {
    loadConfigs();
  }, []);

  useEffect(() => {
    filterData();
  }, [selectedScopeType, debouncedSearchText, configs]);

  const scopeTypeColors: Record<string, string> = {
    GLOBAL: 'blue',
    MACHINE: 'green',
    STEEL_GRADE: 'orange',
    VERSION: 'purple',
  };

  const configDescriptions: Record<string, string> = {
    season_mode: '季节模式 (AUTO/MANUAL)',
    winter_months: '冬季月份 (逗号分隔)',
    manual_season: '手动季节 (WINTER/SUMMER)',
    min_temp_days_winter: '冬季最小适温天数',
    min_temp_days_summer: '夏季最小适温天数',
    urgent_n1_days: 'N1紧急天数阈值',
    urgent_n2_days: 'N2紧急天数阈值',
    roll_suggest_threshold_t: '换辊建议阈值(吨)',
    roll_hard_limit_t: '换辊硬限制(吨)',
    overflow_pct: '产能溢出百分比',
    recalc_window_days: '重算窗口天数',
  };

  const columns: ColumnsType<ConfigItem> = [
    {
      title: '作用域类型',
      dataIndex: 'scope_type',
      key: 'scope_type',
      width: 120,
      render: (type: string) => (
        <Tag color={scopeTypeColors[type] || 'default'}>{type}</Tag>
      ),
    },
    {
      title: '作用域ID',
      dataIndex: 'scope_id',
      key: 'scope_id',
      width: 150,
    },
    {
      title: '配置键',
      dataIndex: 'key',
      key: 'key',
      width: 200,
      render: (key: string) => (
        <Tooltip title={configDescriptions[key] || '无描述'}>
          <span style={{ cursor: 'help' }}>{key}</span>
        </Tooltip>
      ),
    },
    {
      title: '配置值',
      dataIndex: 'value',
      key: 'value',
      width: 150,
      render: (value: string) => (
        <span style={{ fontWeight: 'bold', color: '#1890ff' }}>{value}</span>
      ),
    },
    {
      title: '更新时间',
      dataIndex: 'updated_at',
      key: 'updated_at',
      width: 180,
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
          编辑
        </Button>
      ),
    },
  ];

  const scopeTypeCounts = configs.reduce((acc, config) => {
    acc[config.scope_type] = (acc[config.scope_type] || 0) + 1;
    return acc;
  }, {} as Record<string, number>);

  return (
    <div style={{ padding: '24px' }}>
      <Row justify="space-between" align="middle" style={{ marginBottom: 16 }}>
        <Col>
          <h2 style={{ margin: 0 }}>
            <SettingOutlined /> 配置管理
          </h2>
        </Col>
        <Col>
          <Space>
            <Button icon={<ReloadOutlined />} onClick={loadConfigs}>
              刷新
            </Button>
            <Button icon={<DownloadOutlined />} onClick={handleExportSnapshot}>
              导出快照
            </Button>
            <Button icon={<UploadOutlined />} onClick={handleImportSnapshot}>
              导入快照
            </Button>
          </Space>
        </Col>
      </Row>

      {/* 统计卡片 */}
      <Row gutter={16} style={{ marginBottom: 16 }}>
        <Col span={6}>
          <Card>
            <div style={{ textAlign: 'center' }}>
              <div style={{ fontSize: 24, fontWeight: 'bold', color: '#1890ff' }}>
                {configs.length}
              </div>
              <div style={{ color: '#8c8c8c' }}>总配置数</div>
            </div>
          </Card>
        </Col>
        {Object.entries(scopeTypeCounts).map(([type, count]) => (
          <Col span={6} key={type}>
            <Card>
              <div style={{ textAlign: 'center' }}>
                <div style={{ fontSize: 24, fontWeight: 'bold', color: scopeTypeColors[type] }}>
                  {count}
                </div>
                <div style={{ color: '#8c8c8c' }}>{type}</div>
              </div>
            </Card>
          </Col>
        ))}
      </Row>

      {/* 筛选栏 */}
      <Card style={{ marginBottom: 16 }}>
        <Space wrap>
          <Input
            placeholder="搜索配置键、值或作用域ID"
            value={searchText}
            onChange={(e) => setSearchText(e.target.value)}
            style={{ width: 250 }}
            allowClear
          />

          <Select
            style={{ width: 200 }}
            placeholder="选择作用域类型"
            value={selectedScopeType}
            onChange={setSelectedScopeType}
          >
            <Option value="all">全部类型</Option>
            <Option value="GLOBAL">GLOBAL</Option>
            <Option value="MACHINE">MACHINE</Option>
            <Option value="STEEL_GRADE">STEEL_GRADE</Option>
            <Option value="VERSION">VERSION</Option>
          </Select>

          <Button
            onClick={() => {
              setSearchText('');
              setSelectedScopeType('all');
            }}
          >
            清除筛选
          </Button>
        </Space>
      </Card>

      {/* 配置表格 */}
      <Card>
        <Table
          columns={columns}
          dataSource={filteredConfigs}
          loading={loading}
          rowKey={(record) => `${record.scope_id}-${record.key}`}
          locale={tableEmptyConfig}
          pagination={{
            pageSize: 20,
            showSizeChanger: true,
            showTotal: (total) => `共 ${total} 条配置`,
          }}
          scroll={{ x: 1000 }}
          size="small"
        />
      </Card>

      {/* 编辑模态框 */}
      <Modal
        title="编辑配置"
        open={editModalVisible}
        onOk={handleUpdate}
        onCancel={() => setEditModalVisible(false)}
        confirmLoading={loading}
        width={600}
      >
        {editingConfig && (
          <Space direction="vertical" style={{ width: '100%' }}>
            <Descriptions bordered column={1} size="small">
              <Descriptions.Item label="作用域类型">
                <Tag color={scopeTypeColors[editingConfig.scope_type]}>
                  {editingConfig.scope_type}
                </Tag>
              </Descriptions.Item>
              <Descriptions.Item label="作用域ID">
                {editingConfig.scope_id}
              </Descriptions.Item>
              <Descriptions.Item label="配置键">
                {editingConfig.key}
              </Descriptions.Item>
              <Descriptions.Item label="配置说明">
                {configDescriptions[editingConfig.key] || '无描述'}
              </Descriptions.Item>
            </Descriptions>

            <div style={{ marginTop: 16 }}>
              <label>配置值:</label>
              <Input
                style={{ marginTop: 8 }}
                value={editValue}
                onChange={(e) => setEditValue(e.target.value)}
                placeholder="请输入配置值"
              />
            </div>

            <div>
              <label>修改原因(必填):</label>
              <Input.TextArea
                style={{ marginTop: 8 }}
                placeholder="请输入修改原因"
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

export default ConfigManagement;
