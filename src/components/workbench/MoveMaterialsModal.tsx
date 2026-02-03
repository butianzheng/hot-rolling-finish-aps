import React from 'react';
import type { Dayjs } from 'dayjs';
import { Alert, Button, DatePicker, Divider, Input, InputNumber, Modal, Segmented, Select, Space, Table, Tag, Typography } from 'antd';
import { DEFAULT_MOVE_REASON, QUICK_MOVE_REASONS } from '../../pages/workbench/constants';
import type { MoveImpactPreview, MoveImpactRow, MoveRecommendSummary, MoveSeqMode, MoveValidationMode, SelectedPlanItemStats } from '../../pages/workbench/types';

export interface MoveMaterialsModalProps {
  open: boolean;
  onClose: () => void;
  onSubmit: () => Promise<void>;
  submitting: boolean;

  planItemsLoading: boolean;
  selectedMaterialIds: string[];
  machineOptions: string[];

  selectedPlanItemStats: SelectedPlanItemStats;

  moveTargetMachine: string | null;
  setMoveTargetMachine: (v: string | null) => void;
  moveTargetDate: Dayjs | null;
  setMoveTargetDate: (v: Dayjs | null) => void;

  moveSeqMode: MoveSeqMode;
  setMoveSeqMode: (v: MoveSeqMode) => void;
  moveStartSeq: number;
  setMoveStartSeq: (v: number) => void;

  moveValidationMode: MoveValidationMode;
  setMoveValidationMode: (v: MoveValidationMode) => void;

  moveReason: string;
  setMoveReason: (v: string) => void;

  recommendMoveTarget: () => void;
  moveRecommendLoading: boolean;
  moveRecommendSummary: MoveRecommendSummary | null;
  strategyLabel: string;

  moveImpactPreview: MoveImpactPreview | null;
}

const MoveMaterialsModal: React.FC<MoveMaterialsModalProps> = ({
  open,
  onClose,
  onSubmit,
  submitting,
  planItemsLoading,
  selectedMaterialIds,
  machineOptions,
  selectedPlanItemStats,
  moveTargetMachine,
  setMoveTargetMachine,
  moveTargetDate,
  setMoveTargetDate,
  moveSeqMode,
  setMoveSeqMode,
  moveStartSeq,
  setMoveStartSeq,
  moveValidationMode,
  setMoveValidationMode,
  moveReason,
  setMoveReason,
  recommendMoveTarget,
  moveRecommendLoading,
  moveRecommendSummary,
  strategyLabel,
  moveImpactPreview,
}) => {
  return (
    <Modal
      title="移动到..."
      open={open}
      onCancel={onClose}
      onOk={onSubmit}
      okText="执行移动"
      confirmLoading={submitting}
      okButtonProps={{ disabled: selectedMaterialIds.length === 0 || !moveReason.trim() }}
    >
      <Space direction="vertical" style={{ width: '100%' }} size={12}>
        {planItemsLoading ? (
          <Alert type="info" showIcon message="正在加载排程数据，用于校验/自动排队..." />
        ) : selectedPlanItemStats.outOfPlan > 0 ? (
          <Alert
            type="warning"
            showIcon
            message={`已选 ${selectedMaterialIds.length} 个，其中 ${selectedPlanItemStats.outOfPlan} 个不在当前版本排程中，将跳过`}
          />
        ) : null}

        {selectedPlanItemStats.frozenInPlan > 0 ? (
          <Alert
            type="warning"
            showIcon
            message={`检测到 ${selectedPlanItemStats.frozenInPlan} 个冻结排程项：STRICT 模式会失败，AUTO_FIX 模式会跳过`}
          />
        ) : null}

        <Space wrap align="center">
          <Button
            size="small"
            onClick={() => recommendMoveTarget()}
            loading={moveRecommendLoading}
            disabled={selectedMaterialIds.length === 0 || !moveTargetMachine}
          >
            推荐位置（最近可行）
          </Button>
          <Typography.Text type="secondary" style={{ fontSize: 12 }}>
            策略：{strategyLabel}
          </Typography.Text>
          {moveRecommendSummary ? (
            <Tag color={moveRecommendSummary.overLimitCount === 0 ? 'green' : 'orange'}>
              推荐：{moveRecommendSummary.machine} {moveRecommendSummary.date}{' '}
              {moveRecommendSummary.unknownCount > 0
                ? `· 未知容量 ${moveRecommendSummary.unknownCount}`
                : `· 超限 ${moveRecommendSummary.overLimitCount}`}
            </Tag>
          ) : null}
        </Space>

        <Space wrap>
          <span>目标机组</span>
          <Select
            style={{ minWidth: 180 }}
            value={moveTargetMachine}
            onChange={(v) => setMoveTargetMachine(v)}
            options={machineOptions.map((code) => ({ label: code, value: code }))}
            showSearch
            optionFilterProp="label"
            placeholder="请选择机组"
          />
        </Space>

        <Space wrap>
          <span>目标日期</span>
          <DatePicker
            value={moveTargetDate}
            onChange={(d) => setMoveTargetDate(d)}
            format="YYYY-MM-DD"
            allowClear={false}
          />
        </Space>

        <Space wrap>
          <span>排队方式</span>
          <Segmented
            value={moveSeqMode}
            options={[
              { label: '追加到末尾', value: 'APPEND' },
              { label: '指定起始序号', value: 'START_SEQ' },
            ]}
            onChange={(v) => setMoveSeqMode(v as MoveSeqMode)}
          />
          {moveSeqMode === 'START_SEQ' ? (
            <InputNumber
              min={1}
              precision={0}
              value={moveStartSeq}
              onChange={(v) => setMoveStartSeq(Number(v || 1))}
              style={{ width: 140 }}
            />
          ) : null}
        </Space>

        <Space wrap>
          <span>校验模式</span>
          <Select
            value={moveValidationMode}
            style={{ width: 180 }}
            onChange={(v) => setMoveValidationMode(v as MoveValidationMode)}
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
              QUICK_MOVE_REASONS.some((opt) => opt.value === moveReason.trim())
                ? moveReason.trim()
                : undefined
            }
            onChange={(v) => setMoveReason(String(v || DEFAULT_MOVE_REASON))}
            options={QUICK_MOVE_REASONS}
            placeholder="选择一个常用原因"
          />
          <Typography.Text type="secondary" style={{ fontSize: 12 }}>
            可在下方补充说明
          </Typography.Text>
        </Space>
        <Input.TextArea
          value={moveReason}
          onChange={(e) => setMoveReason(e.target.value)}
          rows={3}
          autoSize={{ minRows: 3, maxRows: 6 }}
          placeholder="例如：为满足L3紧急订单，调整到更早日期"
        />

        <Typography.Text type="secondary" style={{ fontSize: 12 }}>
          提示：当前后端的 move_items 不返回“影响预览”，执行后可通过风险概览/对比页观察变化。
        </Typography.Text>

        <Divider style={{ margin: '4px 0' }} />

        <Space direction="vertical" style={{ width: '100%' }} size={8}>
          <Typography.Text strong>影响预览（本地估算）</Typography.Text>
          {!moveImpactPreview ? (
            <Typography.Text type="secondary" style={{ fontSize: 12 }}>
              暂无可用预览（请先选择目标机组/日期）。
            </Typography.Text>
          ) : moveImpactPreview.rows.length === 0 ? (
            <Alert
              type="info"
              showIcon
              message="未检测到产能变化"
              description="所选物料均在相同机组/日期内（仅可能改变顺序），不会引起产能占用变化。"
            />
          ) : (
            <>
              {moveImpactPreview.loading ? (
                <Alert type="info" showIcon message="正在加载产能池，用于评估超限风险..." />
              ) : null}
              {moveImpactPreview.overflowRows.length > 0 ? (
                <Alert
                  type="warning"
                  showIcon
                  message={`警告：预计有 ${moveImpactPreview.overflowRows.length} 个机组/日期将超出限制产能`}
                  description="可尝试切换到其他日期/机组，或使用 AUTO_FIX 模式（冻结项将跳过）。"
                />
              ) : (
                <Alert type="success" showIcon message="未发现超限风险（按当前估算）" />
              )}
              <Table<MoveImpactRow>
                size="small"
                pagination={false}
                rowKey={(r) => `${r.machine_code}__${r.date}`}
                dataSource={moveImpactPreview.rows}
                columns={[
                  { title: '机组', dataIndex: 'machine_code', width: 90 },
                  { title: '日期', dataIndex: 'date', width: 120 },
                  {
                    title: '操作前(t)',
                    dataIndex: 'before_t',
                    width: 120,
                    render: (v) => (
                      <span style={{ fontFamily: 'monospace' }}>{Number(v).toFixed(1)}</span>
                    ),
                  },
                  {
                    title: '变化(t)',
                    dataIndex: 'delta_t',
                    width: 110,
                    render: (v) => {
                      const n = Number(v);
                      const color = n > 0 ? 'green' : n < 0 ? 'red' : 'default';
                      const label = `${n >= 0 ? '+' : ''}${n.toFixed(1)}`;
                      return <Tag color={color}>{label}</Tag>;
                    },
                  },
                  {
                    title: '操作后(t)',
                    dataIndex: 'after_t',
                    width: 120,
                    render: (v) => (
                      <span style={{ fontFamily: 'monospace' }}>{Number(v).toFixed(1)}</span>
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
                        return <span style={{ fontFamily: 'monospace' }}>{target.toFixed(0)}</span>;
                      }
                      return (
                        <span style={{ fontFamily: 'monospace' }}>
                          {(target ?? 0).toFixed(0)} / {(limit ?? 0).toFixed(0)}
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
                ]}
                scroll={{ y: 240 }}
              />
            </>
          )}
        </Space>
      </Space>
    </Modal>
  );
};

export default MoveMaterialsModal;
