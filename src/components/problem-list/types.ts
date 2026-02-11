/**
 * ProblemList 类型定义和配置
 */

import React from 'react';
import {
  BarChartOutlined,
  DeploymentUnitOutlined,
  FireOutlined,
  InboxOutlined,
  InfoCircleOutlined,
  WarningOutlined,
} from '@ant-design/icons';
import type { DrilldownSpec, ProblemSeverity, RiskProblem, WorkbenchTabKey } from '../../hooks/useRiskOverviewData';

export type { DrilldownSpec, ProblemSeverity, RiskProblem, WorkbenchTabKey };

export interface ProblemListProps {
  loading?: boolean;
  snapshotKey?: string;
  problems: RiskProblem[];
  onOpenDrilldown: (spec: DrilldownSpec) => void;
  onGoWorkbench: (problem: RiskProblem) => void;
}

export interface SeverityMeta {
  badge: string;
  label: string;
  color: string;
  glow: string;
  icon: React.ReactNode;
}

export interface WorkbenchMeta {
  label: string;
  icon: React.ReactNode;
}

export function severityMeta(severity: ProblemSeverity): SeverityMeta {
  switch (severity) {
    case 'P0':
      return {
        badge: 'P0',
        label: '立即处理',
        color: '#ff4d4f',
        glow: 'rgba(255, 77, 79, 0.18)',
        icon: React.createElement(FireOutlined),
      };
    case 'P1':
      return {
        badge: 'P1',
        label: '尽快处理',
        color: '#faad14',
        glow: 'rgba(250, 173, 20, 0.16)',
        icon: React.createElement(WarningOutlined),
      };
    case 'P2':
      return {
        badge: 'P2',
        label: '关注',
        color: '#1677ff',
        glow: 'rgba(22, 119, 255, 0.14)',
        icon: React.createElement(InfoCircleOutlined),
      };
    default:
      return {
        badge: 'P3',
        label: '观察',
        color: '#52c41a',
        glow: 'rgba(82, 196, 26, 0.14)',
        icon: React.createElement(InfoCircleOutlined),
      };
  }
}

export function workbenchMeta(tab: WorkbenchTabKey | undefined): WorkbenchMeta | null {
  switch (tab) {
    case 'materials':
      return { label: '物料', icon: React.createElement(InboxOutlined) };
    case 'capacity':
      return { label: '产能', icon: React.createElement(BarChartOutlined) };
    case 'visualization':
      return { label: '排程', icon: React.createElement(DeploymentUnitOutlined) };
    default:
      return null;
  }
}

export function getActionLabel(tab: WorkbenchTabKey | undefined): string {
  switch (tab) {
    case 'materials':
      return '去物料处理';
    case 'capacity':
      return '去机组配置';
    default:
      return '去排程处理';
  }
}
