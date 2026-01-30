## 单元测试补充计划

**优先级**: P2 (中等)\
**状态**: 📋 计划阶段\
**预计工作量**: 4-6 小时\
**阻塞性**: ❌ 非阻塞

---

## 1. 测试框架选择建议

### 推荐方案：Vitest + React Testing Library

**原因**：
- ✅ 与 Vite 原生集成，构建速度快
- ✅ 与 Jest API 兼容，学习成本低
- ✅ React Testing Library 专为 React 组件设计
- ✅ TypeScript 支持开箱即用

**安装命令**:
```bash
npm install -D vitest @testing-library/react @testing-library/jest-dom happy-dom
```

**配置** (创建 `vitest.config.ts`):
```typescript
import { defineConfig } from 'vitest/config';
import react from '@vitejs/plugin-react';

export default defineConfig({
  plugins: [react()],
  test: {
    globals: true,
    environment: 'happy-dom',
    setupFiles: ['./src/tests/setup.ts'],
  },
});
```

**package.json 脚本**:
```json
{
  "scripts": {
    "test": "vitest",
    "test:ui": "vitest --ui",
    "test:coverage": "vitest --coverage"
  }
}
```

---

## 2. 需要补充的单元测试

### 优先级 1：关键工具函数（易于测试）

#### 文件：`src/components/comparison/utils.ts`

**测试套件**：

```typescript
describe('comparison/utils', () => {
  // ✅ 1. normalizeDateOnly
  describe('normalizeDateOnly', () => {
    test('应该提取 YYYY-MM-DD 部分', () => {
      const result = normalizeDateOnly('2026-01-30 14:30:00');
      expect(result).toBe('2026-01-30');
    });

    test('空输入应返回空字符串', () => {
      expect(normalizeDateOnly('')).toBe('');
      expect(normalizeDateOnly(null as any)).toBe('');
    });
  });

  // ✅ 2. formatVersionLabel
  describe('formatVersionLabel', () => {
    test('有中文名称时优先返回中文名称', () => {
      const version = {
        version_id: 'v123',
        version_no: 1,
        config_snapshot_json: JSON.stringify({
          __meta_version_name_cn: '生产版本 v1',
        }),
      };
      expect(formatVersionLabel(version)).toBe('生产版本 v1');
    });

    test('无中文名称时返回版本号', () => {
      const version = {
        version_id: 'v123',
        version_no: 2,
        config_snapshot_json: null,
      };
      expect(formatVersionLabel(version)).toBe('V2');
    });
  });

  // ✅ 3. normalizePlanItem
  describe('normalizePlanItem', () => {
    test('应该规范化计划项数据', () => {
      const raw = {
        material_id: 'M001',
        machine_code: 'M1',
        plan_date: '2026-01-30 10:00:00',
        seq_no: 1,
        weight_t: 5.5,
      };
      const result = normalizePlanItem(raw);
      expect(result?.material_id).toBe('M001');
      expect(result?.plan_date).toBe('2026-01-30');
      expect(result?.weight_t).toBe(5.5);
    });

    test('material_id 缺失时返回 null', () => {
      const result = normalizePlanItem({ plan_date: '2026-01-30' });
      expect(result).toBeNull();
    });
  });

  // ✅ 4. computeVersionDiffs
  describe('computeVersionDiffs', () => {
    test('应该正确计算版本差异', () => {
      const itemsA: PlanItemSnapshot[] = [
        {
          material_id: 'M1',
          machine_code: 'MA',
          plan_date: '2026-01-30',
          seq_no: 1,
        },
      ];
      const itemsB: PlanItemSnapshot[] = [
        {
          material_id: 'M1',
          machine_code: 'MB',
          plan_date: '2026-01-31',
          seq_no: 2,
        },
      ];
      const result = computeVersionDiffs(itemsA, itemsB);
      expect(result.diffs).toHaveLength(1);
      expect(result.diffs[0].changeType).toBe('MOVED');
      expect(result.summary.movedCount).toBe(1);
    });
  });

  // ✅ 5. computeCapacityMap
  describe('computeCapacityMap', () => {
    test('应该按机组+日期聚合重量', () => {
      const items: PlanItemSnapshot[] = [
        { material_id: 'M1', machine_code: 'M1', plan_date: '2026-01-30', seq_no: 1, weight_t: 10 },
        { material_id: 'M2', machine_code: 'M1', plan_date: '2026-01-30', seq_no: 2, weight_t: 15 },
        { material_id: 'M3', machine_code: 'M2', plan_date: '2026-01-30', seq_no: 1, weight_t: 20 },
      ];
      const map = computeCapacityMap(items);
      expect(map.get('M1__2026-01-30')).toBe(25);
      expect(map.get('M2__2026-01-30')).toBe(20);
    });
  });

  // ✅ 6. computeDailyTotals
  describe('computeDailyTotals', () => {
    test('应该按日期聚合总产量', () => {
      const items: PlanItemSnapshot[] = [
        { material_id: 'M1', machine_code: 'M1', plan_date: '2026-01-30', seq_no: 1, weight_t: 10 },
        { material_id: 'M2', machine_code: 'M2', plan_date: '2026-01-30', seq_no: 1, weight_t: 20 },
        { material_id: 'M3', machine_code: 'M1', plan_date: '2026-01-31', seq_no: 1, weight_t: 15 },
      ];
      const map = computeDailyTotals(items);
      expect(map.get('2026-01-30')).toBe(30);
      expect(map.get('2026-01-31')).toBe(15);
    });
  });
});
```

**测试方法**:
```bash
npm run test -- src/components/comparison/utils.ts
```

---

### 优先级 2：导出工具函数

#### 文件：`src/components/plan-management/exportHelpers.ts`

**测试套件**：

```typescript
describe('exportHelpers', () => {
  // Mock 导出函数
  vi.mock('../../utils/exportUtils', () => ({
    exportCSV: vi.fn(),
    exportJSON: vi.fn(),
    exportMarkdown: vi.fn(),
    exportHTML: vi.fn(),
  }));

  // ✅ 1. exportCapacityDelta
  describe('exportCapacityDelta', () => {
    test('CSV 格式导出应该正确映射字段', async () => {
      const mockContext: ExportContext = {
        compareResult: { version_id_a: 'v1', version_id_b: 'v2' },
        currentUser: 'test_user',
        localDiffResult: null,
        localCapacityRows: {
          rows: [
            { machine_code: 'M1', date: '2026-01-30', used_a: 100, used_b: 120, delta: 20 },
          ],
          totalA: 100,
          totalB: 120,
          dateFrom: '2026-01-30',
          dateTo: '2026-01-30',
          machines: ['M1'],
          overflowRows: [],
        },
        retrospectiveNote: '',
      };

      await exportCapacityDelta('csv', mockContext);
      expect(exportCSV).toHaveBeenCalled();
      const call = (exportCSV as any).mock.calls[0];
      expect(call[0]).toHaveLength(1);
      expect(call[0][0].machine_code).toBe('M1');
      expect(call[0][0].delta).toBe(20);
    });

    test('数据为 null 时应该提前返回', async () => {
      const context: ExportContext = {
        compareResult: { version_id_a: 'v1', version_id_b: 'v2' },
        currentUser: 'test_user',
        localDiffResult: null,
        localCapacityRows: null,
        retrospectiveNote: '',
      };
      await exportCapacityDelta('csv', context);
      expect(exportCSV).not.toHaveBeenCalled();
    });
  });

  // ✅ 2. exportDiffs
  describe('exportDiffs', () => {
    test('应该正确映射版本差异数据', async () => {
      const mockContext: ExportContext = {
        compareResult: { version_id_a: 'v1', version_id_b: 'v2' },
        currentUser: 'test_user',
        localDiffResult: {
          diffs: [
            {
              materialId: 'M001',
              changeType: 'MOVED',
              previousState: { machine_code: 'MA', plan_date: '2026-01-30', seq_no: 1 },
              currentState: { machine_code: 'MB', plan_date: '2026-01-31', seq_no: 2 },
            },
          ],
          summary: { totalChanges: 1, movedCount: 1 },
        },
        localCapacityRows: null,
        retrospectiveNote: '',
      };

      await exportDiffs('json', mockContext);
      expect(exportJSON).toHaveBeenCalled();
      const call = (exportJSON as any).mock.calls[0];
      expect(call[0]).toHaveLength(1);
      expect(call[0][0].change_type).toBe('MOVED');
    });
  });

  // ✅ 3. exportRetrospectiveReport
  describe('exportRetrospectiveReport', () => {
    test('应该导出复盘总结 JSON', async () => {
      const mockContext: ExportContext = {
        compareResult: {
          version_id_a: 'v1',
          version_id_b: 'v2',
          moved_count: 0,
          added_count: 0,
        },
        currentUser: 'operator_001',
        localDiffResult: null,
        localCapacityRows: null,
        retrospectiveNote: '测试复盘',
      };

      await exportRetrospectiveReport(mockContext);
      expect(exportJSON).toHaveBeenCalled();
    });

    test('错误时应该捕获并显示错误信息', async () => {
      vi.mocked(exportJSON).mockImplementationOnce(() => {
        throw new Error('Export failed');
      });

      const mockContext: ExportContext = {
        compareResult: { version_id_a: 'v1', version_id_b: 'v2' },
        currentUser: 'test',
        localDiffResult: null,
        localCapacityRows: null,
        retrospectiveNote: '',
      };

      // 应该不抛出错误，而是显示错误信息
      await expect(exportRetrospectiveReport(mockContext)).resolves.not.toThrow();
    });
  });

  // ✅ 4. exportReportHTML
  describe('exportReportHTML', () => {
    test('应该生成包含 XSS 转义的 HTML', async () => {
      const mockContext: ExportContext = {
        compareResult: {
          version_id_a: 'v1<script>',
          version_id_b: 'v2',
          moved_count: 0,
          added_count: 0,
        },
        currentUser: 'test',
        localDiffResult: null,
        localCapacityRows: null,
        retrospectiveNote: '<img src=x onerror=alert("xss")>',
      };

      await exportReportHTML(mockContext);
      expect(exportHTML).toHaveBeenCalled();
      const html = (exportHTML as any).mock.calls[0][0];
      expect(html).toContain('&lt;script&gt;');
      expect(html).not.toContain('<script>');
      expect(html).toContain('&lt;img');
    });
  });
});
```

**XSS 安全测试**:
```typescript
test('HTML 导出应该防护 XSS 攻击', async () => {
  const context: ExportContext = {
    compareResult: { version_id_a: 'v1' },
    currentUser: 'test',
    localDiffResult: null,
    localCapacityRows: null,
    retrospectiveNote: '"><script>alert("xss")</script><"',
  };

  await exportReportHTML(context);
  const html = (exportHTML as any).mock.calls[0][0];

  // 验证所有特殊字符都被转义
  expect(html).not.toContain('<script>');
  expect(html).not.toContain('</script>');
  expect(html).toContain('&lt;');
  expect(html).toContain('&gt;');
});
```

---

### 优先级 3：React 组件测试

#### 文件：`src/components/comparison/VersionComparisonModal.tsx`

**测试套件**：

```typescript
describe('VersionComparisonModal', () => {
  // ✅ 1. Props 验证
  test('应该接受所有必需的 props', () => {
    const props: VersionComparisonModalProps = {
      open: true,
      onClose: vi.fn(),
      compareResult: {
        version_id_a: 'v1',
        version_id_b: 'v2',
        moved_count: 5,
      },
      compareKpiRows: [],
      localDiffResult: null,
      loadLocalCompareDetail: false,
      planItemsLoading: false,
      localCapacityRows: null,
      showAllCapacityRows: false,
      retrospectiveNote: '',
      onActivateVersion: vi.fn(),
      onToggleShowAllCapacityRows: vi.fn(),
      onRetrospectiveNoteChange: vi.fn(),
      onRetrospectiveNoteSave: vi.fn(),
      onExportReport: vi.fn(),
      onDiffSearchChange: vi.fn(),
      onDiffTypeFilterChange: vi.fn(),
    };

    render(<VersionComparisonModal {...props} />);
    expect(screen.getByRole('dialog')).toBeInTheDocument();
  });

  // ✅ 2. 回调函数触发
  test('关闭按钮应该触发 onClose 回调', () => {
    const onClose = vi.fn();
    const { container } = render(
      <VersionComparisonModal
        open={true}
        onClose={onClose}
        // ... 其他 props
      />
    );

    const closeButton = container.querySelector('.ant-modal-close');
    fireEvent.click(closeButton!);
    expect(onClose).toHaveBeenCalled();
  });

  // ✅ 3. 条件渲染
  test('loading 状态时应该显示 Skeleton', () => {
    render(
      <VersionComparisonModal
        open={true}
        onClose={vi.fn()}
        loadLocalCompareDetail={true}
        planItemsLoading={true}
        // ... 其他 props
      />
    );

    // Ant Design Skeleton 有特定的 class
    expect(document.querySelector('.ant-skeleton')).toBeInTheDocument();
  });
});
```

---

## 3. 测试覆盖率目标

| 模块 | 目标 | 重要性 |
|------|------|--------|
| `comparison/utils.ts` | 90%+ | 🔴 关键 |
| `plan-management/exportHelpers.ts` | 85%+ | 🟠 高 |
| `comparison/VersionComparisonModal.tsx` | 70%+ | 🟡 中 |
| 其他组件 | 50%+ | 🟢 低 |

**验证**:
```bash
npm run test:coverage
```

---

## 4. 集成测试（可选）

### 版本对比完整流程

```typescript
describe('版本对比完整流程', () => {
  test('用户应该能够完整进行版本对比和导出', async () => {
    // 1. 打开版本对比
    const user = userEvent.setup();
    render(<PlanManagement />);

    const compareButton = screen.getByRole('button', { name: /对比/ });
    await user.click(compareButton);

    // 2. 选择版本
    // 3. 执行对比
    // 4. 验证结果
    expect(screen.getByText(/物料差异/)).toBeInTheDocument();

    // 5. 导出
    const exportButton = screen.getByRole('button', { name: /导出.*CSV/ });
    await user.click(exportButton);

    // 6. 验证导出成功
    await waitFor(() => {
      expect(screen.getByText(/已导出/)).toBeInTheDocument();
    });
  });
});
```

---

## 5. 实施步骤

### Step 1: 环境设置（30 分钟）
```bash
# 1. 安装依赖
npm install -D vitest @testing-library/react @testing-library/jest-dom happy-dom @vitest/ui @vitest/coverage-v8

# 2. 创建配置文件
# vitest.config.ts
# src/tests/setup.ts

# 3. 更新 package.json
```

### Step 2: 编写工具函数测试（1 小时）
- ✅ `comparison/utils.ts` 测试
- ✅ 验证所有函数的边界条件

### Step 3: 编写导出函数测试（1.5 小时）
- ✅ `plan-management/exportHelpers.ts` 测试
- ✅ XSS 防护验证
- ✅ 错误处理测试

### Step 4: 编写组件测试（1.5 小时）
- ✅ `VersionComparisonModal.tsx` 基础测试
- ✅ Props 验证
- ✅ 回调函数测试

### Step 5: 检查覆盖率（1 小时）
```bash
npm run test:coverage
```

---

## 6. 后续维护

### 测试编写规范

1. **命名约定**：
   - 文件：`ComponentName.test.tsx`
   - 描述：`describe('ComponentName', () => {})`

2. **最佳实践**：
   - 每个测试一个关键行为（AAA 模式：Arrange, Act, Assert）
   - 避免测试实现细节，重点测试行为
   - 使用有意义的测试名称

3. **Mock 策略**：
   - Mock API 调用（`vi.mock`）
   - Mock React Query（`@tanstack/react-query` 提供测试工具）
   - 避免 Mock DOM API，直接使用 Happy DOM

### CI/CD 集成

```yaml
# .github/workflows/test.yml
name: Tests
on: [push, pull_request]
jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - uses: actions/setup-node@v3
        with:
          node-version: 18
      - run: npm ci
      - run: npm run test
      - run: npm run test:coverage
      - uses: codecov/codecov-action@v3
```

---

## 7. 知识库文档

### 测试编写指南

创建 `docs/TESTING.md`：
```markdown
# 测试指南

## 如何运行测试

- 运行所有测试：`npm run test`
- 监听模式：`npm run test -- --watch`
- 生成覆盖率：`npm run test:coverage`

## 编写测试的黄金法则

1. 测试用户行为，不测试实现细节
2. 使用有意义的测试名称
3. 遵循 AAA 模式
4. 保持测试简洁专注

## 常见问题

Q: 如何测试异步代码？
A: 使用 `waitFor` 和 `async/await`

...
```

---

**总结**：
- ✅ 推荐使用 Vitest + React Testing Library
- ✅ 首先测试工具函数（ROI 最高）
- ✅ 其次测试导出函数（安全关键）
- ✅ 最后测试 React 组件（成本最高）
- ✅ 预计总耗时 4-6 小时
- ✅ 非阻塞项，可在上线后补充
