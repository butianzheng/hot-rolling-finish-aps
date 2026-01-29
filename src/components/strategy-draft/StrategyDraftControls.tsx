/**
 * 策略草案控制卡片
 * 日期范围、策略选择、生成按钮等控制项
 */

import React from 'react';
import { Button, Card, Checkbox, DatePicker, Space, Tag, Typography } from 'antd';
import type { Dayjs } from 'dayjs';
import type { StrategyKey, StrategyPreset } from '../../types/strategy-draft';

const { RangePicker } = DatePicker;
const { Text } = Typography;

export interface StrategyDraftControlsProps {
  activeVersionId: string | null;
  range: [Dayjs, Dayjs];
  headerHint: string;
  strategies: StrategyPreset[];
  selectedStrategies: StrategyKey[];
  strategyTitleMap: Partial<Record<StrategyKey, string>>;
  canGenerate: boolean;
  isGenerating: boolean;
  onRangeChange: (range: [Dayjs, Dayjs]) => void;
  onSelectedStrategiesChange: (keys: StrategyKey[]) => void;
  onGenerate: () => void;
  onNavigateSettings: () => void;
  onNavigateHistorical: () => void;
  onNavigateWorkbench: () => void;
}

export const StrategyDraftControls: React.FC<StrategyDraftControlsProps> = ({
  activeVersionId,
  range,
  headerHint,
  strategies,
  selectedStrategies,
  strategyTitleMap,
  canGenerate,
  isGenerating,
  onRangeChange,
  onSelectedStrategiesChange,
  onGenerate,
  onNavigateSettings,
  onNavigateHistorical,
  onNavigateWorkbench,
}) => {
  return (
    <Card
      size="small"
      title={
        <Space>
          <span>策略草案对比</span>
          <Tag color="gold">草案</Tag>
        </Space>
      }
      extra={
        <Space>
          <Button size="small" onClick={onNavigateSettings}>
            策略配置
          </Button>
          <Button size="small" onClick={onNavigateHistorical}>
            去历史版本对比
          </Button>
          <Button size="small" onClick={onNavigateWorkbench}>
            返回工作台
          </Button>
        </Space>
      }
      style={{ marginBottom: 12 }}
    >
      <Space direction="vertical" style={{ width: '100%' }} size={10}>
        <Space wrap>
          <Text type="secondary">基准版本</Text>
          <Text code>{activeVersionId || '未激活'}</Text>
          {!activeVersionId && (
            <Button size="small" type="primary" onClick={onNavigateHistorical}>
              去激活版本
            </Button>
          )}
        </Space>

        <Space wrap>
          <Text type="secondary">计划范围</Text>
          <RangePicker
            value={range}
            onChange={(values) => {
              if (!values || !values[0] || !values[1]) return;
              onRangeChange([values[0], values[1]]);
            }}
            allowClear={false}
          />
          <Text type="secondary" style={{ fontSize: 12 }}>
            {headerHint}
          </Text>
        </Space>

        <Space wrap>
          <Text type="secondary">参与对比</Text>
          <Checkbox.Group
            value={selectedStrategies}
            onChange={(vals) => onSelectedStrategiesChange(vals as StrategyKey[])}
            options={strategies.map((s) => ({
              value: s.key,
              label: (
                <Space size={6}>
                  <span>{s.title}</span>
                  {s.kind === 'custom' ? <Tag color="gold">自定义</Tag> : null}
                  {s.kind === 'custom' && s.base_strategy ? (
                    <Tag color="blue">{strategyTitleMap[s.base_strategy] || s.base_strategy}</Tag>
                  ) : null}
                </Space>
              ),
            }))}
          />
        </Space>

        <Space wrap>
          <Button type="primary" disabled={!canGenerate} loading={isGenerating} onClick={onGenerate}>
            重新计算策略草案
          </Button>
          <Text type="secondary" style={{ fontSize: 12 }}>
            说明：草案为临时对象，不会写入数据库；确认后才生成正式版本。
          </Text>
          <Text type="secondary" style={{ fontSize: 12 }}>
            自定义策略：后端已支持"参数化排序"（仅影响等级内排序，不触碰冻结区/适温/产能硬约束）。
          </Text>
          <Text type="secondary" style={{ fontSize: 12 }}>
            KPI 说明：成熟/未成熟为计算过程统计；超限机组日按「机组×日期」统计。
          </Text>
        </Space>
      </Space>
    </Card>
  );
};

export default StrategyDraftControls;
