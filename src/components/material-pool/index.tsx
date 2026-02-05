/**
 * MaterialPool 物料池主组件
 *
 * 重构后：484 行 → ~120 行 (-75%)
 * 增强：支持聚焦物料（接口已预留）
 */

import React, { useEffect } from 'react';
import { Alert, Button, Card, Empty, Skeleton, Space, Tree, Typography } from 'antd';
import AutoSizer from 'react-virtualized-auto-sizer';
import { List, useListCallbackRef } from 'react-window';
import type { MaterialPoolProps } from './types';
import { ROW_HEIGHT } from './types';
import { parseTreeKey } from './utils';
import { useMaterialPool } from './useMaterialPool';
import { MaterialPoolToolbar } from './MaterialPoolToolbar';
import { MaterialPoolRow } from './MaterialPoolRow';

const { Text } = Typography;

const MaterialPool: React.FC<MaterialPoolProps> = ({
  materials,
  loading,
  error,
  onRetry,
  treeData,
  hasMore,
  loadingMore,
  onLoadMore,
  selection,
  onSelectionChange,
  filters,
  onFiltersChange,
  selectedMaterialIds,
  onSelectedMaterialIdsChange,
  onInspectMaterial,
  focusedMaterialId,
}) => {
  const pool = useMaterialPool({
    materials,
    treeDataOverride: treeData,
    selection,
    filters,
    selectedMaterialIds,
    onSelectedMaterialIdsChange,
  });

  const [listApi, listRef] = useListCallbackRef(null);

  // 自动滚动到聚焦物料
  useEffect(() => {
    if (focusedMaterialId && pool.rows.length > 0) {
      // 查找聚焦物料在rows中的索引
      const targetIndex = pool.rows.findIndex(
        (row) => row.type === 'material' && row.material.material_id === focusedMaterialId
      );
      if (targetIndex < 0) return;
      const el = listApi?.element;
      if (!el) return;
      const targetTop = Math.max(0, targetIndex * ROW_HEIGHT - ROW_HEIGHT);
      el.scrollTo({ top: targetTop, behavior: 'smooth' });
    }
  }, [focusedMaterialId, listApi, pool.rows]);

  return (
    <Card
      size="small"
      title="物料池"
      style={{ height: '100%' }}
      bodyStyle={{ height: '100%', display: 'flex', flexDirection: 'column', gap: 10 }}
      extra={
        <Space>
          <Button size="small" onClick={pool.selectAllVisible} disabled={pool.filtered.length === 0}>
            全选
          </Button>
          <Button size="small" onClick={pool.clearSelection} disabled={selectedMaterialIds.length === 0}>
            清空
          </Button>
        </Space>
      }
    >
      {error ? (
        <Alert
          type="error"
          showIcon
          message="物料池加载失败"
          description={error instanceof Error ? error.message : String(error)}
          action={
            onRetry ? (
              <Button size="small" onClick={onRetry}>
                重试
              </Button>
            ) : undefined
          }
        />
      ) : null}

      <MaterialPoolToolbar
        searchText={pool.searchText}
        onSearchChange={pool.setSearchText}
        loading={loading}
        filters={filters}
        onFiltersChange={onFiltersChange}
        groupByUrgency={pool.groupByUrgency}
        onGroupByUrgencyChange={pool.setGroupByUrgency}
      />

      <div style={{ flex: '0 0 220px', overflow: 'auto' }}>
        <Tree
          showLine={{ showLeafIcon: false }}
          treeData={pool.treeData}
          selectedKeys={pool.selectedTreeKey ? [pool.selectedTreeKey] : []}
          onSelect={(keys) => {
            const first = keys[0];
            if (!first) return;
            const next = parseTreeKey(String(first));
            onSelectionChange(next);
          }}
        />
      </div>

      <div style={{ flex: 1, minHeight: 260 }}>
        <Space style={{ width: '100%', justifyContent: 'space-between' }}>
          <Space size={8}>
            <Text type="secondary" style={{ fontSize: 12 }}>
              显示 {pool.filtered.length} 条
            </Text>
            {hasMore ? (
              <Button
                size="small"
                loading={!!loadingMore}
                disabled={!onLoadMore || !!loadingMore}
                onClick={onLoadMore}
              >
                加载更多
              </Button>
            ) : null}
          </Space>
          <Text type="secondary" style={{ fontSize: 12 }}>
            已选 {selectedMaterialIds.length} 条
          </Text>
        </Space>

        <div style={{ height: '100%', marginTop: 8 }}>
          {loading ? (
            <Skeleton active paragraph={{ rows: 8 }} />
          ) : pool.filtered.length === 0 ? (
            <div style={{ padding: 24 }}>
              <Empty
                description={
                  materials.length === 0
                    ? '暂无物料数据（请先在"数据导入"导入）'
                    : '当前筛选条件下暂无物料'
                }
              />
            </div>
          ) : (
            <AutoSizer>
              {({ height, width }) => (
                <List
                  listRef={listRef}
                  rowCount={pool.rows.length}
                  rowHeight={ROW_HEIGHT}
                  rowComponent={MaterialPoolRow as any} // React.memo 包装后类型与 react-window 不兼容
                  rowProps={{
                    rows: pool.rows,
                    selected: pool.selectedSet,
                    onToggle: pool.toggleSelection,
                    onInspect: onInspectMaterial,
                    onToggleUrgency: pool.toggleUrgencyCollapse,
                  }}
                  style={{ height, width }}
                >
                </List>
              )}
            </AutoSizer>
          )}
        </div>
      </div>
    </Card>
  );
};

export default React.memo(MaterialPool);
