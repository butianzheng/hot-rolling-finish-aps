/**
 * 复盘总结卡片
 */

import React from 'react';
import { Alert, Button, Card, Input, Space } from 'antd';
import type { BackendVersionComparisonResult } from '../../types/comparison';

interface RetrospectiveCardProps {
  compareResult: BackendVersionComparisonResult | null;
  retrospectiveNote: string;
  retrospectiveSavedAt?: string | null;
  onRetrospectiveNoteChange?: (note: string) => void;
  onRetrospectiveNoteSave?: () => void;
  onExportReport?: (format: 'json' | 'markdown' | 'html') => Promise<void>;
}

export const RetrospectiveCard: React.FC<RetrospectiveCardProps> = ({
  compareResult,
  retrospectiveNote,
  retrospectiveSavedAt,
  onRetrospectiveNoteChange,
  onRetrospectiveNoteSave,
  onExportReport,
}) => {
  return (
    <Card
      title="复盘总结"
      size="small"
      extra={
        <Space>
          <Button size="small" onClick={() => onRetrospectiveNoteSave?.()}>
            保存总结
          </Button>
          <Button size="small" onClick={() => onExportReport?.('json')}>
            导出报告(JSON)
          </Button>
          <Button size="small" onClick={() => onExportReport?.('markdown')} disabled={!compareResult}>
            导出报告(MD)
          </Button>
          <Button size="small" onClick={() => onExportReport?.('html')} disabled={!compareResult}>
            导出报告(HTML)
          </Button>
        </Space>
      }
    >
      <Space direction="vertical" style={{ width: '100%' }} size={8}>
        <Input.TextArea
          rows={5}
          value={retrospectiveNote}
          onChange={(e) => onRetrospectiveNoteChange?.(e.target.value)}
          placeholder="记录本次决策要点、代价与后续关注项（本地保存，不会写入数据库）。"
        />
        <Alert
          type="info"
          showIcon
          message={
            retrospectiveSavedAt ? `已保存（本地）：${retrospectiveSavedAt}` : '未保存（本地）'
          }
        />
      </Space>
    </Card>
  );
};
