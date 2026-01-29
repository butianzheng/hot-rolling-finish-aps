/**
 * 物料详情弹窗
 * 展示物料的详细信息和操作历史
 */

import React from 'react';
import {
  Alert,
  Button,
  Descriptions,
  Divider,
  Empty,
  Modal,
  Space,
  Spin,
  Table,
  Tag,
  Typography,
} from 'antd';
import type { Dayjs } from 'dayjs';
import type { ActionLogRow, MaterialDetailPayload, StrategyDraftDiffItem } from '../../types/strategy-draft';
import {
  buildSqueezedOutHintSections,
  formatBool,
  formatNumber,
  formatPosition,
  formatText,
  prettyReason,
} from '../../utils/strategyDraftFormatters';

const { Text } = Typography;

export interface MaterialDetailModalProps {
  open: boolean;
  loading: boolean;
  context: StrategyDraftDiffItem | null;
  data: MaterialDetailPayload | null;
  error: string | null;
  logsLoading: boolean;
  logsError: string | null;
  logs: ActionLogRow[];
  range: [Dayjs, Dayjs];
  onClose: () => void;
  onGoWorkbench: (materialId: string) => void;
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

export const MaterialDetailModal: React.FC<MaterialDetailModalProps> = ({
  open,
  loading,
  context,
  data,
  error,
  logsLoading,
  logsError,
  logs,
  range,
  onClose,
  onGoWorkbench,
}) => {
  const windowStart = range[0].format('YYYY-MM-DD');
  const windowEnd = range[1].format('YYYY-MM-DD');

  return (
    <Modal
      title={
        <Space>
          <span>物料详情</span>
          {context ? <Text code>{context.material_id}</Text> : null}
          {context?.change_type ? (
            <Tag
              color={
                String(context.change_type) === 'ADDED'
                  ? 'green'
                  : String(context.change_type) === 'SQUEEZED_OUT'
                    ? 'red'
                    : 'blue'
              }
            >
              {String(context.change_type) === 'ADDED'
                ? '新增'
                : String(context.change_type) === 'SQUEEZED_OUT'
                  ? '挤出'
                  : '移动'}
            </Tag>
          ) : null}
        </Space>
      }
      open={open}
      onCancel={onClose}
      footer={[
        <Button
          key="to-workbench"
          type="primary"
          disabled={!context?.material_id}
          onClick={() => {
            const id = String(context?.material_id ?? '').trim();
            if (id) onGoWorkbench(id);
          }}
        >
          去工作台查看
        </Button>,
        <Button key="close" onClick={onClose}>
          关闭
        </Button>,
      ]}
      width={760}
      destroyOnClose
    >
      {loading ? (
        <div style={{ padding: 24, textAlign: 'center' }}>
          <Spin tip="加载中…" />
        </div>
      ) : error ? (
        <Alert type="error" showIcon message="加载失败" description={error} />
      ) : data ? (
        <Space direction="vertical" style={{ width: '100%' }} size={12}>
          {context ? (
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
                  {String(context.change_type) === 'SQUEEZED_OUT' ? (
                    <>
                      <Text type="secondary">挤出提示（基于物料状态，不做臆测）</Text>
                      {(() => {
                        const state = data?.state;
                        const sections = state ? buildSqueezedOutHintSections(state, windowStart, windowEnd) : [];
                        if (!sections.length) return <Text>—</Text>;
                        return (
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
                        );
                      })()}
                    </>
                  ) : null}
                </Space>
              }
            />
          ) : null}

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
              <Descriptions.Item label="重量(t)">{formatNumber(data.master?.weight_t, 3)}</Descriptions.Item>
              <Descriptions.Item label="交期">{formatText(data.master?.due_date)}</Descriptions.Item>
              <Descriptions.Item label="下道机组">
                {formatText(data.master?.next_machine_code || data.master?.current_machine_code)}
              </Descriptions.Item>
              <Descriptions.Item label="库存天数">
                {formatText(data.state?.stock_age_days ?? data.master?.stock_age_days)}
              </Descriptions.Item>
            </Descriptions>
          </div>

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

            <div style={{ marginTop: 12 }}>
              <Text type="secondary">最近相关操作（30天）</Text>
              <div style={{ marginTop: 6 }}>
                {logsLoading ? (
                  <div style={{ padding: 12, textAlign: 'center' }}>
                    <Spin size="small" tip="加载操作历史…" />
                  </div>
                ) : logsError ? (
                  <Alert type="warning" showIcon message="操作历史加载失败" description={logsError} />
                ) : logs.length ? (
                  <Table
                    size="small"
                    rowKey="action_id"
                    pagination={false}
                    dataSource={logs}
                    columns={[
                      {
                        title: '时间',
                        dataIndex: 'action_ts',
                        width: 160,
                        render: (v: string) => <Text>{String(v || '')}</Text>,
                      },
                      {
                        title: '类型',
                        dataIndex: 'action_type',
                        width: 160,
                        render: (v: string) => <Tag>{String(v || '')}</Tag>,
                      },
                      {
                        title: '操作人',
                        dataIndex: 'actor',
                        width: 120,
                        render: (v: string) => <Text>{String(v || '')}</Text>,
                      },
                      {
                        title: '详情',
                        dataIndex: 'detail',
                        render: (v: string) => (
                          <Text ellipsis={{ tooltip: v ? String(v) : '' }}>{v ? String(v) : '—'}</Text>
                        ),
                      },
                    ]}
                  />
                ) : (
                  <Empty description="暂无相关操作" image={Empty.PRESENTED_IMAGE_SIMPLE} />
                )}
              </div>
            </div>
          </div>
        </Space>
      ) : (
        <Empty description="暂无数据" />
      )}
    </Modal>
  );
};

export default MaterialDetailModal;
