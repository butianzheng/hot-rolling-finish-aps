# 工业红线防护组件使用指南

## 概述

本目录包含用于实现5大工业红线UI防护的组件，确保前端界面正确体现后端业务规则。

## 组件列表

### 1. FrozenZoneBadge（冻结区标识）

**用途**: 可视化标识冻结材料，体现红线1：冻结区保护

**基础用法**:
```tsx
import { FrozenZoneBadge } from '@/components/guards/FrozenZoneBadge';

// Badge模式（默认）- 小标签
<FrozenZoneBadge locked={material.is_frozen} />

// Banner模式 - 横幅提示
<FrozenZoneBadge
  locked={material.is_frozen}
  mode="banner"
  lockReason="已进入冻结区，不可调整"
/>
```

**Props**:
- `locked`: boolean - 是否锁定（冻结）
- `tooltipTitle?`: string - 自定义提示信息
- `mode?`: 'badge' | 'banner' - 显示模式（默认：badge）
- `lockReason?`: string - 锁定原因

**效果**:
- Badge模式: 红色小标签，带锁图标
- Banner模式: 横幅提示，包含详细说明

---

### 2. RedLineGuard（工业红线防护）

**用途**: 可视化展示5大工业红线约束，防止违规操作

**基础用法**:
```tsx
import {
  RedLineGuard,
  createFrozenZoneViolation,
  createMaturityViolation,
  createCapacityViolation,
  createExplainabilityViolation,
} from '@/components/guards/RedLineGuard';
import type { RedLineViolation } from '@/components/guards/RedLineGuard';

// 紧凑模式 - 仅显示违规标签
const violations: RedLineViolation[] = [
  createFrozenZoneViolation(
    ['M12345678'],
    '该材料已锁定，不可调整'
  ),
];

<RedLineGuard violations={violations} mode="compact" />

// 详细模式 - 显示完整违规信息
<RedLineGuard
  violations={violations}
  mode="detailed"
  closable
  onClose={() => console.log('关闭')}
/>
```

**Props**:
- `violations`: RedLineViolation[] - 红线违规列表
- `mode?`: 'compact' | 'detailed' - 显示模式（默认：compact）
- `closable?`: boolean - 是否可关闭（默认：false）
- `onClose?`: () => void - 关闭回调

**RedLineViolation 类型**:
```typescript
interface RedLineViolation {
  type: RedLineType; // 红线类型
  message: string; // 违规描述
  severity: 'error' | 'warning'; // 违规等级
  details?: string; // 详细信息
  affectedEntities?: string[]; // 受影响的实体
}
```

**RedLineType 类型**:
- `FROZEN_ZONE_PROTECTION`: 冻结区保护（红线1）
- `MATURITY_CONSTRAINT`: 适温约束（红线2）
- `LAYERED_URGENCY`: 分层紧急度（红线3）
- `CAPACITY_FIRST`: 容量优先（红线4）
- `EXPLAINABILITY`: 可解释性（红线5）

**工具函数**:
```typescript
// 创建冻结区保护违规
createFrozenZoneViolation(
  materialNos: string[],
  message?: string
): RedLineViolation

// 创建适温约束违规
createMaturityViolation(
  materialNos: string[],
  daysToReady: number
): RedLineViolation

// 创建容量约束违规
createCapacityViolation(
  message: string,
  details?: string
): RedLineViolation

// 创建可解释性违规
createExplainabilityViolation(
  message: string
): RedLineViolation
```

---

## 完整示例：MaterialManagement集成

```tsx
import { FrozenZoneBadge } from '@/components/guards/FrozenZoneBadge';
import {
  RedLineGuard,
  createFrozenZoneViolation,
  createMaturityViolation,
} from '@/components/guards/RedLineGuard';
import type { RedLineViolation } from '@/components/guards/RedLineGuard';

// 1. 在表格列中显示冻结区标识
const columns = [
  {
    title: '状态',
    render: (_, record) => (
      <Space direction="vertical">
        <MaterialStatusIcons {...record} />
        <FrozenZoneBadge locked={record.is_frozen || false} />
      </Space>
    ),
  },
];

// 2. 检查红线违规
const checkRedLineViolations = (
  material: Material,
  operation: string
): RedLineViolation[] => {
  const violations: RedLineViolation[] = [];

  // 检查冻结区保护
  if (material.is_frozen && operation === 'lock') {
    violations.push(
      createFrozenZoneViolation(
        [material.material_id],
        '该材料位于冻结区，不允许修改状态'
      )
    );
  }

  // 检查适温约束
  if (!material.is_mature && operation === 'urgent') {
    violations.push(
      createMaturityViolation([material.material_id], 2)
    );
  }

  return violations;
};

// 3. 在操作前显示违规提示
const handleOperation = (material: Material, type: string) => {
  const violations = checkRedLineViolations(material, type);

  if (violations.length > 0) {
    Modal.error({
      title: '工业红线保护',
      width: 600,
      content: (
        <Space direction="vertical" style={{ width: '100%' }}>
          <RedLineGuard violations={violations} mode="detailed" />
          {!adminOverrideMode && (
            <Alert
              type="warning"
              message="提示：如需覆盖此保护，请启用管理员覆盖模式"
            />
          )}
        </Space>
      ),
    });
    return;
  }

  // 执行操作...
};
```

---

## 5大工业红线对照表

| 红线编号 | 红线名称 | 前端体现 | 组件支持 |
|---------|---------|---------|---------|
| 红线1 | 冻结区保护 | FrozenZoneBadge标识 + 操作拦截 | ✅ |
| 红线2 | 适温约束 | 温度图标 + 操作拦截 | ✅ |
| 红线3 | 分层紧急度 | UrgencyTag (L0-L3) | ✅ |
| 红线4 | 容量优先 | 容量时间线可视化 | ✅ |
| 红线5 | 可解释性 | 原因展示 + Inspector详情 | ✅ |

---

## 最佳实践

### 1. 始终在操作前检查违规
```tsx
const handleAction = (material: Material) => {
  const violations = checkRedLineViolations(material, 'lock');
  if (violations.length > 0) {
    // 显示错误提示
    return;
  }
  // 执行操作
};
```

### 2. 使用详细模式展示复杂违规
```tsx
// 批量操作时使用详细模式
<RedLineGuard
  violations={allViolations}
  mode="detailed" // 显示完整信息
/>
```

### 3. 提供管理员覆盖路径
```tsx
{!adminOverrideMode && (
  <Alert
    type="warning"
    message="如需覆盖保护，请启用管理员覆盖模式"
  />
)}
```

### 4. 使用工具函数创建违规对象
```tsx
// ✅ 推荐：使用工具函数
const violation = createFrozenZoneViolation(['M123'], '材料已冻结');

// ❌ 不推荐：手动创建对象
const violation = {
  type: 'FROZEN_ZONE_PROTECTION',
  message: '材料已冻结',
  severity: 'error',
  // ...
};
```

---

## 测试建议

### 单元测试示例

```tsx
import { render, screen } from '@testing-library/react';
import { FrozenZoneBadge } from './FrozenZoneBadge';

test('displays badge when locked', () => {
  render(<FrozenZoneBadge locked={true} />);
  expect(screen.getByText('冻结区')).toBeInTheDocument();
});

test('does not display when unlocked', () => {
  const { container } = render(<FrozenZoneBadge locked={false} />);
  expect(container.firstChild).toBeNull();
});
```

---

## 常见问题

### Q: 什么时候使用badge模式？什么时候使用banner模式？
A:
- **Badge模式**: 在表格、列表等密集信息场景中使用
- **Banner模式**: 在详情页、表单页等需要强调的场景中使用

### Q: 如何处理多个违规？
A:
```tsx
const violations = [
  createFrozenZoneViolation(['M123'], '冻结'),
  createMaturityViolation(['M456'], 2),
];

<RedLineGuard violations={violations} mode="detailed" />
```

### Q: 可以自定义红线元数据吗？
A: 红线元数据在RedLineGuard.tsx的RED_LINE_META中定义，如需扩展可以修改该常量。

---

## 维护说明

- **文件位置**: `src/components/guards/`
- **集成位置**: MaterialManagement.tsx, MaterialInspector.tsx
- **依赖**: Ant Design, @ant-design/icons
- **后端对应**: src/domain/red_line.rs

更新日期: 2026-01-24
