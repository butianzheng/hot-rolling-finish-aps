/**
 * 单个问题卡片组件
 */

import React from 'react';
import { Button, Space, Tag, Typography, theme } from 'antd';
import { ClockCircleOutlined, InfoCircleOutlined, ThunderboltOutlined } from '@ant-design/icons';
import { FONT_FAMILIES } from '../../theme';
import type { DrilldownSpec, RiskProblem } from './types';
import { getActionLabel, severityMeta, workbenchMeta } from './types';

const { Text } = Typography;

interface ProblemCardProps {
  problem: RiskProblem;
  index: number;
  entered: boolean;
  compact: boolean;
  onOpenDrilldown: (spec: DrilldownSpec) => void;
  onGoWorkbench: (problem: RiskProblem) => void;
}

export const ProblemCard: React.FC<ProblemCardProps> = ({
  problem: p,
  index,
  entered,
  compact,
  onOpenDrilldown,
  onGoWorkbench,
}) => {
  const { token } = theme.useToken();
  const meta = severityMeta(p.severity);
  const wb = workbenchMeta(p.workbenchTab);
  const actionLabel = getActionLabel(p.workbenchTab);

  return (
    <div
      role="button"
      tabIndex={0}
      onClick={() => onOpenDrilldown(p.drilldown)}
      onKeyDown={(e) => {
        if (e.key === 'Enter' || e.key === ' ') {
          e.preventDefault();
          onOpenDrilldown(p.drilldown);
        }
      }}
      style={{
        position: 'relative',
        borderRadius: token.borderRadiusLG,
        border: `1px solid ${token.colorBorderSecondary}`,
        background: token.colorBgContainer,
        overflow: 'hidden',
        padding: 12,
        cursor: 'pointer',
        opacity: entered ? 1 : 0,
        transform: entered ? 'translateY(0)' : 'translateY(6px)',
        transition: 'opacity 220ms ease, transform 220ms ease, border-color 220ms ease',
        transitionDelay: `${Math.min(index, 6) * 45}ms`,
      }}
    >
      {/* 背景渐变 */}
      <div
        aria-hidden
        style={{
          position: 'absolute',
          inset: 0,
          backgroundImage: `radial-gradient(520px circle at 0% 0%, ${meta.glow}, transparent 60%)`,
          pointerEvents: 'none',
        }}
      />

      {/* 左侧边框 */}
      <div
        aria-hidden
        style={{
          position: 'absolute',
          left: 0,
          top: 0,
          bottom: 0,
          width: 6,
          background: `linear-gradient(180deg, ${meta.color}, transparent)`,
          opacity: 0.85,
          pointerEvents: 'none',
        }}
      />

      <div
        style={{
          display: 'flex',
          alignItems: compact ? 'stretch' : 'flex-start',
          gap: 12,
          flexDirection: compact ? 'column' : 'row',
        }}
      >
        <div style={{ display: 'flex', gap: 12, alignItems: 'flex-start' }}>
          {/* 严重级别标识 */}
          <div
            style={{
              width: 52,
              minWidth: 52,
              height: 52,
              borderRadius: token.borderRadiusLG,
              background: token.colorFillQuaternary,
              border: `1px solid ${token.colorBorderSecondary}`,
              display: 'flex',
              flexDirection: 'column',
              alignItems: 'center',
              justifyContent: 'center',
              gap: 2,
              color: meta.color,
            }}
          >
            <Text style={{ color: meta.color, fontFamily: FONT_FAMILIES.MONOSPACE, fontWeight: 800 }}>
              {meta.badge}
            </Text>
            <Text style={{ color: token.colorTextSecondary, fontSize: 11 }}>{meta.label}</Text>
          </div>

          {/* 主内容区 */}
          <div style={{ flex: 1, minWidth: 0 }}>
            {/* 标题行 */}
            <div
              style={{
                display: 'flex',
                alignItems: 'baseline',
                justifyContent: 'space-between',
                gap: 12,
                flexWrap: 'wrap',
              }}
            >
              <Space size={10} wrap>
                <Space size={6} align="center">
                  <span style={{ color: meta.color, display: 'inline-flex', alignItems: 'center' }}>
                    {meta.icon}
                  </span>
                  <Text strong style={{ fontSize: 14 }}>
                    {p.title}
                  </Text>
                </Space>

                {typeof p.count === 'number' ? (
                  <Tag
                    style={{
                      marginInlineEnd: 0,
                      borderColor: token.colorBorderSecondary,
                      background: token.colorFillQuaternary,
                      color: token.colorTextSecondary,
                      fontFamily: FONT_FAMILIES.MONOSPACE,
                    }}
                  >
                    {p.count}
                  </Tag>
                ) : null}
              </Space>

              <Space size={6} wrap>
                {p.workbenchMachineCode ? (
                  <Tag
                    style={{
                      marginInlineEnd: 0,
                      borderColor: token.colorBorderSecondary,
                      background: token.colorFillQuaternary,
                      color: token.colorText,
                      fontFamily: FONT_FAMILIES.MONOSPACE,
                    }}
                  >
                    {p.workbenchMachineCode}
                  </Tag>
                ) : null}

                {wb ? (
                  <Tag
                    style={{
                      marginInlineEnd: 0,
                      borderColor: token.colorBorderSecondary,
                      background: token.colorFillQuaternary,
                      color: token.colorTextSecondary,
                    }}
                  >
                    <Space size={6}>
                      {wb.icon}
                      <span>{wb.label}</span>
                    </Space>
                  </Tag>
                ) : null}
              </Space>
            </div>

            {/* 详情标签行 */}
            <div style={{ marginTop: 8 }}>
              <Space size={8} wrap>
                {p.detail ? (
                  <Tag
                    style={{
                      marginInlineEnd: 0,
                      maxWidth: compact ? 360 : 520,
                      borderColor: token.colorBorderSecondary,
                      background: token.colorFillTertiary,
                      color: token.colorTextSecondary,
                    }}
                  >
                    <Space size={6}>
                      <InfoCircleOutlined />
                      <Text
                        style={{ color: token.colorTextSecondary, maxWidth: compact ? 300 : 460 }}
                        ellipsis={{ tooltip: p.detail }}
                      >
                        {p.detail}
                      </Text>
                    </Space>
                  </Tag>
                ) : null}

                {p.impact ? (
                  <Tag
                    style={{
                      marginInlineEnd: 0,
                      maxWidth: compact ? 360 : 520,
                      borderColor: token.colorBorderSecondary,
                      background: token.colorFillTertiary,
                      color: token.colorTextSecondary,
                    }}
                  >
                    <Space size={6}>
                      <ThunderboltOutlined />
                      <Text
                        style={{ color: token.colorTextSecondary, maxWidth: compact ? 300 : 460 }}
                        ellipsis={{ tooltip: p.impact }}
                      >
                        {p.impact}
                      </Text>
                    </Space>
                  </Tag>
                ) : null}

                {p.timeHint ? (
                  <Tag
                    style={{
                      marginInlineEnd: 0,
                      maxWidth: compact ? 360 : 520,
                      borderColor: token.colorBorderSecondary,
                      background: token.colorFillTertiary,
                      color: token.colorTextSecondary,
                    }}
                  >
                    <Space size={6}>
                      <ClockCircleOutlined />
                      <Text
                        style={{ color: token.colorTextSecondary, maxWidth: compact ? 300 : 460 }}
                        ellipsis={{ tooltip: p.timeHint }}
                      >
                        {p.timeHint}
                      </Text>
                    </Space>
                  </Tag>
                ) : null}
              </Space>
            </div>
          </div>
        </div>

        {/* 操作按钮 */}
        <Space
          size={8}
          style={{
            marginLeft: compact ? 0 : 'auto',
            justifyContent: compact ? 'flex-start' : 'flex-end',
          }}
          wrap
        >
          <Button
            onClick={(e) => {
              e.stopPropagation();
              onOpenDrilldown(p.drilldown);
            }}
          >
            详情
          </Button>
          <Button
            type="primary"
            onClick={(e) => {
              e.stopPropagation();
              onGoWorkbench(p);
            }}
          >
            {actionLabel}
          </Button>
        </Space>
      </div>
    </div>
  );
};
