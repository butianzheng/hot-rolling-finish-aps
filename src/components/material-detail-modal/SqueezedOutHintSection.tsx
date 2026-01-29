import React from 'react';
import { Alert, Space, Typography } from 'antd';
import type { StrategyDraftDiffItem, MaterialDetailPayload } from '../../types/strategy-draft';
import { buildSqueezedOutHintSections } from '../../utils/strategyDraftFormatters';

const { Text } = Typography;

const renderSqueezedHintLine = (line: string, key: string) => {
  const text = String(line || '');
  const isBlocking = text.startsWith('窗口内不可') || text.includes('红线');
  return isBlocking ? (
    <Text key={key} type="danger" style={{ fontSize: 12 }}>
      {text}
    </Text>
  ) : (
    <Text key={key} style={{ fontSize: 12 }}>
      {text}
    </Text>
  );
};

export interface SqueezedOutHintSectionProps {
  context: StrategyDraftDiffItem;
  data: MaterialDetailPayload;
  windowStart: string;
  windowEnd: string;
}

export const SqueezedOutHintSection: React.FC<SqueezedOutHintSectionProps> = ({
  context,
  data,
  windowStart,
  windowEnd,
}) => {
  if (String(context.change_type) !== 'SQUEEZED_OUT') return null;

  const state = data?.state;
  const sections = state ? buildSqueezedOutHintSections(state, windowStart, windowEnd) : [];

  return (
    <Alert
      type="warning"
      showIcon
      message="挤出提示（基于物料状态，不做臆测）"
      description={
        !sections.length ? (
          <Text>—</Text>
        ) : (
          <Space direction="vertical" size={10} style={{ width: '100%' }}>
            {sections.map((sec, secIdx) => (
              <div key={`${sec.title}-${secIdx}`}>
                <Text type="secondary" style={{ fontSize: 12 }}>
                  {sec.title}
                </Text>
                <Space direction="vertical" size={2} style={{ marginTop: 4 }}>
                  {sec.lines.map((line, idx) => renderSqueezedHintLine(line, `${sec.title}-${idx}`))}
                </Space>
              </div>
            ))}
          </Space>
        )
      }
    />
  );
};
