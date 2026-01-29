/**
 * 物料详情弹窗类型定义
 */

import type { Dayjs } from 'dayjs';
import type { ActionLogRow, MaterialDetailPayload, StrategyDraftDiffItem } from '../../types/strategy-draft';

export interface MaterialDetailModalProps {
  open: boolean;
  loading: boolean;
  context: StrategyDraftDiffItem | null;
  data: MaterialDetailPayload | null;
  error: string | null;
  logsLoading: boolean;
  logsError: string | null;
  logs: ActionLogRow[];
  range: [Dayjs, Dayjs];
  onClose: () => void;
  onGoWorkbench: (materialId: string) => void;
}
