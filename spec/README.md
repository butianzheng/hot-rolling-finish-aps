# 📋 系统规范文档

本目录包含热轧精整排产系统的所有规范与契约文档，是系统设计和开发的**最高权威**。

所有代码、设计和实施决策都必须严格遵循这些规范。

---

## 规范文档清单

### Master Spec（最高权威）✨

**[Claude_Dev_Master_Spec.md](./Claude_Dev_Master_Spec.md)** - v1.0
- 项目主控文档，开发宪法
- 系统整体设计理念和约束
- 工业红线定义
- 所有实施规范必须遵循

### 集成规范（v0.3 主版本）🔗

以下规范以 v0.3 为主版本；允许小版本补丁（如 API 契约 v0.3.1），并在本页记录：

**[Engine_Specs_v0.3_Integrated.md](./Engine_Specs_v0.3_Integrated.md)**
- 排产引擎工程规格书
- 核心引擎逻辑：成熟度引擎、合规引擎、容量引擎、编排引擎
- 引擎参数和配置说明
- 引擎间交互协议

**[Field_Mapping_Spec_v0.3_Integrated.md](./Field_Mapping_Spec_v0.3_Integrated.md)**
- 字段映射与口径说明书
- 前后端字段对应关系
- 数据类型定义和验证规则
- 业务语义和约束说明

**[Tauri_API_Contract_v0.3_Integrated.md](./Tauri_API_Contract_v0.3_Integrated.md)**
- Tauri API 契约
- 当前小版本：v0.3.1（补充 run_id / plan_rev / STALE_PLAN_REV 一致性）
- 前后端 API 接口定义
- 请求/响应数据结构
- 错误处理和状态码约定

### 单独规范📌

**[DecisionApi_Contract_v1.0.md](./DecisionApi_Contract_v1.0.md)** - v1.0
- 决策 API 契约规范
- 排产决策接口定义
- 策略选择和参数配置
- 决策结���数据结构

**[data_dictionary_v0.1.md](./data_dictionary_v0.1.md)** - v0.1（MVP）
- 数据字典
- 数据库表结构和字段说明
- 业务实体定义
- 数据关系和约束

---

## 版本策略

### Master Spec
- **版本管理**: 独立版本号（当前 v1.0）
- **更新频率**: 只在重大架构变更时更新
- **权威级别**: 最高，所有规范必须与之一致
- **更新流程**: 需系统架构师评审

### 集成规范（v0.3 主版本）
- **版本管理**: 三个文档保持主版本一致（当前主版本 v0.3）
- **更新频率**: 根据功能迭代定期同步更新
- **权威级别**: 高，实施层面的最高权威
- **更新流程**: 主版本统一更新；如仅局部补丁（如 API v0.3.1），需在本页登记并说明影响

### 单独规范
- **版本管理**: 各自独立版本号
- **更新频率**: 根据具体需求独立更新
- **权威级别**: 中，特定领域的权威
- **更新流程**: 可独立更新，但需同步更新 Master Spec 相关章节

---

## 规范版本记录

### Master Spec

| 版本 | 日期 | 变更摘要 |
|------|------|----------|
| v1.0 | 2025-xx-xx | 初始版本，系统宪法和约束定义 |

### 集成规范（统一版本）

| 版本 | 日期 | 变更摘要 |
|------|------|----------|
| v0.3.1 | 2026-02-08 | API 契约补丁：run_id / plan_rev / STALE_PLAN_REV 一致性 |
| v0.3 | 2025-xx-xx | 集成引擎、字段映射、API 契约规范 |
| v0.2 | 2025-xx-xx | 第二版集成规范 |
| v0.1 | 2025-xx-xx | 初始集成规范 |

### 单独规范

| 规范 | 版本 | 日期 | 变更摘要 |
|------|------|------|----------|
| DecisionApi_Contract | v1.0 | 2025-xx-xx | 决策 API 第一版 |
| data_dictionary | v0.1 | 2025-xx-xx | 数据字典 MVP 版本 |

---

## 规范更新流程

规范文档的更新必须遵循严格流程，确保系统设计一致性。

### 1. 准备阶段
- [ ] 确定需要更新的规范文档
- [ ] 评估变更影响范围
- [ ] 创建规范更新分支: `spec-update-<feature>`

### 2. 起草阶段
- [ ] 编写变更内容草案
- [ ] 标记变更部分（使用 `<!-- CHANGED v0.x -->` 注释）
- [ ] 更新版本号（集成规范需保持主版本一致；补丁号差异需记录）

### 3. 审查阶段
- [ ] 创建 PR 并提交审查
- [ ] **系统架构师必须审查**（对于 Master Spec）
- [ ] 技术主管审查（对于集成规范）
- [ ] 评审是否与 Master Spec 冲突

### 4. 同步更新
- [ ] 更新相关的集成规范（如果是 Engine/Field/API 之一）
- [ ] 更新 Master Spec 相关章节（如果是单独规范）
- [ ] 更新本 README.md 的版本记录表

### 5. 发布阶段
- [ ] 合并 PR
- [ ] 创建 Git Tag: `spec-v0.x`
- [ ] 发布 Release Notes
- [ ] 通知开发团队规范变更

---

## 规范冲突处理

当规范之间出现冲突时，遵循以下优先级：

### 优先级顺序

1. **Master Spec** - 最高权威
   - 定义系统约束和原则
   - 所有规范必须符合 Master Spec

2. **集成规范** - 实施权威
   - Engine_Specs ← 引擎行为定义
   - Field_Mapping ← 数据口径定义
   - Tauri_API_Contract ← 接口定义

3. **单独规范** - 特定领域权威
   - DecisionApi_Contract ← 决策 API 领域
   - data_dictionary ← 数据库领域

4. **实施文档** - 参考指南
   - docs/core/ARCHITECTURE.md
   - docs/core/DEVELOPMENT_GUIDE.md

### 冲突解决原则

**规则 1**: 下级规范不得与上级规范冲突
- 示例: Field_Mapping 中定义的字段映射不得违反 Master Spec 中的数据约束

**规则 2**: 同级规范冲突时，通过 Master Spec 判断
- 示例: Engine_Specs 和 Tauri_API_Contract 对引擎参数描述不一致时，参考 Master Spec 判断

**规则 3**: 发现冲突时必须停止开发并澄清
- 不允许开发者自行解释冲突，必须提 Issue 或 Discussion

---

## 常见问题

### Q1: 如何引用规范内容？

在代码注释或文档中使用以下格式：

```
// 遵循 Master Spec §3.2 - 冻结区保护
// 参考 Engine_Specs v0.3 §4.1 - 成熟度引擎
// 字段定义见 Field_Mapping v0.3 §2.3 - material_state
```

### Q2: 集成规范为什么要统一版本号？

集成规范（Engine/Field/API）三者紧密关联，必须保持一致性。统一版本号确保：
- 引擎逻辑、字段定义、API 接口同步更新
- 避免版本不一致导致的理解混乱
- 简化规范管理和追踪

补充：允许局部补丁号（例如 API 从 v0.3 补到 v0.3.1），但必须：
- 明确仅为局部契约增强
- 在本 README 的版本记录登记
- 评估对 Engine/Field/Decision 的兼容性

### Q3: 规范更新后，老代码怎么办？

- 新代码必须遵循新规范
- 老代码建议在下一个重构周期内更新
- 创建技术债务 Issue 追踪

### Q4: 我发现规范有错误，如何报告？

1. 创建 Issue，标题格式: `[Spec Bug] 规范名称: 错误描述`
2. 详细说明错误位置和建议修正方案
3. 等待架构师或维护者响应

---

## 相关资源

- [CLAUDE.md](../CLAUDE.md) - 开发规范和约束（简化版）
- [docs/core/ARCHITECTURE.md](../docs/core/ARCHITECTURE.md) - 系统架构实施文档
- [docs/INDEX.md](../docs/INDEX.md) - 文档导航索引
- [CONTRIBUTING.md](../CONTRIBUTING.md) - 贡献指南

---

**上次更新**: 2026-01-30
**维护者**: 系统架构师、技术主管
