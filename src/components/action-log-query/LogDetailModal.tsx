/**
 * 操作日志详情模态框
 */

import React, { useMemo } from 'react';
import { Button, Card, Collapse, Descriptions, Modal, Tag, Typography } from 'antd';
import type { ActionLog } from './types';
import { actionTypeLabels } from './types';
import { configDescriptions, configKeyLabels, scopeIdLabels } from '../config-management/types';

export interface LogDetailModalProps {
  open: boolean;
  log: ActionLog | null;
  onClose: () => void;
}

type KV = { label: string; value: React.ReactNode; span?: number };

function isRecord(v: unknown): v is Record<string, any> {
  return typeof v === 'object' && v !== null && !Array.isArray(v);
}

function renderString(text: string) {
  if (text.length <= 80) return <Typography.Text copyable>{text}</Typography.Text>;
  return (
    <Typography.Paragraph style={{ marginBottom: 0 }} copyable={{ text }} ellipsis={{ rows: 3 }}>
      {text}
    </Typography.Paragraph>
  );
}

function renderJson(value: unknown) {
  const text = JSON.stringify(value, null, 2);
  return (
    <pre style={{ maxHeight: 220, overflow: 'auto', fontSize: 12, margin: 0 }}>
      {text}
    </pre>
  );
}

function renderAny(value: unknown): React.ReactNode {
  if (value == null) return '-';
  if (typeof value === 'boolean') return value ? <Tag color="green">是</Tag> : <Tag>否</Tag>;
  if (typeof value === 'number') return <Typography.Text>{String(value)}</Typography.Text>;
  if (typeof value === 'string') return renderString(value);

  if (Array.isArray(value)) {
    const list = value.map((v) => String(v)).filter((s) => s.trim().length > 0);
    const full = list.join(', ');
    const preview = list.slice(0, 20).join(', ');
    const suffix = list.length > 20 ? ` …（共 ${list.length} 项）` : '';
    const display = `${preview}${suffix}`;
    return (
      <Typography.Paragraph style={{ marginBottom: 0 }} copyable={{ text: full }} ellipsis={{ rows: 3 }}>
        {display || `（共 ${value.length} 项）`}
      </Typography.Paragraph>
    );
  }

  if (isRecord(value)) {
    const text = JSON.stringify(value, null, 2);
    return (
      <Typography.Paragraph style={{ marginBottom: 0 }} copyable={{ text }} ellipsis={{ rows: 3 }}>
        {text}
      </Typography.Paragraph>
    );
  }

  return <Typography.Text>{String(value)}</Typography.Text>;
}

function scopeIdToCn(scopeId: string | null | undefined): string {
  if (!scopeId) return '-';
  return scopeIdLabels[scopeId] || scopeId;
}

function buildPayloadKVs(log: ActionLog): { items: KV[]; configList?: Array<{ scope_id: string; key: string; value: string }>; raw?: any } {
  const raw = log.payload_json;
  if (!isRecord(raw)) return { items: [], raw };

  const action = log.action_type;
  const items: KV[] = [];

  const push = (label: string, value: unknown, span?: number) => items.push({ label, value: renderAny(value), span });

  switch (action) {
    case 'UPDATE_CONFIG': {
      const scopeId = String(raw.scope_id ?? '');
      const key = String(raw.key ?? '');
      const label = configKeyLabels[key] || key || '配置项';
      const desc = configDescriptions[key];
      push('作用域', scopeIdToCn(scopeId) === scopeId ? scopeId : `${scopeIdToCn(scopeId)}（${scopeId}）`);
      push('配置项', desc ? `${label}（${key}）\n${desc}` : `${label}（${key}）`, 2);
      push('配置值', raw.value, 2);
      push('原因', raw.reason, 2);
      return { items, raw };
    }
    case 'BATCH_UPDATE_CONFIG': {
      const reason = raw.reason;
      const configs = Array.isArray(raw.configs) ? raw.configs : [];
      push('配置条数', configs.length, undefined);
      push('原因', reason, 2);
      const parsed = configs
        .map((c) => ({
          scope_id: String(c?.scope_id ?? ''),
          key: String(c?.key ?? ''),
          value: String(c?.value ?? ''),
        }))
        .filter((c) => c.scope_id && c.key);
      return { items, configList: parsed, raw };
    }
    case 'RESTORE_CONFIG': {
      push('原因', raw.reason, 2);
      if (typeof raw.snapshot_json === 'string') {
        push('快照JSON', `（已保存，可在下方查看原始JSON）`, 2);
      }
      return { items, raw };
    }
    case 'SAVE_CUSTOM_STRATEGY': {
      push('策略ID', raw.strategy_id);
      push('策略名称', raw.title);
      push('基于预设策略', raw.base_strategy);
      push('保存原因', raw.reason, 2);
      push('存储键', raw.stored_key, 2);
      if (raw.parameters != null) {
        push('参数', raw.parameters, 2);
      }
      return { items, raw };
    }
    case 'LOCK_MATERIALS':
    case 'UNLOCK_MATERIALS': {
      const ids = Array.isArray(raw.material_ids) ? raw.material_ids : [];
      push('材料数', ids.length);
      push('材料ID列表', ids, 2);
      push('锁定标志', raw.lock_flag);
      push('原因', raw.reason, 2);
      return { items, raw };
    }
    case 'FORCE_RELEASE': {
      const ids = Array.isArray(raw.material_ids) ? raw.material_ids : [];
      push('材料数', ids.length);
      push('材料ID列表', ids, 2);
      if (raw.immature_count != null) push('未适温数量', raw.immature_count);
      if (raw.violations != null) push('违规明细', raw.violations, 2);
      push('原因', raw.reason, 2);
      return { items, raw };
    }
    case 'SET_URGENT': {
      const ids = Array.isArray(raw.material_ids) ? raw.material_ids : [];
      push('材料数', ids.length);
      push('材料ID列表', ids, 2);
      push('人工紧急标志', raw.manual_urgent_flag);
      push('原因', raw.reason, 2);
      return { items, raw };
    }
    case 'MOVE_ITEMS': {
      push('成功数', raw.success_count);
      push('失败数', raw.failed_count);
      push('存在违规', raw.has_violations);
      push('移动原因', raw.reason, 2);
      const moved = Array.isArray(raw.moved_materials) ? raw.moved_materials : [];
      push('移动材料列表', moved, 2);
      return { items, raw };
    }
    case 'CREATE_ROLL_CAMPAIGN': {
      push('机组', raw.machine_code);
      push('批次号', raw.campaign_no);
      push('开始日期', raw.start_date);
      push('建议阈值(吨)', raw.suggest_threshold_t);
      push('硬限制(吨)', raw.hard_limit_t);
      push('原因', raw.reason, 2);
      return { items, raw };
    }
    case 'CLOSE_ROLL_CAMPAIGN': {
      push('机组', raw.machine_code);
      push('批次号', raw.campaign_no);
      push('结束日期', raw.end_date);
      push('原因', raw.reason, 2);
      return { items, raw };
    }
    case 'RECALC_FULL': {
      push('基准日期', raw.base_date);
      push('窗口天数', raw.window_days);
      push('冻结起始日期', raw.frozen_from_date);
      push('策略', raw.strategy);
      return { items, raw };
    }
    case 'APPLY_STRATEGY_DRAFT': {
      push('草案ID', raw.draft_id);
      push('基准版本ID', raw.base_version_id);
      push('计划范围', `${raw.plan_date_from ?? '-'} ~ ${raw.plan_date_to ?? '-'}`, 2);
      push('窗口天数', raw.window_days);
      push('策略', raw.strategy);
      push('基础策略', raw.strategy_base);
      push('策略名称', raw.strategy_title_cn, 2);
      if (raw.parameters != null) push('策略参数', raw.parameters, 2);
      return { items, raw };
    }
    case 'ROLLBACK_VERSION': {
      push('方案ID', raw.plan_id);
      push('方案名称', raw.plan_name, 2);
      push('回滚前版本', `V${raw.from_version_no ?? '-'}（${raw.from_version_id ?? '-'}）`, 2);
      push('回滚到版本', `V${raw.to_version_no ?? '-'}（${raw.to_version_id ?? '-'}）`, 2);
      if (raw.restored_config_count != null) push('恢复配置数量', raw.restored_config_count);
      if (raw.config_restore_skipped != null) push('配置恢复跳过原因', raw.config_restore_skipped, 2);
      push('原因', raw.reason, 2);
      return { items, raw };
    }
    case 'MANUAL_REFRESH_DECISION': {
      push('版本ID', raw.version_id);
      push('任务ID', raw.task_id);
      push('是否成功', raw.success);
      push('返回信息', raw.message, 2);
      return { items, raw };
    }
    default: {
      // 通用兜底：把 payload 展开为键值对
      Object.keys(raw).forEach((k) => {
        push(k, raw[k], k === 'reason' || k.endsWith('_json') ? 2 : undefined);
      });
      return { items, raw };
    }
  }
}

function buildImpactKVs(log: ActionLog): { items: KV[]; raw?: any } {
  const raw = log.impact_summary_json;
  if (!isRecord(raw)) return { items: [], raw };

  const items: KV[] = [];
  const push = (label: string, value: unknown, span?: number) => items.push({ label, value: renderAny(value), span });

  // 常见：一键重算/发布草案
  if (
    raw.plan_items_count != null ||
    raw.frozen_items_count != null ||
    raw.mature_count != null ||
    raw.immature_count != null
  ) {
    push('排产项数量', raw.plan_items_count);
    push('冻结项数量', raw.frozen_items_count);
    push('适温材料数', raw.mature_count);
    push('未适温材料数', raw.immature_count);
    if (raw.elapsed_ms != null) push('耗时(ms)', raw.elapsed_ms);
    return { items, raw };
  }

  // 常见：批量材料操作/锁定/放行/紧急
  if (raw.success_count != null || raw.fail_count != null || raw.failed_count != null) {
    push('成功数', raw.success_count);
    push('失败数', raw.fail_count ?? raw.failed_count);
    return { items, raw };
  }

  // 配置相关
  if (raw.updated_count != null) {
    push('更新配置数量', raw.updated_count);
    return { items, raw };
  }
  if (raw.restored_count != null) {
    push('恢复配置数量', raw.restored_count);
    return { items, raw };
  }

  // 自定义策略保存
  if (raw.existed != null) {
    push('是否覆盖已有策略', raw.existed);
    return { items, raw };
  }

  // 通用兜底：展开键值
  Object.keys(raw).forEach((k) => push(k, raw[k]));
  return { items, raw };
}

export const LogDetailModal: React.FC<LogDetailModalProps> = ({
  open,
  log,
  onClose,
}) => {
  const payloadView = useMemo(() => (log ? buildPayloadKVs(log) : { items: [], raw: null }), [log]);
  const impactView = useMemo(() => (log ? buildImpactKVs(log) : { items: [], raw: null }), [log]);

  return (
    <Modal
      title="操作日志详情"
      open={open}
      onCancel={onClose}
      footer={[
        <Button key="close" onClick={onClose}>
          关闭
        </Button>,
      ]}
      width={900}
    >
      {log && (
        <div>
          <Descriptions bordered column={2} size="small">
            <Descriptions.Item label="操作ID" span={2}>
              <Typography.Text copyable>{log.action_id}</Typography.Text>
            </Descriptions.Item>
            <Descriptions.Item label="操作时间" span={2}>
              {log.action_ts}
            </Descriptions.Item>
            <Descriptions.Item label="操作类型">
              <Tag color={actionTypeLabels[log.action_type]?.color || 'default'}>
                {actionTypeLabels[log.action_type]?.text || log.action_type}
              </Tag>
            </Descriptions.Item>
            <Descriptions.Item label="操作人">
              <Typography.Text copyable>{log.actor}</Typography.Text>
            </Descriptions.Item>
            <Descriptions.Item label="版本ID">
              {log.version_id ? <Typography.Text copyable>{log.version_id}</Typography.Text> : '-'}
            </Descriptions.Item>
            <Descriptions.Item label="机组">
              {log.machine_code || '-'}
            </Descriptions.Item>
            <Descriptions.Item label="影响范围" span={2}>
              {log.date_range_start || log.date_range_end
                ? `${log.date_range_start || '-'} ~ ${log.date_range_end || '-'}`
                : '-'}
            </Descriptions.Item>
            <Descriptions.Item label="操作详情" span={2}>
              {log.detail ? renderString(log.detail) : '-'}
            </Descriptions.Item>
          </Descriptions>

          <Card title="操作参数" size="small" style={{ marginTop: 16 }}>
            {payloadView.configList && payloadView.configList.length > 0 ? (
              <div>
                <Descriptions bordered column={2} size="small" style={{ marginBottom: 12 }}>
                  {payloadView.items.map((it) => (
                    <Descriptions.Item key={it.label} label={it.label} span={it.span ?? 1}>
                      {it.value}
                    </Descriptions.Item>
                  ))}
                </Descriptions>
                <Card size="small" title="配置列表（本次写入/更新）">
                  <Descriptions bordered column={1} size="small">
                    {payloadView.configList.map((c, idx) => {
                      const label = configKeyLabels[c.key] || c.key;
                      const desc = configDescriptions[c.key];
                      const scope = scopeIdToCn(c.scope_id) === c.scope_id ? c.scope_id : `${scopeIdToCn(c.scope_id)}（${c.scope_id}）`;
                      return (
                        <Descriptions.Item key={`${c.scope_id}-${c.key}-${idx}`} label={`${label}（${c.key}）`}>
                          <div style={{ marginBottom: 4, color: '#888' }}>{desc || '-'}</div>
                          <div>作用域：{scope}</div>
                          <div>值：{renderString(c.value)}</div>
                        </Descriptions.Item>
                      );
                    })}
                  </Descriptions>
                </Card>
              </div>
            ) : payloadView.items.length ? (
              <Descriptions bordered column={2} size="small">
                {payloadView.items.map((it) => (
                  <Descriptions.Item key={it.label} label={it.label} span={it.span ?? 1}>
                    {it.value}
                  </Descriptions.Item>
                ))}
              </Descriptions>
            ) : (
              <Typography.Text type="secondary">无</Typography.Text>
            )}

            {log.payload_json && (
              <Collapse
                style={{ marginTop: 12 }}
                size="small"
                items={[
                  {
                    key: 'raw_payload',
                    label: '查看原始 Payload JSON',
                    children: renderJson(log.payload_json),
                  },
                ]}
              />
            )}
          </Card>

          <Card title="影响摘要" size="small" style={{ marginTop: 16 }}>
            {impactView.items.length ? (
              <Descriptions bordered column={2} size="small">
                {impactView.items.map((it) => (
                  <Descriptions.Item key={it.label} label={it.label} span={it.span ?? 1}>
                    {it.value}
                  </Descriptions.Item>
                ))}
              </Descriptions>
            ) : (
              <Typography.Text type="secondary">无</Typography.Text>
            )}

            {log.impact_summary_json && (
              <Collapse
                style={{ marginTop: 12 }}
                size="small"
                items={[
                  {
                    key: 'raw_impact',
                    label: '查看原始 Impact Summary JSON',
                    children: renderJson(log.impact_summary_json),
                  },
                ]}
              />
            )}
          </Card>
        </div>
      )}
    </Modal>
  );
};

export default LogDetailModal;
