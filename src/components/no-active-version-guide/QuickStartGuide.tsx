/**
 * 快速开始指南组件
 */

import React from 'react';
import { Card, Divider, Steps, theme } from 'antd';
import {
  AppstoreOutlined,
  CheckCircleOutlined,
  NumberOutlined,
  ThunderboltOutlined,
  UploadOutlined,
} from '@ant-design/icons';
import type { StepItem } from './types';
import { TIPS_LIST } from './types';

export interface QuickStartGuideProps {
  showImportStep?: boolean;
}

export const QuickStartGuide: React.FC<QuickStartGuideProps> = ({ showImportStep }) => {
  const { token } = theme.useToken();

  const steps: StepItem[] = [
    ...(showImportStep
      ? [
          {
            title: '导入数据',
            description: '先导入材料/订单等基础数据（支持冲突处理与导入历史）',
            icon: <UploadOutlined />,
          },
        ]
      : []),
    {
      title: '创建排产方案',
      description: '在"排产方案"页面中点击"创建方案"按钮，输入方案名称',
      icon: <AppstoreOutlined />,
    },
    {
      title: '创建版本',
      description: '在方案中点击"创建版本"按钮，设置窗口天数',
      icon: <NumberOutlined />,
    },
    {
      title: '激活版本',
      description: '点击版本右侧的"激活"按钮，激活排产版本',
      icon: <CheckCircleOutlined />,
    },
    {
      title: '开始排产',
      description: '返回当前页面，系统会自动加载排产数据',
      icon: <ThunderboltOutlined />,
    },
  ];

  return (
    <Card
      title={
        <div style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
          <span>快速开始指南</span>
        </div>
      }
      variant="borderless"
    >
      <Steps
        direction="vertical"
        current={-1}
        items={steps.map((step) => ({
          title: (
            <div style={{ display: 'flex', alignItems: 'center', gap: 12 }}>
              <span style={{ fontSize: 16 }}>{step.icon}</span>
              <strong style={{ fontSize: 14 }}>{step.title}</strong>
            </div>
          ),
          description: (
            <p style={{ margin: '8px 0 0 28px', color: token.colorTextSecondary, fontSize: 13 }}>
              {step.description}
            </p>
          ),
        }))}
      />

      <Divider style={{ margin: '24px 0' }} />

      {/* 提示信息 */}
      <div
        style={{
          background: token.colorFillAlter,
          padding: 12,
          borderRadius: token.borderRadiusLG,
        }}
      >
        <div style={{ marginBottom: 8 }}>
          <strong>✨ 说明：</strong>
        </div>
        <ul style={{ margin: 0, paddingLeft: 20, lineHeight: 1.8 }}>
          {TIPS_LIST.map((tip, index) => (
            <li key={index}>{tip}</li>
          ))}
        </ul>
      </div>
    </Card>
  );
};

export default QuickStartGuide;
