import React, { useEffect, useMemo, useState } from 'react';
import { Alert, Button, Card, Divider, Form, Input, InputNumber, Select, Space, Switch, Tag, Typography, message } from 'antd';
import { ReloadOutlined, SaveOutlined } from '@ant-design/icons';
import { pathRuleApi } from '../../api/tauri';
import { useCurrentUser } from '../../stores/use-global-store';

type PathRuleConfigFormValues = {
  enabled: boolean;
  width_tolerance_mm: number;
  thickness_tolerance_mm: number;
  override_allowed_urgency_levels: string[];
  seed_s2_percentile: number;
  seed_s2_small_sample_threshold: number;
};

const DEFAULT_CONFIG: PathRuleConfigFormValues = {
  enabled: true,
  width_tolerance_mm: 50.0,
  thickness_tolerance_mm: 1.0,
  override_allowed_urgency_levels: ['L2', 'L3'],
  seed_s2_percentile: 0.95,
  seed_s2_small_sample_threshold: 10,
};

function normalizeUrgencyLevels(raw: unknown): string[] {
  const list = Array.isArray(raw) ? raw : [];
  const out: string[] = [];
  list.forEach((v) => {
    const s = String(v || '').trim().toUpperCase();
    if (!/^L[0-3]$/.test(s)) return;
    if (!out.includes(s)) out.push(s);
  });
  out.sort();
  return out;
}

function normalizeConfig(raw: any): PathRuleConfigFormValues {
  const enabled = !!raw?.enabled;
  const w = Number(raw?.width_tolerance_mm);
  const t = Number(raw?.thickness_tolerance_mm);
  const p = Number(raw?.seed_s2_percentile);
  const n = Number(raw?.seed_s2_small_sample_threshold);
  return {
    enabled,
    width_tolerance_mm: Number.isFinite(w) ? w : DEFAULT_CONFIG.width_tolerance_mm,
    thickness_tolerance_mm: Number.isFinite(t) ? t : DEFAULT_CONFIG.thickness_tolerance_mm,
    override_allowed_urgency_levels: normalizeUrgencyLevels(raw?.override_allowed_urgency_levels),
    seed_s2_percentile: Number.isFinite(p) ? p : DEFAULT_CONFIG.seed_s2_percentile,
    seed_s2_small_sample_threshold: Number.isFinite(n) ? Math.trunc(n) : DEFAULT_CONFIG.seed_s2_small_sample_threshold,
  };
}

function normalizeForCompare(v: PathRuleConfigFormValues): PathRuleConfigFormValues {
  return {
    enabled: !!v.enabled,
    width_tolerance_mm: Number.isFinite(v.width_tolerance_mm) ? v.width_tolerance_mm : DEFAULT_CONFIG.width_tolerance_mm,
    thickness_tolerance_mm: Number.isFinite(v.thickness_tolerance_mm) ? v.thickness_tolerance_mm : DEFAULT_CONFIG.thickness_tolerance_mm,
    override_allowed_urgency_levels: normalizeUrgencyLevels(v.override_allowed_urgency_levels),
    seed_s2_percentile: Number.isFinite(v.seed_s2_percentile) ? v.seed_s2_percentile : DEFAULT_CONFIG.seed_s2_percentile,
    seed_s2_small_sample_threshold: Number.isFinite(v.seed_s2_small_sample_threshold)
      ? Math.trunc(v.seed_s2_small_sample_threshold)
      : DEFAULT_CONFIG.seed_s2_small_sample_threshold,
  };
}

function stableKey(v: PathRuleConfigFormValues): string {
  const n = normalizeForCompare(v);
  return JSON.stringify({
    ...n,
    override_allowed_urgency_levels: [...n.override_allowed_urgency_levels].sort(),
  });
}

const URGENCY_OPTIONS = ['L0', 'L1', 'L2', 'L3'].map((v) => ({ label: v, value: v }));

const PathRuleConfigPanel: React.FC = () => {
  const currentUser = useCurrentUser();
  const [form] = Form.useForm<PathRuleConfigFormValues>();

  const [loading, setLoading] = useState(false);
  const [loadError, setLoadError] = useState<string | null>(null);
  const [saving, setSaving] = useState(false);
  const [lastLoaded, setLastLoaded] = useState<PathRuleConfigFormValues | null>(null);
  const [reason, setReason] = useState('');

  const loadConfig = async () => {
    setLoading(true);
    setLoadError(null);
    try {
      const raw = await pathRuleApi.getPathRuleConfig();
      const cfg = normalizeConfig(raw);
      setLastLoaded(cfg);
      form.setFieldsValue(cfg);
      setReason('');
    } catch (e: any) {
      console.error('[PathRuleConfigPanel] loadConfig failed:', e);
      setLoadError(String(e?.message || e || '加载配置失败'));
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    void loadConfig();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  const currentValues = Form.useWatch([], form) as Partial<PathRuleConfigFormValues> | undefined;

  const isDirty = useMemo(() => {
    if (!lastLoaded) return false;
    const cur = normalizeForCompare({ ...DEFAULT_CONFIG, ...(currentValues || {}) } as PathRuleConfigFormValues);
    return stableKey(lastLoaded) !== stableKey(cur);
  }, [currentValues, lastLoaded]);

  const saveConfig = async () => {
    const operator = String(currentUser || 'system').trim() || 'system';
    const cleanReason = String(reason || '').trim();
    if (!cleanReason) {
      message.warning('请输入修改原因');
      return;
    }

    const raw = form.getFieldsValue(true);
    const cfg = normalizeForCompare({ ...DEFAULT_CONFIG, ...(raw as any) });

    if (cfg.width_tolerance_mm < 0) {
      message.warning('宽度容差不能为负数');
      return;
    }
    if (cfg.thickness_tolerance_mm < 0) {
      message.warning('厚度容差不能为负数');
      return;
    }
    if (!(cfg.seed_s2_percentile >= 0 && cfg.seed_s2_percentile <= 1)) {
      message.warning('S2 分位数必须在 0~1 之间');
      return;
    }
    if (!Number.isInteger(cfg.seed_s2_small_sample_threshold) || cfg.seed_s2_small_sample_threshold < 1) {
      message.warning('S2 小样本阈值必须为 >= 1 的整数');
      return;
    }

    setSaving(true);
    try {
      await pathRuleApi.updatePathRuleConfig({
        config: cfg,
        operator,
        reason: cleanReason,
      });
      message.success('路径规则配置已保存');
      await loadConfig();
    } catch (e: any) {
      console.error('[PathRuleConfigPanel] saveConfig failed:', e);
      message.error(String(e?.message || e || '保存失败'));
    } finally {
      setSaving(false);
    }
  };

  return (
    <Card
      title={
        <Space size={8} wrap>
          <span>宽厚路径规则（v0.6）</span>
          {isDirty ? <Tag color="gold">已修改</Tag> : <Tag>未修改</Tag>}
        </Space>
      }
      extra={
        <Space>
          <Button icon={<ReloadOutlined />} onClick={loadConfig} loading={loading} disabled={saving}>
            重新加载
          </Button>
          <Button
            icon={<SaveOutlined />}
            type="primary"
            onClick={saveConfig}
            loading={saving}
            disabled={!isDirty || loading}
          >
            保存配置
          </Button>
        </Space>
      }
    >
      <Space direction="vertical" style={{ width: '100%' }} size={12}>
        <Alert
          type="info"
          showIcon
          message="说明"
          description="该配置影响“由宽到窄、由厚到薄”的排产门控；保存后建议执行“一键优化/重算”生成新版本以生效。"
        />

        {loadError ? (
          <Alert
            type="error"
            showIcon
            message="加载失败"
            description={loadError}
            action={
              <Button size="small" onClick={loadConfig}>
                重试
              </Button>
            }
          />
        ) : null}

        <Form<PathRuleConfigFormValues>
          form={form}
          layout="vertical"
          initialValues={DEFAULT_CONFIG}
          disabled={loading || saving}
        >
          <Typography.Title level={5} style={{ marginTop: 0, marginBottom: 0 }}>
            路径门控
          </Typography.Title>
          <Divider style={{ margin: '8px 0' }} />

          <Form.Item label="启用宽厚路径规则" name="enabled" valuePropName="checked">
            <Switch />
          </Form.Item>

          <Space wrap>
            <Form.Item
              label="宽度容差 (mm)"
              name="width_tolerance_mm"
              style={{ marginBottom: 0, minWidth: 260 }}
              extra="候选宽度允许比锚点更宽的最大偏差。"
            >
              <InputNumber min={0} step={1} precision={1} style={{ width: '100%' }} />
            </Form.Item>

            <Form.Item
              label="厚度容差 (mm)"
              name="thickness_tolerance_mm"
              style={{ marginBottom: 0, minWidth: 260 }}
              extra="候选厚度允许比锚点更厚的最大偏差。"
            >
              <InputNumber min={0} step={0.1} precision={2} style={{ width: '100%' }} />
            </Form.Item>
          </Space>

          <Form.Item
            label="允许人工突破的紧急等级"
            name="override_allowed_urgency_levels"
            extra="仅当候选紧急等级命中此列表时，违规将进入“待确认”；否则为硬拦截。"
          >
            <Select mode="multiple" allowClear style={{ width: 320 }} options={URGENCY_OPTIONS} />
          </Form.Item>

          <Typography.Title level={5} style={{ marginTop: 8, marginBottom: 0 }}>
            S2 种子策略
          </Typography.Title>
          <Divider style={{ margin: '8px 0' }} />

          <Space wrap>
            <Form.Item
              label="S2 分位数 (0~1)"
              name="seed_s2_percentile"
              style={{ marginBottom: 0, minWidth: 260 }}
              extra="无冻结/锁定/已确认锚点时，用候选分布生成初始锚点。"
            >
              <InputNumber min={0} max={1} step={0.01} precision={2} style={{ width: '100%' }} />
            </Form.Item>

            <Form.Item
              label="S2 小样本阈值"
              name="seed_s2_small_sample_threshold"
              style={{ marginBottom: 0, minWidth: 260 }}
              extra="候选样本不足时回退到保守规则。"
            >
              <InputNumber min={1} step={1} precision={0} style={{ width: '100%' }} />
            </Form.Item>
          </Space>

          <Divider />

          <Form.Item
            label="保存原因（必填）"
            extra="会写入操作日志，便于审计与追溯。"
          >
            <Input.TextArea
              value={reason}
              onChange={(e) => setReason(e.target.value)}
              placeholder="例如：现场工艺调整/客户临期订单/策略试验等"
              rows={3}
              maxLength={200}
              showCount
            />
          </Form.Item>

          <Space>
            <Button
              onClick={() => {
                form.setFieldsValue(DEFAULT_CONFIG);
              }}
              disabled={loading || saving}
            >
              恢复默认（未保存）
            </Button>
          </Space>
        </Form>
      </Space>
    </Card>
  );
};

export default PathRuleConfigPanel;

