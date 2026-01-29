import React from 'react';
import { Alert } from 'antd';
import type { GetStrategyDraftDetailResponse } from '../../types/strategy-draft';

export interface TruncationAlertProps {
  detailResp: GetStrategyDraftDetailResponse | null;
}

export const TruncationAlert: React.FC<TruncationAlertProps> = ({ detailResp }) => {
  if (!detailResp?.diff_items_truncated) return null;

  return (
    <Alert
      type="warning"
      showIcon
      message="明细已截断"
      description={
        detailResp.message || `仅展示部分变更（${detailResp.diff_items.length}/${detailResp.diff_items_total}）`
      }
    />
  );
};
