/**
 * MaterialPool 物料池主组件
 *
 * 重构后：484 行 → ~120 行 (-75%)
 * 增强：支持聚焦物料（接口已预留）
 */

import React, { useEffect } from 'react';
import { Alert, Button, Card, Empty, Skeleton, Space, Tree, Typography } from 'antd';
import AutoSizer from 'react-virtualized-auto-sizer';
import { List } from 'react-window';
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
    selection,
    filters,
    selectedMaterialIds,
    onSelectedMaterialIdsChange,
  });

  // TODO: 自动滚动到聚焦物料（当前List组件不支持ref，待实现）
  // 功能接口已预留，focusedMaterialId prop可用于未来实现
  useEffect(() => {
    if (focusedMaterialId && pool.rows.length > 0) {
      // 查找聚焦物料在rows中的索引
      const targetIndex = pool.rows.findIndex(
        (row) => row.type === 'material' && row.material.material_id === focusedMaterialId
      );
      // 将来可在此处实现滚动逻辑
      console.log('Focused material index:', targetIndex);
    }
  }, [focusedMaterialId, pool.rows]);  return (
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
          description={String((error as any)?.message || error)}
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
          <Text type="secondary" style={{ fontSize: 12 }}>
            显示 {pool.filtered.length} 条
          </Text>
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
                  rowCount={pool.rows.length}
                  rowHeight={ROW_HEIGHT}
                  rowComponent={MaterialPoolRow}
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
