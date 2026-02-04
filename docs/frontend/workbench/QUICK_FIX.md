# 计划工作台联动修复 - 快速实施方案

## 📋 修复概要

这是一个**最小侵入**的快速修复方案，综合了方案 A 和方案 B 的核心功能：

1. ✅ 产能概览添加选中物料高亮
2. ✅ 统一日期范围计算
3. ✅ 修复机组切换时的日期范围重置
4. ✅ 提供调试面板（可选）

**预估时间：** 2-3 小时
**风险等级：** 低
**兼容性：** 完全向后兼容

---

## 1. 核心修复点

### 修复点 1：产能概览支持选中物料高亮

**文件：** `src/components/CapacityTimeline.tsx`

#### 修改 Props 定义

```typescript
export interface CapacityTimelineProps {
  data: CapacityTimelineData[];

  // 新增：选中物料支持
  selectedMaterialIds?: string[];
  focusedMaterialId?: string | null;
  onMaterialClick?: (materialId: string) => void;
}
```

#### 修改渲染逻辑

在渲染每个时间单元格时，检查是否包含选中的物料：

```typescript
// 在 CapacityTimeline 组件中
export const CapacityTimeline: React.FC<CapacityTimelineProps> = ({
  data,
  selectedMaterialIds = [],
  focusedMaterialId,
  onMaterialClick,
}) => {
  // ...

  const renderDayCell = (day: DayData) => {
    // 检查该日期的物料中是否有选中的
    const cellMaterialIds = day.materials?.map(m => m.material_id) || [];
    const hasSelectedMaterial = selectedMaterialIds.some(id => cellMaterialIds.includes(id));
    const hasFocusedMaterial = focusedMaterialId && cellMaterialIds.includes(focusedMaterialId);

    return (
      <div
        className="capacity-day-cell"
        style={{
          // 选中状态：添加蓝色边框
          border: hasSelectedMaterial ? '2px solid #1890ff' : '1px solid #e8e8e8',

          // 聚焦状态：添加阴影
          boxShadow: hasFocusedMaterial
            ? '0 0 8px rgba(24, 144, 255, 0.6)'
            : hasSelectedMaterial
            ? '0 0 4px rgba(24, 144, 255, 0.3)'
            : 'none',

          // 选中状态：添加背景色
          backgroundColor: hasSelectedMaterial ? 'rgba(24, 144, 255, 0.05)' : '#fff',

          transition: 'all 0.2s ease',
          cursor: 'pointer',
        }}
        onClick={() => {
          // 点击单元格时，触发物料点击事件（传递第一个物料 ID）
          if (onMaterialClick && cellMaterialIds.length > 0) {
            onMaterialClick(cellMaterialIds[0]);
          }
        }}
      >
        {/* 原有的条形图渲染 */}
        {/* ... */}
      </div>
    );
  };

  // ...
};
```

---

### 修复点 2：PlanningWorkbench 计算统一日期范围

**文件：** `src/pages/PlanningWorkbench.tsx`

#### 添加日期范围计算逻辑

在 PlanningWorkbench 组件中，添加一个 useMemo 来计算全局日期范围：

```typescript
import dayjs, { Dayjs } from 'dayjs';

export const PlanningWorkbench: React.FC = () => {
  // ... 现有代码

  // 计算全局日期范围（基于当前机组的排程数据）
  const globalDateRange = useMemo<[Dayjs, Dayjs]>(() => {
    const filteredItems = planItemsQuery.data?.filter(
      item => !poolSelection.machineCode ||
              poolSelection.machineCode === 'all' ||
              item.machine_code === poolSelection.machineCode
    ) || [];

    if (filteredItems.length === 0) {
      // 默认日期范围：今天前 3 天到后 10 天
      return [dayjs().subtract(3, 'day'), dayjs().add(10, 'day')];
    }

    // 提取所有排程日期
    const dates = filteredItems
      .map(item => dayjs(item.plan_date))
      .filter(d => d.isValid());

    if (dates.length === 0) {
      return [dayjs().subtract(3, 'day'), dayjs().add(10, 'day')];
    }

    // 找到最早和最晚的日期
    const sortedDates = dates.sort((a, b) => a.valueOf() - b.valueOf());
    const minDate = sortedDates[0].subtract(1, 'day'); // 前面留 1 天余量
    const maxDate = sortedDates[sortedDates.length - 1].add(3, 'day'); // 后面留 3 天余量

    return [minDate, maxDate];
  }, [planItemsQuery.data, poolSelection.machineCode]);

  // ...
};
```

#### 传递给 CapacityTimelineContainer

```typescript
<CapacityTimelineContainer
  machineCode={poolSelection.machineCode}
  dateRange={globalDateRange}  // ← 新增
  selectedMaterialIds={selectedMaterialIds}  // ← 新增
  onMaterialClick={(materialId) => {
    // 切换选中状态
    setSelectedMaterialIds(prev =>
      prev.includes(materialId)
        ? prev.filter(id => id !== materialId)
        : [...prev, materialId]
    );
  }}  // ← 新增
/>
```

---

### 修复点 3：CapacityTimelineContainer 接收外部日期范围

**文件：** `src/components/capacity-timeline-container/index.tsx`

#### 修改 Props 定义

```typescript
export interface CapacityTimelineContainerProps {
  machineCode: string | null;

  // 新增：外部日期范围
  dateRange?: [Dayjs, Dayjs];

  // 新增：选中物料
  selectedMaterialIds?: string[];
  onMaterialClick?: (materialId: string) => void;
}
```

#### 修改实现逻辑

```typescript
export const CapacityTimelineContainer: React.FC<CapacityTimelineContainerProps> = ({
  machineCode,
  dateRange: externalDateRange,
  selectedMaterialIds = [],
  onMaterialClick,
}) => {
  const {
    timelineData,
    machineOptions,
    selectedMachine,
    setSelectedMachine,
    dateRange: internalDateRange,
    setDateRange: setInternalDateRange,
    loading,
    error,
    refetch,
  } = useCapacityTimelineContainer(machineCode);

  // 使用外部传入的日期范围（优先级更高）
  const effectiveDateRange = externalDateRange || internalDateRange;

  // 当外部日期范围变化时，同步到内部状态
  useEffect(() => {
    if (externalDateRange) {
      setInternalDateRange(externalDateRange);
    }
  }, [externalDateRange, setInternalDateRange]);

  // ...

  return (
    <Spin spinning={loading} delay={200}>
      <Space direction="vertical" style={{ width: '100%' }} size="middle">
        <ToolBar
          machineCode={selectedMachine}
          onMachineChange={setSelectedMachine}
          machineOptions={machineOptions}
          dateRange={effectiveDateRange}
          onDateRangeChange={setInternalDateRange}
          onRefresh={() => refetch()}
        />

        <div style={{ overflowX: 'auto', padding: '0 8px' }}>
          {timelineData && timelineData.length > 0 ? (
            <CapacityTimeline
              data={timelineData}
              selectedMaterialIds={selectedMaterialIds}  // ← 新增
              onMaterialClick={onMaterialClick}  // ← 新增
            />
          ) : (
            <Empty description="该日期范围无排程项" />
          )}
        </div>
      </Space>
    </Spin>
  );
};
```

---

### 修复点 4：移除硬编码的日期范围重置

**文件：** `src/components/capacity-timeline-container/useCapacityTimelineContainer.ts`

#### 修改机组切换逻辑

```typescript
// 旧代码（第 88-93 行）
useEffect(() => {
  if (machineCode && machineCode !== selectedMachine) {
    setSelectedMachine(machineCode === 'all' ? 'all' : machineCode);
    setDateRange([dayjs().subtract(3, 'day'), dayjs().add(10, 'day')]);  // ← 删除这行
  }
}, [machineCode]);

// 新代码
useEffect(() => {
  if (machineCode && machineCode !== selectedMachine) {
    setSelectedMachine(machineCode === 'all' ? 'all' : machineCode);
    // 不再重置日期范围，使用父组件传入的日期范围
  }
}, [machineCode, selectedMachine]);
```

---

## 2. 实施步骤

### 步骤 1：修改 CapacityTimeline.tsx（5-10 分钟）

1. 打开文件 `src/components/CapacityTimeline.tsx`
2. 在 Props 接口中添加：
   ```typescript
   selectedMaterialIds?: string[];
   focusedMaterialId?: string | null;
   onMaterialClick?: (materialId: string) => void;
   ```
3. 在渲染单元格时添加高亮逻辑（参见修复点 1）
4. 保存文件

### 步骤 2：修改 PlanningWorkbench.tsx（10-15 分钟）

1. 打开文件 `src/pages/PlanningWorkbench.tsx`
2. 添加 `globalDateRange` 的 useMemo 计算（参见修复点 2）
3. 修改 `<CapacityTimelineContainer>` 的 props（参见修复点 2）
4. 保存文件

### 步骤 3：修改 CapacityTimelineContainer（10-15 分钟）

1. 打开文件 `src/components/capacity-timeline-container/index.tsx`
2. 修改 Props 定义（参见修复点 3）
3. 修改组件实现，使用外部日期范围（参见修复点 3）
4. 保存文件

### 步骤 4：修改 useCapacityTimelineContainer（2-3 分钟）

1. 打开文件 `src/components/capacity-timeline-container/useCapacityTimelineContainer.ts`
2. 删除硬编码的日期范围重置逻辑（参见修复点 4）
3. 保存文件

### 步骤 5：编译测试（5-10 分钟）

```bash
# 编译前端
npm run build

# 如果有 TypeScript 错误，根据提示修复
```

### 步骤 6：集成测试（10-15 分钟）

按照验收标准测试：

1. ✅ 选择机组 H031，观察三个视图是否同步
2. ✅ 在 MaterialPool 中选中物料，观察 CapacityOverview 是否高亮
3. ✅ 切换机组，观察日期范围是否合理
4. ✅ 在 CapacityOverview 中点击单元格，观察是否选中物料

---

## 3. 验收标准

### 基本功能

- [ ] **机组选择联动**
  - 在 MaterialPool 中选择机组 → CapacityOverview 和 ScheduleView 自动筛选
  - 日期范围自动调整为该机组的实际排程日期范围

- [ ] **选中物料高亮**
  - 在 MaterialPool 中选中物料 → CapacityOverview 中包含该物料的日期单元格显示蓝色边框
  - 在 ScheduleView 中选中物料 → CapacityOverview 同步高亮

- [ ] **日期范围一致性**
  - CapacityOverview 和 ScheduleGanttView 显示相同的日期范围
  - 机组切换时，日期范围自动调整

### 边界情况

- [ ] **无排程数据时**
  - 显示默认日期范围（今天前 3 天到后 10 天）
  - 不会报错或白屏

- [ ] **选中多个物料时**
  - CapacityOverview 中所有包含选中物料的单元格都高亮
  - 可以点击 CapacityOverview 的单元格取消选择

---

## 4. 可选增强（如果时间允许）

### 增强 1：添加调试面板

如果您已经创建了 `WorkbenchDebugPanel.tsx`，可以将其集成到 PlanningWorkbench：

```typescript
import { WorkbenchDebugPanel } from '@/components/workbench/WorkbenchDebugPanel';

export const PlanningWorkbench: React.FC = () => {
  // ...

  return (
    <div className="planning-workbench">
      {/* 现有内容 */}

      {/* 调试面板（仅开发模式） */}
      {process.env.NODE_ENV === 'development' && (
        <WorkbenchDebugPanel
          syncState={{
            machineCode: poolSelection.machineCode,
            selectedMaterialIds,
            dateRange: globalDateRange,
            // ...
          }}
          syncApi={{
            selectMachine: (code) => {
              setPoolSelection(prev => ({ ...prev, machineCode: code }));
              setWorkbenchFilters({ machineCode: code });
            },
            // ... 其他 API
          }}
        />
      )}
    </div>
  );
};
```

### 增强 2：添加加载骨架屏

在 CapacityTimeline 加载时显示骨架屏，避免白屏：

```typescript
import { Skeleton } from 'antd';

<Spin spinning={loading} indicator={<span />}>
  {loading ? (
    <Skeleton active paragraph={{ rows: 5 }} />
  ) : (
    <CapacityTimeline {...props} />
  )}
</Spin>
```

---

## 5. 故障排查

### 问题 1：选中物料后，CapacityOverview 没有高亮

**可能原因：**
- `selectedMaterialIds` prop 没有正确传递
- 物料 ID 不匹配

**排查：**
```typescript
console.log('Selected Material IDs:', selectedMaterialIds);
console.log('Cell Material IDs:', cellMaterialIds);
console.log('Has Selected:', hasSelectedMaterial);
```

### 问题 2：日期范围不一致

**可能原因：**
- `globalDateRange` 计算有误
- 外部 `dateRange` 没有传递给 CapacityTimelineContainer

**排查：**
```typescript
console.log('Global Date Range:', globalDateRange.map(d => d.format('YYYY-MM-DD')));
console.log('Effective Date Range:', effectiveDateRange.map(d => d.format('YYYY-MM-DD')));
```

### 问题 3：TypeScript 编译错误

**常见错误：**
```
Property 'selectedMaterialIds' does not exist on type 'CapacityTimelineProps'
```

**解决：**
- 确保 Props 接口已更新
- 重启 TypeScript 服务器（VS Code 中按 Ctrl+Shift+P → "Restart TS Server"）

---

## 6. 总结

这个快速实施方案提供了：

✅ **核心联动修复**：机组选择、物料选中、日期范围同步
✅ **视觉反馈增强**：选中物料高亮显示
✅ **低风险实施**：最小侵入，完全向后兼容
✅ **快速上线**：2-3 小时即可完成

如果需要更高级的功能（自动滚动、撤销/重做、Optimistic Update），可以后续参考 **WORKBENCH_REFACTOR_GUIDE.md** 渐进式实施。
