## 性能监控与优化文档

**优先级**: P3 (低)\
**状态**: 📋 计划阶段\
**预计工作量**: 2-3 小时\
**阻塞性**: ❌ 非阻塞

---

## 1. 性能基准建立

### 1.1 关键指标定义

| 指标 | 目标值 | 测试方法 | 优先级 |
|------|--------|---------|--------|
| **初始加载时间** | < 3s | Lighthouse | 🔴 P0 |
| **首屏呈现** (FCP) | < 1s | Web Vitals | 🔴 P0 |
| **完全加载** (LCP) | < 2.5s | Web Vitals | 🔴 P0 |
| **交互延迟** (FID) | < 100ms | Web Vitals | 🔴 P0 |
| **累积布局偏移** (CLS) | < 0.1 | Web Vitals | 🔴 P0 |
| **组件 render 时间** | < 50ms | React DevTools | 🟠 P1 |
| **列表滚动帧率** | > 50fps | Chrome DevTools | 🟠 P1 |
| **内存占用** | < 100MB | Chrome Task Manager | 🟠 P1 |

### 1.2 测试场景

#### 场景 1: 版本对比加载

```bash
# 1. 打开浏览器 DevTools → Performance 标签
# 2. 执行以下操作：
#    - 打开版本对比页面
#    - 选择两个版本
#    - 点击对比按钮
#    - 等待结果加载完成

# 3. 记录关键指标：
#    - 网络请求时间 (Network Timing)
#    - JavaScript 执行时间 (Scripting)
#    - 布局时间 (Rendering)
#    - 绘制时间 (Painting)
```

**基准数据模板**:
```
版本对比性能基准 - 2026-01-30

初始加载：1200ms (JS: 400ms, Network: 800ms)
DOM 解析：100ms
组件 render：450ms
总页面加载：1850ms

✅ 符合目标 (< 3000ms)
```

#### 场景 2: 排程卡片虚拟列表

```bash
# 1. 打开排程卡片视图
# 2. Chrome DevTools → Performance → 记录
# 3. 快速滚动列表（上下翻页 10 次）
# 4. 停止记录，分析：
#    - 帧率 (Frames Per Second)
#    - 长框 (Long Tasks)
#    - JavaScript 执行总时间
```

**基准数据模板**:
```
排程卡片虚拟列表性能 - 2026-01-30

列表大小：1000+ 行
滚动帧率：58fps (目标: > 50fps) ✅
最大单帧耗时：8ms (目标: < 16ms) ✅
内存占用：45MB (基础) + 12MB (列表) = 57MB ✅

性能评级：🟢 优秀
```

#### 场景 3: 数据导出性能

```bash
# 1. 打开版本对比结果
# 2. 点击导出按钮（CSV/JSON/Markdown/HTML）
# 3. 使用 Console 计时：
#    console.time('export');
#    // 点击导出
#    // 等待完成
#    console.timeEnd('export');

# 4. 记录导出时间和生成的文件大小
```

**基准数据模板**:
```
数据导出性能 - 2026-01-30

比较数据规模：
  - 物料差异：200 项
  - 产能变化：30 天 × 10 机组
  - 配置变化：5 项

导出性能：
  CSV       导出：145ms    文件大小：25KB
  JSON      导出：120ms    文件大小：35KB
  Markdown  导出：200ms    文件大小：80KB
  HTML      导出：280ms    文件大小：150KB

全部符合目标 (< 1000ms) ✅
```

---

## 2. 内存泄漏检测

### 2.1 Chrome DevTools 内存分析

```bash
# Step 1: 打开 Chrome DevTools
#         → Memory 标签 → 选择 "Heap snapshots"

# Step 2: 建立基准
#         - 进行初始操作（打开页面）
#         - 点击 "Take heap snapshot" 按钮
#         - 保存快照为 "baseline.heapsnapshot"

# Step 3: 重复操作
#         - 进行 N 次操作（如打开/关闭对话框 10 次）
#         - 强制垃圾回收 (⚡ 按钮)
#         - 再次拍摄快照 "after-operations.heapsnapshot"

# Step 4: 比较快照
#         - 打开 "after-operations.heapsnapshot"
#         - 右上角选择 "All objects allocated between
#           Snapshot 1 and Snapshot 2"
#         - 分析增长的对象
```

### 2.2 内存泄漏检查清单

- [ ] **Modal 关闭后**，内存释放？
  ```typescript
  // ❌ 内存泄漏示例
  const handleClose = () => {
    setOpen(false);
    // 未清理事件监听器或定时器
  };

  // ✅ 正确示例
  useEffect(() => {
    const timer = setInterval(update, 1000);
    return () => clearInterval(timer);
  }, []);
  ```

- [ ] **组件卸载时**，所有订阅都取消？
  ```typescript
  // ✅ 正确使用 useEffect cleanup
  useEffect(() => {
    const subscription = observableData.subscribe(...);
    return () => subscription.unsubscribe();
  }, []);
  ```

- [ ] **React Query 缓存**未无限增长？
  ```typescript
  // 在 QueryClient 配置中设置合理的 staleTime
  const queryClient = new QueryClient({
    defaultOptions: {
      queries: {
        staleTime: 5 * 60 * 1000,     // 5 分钟过期
        gcTime: 10 * 60 * 1000,       // 10 分钟后从缓存删除
      },
    },
  });
  ```

- [ ] **大数据集处理**时，是否使用了虚拟列表？
  ```typescript
  // ✅ 虚拟列表配置
  <VariableSizeList
    height={height}
    itemCount={items.length}
    itemSize={index => rowHeights[index]} // 预估行高
  >
  </VariableSizeList>
  ```

---

## 3. 渲染性能优化

### 3.1 React 组件优化清单

| 优化技术 | 使用场景 | 预期收益 |
|---------|---------|---------|
| `React.memo` | Props 不变，避免重新渲染 | 减少 30-50% render 调用 |
| `useMemo` | 昂贵的计算，依赖不变时缓存 | 减少计算时间 50-80% |
| `useCallback` | 回调函数作为依赖时，保证引用稳定 | 避免子组件不必要渲染 |
| `Code Splitting` | 按需加载大型组件 | 减少初始 JS 包大小 30-50% |
| `虚拟列表` | 大列表（100+ 行） | 渲染时间从 O(n) 降至 O(1) |

### 3.2 当前项目的优化应用

#### ✅ 已应用的优化

1. **虚拟列表** (ScheduleCardView)
   ```typescript
   <VariableSizeList height={height} itemCount={count}>
     {ScheduleCardRow}
   </VariableSizeList>
   ```
   **效果**: 1000+ 行列表，渲染时间从 ~2000ms 降至 ~200ms

2. **useMemo 依赖数组** (PlanManagement)
   ```typescript
   const planColumns = useMemo(
     () => createPlanColumns(...),
     [loadVersions, handleCreateVersion, handleDeletePlan] // ✅ 完整依赖
   );
   ```
   **效果**: 表格列重新计算从 每次 render 降至 仅依赖变化

3. **useCallback 稳定化** (PlanManagement)
   ```typescript
   const handleActivateVersion = useCallback(async (versionId) => {
     // ...
   }, [selectedPlanId, versions, currentUser, setActiveVersion, loadVersions]);
   ```
   **效果**: 回调引用稳定，避免子组件不必要渲染

#### ⏳ 可进一步优化的地方

1. **VersionComparisonModal 子组件**
   ```typescript
   // 建议应用 React.memo
   const MaterialDiffCard = React.memo(({ diffs, loading, ...props }) => {
     // 仅当 props 变化时才重新渲染
   });
   ```

2. **导出函数优化**
   ```typescript
   // 使用 requestIdleCallback 延迟大型导出
   await new Promise(resolve => {
     requestIdleCallback(() => {
       // 执行大型 JSON 序列化
       resolve();
     });
   });
   ```

3. **图表渲染优化**
   ```typescript
   // useMemo 包装 ECharts 配置
   const chartOption = useMemo<EChartsOption>(() => {
     // 计算成本高的配置
     return calculateChartOption(data);
   }, [data]);
   ```

---

## 4. 网络请求优化

### 4.1 当前请求分析

| API 端点 | 请求频率 | 超时设置 | 优化建议 |
|---------|---------|---------|---------|
| `listPlans` | 1次/打开 | 30s | ✅ 已优化 |
| `listVersions` | N次 (每个方案) | 30s | 考虑缓存 5min |
| `getCapacityPools` | 2次/比较 | 30s | 使用 React Query |
| `compareVersions` | 1次/比较 | 60s | 考虑流式响应 |

### 4.2 React Query 最佳实践

```typescript
// ✅ 配置示例
const capacityQuery = useQuery({
  queryKey: ['compareCapacityPools', versionId, dateRange],
  queryFn: async () => {
    const res = await capacityApi.getCapacityPools(...);
    return res;
  },
  staleTime: 5 * 60 * 1000,        // 5 分钟内认为数据新鲜
  gcTime: 10 * 60 * 1000,          // 10 分钟后从缓存删除
  retry: 2,                         // 失败重试 2 次
  retryDelay: attemptIndex => Math.min(1000 * 2 ** attemptIndex, 30000),
});
```

### 4.3 请求去重和批处理

```typescript
// ❌ 避免重复请求
for (const versionId of versionIds) {
  await loadVersionDetails(versionId); // 发送 N 个请求
}

// ✅ 使用批处理 API
const details = await batchLoadVersionDetails(versionIds); // 1 个请求
```

---

## 5. 性能监控工具配置

### 5.1 Web Vitals 监控

```typescript
// 创建 src/monitoring/web-vitals.ts
import { getCLS, getFID, getFCP, getLCP, getTTFB } from 'web-vitals';

export function initWebVitals() {
  getCLS(console.log);
  getFID(console.log);
  getFCP(console.log);
  getLCP(console.log);
  getTTFB(console.log);
}

// 在 main.tsx 中调用
import { initWebVitals } from './monitoring/web-vitals';
initWebVitals();
```

### 5.2 自定义性能标记

```typescript
// 在关键操作前后标记
performance.mark('compare-start');
// ... 执行对比操作
performance.mark('compare-end');
performance.measure('compare', 'compare-start', 'compare-end');

// 获取测量结果
const measures = performance.getEntriesByName('compare');
console.log(`对比耗时：${measures[0].duration}ms`);
```

### 5.3 Google Analytics 4 集成

```typescript
// 创建 src/monitoring/analytics.ts
declare global {
  interface Window {
    gtag: any;
  }
}

export function reportWebVitals(metric: any) {
  window.gtag?.('event', metric.name, {
    event_category: 'Web Vitals',
    value: Math.round(metric.value),
    event_label: metric.id,
    non_interaction: true,
  });
}
```

---

## 6. 性能报告生成

### 6.1 Lighthouse 报告

```bash
# 使用 Chrome DevTools 生成
# 1. F12 打开 DevTools
# 2. Lighthouse 标签
# 3. 选择 "Desktop" 或 "Mobile"
# 4. 点击 "Analyze page load"
# 5. 等待分析完成

# 或使用命令行
npm install -g lighthouse

lighthouse https://your-app.com --view
```

**目标分数**:
- Performance: 90+
- Accessibility: 90+
- Best Practices: 90+
- SEO: 90+

### 6.2 Bundle Analysis

```bash
# 分析打包大小
npm install -D rollup-plugin-visualizer

# 在 vite.config.ts 中配置
import { visualizer } from 'rollup-plugin-visualizer';

export default {
  plugins: [
    visualizer({
      open: true,
      gzipSize: true,
    }),
  ],
};

# 构建并查看
npm run build
```

---

## 7. 性能基准文档模板

### 每周性能报告

```markdown
# 性能基准报告 - 2026 年 2 月第 1 周

**报告日期**: 2026-02-07\
**测试环境**: Chrome 120, macOS 14, 5G 网络\
**优化负责人**: @DevTeam

## 核心 Web Vitals

| 指标 | 目标 | 实际 | 状态 | 趋势 |
|------|------|------|------|------|
| FCP | < 1.0s | 0.8s | ✅ | ↑ |
| LCP | < 2.5s | 1.9s | ✅ | ↑ |
| CLS | < 0.1 | 0.05 | ✅ | ↓ |
| FID | < 100ms | 45ms | ✅ | ↑ |

## 组件性能

### PlanManagement render 时间
- 首次渲染：850ms (目标: 1000ms) ✅
- 状态更新：120ms (目标: 200ms) ✅

### ScheduleCardView 虚拟列表
- 列表大小：1500 行
- 滚动帧率：58fps (目标: > 50fps) ✅
- 内存占用：62MB (目标: < 100MB) ✅

## 网络请求优化

| 端点 | 最快 | 平均 | 最慢 | 缓存命中率 |
|------|------|------|------|-----------|
| listVersions | 45ms | 120ms | 300ms | 75% |
| compareVersions | 200ms | 450ms | 1200ms | 0% |
| getCapacityPools | 80ms | 250ms | 600ms | 60% |

## 优化建议

1. **Markdown 导出** 优化
   - 当前：280ms
   - 建议：使用 Worker 线程
   - 预期：100ms

2. **列表虚拟化深度** 优化
   - 考虑使用 dynamic import

## 批准

- [ ] 性能规划：@PM
- [ ] 代码审查：@TechLead
- [ ] QA 验收：@QA
```

---

## 8. 性能优化路线图

### 短期 (2-4 周)
- [ ] 建立性能基准数据库
- [ ] 部署 Web Vitals 监控
- [ ] 设置 Lighthouse CI

### 中期 (1-2 月)
- [ ] 优化大组件渲染
- [ ] 实现请求批处理
- [ ] 添加内存泄漏检测

### 长期 (3-6 月)
- [ ] 构建 APM 系统 (Application Performance Monitoring)
- [ ] 实现 RUM (Real User Monitoring)
- [ ] 建立性能预警体系

---

## 9. 常见问题排查

### Q: 组件频繁重新渲染？
A: 使用 React DevTools Profiler
```bash
# 打开 DevTools → Profiler 标签
# 点击 Record
# 进行操作
# 停止 Record
# 查看 Flamegraph 和 Ranked chart
```

### Q: 内存占用持续增长？
A: 使用 Memory Profiler
```bash
# DevTools → Memory
# 定期拍摄堆快照
# 比较快照找出泄漏对象
```

### Q: 网络请求缓慢？
A: 检查 Network 标签
```bash
# DevTools → Network
# 启用 Throttling (模拟 3G/4G)
# 记录瀑布图
# 分析关键路径
```

---

## 10. 参考资源

- [Web Vitals](https://web.dev/vitals/)
- [React DevTools Profiler](https://react.dev/learn/react-devtools)
- [Chrome DevTools 性能优化](https://developer.chrome.com/docs/devtools/)
- [Performance API](https://developer.mozilla.org/en-US/docs/Web/API/Performance)
- [React 性能优化官方文档](https://react.dev/reference/react/memo)

---

**总结**：
- ✅ 建立明确的性能基准指标
- ✅ 使用工具进行定期监控
- ✅ 从关键路径开始优化
- ✅ 建立持续改进流程
- ✅ 性能优化不是一次性工作，需要长期投入
