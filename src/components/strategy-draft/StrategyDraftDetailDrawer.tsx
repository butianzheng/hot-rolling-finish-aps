/**
 * 变更明细抽屉
 * 展示策略草案的具体变更项
 */

import React from 'react';
import { Alert, Button, Drawer, Input, Segmented, Space, Spin, Table, Tag, Tooltip, Typography } from 'antd';
import type { Dayjs } from 'dayjs';
import type {
  GetStrategyDraftDetailResponse,
  SqueezedHintCache,
  StrategyDraftDiffItem,
  StrategyDraftSummary,
  StrategyKey,
} from '../../types/strategy-draft';
import { buildSqueezedOutHintSections, formatPosition, prettyReason } from '../../utils/strategyDraftFormatters';

const { Text } = Typography;

export interface StrategyDraftDetailDrawerProps {
  open: boolean;
  loading: boolean;
  draft: StrategyDraftSummary | null;
  detailResp: GetStrategyDraftDetailResponse | null;
  detailItems: StrategyDraftDiffItem[];
  filter: 'ALL' | 'MOVED' | 'ADDED' | 'SQUEEZED_OUT';
  search: string;
  strategyTitleMap: Partial<Record<StrategyKey, string>>;
  squeezedHintCache: SqueezedHintCache;
  range: [Dayjs, Dayjs];
  onClose: () => void;
  onFilterChange: (filter: 'ALL' | 'MOVED' | 'ADDED' | 'SQUEEZED_OUT') => void;
  onSearchChange: (search: string) => void;
  onOpenMaterialDetail: (row: StrategyDraftDiffItem) => void;
  onEnsureSqueezedHint: (materialId: string) => void;
}

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

export const StrategyDraftDetailDrawer: React.FC<StrategyDraftDetailDrawerProps> = ({
  open,
  loading,
  draft,
  detailResp,
  detailItems,
  filter,
  search,
  strategyTitleMap,
  squeezedHintCache,
  range,
  onClose,
  onFilterChange,
  onSearchChange,
  onOpenMaterialDetail,
  onEnsureSqueezedHint,
}) => {
  const windowStart = range[0].format('YYYY-MM-DD');
  const windowEnd = range[1].format('YYYY-MM-DD');

  const columns = [
    {
      title: '变更',
      dataIndex: 'change_type',
      width: 90,
      render: (_: any, r: StrategyDraftDiffItem) => {
        const t = String(r?.change_type || '');
        const color = t === 'ADDED' ? 'green' : t === 'SQUEEZED_OUT' ? 'red' : 'blue';
        const label = t === 'ADDED' ? '新增' : t === 'SQUEEZED_OUT' ? '挤出' : '移动';

        if (t !== 'SQUEEZED_OUT') return <Tag color={color}>{label}</Tag>;

        const id = String(r?.material_id || '').trim();
        const cached = id ? squeezedHintCache[id] : undefined;
        const snapshot = r?.material_state_snapshot ?? null;
        const snapshotSections = snapshot ? buildSqueezedOutHintSections(snapshot, windowStart, windowEnd) : [];

        const titleNode = (
          <Space direction="vertical" size={4}>
            <Text strong style={{ fontSize: 12 }}>
              挤出（窗口内未排入）
            </Text>
            <Text type="secondary" style={{ fontSize: 12 }}>
              基线：{formatPosition(r.from_plan_date, r.from_machine_code, r.from_seq_no)}
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
      },
    },
    {
      title: 'Material ID',
      dataIndex: 'material_id',
      width: 180,
      render: (_: any, r: StrategyDraftDiffItem) => (
        <Button
          type="link"
          size="small"
          style={{ padding: 0, height: 'auto' }}
          onClick={() => onOpenMaterialDetail(r)}
        >
          <Text code>{String(r?.material_id || '')}</Text>
        </Button>
      ),
    },
    {
      title: 'From',
      key: 'from',
      width: 240,
      render: (_: any, r: StrategyDraftDiffItem) => (
        <Text>{formatPosition(r.from_plan_date, r.from_machine_code, r.from_seq_no)}</Text>
      ),
    },
    {
      title: 'To',
      key: 'to',
      width: 240,
      render: (_: any, r: StrategyDraftDiffItem) => (
        <Text>{formatPosition(r.to_plan_date, r.to_machine_code, r.to_seq_no)}</Text>
      ),
    },
    {
      title: '草案原因',
      key: 'reason',
      width: 220,
      render: (_: any, r: StrategyDraftDiffItem) => {
        const reason = prettyReason(r.to_assign_reason);
        const display = !reason ? '—' : reason.includes('\n') ? `${reason.split('\n')[0]} …` : reason;
        return (
          <Text
            ellipsis={{
              tooltip: reason ? (
                <pre
                  style={{
                    margin: 0,
                    maxWidth: 640,
                    whiteSpace: 'pre-wrap',
                    fontSize: 12,
                  }}
                >
                  {reason}
                </pre>
              ) : (
                ''
              ),
            }}
            copyable={reason ? { text: reason } : false}
          >
            {display}
          </Text>
        );
      },
    },
  ];

  return (
    <Drawer
      title={
        <Space>
          <span>变更明细</span>
          {draft ? <Tag color="blue">{strategyTitleMap[draft.strategy] || draft.strategy}</Tag> : null}
        </Space>
      }
      open={open}
      onClose={onClose}
      width={860}
      destroyOnClose
    >
      <Space direction="vertical" style={{ width: '100%' }} size={10}>
        {draft ? (
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
        ) : null}

        {detailResp?.diff_items_truncated ? (
          <Alert
            type="warning"
            showIcon
            message="明细已截断"
            description={
              detailResp.message || `仅展示部分变更（${detailResp.diff_items.length}/${detailResp.diff_items_total}）`
            }
          />
        ) : null}

        <Space wrap>
          <Segmented
            value={filter}
            onChange={(v) => onFilterChange(v as any)}
            options={[
              { label: '全部', value: 'ALL' },
              { label: '移动', value: 'MOVED' },
              { label: '新增', value: 'ADDED' },
              { label: '挤出', value: 'SQUEEZED_OUT' },
            ]}
          />
          <Input
            allowClear
            placeholder="搜索 material_id"
            style={{ width: 240 }}
            value={search}
            onChange={(e) => onSearchChange(e.target.value)}
          />
          <Text type="secondary" style={{ fontSize: 12 }}>
            {loading ? '加载中…' : `共 ${detailItems.length} 条`}
          </Text>
        </Space>

        <Table
          size="small"
          rowKey={(r) => `${r.change_type}-${r.material_id}`}
          loading={loading}
          pagination={{ pageSize: 20, showSizeChanger: true }}
          dataSource={detailItems}
          scroll={{ x: 980 }}
          columns={columns}
        />
      </Space>
    </Drawer>
  );
};

export default StrategyDraftDetailDrawer;
