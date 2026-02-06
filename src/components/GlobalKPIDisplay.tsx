// ==========================================
// 全局 KPI 显示组件
// ==========================================
// 在 Header 中显示关键性能指标
// ==========================================

import React from 'react';
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

// 状态颜色常量
const ROLL_STATUS_COLORS: Record<string, string> = {
  critical: '#ff4d4f',
  warning: '#faad14',
  healthy: '#52c41a',
};

const RISK_LEVEL_COLORS: Record<string, string> = {
  critical: '#ff4d4f',
  high: '#faad14',
  medium: '#1677ff',
  low: '#52c41a',
};

interface GlobalKPIDisplayProps {
  kpi: GlobalKPI;
}

export const GlobalKPIDisplay: React.FC<GlobalKPIDisplayProps> = ({ kpi }) => {
  const { theme } = useTheme();

  // 使用常量对象替代 useMemo
  const rollStatusColor = (kpi.rollStatus && ROLL_STATUS_COLORS[kpi.rollStatus]) || ROLL_STATUS_COLORS.healthy;
  const riskColor = (kpi.riskLevel && RISK_LEVEL_COLORS[kpi.riskLevel]) || RISK_LEVEL_COLORS.low;

  const textColor = theme === 'dark' ? 'rgba(255, 255, 255, 0.85)' : 'rgba(255, 255, 255, 0.85)';
  const urgentL3Total = kpi.urgentBreakdown?.L3?.total ?? null;
  const urgentL3Blocked = kpi.urgentBreakdown?.L3?.blocked ?? null;
  const urgentL2Total = kpi.urgentBreakdown?.L2?.total ?? null;
  const urgentL2Blocked = kpi.urgentBreakdown?.L2?.blocked ?? null;
  const showUrgentGroups =
    urgentL3Total != null && urgentL2Total != null && (urgentL3Total > 0 || urgentL2Total > 0);

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
      <Tooltip
        title={
          showUrgentGroups
            ? `紧急物料: ${kpi.urgentOrdersCount} | 阻塞: ${kpi.blockedUrgentCount} | L3: ${urgentL3Total} (阻${urgentL3Blocked || 0}) | L2: ${urgentL2Total} (阻${urgentL2Blocked || 0})`
            : `紧急物料: ${kpi.urgentOrdersCount} | 阻塞: ${kpi.blockedUrgentCount}`
        }
      >
        <Space size={6}>
          <FireOutlined style={{ color: '#ff4d4f', fontSize: 16 }} />
          {showUrgentGroups ? (
            <Text style={{ color: textColor }}>
              <span style={{ color: '#ff4d4f', fontFamily: 'monospace', fontWeight: 700 }}>L3</span>{' '}
              <span style={{ fontFamily: 'monospace', fontWeight: 700 }}>{urgentL3Total}</span>
              {urgentL3Blocked ? (
                <span style={{ color: '#ff4d4f', fontFamily: 'monospace' }}>({urgentL3Blocked})</span>
              ) : null}
              <span style={{ opacity: 0.6, margin: '0 6px' }}>·</span>
              <span style={{ color: '#faad14', fontFamily: 'monospace', fontWeight: 700 }}>L2</span>{' '}
              <span style={{ fontFamily: 'monospace', fontWeight: 700 }}>{urgentL2Total}</span>
              {urgentL2Blocked ? (
                <span style={{ color: '#ff4d4f', fontFamily: 'monospace' }}>({urgentL2Blocked})</span>
              ) : null}
            </Text>
          ) : (
            <Text style={{ color: textColor }}>
              {kpi.urgentOrdersCount}
              {kpi.blockedUrgentCount > 0 && (
                <span style={{ color: '#ff4d4f' }}> ({kpi.blockedUrgentCount})</span>
              )}
            </Text>
          )}
        </Space>
      </Tooltip>

      {/* 产能利用率 */}
      <Tooltip title={`产能利用率: ${kpi.capacityUtilization.toFixed(2)}%`}>
        <Space size={6}>
          <DashboardOutlined style={{ color: '#1677ff', fontSize: 16 }} />
          <Text style={{ color: textColor }}>{kpi.capacityUtilization.toFixed(2)}%</Text>
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
        title={`轧辊吨位: ${kpi.rollCampaignProgress.toFixed(3)}t / ${kpi.rollChangeThreshold.toFixed(3)}t`}
      >
        <Space size={6}>
          <ToolOutlined style={{ color: rollStatusColor, fontSize: 16 }} />
          <Text style={{ color: textColor }}>
            {kpi.rollCampaignProgress.toFixed(3)}t
          </Text>
        </Space>
      </Tooltip>
    </Space>
  );
};
