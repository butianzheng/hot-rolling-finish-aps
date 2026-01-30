# 开发指南

**版本**: v1.0
**更新日期**: 2026-01-27

---

## 一、环境配置

### 1.1 必要工具

```bash
# Node.js (>= 18)
node --version

# Rust (>= 1.70)
rustc --version

# npm (>= 9)
npm --version
```

### 1.2 安装依赖

```bash
# 前端依赖
npm install

# Rust 依赖 (自动通过 Cargo 管理)
cargo check
```

---

## 二、开发命令

### 2.1 启动开发服务器

```bash
# 启动 Tauri 开发模式 (前端热更新 + Rust 自动编译)
npm run tauri dev
```

### 2.2 构建生产版本

```bash
# 构建前端
npm run build

# 构建完整应用
npm run tauri build
```

### 2.3 运行测试

```bash
# 全部 Rust 测试
cargo test

# 特定测试
cargo test --test full_business_flow_e2e_test

# 带日志输出
cargo test -- --nocapture
```

---

## 三、代码结构说明

### 3.1 Rust 后端

| 目录 | 说明 |
|-----|------|
| `src/domain/` | 领域模型 (Material, Plan, Capacity 等) |
| `src/repository/` | 数据库操作 (SQLite CRUD) |
| `src/engine/` | 业务引擎 (适温、紧急、产能等) |
| `src/api/` | API 接口实现 |
| `src/decision/` | 决策支持层 (D1-D6) |
| `src/importer/` | 数据导入模块 |
| `src/app/` | Tauri 应用集成 |

### 3.2 React 前端

| 目录 | 说明 |
|-----|------|
| `src/components/` | React 组件 |
| `src/pages/` | 页面组件 (DecisionBoard 等) |
| `src/hooks/` | React Hooks |
| `src/stores/` | Zustand 状态管理 |
| `src/api/` | IPC 通信层 |
| `src/types/` | TypeScript 类型定义 |

---

## 四、开发规范

### 4.1 必须遵守的工业红线

1. **冻结区保护** - 不要修改冻结区相关逻辑
2. **适温约束** - 不要绕过适温检查
3. **分层紧急** - 紧急等级只能是 L0-L3，不能用数值
4. **产能优先** - 产能约束不可被覆盖
5. **可解释性** - 所有决策必须有 reason 字段

### 4.2 代码风格

**Rust**:
- 使用 `cargo fmt` 格式化
- 使用 `cargo clippy` 检查

**TypeScript**:
- 使用 ESLint 规范
- 严格模式 (strict: true)

### 4.3 提交规范

```bash
# 提交信息格式
<type>(<scope>): <description>

# 示例
feat(engine): add new urgency calculation rule
fix(api): fix capacity overflow validation
docs: update architecture document
```

---

## 五、常见任务

### 5.1 添加新的 Tauri 命令

1. 在 `src/api/` 中添加 API 方法
2. 在 `src/app/tauri_commands.rs` 中注册命令
3. 在 `src/main.rs` 中添加到 `invoke_handler`
4. 在前端 `src/api/tauri.ts` 中添加调用

### 5.2 添加新的决策查询 (D7+)

1. 在 `src/decision/models/` 中定义数据模型
2. 在 `src/decision/repository/` 中添加数据仓储
3. 在 `src/decision/use_cases/` 中实现用例
4. 在 `src/decision/api/` 中暴露 API
5. 更新 `spec/DecisionApi_Contract_v1.0.md`

### 5.3 修改业务规则

1. **先阅读规范**: `spec/Claude_Dev_Master_Spec.md`
2. **确认影响范围**: 哪些引擎/API 会受影响
3. **编写测试**: 在 `tests/` 目录添加测试用例
4. **更新文档**: 同步更新规范文档

---

## 六、调试技巧

### 6.1 Rust 日志

```rust
// 添加日志
tracing::info!("Processing material: {}", material_id);
tracing::debug!("State: {:?}", state);
```

### 6.2 前端调试

```typescript
// IPC 调用会自动打印调试日志
// 查看浏览器控制台
```

### 6.3 数据库查看

```bash
# 数据库路径
~/.local/share/hot-rolling-aps-dev/hot_rolling_aps.db

# 使用 sqlite3 查看
sqlite3 ~/.local/share/hot-rolling-aps-dev/hot_rolling_aps.db ".tables"
```

---

## 七、问题排查

### 7.1 常见问题

| 问题 | 解决方案 |
|-----|---------|
| Tauri 启动失败 | 检查 Rust 环境，运行 `cargo check` |
| 前端热更新不生效 | 重启 `npm run tauri dev` |
| 数据库锁定 | 关闭其他数据库连接 |
| 测试超时 | 增加 `--test-threads=1` |

### 7.2 日志位置

- **Rust 日志**: 控制台输出
- **前端日志**: 浏览器开发者工具
- **Tauri 日志**: 应用日志目录
