import React from 'react';
import { Alert, Space, Typography } from 'antd';
import type { StrategyDraftDiffItem, MaterialDetailPayload } from '../../types/strategy-draft';
import { buildSqueezedOutHintSections, formatPosition, formatText, prettyReason } from '../../utils/strategyDraftFormatters';

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

export interface DraftInfoSectionProps {
  context: StrategyDraftDiffItem;
  data: MaterialDetailPayload;
  windowStart: string;
  windowEnd: string;
}

export const DraftInfoSection: React.FC<DraftInfoSectionProps> = ({
  context,
  data,
  windowStart,
  windowEnd,
}) => {
  const state = data?.state;
  const sections = state ? buildSqueezedOutHintSections(state, windowStart, windowEnd) : [];

  return (
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
          <Text type="secondary">草案原因</Text>
          {(() => {
            const reason = prettyReason(context.to_assign_reason);
            if (!reason) return <Text>—</Text>;
            if (reason.includes('\n')) {
              return (
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
                  {reason}
                </pre>
              );
            }
            return <Text>{reason}</Text>;
          })()}
          <Text type="secondary">草案快照</Text>
          <Text>
            {formatText(
              [context.to_urgent_level, context.to_sched_state]
                .map((v) => (v == null ? '' : String(v).trim()))
                .filter(Boolean)
                .join(' / ')
            )}
          </Text>
          {String(context.change_type) === 'SQUEEZED_OUT' && (
            <>
              <Text type="secondary">挤出提示（基于物料状态，不做臆测）</Text>
              {!sections.length ? (
                <Text>—</Text>
              ) : (
                <Space direction="vertical" size={10} style={{ width: '100%' }}>
                  {sections.map((sec, secIdx) => (
                    <div key={`${sec.title}-${secIdx}`}>
                      <Text type="secondary" style={{ fontSize: 12 }}>
                        {sec.title}
                      </Text>
                      <Space direction="vertical" size={2} style={{ marginTop: 4 }}>
                        {sec.lines.map((line, idx) =>
                          renderSqueezedHintLine(line, `${sec.title}-${idx}`)
                        )}
                      </Space>
                    </div>
                  ))}
                </Space>
              )}
            </>
          )}
        </Space>
      }
    />
  );
};
