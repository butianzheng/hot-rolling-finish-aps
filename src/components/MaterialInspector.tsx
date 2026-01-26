// ==========================================
// Inspector 侧边栏组件
// ==========================================
// 显示选中材料的详细信息、引擎推理原因和操作历史
// ==========================================

import React, { useState, useEffect } from 'react';
import { Drawer, Descriptions, Space, Button, Typography, Divider, Empty, Timeline, Alert, Spin } from 'antd';
import { CloseOutlined, InfoCircleOutlined, HistoryOutlined } from '@ant-design/icons';
import { UrgencyTag } from './UrgencyTag';
import { MaterialStatusIcons } from './MaterialStatusIcons';
import { FONT_FAMILIES } from '../theme';
import { dashboardApi } from '../api/tauri';
import { formatDateTime } from '../utils/formatters';

const { Title, Text, Paragraph } = Typography;

interface Material {
  material_id: string;
  machine_code: string;
  weight_t: number;
  steel_mark: string;
  sched_state: string;
  urgent_level: string;
  lock_flag: boolean;
  manual_urgent_flag: boolean;
  is_frozen?: boolean;
  is_mature?: boolean;
  temp_issue?: boolean;
  urgent_reason?: string; // 紧急等级判定原因
  eligibility_reason?: string; // 适温判定原因
  priority_reason?: string; // 优先级排序原因
}

interface ActionLog {
  action_id: string;
  version_id: string;
  action_type: string;
  action_ts: string;
  actor: string;
  payload_json?: any;
  impact_summary_json?: any;
  machine_code?: string | null;
  date_range_start?: string | null;
  date_range_end?: string | null;
  detail?: string | null;
}

interface MaterialInspectorProps {
  visible: boolean;
  material: Material | null;
  onClose: () => void;
  onLock?: (_materialId: string) => void;
  onUnlock?: (_materialId: string) => void;
  onSetUrgent?: (_materialId: string) => void;
}

export const MaterialInspector: React.FC<MaterialInspectorProps> = ({
  visible,
  material,
  onClose,
  onLock,
  onUnlock,
  onSetUrgent,
}) => {
  const [actionLogs, setActionLogs] = useState<ActionLog[]>([]);
  const [loadingLogs, setLoadingLogs] = useState(false);

  // 加载操作历史
  useEffect(() => {
    if (visible && material) {
      loadActionLogs();
    }
  }, [visible, material]);

  const loadActionLogs = async () => {
    if (!material) return;

    setLoadingLogs(true);
    try {
      // 获取最近30天的操作日志
      const endTime = formatDateTime(new Date());
      const startTime = formatDateTime(new Date(Date.now() - 30 * 24 * 60 * 60 * 1000));
      const logs = await dashboardApi.listActionLogs(startTime, endTime);

      // ActionLog 域模型没有专门的 target_id 字段，这里从 detail/payload/impact 里做包含匹配。
      const materialLogs = (Array.isArray(logs) ? (logs as ActionLog[]) : []).filter((log) => {
        const haystacks = [
          log.detail || '',
          log.payload_json ? JSON.stringify(log.payload_json) : '',
          log.impact_summary_json ? JSON.stringify(log.impact_summary_json) : '',
        ].join(' ');
        return haystacks.includes(material.material_id);
      });

      setActionLogs(materialLogs.slice(0, 10)); // 只显示最近10条
    } catch (error) {
      console.error('加载操作历史失败:', error);
      setActionLogs([]);
    } finally {
      setLoadingLogs(false);
    }
  };

  if (!material) {
    return (
      <Drawer
        title="材料详情"
        placement="right"
        width={480}
        onClose={onClose}
        open={visible}
        closeIcon={<CloseOutlined />}
      >
        <Empty description="未选择材料" />
      </Drawer>
    );
  }

  return (
    <Drawer
      title={
        <Space>
          <Text strong>材料详情</Text>
          <MaterialStatusIcons
            lockFlag={material.lock_flag}
            schedState={material.sched_state}
            tempIssue={material.temp_issue || !material.is_mature}
          />
        </Space>
      }
      placement="right"
      width={480}
      onClose={onClose}
      open={visible}
      closeIcon={<CloseOutlined />}
      extra={
        <Space>
          {material.lock_flag ? (
            <Button size="small" onClick={() => onUnlock?.(material.material_id)}>
              解锁
            </Button>
          ) : (
            <Button size="small" onClick={() => onLock?.(material.material_id)}>
              锁定
            </Button>
          )}
          <Button
            size="small"
            type="primary"
            danger
            onClick={() => onSetUrgent?.(material.material_id)}
          >
            设为紧急
          </Button>
        </Space>
      }
    >
      {/* 基本信息 */}
      <Title level={5}>基本信息</Title>
      <Descriptions column={1} size="small" bordered>
        <Descriptions.Item label="材料号">
          <Text
            copyable
            style={{ fontFamily: FONT_FAMILIES.MONOSPACE, fontSize: 13 }}
          >
            {material.material_id}
          </Text>
        </Descriptions.Item>
        <Descriptions.Item label="机组">
          {material.machine_code || '-'}
        </Descriptions.Item>
        <Descriptions.Item label="重量">
          <Text style={{ fontFamily: FONT_FAMILIES.MONOSPACE }}>
            {material.weight_t ? `${material.weight_t.toFixed(2)} 吨` : '-'}
          </Text>
        </Descriptions.Item>
        <Descriptions.Item label="钢种">
          {material.steel_mark || '-'}
        </Descriptions.Item>
      </Descriptions>

      <Divider />

      {/* 状态信息 */}
      <Title level={5}>状态信息</Title>
      <Descriptions column={1} size="small" bordered>
        <Descriptions.Item label="排产状态">
          {material.sched_state}
        </Descriptions.Item>
        <Descriptions.Item label="紧急等级">
          <UrgencyTag level={material.urgent_level} reason={material.urgent_reason} />
        </Descriptions.Item>
        <Descriptions.Item label="人工紧急">
          {material.manual_urgent_flag ? (
            <Text type="danger" strong>是</Text>
          ) : (
            <Text type="secondary">否</Text>
          )}
        </Descriptions.Item>
        <Descriptions.Item label="锁定状态">
          {material.lock_flag ? (
            <Text type="warning" strong>已锁定</Text>
          ) : (
            <Text type="secondary">未锁定</Text>
          )}
        </Descriptions.Item>
        <Descriptions.Item label="冻结区">
          {material.is_frozen ? (
            <Text type="warning" strong>是</Text>
          ) : (
            <Text type="secondary">否</Text>
          )}
        </Descriptions.Item>
        <Descriptions.Item label="适温状态">
          {material.is_mature ? (
            <Text type="success" strong>已适温</Text>
          ) : (
            <Text type="warning" strong>未适温</Text>
          )}
        </Descriptions.Item>
      </Descriptions>

      <Divider />

      {/* 引擎推理原因 */}
      <Title level={5}>
        <Space>
          <InfoCircleOutlined />
          引擎推理原因
        </Space>
      </Title>

      {material.urgent_reason && (
        <Alert
          message="紧急等级判定"
          description={
            <Paragraph style={{ marginBottom: 0, fontSize: 13 }}>
              {material.urgent_reason}
            </Paragraph>
          }
          type="info"
          showIcon
          style={{ marginBottom: 12 }}
        />
      )}

      {material.eligibility_reason && (
        <Alert
          message="适温判定"
          description={
            <Paragraph style={{ marginBottom: 0, fontSize: 13 }}>
              {material.eligibility_reason}
            </Paragraph>
          }
          type="info"
          showIcon
          style={{ marginBottom: 12 }}
        />
      )}

      {material.priority_reason && (
        <Alert
          message="优先级排序"
          description={
            <Paragraph style={{ marginBottom: 0, fontSize: 13 }}>
              {material.priority_reason}
            </Paragraph>
          }
          type="info"
          showIcon
          style={{ marginBottom: 12 }}
        />
      )}

      {!material.urgent_reason && !material.eligibility_reason && !material.priority_reason && (
        <Empty
          description="暂无引擎推理信息"
          image={Empty.PRESENTED_IMAGE_SIMPLE}
        />
      )}

      <Divider />

      {/* 操作历史 */}
      <Title level={5}>
        <Space>
          <HistoryOutlined />
          操作历史
        </Space>
      </Title>

      {loadingLogs ? (
        <div style={{ textAlign: 'center', padding: 24 }}>
          <Spin tip="加载中...">
            <div style={{ minHeight: 80 }} />
          </Spin>
        </div>
      ) : actionLogs.length > 0 ? (
        <Timeline
          mode="left"
          items={actionLogs.map((log) => ({
            children: (
              <div>
                <Text strong>{log.action_type}</Text>
                <br />
                <Text type="secondary" style={{ fontSize: 12 }}>
                  操作人: {log.actor}
                </Text>
                <br />
                <Text type="secondary" style={{ fontSize: 12 }}>
                  时间: {new Date(log.action_ts).toLocaleString('zh-CN')}
                </Text>
                {log.detail && (
                  <>
                    <br />
                    <Text style={{ fontSize: 12 }}>详情: {log.detail}</Text>
                  </>
                )}
              </div>
            ),
          }))}
        />
      ) : (
        <Empty
          description="暂无操作历史"
          image={Empty.PRESENTED_IMAGE_SIMPLE}
        />
      )}
    </Drawer>
  );
};
