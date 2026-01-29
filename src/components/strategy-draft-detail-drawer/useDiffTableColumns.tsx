import { useMemo } from 'react';
import { Button, Typography } from 'antd';
import type { ColumnsType } from 'antd/es/table';
import type { StrategyDraftDiffItem, SqueezedHintCache } from '../../types/strategy-draft';
import { formatPosition, prettyReason } from '../../utils/strategyDraftFormatters';
import { ChangeTypeRenderer } from './ChangeTypeRenderer';

const { Text } = Typography;

export const useDiffTableColumns = (
  squeezedHintCache: SqueezedHintCache,
  windowStart: string,
  windowEnd: string,
  onOpenMaterialDetail: (row: StrategyDraftDiffItem) => void,
  onEnsureSqueezedHint: (materialId: string) => void,
): ColumnsType<StrategyDraftDiffItem> => {
  return useMemo(() => {
    return [
      {
        title: '变更',
        dataIndex: 'change_type',
        width: 90,
        render: (_: any, r: StrategyDraftDiffItem) => (
          <ChangeTypeRenderer
            row={r}
            squeezedHintCache={squeezedHintCache}
            windowStart={windowStart}
            windowEnd={windowEnd}
            onEnsureSqueezedHint={onEnsureSqueezedHint}
          />
        ),
      },
      {
        title: 'Material ID',
        dataIndex: 'material_id',
        width: 180,
        render: (_: any, r: StrategyDraftDiffItem) => (
          <Button
            type="link"
            size="small"
            style={{ padding: 0, height: 'auto' }}
            onClick={() => onOpenMaterialDetail(r)}
          >
            <Text code>{String(r?.material_id || '')}</Text>
          </Button>
        ),
      },
      {
        title: 'From',
        key: 'from',
        width: 240,
        render: (_: any, r: StrategyDraftDiffItem) => (
          <Text>{formatPosition(r.from_plan_date, r.from_machine_code, r.from_seq_no)}</Text>
        ),
      },
      {
        title: 'To',
        key: 'to',
        width: 240,
        render: (_: any, r: StrategyDraftDiffItem) => (
          <Text>{formatPosition(r.to_plan_date, r.to_machine_code, r.to_seq_no)}</Text>
        ),
      },
      {
        title: '草案原因',
        key: 'reason',
        width: 220,
        render: (_: any, r: StrategyDraftDiffItem) => {
          const reason = prettyReason(r.to_assign_reason);
          const display = !reason ? '—' : reason.includes('\n') ? `${reason.split('\n')[0]} …` : reason;
          return (
            <Text
              ellipsis={{
                tooltip: reason ? (
                  <pre
                    style={{
                      margin: 0,
                      maxWidth: 640,
                      whiteSpace: 'pre-wrap',
                      fontSize: 12,
                    }}
                  >
                    {reason}
                  </pre>
                ) : (
                  ''
                ),
              }}
              copyable={reason ? { text: reason } : false}
            >
              {display}
            </Text>
          );
        },
      },
    ];
  }, [squeezedHintCache, windowStart, windowEnd, onOpenMaterialDetail, onEnsureSqueezedHint]);
};
