import React from 'react';
import { Space, Spin, Tag, Tooltip, Typography } from 'antd';
import type { StrategyDraftDiffItem, SqueezedHintCache } from '../../types/strategy-draft';
import { buildSqueezedOutHintSections, formatPosition } from '../../utils/strategyDraftFormatters';

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

export interface ChangeTypeRendererProps {
  row: StrategyDraftDiffItem;
  squeezedHintCache: SqueezedHintCache;
  windowStart: string;
  windowEnd: string;
  onEnsureSqueezedHint: (materialId: string) => void;
}

export const ChangeTypeRenderer: React.FC<ChangeTypeRendererProps> = ({
  row,
  squeezedHintCache,
  windowStart,
  windowEnd,
  onEnsureSqueezedHint,
}) => {
  const t = String(row?.change_type || '');
  const color = t === 'ADDED' ? 'green' : t === 'SQUEEZED_OUT' ? 'red' : 'blue';
  const label = t === 'ADDED' ? '新增' : t === 'SQUEEZED_OUT' ? '挤出' : '移动';

  if (t !== 'SQUEEZED_OUT') return <Tag color={color}>{label}</Tag>;

  const id = String(row?.material_id || '').trim();
  const cached = id ? squeezedHintCache[id] : undefined;
  const snapshot = row?.material_state_snapshot ?? null;
  const snapshotSections = snapshot ? buildSqueezedOutHintSections(snapshot, windowStart, windowEnd) : [];

  const titleNode = (
    <Space direction="vertical" size={4}>
      <Text strong style={{ fontSize: 12 }}>
        挤出（窗口内未排入）
      </Text>
      <Text type="secondary" style={{ fontSize: 12 }}>
        基线：{formatPosition(row.from_plan_date, row.from_machine_code, row.from_seq_no)}
      </Text>
      <Text type="secondary" style={{ fontSize: 12 }}>
        提示：若草案把物料排到窗口外，也会显示为"挤出"
      </Text>
      {snapshotSections.length ? (
        <Space direction="vertical" size={6}>
          {snapshotSections
            .filter((sec) => sec.lines && sec.lines.length)
            .slice(0, 3)
            .map((sec, secIdx) => (
              <div key={`${sec.title}-${secIdx}`}>
                <Text type="secondary" style={{ fontSize: 12 }}>
                  {sec.title}
                </Text>
                <Space direction="vertical" size={2}>
                  {sec.lines.slice(0, 4).map((line, idx) => renderSqueezedHintLine(line, `${sec.title}-${idx}`))}
                </Space>
              </div>
            ))}
        </Space>
      ) : cached?.status === 'ready' ? (
        (() => {
          const sections = cached.sections || [];
          let remaining = 10;
          const nodes = sections
            .map((sec) => {
              if (remaining <= 0) return null;
              const lines = sec.lines.slice(0, remaining);
              remaining -= lines.length;
              if (!lines.length) return null;
              return (
                <div key={sec.title}>
                  <Text type="secondary" style={{ fontSize: 12 }}>
                    {sec.title}
                  </Text>
                  <Space direction="vertical" size={2}>
                    {lines.map((line, idx) => renderSqueezedHintLine(line, `${sec.title}-${idx}`))}
                  </Space>
                </div>
              );
            })
            .filter(Boolean);

          if (!nodes.length) {
            return (
              <Text type="secondary" style={{ fontSize: 12 }}>
                暂无可提示的状态
              </Text>
            );
          }
          return <Space direction="vertical" size={6}>{nodes as any}</Space>;
        })()
      ) : cached?.status === 'error' ? (
        <Text type="secondary" style={{ fontSize: 12 }}>
          {cached.error || '加载失败'}
        </Text>
      ) : (
        <Space>
          <Spin size="small" />
          <Text type="secondary" style={{ fontSize: 12 }}>
            加载中…
          </Text>
        </Space>
      )}
      <Text type="secondary" style={{ fontSize: 12 }}>
        点击 Material ID 可查看完整详情
      </Text>
    </Space>
  );

  return (
    <Tooltip
      title={titleNode}
      onOpenChange={(tooltipOpen) => {
        if (!tooltipOpen) return;
        if (!id) return;
        if (snapshot) return;
        onEnsureSqueezedHint(id);
      }}
    >
      <Tag color={color}>{label}</Tag>
    </Tooltip>
  );
};
