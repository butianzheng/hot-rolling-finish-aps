# 重量显示精度统一修改总结

## 修改目标

将所有页面和弹窗中显示的重量相关数字从保留2位小数改为保留3位小数（四舍五入）。

## 修改时间

2026-02-06 00:14

## 修改范围

### 1. 核心格式化函数（2个文件）

#### 1.1 `src/utils/formatters.ts`

**修改内容**：
- `formatWeight()`: 从 `toFixed(2)` 改为 `toFixed(3)`
- `formatCapacity()`: 从 `toFixed(2)` 改为 `toFixed(3)`

**影响**：
- 所有使用这两个函数的组件自动应用新精度
- 包括表格列定义中的格式化

#### 1.2 `src/utils/formatters.test.ts`

**修改内容**：
- 更新 `formatWeight` 测试用例的期望值（2位→3位）
- 更新 `formatCapacity` 测试用例的期望值（2位→3位）

**测试结果**：
- ✅ 43个测试全部通过

### 2. 直接使用 toFixed 的组件（12个文件）

以下文件中直接使用 `toFixed(2)` 格式化重量/产能的代码已全部更新为 `toFixed(3)`：

| 文件路径 | 修改内容 | 影响范围 |
|---------|---------|---------|
| `src/components/CapacityImpactPanel/index.tsx` | weight_t.toFixed(3) | 产能影响面板 |
| `src/components/material-inspector/BasicInfoSection.tsx` | weight_t.toFixed(3) | 材料检查器-基本信息 |
| `src/components/material-pool/MaterialPoolRow.tsx` | weight.toFixed(3) | 材料池行组件 |
| `src/components/overview/drilldown/ColdStockContent.tsx` | weightT.toFixed(3) | 概览-冷坨内容 |
| `src/components/plan-item-visualization/StatisticsCards.tsx` | total_weight.toFixed(3) | 计划项可视化-统计卡片 |
| `src/components/schedule-gantt-view/index.tsx` | weightT.toFixed(3) | 排产甘特图 |
| `src/components/workbench/ConditionalSelectModal.tsx` | weight.toFixed(3) | 工作台-条件选择对话框 |
| `src/components/material-management/materialTableColumns.tsx` | toFixed(3) | 材料管理表格列 |
| `src/pages/DecisionBoard/D3ColdStock.tsx` | weightT.toFixed(3) | 决策看板-冷坨分析 |
| `src/pages/DecisionBoard/D5RollCampaign.tsx` | weight.toFixed(3) | 决策看板-换辊计划 |
| `src/services/capacityImpactService.ts` | capacity.toFixed(3) | 产能影响服务 |
| `src/utils/operabilityStatus.ts` | capacity.toFixed(3) | 可操作性状态工具 |

### 3. 使用格式化函数的组件（自动更新）

以下组件使用了 `formatWeight` 或 `formatCapacity` 函数，因此**自动应用**新的3位小数精度：

- `src/components/plan-item-visualization/planItemColumns.tsx` - 使用 `formatWeight`
- `src/components/capacity-pool-management/capacityPoolColumns.tsx` - 使用 `formatCapacity`
- 其他所有导入并使用这两个函数的组件

### 4. 未修改的内容

以下内容使用 `toFixed(1)` 但**不是重量**，因此保持不变：

- 堵塞分数（0-100分）- `bottleneckScore.toFixed(1)`
- 风险分数（0-100分）- `riskScore.toFixed(1)`
- 权重百分比 - `(weight * 100).toFixed(1)%`（这里的weight是权重系数，不是吨位）
- 产能利用率百分比 - `capacityUtilization.toFixed(1)%`

## 验证方法

### 前端测试

```bash
# 运行格式化函数测试
npm run test -- src/utils/formatters.test.ts

# 结果: ✅ 43 个测试全部通过
```

### 视觉验证

1. **产能池管理页面**
   - 路径: http://localhost:5173（产能池管理）
   - 检查: 目标产能、限制产能、已用产能、可用产能
   - 预期: 所有数字显示为 `XXXX.XXX` 格式

2. **材料管理页面**
   - 路径: http://localhost:5173（材料管理）
   - 检查: 材料重量列
   - 预期: 显示为 `XX.XXXt` 格式

3. **决策看板**
   - 路径: http://localhost:5173/decision/*
   - 检查: D3冷坨分析、D5换辊计划中的重量数据
   - 预期: 所有重量显示3位小数

4. **工作台**
   - 路径: http://localhost:5173/workbench
   - 检查: 材料列表、条件选择等位置的重量显示
   - 预期: 3位小数精度

## 回滚方案

如需回滚，所有修改前的文件已备份到：
```
backups/weight_format_YYYYMMDD_HHMMSS/
```

恢复步骤：
```bash
# 1. 停止应用
# 2. 从备份恢复文件
cp backups/weight_format_YYYYMMDD_HHMMSS/*.bak 到对应位置
# 3. 重启应用
```

## 影响评估

### 正面影响
- ✅ 重量数据显示更精确
- ✅ 统一的格式化标准
- ✅ 满足工业精度需求

### 潜在影响
- ⚠️ 表格列宽可能略有增加（多1个字符）
- ⚠️ 用户需要适应新的显示格式
- ⚠️ 截图/文档中的历史数据可能显示2位小数（正常）

### 无影响
- ✅ 数据库存储精度不变（仍为float/double）
- ✅ 计算逻辑不变
- ✅ API返回数据不变（仅前端显示变化）

## 完成状态

- [x] 修改核心格式化函数
- [x] 更新所有直接使用toFixed的组件
- [x] 更新测试用例
- [x] 运行测试验证（43/43通过）
- [x] 创建总结文档
- [ ] 用户验证显示效果

## 相关代码示例

### 修改前
```typescript
// formatters.ts
export const formatWeight = (value: number) => {
  return `${value.toFixed(2)}t`;
};

// 组件中
{material.weight_t.toFixed(2)}t
```

### 修改后
```typescript
// formatters.ts
export const formatWeight = (value: number) => {
  return `${value.toFixed(3)}t`;
};

// 组件中
{material.weight_t.toFixed(3)}t
```

### 显示效果对比

| 原值 | 修改前 | 修改后 |
|------|--------|--------|
| 123.456789 | 123.46t | 123.457t |
| 100.123456 | 100.12t | 100.123t |
| 0.001 | 0.00t | 0.001t |
| 1800.0 | 1800.00t | 1800.000t |

---

**修改人**: Claude Code
**日期**: 2026-02-06
**版本**: v1.0
