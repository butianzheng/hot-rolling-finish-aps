# 📑 项目文档整理计划

**制定时间**: 2026-01-30
**状态**: 待执行
**目标**: 将 37 个文档整理为清晰、规范的文档体系

---

## 问题分析

### 当前文档结构问题

1. **分布不均**
   - 根目录 13 个文件 → 过多，混乱
   - docs/ 15 个文件 → 结构不清
   - spec/ 6 个文件 → 仍为规范集合

2. **重复和冗余**
   - 代码审查文档过多（5个）：
     - `CODE_REVIEW_GUIDE.md` (根目录)
     - `CODE_REVIEW_QUICK_REFERENCE.md` (根目录)
     - `CODE_REVIEW_EXECUTIVE_SUMMARY.md` (根目录)
     - `CODE_REVIEW_MEETING_AGENDA.md` (根目录)
     - `docs/CODE_REVIEW_REPORT.md` (docs/)

   - 报告文档重复（12 个报告在 2 个目录）
   - 多个代码质量评估文档：
     - `PROJECT_AUDIT_REPORT.md` (根目录)
     - `docs/CODE_REVIEW_REPORT.md` (docs/)
     - `docs/CODE_QUALITY_COMPREHENSIVE_REPORT.md` (docs/)
     - `docs/REVIEW_SUMMARY.md` (docs/)

3. **过期文档**
   - `UNIT_TEST_PLAN.md` - 已完成，内容已执行
   - `PERFORMANCE_MONITORING.md` - 还是计划状态，未来任务
   - `docs/REFACTOR_TODO.md` - 任务清单，应归档或更新
   - `docs/REVIEW_FIX_TODO.md` - 任务清单，应归档或更新

4. **版本混乱**
   - spec/ 文件版本号不一致（v0.1-v1.0）
   - 无清晰的版本管理策略

5. **缺少导航**
   - 无统一的文档索引
   - 文档间相互引用关系不清晰
   - 新开发者难以快速定位文档

---

## 整理方案

### 第一步：根目录简化

**目标**: 根目录仅保留 5 个核心文档

| 保留文档 | 用途 | 备注 |
|--------|------|------|
| `README.md` | 项目入口 | 保留，更新最新信息 |
| `CLAUDE.md` | 开发规范 | 保留（当前为 claude.md） |
| `ARCHITECTURE.md` | 系统设计 | 从 docs/ 移至根目录 |
| `CONTRIBUTING.md` | 贡献指南 | 新建 |
| `docs/INDEX.md` | 文档索引 | 新建，指向 docs/ |

| 迁移至 docs/archived/ | 原因 |
|--------|------|
| `FRONTEND_README.md` | 前端专用，应归档在 docs/frontend/ |
| `PROJECT_AUDIT_REPORT.md` | 审计报告，不属于核心文档 |
| `TECHNICAL_DEBT_REPORT.md` | 旧报告，已有新的 FINAL_WORK_SUMMARY.md |
| `UNIT_TEST_PLAN.md` | 已完成，归档到 docs/archived/reports/ |
| `PERFORMANCE_MONITORING.md` | 未来计划，归档 |
| `CODE_REVIEW_GUIDE.md` | 审查流程，移至 docs/process/ |
| `CODE_REVIEW_QUICK_REFERENCE.md` | 审查参考，移至 docs/process/ |
| `CODE_REVIEW_EXECUTIVE_SUMMARY.md` | 审查总结，移至 docs/archived/reports/ |
| `CODE_REVIEW_MEETING_AGENDA.md` | 会议议程，归档 |
| `FINAL_WORK_SUMMARY.md` | 工作总结，更新后归档 |
| `TEST_REPORT.md` | 测试报告，归档 |

**结果**: 根目录 13 → 5 个文件 (-62%)

---

### 第二步：docs/ 目录重构

**新目录结构**:

```
docs/
├── INDEX.md                           # 文档导航索引（新建）
├── core/                              # 核心文档
│   ├── ARCHITECTURE.md                # 从根目录移过来
│   ├── DEVELOPMENT_GUIDE.md           # 开发指南（现有）
│   ├── API_REFERENCE.md               # API 参考（现有）
│   └── IMPLEMENTATION_GUIDE.md        # 实现指南（现有）
├── specs/                             # 规范文档（符号链接到 ../spec/）
│   ├── Master_Spec.md                 # 从 spec/ 链接
│   ├── Engine_Specs.md                # 从 spec/ 链接
│   ├── Field_Mapping_Specs.md         # 从 spec/ 链接
│   ├── Tauri_API_Contract.md          # 从 spec/ 链接
│   ├── DecisionApi_Contract.md        # 从 spec/ 链接
│   └── Data_Dictionary.md             # 从 spec/ 链接
├── process/                           # 工作流程
│   ├── CODE_REVIEW_PROCESS.md         # 合并审查指南 + 参考（新建）
│   ├── REFACTOR_PROCESS.md            # 重构流程（新建）
│   └── TESTING_PROCESS.md             # 测试流程（新建）
├── frontend/                          # 前端相关文档
│   ├── REFACTOR_PLAN.md               # 从现有 FRONTEND_REFACTOR_PLAN.md
│   └── README.md                      # 从根目录 FRONTEND_README.md 移过来
├── reports/                           # 项目报告与总结
│   ├── FINAL_WORK_SUMMARY.md          # 最终工作总结
│   ├── P0_FIXES_SUMMARY.md            # P0 修复总结
│   ├── P1_PROGRESS.md                 # 合并：P1-3 和 P1-4 总结（新建）
│   └── QUALITY_METRICS.md             # 合并代码质量报告（新建）
├── archived/                          # 已完成的任务与历史报告
│   ├── UNIT_TEST_PLAN.md              # 已完成
│   ├── PERFORMANCE_MONITORING.md      # 未来计划
│   ├── REFACTOR_TODO.md               # 任务清单（已完成）
│   ├── REVIEW_FIX_TODO.md             # 任务清单（已完成）
│   ├── audit_reports/                 # 审计报告
│   │   ├── PROJECT_AUDIT_REPORT.md
│   │   ├── CODE_REVIEW_REPORT.md
│   │   └── CODE_QUALITY_COMPREHENSIVE_REPORT.md
│   ├── meeting_notes/                 # 会议记录
│   │   └── CODE_REVIEW_MEETING_AGENDA.md
│   └── review_summaries/              # 评审总结
│       ├── CODE_REVIEW_EXECUTIVE_SUMMARY.md
│       └── REVIEW_SUMMARY.md
├── guides/                            # 使用指南
│   ├── GUARDS_COMPONENT_GUIDE.md      # 从 src/components/guards/README.md 提升
│   └── TEST_DATA_GUIDE.md             # 从 tests/fixtures/datasets/README.md 提升
└── MIGRATION_NOTES.md                 # 文档迁移说明（新建）
```

**关键变化**:
- docs/ 由 15 个平铺文件 → 9 个清晰目录
- 报告数量: 12 个 → 分类整理，archived/ 存放历史报告
- 流程文档集中: 分散的 4 个 → process/ 目录统一管理

---

### 第三步：spec/ 目录清理

**现状**: 6 个版本不一的规范文档

**方案**:
1. 保留 spec/ 目录，不改动内容
2. 在 docs/specs/ 创建符号链接（或在 docs/INDEX.md 中详细链接）
3. 建立版本管理策略说明文档

**新建**: `spec/README.md` - 规范文档说明

```markdown
# 系统规范文档

本目录包含项目的所有规范与契约文档，是系统设计和开发的最高权威。

## 规范文档清单

### Master Spec（最高权威）
- [Claude_Dev_Master_Spec.md](./Claude_Dev_Master_Spec.md) - v1.0 - 项目主控文档

### 集成规范（v0.3）
- [Engine_Specs_v0.3_Integrated.md](./Engine_Specs_v0.3_Integrated.md) - 排产引擎规格
- [Field_Mapping_Spec_v0.3_Integrated.md](./Field_Mapping_Spec_v0.3_Integrated.md) - 字段映射规范
- [Tauri_API_Contract_v0.3_Integrated.md](./Tauri_API_Contract_v0.3_Integrated.md) - API 契约

### 单独规范
- [DecisionApi_Contract_v1.0.md](./DecisionApi_Contract_v1.0.md) - 决策 API v1.0
- [data_dictionary_v0.1.md](./data_dictionary_v0.1.md) - 数据字典 v0.1

## 版本策略

- **Master Spec**: 最高权威，所有实施规范必须遵循
- **集成规范**: 统一版本管理（当前 v0.3）
- **单独规范**: 独立版本管理（API/数据字典等）

## 更新流程

规范文档更新需遵循以下流程：
1. 创建更新分支 `spec-update-<feature>`
2. 必须同步更新 Master Spec 中的相关章节
3. 必须通过代码审查（特别是系统架构师审查）
4. 版本号必须递增（v0.3 → v0.4 或 v1.0）
5. 在本 README.md 中更新版本记录
```

---

### 第四步：新建文档

#### 1. `docs/INDEX.md` - 文档导航索引

```markdown
# 📚 项目文档导航

欢迎来到热轧精整排产系统文档中心。本页提供快速导航，帮助您快速找到所需文档。

## 🚀 快速开始

**新开发者推荐阅读顺序**:
1. [README.md](../README.md) - 项目简介
2. [CLAUDE.md](../CLAUDE.md) - 开发规范与约束 ⚠️ 必读
3. [docs/core/ARCHITECTURE.md](./core/ARCHITECTURE.md) - 系统架构
4. [docs/core/DEVELOPMENT_GUIDE.md](./core/DEVELOPMENT_GUIDE.md) - 开发指南

---

## 📖 文档目录

### 🎯 核心文档
- [系统架构](./core/ARCHITECTURE.md) - 系统设计与模块划分
- [开发指南](./core/DEVELOPMENT_GUIDE.md) - 开发环境与工作流程
- [API 参考](./core/API_REFERENCE.md) - API 接口文档
- [实现指南](./core/IMPLEMENTATION_GUIDE.md) - 关键修复方案

### 📋 规范与契约
所有规范文档在 [spec/](../spec/) 目录中，这是最高权威。
- **Master Spec**: 项目主控文档（必读）
- **集成规范 v0.3**: 引擎、字段映射、API 契约
- **单独规范**: DecisionApi、数据字典

[查看规范文档详情](../spec/README.md)

### 🔄 工作流程
- [代码审查流程](./process/CODE_REVIEW_PROCESS.md) - 代码评审指南��标准
- [重构流程](./process/REFACTOR_PROCESS.md) - 技术债务与重构指南
- [测试流程](./process/TESTING_PROCESS.md) - 单元测试与质量保证

### 🖼️ 前端文档
- [前端重构方案](./frontend/REFACTOR_PLAN.md) - 前端架构优化计划
- [前端说明](./frontend/README.md) - 前端项目详情

### 📊 项目报告
- [最终工作总结](./reports/FINAL_WORK_SUMMARY.md) - 本周期工作总结与成果
- [P0 修复总结](./reports/P0_FIXES_SUMMARY.md) - 关键问题修复清单
- [P1 进展](./reports/P1_PROGRESS.md) - 技术债务修复进度
- [质量指标](./reports/QUALITY_METRICS.md) - 代码质量评估

### 📚 使用指南
- [工业防护组件指南](./guides/GUARDS_COMPONENT_GUIDE.md) - 红线保护组件使用说明
- [测试数据指南](./guides/TEST_DATA_GUIDE.md) - 测试数据集说明

### 🗃️ 历史文档与归档
- [已完成任务](./archived/) - 已完成的计划与任务清单
- [审计报告](./archived/audit_reports/) - 历史审计报告
- [会议记录](./archived/meeting_notes/) - 历史会议记录

[查看完整归档](./archived/)

---

## 🔍 按用途快速查找

### 我是...

**系统设计师**
→ 阅读: Master Spec, ARCHITECTURE.md, 所有集成规范

**前端开发者**
→ 阅读: DEVELOPMENT_GUIDE.md, FRONTEND_REFACTOR_PLAN.md, ARCHITECTURE.md

**后端开发者**
→ 阅读: DEVELOPMENT_GUIDE.md, Engine_Specs, Tauri_API_Contract

**测试人员**
→ 阅读: TESTING_PROCESS.md, TEST_DATA_GUIDE.md

**代码审查人**
→ 阅读: CODE_REVIEW_PROCESS.md, CLAUDE.md

**项目经理**
→ 阅读: FINAL_WORK_SUMMARY.md, P1_PROGRESS.md, QUALITY_METRICS.md

---

## 📅 文档更新日志

| 日期 | 文档 | 变更 |
|------|------|------|
| 2026-01-30 | INDEX.md | 新建文档导航索引 |
| 2026-01-30 | docs/ | 重构目录结构 |

---

## ✅ 文档维护清单

- [ ] 每次提交规范文档时更新版本号
- [ ] 每周检查报告文档并更新进度
- [ ] 每月审查文档结构和链接完整性
- [ ] 当项目阶段完成时，归档相关任务清单

---

*上次更新: 2026-01-30*
```

#### 2. 合并代码审查文档

**新建**: `docs/process/CODE_REVIEW_PROCESS.md` (合并现有 4 个文档)

内容整合:
- 目的和原则 (来自 CODE_REVIEW_GUIDE.md)
- 审查清单 (来自 CODE_REVIEW_QUICK_REFERENCE.md)
- 会议流程 (来自 CODE_REVIEW_MEETING_AGENDA.md)
- 审查标准和检验点 (来自其他文档)

#### 3. 合并 P1 进度报告

**新建**: `docs/reports/P1_PROGRESS.md` (合并 P1-3 和 P1-4 总结)

结构:
- P1-1: API 接口重构 (链接到 P1_API_REFACTOR_PLAN.md)
- P1-3: API 类型验证 (来自 P1-3_API_TYPE_VALIDATION_SUMMARY.md)
- P1-4: 组件分解 (来自 P1-4_COMPONENT_DECOMPOSITION_SUMMARY.md)

#### 4. 合并质量指标

**新建**: `docs/reports/QUALITY_METRICS.md` (合并代码质量评估)

合并的文档:
- PROJECT_AUDIT_REPORT.md 的关键指标
- CODE_REVIEW_REPORT.md 的评分
- CODE_QUALITY_COMPREHENSIVE_REPORT.md 的详细数据

---

### 第五步：迁移规则

#### 删除文件 ❌
- 完全过期无用的文件（无）

#### 保留在根目录 ✅
```
README.md                    # 项目入口（保留）
CLAUDE.md                    # 开发规范（保留）
ARCHITECTURE.md              # 系统设计（从 docs/ 移上来）
CONTRIBUTING.md              # 贡献指南（新建）
docs/INDEX.md               # 文档索引（指向 docs/）
```

#### 迁移至 docs/core/ 📁
```
docs/ARCHITECTURE.md → docs/core/ARCHITECTURE.md（已有）
docs/DEVELOPMENT_GUIDE.md → docs/core/DEVELOPMENT_GUIDE.md
docs/API_REFERENCE.md → docs/core/API_REFERENCE.md
docs/IMPLEMENTATION_GUIDE.md → docs/core/IMPLEMENTATION_GUIDE.md
```

#### 迁移至 docs/process/ 📁
```
CODE_REVIEW_GUIDE.md → docs/process/CODE_REVIEW_PROCESS.md（合并）
CODE_REVIEW_QUICK_REFERENCE.md → docs/process/CODE_REVIEW_PROCESS.md（合并）
CODE_REVIEW_MEETING_AGENDA.md → docs/process/CODE_REVIEW_PROCESS.md（合并）
新建: docs/process/REFACTOR_PROCESS.md
新建: docs/process/TESTING_PROCESS.md
```

#### 迁移至 docs/frontend/ 📁
```
FRONTEND_README.md → docs/frontend/README.md
docs/FRONTEND_REFACTOR_PLAN.md → docs/frontend/REFACTOR_PLAN.md
```

#### 合并至 docs/reports/ 📁
```
P1-3_API_TYPE_VALIDATION_SUMMARY.md ┐
P1-4_COMPONENT_DECOMPOSITION_SUMMARY.md ┘ → docs/reports/P1_PROGRESS.md

PROJECT_AUDIT_REPORT.md ┐
CODE_REVIEW_REPORT.md ├→ docs/reports/QUALITY_METRICS.md
CODE_QUALITY_COMPREHENSIVE_REPORT.md ┘
REVIEW_SUMMARY.md ┘

TEST_REPORT.md → 保留，放在 docs/reports/
FINAL_WORK_SUMMARY.md → 保留，放在 docs/reports/
```

#### 迁移至 docs/archived/ 📁
```
UNIT_TEST_PLAN.md → docs/archived/UNIT_TEST_PLAN.md
PERFORMANCE_MONITORING.md → docs/archived/PERFORMANCE_MONITORING.md
REFACTOR_TODO.md → docs/archived/REFACTOR_TODO.md
REVIEW_FIX_TODO.md → docs/archived/REVIEW_FIX_TODO.md
CODE_REVIEW_EXECUTIVE_SUMMARY.md → docs/archived/review_summaries/

以及来自根目录的已归档文件:
PROJECT_AUDIT_REPORT.md → docs/archived/audit_reports/
CODE_REVIEW_REPORT.md → docs/archived/audit_reports/
CODE_QUALITY_COMPREHENSIVE_REPORT.md → docs/archived/audit_reports/
CODE_REVIEW_MEETING_AGENDA.md → docs/archived/meeting_notes/
TECHNICAL_DEBT_REPORT.md → docs/archived/audit_reports/
```

#### 提升至 docs/guides/ 📁
```
tests/fixtures/datasets/README.md → docs/guides/TEST_DATA_GUIDE.md
src/components/guards/README.md → docs/guides/GUARDS_COMPONENT_GUIDE.md
```

---

## 执行步骤

### 阶段 1: 准备 (5 分钟)
- [ ] 创建 git 分支 `docs-refactor`
- [ ] 创建 docs/specs/ 等新目录结构

### 阶段 2: 创建新文档 (20 分钟)
- [ ] 创建 docs/INDEX.md
- [ ] 合并创建 docs/process/CODE_REVIEW_PROCESS.md
- [ ] 合并创建 docs/reports/P1_PROGRESS.md
- [ ] 合并创建 docs/reports/QUALITY_METRICS.md
- [ ] 创建 spec/README.md
- [ ] 创建 CONTRIBUTING.md

### 阶段 3: 文件迁移 (15 分钟)
- [ ] 迁移文档至 docs/core/, docs/process/, docs/frontend/ 等
- [ ] 迁移过期文档至 docs/archived/
- [ ] 删除根目录已迁移文件

### 阶段 4: 验证与修复 (10 分钟)
- [ ] 验证所有文档链接完整性
- [ ] 更新交叉引用
- [ ] 检查 README.md 文档引用是否指向正确位置

### 阶段 5: 提交 (5 分钟)
- [ ] git add .
- [ ] git commit -m "docs: 重构文档结构，优化分类和导航"
- [ ] 创建 PR 供审查

---

## 预期结果

### 根目录
**修改前**: 13 个文件
**修改后**: 5 个文件 (README.md, CLAUDE.md, ARCHITECTURE.md, CONTRIBUTING.md, 指向 docs/)
**改进**: -62% 文件数，结构清晰

### docs/ 目录
**修改前**: 15 个平铺文件
**修改后**: 9 个分类目录
**改进**: 逻辑分组，易于导航

### 总体
**修改前**: 37 个文档文件
**修改后**: ~30 个文档文件（删除重复，保留核心）
**新增**: 3 个导航/索引文档，1 个规范说明文档

---

## 注意事项

⚠️ **重要**:
1. 不修改任何 spec/ 文件内容，仅添加 README.md
2. 合并文档时确保所有内容都被保留
3. 更新所有内部链接和交叉引用
4. 保留 git 历史，使用 git mv 而非 cp + rm

✅ **后续维护**:
1. 定期清理 archived/ 目录（每季度一次）
2. 维护 docs/INDEX.md 为最新
3. 更新报告文档时遵循文件夹结构
4. 新文档创建时参考本计划中的分类

---

*计划制定: 2026-01-30*
*版本: 1.0*
*维护人: 代码质量团队*
