/**
 * KPI 对比卡片
 */

import React from 'react';
import { Alert, Card, Table } from 'antd';

interface KpiCompareCardProps {
  loading?: boolean;
  error?: Error | null;
  rows: Array<{ key: string; metric: string; a: string; b: string; delta: string }>;
}

export const KpiCompareCard: React.FC<KpiCompareCardProps> = ({ loading, error, rows }) => {
  return (
    <Card title="KPI 总览（后端聚合）" size="small">
      {loading ? (
        <Alert type="info" showIcon message="正在计算 KPI…" />
      ) : error ? (
        <Alert
          type="error"
          showIcon
          message="KPI 计算失败"
          description={String((error as any)?.message || error)}
        />
      ) : !rows || rows.length === 0 ? (
        <Alert type="info" showIcon message="暂无 KPI 数据" />
      ) : (
        <Table
          size="small"
          pagination={false}
          rowKey={(r) => String((r as any).key)}
          dataSource={rows}
          columns={[
            { title: '指标', dataIndex: 'metric', width: 180 },
            { title: '版本A', dataIndex: 'a', width: 160 },
            { title: '版本B', dataIndex: 'b', width: 160 },
            { title: 'Δ(B-A)', dataIndex: 'delta' },
          ]}
        />
      )}
    </Card>
  );
};
