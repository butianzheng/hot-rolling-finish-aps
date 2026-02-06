import React from 'react';
import { Descriptions, Divider, Typography } from 'antd';
import type { MaterialDetailPayload } from '../../types/strategy-draft';
import { formatNumber, formatText } from '../../utils/strategyDraftFormatters';

const { Text } = Typography;

export interface MaterialInfoSectionProps {
  data: MaterialDetailPayload;
}

export const MaterialInfoSection: React.FC<MaterialInfoSectionProps> = ({ data }) => (
  <div>
    <Text strong>物料信息</Text>
    <Divider style={{ margin: '8px 0' }} />
    <Descriptions size="small" column={2} bordered>
      <Descriptions.Item label="材料号">
        <Text code copyable>
          {formatText(data.master?.material_id || data.state?.material_id)}
        </Text>
      </Descriptions.Item>
      <Descriptions.Item label="钢种">{formatText(data.master?.steel_mark)}</Descriptions.Item>
      <Descriptions.Item label="重量（吨）">{formatNumber(data.master?.weight_t, 3)}</Descriptions.Item>
      <Descriptions.Item label="交期">{formatText(data.master?.due_date)}</Descriptions.Item>
      <Descriptions.Item label="下道机组">
        {formatText(data.master?.next_machine_code || data.master?.current_machine_code)}
      </Descriptions.Item>
      <Descriptions.Item label="库存天数">
        {formatText(data.state?.stock_age_days ?? data.master?.stock_age_days)}
      </Descriptions.Item>
    </Descriptions>
  </div>
);
