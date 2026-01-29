import React, { useMemo, useState } from 'react';
import { Alert, Button, DatePicker, Dropdown, Modal, Select, Space, Typography, message } from 'antd';
import { DownOutlined, ThunderboltOutlined } from '@ant-design/icons';
import dayjs from 'dayjs';
import { planApi } from '../../api/tauri';
import { useGlobalActions } from '../../stores/use-global-store';
import { formatDate } from '../../utils/formatters';

type OptimizeMenuKey =
  | 'preview'
  | 'execute'
  | 'balanced'
  | 'urgent_first'
  | 'capacity_first'
  | 'cold_stock_first';

type OptimizeStrategy = Exclude<OptimizeMenuKey, 'preview' | 'execute'>;

interface OneClickOptimizeMenuProps {
  activeVersionId: string | null;
  operator: string;
  onBeforeExecute?: () => void;
  onAfterExecute?: () => void;
}

const OneClickOptimizeMenu: React.FC<OneClickOptimizeMenuProps> = ({
  activeVersionId,
  operator,
  onBeforeExecute,
  onAfterExecute,
}) => {
  const { setActiveVersion } = useGlobalActions();

  const [previewOpen, setPreviewOpen] = useState(false);
  const [baseDate, setBaseDate] = useState(dayjs());
  const [simulateLoading, setSimulateLoading] = useState(false);
  const [executeLoading, setExecuteLoading] = useState(false);
  const [simulateResult, setSimulateResult] = useState<any | null>(null);
  const [strategy, setStrategy] = useState<OptimizeStrategy>('balanced');
  const [postCreateOpen, setPostCreateOpen] = useState(false);
  const [createdVersionId, setCreatedVersionId] = useState<string | null>(null);
  const [postActionLoading, setPostActionLoading] = useState<'switch' | 'activate' | null>(null);

  const strategyLabel = useMemo(() => {
    if (strategy === 'urgent_first') return '紧急优先';
    if (strategy === 'capacity_first') return '产能优先';
    if (strategy === 'cold_stock_first') return '冷坨消化';
    return '均衡方案';
  }, [strategy]);

  const runSimulate = async () => {
    if (!activeVersionId) {
      message.warning('请先激活一个版本');
      return;
    }

    setSimulateLoading(true);
    try {
      const res = await planApi.simulateRecalc(
        activeVersionId,
        formatDate(baseDate),
        undefined,
        operator,
        strategy
      );
      setSimulateResult(res);
      message.success('试算完成');
    } catch (e: any) {
      console.error('[OneClickOptimizeMenu] simulate failed:', e);
      message.error(e?.message || '试算失败');
      setSimulateResult(null);
    } finally {
      setSimulateLoading(false);
    }
  };

  const runExecute = async () => {
    if (!activeVersionId) {
      message.warning('请先激活一个版本');
      return;
    }

    setExecuteLoading(true);
    onBeforeExecute?.();
    try {
      const res: any = await planApi.recalcFull(
        activeVersionId,
        formatDate(baseDate),
        undefined,
        operator,
        strategy
      );
      message.success(String(res?.message || '重算完成'));
      const newVersionId = String(res?.version_id ?? '').trim();
      if (newVersionId) {
        setCreatedVersionId(newVersionId);
        setPostCreateOpen(true);
      }
      setPreviewOpen(false);
      setSimulateResult(null);
    } catch (e: any) {
      console.error('[OneClickOptimizeMenu] execute failed:', e);
      message.error(e?.message || '重算失败');
    } finally {
      setExecuteLoading(false);
      onAfterExecute?.();
    }
  };

  return (
    <>
      <Dropdown
        disabled={!activeVersionId}
        menu={{
          onClick: ({ key }) => {
            const k = key as OptimizeMenuKey;
            if (k === 'preview') {
              setPreviewOpen(true);
              return;
            }
            if (k === 'execute') {
              setPreviewOpen(true);
              setSimulateResult(null);
              return;
            }
            if (k === 'balanced' || k === 'urgent_first' || k === 'capacity_first' || k === 'cold_stock_first') {
              setStrategy(k);
              setPreviewOpen(true);
              setSimulateResult(null);
              return;
            }
          },
          items: [
            { key: 'preview', icon: <ThunderboltOutlined />, label: '预览（试算，不落库）' },
            { key: 'execute', icon: <ThunderboltOutlined />, label: '执行（重算，落库）' },
            { type: 'divider' },
            { key: 'balanced', label: '均衡方案' },
            { key: 'urgent_first', label: '紧急优先' },
            { key: 'capacity_first', label: '产能优先' },
            { key: 'cold_stock_first', label: '冷坨消化' },
          ],
        }}
      >
        <Button icon={<ThunderboltOutlined />}>
          一键优化 <DownOutlined />
        </Button>
      </Dropdown>

      <Modal
        title={`一键优化 - ${strategyLabel}`}
        open={previewOpen}
        onCancel={() => {
          setPreviewOpen(false);
          setSimulateResult(null);
        }}
        onOk={runExecute}
        okText="执行重算"
        okButtonProps={{ disabled: !activeVersionId }}
        confirmLoading={executeLoading}
      >
        <Space direction="vertical" style={{ width: '100%' }} size={12}>
          <Alert
            type="info"
            showIcon
            message="说明"
            description={`试算（simulate_recalc）仅返回排产数量等摘要，不落库、不写日志；执行重算会落库并触发 plan_updated 事件。当前策略：${strategyLabel}`}
          />

          <Space wrap>
            <span>策略</span>
            <Select
              value={strategy}
              onChange={(v) => {
                setStrategy(v as OptimizeStrategy);
                setSimulateResult(null);
              }}
              style={{ minWidth: 160 }}
              options={[
                { value: 'balanced', label: '均衡方案' },
                { value: 'urgent_first', label: '紧急优先' },
                { value: 'capacity_first', label: '产能优先' },
                { value: 'cold_stock_first', label: '冷坨消化' },
              ]}
            />
          </Space>

          <Space wrap>
            <span>基准日期</span>
            <DatePicker value={baseDate} onChange={(d) => d && setBaseDate(d)} format="YYYY-MM-DD" />
            <Button loading={simulateLoading} onClick={runSimulate}>
              试算预览
            </Button>
          </Space>

          {simulateResult ? (
            <Alert
              type="success"
              showIcon
              message={String(simulateResult?.message || '试算完成')}
              description={
                <Space size={12} wrap>
                  <Typography.Text type="secondary">
                    排产数量: {Number(simulateResult?.plan_items_count ?? 0)}
                  </Typography.Text>
                  <Typography.Text type="secondary">
                    冻结数量: {Number(simulateResult?.frozen_items_count ?? 0)}
                  </Typography.Text>
                </Space>
              }
            />
          ) : (
            <Typography.Text type="secondary" style={{ fontSize: 12 }}>
              点击“试算预览”查看摘要；如需KPI/风险变化对比，需要后端补充影响分析数据结构。
            </Typography.Text>
          )}
        </Space>
      </Modal>

      <Modal
        title="已生成新版本"
        open={postCreateOpen}
        onCancel={() => {
          setPostCreateOpen(false);
          setCreatedVersionId(null);
          setPostActionLoading(null);
        }}
        footer={[
          <Button
            key="later"
            onClick={() => {
              setPostCreateOpen(false);
              setCreatedVersionId(null);
              setPostActionLoading(null);
            }}
          >
            稍后
          </Button>,
          <Button
            key="switch"
            disabled={!createdVersionId}
            loading={postActionLoading === 'switch'}
            onClick={async () => {
              if (!createdVersionId) return;
              setPostActionLoading('switch');
              try {
                setActiveVersion(createdVersionId);
                message.success('已切换到新版本');
                setPostCreateOpen(false);
                setCreatedVersionId(null);
              } finally {
                setPostActionLoading(null);
              }
            }}
          >
            切换到新版本
          </Button>,
          <Button
            key="activate"
            type="primary"
            disabled={!createdVersionId}
            loading={postActionLoading === 'activate'}
            onClick={async () => {
              if (!createdVersionId) return;
              setPostActionLoading('activate');
              try {
                await planApi.activateVersion(createdVersionId, operator || 'admin');
                setActiveVersion(createdVersionId);
                message.success('已激活并切换到新版本');
                setPostCreateOpen(false);
                setCreatedVersionId(null);
              } finally {
                setPostActionLoading(null);
              }
            }}
          >
            切换并激活
          </Button>,
        ]}
      >
        <Space direction="vertical" style={{ width: '100%' }} size={10}>
          <Alert
            type="success"
            showIcon
            message="重算已完成"
            description={
              <Space direction="vertical" size={6}>
                <Typography.Text type="secondary">新版本ID</Typography.Text>
                <Typography.Text code>{createdVersionId || '-'}</Typography.Text>
                <Typography.Text type="secondary" style={{ fontSize: 12 }}>
                  切换后工作台将加载该版本排程；激活会归档当前激活版本。
                </Typography.Text>
              </Space>
            }
          />
        </Space>
      </Modal>
    </>
  );
};

export default React.memo(OneClickOptimizeMenu);
