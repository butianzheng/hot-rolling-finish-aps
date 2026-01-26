// ==========================================
// 亮色主题配置
// ==========================================
// 适用于办公室环境的亮色模式
// ==========================================

import type { ThemeConfig } from 'antd';
import {
  BRAND_COLORS,
  LIGHT_BACKGROUNDS,
  TEXT_COLORS,
  FONT_FAMILIES,
  BORDER_RADIUS,
  SPACING,
} from './tokens';

export const lightTheme: ThemeConfig = {
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
    colorBgBase: LIGHT_BACKGROUNDS.BASE,
    colorBgContainer: LIGHT_BACKGROUNDS.SURFACE,
    colorBgElevated: LIGHT_BACKGROUNDS.ELEVATED,
    colorBgLayout: LIGHT_BACKGROUNDS.BASE,

    // 文本颜色
    colorText: TEXT_COLORS.LIGHT.PRIMARY,
    colorTextSecondary: TEXT_COLORS.LIGHT.SECONDARY,
    colorTextDisabled: TEXT_COLORS.LIGHT.DISABLED,

    // 边框颜色
    colorBorder: 'rgba(0, 0, 0, 0.12)',
    colorBorderSecondary: 'rgba(0, 0, 0, 0.06)',

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
    // 阴影
    // ==========================================
    boxShadow: '0 1px 2px 0 rgba(0, 0, 0, 0.05)',
    boxShadowSecondary: '0 4px 6px -1px rgba(0, 0, 0, 0.1)',

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
      headerBg: LIGHT_BACKGROUNDS.HEADER,
      headerColor: TEXT_COLORS.LIGHT.INVERSE,
      headerHeight: 64,
      headerPadding: '0 24px',
      siderBg: LIGHT_BACKGROUNDS.SIDEBAR,
      bodyBg: LIGHT_BACKGROUNDS.BASE,
      footerBg: LIGHT_BACKGROUNDS.SURFACE,
    },

    // ==========================================
    // Menu 组件
    // ==========================================
    Menu: {
      itemBg: 'transparent',
      itemColor: TEXT_COLORS.LIGHT.SECONDARY,
      itemHoverBg: 'rgba(0, 0, 0, 0.04)',
      itemHoverColor: TEXT_COLORS.LIGHT.PRIMARY,
      itemSelectedBg: BRAND_COLORS.PRIMARY,
      itemSelectedColor: '#ffffff',
      itemActiveBg: 'rgba(0, 0, 0, 0.08)',
      iconSize: 16,
      iconMarginInlineEnd: 10,
    },

    // ==========================================
    // Table 组件
    // ==========================================
    Table: {
      headerBg: LIGHT_BACKGROUNDS.ELEVATED,
      headerColor: TEXT_COLORS.LIGHT.PRIMARY,
      rowHoverBg: 'rgba(0, 0, 0, 0.02)',
      rowSelectedBg: 'rgba(22, 119, 255, 0.1)',
      rowSelectedHoverBg: 'rgba(22, 119, 255, 0.15)',
      borderColor: 'rgba(0, 0, 0, 0.12)',
      headerSplitColor: 'rgba(0, 0, 0, 0.06)',
      fixedHeaderSortActiveBg: LIGHT_BACKGROUNDS.ELEVATED,
    },

    // ==========================================
    // Card 组件
    // ==========================================
    Card: {
      headerBg: 'transparent',
      colorBgContainer: LIGHT_BACKGROUNDS.SURFACE,
      colorBorderSecondary: 'rgba(0, 0, 0, 0.12)',
    },

    // ==========================================
    // Button 组件
    // ==========================================
    Button: {
      primaryColor: '#ffffff',
      defaultBg: LIGHT_BACKGROUNDS.SURFACE,
      defaultColor: TEXT_COLORS.LIGHT.PRIMARY,
      defaultBorderColor: 'rgba(0, 0, 0, 0.12)',
      defaultHoverBg: 'rgba(0, 0, 0, 0.04)',
      defaultHoverColor: TEXT_COLORS.LIGHT.PRIMARY,
      defaultHoverBorderColor: 'rgba(0, 0, 0, 0.24)',
    },

    // ==========================================
    // Input 组件
    // ==========================================
    Input: {
      colorBgContainer: LIGHT_BACKGROUNDS.SURFACE,
      colorBorder: 'rgba(0, 0, 0, 0.12)',
      colorText: TEXT_COLORS.LIGHT.PRIMARY,
      colorTextPlaceholder: TEXT_COLORS.LIGHT.DISABLED,
      hoverBorderColor: BRAND_COLORS.PRIMARY,
      activeBorderColor: BRAND_COLORS.PRIMARY,
    },

    // ==========================================
    // Select 组件
    // ==========================================
    Select: {
      colorBgContainer: LIGHT_BACKGROUNDS.SURFACE,
      colorBorder: 'rgba(0, 0, 0, 0.12)',
      colorText: TEXT_COLORS.LIGHT.PRIMARY,
      colorTextPlaceholder: TEXT_COLORS.LIGHT.DISABLED,
      optionSelectedBg: 'rgba(22, 119, 255, 0.1)',
    },

    // ==========================================
    // Modal 组件
    // ==========================================
    Modal: {
      contentBg: LIGHT_BACKGROUNDS.SURFACE,
      headerBg: LIGHT_BACKGROUNDS.SURFACE,
      titleColor: TEXT_COLORS.LIGHT.PRIMARY,
    },

    // ==========================================
    // Tooltip 组件
    // ==========================================
    Tooltip: {
      colorBgSpotlight: 'rgba(0, 0, 0, 0.75)',
      colorTextLightSolid: '#ffffff',
    },

    // ==========================================
    // Tag 组件
    // ==========================================
    Tag: {
      defaultBg: LIGHT_BACKGROUNDS.ELEVATED,
      defaultColor: TEXT_COLORS.LIGHT.PRIMARY,
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
      colorSplit: 'rgba(0, 0, 0, 0.12)',
    },
  },

  algorithm: undefined, // 不使用 Ant Design 的算法，完全自定义
};
