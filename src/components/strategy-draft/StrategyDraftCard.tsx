/**
 * 策略草案卡片
 * 单个策略的草案展示和操作
 */

import React from 'react';
import { Alert, Button, Card, Space, Tag, Typography } from 'antd';
import type { StrategyDraftSummary, StrategyKey, StrategyPreset } from '../../types/strategy-draft';
import { formatTon } from '../../utils/strategyDraftFormatters';

const { Text } = Typography;

export interface StrategyDraftCardProps {
  strategy: StrategyPreset;
  draft: StrategyDraftSummary | undefined;
  strategyTitleMap: Partial<Record<StrategyKey, string>>;
  activeVersionId: string | null;
  publishingDraftId: string | null;
  onApply: (draft: StrategyDraftSummary) => void;
  onOpenDetail: (draft: StrategyDraftSummary) => void;
}

export const StrategyDraftCard: React.FC<StrategyDraftCardProps> = ({
  strategy,
  draft,
  strategyTitleMap,
  activeVersionId,
  publishingDraftId,
  onApply,
  onOpenDetail,
}) => {
  const hasDraft = Boolean(draft?.draft_id);

  return (
    <Card
      size="small"
      title={
        <Space size={6}>
          <span>{strategy.title}</span>
          {strategy.kind === 'custom' ? <Tag color="gold">自定义</Tag> : null}
          {strategy.kind === 'custom' && strategy.base_strategy ? (
            <Tag color="blue">{strategyTitleMap[strategy.base_strategy] || strategy.base_strategy}</Tag>
          ) : null}
        </Space>
      }
      extra={
        hasDraft ? (
          <Space size={6}>
            <Tag color="green">已生成</Tag>
            {Number(draft?.overflow_days ?? 0) > 0 ? <Tag color="red">超限</Tag> : null}
          </Space>
        ) : (
          <Tag color="default">未生成</Tag>
        )
      }
      style={{ height: '100%' }}
      actions={[
        <Button
          key="select"
          type="primary"
          disabled={!hasDraft}
          loading={publishingDraftId === draft?.draft_id}
          onClick={() => draft && onApply(draft)}
        >
          选择该草案
        </Button>,
      ]}
    >
      <Space direction="vertical" style={{ width: '100%' }} size={8}>
        <Text type="secondary" style={{ fontSize: 12 }}>
          {strategy.description}
        </Text>
        <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: 8 }}>
          <div>
            <Text type="secondary">排产项</Text>
            <div style={{ fontWeight: 600 }}>{hasDraft ? draft?.plan_items_count : '—'}</div>
          </div>
          <div>
            <Text type="secondary">预计产量（吨）</Text>
            <div style={{ fontWeight: 600 }}>{hasDraft ? formatTon(draft?.total_capacity_used_t) : '—'}</div>
          </div>
          <div>
            <Text type="secondary">冻结</Text>
            <div style={{ fontWeight: 600 }}>{hasDraft ? draft?.frozen_items_count : '—'}</div>
          </div>
          <div>
            <Text type="secondary">新排</Text>
            <div style={{ fontWeight: 600 }}>{hasDraft ? draft?.calc_items_count : '—'}</div>
          </div>
          <div>
            <Text type="secondary">成熟/未成熟</Text>
            <div style={{ fontWeight: 600 }}>
              {hasDraft ? `${draft?.mature_count ?? 0}/${draft?.immature_count ?? 0}` : '—'}
            </div>
          </div>
          <div>
            <Text type="secondary">超限机组日</Text>
            <div style={{ fontWeight: 600 }}>{hasDraft ? draft?.overflow_days : '—'}</div>
          </div>
        </div>

        <Text type="secondary" style={{ fontSize: 12 }}>
          变更：移动 {hasDraft ? draft?.moved_count : '—'} · 新增 {hasDraft ? draft?.added_count : '—'} · 挤出{' '}
          {hasDraft ? draft?.squeezed_out_count : '—'}
        </Text>
        {hasDraft ? (
          <Button
            size="small"
            type="link"
            style={{ padding: 0, height: 'auto' }}
            onClick={() => draft && onOpenDetail(draft)}
          >
            查看变更明细
          </Button>
        ) : null}

        {!activeVersionId ? (
          <Alert type="info" showIcon message="未选择基准版本" description="请先激活一个版本再生成草案" />
        ) : hasDraft ? (
          Number(draft?.overflow_days ?? 0) > 0 ? (
            <Alert
              type="warning"
              showIcon
              message="存在产能超限风险"
              description={draft?.message || '可尝试“产能优先”或缩短窗口、调整产能配置后再生成'}
            />
          ) : (
            <Alert
              type="success"
              showIcon
              message="草案可用"
              description={draft?.message || '可点击“选择该草案”生成正式版本'}
            />
          )
        ) : (
          <Alert type="warning" showIcon message="尚未生成" description={'点击“重新计算策略草案”后生成该策略的草案'} />
        )}
      </Space>
    </Card>
  );
};

export default StrategyDraftCard;
