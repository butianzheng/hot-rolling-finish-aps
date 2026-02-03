/**
 * 冷库压力卡片
 */

import React from 'react';
import { Card, Empty, Space, Statistic, Typography } from 'antd';
import { InboxOutlined } from '@ant-design/icons';
import {
  ScatterChart,
  Scatter,
  XAxis,
  YAxis,
  CartesianGrid,
  Tooltip,
  ResponsiveContainer,
  Cell,
} from 'recharts';
import { FONT_FAMILIES } from '../../theme';
import type { ColdStockBucketRow } from './types';

const { Title } = Typography;

export interface ColdStockCardProps {
  coldStockBuckets: ColdStockBucketRow[];
}

export const ColdStockCard: React.FC<ColdStockCardProps> = ({ coldStockBuckets }) => {
  return (
    <Card hoverable style={{ height: '100%' }}>
      <Space direction="vertical" style={{ width: '100%' }} size={16}>
        <Title level={5} style={{ margin: 0 }}>
          <InboxOutlined style={{ marginRight: 8, color: '#13c2c2' }} />
          冷库压力
        </Title>

        {coldStockBuckets.length > 0 ? (
          <>
            <Statistic
              title="超期材料"
              value={coldStockBuckets.reduce((sum, b) => sum + (b.count || 0), 0)}
              suffix="件"
              valueStyle={{ color: '#13c2c2' }}
            />
            <div style={{ height: 180 }}>
              <ResponsiveContainer width="100%" height="100%">
                <ScatterChart margin={{ top: 10, right: 10, bottom: 10, left: 10 }}>
                  <CartesianGrid strokeDasharray="3 3" />
                  <XAxis
                    type="number"
                    dataKey="avgAgeDays"
                    name="平均库龄"
                    label={{ value: '平均库龄(天)', position: 'insideBottom', offset: -5 }}
                  />
                  <YAxis
                    type="number"
                    dataKey="weightT"
                    name="重量(t)"
                    label={{ value: '重量(t)', angle: -90, position: 'insideLeft' }}
                  />
                  <Tooltip
                    cursor={{ strokeDasharray: '3 3' }}
                    content={({ active, payload }) => {
                      if (active && payload && payload.length) {
                        const data = payload[0].payload as ColdStockBucketRow;
                        return (
                          <div
                            style={{
                              backgroundColor: 'rgba(0, 0, 0, 0.8)',
                              padding: '8px 12px',
                              borderRadius: 4,
                              color: '#fff',
                            }}
                          >
                            <div style={{ fontFamily: FONT_FAMILIES.MONOSPACE }}>{data.machineCode}</div>
                            <div>库龄分桶: {data.ageBin}</div>
                            <div>压力等级: {data.pressureLevel}</div>
                            <div>数量: {data.count}</div>
                            <div>重量: {data.weightT} 吨</div>
                            <div>平均库龄: {data.avgAgeDays} 天</div>
                          </div>
                        );
                      }
                      return null;
                    }}
                  />
                  <Scatter data={coldStockBuckets} fill="#13c2c2">
                    {coldStockBuckets.map((entry, index) => (
                      <Cell
                        key={`cell-${index}`}
                        fill={
                          entry.pressureLevel === 'CRITICAL'
                            ? '#ff4d4f'
                            : entry.pressureLevel === 'HIGH'
                            ? '#faad14'
                            : '#13c2c2'
                        }
                      />
                    ))}
                  </Scatter>
                </ScatterChart>
              </ResponsiveContainer>
            </div>
          </>
        ) : (
          <Empty description="无冷库压力" image={Empty.PRESENTED_IMAGE_SIMPLE} />
        )}
      </Space>
    </Card>
  );
};

export default ColdStockCard;
