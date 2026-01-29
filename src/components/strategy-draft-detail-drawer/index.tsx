import React from 'react';
import { Drawer, Space, Tag, Table } from 'antd';
import type { StrategyDraftDetailDrawerProps } from './types';
import { DraftSummaryInfo } from './DraftSummaryInfo';
import { TruncationAlert } from './TruncationAlert';
import { FilterAndSearch } from './FilterAndSearch';
import { useDiffTableColumns } from './useDiffTableColumns';

export const StrategyDraftDetailDrawer: React.FC<StrategyDraftDetailDrawerProps> = ({
  open,
  loading,
  draft,
  detailResp,
  detailItems,
  filter,
  search,
  strategyTitleMap,
  squeezedHintCache,
  range,
  onClose,
  onFilterChange,
  onSearchChange,
  onOpenMaterialDetail,
  onEnsureSqueezedHint,
}) => {
  const windowStart = range[0].format('YYYY-MM-DD');
  const windowEnd = range[1].format('YYYY-MM-DD');

  const columns = useDiffTableColumns(
    squeezedHintCache,
    windowStart,
    windowEnd,
    onOpenMaterialDetail,
    onEnsureSqueezedHint
  );

  return (
    <Drawer
      title={
        <Space>
          <span>变更明细</span>
          {draft ? <Tag color="blue">{strategyTitleMap[draft.strategy] || draft.strategy}</Tag> : null}
        </Space>
      }
      open={open}
      onClose={onClose}
      width={860}
      destroyOnClose
    >
      <Space direction="vertical" style={{ width: '100%' }} size={10}>
        <DraftSummaryInfo draft={draft} detailResp={detailResp} />
        <TruncationAlert detailResp={detailResp} />
        <FilterAndSearch
          filter={filter}
          search={search}
          loading={loading}
          itemCount={detailItems.length}
          onFilterChange={onFilterChange}
          onSearchChange={onSearchChange}
        />
        <Table
          size="small"
          rowKey={(r) => `${r.change_type}-${r.material_id}`}
          loading={loading}
          pagination={{ pageSize: 20, showSizeChanger: true }}
          dataSource={detailItems}
          scroll={{ x: 980 }}
          columns={columns}
        />
      </Space>
    </Drawer>
  );
};

export default StrategyDraftDetailDrawer;
