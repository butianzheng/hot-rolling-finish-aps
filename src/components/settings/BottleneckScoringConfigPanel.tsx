import React, { useEffect, useMemo, useState } from 'react';
import { Alert, Button, Card, Form, Input, InputNumber, Space, Tag, Typography, message } from 'antd';
import { ReloadOutlined, SaveOutlined } from '@ant-design/icons';
import { configApi } from '../../api/tauri';
import { useCurrentUser } from '../../stores/use-global-store';

type BottleneckScoringFormValues = {
  d4_capacity_hard_threshold: number;
  d4_capacity_full_threshold: number;
  d4_structure_dev_threshold: number;
  d4_structure_dev_full_multiplier: number;
  d4_structure_small_category_threshold: number;
  d4_structure_violation_full_count: number;
  d4_bottleneck_low_threshold: number;
  d4_bottleneck_medium_threshold: number;
  d4_bottleneck_high_threshold: number;
  d4_bottleneck_critical_threshold: number;
};

const DEFAULT_CONFIG: BottleneckScoringFormValues = {
  d4_capacity_hard_threshold: 0.95,
  d4_capacity_full_threshold: 1.0,
  d4_structure_dev_threshold: 0.1,
  d4_structure_dev_full_multiplier: 2.0,
  d4_structure_small_category_threshold: 0.05,
  d4_structure_violation_full_count: 10,
  d4_bottleneck_low_threshold: 0.3,
  d4_bottleneck_medium_threshold: 0.6,
  d4_bottleneck_high_threshold: 0.9,
  d4_bottleneck_critical_threshold: 0.95,
};

type FieldMeta = {
  key: keyof BottleneckScoringFormValues;
  label: string;
  min?: number;
  max?: number;
  step?: number;
  precision?: number;
  extra: string;
};

const FIELD_GROUPS: Array<{ title: string; fields: FieldMeta[] }> = [
  {
    title: '产能评分',
    fields: [
      {
        key: 'd4_capacity_hard_threshold',
        label: '产能硬阈值',
        min: 0,
        max: 1.2,
        step: 0.01,
        precision: 2,
        extra: 'used/limit 低于该值不计堵塞，仅提醒；建议 0.90~0.98。',
      },
      {
        key: 'd4_capacity_full_threshold',
        label: '产能满载阈值',
        min: 0,
        max: 1.5,
        step: 0.01,
        precision: 2,
        extra: '用于将>硬阈值的利用率线性映射为 0~1 严重度；建议 ≥硬阈值。',
      },
    ],
  },
  {
    title: '结构评分（加权偏差）',
    fields: [
      {
        key: 'd4_structure_dev_threshold',
        label: '结构偏差阈值',
        min: 0,
        max: 1,
        step: 0.01,
        precision: 2,
        extra: '加权偏差起算阈值（0~1），低于该值不计堵塞。',
      },
      {
        key: 'd4_structure_dev_full_multiplier',
        label: '结构满载倍数',
        min: 1,
        max: 10,
        step: 0.1,
        precision: 2,
        extra: '满载阈值=偏差阈值×倍数，达到后严重度=1。',
      },
      {
        key: 'd4_structure_small_category_threshold',
        label: '小类忽略阈值',
        min: 0,
        max: 0.5,
        step: 0.01,
        precision: 2,
        extra: '当某品类目标/实际占比均低于该值时不参与偏差计算。',
      },
      {
        key: 'd4_structure_violation_full_count',
        label: '结构违规满载数',
        min: 1,
        max: 999,
        step: 1,
        precision: 0,
        extra: '结构违规数量达到该值时严重度=1（线性映射）。',
      },
    ],
  },
  {
    title: '等级阈值',
    fields: [
      {
        key: 'd4_bottleneck_low_threshold',
        label: 'LOW 阈值',
        min: 0,
        max: 1,
        step: 0.01,
        precision: 2,
        extra: '低于该值为“无”；LOW/MEDIUM 仅作为提醒。',
      },
      {
        key: 'd4_bottleneck_medium_threshold',
        label: 'MEDIUM 阈值',
        min: 0,
        max: 1,
        step: 0.01,
        precision: 2,
        extra: '介于 LOW 与 HIGH 之间为 MEDIUM（提醒）。',
      },
      {
        key: 'd4_bottleneck_high_threshold',
        label: 'HIGH 阈值',
        min: 0,
        max: 1,
        step: 0.01,
        precision: 2,
        extra: '≥该值视为“堵塞”。',
      },
      {
        key: 'd4_bottleneck_critical_threshold',
        label: 'CRITICAL 阈值',
        min: 0,
        max: 1,
        step: 0.01,
        precision: 2,
        extra: '≥该值视为“严重堵塞”。',
      },
    ],
  },
];

function parseNumber(value: unknown, fallback: number): number {
  const v = Number(value);
  return Number.isFinite(v) ? v : fallback;
}

function normalizeConfig(raw: Record<string, string | undefined>): BottleneckScoringFormValues {
  return {
    d4_capacity_hard_threshold: parseNumber(raw.d4_capacity_hard_threshold, DEFAULT_CONFIG.d4_capacity_hard_threshold),
    d4_capacity_full_threshold: parseNumber(raw.d4_capacity_full_threshold, DEFAULT_CONFIG.d4_capacity_full_threshold),
    d4_structure_dev_threshold: parseNumber(raw.d4_structure_dev_threshold, DEFAULT_CONFIG.d4_structure_dev_threshold),
    d4_structure_dev_full_multiplier: parseNumber(raw.d4_structure_dev_full_multiplier, DEFAULT_CONFIG.d4_structure_dev_full_multiplier),
    d4_structure_small_category_threshold: parseNumber(raw.d4_structure_small_category_threshold, DEFAULT_CONFIG.d4_structure_small_category_threshold),
    d4_structure_violation_full_count: parseNumber(raw.d4_structure_violation_full_count, DEFAULT_CONFIG.d4_structure_violation_full_count),
    d4_bottleneck_low_threshold: parseNumber(raw.d4_bottleneck_low_threshold, DEFAULT_CONFIG.d4_bottleneck_low_threshold),
    d4_bottleneck_medium_threshold: parseNumber(raw.d4_bottleneck_medium_threshold, DEFAULT_CONFIG.d4_bottleneck_medium_threshold),
    d4_bottleneck_high_threshold: parseNumber(raw.d4_bottleneck_high_threshold, DEFAULT_CONFIG.d4_bottleneck_high_threshold),
    d4_bottleneck_critical_threshold: parseNumber(raw.d4_bottleneck_critical_threshold, DEFAULT_CONFIG.d4_bottleneck_critical_threshold),
  };
}

function stableKey(values: BottleneckScoringFormValues): string {
  return JSON.stringify(values);
}

const BottleneckScoringConfigPanel: React.FC = () => {
  const currentUser = useCurrentUser();
  const [form] = Form.useForm<BottleneckScoringFormValues>();

  const [loading, setLoading] = useState(false);
  const [saving, setSaving] = useState(false);
  const [loadError, setLoadError] = useState<string | null>(null);
  const [lastLoaded, setLastLoaded] = useState<BottleneckScoringFormValues | null>(null);
  const [reason, setReason] = useState('');

  const loadConfig = async () => {
    setLoading(true);
    setLoadError(null);
    try {
      const items = await configApi.listConfigs();
      const map: Record<string, string | undefined> = {};
      items
        .filter((item) => item.scope_id === 'global')
        .forEach((item) => {
          map[item.key] = item.value;
        });
      const cfg = normalizeConfig(map);
      setLastLoaded(cfg);
      form.setFieldsValue(cfg);
      setReason('');
    } catch (e: any) {
      console.error('[BottleneckScoringConfigPanel] loadConfig failed:', e);
      setLoadError(String(e?.message || e || '加载配置失败'));
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    void loadConfig();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  const currentValues = Form.useWatch([], form) as Partial<BottleneckScoringFormValues> | undefined;

  const isDirty = useMemo(() => {
    if (!lastLoaded) return false;
    const cur = { ...DEFAULT_CONFIG, ...(currentValues || {}) } as BottleneckScoringFormValues;
    return stableKey(lastLoaded) !== stableKey(cur);
  }, [currentValues, lastLoaded]);

  const validate = (values: BottleneckScoringFormValues): string | null => {
    if (values.d4_capacity_hard_threshold < 0 || values.d4_capacity_hard_threshold > 1.5) {
      return '产能硬阈值需在 0~1.5 之间';
    }
    if (values.d4_capacity_full_threshold < values.d4_capacity_hard_threshold) {
      return '产能满载阈值不能小于硬阈值';
    }
    if (values.d4_structure_dev_threshold < 0 || values.d4_structure_dev_threshold > 1) {
      return '结构偏差阈值需在 0~1 之间';
    }
    if (values.d4_structure_dev_full_multiplier <= 1) {
      return '结构满载倍数需大于 1';
    }
    if (values.d4_structure_small_category_threshold < 0 || values.d4_structure_small_category_threshold > 0.5) {
      return '小类忽略阈值需在 0~0.5 之间';
    }
    if (values.d4_structure_violation_full_count < 1) {
      return '结构违规满载数需大于等于 1';
    }
    const { d4_bottleneck_low_threshold, d4_bottleneck_medium_threshold, d4_bottleneck_high_threshold, d4_bottleneck_critical_threshold } = values;
    if (d4_bottleneck_low_threshold < 0 || d4_bottleneck_low_threshold > 1) return 'LOW 阈值需在 0~1 之间';
    if (d4_bottleneck_medium_threshold < 0 || d4_bottleneck_medium_threshold > 1) return 'MEDIUM 阈值需在 0~1 之间';
    if (d4_bottleneck_high_threshold < 0 || d4_bottleneck_high_threshold > 1) return 'HIGH 阈值需在 0~1 之间';
    if (d4_bottleneck_critical_threshold < 0 || d4_bottleneck_critical_threshold > 1) return 'CRITICAL 阈值需在 0~1 之间';
    if (!(d4_bottleneck_low_threshold <= d4_bottleneck_medium_threshold
      && d4_bottleneck_medium_threshold <= d4_bottleneck_high_threshold
      && d4_bottleneck_high_threshold <= d4_bottleneck_critical_threshold)) {
      return '等级阈值需满足 LOW ≤ MEDIUM ≤ HIGH ≤ CRITICAL';
    }
    return null;
  };

  const saveConfig = async () => {
    const operator = String(currentUser || 'system').trim() || 'system';
    const cleanReason = String(reason || '').trim();
    if (!cleanReason) {
      message.warning('请输入修改原因');
      return;
    }

    const raw = form.getFieldsValue(true);
    const values = { ...DEFAULT_CONFIG, ...(raw as any) } as BottleneckScoringFormValues;
    const validationError = validate(values);
    if (validationError) {
      message.warning(validationError);
      return;
    }

    setSaving(true);
    try {
      const configs = Object.entries(values).map(([key, value]) => ({
        scope_id: 'global',
        key,
        value: String(value),
      }));
      await configApi.batchUpdateConfigs(configs, operator, cleanReason);
      message.success('堵塞评分参数已保存');
      await loadConfig();
    } catch (e: any) {
      console.error('[BottleneckScoringConfigPanel] saveConfig failed:', e);
      message.error(String(e?.message || e || '保存失败'));
    } finally {
      setSaving(false);
    }
  };

  return (
    <Card
      title={
        <Space size={8} wrap>
          <span>D4 堵塞评分参数</span>
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
      <Space direction="vertical" size={12} style={{ width: '100%' }}>
        <Alert
          type="info"
          showIcon
          message="评分口径"
          description={
            <div>
              <div>分数 = max(产能严重度, 结构偏差严重度, 结构违规严重度) × 100。</div>
              <div>HIGH/CRITICAL 视为“堵塞”，LOW/MEDIUM 为“提醒”。</div>
            </div>
          }
        />
        <Alert
          type="warning"
          showIcon
          message="生效方式"
          description='修改后需执行“刷新/重算”以写入读模型（建议：设置中心保存后进行刷新），D4 实时回退口径会直接读取最新配置。'
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

        <Form form={form} layout="vertical">
          {FIELD_GROUPS.map((group) => (
            <Card key={group.title} size="small" title={group.title}>
              <Space direction="vertical" style={{ width: '100%' }} size={8}>
                {group.fields.map((field) => (
                  <Form.Item
                    key={field.key}
                    label={field.label}
                    name={field.key}
                    extra={field.extra}
                  >
                    <InputNumber
                      min={field.min}
                      max={field.max}
                      step={field.step}
                      precision={field.precision}
                      style={{ width: 240 }}
                    />
                  </Form.Item>
                ))}
              </Space>
            </Card>
          ))}
        </Form>

        <Card size="small" title="变更说明">
          <Space direction="vertical" size={8}>
            <Typography.Text>
              - 产能口径基于 used/limit；仅当超过硬阈值才进入堵塞评分。
            </Typography.Text>
            <Typography.Text>
              - 结构偏差使用加权偏差（Σ|实际-目标|/2），并忽略占比过小的品类。
            </Typography.Text>
            <Typography.Text>
              - 结构违规严重度按“违规数/满载数”线性映射。
            </Typography.Text>
          </Space>
        </Card>

        <div>
          <label>修改原因（必填）：</label>
          <Input.TextArea
            style={{ marginTop: 8 }}
            placeholder="请输入修改原因"
            value={reason}
            onChange={(e) => setReason(e.target.value)}
            rows={3}
          />
        </div>
      </Space>
    </Card>
  );
};

export default BottleneckScoringConfigPanel;
