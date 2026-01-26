import React, { useState, useEffect } from 'react';
import { useDebounce } from '../hooks/useDebounce';
import { Table, Button, Space, message, Modal, Input, InputNumber, Card } from 'antd';
import type { ColumnsType } from 'antd/es/table';
import { ExclamationCircleOutlined } from '@ant-design/icons';
import { planApi } from '../api/tauri';
import { useCurrentUser, useGlobalActions } from '../stores/use-global-store';
import dayjs from 'dayjs';
import { formatDate } from '../utils/formatters';

interface Plan {
  plan_id: string;
  plan_name: string;
  created_by: string;
  created_at: string;
}

interface Version {
  version_id: string;
  version_no: number;
  status: string;
  recalc_window_days: number;
  created_at: string;
}

const PlanManagement: React.FC = () => {
  const [plans, setPlans] = useState<Plan[]>([]);
  const [filteredPlans, setFilteredPlans] = useState<Plan[]>([]);
  const [versions, setVersions] = useState<Version[]>([]);
  const [loading, setLoading] = useState(false);
  const [selectedPlanId, setSelectedPlanId] = useState<string | null>(null);
  const [createPlanVisible, setCreatePlanVisible] = useState(false);
  const [createVersionVisible, setCreateVersionVisible] = useState(false);
  const [planName, setPlanName] = useState('');
  const [windowDays, setWindowDays] = useState(30);
  const [compareModalVisible, setCompareModalVisible] = useState(false);
  const [selectedVersions, setSelectedVersions] = useState<string[]>([]);
  const [compareResult, setCompareResult] = useState<any>(null);
  const [planSearchText, setPlanSearchText] = useState('');
  const currentUser = useCurrentUser();
  const { setRecalculating, setActiveVersion } = useGlobalActions();

  // 防抖搜索文本（延迟 300ms）
  const debouncedPlanSearchText = useDebounce(planSearchText, 300);

  const planColumns: ColumnsType<Plan> = [
    {
      title: '方案名称',
      dataIndex: 'plan_name',
      key: 'plan_name',
    },
    {
      title: '创建人',
      dataIndex: 'created_by',
      key: 'created_by',
    },
    {
      title: '创建时间',
      dataIndex: 'created_at',
      key: 'created_at',
    },
    {
      title: '操作',
      key: 'action',
      render: (_, record) => (
        <Space>
          <Button size="small" onClick={() => loadVersions(record.plan_id)}>
            查看版本
          </Button>
          <Button size="small" onClick={() => handleCreateVersion(record.plan_id)}>
            创建版本
          </Button>
          <Button
            size="small"
            danger
            onClick={() => handleDeletePlan(record)}
          >
            删除
          </Button>
        </Space>
      ),
    },
  ];

  const versionColumns: ColumnsType<Version> = [
    {
      title: '版本号',
      dataIndex: 'version_no',
      key: 'version_no',
    },
    {
      title: '状态',
      dataIndex: 'status',
      key: 'status',
    },
    {
      title: '窗口天数',
      dataIndex: 'recalc_window_days',
      key: 'recalc_window_days',
    },
    {
      title: '创建时间',
      dataIndex: 'created_at',
      key: 'created_at',
    },
    {
      title: '操作',
      key: 'action',
      render: (_, record) => (
        <Space>
          <Button
            size="small"
            type="primary"
            disabled={record.status === 'ACTIVE'}
            onClick={() => handleActivateVersion(record.version_id)}
          >
            {record.status === 'ACTIVE' ? '已激活' : '激活'}
          </Button>
          {record.status === 'ACTIVE' && (
            <Button
              size="small"
              type="default"
              onClick={() => handleRecalc(record.version_id)}
            >
              一键重算
            </Button>
          )}
          {record.status !== 'ACTIVE' && (
            <Button
              size="small"
              danger
              onClick={() => handleDeleteVersion(record)}
            >
              删除
            </Button>
          )}
        </Space>
      ),
    },
  ];

  const loadPlans = async () => {
    setLoading(true);
    try {
      const result = await planApi.listPlans();
      setPlans(result);
      setFilteredPlans(result);
    } catch (error: any) {
      message.error(`加载失败: ${error.message || error}`);
    } finally {
      setLoading(false);
    }
  };

  // 筛选排产方案
  const filterPlans = () => {
    let filtered = [...plans];

    // 按搜索文本筛选（方案名称或创建人）
    if (debouncedPlanSearchText) {
      const searchLower = debouncedPlanSearchText.toLowerCase();
      filtered = filtered.filter(
        (plan) =>
          plan.plan_name.toLowerCase().includes(searchLower) ||
          plan.created_by.toLowerCase().includes(searchLower)
      );
    }

    setFilteredPlans(filtered);
  };

  const loadVersions = async (planId: string) => {
    setSelectedPlanId(planId);
    setLoading(true);
    try {
      const result = await planApi.listVersions(planId);
      setVersions(result);
      const active = (result || []).find((v: Version) => v.status === 'ACTIVE');
      if (active) {
        setActiveVersion(active.version_id);
      }
    } catch (error: any) {
      message.error(`加载失败: ${error.message || error}`);
    } finally {
      setLoading(false);
    }
  };

  const handleCreatePlan = async () => {
    if (!planName.trim()) {
      message.warning('请输入方案名称');
      return;
    }

    setLoading(true);
    try {
      await planApi.createPlan(planName, currentUser);
      message.success('创建成功');
      setCreatePlanVisible(false);
      setPlanName('');
      await loadPlans();
    } catch (error: any) {
      message.error(`创建失败: ${error.message || error}`);
    } finally {
      setLoading(false);
    }
  };

  const handleCreateVersion = (planId: string) => {
    setSelectedPlanId(planId);
    setCreateVersionVisible(true);
  };

  const handleCreateVersionSubmit = async () => {
    if (!selectedPlanId) return;

    setLoading(true);
    try {
      await planApi.createVersion(selectedPlanId, windowDays, undefined, undefined, currentUser);
      message.success('创建版本成功');
      setCreateVersionVisible(false);
      setWindowDays(30);
      await loadVersions(selectedPlanId);
    } catch (error: any) {
      message.error(`创建失败: ${error.message || error}`);
    } finally {
      setLoading(false);
    }
  };

  const handleActivateVersion = async (versionId: string) => {
    setLoading(true);
    try {
      await planApi.activateVersion(versionId, currentUser);
      setActiveVersion(versionId);
      message.success('激活成功');
      if (selectedPlanId) {
        await loadVersions(selectedPlanId);
      }
    } catch (error: any) {
      message.error(`激活失败: ${error.message || error}`);
    } finally {
      setLoading(false);
    }
  };

  const handleDeletePlan = async (plan: Plan) => {
    Modal.confirm({
      title: '确认删除排产方案？',
      icon: <ExclamationCircleOutlined />,
      content: (
        <div>
          <p>
            将删除方案 <strong>{plan.plan_name}</strong>，并级联删除其所有版本与排产明细。
          </p>
          <p style={{ marginBottom: 0 }}>该操作不可恢复（建议先备份数据库文件）。</p>
        </div>
      ),
      okText: '删除',
      okButtonProps: { danger: true },
      cancelText: '取消',
      onOk: async () => {
        setLoading(true);
        try {
          await planApi.deletePlan(plan.plan_id, currentUser);
          message.success('删除成功');

          // 如果当前正在查看该方案，清空右侧版本区
          if (selectedPlanId === plan.plan_id) {
            setSelectedPlanId(null);
            setVersions([]);
            setSelectedVersions([]);
          }

          // 删除后尝试自动回填最新激活版本
          try {
            const latest = await planApi.getLatestActiveVersionId();
            setActiveVersion(latest || null);
          } catch {
            // 忽略：该错误已由 IpcClient 统一处理
          }

          await loadPlans();
        } catch (error: any) {
          message.error(`删除失败: ${error.message || error}`);
        } finally {
          setLoading(false);
        }
      },
    });
  };

  const handleDeleteVersion = async (version: Version) => {
    Modal.confirm({
      title: '确认删除版本？',
      icon: <ExclamationCircleOutlined />,
      content: (
        <div>
          <p>
            将删除版本 <strong>V{version.version_no}</strong>（{version.version_id}）及其排产明细。
          </p>
          <p style={{ marginBottom: 0 }}>该操作不可恢复。</p>
        </div>
      ),
      okText: '删除',
      okButtonProps: { danger: true },
      cancelText: '取消',
      onOk: async () => {
        setLoading(true);
        try {
          await planApi.deleteVersion(version.version_id, currentUser);
          message.success('删除成功');
          setSelectedVersions((prev) => prev.filter((id) => id !== version.version_id));
          if (selectedPlanId) {
            await loadVersions(selectedPlanId);
          }
        } catch (error: any) {
          message.error(`删除失败: ${error.message || error}`);
        } finally {
          setLoading(false);
        }
      },
    });
  };

  const handleRecalc = async (versionId: string) => {
    setRecalculating(true);
    try {
      const baseDate = formatDate(dayjs());
      await planApi.recalcFull(versionId, baseDate, undefined, currentUser);
      message.success('重算完成');
      if (selectedPlanId) {
        await loadVersions(selectedPlanId);
      }
    } catch (error: any) {
      console.error('重算失败:', error);
    } finally {
      setRecalculating(false);
    }
  };

  const handleCompareVersions = async () => {
    if (selectedVersions.length !== 2) {
      message.warning('请选择两个版本进行对比');
      return;
    }

    setLoading(true);
    try {
      const result = await planApi.compareVersions(selectedVersions[0], selectedVersions[1]);
      setCompareResult(result);
      setCompareModalVisible(true);
      message.success('版本对比完成');
    } catch (error: any) {
      console.error('版本对比失败:', error);
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    loadPlans();
  }, []);

  // 监听搜索文本变化（使用防抖后的文本）
  useEffect(() => {
    filterPlans();
  }, [debouncedPlanSearchText, plans]);

  return (
    <div>
      <Space style={{ marginBottom: 16 }} wrap>
        <Button type="primary" onClick={() => setCreatePlanVisible(true)}>
          创建方案
        </Button>
        <Button onClick={loadPlans}>刷新</Button>
      </Space>

      <Space style={{ marginBottom: 16 }} wrap>
        <Input
          placeholder="搜索方案名称或创建人"
          value={planSearchText}
          onChange={(e) => setPlanSearchText(e.target.value)}
          style={{ width: 250 }}
          allowClear
        />
        <Button onClick={() => setPlanSearchText('')}>清除搜索</Button>
      </Space>

      <h3>排产方案列表</h3>
      <Table
        columns={planColumns}
        dataSource={filteredPlans}
        rowKey="plan_id"
        loading={loading}
        pagination={false}
      />

      {selectedPlanId && (
        <>
          <h3 style={{ marginTop: 24 }}>版本列表</h3>
          <Space style={{ marginBottom: 16 }}>
            <Button
              type="primary"
              disabled={selectedVersions.length !== 2}
              onClick={handleCompareVersions}
            >
              对比选中版本
            </Button>
            <Button onClick={() => setSelectedVersions([])}>清除选择</Button>
          </Space>
          <Table
            columns={versionColumns}
            dataSource={versions}
            rowKey="version_id"
            loading={loading}
            pagination={false}
            rowSelection={{
              type: 'checkbox',
              selectedRowKeys: selectedVersions,
              onChange: (selectedKeys) => {
                if (selectedKeys.length <= 2) {
                  setSelectedVersions(selectedKeys as string[]);
                } else {
                  message.warning('最多只能选择2个版本进行对比');
                }
              },
            }}
          />
        </>
      )}

      <Modal
        title="创建排产方案"
        open={createPlanVisible}
        onOk={handleCreatePlan}
        onCancel={() => {
          setCreatePlanVisible(false);
          setPlanName('');
        }}
        confirmLoading={loading}
      >
        <Input
          placeholder="请输入方案名称"
          value={planName}
          onChange={(e) => setPlanName(e.target.value)}
        />
      </Modal>

      <Modal
        title="创建新版本"
        open={createVersionVisible}
        onOk={handleCreateVersionSubmit}
        onCancel={() => {
          setCreateVersionVisible(false);
          setWindowDays(30);
        }}
        confirmLoading={loading}
      >
        <Space direction="vertical" style={{ width: '100%' }}>
          <div>
            <label>窗口天数：</label>
            <InputNumber
              min={1}
              max={60}
              value={windowDays}
              onChange={(val) => setWindowDays(val || 30)}
            />
          </div>
        </Space>
      </Modal>

      <Modal
        title="版本对比结果"
        open={compareModalVisible}
        onCancel={() => {
          setCompareModalVisible(false);
          setCompareResult(null);
        }}
        footer={[
          <Button key="close" onClick={() => setCompareModalVisible(false)}>
            关闭
          </Button>,
        ]}
        width={800}
      >
        {compareResult && (
          <Space direction="vertical" style={{ width: '100%' }}>
            <Card title="对比摘要" size="small">
              <p>版本A: {selectedVersions[0]}</p>
              <p>版本B: {selectedVersions[1]}</p>
            </Card>

            {compareResult.moved_items && compareResult.moved_items.length > 0 && (
              <Card title={`移动的材料 (${compareResult.moved_items.length})`} size="small">
                <ul>
                  {compareResult.moved_items.slice(0, 10).map((item: any, idx: number) => (
                    <li key={idx}>
                      {item.material_id}: {item.old_date} → {item.new_date}
                    </li>
                  ))}
                  {compareResult.moved_items.length > 10 && (
                    <li>...还有 {compareResult.moved_items.length - 10} 项</li>
                  )}
                </ul>
              </Card>
            )}

            {compareResult.added_items && compareResult.added_items.length > 0 && (
              <Card title={`新增的材料 (${compareResult.added_items.length})`} size="small">
                <ul>
                  {compareResult.added_items.slice(0, 10).map((item: any, idx: number) => (
                    <li key={idx}>{item.material_id}</li>
                  ))}
                  {compareResult.added_items.length > 10 && (
                    <li>...还有 {compareResult.added_items.length - 10} 项</li>
                  )}
                </ul>
              </Card>
            )}

            {compareResult.removed_items && compareResult.removed_items.length > 0 && (
              <Card title={`移除的材料 (${compareResult.removed_items.length})`} size="small">
                <ul>
                  {compareResult.removed_items.slice(0, 10).map((item: any, idx: number) => (
                    <li key={idx}>{item.material_id}</li>
                  ))}
                  {compareResult.removed_items.length > 10 && (
                    <li>...还有 {compareResult.removed_items.length - 10} 项</li>
                  )}
                </ul>
              </Card>
            )}

            {compareResult.risk_change && (
              <Card title="风险变化" size="small">
                <p>版本A风险分数: {compareResult.risk_change.version_a_score}</p>
                <p>版本B风险分数: {compareResult.risk_change.version_b_score}</p>
                <p>
                  变化:{' '}
                  {compareResult.risk_change.version_b_score -
                    compareResult.risk_change.version_a_score >
                  0
                    ? '↑'
                    : '↓'}{' '}
                  {Math.abs(
                    compareResult.risk_change.version_b_score -
                      compareResult.risk_change.version_a_score
                  )}
                </p>
              </Card>
            )}
          </Space>
        )}
      </Modal>
    </div>
  );
};

export default PlanManagement;
