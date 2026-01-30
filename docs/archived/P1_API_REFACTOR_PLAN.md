# P1: API接口重构计划 - 消除重复定义

> **任务**: 消除6对重复的API定义（带`_full`后缀的版本）
> **优先级**: P1
> **预计工作量**: 1-2天
> **状态**: 进行中

---

## 问题分析

### 当前设计问题

目前的API设计存在重复定义：
```rust
// ❌ 问题：两个版本的方法
pub fn get_most_risky_date(&self, version_id: &str) -> ApiResult<...> {
    self.get_most_risky_date_full(version_id, None, None, None, Some(10))
}

pub fn get_most_risky_date_full(
    &self,
    version_id: &str,
    date_from: Option<&str>,
    date_to: Option<&str>,
    risk_level_filter: Option<Vec<String>>,
    limit: Option<u32>,
) -> ApiResult<...> {
    // 实际实现
}
```

**问题**：
1. 维护两个方法增加维护成本
2. 调用者需要选择使用哪个版本
3. 违反DRY原则（Don't Repeat Yourself）
4. 增加API表面积

---

## 重构方案

### 设计原则

✅ **保留单一入口**：每个功能只有一个方法
✅ **使用Option参数**：可选参数用Option<T>表示
✅ **向后兼容**：保持原有方法签名逻辑
✅ **清晰文档**：明确说明参数默认值

### 重构模式

```rust
// ✅ 重构后：单一方法with可选参数
pub fn get_most_risky_date(
    &self,
    version_id: &str,
    date_from: Option<&str>,        // 新增可选参数
    date_to: Option<&str>,          // 新增可选参数
    risk_level_filter: Option<Vec<String>>,  // 新增可选参数
    limit: Option<u32>,             // 新增可选参数，默认10
) -> ApiResult<DecisionDaySummaryResponse> {
    // 合并后的实现
}
```

---

## 重构清单

### dashboard_api.rs (4对重复)

| # | 基础方法 | _full方法 | 状态 |
|---|---------|----------|------|
| 1 | `get_most_risky_date` (235行) | `get_most_risky_date_full` (256行) | 🔄 待重构 |
| 2 | `get_unsatisfied_urgent_materials` (300行) | `get_unsatisfied_urgent_materials_full` (324行) | 🔄 待重构 |
| 3 | `get_cold_stock_materials` (367行) | `get_cold_stock_materials_full` (388行) | 🔄 待重构 |
| 4 | `get_most_congested_machine` (425行) | `get_most_congested_machine_full` (459行) | 🔄 待重构 |

### app/tauri_commands.rs (对应的命令层)

需要同步更新的Tauri命令：
- `get_most_risky_date`
- `get_unsatisfied_urgent_materials`
- `get_cold_stock_materials`
- `get_most_congested_machine`

### 前端调用 (src/api/tauri.ts)

需要更新前端API调用（添加可选参数）

---

## 详细重构步骤

### 步骤 1: 重构 `get_most_risky_date`

**原有签名**：
```rust
// 基础版 (235行)
pub fn get_most_risky_date(&self, version_id: &str) -> ApiResult<...>

// _full版 (256行)
pub fn get_most_risky_date_full(
    &self,
    version_id: &str,
    date_from: Option<&str>,
    date_to: Option<&str>,
    risk_level_filter: Option<Vec<String>>,
    limit: Option<u32>,
) -> ApiResult<...>
```

**重构后签名**：
```rust
pub fn get_most_risky_date(
    &self,
    version_id: &str,
    date_from: Option<&str>,
    date_to: Option<&str>,
    risk_level_filter: Option<Vec<String>>,
    limit: Option<u32>,
) -> ApiResult<DecisionDaySummaryResponse>
```

**实现逻辑**：
- 将`_full`版本的实现移到基础版本
- 删除`_full`版本方法
- 默认limit为`Some(10)`（如果传入None）

---

### 步骤 2: 重构 `get_unsatisfied_urgent_materials`

**原有签名**：
```rust
// 基础版 (300行)
pub fn get_unsatisfied_urgent_materials(&self, version_id: &str) -> ApiResult<...>

// _full版 (324行)
pub fn get_unsatisfied_urgent_materials_full(
    &self,
    version_id: &str,
    fail_type_filter: Option<Vec<String>>,
    urgency_level_filter: Option<Vec<String>>,
    limit: Option<u32>,
) -> ApiResult<...>
```

**重构后签名**：
```rust
pub fn get_unsatisfied_urgent_materials(
    &self,
    version_id: &str,
    fail_type_filter: Option<Vec<String>>,
    urgency_level_filter: Option<Vec<String>>,
    limit: Option<u32>,
) -> ApiResult<OrderFailureSetResponse>
```

---

### 步骤 3: 重构 `get_cold_stock_materials`

**原有签名**：
```rust
// 基础版 (367行)
pub fn get_cold_stock_materials(&self, version_id: &str) -> ApiResult<...>

// _full版 (388行)
pub fn get_cold_stock_materials_full(
    &self,
    version_id: &str,
    machine_codes: Option<Vec<String>>,
    pressure_level_filter: Option<Vec<String>>,
    limit: Option<u32>,
) -> ApiResult<...>
```

**重构后签名**：
```rust
pub fn get_cold_stock_materials(
    &self,
    version_id: &str,
    machine_codes: Option<Vec<String>>,
    pressure_level_filter: Option<Vec<String>>,
    limit: Option<u32>,
) -> ApiResult<ColdStockProfileResponse>
```

---

### 步骤 4: 重构 `get_most_congested_machine`

**原有签名**：
```rust
// 基础版 (425行)
pub fn get_most_congested_machine(&self, version_id: &str) -> ApiResult<...>

// _full版 (459行)
pub fn get_most_congested_machine_full(
    &self,
    version_id: &str,
    date_from: Option<&str>,
    date_to: Option<&str>,
    machine_codes: Option<Vec<String>>,
    bottleneck_level_filter: Option<Vec<String>>,
    limit: Option<u32>,
) -> ApiResult<...>
```

**重构后签名**：
```rust
pub fn get_most_congested_machine(
    &self,
    version_id: &str,
    date_from: Option<&str>,
    date_to: Option<&str>,
    machine_codes: Option<Vec<String>>,
    bottleneck_level_filter: Option<Vec<String>>,
    limit: Option<u32>,
) -> ApiResult<MachineBottleneckProfileResponse>
```

---

### 步骤 5: 更新 Tauri 命令层

需要更新 `src/app/tauri_commands.rs` 中的对应命令，添加可选参数。

**示例**：
```rust
#[tauri::command(rename_all = "snake_case")]
pub async fn get_most_risky_date(
    state: State<'_, AppState>,
    version_id: String,
    date_from: Option<String>,      // 新增
    date_to: Option<String>,        // 新增
    risk_level_filter: Option<Vec<String>>,  // 新增
    limit: Option<u32>,             // 新增
) -> Result<String, String> {
    // ...
}
```

---

### 步骤 6: 更新前端 API 调用

更新 `src/api/tauri.ts` 中的方法签名：

```typescript
// 修改前
async getMostRiskyDate(versionId: string): Promise<any> {
  return IpcClient.call('get_most_risky_date', { version_id: versionId });
}

// 修改后
async getMostRiskyDate(
  versionId: string,
  options?: {
    dateFrom?: string;
    dateTo?: string;
    riskLevelFilter?: string[];
    limit?: number;
  }
): Promise<any> {
  return IpcClient.call('get_most_risky_date', {
    version_id: versionId,
    date_from: options?.dateFrom,
    date_to: options?.dateTo,
    risk_level_filter: options?.riskLevelFilter,
    limit: options?.limit,
  });
}
```

---

## 向后兼容性分析

### Rust层
✅ **完全向后兼容** - 新增的参数都是`Option<T>`，调用者可以传`None`

### Tauri命令层
⚠️ **可能需要调整** - 如果前端已经使用了这些命令，需要确保：
1. 可选参数在Tauri命令中也是可选的
2. 使用`#[serde(default)]`或Option<T>确保向后兼容

### 前端层
⚠️ **API签名变化** - 前端需要更新调用方式：
- 如果不需要高级功能，可以不传options参数
- 如果需要筛选或分页，可以传入options对象

---

## 测试策略

### 单元测试
- [ ] 测试默认参数行为（limit=10）
- [ ] 测试可选参数传入
- [ ] 测试参数验证（version_id不能为空等）

### 集成测试
- [ ] 测试Tauri命令层调用
- [ ] 测试前端API调用
- [ ] 测试向后兼容性

### 手动测试
- [ ] 在前端UI中验证D1-D4决策看板功能正常
- [ ] 验证筛选和分页功能
- [ ] 验证错误处理

---

## 风险评估

| 风险 | 等级 | 缓解措施 |
|------|------|---------|
| 破坏现有调用 | 🟡 中 | 添加可选参数保持向后兼容 |
| 前端调用失败 | 🟡 中 | 更新前端调用，添加测试 |
| 文档不同步 | 🟢 低 | 同步更新注释和文档 |
| 测试覆盖不足 | 🟡 中 | 添加单元和集成测试 |

---

## 预期收益

### 维护性提升
- 减少4对重复方法（8个方法 → 4个方法）
- API表面积减少50%
- 代码行数减少约200行

### 可读性提升
- 单一入口，调用者不需要选择版本
- 清晰的可选参数语义

### 扩展性提升
- 未来添加新参数更容易
- 不需要创建新的`_full2`版本

---

## 执行时间表

| 阶段 | 任务 | 预计时间 |
|------|------|---------|
| 1 | dashboard_api.rs重构 | 2小时 |
| 2 | tauri_commands.rs更新 | 1小时 |
| 3 | 前端API更新 | 1小时 |
| 4 | 测试和验证 | 2小时 |
| 5 | 文档更新 | 30分钟 |
| **总计** | | **6.5小时** |

---

## 检查清单

### 代码修改
- [ ] dashboard_api.rs - 合并4对方法
- [ ] tauri_commands.rs - 更新4个命令
- [ ] tauri.ts - 更新4个前端方法
- [ ] 删除所有`_full`方法引用

### 测试
- [ ] cargo check - 编译通过
- [ ] cargo test - 单元测试通过
- [ ] 前端类型检查 - npx tsc --noEmit
- [ ] 手动UI测试 - D1-D4看板

### 文档
- [ ] API方法注释更新
- [ ] 本重构计划文档
- [ ] CHANGELOG.md更新

---

**状态**: 📝 计划完成，准备执行
**开始时间**: 2026-01-29
**预计完成时间**: 2026-01-29 (当天内)

