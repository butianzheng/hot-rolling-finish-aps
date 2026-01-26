// ==========================================
// 设计令牌 (Design Tokens)
// ==========================================
// 定义系统级的颜色、字体、间距常量
// 符合工业级 SaaS 设计规范
// ==========================================

// ==========================================
// 紧急度颜色映射 (Urgency Levels L0-L3)
// ==========================================
export const URGENCY_COLORS = {
  L3_EMERGENCY: '#ff4d4f', // 红色 - 紧急/红线，必须突出
  L2_HIGH: '#faad14',      // 金色 - 高优先级/冻结
  L1_MEDIUM: '#1677ff',    // 蓝色 - 中等优先级
  L0_NORMAL: '#8c8c8c',    // 灰色 - 正常
} as const;

// ==========================================
// 材料状态颜色
// ==========================================
export const STATE_COLORS = {
  FROZEN_LOCKED: '#262626',    // 深灰背景 - 冻结/锁定
  TEMP_ISSUE: '#13c2c2',       // 青色 - 温度问题（冷却中）
  CAPACITY_OVERFLOW: '#cf1322', // 深红 - 产能溢出
  READY: '#52c41a',            // 绿色 - 就绪
  SCHEDULED: '#1677ff',        // 蓝色 - 已排产
} as const;

// ==========================================
// 品牌颜色
// ==========================================
export const BRAND_COLORS = {
  PRIMARY: '#1677ff',      // 科技蓝 - 代表决策/行动
  SUCCESS: '#52c41a',      // 成功绿
  WARNING: '#faad14',      // 警告橙
  ERROR: '#ff4d4f',        // 错误红
  INFO: '#1677ff',         // 信息蓝
} as const;

// ==========================================
// 暗色主题背景色
// ==========================================
export const DARK_BACKGROUNDS = {
  BASE: '#141414',         // 哑光黑（非纯黑）
  SURFACE: '#1f1f1f',      // 卡片背景
  ELEVATED: '#2a2a2a',     // 悬浮元素背景
  HEADER: '#000000',       // Header 背景（纯黑）
  SIDEBAR: '#1f1f1f',      // 侧边栏背景
} as const;

// ==========================================
// 亮色主题背景色
// ==========================================
export const LIGHT_BACKGROUNDS = {
  BASE: '#f5f5f5',         // 浅灰
  SURFACE: '#ffffff',      // 卡片背景（白色）
  ELEVATED: '#fafafa',     // 悬浮元素背景
  HEADER: '#001529',       // Header 背景（深蓝）
  SIDEBAR: '#ffffff',      // 侧边栏背景
} as const;

// ==========================================
// 文本颜色
// ==========================================
export const TEXT_COLORS = {
  DARK: {
    PRIMARY: 'rgba(255, 255, 255, 0.85)',   // 主要文本
    SECONDARY: 'rgba(255, 255, 255, 0.65)', // 次要文本
    DISABLED: 'rgba(255, 255, 255, 0.45)',  // 禁用文本
    INVERSE: 'rgba(0, 0, 0, 0.85)',         // 反色文本
  },
  LIGHT: {
    PRIMARY: 'rgba(0, 0, 0, 0.85)',         // 主要文本
    SECONDARY: 'rgba(0, 0, 0, 0.65)',       // 次要文本
    DISABLED: 'rgba(0, 0, 0, 0.45)',        // 禁用文本
    INVERSE: 'rgba(255, 255, 255, 0.85)',   // 反色文本
  },
} as const;

// ==========================================
// 字体家族
// ==========================================
export const FONT_FAMILIES = {
  BASE: `-apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, 'Helvetica Neue', Arial, sans-serif`,
  MONOSPACE: `'JetBrains Mono', 'Roboto Mono', 'Courier New', monospace`, // 用于数值数据
  HEADING: `-apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, 'Helvetica Neue', Arial, sans-serif`,
} as const;

// ==========================================
// 字体大小
// ==========================================
export const FONT_SIZES = {
  XS: 12,
  SM: 14,
  BASE: 16,
  LG: 18,
  XL: 20,
  XXL: 24,
  XXXL: 30,
} as const;

// ==========================================
// 间距系统 (8px 基准)
// ==========================================
export const SPACING = {
  XS: 4,
  SM: 8,
  BASE: 16,
  LG: 24,
  XL: 32,
  XXL: 48,
  XXXL: 64,
} as const;

// ==========================================
// 圆角
// ==========================================
export const BORDER_RADIUS = {
  SM: 2,
  BASE: 4,
  LG: 8,
  XL: 12,
  ROUND: 9999, // 完全圆角
} as const;

// ==========================================
// 阴影
// ==========================================
export const SHADOWS = {
  SM: '0 1px 2px 0 rgba(0, 0, 0, 0.05)',
  BASE: '0 1px 3px 0 rgba(0, 0, 0, 0.1), 0 1px 2px 0 rgba(0, 0, 0, 0.06)',
  MD: '0 4px 6px -1px rgba(0, 0, 0, 0.1), 0 2px 4px -1px rgba(0, 0, 0, 0.06)',
  LG: '0 10px 15px -3px rgba(0, 0, 0, 0.1), 0 4px 6px -2px rgba(0, 0, 0, 0.05)',
  XL: '0 20px 25px -5px rgba(0, 0, 0, 0.1), 0 10px 10px -5px rgba(0, 0, 0, 0.04)',
} as const;

// ==========================================
// Z-Index 层级
// ==========================================
export const Z_INDEX = {
  BASE: 0,
  DROPDOWN: 1000,
  STICKY: 1020,
  FIXED: 1030,
  MODAL_BACKDROP: 1040,
  MODAL: 1050,
  POPOVER: 1060,
  TOOLTIP: 1070,
} as const;

// ==========================================
// 动画时长
// ==========================================
export const TRANSITIONS = {
  FAST: '150ms',
  BASE: '300ms',
  SLOW: '500ms',
} as const;

// ==========================================
// 布局尺寸
// ==========================================
export const LAYOUT = {
  HEADER_HEIGHT: 64,
  SIDEBAR_WIDTH: 200,
  SIDEBAR_COLLAPSED_WIDTH: 80,
  INSPECTOR_WIDTH: 400,
  CONTENT_MAX_WIDTH: 1920,
} as const;

// ==========================================
// 轧辊更换阈值 (吨位)
// ==========================================
export const ROLL_CHANGE_THRESHOLDS = {
  WARNING: 1500,  // 警告阈值
  CRITICAL: 2500, // 临界阈值
} as const;
