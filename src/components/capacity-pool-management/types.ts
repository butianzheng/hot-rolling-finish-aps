/**
 * 产能池管理 - 类型定义
 */

export interface CapacityPool {
  machine_code: string;
  plan_date: string;
  target_capacity_t: number;
  limit_capacity_t: number;
  used_capacity_t: number;
  available_capacity_t: number;
}

export interface CapacityPoolManagementProps {
  onNavigateToPlan?: () => void;
}

export interface TotalStats {
  totalTarget: number;
  totalUsed: number;
  totalAvailable: number;
}
