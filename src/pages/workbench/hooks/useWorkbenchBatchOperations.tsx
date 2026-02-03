import { useCallback, useEffect, useRef } from 'react';
import type { Dispatch, SetStateAction } from 'react';
import { Alert, Input, Modal, Select, Space, Table, Tag, Typography, message } from 'antd';
import { InfoCircleOutlined } from '@ant-design/icons';
import { materialApi } from '../../../api/tauri';
import { RedLineGuard, createFrozenZoneViolation, createMaturityViolation } from '../../../components/guards/RedLineGuard';
import type { RedLineViolation } from '../../../components/guards/RedLineGuard';
import type { MaterialPoolMaterial } from '../../../components/workbench/MaterialPool';
import type { MaterialOperationType } from '../types';
import { extractForceReleaseViolations } from '../utils';

type IpcImpactSummary = Awaited<ReturnType<typeof materialApi.batchForceRelease>>;

export type { MaterialOperationType } from '../types';

export function useWorkbenchBatchOperations(params: {
  adminOverrideMode: boolean;
  currentUser: string | null;
  materials: MaterialPoolMaterial[];
  setSelectedMaterialIds: Dispatch<SetStateAction<string[]>>;
  bumpRefreshSignal: () => void;
  materialsRefetch: () => void;
}): {
  runMaterialOperation: (materialIds: string[], type: MaterialOperationType) => void;
  runForceReleaseOperation: (materialIds: string[]) => void;
} {
  const {
    adminOverrideMode,
    currentUser,
    materials,
    setSelectedMaterialIds,
    bumpRefreshSignal,
    materialsRefetch,
  } = params;

  const materialsRef = useRef<MaterialPoolMaterial[]>(materials);
  useEffect(() => {
    materialsRef.current = materials;
  }, [materials]);

  const checkRedLineViolations = useCallback(
    (material: MaterialPoolMaterial, operation: MaterialOperationType): RedLineViolation[] => {
      if (adminOverrideMode) return [];
      const violations: RedLineViolation[] = [];

      if (
        material.is_frozen === true &&
        (operation === 'lock' || operation === 'unlock' || operation === 'urgent_on' || operation === 'urgent_off')
      ) {
        violations.push(createFrozenZoneViolation([material.material_id], '该材料位于冻结区，不允许修改状态'));
      }

      if (material.is_mature === false && operation === 'urgent_on') {
        violations.push(createMaturityViolation([material.material_id], 1));
      }

      return violations;
    },
    [adminOverrideMode]
  );

  const showRedLineModal = useCallback(
    (violations: RedLineViolation[]) => {
      Modal.error({
        title: '工业红线保护',
        width: 700,
        content: (
          <Space direction="vertical" style={{ width: '100%' }} size={16}>
            <div style={{ maxHeight: 420, overflow: 'auto' }}>
              <RedLineGuard violations={violations} mode="detailed" />
            </div>
            {!adminOverrideMode && (
              <div
                style={{
                  padding: 12,
                  background: '#fff7e6',
                  border: '1px solid #ffd591',
                  borderRadius: 4,
                }}
              >
                <Space>
                  <InfoCircleOutlined style={{ color: '#faad14' }} />
                  <div>
                    <div style={{ fontWeight: 600, color: '#faad14' }}>提示</div>
                    <div style={{ fontSize: 12, color: '#8c8c8c', marginTop: 4 }}>
                      如需覆盖此保护，请启用“管理员覆盖模式”。
                    </div>
                  </div>
                </Space>
              </div>
            )}
          </Space>
        ),
      });
    },
    [adminOverrideMode]
  );

  const runMaterialOperation = useCallback(
    (materialIds: string[], type: MaterialOperationType) => {
      if (materialIds.length === 0) {
        message.warning('请先选择物料');
        return;
      }

      if (!adminOverrideMode) {
        const set = new Set(materialIds);
        const targets = materialsRef.current.filter((m) => set.has(m.material_id));
        const violations: RedLineViolation[] = [];
        targets.forEach((m) => violations.push(...checkRedLineViolations(m, type)));
        if (violations.length > 0) {
          showRedLineModal(violations);
          return;
        }
      }

      let reason = '';
      Modal.confirm({
        title:
          type === 'lock'
            ? `锁定物料（${materialIds.length}）`
            : type === 'unlock'
              ? `解锁物料（${materialIds.length}）`
              : type === 'urgent_on'
                ? `设为紧急（${materialIds.length}）`
                : `取消紧急（${materialIds.length}）`,
        width: 520,
        content: (
          <Space direction="vertical" style={{ width: '100%' }} size={10}>
            <Typography.Text type="secondary">请输入操作原因（必填）</Typography.Text>
            <Input.TextArea
              rows={3}
              autoSize={{ minRows: 3, maxRows: 6 }}
              onChange={(e) => (reason = e.target.value)}
            />
          </Space>
        ),
        onOk: async () => {
          const trimmed = reason.trim();
          if (!trimmed) {
            message.warning('请输入操作原因');
            return Promise.reject(new Error('reason_required'));
          }

          const operator = currentUser || 'admin';
          const lockMode = adminOverrideMode ? 'AutoFix' : undefined;

          if (type === 'lock') {
            await materialApi.batchLockMaterials(materialIds, true, operator, trimmed, lockMode);
            message.success('锁定成功');
          } else if (type === 'unlock') {
            await materialApi.batchLockMaterials(materialIds, false, operator, trimmed, lockMode);
            message.success('解锁成功');
          } else if (type === 'urgent_on') {
            await materialApi.batchSetUrgent(materialIds, true, operator, trimmed);
            message.success('已设置紧急标志');
          } else {
            await materialApi.batchSetUrgent(materialIds, false, operator, trimmed);
            message.success('已取消紧急标志');
          }

          bumpRefreshSignal();
          materialsRefetch();
        },
      });
    },
    [
      adminOverrideMode,
      bumpRefreshSignal,
      checkRedLineViolations,
      currentUser,
      materialsRefetch,
      showRedLineModal,
    ]
  );

  const runForceReleaseOperation = useCallback(
    (materialIds: string[]) => {
      if (materialIds.length === 0) {
        message.warning('请先选择物料');
        return;
      }

      const set = new Set(materialIds);
      const targets = materialsRef.current.filter((m) => set.has(m.material_id));
      const totalWeight = targets.reduce((sum, m) => sum + Number(m.weight_t || 0), 0);
      const immatureCount = targets.filter((m) => m.is_mature === false).length;
      const unknownMaturityCount = targets.filter((m) => m.is_mature == null).length;
      const frozenCount = targets.filter((m) => m.is_frozen === true).length;

      let reason = '';
      let mode: 'AutoFix' | 'Strict' = 'AutoFix';

      Modal.confirm({
        title: `强制放行（${materialIds.length}）`,
        width: 560,
        content: (
          <Space direction="vertical" style={{ width: '100%' }} size={10}>
            <Alert
              type="info"
              showIcon
              message="说明"
              description="强制放行会将材料状态标记为“强制放行”，并写入操作日志；通常用于人工决策放行未适温材料。"
            />

            <Space wrap>
              <Tag color="blue">可识别 {targets.length}/{materialIds.length}</Tag>
              <Tag color="geekblue">总重 {totalWeight.toFixed(2)}t</Tag>
              {frozenCount > 0 ? <Tag color="purple">冻结区 {frozenCount}</Tag> : null}
              {immatureCount > 0 ? <Tag color="orange">未适温 {immatureCount}</Tag> : null}
              {unknownMaturityCount > 0 ? <Tag>适温未知 {unknownMaturityCount}</Tag> : null}
            </Space>

            {immatureCount > 0 ? (
              <Alert
                type="warning"
                showIcon
                message={`检测到 ${immatureCount} 个未适温材料`}
                description="AUTO_FIX：允许放行并记录警告；STRICT：将阻止操作。"
              />
            ) : null}

            <Space wrap>
              <span>校验模式</span>
              <Select
                defaultValue="AutoFix"
                style={{ width: 220 }}
                onChange={(v) => {
                  mode = v as 'AutoFix' | 'Strict';
                }}
                options={[
                  { value: 'AutoFix', label: 'AUTO_FIX（允许未适温）' },
                  { value: 'Strict', label: 'STRICT（未适温则失败）' },
                ]}
              />
            </Space>

            <Typography.Text type="secondary" style={{ fontSize: 12 }}>
              请输入强制放行原因（必填）
            </Typography.Text>
            <Input.TextArea
              rows={3}
              autoSize={{ minRows: 3, maxRows: 6 }}
              onChange={(e) => {
                reason = e.target.value;
              }}
            />
          </Space>
        ),
        onOk: async () => {
          const trimmed = reason.trim();
          if (!trimmed) {
            message.warning('请输入操作原因');
            return Promise.reject(new Error('reason_required'));
          }

          const operator = currentUser || 'admin';
          const res: IpcImpactSummary = await materialApi.batchForceRelease(materialIds, operator, trimmed, mode);

          message.success(String(res?.message || '强制放行完成'));

          const violations = extractForceReleaseViolations(res?.details);
          if (violations.length > 0) {
            const rows = violations.map((v, idx: number) => ({
              key: `${String(v?.material_id ?? idx)}__${idx}`,
              material_id: String(v?.material_id ?? ''),
              violation_type: String(v?.violation_type ?? ''),
              reason: String(v?.reason ?? ''),
            }));

            Modal.info({
              title: '强制放行警告（未适温材料）',
              width: 820,
              content: (
                <Space direction="vertical" style={{ width: '100%' }} size={12}>
                  <Alert
                    type="warning"
                    showIcon
                    message={`本次包含 ${violations.length} 个未适温材料（AUTO_FIX 模式允许）`}
                  />
                  <Table
                    size="small"
                    pagination={false}
                    dataSource={rows}
                    columns={[
                      { title: '材料', dataIndex: 'material_id', width: 180 },
                      { title: '类型', dataIndex: 'violation_type', width: 140 },
                      { title: '说明', dataIndex: 'reason' },
                    ]}
                    scroll={{ y: 260 }}
                  />
                </Space>
              ),
            });
          }

          setSelectedMaterialIds([]);
          bumpRefreshSignal();
          materialsRefetch();
        },
      });
    },
    [
      bumpRefreshSignal,
      currentUser,
      materialsRefetch,
      setSelectedMaterialIds,
    ]
  );

  return { runMaterialOperation, runForceReleaseOperation };
}
