import React, { useCallback, useEffect, useMemo, useState } from 'react';
import { useDebounce } from '../../hooks/useDebounce';
import { Button, Divider, Input, InputNumber, Modal, Space, Table, Typography, message } from 'antd';
import { ExclamationCircleOutlined } from '@ant-design/icons';
import { useNavigate } from 'react-router-dom';
import dayjs from 'dayjs';
import { planApi } from '../../api/tauri';
import { useCurrentUser, useGlobalActions, useGlobalStore } from '../../stores/use-global-store';
import { formatDate } from '../../utils/formatters';
import { getLatestRunTtlMs } from '../../stores/latestRun';
import { createRunId } from '../../utils/runId';
import { getErrorMessage } from '../../utils/errorUtils';
import VersionComparisonModal from '../comparison/VersionComparisonModal';
import type { Plan, Version } from '../comparison/types';
import { formatVersionLabel } from '../comparison/utils';
import { createPlanColumns, createVersionColumns } from './columns';
import { useVersionComparison } from './useVersionComparison';

const PlanManagement: React.FC = () => {
  const navigate = useNavigate();
  const [plans, setPlans] = useState<Plan[]>([]);
  const [versions, setVersions] = useState<Version[]>([]);
  const [loading, setLoading] = useState(false);
  const [selectedPlanId, setSelectedPlanId] = useState<string | null>(null);
  const [createPlanVisible, setCreatePlanVisible] = useState(false);
  const [createVersionVisible, setCreateVersionVisible] = useState(false);
  const [planName, setPlanName] = useState('');
  const [windowDays, setWindowDays] = useState(30);
  const [selectedVersions, setSelectedVersions] = useState<string[]>([]);
  const [planSearchText, setPlanSearchText] = useState('');
  const currentUser = useCurrentUser();
  const {
    setRecalculating,
    setActiveVersion,
    beginLatestRun,
    markLatestRunRunning,
    markLatestRunDone,
    markLatestRunFailed,
    expireLatestRunIfNeeded,
  } = useGlobalActions();

  const debouncedPlanSearchText = useDebounce(planSearchText, 300);

  const loadPlans = useCallback(async () => {
    setLoading(true);
    try {
      const result = await planApi.listPlans();
      setPlans(result);
    } catch (error: unknown) {
      message.error(`加载失败: ${getErrorMessage(error)}`);
    } finally {
      setLoading(false);
    }
  }, []);

  const filteredPlans = useMemo(() => {
    if (!debouncedPlanSearchText) return plans;
    const searchLower = debouncedPlanSearchText.toLowerCase();
    return plans.filter(
      (plan) =>
        plan.plan_name.toLowerCase().includes(searchLower) ||
        plan.created_by.toLowerCase().includes(searchLower)
    );
  }, [debouncedPlanSearchText, plans]);

  const loadVersions = useCallback(
    async (planId: string) => {
      setSelectedPlanId(planId);
      setSelectedVersions([]);
      setLoading(true);
      try {
        const result = await planApi.listVersions(planId);
        setVersions(result);
        const active = (result || []).find((v: Version) => v.status === 'ACTIVE');
        if (active) {
          setActiveVersion(active.version_id);
        }
      } catch (error: unknown) {
        message.error(`加载失败: ${getErrorMessage(error)}`);
      } finally {
        setLoading(false);
      }
    },
    [setActiveVersion]
  );

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
    } catch (error: unknown) {
      message.error(`创建失败: ${getErrorMessage(error)}`);
    } finally {
      setLoading(false);
    }
  };

  const handleCreateVersion = useCallback((planId: string) => {
    setSelectedPlanId(planId);
    setSelectedVersions([]);
    setCreateVersionVisible(true);
  }, []);

  const handleCreateVersionSubmit = async () => {
    if (!selectedPlanId) return;

    setLoading(true);
    try {
      await planApi.createVersion(selectedPlanId, windowDays, undefined, undefined, currentUser);
      message.success('创建版本成功');
      setCreateVersionVisible(false);
      setWindowDays(30);
      await loadVersions(selectedPlanId);
    } catch (error: unknown) {
      message.error(`创建失败: ${getErrorMessage(error)}`);
    } finally {
      setLoading(false);
    }
  };

  const handleActivateVersion = useCallback(
    async (versionId: string) => {
      if (!selectedPlanId) {
        message.warning('请先选择方案');
        return;
      }

      const target = versions.find((v) => v.version_id === versionId);
      const label = target ? formatVersionLabel(target) : versionId;

      let inputReason = '';
      Modal.confirm({
        title: `确认回滚并激活 ${label}？`,
        icon: <ExclamationCircleOutlined />,
        okText: '确认回滚',
        okButtonProps: { danger: true },
        cancelText: '取消',
        content: (
          <div>
            <p style={{ marginBottom: 8 }}>
              该操作将把当前方案的激活版本切换为 <strong>{label}</strong>（{versionId}）。
            </p>
            <p style={{ marginBottom: 8 }}>
              回滚会尝试<strong>恢复该版本的配置快照</strong>（覆盖当前全局配置），并触发决策数据刷新。
            </p>
            <p style={{ marginBottom: 8 }}>请填写回滚原因（将写入审计日志）：</p>
            <Input.TextArea
              rows={3}
              placeholder="例如：回滚到上周稳定版本，等待产能参数确认后再发布新方案"
              onChange={(e) => {
                inputReason = e.target.value;
              }}
            />
            <Typography.Text type="secondary" style={{ fontSize: 12 }}>
              提示：回滚完成后，驾驶舱/风险等数据可能需要几十秒刷新。
            </Typography.Text>
          </div>
        ),
        onOk: async () => {
          const reason = String(inputReason || '').trim();
          if (!reason) {
            message.warning('请输入回滚原因');
            return Promise.reject(new Error('MISSING_REASON'));
          }

          setLoading(true);
          try {
            const res = await planApi.rollbackVersion(selectedPlanId, versionId, currentUser, reason);
            setActiveVersion(versionId);
            message.success('回滚成功');
            if (res?.config_restore_skipped) {
              message.warning(String(res.config_restore_skipped));
            }
            await loadVersions(selectedPlanId);
          } catch (error: unknown) {
            message.error(`回滚失败: ${getErrorMessage(error)}`);
            throw error;
          } finally {
            setLoading(false);
          }
        },
      });
    },
    [selectedPlanId, versions, currentUser, setActiveVersion, loadVersions]
  );

  const handleDeletePlan = useCallback(
    async (plan: Plan) => {
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

            if (selectedPlanId === plan.plan_id) {
              setSelectedPlanId(null);
              setVersions([]);
              setSelectedVersions([]);
            }

            try {
              const latest = await planApi.getLatestActiveVersionId();
              setActiveVersion(latest || null);
            } catch {
              // ignore (handled by IpcClient)
            }

            await loadPlans();
          } catch (error: unknown) {
            message.error(`删除失败: ${getErrorMessage(error)}`);
          } finally {
            setLoading(false);
          }
        },
      });
    },
    [currentUser, selectedPlanId, setActiveVersion, loadPlans]
  );

  const handleDeleteVersion = useCallback(
    async (version: Version) => {
      const label = formatVersionLabel(version);
      Modal.confirm({
        title: `确认删除版本 ${label}？`,
        icon: <ExclamationCircleOutlined />,
        content: (
          <div>
            <p>
              将删除版本 <strong>{label}</strong>（{version.version_id}）及其排产明细。
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
          } catch (error: unknown) {
            message.error(`删除失败: ${getErrorMessage(error)}`);
          } finally {
            setLoading(false);
          }
        },
      });
    },
    [currentUser, selectedPlanId, loadVersions]
  );

  const handleRecalc = useCallback(
    async (versionId: string) => {
      expireLatestRunIfNeeded();
      const localRunId = createRunId('recalc');
      const beginResult = beginLatestRun({
        runId: localRunId,
        versionId,
        ttlMs: getLatestRunTtlMs(),
      });

      if (!beginResult.accepted) {
        message.info('已存在更新的重算触发，本次请求已忽略');
        return;
      }

      setRecalculating(true);
      markLatestRunRunning(localRunId);
      try {
        const baseDate = formatDate(dayjs());
        const res = await planApi.recalcFull(versionId, baseDate, undefined, currentUser, undefined, undefined, localRunId);

        const responseRunId = String((res as any)?.run_id ?? localRunId).trim() || localRunId;
        const nextVersionId = String((res as any)?.version_id ?? '').trim();
        const planRevRaw = Number((res as any)?.plan_rev);

        markLatestRunDone(responseRunId, {
          versionId: nextVersionId || versionId,
          planRev: Number.isFinite(planRevRaw) ? planRevRaw : undefined,
        });

        const latestRunId = useGlobalStore.getState().latestRun.runId;
        if (latestRunId !== responseRunId) {
          return;
        }

        message.success('重算完成');
        if (selectedPlanId) {
          await loadVersions(selectedPlanId);
        }
      } catch (error: unknown) {
        console.error('重算失败:', error);
        markLatestRunFailed(localRunId, getErrorMessage(error) || '重算失败');
      } finally {
        setRecalculating(false);
      }
    },
    [
      currentUser,
      selectedPlanId,
      loadVersions,
      setRecalculating,
      beginLatestRun,
      markLatestRunRunning,
      markLatestRunDone,
      markLatestRunFailed,
      expireLatestRunIfNeeded,
    ]
  );

  const planColumns = useMemo(
    () => createPlanColumns(loadVersions, handleCreateVersion, handleDeletePlan),
    [loadVersions, handleCreateVersion, handleDeletePlan]
  );

  const versionColumns = useMemo(
    () => createVersionColumns(handleActivateVersion, handleRecalc, handleDeleteVersion),
    [handleActivateVersion, handleRecalc, handleDeleteVersion]
  );

  const versionComparison = useVersionComparison({
    selectedVersions,
    currentUser,
    setLoading,
    onActivateVersion: handleActivateVersion,
  });

  useEffect(() => {
    void loadPlans();
  }, [loadPlans]);

  return (
    <div>
      <Space style={{ marginBottom: 16 }} wrap>
        <Button onClick={() => navigate('/workbench')}>返回计划工作台</Button>
        <Button onClick={() => navigate('/overview')}>返回风险概览</Button>
        <Divider type="vertical" />
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
              onClick={versionComparison.handleCompareVersions}
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

      <VersionComparisonModal {...versionComparison.modalProps} />
    </div>
  );
};

export default PlanManagement;
