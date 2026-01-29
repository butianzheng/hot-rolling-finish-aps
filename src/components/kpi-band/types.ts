/**
 * KPIBand 类型和工具函数
 */

import type { GlobalKPI } from '../../types/kpi';
import type { DrilldownSpec, WorkbenchTabKey } from '../../hooks/useRiskOverviewData';

// ==========================================
// Props 定义
// ==========================================

export interface KPIBandProps {
  loading?: boolean;
  kpi: GlobalKPI | null;
  onOpenDrilldown?: (spec: DrilldownSpec) => void;
  onGoWorkbench?: (opts: {
    workbenchTab?: WorkbenchTabKey;
    machineCode?: string | null;
    urgencyLevel?: string | null;
  }) => void;
}

// ==========================================
// 风险/换辊元数据
// ==========================================

export interface StatusMeta {
  label: string;
  color: string;
}

export function getRiskMeta(level: GlobalKPI['riskLevel']): StatusMeta {
  switch (level) {
    case 'critical':
      return { label: '严重', color: '#ff4d4f' };
    case 'high':
      return { label: '高', color: '#faad14' };
    case 'medium':
      return { label: '中', color: '#1677ff' };
    default:
      return { label: '低', color: '#52c41a' };
  }
}

export function getRollMeta(status: GlobalKPI['rollStatus']): StatusMeta {
  switch (status) {
    case 'critical':
      return { label: '硬停止', color: '#ff4d4f' };
    case 'warning':
      return { label: '预警', color: '#faad14' };
    default:
      return { label: '正常', color: '#52c41a' };
  }
}

// ==========================================
// 默认 KPI 值
// ==========================================

export const DEFAULT_KPI: Required<Omit<GlobalKPI, 'mostRiskyDate'>> & { mostRiskyDate?: string } = {
  urgentOrdersCount: 0,
  blockedUrgentCount: 0,
  urgentBreakdown: {
    L2: { total: 0, blocked: 0 },
    L3: { total: 0, blocked: 0 },
  },
  capacityUtilization: 0,
  coldStockCount: 0,
  rollCampaignProgress: 0,
  rollChangeThreshold: 1500,
  rollStatus: 'healthy' as const,
  riskLevel: 'low' as const,
};
