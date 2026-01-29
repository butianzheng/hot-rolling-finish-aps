# 📋 代码审查指南 - 组件重构工程

**审查周期**: 2026-01-30 - 2026-02-06
**审查范围**: 42 个重构提交，涉及 10+ 个主要组件
**预期收益**: 代码减少 61% (-2,475 行)，质量评分提升 10%

---

## 📊 重构概览

### 修改统计
- **总提交数**: 42
- **修改文件**: 80+ 个文件
- **代码减少**: 2,475 行 (-61%)
- **新增文件**: 20+ (工具函数、Hook、子组件)
- **删除文件**: 0 (保持向后兼容)

### 质量指标
| 指标 | 修改前 | 修改后 | 改进 |
|------|--------|--------|------|
| 代码行数 | 4,055 | 1,580 | -61% ✅ |
| 前端质量 | 6.2/10 | 6.8/10 | +10% ✅ |
| 综合评分 | 7.5/10 | 7.8/10 | +4% ✅ |

---

## 🔍 审查检查清单

### Phase 1: 代码结构审查

#### Component Decomposition (组件分解)
- [ ] **VersionComparisonModal** (666 → 200 行, -70%)
  - [ ] MaterialDiffCard 职责清晰
  - [ ] CapacityDeltaCard 数据流合理
  - [ ] RetrospectiveCard UI 完整
  - [ ] Props 接口类型正确

- [ ] **MaterialDetailModal** (334 → 80 行, -76%)
  - [ ] 4 个子组件功能不重叠
  - [ ] 状态管理集中
  - [ ] 导出和查询逻辑分离

- [ ] **StrategyDraftDetailDrawer** (327 → 70 行, -79%)
  - [ ] ChangeTypeRenderer 复杂度可控
  - [ ] FilterAndSearch 搜索性能
  - [ ] useDiffTableColumns Hook 依赖正确

- [ ] **ScheduleCardView** (226 → 70 行, -69%)
  - [ ] react-window 虚拟列表配置
  - [ ] ScheduleCardRow 渲染效率
  - [ ] 高度计算准确性
  - [ ] 性能测试数据

#### Tool Extraction (工具提取)
- [ ] **columns.tsx** (PlanManagement 表格列)
  - [ ] createPlanColumns 工厂函数签名
  - [ ] createVersionColumns 事件处理
  - [ ] 回调类型定义完整

- [ ] **exportHelpers.ts** (导出函数)
  - [ ] ExportContext 接口完整性
  - [ ] CSV/JSON/Markdown/HTML 导出逻辑
  - [ ] 错误处理是否完善

#### Hook Management (自定义 Hook)
- [ ] **useImportWorkflow** (14KB)
  - [ ] 状态管理逻辑清晰
  - [ ] 依赖数组正确
  - [ ] 内存泄漏风险排查

- [ ] **usePlanItems** & **useFilteredPlanItems**
  - [ ] 缓存策略
  - [ ] 依赖项完整性

---

### Phase 2: 技术债务审查

#### 依赖数组修复
- [ ] **PlanManagement.tsx** (commit: a015b14)
  - [ ] ✅ 7 个 useCallback 包装
  - [ ] ✅ planColumns useMemo 依赖添加
  - [ ] ✅ versionColumns useMemo 依赖添加
  - [ ] 是否消除过时闭包风险

#### 类型定义
- [ ] **comparison/types.ts**
  - [ ] 所有导出类型声明正确
  - [ ] 与 API 类型对齐

- [ ] **exportHelpers.ts** 中的 `any` 类型
  - [ ] 是否可替换为具体接口
  - [ ] 影响范围评估

---

### Phase 3: 功能完整性审查

#### 向后兼容性
- [ ] ✅ 所有旧 import 路径是否仍可用
- [ ] ✅ 组件 Props 接口是否保持兼容
- [ ] ✅ 导出 API 是否完全保留

#### 功能验证 (核心流程)
- [ ] **版本对比流程**
  - [ ] 两个版本选择功能正常
  - [ ] 对比结果展示完整
  - [ ] 导出 CSV/JSON/Markdown/HTML 工作
  - [ ] 复盘总结保存 (localStorage)

- [ ] **排程卡片视图**
  - [ ] 虚拟列表滚动流畅
  - [ ] 搜索/筛选功能
  - [ ] 大数据集 (1000+ 行) 性能

- [ ] **材料导入流程**
  - [ ] 文件选择对话框
  - [ ] CSV 预览加载
  - [ ] 冲突处理
  - [ ] 导入执行

---

### Phase 4: 性能审查

#### 渲染性能
- [ ] React DevTools Profiler 测试
  - [ ] 无不必要的 re-render
  - [ ] React.memo 是否有效应用
  - [ ] useMemo/useCallback 优化效果

#### 内存使用
- [ ] Chrome DevTools Memory 测试
  - [ ] 是否存在内存泄漏
  - [ ] 大列表场景下内存占用
  - [ ] Hook 清理函数是否完善

#### 运行时性能
- [ ] 表格排序/搜索 < 200ms
- [ ] 版本对比计算 < 500ms
- [ ] 虚拟列表滚动帧率 > 50fps

---

### Phase 5: 安全性审查

#### 代码安全
- [ ] ✅ XSS 风险排查
  - [ ] exportHelpers.ts 中 HTML 转义
  - [ ] 用户输入是否正确处理

- [ ] ✅ 数据验证
  - [ ] 导出时的数据检验
  - [ ] 不合法数据处理

#### 依赖安全
- [ ] 新增依赖是否必要
- [ ] 第三方库版本更新

---

## 📝 审查问题集

### 必答问题 (提交 PR 前必须回答)

1. **代码质量**
   - Q: 新的组件分解是否遵循单一职责原则？
   - A: _______________

2. **向后兼容**
   - Q: 是否所有现有的导入路径都仍然有效？
   - A: _______________

3. **功能测试**
   - Q: 核心业务流程是否在修改后仍正常工作？
   - A: _______________

4. **性能影响**
   - Q: 是否存在性能回归？用什么指标验证？
   - A: _______________

5. **TypeScript**
   - Q: 编译是否通过，是否有新的 TS 错误？
   - A: _______________

### 深度审查问题

6. **Hook 依赖**
   - Q: useCallback 和 useMemo 的依赖数组是否完整？
   - A: _______________

7. **闭包安全**
   - Q: 是否存在闭包陷阱（过时的状态引用）？
   - A: _______________

8. **内存管理**
   - Q: 是否所有 useEffect 都有清理函数？
   - A: _______________

9. **测试覆盖**
   - Q: 新的子组件是否需要单元测试？
   - A: _______________

10. **文档更新**
    - Q: 是否需要更新组件文档或 README？
    - A: _______________

---

## 🎯 重点审查项

### 高风险区域 (必须详细审查)

#### 1. PlanManagement 回调函数绑定 (a015b14)
**文件**: src/components/PlanManagement.tsx (行 58-407)

**改动内容**:
```typescript
// ✅ 使用 useCallback 包装所有回调
const loadPlans = useCallback(async () => {...}, []);
const handleActivateVersion = useCallback(async (versionId) => {
  // 依赖: selectedPlanId, versions, currentUser, setActiveVersion, loadVersions
}, [selectedPlanId, versions, currentUser, setActiveVersion, loadVersions]);
```

**审查问题**:
- [ ] 每个 useCallback 的依赖数组是否完整？
- [ ] 是否存在闭包陷阱（使用过时的状态）？
- [ ] 依赖项变化是否会导致不必要的重新创建？

**验证方法**:
```bash
# 使用 React DevTools 检查
1. 打开 Components 标签
2. 选择 PlanManagement 组件
3. 检查 Hooks 中的 useCallback 和 useMemo
4. 修改 state 观察是否正确触发更新
```

---

#### 2. VersionComparisonModal 数据流 (aaad14f)
**文件**: src/components/version-comparison-modal/ (200 行)

**改动内容**:
- 从 666 行分解为 4 个子组件 + 类型定义
- 所有状态通过 Props 从父组件传递

**审查问题**:
- [ ] Props 接口类型是否准确？
- [ ] 回调函数签名是否一致？
- [ ] 数据流是否单向（避免双向绑定）？

---

#### 3. ScheduleCardView 虚拟列表 (3cdbd40)
**文件**: src/components/schedule-card-view/ (70 行)

**改动内容**:
- 使用 react-window 虚拟列表
- 高度计算（ROW_HEIGHT = 92px）

**审查问题**:
- [ ] 虚拟列表高度计算是否正确？
- [ ] 大数据集 (1000+ 行) 性能如何？
- [ ] 是否存在闪烁或滚动卡顿？

**测试方法**:
```javascript
// 在浏览器 DevTools 中运行
performance.mark('scroll-start');
// 然后滚动列表
performance.mark('scroll-end');
performance.measure('scroll', 'scroll-start', 'scroll-end');
console.log(performance.getEntriesByName('scroll')[0]);
```

---

#### 4. exportHelpers 导出函数 (42152d0)
**文件**: src/components/plan-management/exportHelpers.ts (247 行)

**改动内容**:
- 集中所有导出逻辑（CSV, JSON, Markdown, HTML）
- 使用 ExportContext 接口传递参数

**审查问题**:
- [ ] HTML 转义（escape 函数）是否完善？
- [ ] 是否有 XSS 风险？
- [ ] 导出性能（大数据集）是否可接受？
- [ ] 错误处理是否完善？

**安全测试**:
```javascript
// 测试 XSS 转义
const testData = "<img src=x onerror=alert('xss')>";
// 应该转义为 &lt;img src=x onerror=alert('xss')&gt;
```

---

### 中风险区域 (重点审查)

#### 5. MaterialDetailModal 子组件 (d714b83)
- [ ] 4 个子组件数据流是否清晰
- [ ] ActionLogsSection 表格性能
- [ ] 状态管理是否集中

#### 6. StrategyDraftDetailDrawer 复杂渲染 (e756ee7)
- [ ] ChangeTypeRenderer 中的 Tooltip 缓存
- [ ] 异步加载提示逻辑

---

## ✅ 审查检查表

### 提交前检查
- [ ] 运行 `npx tsc --noEmit` - 无 TS 错误
- [ ] 运行 `npm run lint` - 无 Lint 错误
- [ ] 运行 `npm run build` - 构建成功
- [ ] 运行现有测试 - 全部通过

### 代码审查检查
- [ ] 代码风格一致
- [ ] 无注释代码
- [ ] 无调试日志 (console.log, debugger)
- [ ] 错误处理完善
- [ ] 性能优化合理

### 功能验证检查
- [ ] 核心流程测试通过
- [ ] 边界条件处理
- [ ] 错误场景处理
- [ ] 无性能回归

---

## 🚀 审查流程

### 步骤 1: 准备 (15 分钟)
1. 检出最新代码: `git checkout a015b14`
2. 安装依赖: `npm install`
3. 阅读这份审查指南

### 步骤 2: 静态审查 (30 分钟)
1. 查看 `git log --stat a015b14~10..a015b14` 了解改动范围
2. 审查关键提交: `git show <commit-hash>`
3. 检查类型安全: `npx tsc --noEmit`

### 步骤 3: 动态审查 (45 分钟)
1. 启动应用: `npm run dev`
2. 执行核心业务流程
3. 使用 React DevTools 检查性能
4. 使用 Chrome DevTools 检查内存

### 步骤 4: 反馈 (15 分钟)
1. 记录发现的问题
2. 提交审查意见
3. 讨论风险项

---

## 📞 审查联系人

| 角色 | 名字 | 职责 |
|------|------|------|
| 代码审查负责人 | TBD | 综合把关 |
| 性能审查 | TBD | 性能测试 |
| 安全审查 | TBD | 安全检查 |
| 功能验证 | TBD | 业务流程 |

---

## 📅 审查时间表

| 阶段 | 日期 | 负责人 | 状态 |
|------|------|--------|------|
| 静态审查 | 2026-02-01 | - | ⏳ |
| 动态审查 | 2026-02-02 | - | ⏳ |
| 问题修复 | 2026-02-03 | - | ⏳ |
| 最终检查 | 2026-02-04 | - | ⏳ |
| 上线准备 | 2026-02-05 | - | ⏳ |

---

## 📌 相关文档

- [重构工程总结](https://github.com/butianzheng/hot-rolling-finish-aps/commit/a015b14)
- [PlanManagement 重构计划](/.claude/plans/shimmying-yawning-turtle.md)
- [最佳实践指南](BEST_PRACTICES.md)

---

**审查版本**: v1.0
**最后更新**: 2026-01-30
**下一次审查**: 定期季度代码审查
