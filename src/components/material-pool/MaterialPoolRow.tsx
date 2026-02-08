/**
 * MaterialPool 行渲染组件
 *
 * 增强版：显示操作状态徽章、风险标记
 */

import React, { useMemo } from 'react';
import { Checkbox, Typography, Tooltip } from 'antd';
import { DownOutlined, RightOutlined } from '@ant-design/icons';
import type { RowComponentProps } from 'react-window';
import { UrgencyTag } from '../UrgencyTag';
import { FONT_FAMILIES } from '../../theme';
import type { MaterialPoolMaterial, PoolRow, UrgencyBucket } from './types';
import {
  computeOperabilityStatus,
  computeRiskBadges,
  getOperabilityConfig,
} from '../../utils/operabilityStatus';
import type { RiskBadge } from '../../utils/operabilityStatus';
import { getRiskSeverityColor } from '../../utils/operabilityStatus';
import { formatWeight } from '../../utils/formatters';

const { Text } = Typography;

export type RowData = {
  rows: PoolRow[];
  selected: Set<string>;
  onToggle: (id: string, checked: boolean) => void;
  onInspect?: (material: MaterialPoolMaterial) => void;
  onToggleUrgency: (level: UrgencyBucket) => void;
};

/**
 * 紧凑风险标记显示（只显示图标，hover显示详情）
 */
const CompactRiskBadge: React.FC<{ badge: RiskBadge }> = ({ badge }) => {
  const tooltipText = badge.tooltip ? `${badge.label}: ${badge.tooltip}` : badge.label;

  return (
    <Tooltip title={tooltipText}>
      <span
        style={{
          display: 'inline-flex',
          alignItems: 'center',
          justifyContent: 'center',
          width: 16,
          height: 16,
          borderRadius: 8,
          backgroundColor: getRiskSeverityColor(badge.severity),
          color: '#fff',
          fontSize: 10,
          fontWeight: 600,
        }}
      >
        {badge.severity === 'CRITICAL' ? '!' : badge.severity === 'HIGH' ? '!' : '•'}
      </span>
    </Tooltip>
  );
};

export const MaterialPoolRow = React.memo(function MaterialPoolRow({
  index,
  style,
  rows,
  selected,
  onToggle,
  onInspect,
  onToggleUrgency,
}: RowComponentProps<RowData>) {
  const row = rows[index];

  // Header row (紧急度分组头)
  if (row.type === 'header') {
    return (
      <div
        style={{
          ...style,
          display: 'flex',
          alignItems: 'center',
          padding: '0 10px',
          borderBottom: '1px solid rgba(0,0,0,0.06)',
          background: 'rgba(0,0,0,0.02)',
          cursor: 'pointer',
          gap: 8,
        }}
        onClick={() => onToggleUrgency(row.level)}
      >
        <span style={{ width: 16, display: 'flex', justifyContent: 'center' }}>
          {row.collapsed ? <RightOutlined /> : <DownOutlined />}
        </span>
        <UrgencyTag level={row.level} />
        <Text style={{ fontWeight: 600 }}>{row.level}</Text>
        <Text type="secondary">({row.count})</Text>
        <Text type="secondary" style={{ marginLeft: 'auto', fontFamily: FONT_FAMILIES.MONOSPACE }}>
          {formatWeight(row.weight)}
        </Text>
      </div>
    );
  }

  // Material row (物料行)
  const m = row.material;
  const checked = selected.has(m.material_id);

  // 计算操作状态和风险标记
  const operabilityStatus = useMemo(() => computeOperabilityStatus(m), [m]);
  const riskBadges = useMemo(() => computeRiskBadges(m), [m]);
  const statusConfig = useMemo(() => getOperabilityConfig(operabilityStatus), [operabilityStatus]);

  return (
    <div
      style={{
        ...style,
        display: 'flex',
        flexDirection: 'column',
        justifyContent: 'center',
        padding: '4px 10px',
        borderBottom: '1px solid rgba(0,0,0,0.06)',
        cursor: 'pointer',
        transition: 'background-color 0.2s ease',
      }}
      onClick={() => onInspect?.(m)}
      onMouseEnter={(e) => {
        e.currentTarget.style.backgroundColor = 'rgba(24, 144, 255, 0.04)';
      }}
      onMouseLeave={(e) => {
        e.currentTarget.style.backgroundColor = 'transparent';
      }}
    >
      {/* 第一行：勾选框 + 状态指示点 + 物料号 + 紧急度 */}
      <div style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
        <Checkbox
          checked={checked}
          onClick={(e) => e.stopPropagation()}
          onChange={(e) => onToggle(m.material_id, e.target.checked)}
        />

        {/* 操作状态指示点 */}
        <Tooltip title={`${statusConfig.label}: ${statusConfig.description}`}>
          <span
            style={{
              display: 'inline-block',
              width: 8,
              height: 8,
              borderRadius: 4,
              backgroundColor: statusConfig.color,
              flexShrink: 0,
            }}
          />
        </Tooltip>

        <Text
          style={{
            fontFamily: FONT_FAMILIES.MONOSPACE,
            flex: 1,
            minWidth: 0,
            fontSize: 13,
          }}
          ellipsis
        >
          {m.material_id}
        </Text>

        <UrgencyTag level={m.urgent_level} />
      </div>

      {/* 第二行：钢种 + 重量 + 状态标签 + 风险标记 */}
      <div style={{ display: 'flex', alignItems: 'center', gap: 8, marginTop: 4, paddingLeft: 24 }}>
        <Text type="secondary" style={{ fontSize: 11 }} ellipsis>
          {m.steel_mark || '-'} · {formatWeight(Number(m.weight_t || 0))}
        </Text>

        {m.contract_no ? (
          <Text type="secondary" style={{ fontSize: 11 }} ellipsis>
            合同:{m.contract_no}
          </Text>
        ) : null}

        {m.due_date ? (
          <Text type="secondary" style={{ fontSize: 11 }} ellipsis>
            交期:{m.due_date}
          </Text>
        ) : null}

        {m.scheduled_date ? (
          <Text type="secondary" style={{ fontSize: 11 }} ellipsis>
            排程:{m.scheduled_date}
          </Text>
        ) : null}

        {/* 状态标签 */}
        <Tooltip title={statusConfig.description}>
          <Text
            style={{
              fontSize: 11,
              color: statusConfig.color,
              fontWeight: 500,
            }}
          >
            {statusConfig.label}
          </Text>
        </Tooltip>

        {/* 风险标记 */}
        <div style={{ marginLeft: 'auto', display: 'flex', gap: 2 }}>
          {riskBadges.slice(0, 3).map((badge, idx) => (
            <CompactRiskBadge key={idx} badge={badge} />
          ))}
          {riskBadges.length > 3 && (
            <Tooltip
              title={
                <div>
                  <div style={{ fontWeight: 600, marginBottom: 4 }}>其他风险：</div>
                  {riskBadges.slice(3).map((badge, idx) => (
                    <div key={idx}>• {badge.label}</div>
                  ))}
                </div>
              }
            >
              <span
                style={{
                  display: 'inline-flex',
                  alignItems: 'center',
                  justifyContent: 'center',
                  width: 16,
                  height: 16,
                  borderRadius: 8,
                  backgroundColor: '#d9d9d9',
                  color: '#fff',
                  fontSize: 10,
                  fontWeight: 600,
                }}
              >
                +{riskBadges.length - 3}
              </span>
            </Tooltip>
          )}
        </div>
      </div>
    </div>
  );
});
