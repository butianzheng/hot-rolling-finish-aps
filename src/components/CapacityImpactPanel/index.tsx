/**
 * 产能影响预测面板组件
 *
 * 显示选中物料对产能的影响预测
 */

import React from 'react';
import { Alert, Card, Space, Statistic, Tag, Tooltip, Typography } from 'antd';
import {
  ArrowDownOutlined,
  ArrowUpOutlined,
  CheckCircleOutlined,
  ExclamationCircleOutlined,
  WarningOutlined,
} from '@ant-design/icons';
import type { CapacityImpactPrediction } from '../../services/capacityImpactService';
import { getImpactRiskColor } from '../../services/capacityImpactService';
import { FONT_FAMILIES } from '../../theme';

const { Text } = Typography;

export interface CapacityImpactPanelProps {
  prediction: CapacityImpactPrediction;
  compact?: boolean;
}

export const CapacityImpactPanel: React.FC<CapacityImpactPanelProps> = ({
  prediction,
  compact = false,
}) => {
  const {
    originalCapacity,
    predictedCapacity,
    capacityDelta,
    utilizationChangePercent,
    affectedWeight,
    improves,
    risk,
    message,
    exceedsTargetAfter,
    exceedsLimitAfter,
    materialDetails,
  } = prediction;

  // 判断icon类型
  const getIcon = () => {
    if (exceedsLimitAfter) return <ExclamationCircleOutlined style={{ color: '#ff4d4f' }} />;
    if (exceedsTargetAfter) return <WarningOutlined style={{ color: '#faad14' }} />;
    if (improves) return <CheckCircleOutlined style={{ color: '#52c41a' }} />;
    return null;
  };

  // 判断Alert类型
  const getAlertType = (): 'success' | 'warning' | 'error' | 'info' => {
    if (exceedsLimitAfter) return 'error';
    if (exceedsTargetAfter) return 'warning';
    if (improves) return 'success';
    return 'info';
  };

  // 紧凑模式：只显示关键信息
  if (compact) {
    return (
      <Alert
        message={
          <Space size={4} style={{ fontSize: 12 }}>
            {getIcon()}
            <Text strong style={{ color: getImpactRiskColor(risk) }}>
              {message}
            </Text>
          </Space>
        }
        type={getAlertType()}
        showIcon={false}
        banner
        style={{ marginBottom: 8 }}
      />
    );
  }

  // 完整模式：显示详细的预测信息
  return (
    <Card
      size="small"
      title={
        <Space>
          <Text strong>产能影响预测</Text>
          <Tag color={getImpactRiskColor(risk)}>
            {risk === 'HIGH' ? '高风险' : risk === 'MEDIUM' ? '中风险' : '低风险'}
          </Tag>
        </Space>
      }
      style={{ marginBottom: 12, borderColor: getImpactRiskColor(risk) }}
    >
      {/* 预测结果摘要 */}
      <Alert
        message={message}
        type={getAlertType()}
        showIcon
        icon={getIcon()}
        style={{ marginBottom: 12 }}
      />

      {/* 产能变化统计 */}
      <Space direction="vertical" style={{ width: '100%' }} size={8}>
        <Space size={16}>
          <Statistic
            title="原始产能"
            value={originalCapacity.toFixed(1)}
            suffix="t"
            valueStyle={{ fontSize: 16, fontFamily: FONT_FAMILIES.MONOSPACE }}
          />

          <div style={{ fontSize: 20, color: '#d9d9d9' }}>→</div>

          <Statistic
            title="预测产能"
            value={predictedCapacity.toFixed(1)}
            suffix="t"
            valueStyle={{
              fontSize: 16,
              fontFamily: FONT_FAMILIES.MONOSPACE,
              color: getImpactRiskColor(risk),
            }}
          />

          <Statistic
            title="变化量"
            value={Math.abs(capacityDelta).toFixed(1)}
            suffix="t"
            prefix={capacityDelta < 0 ? <ArrowDownOutlined /> : <ArrowUpOutlined />}
            valueStyle={{
              fontSize: 16,
              fontFamily: FONT_FAMILIES.MONOSPACE,
              color: capacityDelta < 0 ? '#52c41a' : '#ff4d4f',
            }}
          />

          <Statistic
            title="利用率变化"
            value={Math.abs(utilizationChangePercent).toFixed(1)}
            suffix="%"
            prefix={utilizationChangePercent < 0 ? <ArrowDownOutlined /> : <ArrowUpOutlined />}
            valueStyle={{
              fontSize: 16,
              fontFamily: FONT_FAMILIES.MONOSPACE,
              color: utilizationChangePercent < 0 ? '#52c41a' : '#ff4d4f',
            }}
          />
        </Space>

        {/* 选中物料详情 */}
        {materialDetails.length > 0 && (
          <div>
            <Text type="secondary" style={{ fontSize: 12 }}>
              选中 {materialDetails.length} 件物料，合计 {affectedWeight.toFixed(1)}t
            </Text>
            <div style={{ marginTop: 4 }}>
              {materialDetails.slice(0, 3).map((m, idx) => (
                <Tooltip
                  key={idx}
                  title={`${m.material_id} - ${m.weight_t.toFixed(1)}t - ${m.urgent_level} - ${m.status}`}
                >
                  <Tag
                    style={{
                      fontSize: 11,
                      fontFamily: FONT_FAMILIES.MONOSPACE,
                      marginBottom: 4,
                    }}
                  >
                    {m.material_id} ({m.weight_t.toFixed(1)}t)
                  </Tag>
                </Tooltip>
              ))}
              {materialDetails.length > 3 && (
                <Tag style={{ fontSize: 11 }}>+{materialDetails.length - 3}更多</Tag>
              )}
            </div>
          </div>
        )}
      </Space>
    </Card>
  );
};
