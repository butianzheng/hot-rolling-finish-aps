import React from 'react';
import { Descriptions, Divider, Empty, Typography } from 'antd';
import type { MaterialDetailPayload } from '../../types/strategy-draft';
import { formatBool, formatText, prettyReason } from '../../utils/strategyDraftFormatters';

const { Text } = Typography;

export interface StateReasonSectionProps {
  data: MaterialDetailPayload;
}

export const StateReasonSection: React.FC<StateReasonSectionProps> = ({ data }) => (
  <div>
    <Text strong>状态/原因</Text>
    <Divider style={{ margin: '8px 0' }} />
    <Descriptions size="small" column={2} bordered>
      <Descriptions.Item label="排产状态">{formatText(data.state?.sched_state)}</Descriptions.Item>
      <Descriptions.Item label="紧急等级">{formatText(data.state?.urgent_level)}</Descriptions.Item>
      <Descriptions.Item label="锁定">{formatBool(data.state?.lock_flag)}</Descriptions.Item>
      <Descriptions.Item label="人工紧急">{formatBool(data.state?.manual_urgent_flag)}</Descriptions.Item>
      <Descriptions.Item label="强制放行">{formatBool(data.state?.force_release_flag)}</Descriptions.Item>
      <Descriptions.Item label="距适温(天)">{formatText(data.state?.ready_in_days)}</Descriptions.Item>
      <Descriptions.Item label="最早可排">{formatText(data.state?.earliest_sched_date)}</Descriptions.Item>
      <Descriptions.Item label="冻结区">{formatBool(data.state?.in_frozen_zone)}</Descriptions.Item>
      <Descriptions.Item label="已排日期">{formatText(data.state?.scheduled_date)}</Descriptions.Item>
      <Descriptions.Item label="已排机组/序号">
        {formatText(
          data.state?.scheduled_machine_code
            ? `${data.state.scheduled_machine_code} / #${data.state?.seq_no ?? '-'}`
            : '—'
        )}
      </Descriptions.Item>
    </Descriptions>

    <div style={{ marginTop: 12 }}>
      <Text type="secondary">紧急原因（urgent_reason）</Text>
      <div style={{ marginTop: 6 }}>
        {prettyReason(data.state?.urgent_reason) ? (
          <pre
            style={{
              margin: 0,
              padding: 12,
              borderRadius: 6,
              border: '1px solid #f0f0f0',
              background: '#fafafa',
              whiteSpace: 'pre-wrap',
              fontSize: 12,
            }}
          >
            {prettyReason(data.state?.urgent_reason)}
          </pre>
        ) : (
          <Empty description="暂无原因信息" image={Empty.PRESENTED_IMAGE_SIMPLE} />
        )}
      </div>
    </div>
  </div>
);
