import React, { useCallback, useMemo, useState } from 'react';
import { Alert, Button, Card, Descriptions, Input, Modal, Space, Tag, Typography, message } from 'antd';
import { ReloadOutlined } from '@ant-design/icons';
import { useQuery } from '@tanstack/react-query';
import { pathRuleApi } from '../../api/tauri';
import { workbenchQueryKeys } from '../../pages/workbench/queryKeys';
import { formatWeight } from '../../utils/formatters';

type RollCycleAnchorDto = {
  version_id: string;
  machine_code: string;
  campaign_no: number;
  cum_weight_t: number;
  anchor_source: string;
  anchor_material_id?: string | null;
  anchor_width_mm?: number | null;
  anchor_thickness_mm?: number | null;
  status: string;
};

const anchorSourceLabels: Record<string, { text: string; color: string }> = {
  FROZEN_LAST: { text: '冻结末端', color: 'purple' },
  LOCKED_LAST: { text: '锁定末端', color: 'geekblue' },
  USER_CONFIRMED_LAST: { text: '已确认末端', color: 'green' },
  SEED_S2: { text: '种子锚点', color: 'gold' },
  NONE: { text: '无', color: 'default' },
};

function fmtNum(v: unknown, digits: number = 1): string {
  const n = typeof v === 'number' ? v : Number(v);
  if (!Number.isFinite(n)) return '-';
  return n.toFixed(digits);
}

export type RollCycleAnchorCardProps = {
  versionId: string | null;
  machineCode: string | null;
  operator: string;
  onAfterReset?: () => void;
};

const RollCycleAnchorCard: React.FC<RollCycleAnchorCardProps> = ({
  versionId,
  machineCode,
  operator,
  onAfterReset,
}) => {
  const [resetOpen, setResetOpen] = useState(false);
  const [resetReason, setResetReason] = useState('');
  const [resetSubmitting, setResetSubmitting] = useState(false);

  const canQuery = !!(versionId && machineCode);

  // 使用 React Query 获取换辊周期锚点数据
  const {
    data,
    isLoading: loading,
    error: loadError,
    refetch,
  } = useQuery({
    queryKey: workbenchQueryKeys.rollCycleAnchor.byMachine(versionId, machineCode),
    enabled: canQuery,
    queryFn: async () => {
      if (!canQuery) return null;
      const raw = await pathRuleApi.getRollCycleAnchor({
        versionId: versionId!,
        machineCode: machineCode!,
      });
      return raw ? (raw as RollCycleAnchorDto) : null;
    },
    staleTime: 30 * 1000, // 30s 缓存
  });

  const errorMessage = loadError ? String((loadError as any)?.message || loadError || '加载锚点失败') : null;

  const handleRefresh = useCallback(() => {
    void refetch();
  }, [refetch]);

  const anchorSourceMeta = useMemo(() => {
    const key = String(data?.anchor_source || 'NONE').toUpperCase();
    return anchorSourceLabels[key] || anchorSourceLabels.NONE;
  }, [data?.anchor_source]);

  const openReset = () => {
    setResetReason('');
    setResetOpen(true);
  };

  const doReset = async () => {
    if (!versionId || !machineCode) return;
    const reason = String(resetReason || '').trim();
    if (!reason) {
      message.warning('请输入换辊/重置原因');
      return;
    }
    setResetSubmitting(true);
    try {
      await pathRuleApi.resetRollCycle({
        versionId,
        machineCode,
        actor: operator || 'system',
        reason,
      });
      message.success('已重置换辊周期（锚点已清空）');
      setResetOpen(false);
      setResetReason('');
      await refetch();
      onAfterReset?.();
    } catch (e: any) {
      console.error('【轧辊锚点卡片】重置轧辊周期失败：', e);
      message.error(String(e?.message || e || '重置失败'));
    } finally {
      setResetSubmitting(false);
    }
  };

  return (
    <>
      <Card
        size="small"
        title="换辊周期锚点"
        extra={
          <Space>
            <Button icon={<ReloadOutlined />} size="small" onClick={handleRefresh} disabled={!canQuery || loading}>
              刷新
            </Button>
            <Button size="small" danger onClick={openReset} disabled={!canQuery || loading}>
              手动换辊/重置
            </Button>
          </Space>
        }
        style={{ width: '100%' }}
      >
        {!versionId ? (
          <Alert type="warning" showIcon message="尚未选择版本" />
        ) : !machineCode ? (
          <Alert type="info" showIcon message="请选择机组后查看锚点" />
        ) : errorMessage ? (
          <Alert type="error" showIcon message={errorMessage} />
        ) : loading ? (
          <Typography.Text type="secondary">加载中...</Typography.Text>
        ) : !data ? (
          <Alert type="info" showIcon message="暂无活跃换辊周期（或尚未初始化）" />
        ) : (
          <Descriptions size="small" column={2} bordered>
            <Descriptions.Item label="机组">
              <Tag color="blue">{data.machine_code}</Tag>
            </Descriptions.Item>
            <Descriptions.Item label="周期号">
              <Typography.Text>{data.campaign_no}</Typography.Text>
            </Descriptions.Item>
            <Descriptions.Item label="累计吨位">
              <Typography.Text>{formatWeight(data.cum_weight_t)}</Typography.Text>
            </Descriptions.Item>
            <Descriptions.Item label="状态">
              <Tag>{String(data.status || '-')}</Tag>
            </Descriptions.Item>
            <Descriptions.Item label="锚点来源">
              <Tag color={anchorSourceMeta.color}>{anchorSourceMeta.text}</Tag>
            </Descriptions.Item>
            <Descriptions.Item label="锚点物料">
              {data.anchor_material_id ? <Typography.Text code>{data.anchor_material_id}</Typography.Text> : <span>-</span>}
            </Descriptions.Item>
            <Descriptions.Item label="锚点宽度">
              <Typography.Text>{data.anchor_width_mm != null ? `${fmtNum(data.anchor_width_mm, 1)} mm` : '-'}</Typography.Text>
            </Descriptions.Item>
            <Descriptions.Item label="锚点厚度">
              <Typography.Text>{data.anchor_thickness_mm != null ? `${fmtNum(data.anchor_thickness_mm, 2)} mm` : '-'}</Typography.Text>
            </Descriptions.Item>
          </Descriptions>
        )}
      </Card>

      <Modal
        open={resetOpen}
        title="手动换辊/重置换辊周期"
        okText="确认重置"
        okButtonProps={{ danger: true, loading: resetSubmitting }}
        cancelText="取消"
        onCancel={() => setResetOpen(false)}
        onOk={doReset}
      >
        <Alert
          type="warning"
          showIcon
          message="该操作会创建新的换辊周期号，并清空锚点与累计吨位"
          description="建议在现场确认已换辊/需要重新开始路径规则周期时使用。"
        />
        <div style={{ marginTop: 12 }}>
          <Input.TextArea
            value={resetReason}
            onChange={(e) => setResetReason(e.target.value)}
            rows={3}
            placeholder="请输入操作原因（必填，例如：实际换辊/工艺要求/异常回退等）"
            maxLength={200}
            showCount
          />
        </div>
      </Modal>
    </>
  );
};

export default RollCycleAnchorCard;
