/**
 * 配置管理 - 类型定义
 */

export interface ConfigItem {
  scope_id: string;
  scope_type: string;
  key: string;
  value: string;
  updated_at?: string;
}

// 作用域类型颜色
export const scopeTypeColors: Record<string, string> = {
  GLOBAL: 'blue',
  MACHINE: 'green',
  STEEL_GRADE: 'orange',
  VERSION: 'purple',
};

// 配置键说明
export const configDescriptions: Record<string, string> = {
  season_mode: '季节模式 (AUTO/MANUAL)',
  winter_months: '冬季月份 (逗号分隔)',
  manual_season: '手动季节 (WINTER/SUMMER)',
  min_temp_days_winter: '冬季最小适温天数',
  min_temp_days_summer: '夏季最小适温天数',
  urgent_n1_days: 'N1紧急天数阈值',
  urgent_n2_days: 'N2紧急天数阈值',
  roll_suggest_threshold_t: '换辊建议阈值(吨)',
  roll_hard_limit_t: '换辊硬限制(吨)',
  overflow_pct: '产能溢出百分比',
  recalc_window_days: '重算窗口天数',
};
