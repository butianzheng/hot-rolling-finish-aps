/**
 * 策略草案变更明细抽屉类型定义
 */

import type { Dayjs } from 'dayjs';
import type {
  GetStrategyDraftDetailResponse,
  SqueezedHintCache,
  StrategyDraftDiffItem,
  StrategyDraftSummary,
  StrategyKey,
} from '../../types/strategy-draft';

export interface StrategyDraftDetailDrawerProps {
  open: boolean;
  loading: boolean;
  draft: StrategyDraftSummary | null;
  detailResp: GetStrategyDraftDetailResponse | null;
  detailItems: StrategyDraftDiffItem[];
  filter: 'ALL' | 'MOVED' | 'ADDED' | 'SQUEEZED_OUT';
  search: string;
  strategyTitleMap: Partial<Record<StrategyKey, string>>;
  squeezedHintCache: SqueezedHintCache;
  range: [Dayjs, Dayjs];
  onClose: () => void;
  onFilterChange: (filter: 'ALL' | 'MOVED' | 'ADDED' | 'SQUEEZED_OUT') => void;
  onSearchChange: (search: string) => void;
  onOpenMaterialDetail: (row: StrategyDraftDiffItem) => void;
  onEnsureSqueezedHint: (materialId: string) => void;
}
