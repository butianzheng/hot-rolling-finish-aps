import React, { useMemo } from 'react';
import { Alert, Button, DatePicker, Divider, Input, InputNumber, Modal, Segmented, Select, Space, Table, Tag, Typography } from 'antd';
import type { ColumnsType } from 'antd/es/table';
import { DEFAULT_MOVE_REASON, QUICK_MOVE_REASONS } from '../../pages/workbench/constants';
import type { MoveImpactRow, MoveSeqMode, MoveValidationMode } from '../../pages/workbench/types';
import type { MoveModalState, MoveModalActions } from '../../pages/workbench/hooks/useWorkbenchMoveModal';

/**
 * MoveMaterialsModal Props（Phase 2 重构：使用聚合对象）
 *
 * 原来：25 个散列 props
 * 重构后：5 个 props（2 个聚合对象 + 3 个独立 props）
 */
export interface MoveMaterialsModalProps {
  /** 移动弹窗状态对象（包含 13 个状态字段） */
  state: MoveModalState;
  /** 移动弹窗操作对象（包含 12 个操作方法） */
  actions: MoveModalActions;

  /** 排程数据加载状态（来自 PlanningWorkbench） */
  planItemsLoading: boolean;
  /** 选中的物料 ID 列表 */
  selectedMaterialIds: string[];
  /** 机组选项列表 */
  machineOptions: string[];
}

const MoveMaterialsModal: React.FC<MoveMaterialsModalProps> = ({
  state,
  actions,
  planItemsLoading,
  selectedMaterialIds,
  machineOptions,
}) => {
  // 使用 useMemo 稳定列定义，避免每次渲染都创建新对象和函数
  const impactTableColumns = useMemo<ColumnsType<MoveImpactRow>>(() => [
    { title: '机组', dataIndex: 'machine_code', width: 90 },
    { title: '日期', dataIndex: 'date', width: 120 },
    {
      title: '操作前(t)',
      dataIndex: 'before_t',
      width: 120,
      render: (v) => (
        <span style={{ fontFamily: 'monospace' }}>{Number(v).toFixed(2)}</span>
      ),
    },
    {
      title: '变化(t)',
      dataIndex: 'delta_t',
      width: 110,
      render: (v) => {
        const n = Number(v);
        const color = n > 0 ? 'green' : n < 0 ? 'red' : 'default';
        const label = `${n >= 0 ? '+' : ''}${n.toFixed(2)}`;
        return <Tag color={color}>{label}</Tag>;
      },
    },
    {
      title: '操作后(t)',
      dataIndex: 'after_t',
      width: 120,
      render: (v) => (
        <span style={{ fontFamily: 'monospace' }}>{Number(v).toFixed(2)}</span>
      ),
    },
    {
      title: '目标/限制(t)',
      key: 'cap',
      render: (_, r) => {
        const target = r.target_capacity_t;
        const limit = r.limit_capacity_t;
        if (target == null && limit == null) return <span>-</span>;
        if (limit != null && target != null && Math.abs(limit - target) < 1e-9) {
          return <span style={{ fontFamily: 'monospace' }}>{target.toFixed(2)}</span>;
        }
        return (
          <span style={{ fontFamily: 'monospace' }}>
            {(target ?? 0).toFixed(2)} / {(limit ?? 0).toFixed(2)}
          </span>
        );
      },
    },
    {
      title: '风险',
      key: 'risk',
      width: 110,
      render: (_, r) => {
        const limit = r.limit_capacity_t;
        if (limit == null || limit <= 0) return <Tag>未知</Tag>;
        const pct = (r.after_t / limit) * 100;
        if (pct > 100) return <Tag color="red">超限 {pct.toFixed(0)}%</Tag>;
        if (pct > 90) return <Tag color="orange">偏高 {pct.toFixed(0)}%</Tag>;
        return <Tag color="green">正常 {pct.toFixed(0)}%</Tag>;
      },
    },
  ], []); // 空依赖数组：列定义不依赖任何 props/state

  return (
    <Modal
      title="移动到..."
      open={state.open}
      onCancel={() => actions.setOpen(false)}
      onOk={() => actions.submit()}
      okText="执行移动"
      confirmLoading={state.submitting}
      okButtonProps={{ disabled: selectedMaterialIds.length === 0 || !state.reason.trim() }}
    >
      <Space direction="vertical" style={{ width: '100%' }} size={12}>
        {planItemsLoading ? (
          <Alert type="info" showIcon message="正在加载排程数据，用于校验/自动排队..." />
        ) : state.selectedPlanItemStats.outOfPlan > 0 ? (
          <Alert
            type="warning"
            showIcon
            message={`已选 ${selectedMaterialIds.length} 个，其中 ${state.selectedPlanItemStats.outOfPlan} 个不在当前版本排程中，将跳过`}
          />
        ) : null}

        {state.selectedPlanItemStats.frozenInPlan > 0 ? (
          <Alert
            type="warning"
            showIcon
            message={`检测到 ${state.selectedPlanItemStats.frozenInPlan} 个冻结排程项：STRICT 模式会失败，AUTO_FIX 模式会跳过`}
          />
        ) : null}

        <Space wrap align="center">
          <Button
            size="small"
            onClick={() => actions.recommendTarget()}
            loading={state.recommendLoading}
            disabled={selectedMaterialIds.length === 0 || !state.targetMachine}
          >
            推荐位置（最近可行）
          </Button>
          <Typography.Text type="secondary" style={{ fontSize: 12 }}>
            策略：{state.strategyLabel}
          </Typography.Text>
          {state.recommendSummary ? (
            <Tag color={state.recommendSummary.overLimitCount === 0 ? 'green' : 'orange'}>
              推荐：{state.recommendSummary.machine} {state.recommendSummary.date}{' '}
              {state.recommendSummary.unknownCount > 0
                ? `· 未知容量 ${state.recommendSummary.unknownCount}`
                : `· 超限 ${state.recommendSummary.overLimitCount}`}
            </Tag>
          ) : null}
        </Space>

        <Space wrap>
          <span>目标机组</span>
          <Select
            style={{ minWidth: 180 }}
            value={state.targetMachine}
            onChange={(v) => actions.setTargetMachine(v)}
            options={machineOptions.map((code) => ({ label: code, value: code }))}
            showSearch
            optionFilterProp="label"
            placeholder="请选择机组"
          />
        </Space>

        <Space wrap>
          <span>目标日期</span>
          <DatePicker
            value={state.targetDate}
            onChange={(d) => actions.setTargetDate(d)}
            format="YYYY-MM-DD"
            allowClear={false}
          />
        </Space>

        <Space wrap>
          <span>排队方式</span>
          <Segmented
            value={state.seqMode}
            options={[
              { label: '追加到末尾', value: 'APPEND' },
              { label: '指定起始序号', value: 'START_SEQ' },
            ]}
            onChange={(v) => actions.setSeqMode(v as MoveSeqMode)}
          />
          {state.seqMode === 'START_SEQ' ? (
            <InputNumber
              min={1}
              precision={0}
              value={state.startSeq}
              onChange={(v) => actions.setStartSeq(Number(v || 1))}
              style={{ width: 140 }}
            />
          ) : null}
        </Space>

        <Space wrap>
          <span>校验模式</span>
          <Select
            value={state.validationMode}
            style={{ width: 180 }}
            onChange={(v) => actions.setValidationMode(v as MoveValidationMode)}
            options={[
              { label: 'AUTO_FIX（跳过冻结）', value: 'AUTO_FIX' },
              { label: 'STRICT（遇冻结失败）', value: 'STRICT' },
            ]}
          />
        </Space>

        <Typography.Text type="secondary" style={{ fontSize: 12 }}>
          请输入移动原因（必填，将写入操作日志）
        </Typography.Text>
        <Space wrap align="center">
          <span>快捷原因</span>
          <Select
            style={{ minWidth: 220 }}
            value={
              QUICK_MOVE_REASONS.some((opt) => opt.value === state.reason.trim())
                ? state.reason.trim()
                : undefined
            }
            onChange={(v) => actions.setReason(String(v || DEFAULT_MOVE_REASON))}
            options={QUICK_MOVE_REASONS}
            placeholder="选择一个常用原因"
          />
          <Typography.Text type="secondary" style={{ fontSize: 12 }}>
            可在下方补充说明
          </Typography.Text>
        </Space>
        <Input.TextArea
          value={state.reason}
          onChange={(e) => actions.setReason(e.target.value)}
          rows={3}
          autoSize={{ minRows: 3, maxRows: 6 }}
          placeholder="例如：为满足L3紧急订单，调整到更早日期"
        />

        <Typography.Text type="secondary" style={{ fontSize: 12 }}>
          提示：当前后端的 move_items 不返回"影响预览"，执行后可通过风险概览/对比页观察变化。
        </Typography.Text>

        <Divider style={{ margin: '4px 0' }} />

        <Space direction="vertical" style={{ width: '100%' }} size={8}>
          <Typography.Text strong>影响预览（本地估算）</Typography.Text>
          {!state.impactPreview ? (
            <Typography.Text type="secondary" style={{ fontSize: 12 }}>
              暂无可用预览（请先选择目标机组/日期）。
            </Typography.Text>
          ) : state.impactPreview.rows.length === 0 ? (
            <Alert
              type="info"
              showIcon
              message="未检测到产能变化"
              description="所选物料均在相同机组/日期内（仅可能改变顺序），不会引起产能占用变化。"
            />
          ) : (
            <>
              {state.impactPreview.loading ? (
                <Alert type="info" showIcon message="正在加载产能池，用于评估超限风险..." />
              ) : null}
              {state.impactPreview.overflowRows.length > 0 ? (
                <Alert
                  type="warning"
                  showIcon
                  message={`警告：预计有 ${state.impactPreview.overflowRows.length} 个机组/日期将超出限制产能`}
                  description="可尝试切换到其他日期/机组，或使用 AUTO_FIX 模式（冻结项将跳过）。"
                />
              ) : (
                <Alert type="success" showIcon message="未发现超限风险（按当前估算）" />
              )}
              <Table<MoveImpactRow>
                size="small"
                pagination={false}
                rowKey={(r) => `${r.machine_code}__${r.date}`}
                dataSource={state.impactPreview.rows}
                columns={impactTableColumns}
                scroll={{ y: 240 }}
              />
            </>
          )}
        </Space>
      </Space>
    </Modal>
  );
};

export default React.memo(MoveMaterialsModal);
