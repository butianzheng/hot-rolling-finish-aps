// ==========================================
// 暗色主题配置
// ==========================================
// 适用于控制室环境的暗色模式
// ==========================================

import type { ThemeConfig } from 'antd';
import {
  BRAND_COLORS,
  DARK_BACKGROUNDS,
  TEXT_COLORS,
  FONT_FAMILIES,
  BORDER_RADIUS,
  SPACING,
} from './tokens';

export const darkTheme: ThemeConfig = {
  token: {
    // ==========================================
    // 颜色系统
    // ==========================================
    colorPrimary: BRAND_COLORS.PRIMARY,
    colorSuccess: BRAND_COLORS.SUCCESS,
    colorWarning: BRAND_COLORS.WARNING,
    colorError: BRAND_COLORS.ERROR,
    colorInfo: BRAND_COLORS.INFO,

    // 背景色
    colorBgBase: DARK_BACKGROUNDS.BASE,
    colorBgContainer: DARK_BACKGROUNDS.SURFACE,
    colorBgElevated: DARK_BACKGROUNDS.ELEVATED,
    colorBgLayout: DARK_BACKGROUNDS.BASE,

    // 文本颜色
    colorText: TEXT_COLORS.DARK.PRIMARY,
    colorTextSecondary: TEXT_COLORS.DARK.SECONDARY,
    colorTextDisabled: TEXT_COLORS.DARK.DISABLED,

    // 边框颜色
    colorBorder: 'rgba(255, 255, 255, 0.12)',
    colorBorderSecondary: 'rgba(255, 255, 255, 0.06)',

    // ==========================================
    // 字体系统
    // ==========================================
    fontFamily: FONT_FAMILIES.BASE,
    fontSize: 14,
    fontSizeHeading1: 30,
    fontSizeHeading2: 24,
    fontSizeHeading3: 20,
    fontSizeHeading4: 18,
    fontSizeHeading5: 16,

    // ==========================================
    // 圆角
    // ==========================================
    borderRadius: BORDER_RADIUS.BASE,
    borderRadiusLG: BORDER_RADIUS.LG,
    borderRadiusSM: BORDER_RADIUS.SM,

    // ==========================================
    // 间距
    // ==========================================
    padding: SPACING.BASE,
    paddingLG: SPACING.LG,
    paddingSM: SPACING.SM,
    paddingXS: SPACING.XS,

    margin: SPACING.BASE,
    marginLG: SPACING.LG,
    marginSM: SPACING.SM,
    marginXS: SPACING.XS,

    // ==========================================
    // 阴影（暗色模式下减弱）
    // ==========================================
    boxShadow: '0 1px 2px 0 rgba(0, 0, 0, 0.3)',
    boxShadowSecondary: '0 4px 6px -1px rgba(0, 0, 0, 0.3)',

    // ==========================================
    // 控件高度
    // ==========================================
    controlHeight: 32,
    controlHeightLG: 40,
    controlHeightSM: 24,

    // ==========================================
    // 线条宽度
    // ==========================================
    lineWidth: 1,
    lineType: 'solid',

    // ==========================================
    // 动画
    // ==========================================
    motionDurationSlow: '0.3s',
    motionDurationMid: '0.2s',
    motionDurationFast: '0.1s',
  },

  components: {
    // ==========================================
    // Layout 组件
    // ==========================================
    Layout: {
      headerBg: DARK_BACKGROUNDS.HEADER,
      headerColor: TEXT_COLORS.DARK.PRIMARY,
      headerHeight: 64,
      headerPadding: '0 24px',
      siderBg: DARK_BACKGROUNDS.SIDEBAR,
      bodyBg: DARK_BACKGROUNDS.BASE,
      footerBg: DARK_BACKGROUNDS.SURFACE,
    },

    // ==========================================
    // Menu 组件
    // ==========================================
    Menu: {
      itemBg: 'transparent',
      itemColor: TEXT_COLORS.DARK.SECONDARY,
      itemHoverBg: 'rgba(255, 255, 255, 0.08)',
      itemHoverColor: TEXT_COLORS.DARK.PRIMARY,
      itemSelectedBg: BRAND_COLORS.PRIMARY,
      itemSelectedColor: '#ffffff',
      itemActiveBg: 'rgba(255, 255, 255, 0.12)',
      iconSize: 16,
      iconMarginInlineEnd: 10,
    },

    // ==========================================
    // Table 组件
    // ==========================================
    Table: {
      headerBg: DARK_BACKGROUNDS.ELEVATED,
      headerColor: TEXT_COLORS.DARK.PRIMARY,
      rowHoverBg: 'rgba(255, 255, 255, 0.04)',
      rowSelectedBg: 'rgba(22, 119, 255, 0.15)',
      rowSelectedHoverBg: 'rgba(22, 119, 255, 0.2)',
      borderColor: 'rgba(255, 255, 255, 0.12)',
      headerSplitColor: 'rgba(255, 255, 255, 0.06)',
      fixedHeaderSortActiveBg: DARK_BACKGROUNDS.ELEVATED,
    },

    // ==========================================
    // Card 组件
    // ==========================================
    Card: {
      headerBg: 'transparent',
      colorBgContainer: DARK_BACKGROUNDS.SURFACE,
      colorBorderSecondary: 'rgba(255, 255, 255, 0.12)',
    },

    // ==========================================
    // Button 组件
    // ==========================================
    Button: {
      primaryColor: '#ffffff',
      defaultBg: DARK_BACKGROUNDS.ELEVATED,
      defaultColor: TEXT_COLORS.DARK.PRIMARY,
      defaultBorderColor: 'rgba(255, 255, 255, 0.12)',
      defaultHoverBg: 'rgba(255, 255, 255, 0.08)',
      defaultHoverColor: TEXT_COLORS.DARK.PRIMARY,
      defaultHoverBorderColor: 'rgba(255, 255, 255, 0.24)',
    },

    // ==========================================
    // Input 组件
    // ==========================================
    Input: {
      colorBgContainer: DARK_BACKGROUNDS.ELEVATED,
      colorBorder: 'rgba(255, 255, 255, 0.12)',
      colorText: TEXT_COLORS.DARK.PRIMARY,
      colorTextPlaceholder: TEXT_COLORS.DARK.DISABLED,
      hoverBorderColor: BRAND_COLORS.PRIMARY,
      activeBorderColor: BRAND_COLORS.PRIMARY,
    },

    // ==========================================
    // Select 组件
    // ==========================================
    Select: {
      colorBgContainer: DARK_BACKGROUNDS.ELEVATED,
      colorBorder: 'rgba(255, 255, 255, 0.12)',
      colorText: TEXT_COLORS.DARK.PRIMARY,
      colorTextPlaceholder: TEXT_COLORS.DARK.DISABLED,
      optionSelectedBg: 'rgba(22, 119, 255, 0.15)',
    },

    // ==========================================
    // Modal 组件
    // ==========================================
    Modal: {
      contentBg: DARK_BACKGROUNDS.SURFACE,
      headerBg: DARK_BACKGROUNDS.SURFACE,
      titleColor: TEXT_COLORS.DARK.PRIMARY,
    },

    // ==========================================
    // Tooltip 组件
    // ==========================================
    Tooltip: {
      colorBgSpotlight: 'rgba(0, 0, 0, 0.85)',
      colorTextLightSolid: TEXT_COLORS.DARK.PRIMARY,
    },

    // ==========================================
    // Tag 组件
    // ==========================================
    Tag: {
      defaultBg: DARK_BACKGROUNDS.ELEVATED,
      defaultColor: TEXT_COLORS.DARK.PRIMARY,
    },

    // ==========================================
    // Badge 组件
    // ==========================================
    Badge: {
      textFontSize: 12,
      textFontWeight: 'normal',
    },

    // ==========================================
    // Divider 组件
    // ==========================================
    Divider: {
      colorSplit: 'rgba(255, 255, 255, 0.12)',
    },
  },

  algorithm: undefined, // 不使用 Ant Design 的算法，完全自定义
};
