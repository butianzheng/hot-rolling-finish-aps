/**
 * 材料管理 - Scheduler Workbench
 * 使用 ProTable 的专业调度工作台
 *
 * 重构后：1000 行 → ~280 行 (-72%)
 */

import React, { useCallback, useMemo, useRef, useState } from 'react';
import { useQuery, useQueryClient } from '@tanstack/react-query';
import { ProTable } from '@ant-design/pro-components';
import type { ActionType } from '@ant-design/pro-components';
import { Button, Modal, Space, message } from 'antd';
import {
  ReloadOutlined,
  LockOutlined,
  UnlockOutlined,
  FireOutlined,
  StopOutlined,
  WarningOutlined,
  InfoCircleOutlined,
} from '@ant-design/icons';
import { materialApi } from '../../api/tauri';
import { useEvent } from '../../api/eventBus';
import { MaterialInspector } from '../MaterialInspector';
import { RedLineGuard } from '../guards/RedLineGuard';
import { useCurrentUser, useAdminOverrideMode } from '../../stores/use-global-store';
import { normalizeSchedState } from '../../utils/schedState';

import { useMaterialTimeline } from './useMaterialTimeline';
import { CapacityTimelineSection } from './CapacityTimelineSection';
import { MaterialOperationModal } from './MaterialOperationModal';
import { createMaterialTableColumns } from './materialTableColumns';
import { checkRedLineViolations, type Material, type OperationType } from './materialTypes';

// M4修复：定义ProTable搜索参数类型，替换any
interface MaterialSearchParams {
  machine_code?: string;
  sched_state?: string;
  urgent_level?: string;
  manual_urgent_flag?: string | boolean;
  lock_flag?: string | boolean;
  material_id?: string;
  steel_mark?: string;
}

const MaterialManagement: React.FC = () => {
  const actionRef = useRef<ActionType>();
  const queryClient = useQueryClient();
  const [selectedRowKeys, setSelectedRowKeys] = useState<React.Key[]>([]);
  const [selectedMaterial, setSelectedMaterial] = useState<Material | null>(null);
  const [inspectorVisible, setInspectorVisible] = useState(false);
  const [modalVisible, setModalVisible] = useState(false);
  const [modalType, setModalType] = useState<OperationType>('lock');
  const [reason, setReason] = useState('');

  const currentUser = useCurrentUser();
  const adminOverrideMode = useAdminOverrideMode();

  // 产能时间线 Hook
  const timeline = useMaterialTimeline();

  // C8修复：使用React Query缓存全量材料数据，避免每次筛选都重新加载
  const {
    data: allMaterials = [],
    isLoading: isLoadingMaterials,
    error: materialsError,
  } = useQuery({
    queryKey: ['materials', 'all'],
    queryFn: async () => {
      const result = await materialApi.listMaterials({ limit: 1000, offset: 0 });
      return result;
    },
    staleTime: 2 * 60 * 1000, // 2分钟内数据视为新鲜，不重新请求
    gcTime: 5 * 60 * 1000, // 缓存5分钟
  });

  // 订阅事件并刷新缓存
  useEvent('material_state_changed', () => {
    queryClient.invalidateQueries({ queryKey: ['materials', 'all'] });
    actionRef.current?.reload();
  });
  useEvent('plan_updated', () => {
    timeline.loadTimeline();
  });

  // 查看详情
  const handleViewDetail = useCallback((record: Material) => {
    setSelectedMaterial(record);
    setInspectorVisible(true);
  }, []);

  // 单个材料操作
  const handleSingleOperation = useCallback(
    (record: Material, type: OperationType) => {
      const violations = checkRedLineViolations(record, type, adminOverrideMode);
      if (violations.length > 0) {
        Modal.error({
          title: '工业红线保护',
          width: 600,
          content: (
            <Space direction="vertical" style={{ width: '100%' }} size={16}>
              <RedLineGuard violations={violations} mode="detailed" />
              {!adminOverrideMode && (
                <div
                  style={{
                    padding: '12px',
                    background: '#fff7e6',
                    border: '1px solid #ffd591',
                    borderRadius: '4px',
                  }}
                >
                  <Space>
                    <InfoCircleOutlined style={{ color: '#faad14' }} />
                    <div>
                      <div style={{ fontWeight: 'bold', color: '#faad14' }}>提示</div>
                      <div style={{ fontSize: '12px', color: '#8c8c8c', marginTop: '4px' }}>
                        如需覆盖此保护，请启用"管理员覆盖模式"。
                      </div>
                    </div>
                  </Space>
                </div>
              )}
            </Space>
          ),
        });
        return;
      }

      setSelectedRowKeys([record.material_id]);
      setModalType(type);
      setModalVisible(true);
    },
    [adminOverrideMode]
  );

  // 批量操作
  const handleBatchOperation = useCallback(
    async (type: OperationType) => {
      if (selectedRowKeys.length === 0) {
        message.warning('请先选择材料');
        return;
      }

      if (!adminOverrideMode) {
        // C8修复：使用缓存的材料数据而不是重新调用API
        const selectedMaterials = allMaterials.filter((m: Material) =>
          selectedRowKeys.includes(m.material_id)
        );

        const allViolations = selectedMaterials.flatMap((material: Material) =>
          checkRedLineViolations(material, type, adminOverrideMode)
        );

        if (allViolations.length > 0) {
          Modal.error({
            title: '工业红线保护',
            width: 700,
            content: (
              <Space direction="vertical" style={{ width: '100%' }} size={16}>
                <div>
                  <div style={{ fontWeight: 'bold', marginBottom: '8px' }}>
                    以下材料违反工业红线保护规则:
                  </div>
                  <div style={{ maxHeight: '400px', overflowY: 'auto' }}>
                    <RedLineGuard violations={allViolations} mode="detailed" />
                  </div>
                </div>
                <div
                  style={{
                    padding: '12px',
                    background: '#fff7e6',
                    border: '1px solid #ffd591',
                    borderRadius: '4px',
                  }}
                >
                  <Space>
                    <InfoCircleOutlined style={{ color: '#faad14' }} />
                    <div>
                      <div style={{ fontWeight: 'bold', color: '#faad14' }}>提示</div>
                      <div style={{ fontSize: '12px', color: '#8c8c8c', marginTop: '4px' }}>
                        如需覆盖此保护，请启用"管理员覆盖模式"。
                      </div>
                    </div>
                  </Space>
                </div>
              </Space>
            ),
          });
          return;
        }
      }

      setModalType(type);
      setModalVisible(true);
    },
    [adminOverrideMode, selectedRowKeys, allMaterials]
  );

  // 执行操作
  const executeOperation = useCallback(async () => {
    if (!reason.trim()) {
      message.warning('请输入操作原因');
      return;
    }

    try {
      const materialIds = selectedRowKeys as string[];
      const operator = currentUser || 'admin';
      const validationMode = adminOverrideMode ? 'AutoFix' : undefined;

      if (modalType === 'lock') {
        await materialApi.batchLockMaterials(materialIds, true, operator, reason, validationMode);
        message.success('锁定成功');
      } else if (modalType === 'unlock') {
        await materialApi.batchLockMaterials(
          materialIds,
          false,
          operator,
          reason,
          validationMode
        );
        message.success('解锁成功');
      } else if (modalType === 'urgent') {
        await materialApi.batchSetUrgent(materialIds, true, operator, reason);
        message.success('设置紧急标志成功');
      } else if (modalType === 'clearUrgent') {
        await materialApi.batchSetUrgent(materialIds, false, operator, reason);
        message.success('取消紧急标志成功');
      } else if (modalType === 'forceRelease') {
        await materialApi.batchForceRelease(materialIds, operator, reason, validationMode);
        message.success('强制放行成功');
      }

      setModalVisible(false);
      setReason('');
      setSelectedRowKeys([]);

      // C8修复：刷新React Query缓存以更新材料数据
      queryClient.invalidateQueries({ queryKey: ['materials', 'all'] });
      actionRef.current?.reload();
    } catch (error) {
      // M4修复：使用unknown替代any，通过类型守卫安全访问error属性
      const errorMessage = error instanceof Error ? error.message : String(error);
      message.error(`操作失败: ${errorMessage}`);
    }
  }, [adminOverrideMode, currentUser, modalType, reason, selectedRowKeys, queryClient]);

  // C8修复：基于缓存数据进行前端筛选，而不是每次调用API
  // M4修复：loadMaterials参数使用明确的MaterialSearchParams类型 + 兼容ProTable分页参数
  const loadMaterials = useCallback(
    async (
      params: MaterialSearchParams & {
        pageSize?: number;
        current?: number;
        keyword?: string;
      }
    ) => {
      try {
        // 如果缓存数据加载中或出错，返回空数据
        if (isLoadingMaterials) {
          return { data: [], success: true, total: 0 };
        }
        if (materialsError) {
          const errorMessage =
            materialsError instanceof Error ? materialsError.message : String(materialsError);
          message.error(`加载失败: ${errorMessage}`);
          return { data: [], success: false, total: 0 };
        }

        // 基于缓存数据进行前端筛选
        let filtered = allMaterials;

        if (params.machine_code) {
          filtered = filtered.filter((m: Material) => m.machine_code === params.machine_code);
        }
        if (params.sched_state) {
          const want = normalizeSchedState(params.sched_state);
          filtered = filtered.filter((m: Material) => normalizeSchedState(m.sched_state) === want);
        }
        if (params.urgent_level) {
          filtered = filtered.filter((m: Material) => m.urgent_level === params.urgent_level);
        }
        if (params.manual_urgent_flag !== undefined) {
          const flag = params.manual_urgent_flag === 'true' || params.manual_urgent_flag === true;
          filtered = filtered.filter((m: Material) => m.manual_urgent_flag === flag);
        }
        if (params.lock_flag !== undefined) {
          const flag = params.lock_flag === 'true' || params.lock_flag === true;
          filtered = filtered.filter((m: Material) => m.lock_flag === flag);
        }
        if (params.material_id) {
          filtered = filtered.filter((m: Material) =>
            m.material_id.toLowerCase().includes(params.material_id!.toLowerCase())
          );
        }
        if (params.steel_mark) {
          filtered = filtered.filter((m: Material) =>
            String(m.steel_mark ?? '')
              .toLowerCase()
              .includes(String(params.steel_mark).toLowerCase())
          );
        }

        return { data: filtered, success: true, total: filtered.length };
      } catch (error) {
        // M4修复：使用unknown替代any，通过类型守卫安全访问error属性
        const errorMessage = error instanceof Error ? error.message : String(error);
        message.error(`筛选失败: ${errorMessage}`);
        return { data: [], success: false, total: 0 };
      }
    },
    [allMaterials, isLoadingMaterials, materialsError]
  );

  // 表格列配置
  const columns = useMemo(
    () =>
      createMaterialTableColumns({
        machineOptions: timeline.machineOptions,
        loadMachineOptions: timeline.loadMachineOptions,
        onViewDetail: handleViewDetail,
        onOperation: handleSingleOperation,
      }),
    [timeline.machineOptions, timeline.loadMachineOptions, handleViewDetail, handleSingleOperation]
  );

  return (
    <>
      {/* 产能时间线 */}
      <CapacityTimelineSection
        machineOptions={timeline.machineOptions}
        timelineMachine={timeline.timelineMachine}
        timelineDate={timeline.timelineDate}
        timelineData={timeline.timelineData}
        timelineLoading={timeline.timelineLoading}
        timelineError={timeline.timelineError}
        activeVersionId={timeline.activeVersionId}
        onMachineChange={timeline.setTimelineMachine}
        onDateChange={timeline.setTimelineDate}
        onReload={() => timeline.loadTimeline()}
      />

      {/* 材料表格 */}
      <ProTable<Material>
        columns={columns}
        actionRef={actionRef}
        request={loadMaterials}
        rowKey="material_id"
        search={{
          labelWidth: 'auto',
          defaultCollapsed: false,
          optionRender: (_searchConfig, formProps, dom) => [
            ...dom,
            <Button
              key="reset"
              onClick={() => {
                formProps.form?.resetFields();
                formProps.form?.submit();
              }}
            >
              重置
            </Button>,
          ],
        }}
        pagination={{
          defaultPageSize: 50,
          showSizeChanger: true,
          showQuickJumper: true,
          showTotal: (total) => `共 ${total} 条`,
        }}
        rowSelection={{
          selectedRowKeys,
          onChange: setSelectedRowKeys,
          preserveSelectedRowKeys: true,
        }}
        tableAlertRender={({ selectedRowKeys }) => (
          <Space size={16}>
            <span>已选择 {selectedRowKeys.length} 项</span>
          </Space>
        )}
        tableAlertOptionRender={() => (
          <Space size={8}>
            <Button size="small" icon={<LockOutlined />} onClick={() => handleBatchOperation('lock')}>
              批量锁定
            </Button>
            <Button size="small" icon={<UnlockOutlined />} onClick={() => handleBatchOperation('unlock')}>
              批量解锁
            </Button>
            <Button
              size="small"
              type="primary"
              danger
              icon={<FireOutlined />}
              onClick={() => handleBatchOperation('urgent')}
            >
              批量设为紧急
            </Button>
            <Button size="small" icon={<StopOutlined />} onClick={() => handleBatchOperation('clearUrgent')}>
              批量取消紧急
            </Button>
            <Button
              size="small"
              danger
              icon={<WarningOutlined />}
              onClick={() => handleBatchOperation('forceRelease')}
            >
              批量强制放行
            </Button>
            <Button size="small" onClick={() => setSelectedRowKeys([])}>
              取消选择
            </Button>
          </Space>
        )}
        toolbar={{
          actions: [
            <Button key="reload" icon={<ReloadOutlined />} onClick={() => actionRef.current?.reload()}>
              刷新
            </Button>,
          ],
        }}
        scroll={{ x: 1200 }}
        options={{ density: true, fullScreen: false, reload: true, setting: true }}
        onRow={(record) => ({
          onClick: () => handleViewDetail(record),
          style: { cursor: 'pointer', opacity: record.lock_flag ? 0.7 : 1 },
        })}
      />

      {/* Inspector 侧边栏 */}
      <MaterialInspector
        visible={inspectorVisible}
        material={selectedMaterial}
        onClose={() => setInspectorVisible(false)}
        onLock={() => handleSingleOperation(selectedMaterial!, 'lock')}
        onUnlock={() => handleSingleOperation(selectedMaterial!, 'unlock')}
        onSetUrgent={() => handleSingleOperation(selectedMaterial!, 'urgent')}
        onClearUrgent={() => handleSingleOperation(selectedMaterial!, 'clearUrgent')}
      />

      {/* 操作确认模态框 */}
      <MaterialOperationModal
        open={modalVisible}
        modalType={modalType}
        selectedCount={selectedRowKeys.length}
        reason={reason}
        adminOverrideMode={adminOverrideMode}
        onReasonChange={setReason}
        onOk={executeOperation}
        onCancel={() => {
          setModalVisible(false);
          setReason('');
        }}
      />
    </>
  );
};

export default MaterialManagement;
