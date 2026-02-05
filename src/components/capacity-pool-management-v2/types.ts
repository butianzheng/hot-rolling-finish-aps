/**
 * 产能池管理日历视图 - 类型定义
 */

import type { CapacityPoolCalendarData } from '../../api/ipcSchemas/machineConfigSchemas';

/**
 * 视图模式
 */
export type ViewMode = 'day' | 'month';

/**
 * 日期范围快捷选项
 */
export interface DateRangePreset {
  label: string;
  getValue: () => { dateFrom: string; dateTo: string };
}

/**
 * 选中日期信息
 */
export interface SelectedDateInfo {
  date: string;
  data: CapacityPoolCalendarData;
}

/**
 * 批量调整请求
 */
export interface BatchAdjustRequest {
  dates: string[];
  targetCapacityT?: number;
  limitCapacityT?: number;
  reason: string;
}
