import React from 'react';
import { Typography } from 'antd';
import type { StrategyDraftSummary, GetStrategyDraftDetailResponse } from '../../types/strategy-draft';

const { Text } = Typography;

export interface DraftSummaryInfoProps {
  draft: StrategyDraftSummary | null;
  detailResp: GetStrategyDraftDetailResponse | null;
}

export const DraftSummaryInfo: React.FC<DraftSummaryInfoProps> = ({
  draft,
  detailResp,
}) => {
  if (!draft) return null;

  return (
    <div style={{ display: 'flex', flexWrap: 'wrap', gap: 8, alignItems: 'center' }}>
      <Text type="secondary">草案ID</Text>
      <Text code>{draft.draft_id}</Text>
      <Text type="secondary">基准</Text>
      <Text code>{draft.base_version_id}</Text>
      <Text type="secondary">窗口</Text>
      <Text code>
        {detailResp?.plan_date_from || '—'} ~ {detailResp?.plan_date_to || '—'}
      </Text>
      <Text type="secondary">移动</Text>
      <Text strong>{draft.moved_count}</Text>
      <Text type="secondary">新增</Text>
      <Text strong>{draft.added_count}</Text>
      <Text type="secondary">挤出</Text>
      <Text strong>{draft.squeezed_out_count}</Text>
    </div>
  );
};
