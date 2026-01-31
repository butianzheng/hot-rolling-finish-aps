# 计划工作台联动重构 - 方案 C 实施指南

## 📋 目录

1. [重构概览](#重构概览)
2. [已完成的工作](#已完成的工作)
3. [待实施的修改](#待实施的修改)
4. [实施步骤](#实施步骤)
5. [风险评估](#风险评估)
6. [回滚方案](#回滚方案)

---

## 1. 重构概览

### 核心目标

解决计划工作台中物料池、产能概览、排程视图的联动失效问题，并提升长期可维护性。

### 架构变化

```
旧架构：
PlanningWorkbench.tsx (1546 行)
├─ 内部状态：useState × 30+
├─ MaterialPool (独立状态)
├─ CapacityOverview (独立状态)
└─ ScheduleView (独立状态)
   ↓ 问题：状态分散，联动困难

新架构：
PlanningWorkbench.tsx (简化版)
├─ useWorkbenchSync() (统一状态管理器)
│  ├─ state: 所有联动状态
│  └─ api: 统一操作接口
├─ MaterialPool (受控组件)
│  └─ syncApi props
├─ CapacityOverview (受控组件)
│  └─ syncApi props
└─ ScheduleView (受控组件)
   └─ syncApi props
   ↓ 优势：状态集中，联动清晰
```

---

## 2. 已完成的工作

### ✅ 创建的新文件

1. **src/hooks/useWorkbenchSync.ts** (主状态管理器)
   - 提供统一的联动状态管理
   - 支持撤销/重做功能
   - 包含调试模式

2. **src/components/capacity-timeline-container/index-v2.tsx** (产能概览改进版)
   - 支持选中物料高亮
   - 日期范围同步
   - 与 syncApi 集成

3. **src/components/workbench/WorkbenchDebugPanel.tsx** (调试面板)
   - 实时显示联动状态
   - 变化日志记录
   - 快捷测试按钮

### 核心功能

#### useWorkbenchSync API

```typescript
const [syncState, syncApi] = useWorkbenchSync();

// 机组选择
syncApi.selectMachine('H031');

// 物料选择
syncApi.selectMaterial(materialId, multiSelect);
syncApi.selectMaterials(['id1', 'id2'], replace);
syncApi.clearSelection();
syncApi.toggleMaterialSelection(materialId);

// 日期范围
syncApi.setDateRange([start, end]);
syncApi.resetDateRangeToAuto();

// 视图聚焦
await syncApi.focusMaterial(materialId, machineCode);
syncApi.focusMachine(machineCode);
syncApi.clearFocus();

// 历史操作
syncApi.undo();
syncApi.redo();

// 调试
syncApi.toggleDebugMode();
syncApi.getDebugInfo();
```

---

## 3. 待实施的修改

### 3.1 PlanningWorkbench.tsx 重构

**当前问题：**
- 状态分散在 30+ 个 useState
- 联动逻辑散落在各个回调函数中
- 难以追踪状态变化

**重构方案：**

```typescript
// 旧代码 (部分)
const [poolSelection, setPoolSelection] = useState<MaterialPoolSelection>({...});
const [selectedMaterialIds, setSelectedMaterialIds] = useState<string[]>([]);
const [workbenchFilters, setWorkbenchFilters] = useState({...});
// ... 30+ 个状态

// 新代码
const [syncState, syncApi] = useWorkbenchSync();

// 所有状态统一管理
const {
  machineCode,
  selectedMaterialIds,
  dateRange,
  focusedMaterialId,
  // ...
} = syncState;
```

**主要改动点：**

1. **移除冗余状态** (约 150 行)
```typescript
// ❌ 删除
const [poolSelection, setPoolSelection] = useState(...);
const [selectedMaterialIds, setSelectedMaterialIds] = useState([]);
const [workbenchFilters, setWorkbenchFilters] = useState({...});

// ✅ 改为
const [syncState, syncApi] = useWorkbenchSync();
```

2. **简化回调函数** (约 200 行)
```typescript
// ❌ 旧代码
const handleMachineChange = (machineCode) => {
  setPoolSelection({machineCode, ...});
  setWorkbenchFilters({machineCode});
  setSelectedMaterialIds([]);
  // ... 更多逻辑
};

// ✅ 新代码
const handleMachineChange = (machineCode) => {
  syncApi.selectMachine(machineCode);
};
```

3. **传递 syncApi 给子组件** (约 50 行)
```typescript
<MaterialPool
  syncApi={syncApi}
  syncState={syncState}
  // ... 其他 props
/>

<CapacityTimelineContainer
  syncApi={syncApi}
  syncState={syncState}
  machineCode={syncState.machineCode}
  dateRange={syncState.dateRange}
  selectedMaterialIds={syncState.selectedMaterialIds}
  // ...
/>

<ScheduleView
  syncApi={syncApi}
  syncState={syncState}
  // ...
/>
```

### 3.2 MaterialPool 改造

**文件：** `src/components/material-pool/index.tsx`

**改动：**

```typescript
interface MaterialPoolProps {
  // 新增
  syncApi?: WorkbenchSyncAPI;
  syncState?: WorkbenchSyncState;

  // 保留
  materials: Material[];
  // ...
}

export const MaterialPool: React.FC<MaterialPoolProps> = ({
  syncApi,
  syncState,
  materials,
  // ...
}) => {
  // 使用 syncApi 替代本地状态更新
  const handleMaterialSelect = (materialId: string, multiSelect: boolean) => {
    if (syncApi) {
      syncApi.selectMaterial(materialId, multiSelect);
    } else {
      // 降级为旧逻辑（兼容性）
      onSelectedMaterialIdsChange([materialId]);
    }
  };

  // 使用 syncState 读取选中状态
  const selectedSet = useMemo(() => {
    return new Set(syncState?.selectedMaterialIds || selectedMaterialIds);
  }, [syncState, selectedMaterialIds]);

  // ...
};
```

### 3.3 CapacityTimeline 添加高亮

**文件：** `src/components/CapacityTimeline.tsx`

**新增 Props：**
```typescript
interface CapacityTimelineProps {
  data: CapacityTimelineData[];

  // 新增
  selectedMaterialIds?: string[];
  focusedMaterialId?: string | null;
  onMaterialSelect?: (materialId: string, add: boolean) => void;
  onMaterialFocus?: (materialId: string) => void;
}
```

**渲染改动：**
```typescript
// 在渲染时间线条形图时
const cellMaterialIds = dayData.materials.map(m => m.material_id);
const hasSelectedMaterial = selectedMaterialIds.some(id => cellMaterialIds.includes(id));
const isFocused = focusedMaterialId && cellMaterialIds.includes(focusedMaterialId);

<div
  className="capacity-cell"
  style={{
    border: hasSelectedMaterial ? '2px solid #1890ff' : '1px solid #e8e8e8',
    boxShadow: isFocused ? '0 0 8px rgba(24, 144, 255, 0.6)' : 'none',
    backgroundColor: hasSelectedMaterial ? 'rgba(24, 144, 255, 0.05)' : '#fff',
  }}
  onClick={() => {
    // 点击单元格时选中该单元格的所有物料
    if (onMaterialsSelect) {
      onMaterialsSelect(cellMaterialIds, false);
    }
  }}
>
  {/* 条形图内容 */}
</div>
```

### 3.4 ScheduleView 聚焦滚动

**文件：** `src/components/schedule-card-view/index.tsx`

**添加自动滚动逻辑：**
```typescript
import { useWorkbenchFocusListener } from '@/hooks/useWorkbenchSync';

export const ScheduleCardView: React.FC<ScheduleCardViewProps> = ({
  syncApi,
  focusedMaterialId,
  // ...
}) => {
  const listRef = useRef<VariableSizeList>(null);
  const rowRefs = useRef<Map<string, HTMLDivElement>>(new Map());

  // 监听聚焦事件，自动滚动到对应物料
  useWorkbenchFocusListener((materialId, machineCode) => {
    // 找到物料在列表中的索引
    const rowIndex = filteredItems.findIndex(item =>
      item.material_id === materialId
    );

    if (rowIndex >= 0 && listRef.current) {
      // 滚动到可见区域
      listRef.current.scrollToItem(rowIndex, 'center');

      // 高亮动画
      setTimeout(() => {
        const rowElement = rowRefs.current.get(materialId);
        if (rowElement) {
          rowElement.style.animation = 'highlight-flash 1s ease';
        }
      }, 300);
    }
  });

  // ...
};
```

**添加 CSS 动画：**
```css
@keyframes highlight-flash {
  0%, 100% {
    background-color: transparent;
  }
  50% {
    background-color: rgba(24, 144, 255, 0.2);
  }
}
```

### 3.5 Optimistic Update 实现

**文件：** `src/pages/PlanningWorkbench.tsx`

**旧代码：**
```typescript
const submitBatchLock = async () => {
  await materialApi.batchLockMaterials(selectedMaterialIds, operator, reason);
  message.success(`成功锁定 ${selectedMaterialIds.length} 个物料`);
  setRefreshSignal((v) => v + 1);  // ← 全局刷新，会闪烁
  setSelectedMaterialIds([]);
};
```

**新代码：**
```typescript
import { useMutation, useQueryClient } from '@tanstack/react-query';

const queryClient = useQueryClient();

const lockMutation = useMutation({
  mutationFn: (ids: string[]) => materialApi.batchLockMaterials(ids, operator, reason),

  onMutate: async (ids) => {
    // 1. 取消正在进行的查询
    await queryClient.cancelQueries({ queryKey: ['materials'] });

    // 2. 获取当前缓存
    const previousMaterials = queryClient.getQueryData<Material[]>(['materials']);

    // 3. 乐观更新缓存
    queryClient.setQueryData<Material[]>(['materials'], (old) =>
      old?.map(m => ids.includes(m.material_id) ? { ...m, is_locked: true } : m) || []
    );

    return { previousMaterials };
  },

  onError: (err, ids, context) => {
    // 4. 出错时回滚
    queryClient.setQueryData(['materials'], context?.previousMaterials);
    message.error('锁定失败');
  },

  onSuccess: (data, ids) => {
    message.success(`成功锁定 ${ids.length} 个物料`);
    syncApi.clearSelection();
  },

  onSettled: () => {
    // 5. 后台重新验证
    queryClient.invalidateQueries({ queryKey: ['materials'] });
  }
});

// 使用
const submitBatchLock = () => {
  lockMutation.mutate(syncState.selectedMaterialIds);
};
```

---

## 4. 实施步骤

### 第一阶段：基础集成（1-2 小时）

1. ✅ 创建 useWorkbenchSync.ts
2. ✅ 创建 WorkbenchDebugPanel.tsx
3. ⏳ 在 PlanningWorkbench.tsx 中引入 useWorkbenchSync
4. ⏳ 添加调试面板到页面（开发模式）
5. ⏳ 验证状态同步是否正常

### 第二阶段：组件改造（3-4 小时）

6. ⏳ 改造 MaterialPool 组件
7. ⏳ 改造 CapacityTimelineContainer 组件
8. ⏳ 改造 ScheduleCardView 组件
9. ⏳ 改造 ScheduleGanttView 组件
10. ⏳ 验证联动是否生效

### 第三阶段：高级功能（4-5 小时）

11. ⏳ 实现视图聚焦（自动滚动）
12. ⏳ 实现 Optimistic Update
13. ⏳ 添加撤销/重做快捷键（Ctrl+Z / Ctrl+Y）
14. ⏳ 添加日期范围自动计算
15. ⏳ 验证所有功能

### 第四阶段：测试与优化（2-3 小时）

16. ⏳ 编写单元测试
17. ⏳ 集成测试
18. ⏳ 性能优化
19. ⏳ 文档更新

---

## 5. 风险评估

### 高风险点

1. **状态迁移不完整**
   - 风险：旧状态和新状态混用，导致不一致
   - 缓解：分阶段迁移，保留降级逻辑

2. **React Query 缓存冲突**
   - 风险：Optimistic Update 与自动 refetch 冲突
   - 缓解：使用 `cancelQueries` 和 `onSettled`

3. **性能回退**
   - 风险：状态集中导致不必要的重新渲染
   - 缓解：使用 useMemo、useCallback 优化

### 中风险点

4. **快捷键冲突**
   - 风险：Ctrl+Z 与浏览器默认行为冲突
   - 缓解：仅在工作台页面激活快捷键

5. **调试面板性能影响**
   - 风险：日志记录影响性能
   - 缓解：仅开发模式启用

---

## 6. 回滚方案

### 如果出现严重问题

1. **立即回滚**
```bash
git revert <commit-hash>
git push
```

2. **保留调试工具**
- 调试面板可以独立使用，不影响现有功能
- useWorkbenchSync 可以渐进式集成

3. **分支策略**
```bash
# 在新分支开发
git checkout -b feature/workbench-sync-refactor

# 完成后合并到 main
git checkout main
git merge feature/workbench-sync-refactor
```

---

## 7. 验收标准

### 功能测试

- [ ] 机组选择：MaterialPool → CapacityOverview + ScheduleView 同步
- [ ] 物料选择：三个视图的复选框状态同步
- [ ] 日期范围：CapacityOverview 和 GanttView 显示一致的日期
- [ ] 高亮显示：选中物料在 CapacityOverview 中高亮
- [ ] 自动滚动：选中物料后，ScheduleView 自动滚动到可见区域
- [ ] 批量操作：锁定/解锁后无闪烁，立即更新
- [ ] 撤销/重做：Ctrl+Z 和 Ctrl+Y 正常工作

### 性能测试

- [ ] 初始加载时间 < 2s
- [ ] 选中物料响应时间 < 100ms
- [ ] 机组切换响应时间 < 200ms
- [ ] 无内存泄漏（长时间使用）

### 代码质量

- [ ] TypeScript 无 any 类型
- [ ] 所有 Hook 有正确的依赖
- [ ] 无 React Warning
- [ ] 单元测试覆盖率 > 80%

---

## 8. 下一步行动

### 建议

鉴于方案 C 的复杂度，建议：

**选项 1：完整实施方案 C**
- 优点：长期收益最大
- 缺点：开发周期较长（2-3 天）
- 适合：有充足时间，追求长期可维护性

**选项 2：分阶段实施**
- 第一阶段：仅实施基础联动修复（方案 A）
- 第二阶段：逐步引入 useWorkbenchSync
- 第三阶段：完善高级功能
- 优点：风险可控，渐进式改进
- 适合：时间有限，需要快速见效

**选项 3：混合方案**
- 立即实施方案 A 的核心修复（2-3 小时）
- 同时引入调试面板（便于后续优化）
- 预留接口，为后续重构做准备
- 优点：兼顾短期和长期目标
- 适合：当前最推荐

---

## 附录：关键代码片段

### A. PlanningWorkbench 集成示例

```typescript
import { useWorkbenchSync } from '@/hooks/useWorkbenchSync';
import { WorkbenchDebugPanel } from '@/components/workbench/WorkbenchDebugPanel';

export const PlanningWorkbench: React.FC = () => {
  // 使用统一状态管理器
  const [syncState, syncApi] = useWorkbenchSync();

  // 其他状态（非联动相关）
  const [inspectorOpen, setInspectorOpen] = useState(false);
  // ...

  return (
    <div className="planning-workbench">
      <MaterialPool
        syncApi={syncApi}
        machineCode={syncState.machineCode}
        selectedMaterialIds={syncState.selectedMaterialIds}
        onMachineChange={(code) => syncApi.selectMachine(code)}
        onMaterialSelect={(id, multi) => syncApi.selectMaterial(id, multi)}
      />

      <CapacityTimelineContainer
        syncApi={syncApi}
        machineCode={syncState.machineCode}
        dateRange={syncState.dateRange}
        selectedMaterialIds={syncState.selectedMaterialIds}
      />

      <ScheduleView
        syncApi={syncApi}
        machineCode={syncState.machineCode}
        selectedMaterialIds={syncState.selectedMaterialIds}
        focusedMaterialId={syncState.focusedMaterialId}
      />

      {/* 调试面板（仅开发模式） */}
      {process.env.NODE_ENV === 'development' && (
        <WorkbenchDebugPanel
          syncState={syncState}
          syncApi={syncApi}
        />
      )}
    </div>
  );
};
```

---

**总结：** 方案 C 的完整代码约 2000+ 行，建议您先审阅本指南，决定是否继续实施。
