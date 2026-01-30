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

// 配置键说明（完整汉化版）
export const configDescriptions: Record<string, string> = {
  // 季节与适温配置
  season_mode: '季节模式（AUTO=自动判断，MANUAL=手动指定）',
  winter_months: '冬季月份定义（逗号分隔，如：11,12,1,2,3）',
  manual_season: '手动指定季节（WINTER=冬季，SUMMER=夏季）',
  min_temp_days_winter: '冬季最小适温天数（默认3天）',
  min_temp_days_summer: '夏季最小适温天数（默认4天）',

  // 机组代码配置
  standard_finishing_machines: '标准精整机组代码列表（逗号分隔，如：H032,H033,H034）',
  machine_offset_days: '非标机组出料偏移天数（默认4天）',

  // 紧急等级阈值配置
  urgent_n1_days: 'N1紧急天数阈值（临期关注，可选：2/3/5天，默认2天）',
  urgent_n2_days: 'N2紧急天数阈值（临期提示，可选：7/10/14天，默认7天）',

  // 换辊配置
  roll_suggest_threshold_t: '换辊建议阈值（单位：吨，默认1500吨）',
  roll_hard_limit_t: '换辊强制限制（单位：吨，默认2500吨）',

  // 产能配置
  overflow_pct: '产能溢出百分比（允许超出目标产能的比例，默认0.05即5%）',

  // 重算配置
  recalc_window_days: '重算窗口天数（重新计算排产的时间窗口，默认7天）',
  cascade_window_days: '级联重算窗口天数（影响后续排产的级联范围，默认14天）',

  // 结构校正配置
  target_ratio: '目标钢种配比（JSON格式，如：{"钢种A":0.3,"钢种B":0.5}，空对象{}表示不启用）',
  deviation_threshold: '结构偏差阈值（允许的目标配比偏差，默认0.1即10%）',

  // 数据质量配置
  weight_anomaly_threshold: '重量异常阈值（单位：吨，超过此值视为异常，默认100.0吨）',
  batch_retention_days: '批次数据保留天数（导入批次记录保留时长，默认90天）',
};
