import React, { useEffect, useMemo, useState } from 'react';
import { Alert, Button, Card, Dropdown, Input, Modal, Select, Space, Table, Tag, Typography } from 'antd';
import { DownOutlined } from '@ant-design/icons';
import { normalizeSchedState } from '../../utils/schedState';
import type { MaterialPoolMaterial } from './MaterialPool';
import type { ConditionLockFilter } from '../../pages/workbench/types';

type BatchApplyKey = 'lock' | 'unlock' | 'urgent_on' | 'urgent_off' | 'force_release';

export interface ConditionalSelectModalProps {
  open: boolean;
  onClose: () => void;
  defaultMachine: string | null;
  machineOptions: string[];
  materials: MaterialPoolMaterial[];
  selectedMaterialIds: string[];
  onSelectedMaterialIdsChange: (ids: string[]) => void;
  onMaterialOperation: (materialIds: string[], type: 'lock' | 'unlock' | 'urgent_on' | 'urgent_off') => void;
  onForceReleaseOperation: (materialIds: string[]) => void;
}

const ConditionalSelectModal: React.FC<ConditionalSelectModalProps> = ({
  open,
  onClose,
  defaultMachine,
  machineOptions,
  materials,
  selectedMaterialIds,
  onSelectedMaterialIdsChange,
  onMaterialOperation,
  onForceReleaseOperation,
}) => {
  const [conditionMachine, setConditionMachine] = useState<string>('all');
  const [conditionSchedState, setConditionSchedState] = useState<string>('all');
  const [conditionUrgency, setConditionUrgency] = useState<string>('all');
  const [conditionLock, setConditionLock] = useState<ConditionLockFilter>('ALL');
  const [conditionSearch, setConditionSearch] = useState<string>('');

  useEffect(() => {
    if (!open) return;
    setConditionMachine(defaultMachine || 'all');
    setConditionSchedState('all');
    setConditionUrgency('all');
    setConditionLock('ALL');
    setConditionSearch('');
  }, [defaultMachine, open]);

  const conditionalMatches = useMemo(() => {
    let list = materials;
    if (conditionMachine && conditionMachine !== 'all') {
      list = list.filter((m) => String(m.machine_code || '') === conditionMachine);
    }
    if (conditionSchedState && conditionSchedState !== 'all') {
      const want = normalizeSchedState(conditionSchedState);
      list = list.filter((m) => normalizeSchedState(m.sched_state) === want);
    }
    if (conditionUrgency && conditionUrgency !== 'all') {
      list = list.filter((m) => String(m.urgent_level || '') === conditionUrgency);
    }
    if (conditionLock === 'LOCKED') {
      list = list.filter((m) => !!m.lock_flag);
    } else if (conditionLock === 'UNLOCKED') {
      list = list.filter((m) => !m.lock_flag);
    }
    const q = conditionSearch.trim().toLowerCase();
    if (q) {
      list = list.filter((m) => {
        const id = String(m.material_id || '').toLowerCase();
        const steel = String(m.steel_mark || '').toLowerCase();
        return id.includes(q) || steel.includes(q);
      });
    }
    return [...list].sort((a, b) => String(a.material_id || '').localeCompare(String(b.material_id || '')));
  }, [conditionLock, conditionMachine, conditionSchedState, conditionSearch, conditionUrgency, materials]);

  const conditionalSummary = useMemo(() => {
    const count = conditionalMatches.length;
    const weight = conditionalMatches.reduce((sum, m) => sum + Number(m.weight_t || 0), 0);
    return { count, weight };
  }, [conditionalMatches]);

  return (
    <Modal
      title="按条件选中..."
      open={open}
      onCancel={onClose}
      width={820}
      footer={[
        <Button key="close" onClick={onClose}>
          关闭
        </Button>,
        <Dropdown
          key="apply"
          disabled={conditionalMatches.length === 0}
          menu={{
            onClick: ({ key }) => {
              const ids = conditionalMatches.map((m) => m.material_id);
              onClose();
              const k = key as BatchApplyKey;
              if (k === 'lock') return onMaterialOperation(ids, 'lock');
              if (k === 'unlock') return onMaterialOperation(ids, 'unlock');
              if (k === 'urgent_on') return onMaterialOperation(ids, 'urgent_on');
              if (k === 'urgent_off') return onMaterialOperation(ids, 'urgent_off');
              if (k === 'force_release') return onForceReleaseOperation(ids);
            },
            items: [
              { key: 'lock', label: `锁定命中（${conditionalMatches.length}）` },
              { key: 'unlock', label: `解锁命中（${conditionalMatches.length}）` },
              { type: 'divider' },
              { key: 'urgent_on', label: `设为紧急（${conditionalMatches.length}）` },
              { key: 'urgent_off', label: `取消紧急（${conditionalMatches.length}）` },
              { type: 'divider' },
              { key: 'force_release', label: `强制放行（${conditionalMatches.length}）` },
            ],
          }}
        >
          <Button disabled={conditionalMatches.length === 0}>
            对命中执行 <DownOutlined />
          </Button>
        </Dropdown>,
        <Button
          key="replace"
          type="primary"
          onClick={() => {
            onSelectedMaterialIdsChange(conditionalMatches.map((m) => m.material_id));
            onClose();
          }}
          disabled={conditionalMatches.length === 0}
        >
          替换为这些物料
        </Button>,
        <Button
          key="merge"
          onClick={() => {
            const next = new Set(selectedMaterialIds);
            conditionalMatches.forEach((m) => next.add(m.material_id));
            onSelectedMaterialIdsChange(Array.from(next));
            onClose();
          }}
          disabled={conditionalMatches.length === 0}
        >
          叠加到当前选择
        </Button>,
      ]}
    >
      <Space direction="vertical" style={{ width: '100%' }} size={12}>
        <Alert
          type="info"
          showIcon
          message="说明"
          description="先按条件筛选出物料集合，再“替换/叠加”为当前选择，随后可用工具栏/状态栏执行批量操作。"
        />
        <Space wrap>
          <span>机组</span>
          <Select
            value={conditionMachine}
            onChange={(v) => setConditionMachine(v)}
            style={{ width: 160 }}
            options={[{ label: '全部', value: 'all' }, ...machineOptions.map((m) => ({ label: m, value: m }))]}
            showSearch
            optionFilterProp="label"
          />
          <span>状态</span>
          <Select
            value={conditionSchedState}
            onChange={(v) => setConditionSchedState(v)}
            style={{ width: 160 }}
            options={[
              { label: '全部', value: 'all' },
              { label: '未成熟/冷料', value: 'PENDING_MATURE' },
              { label: '待排/就绪', value: 'READY' },
              { label: '强制放行', value: 'FORCE_RELEASE' },
              { label: '已锁定', value: 'LOCKED' },
              { label: '已排产', value: 'SCHEDULED' },
              { label: '阻断', value: 'BLOCKED' },
            ]}
          />
          <span>紧急度</span>
          <Select
            value={conditionUrgency}
            onChange={(v) => setConditionUrgency(v)}
            style={{ width: 140 }}
            options={[
              { label: '全部', value: 'all' },
              { label: 'L3', value: 'L3' },
              { label: 'L2', value: 'L2' },
              { label: 'L1', value: 'L1' },
              { label: 'L0', value: 'L0' },
            ]}
          />
          <span>锁定</span>
          <Select
            value={conditionLock}
            onChange={(v) => setConditionLock(v as ConditionLockFilter)}
            style={{ width: 140 }}
            options={[
              { label: '全部', value: 'ALL' },
              { label: '已锁', value: 'LOCKED' },
              { label: '未锁', value: 'UNLOCKED' },
            ]}
          />
          <Input.Search
            placeholder="搜索材料号/钢种"
            allowClear
            value={conditionSearch}
            onChange={(e) => setConditionSearch(e.target.value)}
            style={{ width: 220 }}
          />
        </Space>

        <Card size="small">
          <Space wrap align="center" style={{ width: '100%', justifyContent: 'space-between' }}>
            <Space wrap>
              <Typography.Text>命中 {conditionalSummary.count} 条</Typography.Text>
              <Typography.Text type="secondary">总重 {conditionalSummary.weight.toFixed(3)}t</Typography.Text>
            </Space>
            {conditionalSummary.count > 2000 ? <Tag color="orange">命中较多，建议增加筛选条件</Tag> : null}
          </Space>
        </Card>

        <Table<MaterialPoolMaterial>
          size="small"
          rowKey={(r) => r.material_id}
          pagination={{ pageSize: 8, showSizeChanger: true }}
          dataSource={conditionalMatches}
          columns={[
            {
              title: '材料号',
              dataIndex: 'material_id',
              width: 160,
              render: (v) => <span style={{ fontFamily: 'monospace' }}>{String(v)}</span>,
            },
            { title: '机组', dataIndex: 'machine_code', width: 90 },
            { title: '状态', dataIndex: 'sched_state', width: 120 },
            { title: '紧急度', dataIndex: 'urgent_level', width: 90, render: (v) => <Tag>{String(v || 'L0')}</Tag> },
            {
              title: '重量(t)',
              dataIndex: 'weight_t',
              width: 110,
              render: (v) => <span style={{ fontFamily: 'monospace' }}>{Number(v || 0).toFixed(2)}</span>,
            },
            { title: '钢种', dataIndex: 'steel_mark', ellipsis: true },
          ]}
        />
      </Space>
    </Modal>
  );
};

export default React.memo(ConditionalSelectModal);

