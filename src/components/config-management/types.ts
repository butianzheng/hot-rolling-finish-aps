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

// 作用域类型中文名称
export const scopeTypeLabels: Record<string, string> = {
  GLOBAL: '全局',
  MACHINE: '机组',
  STEEL_GRADE: '钢种',
  VERSION: '版本',
};

// 作用域ID中文名称（常见值映射）
export const scopeIdLabels: Record<string, string> = {
  global: '全局',
  // 机组作用域ID会动态显示，如 H031, H032 等
  // 可根据需要扩展
};

// 配置键中文名称
export const configKeyLabels: Record<string, string> = {
  // 季节与适温配置
  season_mode: '季节模式',
  winter_months: '冬季月份',
  manual_season: '手动季节',
  min_temp_days_winter: '冬季适温天数',
  min_temp_days_summer: '夏季适温天数',

  // 机组代码配置
  standard_finishing_machines: '标准精整机组',
  machine_offset_days: '机组偏移天数',

  // 紧急等级阈值配置
  urgent_n1_days: 'N1紧急阈值',
  urgent_n2_days: 'N2紧急阈值',

  // 换辊配置
  roll_suggest_threshold_t: '换辊建议阈值',
  roll_hard_limit_t: '换辊强制限制',
  roll_change_downtime_minutes: '换辊停机时长',

  // 产能配置
  overflow_pct: '产能溢出比例',

  // 重算配置
  recalc_window_days: '重算窗口天数',
  cascade_window_days: '级联窗口天数',

  // 结构校正配置
  target_ratio: '目标钢种配比',
  deviation_threshold: '结构偏差阈值',
  rhythm_deviation_threshold: '节奏偏差阈值',

  // D4 堵塞评分配置
  d4_capacity_hard_threshold: 'D4 产能硬阈值',
  d4_capacity_full_threshold: 'D4 产能满载阈值',
  d4_structure_dev_threshold: 'D4 结构偏差阈值',
  d4_structure_dev_full_multiplier: 'D4 结构满载倍数',
  d4_structure_small_category_threshold: 'D4 小类忽略阈值',
  d4_structure_violation_full_count: 'D4 结构违规满载数',
  d4_bottleneck_low_threshold: 'D4 等级-LOW阈值',
  d4_bottleneck_medium_threshold: 'D4 等级-MEDIUM阈值',
  d4_bottleneck_high_threshold: 'D4 等级-HIGH阈值',
  d4_bottleneck_critical_threshold: 'D4 等级-CRITICAL阈值',

  // 数据质量配置
  weight_anomaly_threshold: '重量异常阈值',
  batch_retention_days: '批次保留天数',

  // 宽厚路径规则（v0.6）
  path_rule_enabled: '宽厚路径规则开关',
  path_width_tolerance_mm: '路径宽度容差 (mm)',
  path_thickness_tolerance_mm: '路径厚度容差 (mm)',
  path_override_allowed_urgency_levels: '允许人工突破的紧急等级',
  seed_s2_percentile: 'S2 种子分位数',
  seed_s2_small_sample_threshold: 'S2 小样本阈值',
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
  roll_change_downtime_minutes: '换辊停机时长（单位：分钟，典型30~60分钟，默认45分钟）',

  // 产能配置
  overflow_pct: '产能溢出百分比（允许超出目标产能的比例，默认0.05即5%）',

  // 重算配置
  recalc_window_days: '重算窗口天数（重新计算排产的时间窗口，默认7天）',
  cascade_window_days: '级联重算窗口天数（影响后续排产的级联范围，默认14天）',

  // 结构校正配置
  target_ratio: '目标钢种配比（JSON格式，如：{"钢种A":0.3,"钢种B":0.5}，空对象{}表示不启用）',
  deviation_threshold: '结构偏差阈值（允许的目标配比偏差，默认0.1即10%）',
  rhythm_deviation_threshold: '每日生产节奏偏差阈值（用于节奏监控的最大偏差阈值，默认0.1即10%）',

  // D4 堵塞评分配置
  d4_capacity_hard_threshold: '产能硬阈值（used/limit）。低于该值不计堵塞，仅提示。默认0.95。',
  d4_capacity_full_threshold: '产能满载阈值（used/limit）。用于将>硬阈值的利用率线性映射为0~1严重度，默认1.0。',
  d4_structure_dev_threshold: '结构加权偏差起算阈值（0~1）。低于该值不计堵塞。默认0.10。',
  d4_structure_dev_full_multiplier: '结构偏差满载倍数。满载阈值=偏差阈值×倍数，默认2.0。',
  d4_structure_small_category_threshold: '小类忽略阈值（0~1）。当某品类目标/实际占比均低于该值时不参与偏差计算，默认0.05。',
  d4_structure_violation_full_count: '结构违规满载数量。违规数达到该值时严重度=1，默认10。',
  d4_bottleneck_low_threshold: '堵塞等级阈值-LOW（严重度0~1）。低于该值为“无”，默认0.30。',
  d4_bottleneck_medium_threshold: '堵塞等级阈值-MEDIUM（严重度0~1），默认0.60。',
  d4_bottleneck_high_threshold: '堵塞等级阈值-HIGH（严重度0~1），默认0.90。HIGH/CRITICAL视为堵塞。',
  d4_bottleneck_critical_threshold: '堵塞等级阈值-CRITICAL（严重度0~1），默认0.95。',

  // 数据质量配置
  weight_anomaly_threshold: '重量异常阈值（单位：吨，超过此值视为异常，默认100.0吨）',
  batch_retention_days: '批次数据保留天数（导入批次记录保留时长，默认90天）',

  // 宽厚路径规则（v0.6）
  path_rule_enabled: '是否启用“由宽到窄、由厚到薄”的路径约束（true/false）',
  path_width_tolerance_mm: '宽度容差（单位：mm）。候选宽度允许小于锚点宽度的最大差值，超过则判定违规。',
  path_thickness_tolerance_mm: '厚度容差（单位：mm）。候选厚度允许小于锚点厚度的最大差值，超过则判定违规。',
  path_override_allowed_urgency_levels:
    '允许人工确认突破的紧急等级列表（逗号分隔，如：L2,L3）。不在列表内的违规将被硬拦截。',
  seed_s2_percentile:
    '无冻结/锁定/已确认锚点时，用历史分布生成初始锚点：取宽度与厚度的该分位数（0~1）。',
  seed_s2_small_sample_threshold:
    'S2 种子策略的小样本阈值（整数）。样本数不足时回退到“最大宽度/最大厚度”等保守规则。',
};
