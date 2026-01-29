import React from 'react';
import { Alert, Space, Typography } from 'antd';
import type { StrategyDraftDiffItem } from '../../types/strategy-draft';
import { formatPosition } from '../../utils/strategyDraftFormatters';

const { Text } = Typography;

export interface ChangePositionAlertProps {
  context: StrategyDraftDiffItem;
}

export const ChangePositionAlert: React.FC<ChangePositionAlertProps> = ({ context }) => (
  <Alert
    type="info"
    showIcon
    message="本次变更位置"
    description={
      <Space direction="vertical" size={4}>
        <Text type="secondary">From</Text>
        <Text>
          {formatPosition(context.from_plan_date, context.from_machine_code, context.from_seq_no)}
        </Text>
        <Text type="secondary">To</Text>
        <Text>
          {String(context.change_type) === 'SQUEEZED_OUT'
            ? '未安排（挤出）'
            : formatPosition(context.to_plan_date, context.to_machine_code, context.to_seq_no)}
        </Text>
      </Space>
    }
  />
);
