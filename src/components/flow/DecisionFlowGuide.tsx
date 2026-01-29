import React from 'react';
import { Button, Card, Space, Steps, Typography } from 'antd';

const { Text } = Typography;

export type DecisionFlowStage = 'overview' | 'workbench' | 'comparison' | 'publish';

export type DecisionFlowAction = {
  label: string;
  onClick: () => void;
  disabled?: boolean;
  loading?: boolean;
};

export interface DecisionFlowGuideProps {
  stage: Exclude<DecisionFlowStage, 'publish'>;
  title: string;
  tags?: React.ReactNode;
  description?: React.ReactNode;
  primaryAction?: DecisionFlowAction;
  secondaryAction?: DecisionFlowAction;
}

function getCurrentIndex(stage: DecisionFlowGuideProps['stage']): number {
  if (stage === 'overview') return 0;
  if (stage === 'workbench') return 1;
  if (stage === 'comparison') return 2;
  return 0;
}

const FLOW_STEPS = [
  { title: '风险概览', description: '定位 P0/P1' },
  { title: '工作台处理', description: '移位/锁定/紧急' },
  { title: '策略草案对比', description: '生成+解释' },
  { title: '发布并激活', description: '生成版本' },
];

const DecisionFlowGuide: React.FC<DecisionFlowGuideProps> = ({
  stage,
  title,
  tags,
  description,
  primaryAction,
  secondaryAction,
}) => {
  const current = getCurrentIndex(stage);

  return (
    <Card size="small" style={{ marginBottom: 12 }}>
      <Space direction="vertical" size={8} style={{ width: '100%' }}>
        <Steps
          size="small"
          current={current}
          items={FLOW_STEPS}
        />

        <div style={{ display: 'flex', alignItems: 'flex-start', justifyContent: 'space-between', gap: 12 }}>
          <Space direction="vertical" size={4} style={{ minWidth: 0 }}>
            <Space wrap size={6}>
              <Text strong>{title}</Text>
              {tags}
            </Space>
            {description ? <Text type="secondary">{description}</Text> : null}
          </Space>

          {(primaryAction || secondaryAction) ? (
            <Space wrap>
              {secondaryAction ? (
                <Button
                  size="small"
                  onClick={secondaryAction.onClick}
                  disabled={secondaryAction.disabled}
                  loading={secondaryAction.loading}
                >
                  {secondaryAction.label}
                </Button>
              ) : null}
              {primaryAction ? (
                <Button
                  size="small"
                  type="primary"
                  onClick={primaryAction.onClick}
                  disabled={primaryAction.disabled}
                  loading={primaryAction.loading}
                >
                  {primaryAction.label}
                </Button>
              ) : null}
            </Space>
          ) : null}
        </div>
      </Space>
    </Card>
  );
};

export default DecisionFlowGuide;

