/**
 * 操作历史区组件
 */

import React from 'react';
import { Empty, Space, Spin, Timeline, Typography } from 'antd';
import { HistoryOutlined } from '@ant-design/icons';
import type { ActionLog } from './types';

const { Title, Text } = Typography;

export interface ActionHistorySectionProps {
  actionLogs: ActionLog[];
  loading: boolean;
}

export const ActionHistorySection: React.FC<ActionHistorySectionProps> = ({ actionLogs, loading }) => {
  return (
    <>
      <Title level={5}>
        <Space>
          <HistoryOutlined />
          操作历史
        </Space>
      </Title>

      {loading ? (
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
    </>
  );
};

export default ActionHistorySection;
