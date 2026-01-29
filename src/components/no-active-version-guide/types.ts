/**
 * NoActiveVersionGuide 类型定义
 */

import type { ReactNode } from 'react';

export interface NoActiveVersionGuideProps {
  onNavigateToPlan: () => void; // 导航到排产方案的回调
  onNavigateToImport?: () => void; // 导航到数据导入的回调（可选）
  title?: string; // 自定义标题
  description?: string; // 自定义描述
}

export interface StepItem {
  title: string;
  description: string;
  icon: ReactNode;
}

export interface FAQItem {
  question: string;
  answer: string;
}

// 常见问题列表
export const FAQ_ITEMS: FAQItem[] = [
  {
    question: '为什么看不到排产数据？',
    answer: '系统需要一个激活的排产版本作为基础。没有激活版本时，所有依赖版本的功能都会显示此引导页面。',
  },
  {
    question: '如何切换到其他版本？',
    answer: '在"排产方案"页面中，选择要激活的版本，点击"激活"按钮即可。系统会自动应用新版本的数据。',
  },
  {
    question: '激活版本会影响已有数据吗？',
    answer: '不会。激活版本只是改变当前工作版本，不会删除或修改任何已有数据。',
  },
];

// 说明列表
export const TIPS_LIST = [
  '一个方案可以包含多个版本，用于不同的排产方案对比',
  '同一时刻只能有一个版本处于"激活"状态',
  '激活版本后，所有排产和调度操作都基于该版本进行',
  '版本激活后，可随时切换到其他版本，无需重复创建',
];
