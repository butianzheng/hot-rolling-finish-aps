// ==========================================
// 全局 KPI 显示组件
// ==========================================
// 在 Header 中显示关键性能指标
// 使用 React.memo 和 useMemo 优化性能
// ==========================================

import React, { useMemo } from 'react';
import { Space, Badge, Tooltip, Typography } from 'antd';
import {
  WarningOutlined,
  FireOutlined,
  DashboardOutlined,
  InboxOutlined,
  ToolOutlined,
} from '@ant-design/icons';
import { useTheme } from '../theme';
import type { GlobalKPI } from '../types/kpi';

const { Text } = Typography;

interface GlobalKPIDisplayProps {
  kpi: GlobalKPI;
}

export const GlobalKPIDisplay: React.FC<GlobalKPIDisplayProps> = ({ kpi }) => {
  const { theme } = useTheme();

  // 使用 useMemo 缓存颜色计算
  const rollStatusColor = useMemo(() => {
    switch (kpi.rollStatus) {
      case 'critical':
        return '#ff4d4f';
      case 'warning':
        return '#faad14';
      default:
        return '#52c41a';
    }
  }, [kpi.rollStatus]);

  const riskColor = useMemo(() => {
    switch (kpi.riskLevel) {
      case 'critical':
        return '#ff4d4f';
      case 'high':
        return '#faad14';
      case 'medium':
        return '#1677ff';
      default:
        return '#52c41a';
    }
  }, [kpi.riskLevel]);

  const textColor = theme === 'dark' ? 'rgba(255, 255, 255, 0.85)' : 'rgba(255, 255, 255, 0.85)';

  return (
    <Space size={24} style={{ fontSize: 13 }}>
      {/* 风险日期 */}
      {kpi.mostRiskyDate && (
        <Tooltip title={`最高风险日期: ${kpi.mostRiskyDate}`}>
          <Space size={6}>
            <WarningOutlined style={{ color: riskColor, fontSize: 16 }} />
            <Text style={{ color: textColor }}>{kpi.mostRiskyDate}</Text>
          </Space>
        </Tooltip>
      )}

      {/* 紧急订单 */}
      <Tooltip title={`紧急订单: ${kpi.urgentOrdersCount} | 阻塞: ${kpi.blockedUrgentCount}`}>
        <Space size={6}>
          <FireOutlined style={{ color: '#ff4d4f', fontSize: 16 }} />
          <Text style={{ color: textColor }}>
            {kpi.urgentOrdersCount}
            {kpi.blockedUrgentCount > 0 && (
              <span style={{ color: '#ff4d4f' }}> ({kpi.blockedUrgentCount})</span>
            )}
          </Text>
        </Space>
      </Tooltip>

      {/* 产能利用率 */}
      <Tooltip title={`产能利用率: ${kpi.capacityUtilization.toFixed(1)}%`}>
        <Space size={6}>
          <DashboardOutlined style={{ color: '#1677ff', fontSize: 16 }} />
          <Text style={{ color: textColor }}>{kpi.capacityUtilization.toFixed(1)}%</Text>
        </Space>
      </Tooltip>

      {/* 冷库压力 */}
      {kpi.coldStockCount > 0 && (
        <Tooltip title={`冷库材料: ${kpi.coldStockCount} 件`}>
          <Space size={6}>
            <InboxOutlined style={{ color: '#13c2c2', fontSize: 16 }} />
            <Badge count={kpi.coldStockCount} overflowCount={999} style={{ backgroundColor: '#13c2c2' }}>
              <Text style={{ color: textColor, marginRight: 8 }}>冷库</Text>
            </Badge>
          </Space>
        </Tooltip>
      )}

      {/* 轧辊状态 */}
      <Tooltip
        title={`轧辊吨位: ${kpi.rollCampaignProgress}t / ${kpi.rollChangeThreshold}t`}
      >
        <Space size={6}>
          <ToolOutlined style={{ color: rollStatusColor, fontSize: 16 }} />
          <Text style={{ color: textColor }}>
            {kpi.rollCampaignProgress}t
          </Text>
        </Space>
      </Tooltip>
    </Space>
  );
};
