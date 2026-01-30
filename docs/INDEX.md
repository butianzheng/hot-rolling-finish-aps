# 📚 项目文档导航

欢迎来到热轧精整排产系统文档中心。本页提供快速导航，帮助您快速找到所需文档。

---

## 🚀 快速开始

**新开发者推荐阅读顺序**:
1. [README.md](../README.md) - 项目简介与快速开始
2. [CLAUDE.md](../CLAUDE.md) - 开发规范与约束 ⚠️ **必读**
3. [系统架构](./core/ARCHITECTURE.md) - 系统设计与模块划分
4. [开发指南](./core/DEVELOPMENT_GUIDE.md) - 开发环境与工作流程

---

## 📖 文档目录

### 🎯 核心文档 (`docs/core/`)
项目开发和理解系统的基础文档。

- **[系统架构](./core/ARCHITECTURE.md)** - 系统设计、模块划分、技术栈
- **[开发指南](./core/DEVELOPMENT_GUIDE.md)** - 开发环境搭建、工作流程、最佳实践
- **[API 参考](./core/API_REFERENCE.md)** - Tauri API、后端接口快速查询
- **[实现指南](./core/IMPLEMENTATION_GUIDE.md)** - 关键功能实现方案、修复方案

### 📋 规范与契约 (`spec/`)
系统设计和实施的最高权威文档。

**Master Spec（最高权威）**
- [Claude_Dev_Master_Spec.md](../spec/Claude_Dev_Master_Spec.md) - 项目主控文档 v1.0

**集成规范（v0.3）**
- [Engine_Specs_v0.3_Integrated.md](../spec/Engine_Specs_v0.3_Integrated.md) - 排产引擎工程规格书
- [Field_Mapping_Spec_v0.3_Integrated.md](../spec/Field_Mapping_Spec_v0.3_Integrated.md) - 字段映射与口径说明
- [Tauri_API_Contract_v0.3_Integrated.md](../spec/Tauri_API_Contract_v0.3_Integrated.md) - Tauri API 契约

**单独规范**
- [DecisionApi_Contract_v1.0.md](../spec/DecisionApi_Contract_v1.0.md) - 决策 API 契约 v1.0
- [data_dictionary_v0.1.md](../spec/data_dictionary_v0.1.md) - 数据字典 v0.1（MVP）

**[规范文档说明](../spec/README.md)** - 规范版本管理和更新流程

### 🔄 工作流程 (`docs/process/`)
开发、审查、测试的标准流程和检查清单。

- **[代码审查流程](./process/CODE_REVIEW_PROCESS.md)** - 审查标准、检查清单、会议流程
- **[重构流程](./process/REFACTOR_PROCESS.md)** - 技术债务处理、重构方法论
- **[测试流程](./process/TESTING_PROCESS.md)** - 单元测试、集成测试、质量保证流程

### 🖼️ 前端文档 (`docs/frontend/`)
前端架构、重构方案、组件使用指南。

- **[前端重构方案](./frontend/REFACTOR_PLAN.md)** - 前端架构优化计划、组件分解
- **[前端说明](./frontend/README.md)** - 前端项目结构、模块说明

### 📊 项目报告 (`docs/reports/`)
周期性的工作总结、质量评估、进度报告。

**最新报告**
- **[最终工作总结](./reports/FINAL_WORK_SUMMARY.md)** - 本周期完成的工作、成果、质量指标
- **[单元测试报告](./reports/TEST_REPORT.md)** - 测试执行结果、覆盖率、性能指标
- **[P1 进展](./reports/P1_PROGRESS.md)** - 技术债务修复进度总结
  - P1-3: API 类型验证完成总结
  - P1-4: 组件分解完成总结
- **[P0 修复总结](./reports/P0_FIXES_SUMMARY.md)** - 关键问题修复清单

**质量评估**
- **[质量指标](./reports/QUALITY_METRICS.md)** - 代码质量评估、审计结果汇总

### 📚 使用指南 (`docs/guides/`)
特定功能或组件的使用说明。

- **[工业防护组件指南](./guides/GUARDS_COMPONENT_GUIDE.md)** - 红线保护组件使用、工业约束实现
- **[测试数据指南](./guides/TEST_DATA_GUIDE.md)** - 测试数据集说明、模拟数据生成

### 🗃️ 历史文档与归档 (`docs/archived/`)
已完成的任务、历史报告、过期计划。

- **[任务清单](./archived/)** - 已完成的项目任务清单
  - UNIT_TEST_PLAN.md - 单元测试补充计划（已完成）
  - REFACTOR_TODO.md - 重构进度清单（已完成）
  - REVIEW_FIX_TODO.md - 修复任务清单（已完成）

- **[审计报告](./archived/audit_reports/)** - 历史审计与代码评估报告
- **[会议记录](./archived/meeting_notes/)** - 历史会议议程和记录
- **[评审总结](./archived/review_summaries/)** - 历史代码审查总结

---

## 🔍 按用途快速查找

### 我是...

**👨‍💼 系统设计师 / 架构师**
→ 必读: [Claude_Dev_Master_Spec.md](../spec/Claude_Dev_Master_Spec.md)
→ 然后: [系统架构](./core/ARCHITECTURE.md)，所有集成规范
→ 参考: [质量指标](./reports/QUALITY_METRICS.md)

**👨‍💻 前端开发者**
→ 必读: [CLAUDE.md](../CLAUDE.md)（开发约束）
→ 然后: [开发指南](./core/DEVELOPMENT_GUIDE.md), [前端重构方案](./frontend/REFACTOR_PLAN.md)
→ 参考: [系统架构](./core/ARCHITECTURE.md), [API 参考](./core/API_REFERENCE.md)

**🔧 后端开发者**
→ 必读: [CLAUDE.md](../CLAUDE.md)（开发约束）
→ 然后: [开发指南](./core/DEVELOPMENT_GUIDE.md), [排产引擎规格](../spec/Engine_Specs_v0.3_Integrated.md)
→ 参考: [字段映射](../spec/Field_Mapping_Spec_v0.3_Integrated.md), [API 契约](../spec/Tauri_API_Contract_v0.3_Integrated.md)

**🧪 测试人员 / QA**
→ 必读: [测试流程](./process/TESTING_PROCESS.md)
→ 然后: [测试数据指南](./guides/TEST_DATA_GUIDE.md), [单元测试报告](./reports/TEST_REPORT.md)
→ 参考: [系统架构](./core/ARCHITECTURE.md)

**📋 代码审查人**
→ 必读: [CLAUDE.md](../CLAUDE.md)（开发约束）
→ 然后: [代码审查流程](./process/CODE_REVIEW_PROCESS.md)
→ 参考: [质量指标](./reports/QUALITY_METRICS.md), [最终工作总结](./reports/FINAL_WORK_SUMMARY.md)

**👔 项目经理 / 技术主管**
→ 必读: [最终工作总结](./reports/FINAL_WORK_SUMMARY.md)
→ 然后: [P1 进展](./reports/P1_PROGRESS.md), [质量指标](./reports/QUALITY_METRICS.md)
→ 参考: [Claude_Dev_Master_Spec.md](../spec/Claude_Dev_Master_Spec.md)（项目约束）

---

## 📅 文档更新日志

| 日期 | 文档 | 版本 | 变更摘要 |
|------|------|------|--------|
| 2026-01-30 | docs/INDEX.md | 1.0 | 新建文档导航索引 |
| 2026-01-30 | docs/ 结构 | - | 重构目录为 core/process/frontend/reports/archived/guides |
| 2026-01-30 | spec/README.md | 1.0 | 新建规范说明文档 |
| 2026-01-30 | CONTRIBUTING.md | 1.0 | 新建贡献指南 |

---

## ✅ 文档维护清单

**每次提交规范文档时**
- [ ] 更新 spec 文件版本号
- [ ] 在此 INDEX.md 中更新版本记录
- [ ] 同步更新相关集成规范

**每周检查**
- [ ] 报告文档是否需要更新
- [ ] 进度清单是否有新完成项目

**每月审查**
- [ ] 文档结构和链接完整性
- [ ] 过期文档是否应该归档
- [ ] 是否有新文档需要添加

**项目阶段完成时**
- [ ] 将任务清单存档到 `archived/`
- [ ] 更新最终工作总结
- [ ] 生成新的质量评估报告

---

## 🔗 关键链接

**项目根目录**
- [README.md](../README.md) - 项目简介
- [CLAUDE.md](../CLAUDE.md) - 开发规范
- [CONTRIBUTING.md](../CONTRIBUTING.md) - 贡献指南
- [ARCHITECTURE.md](../ARCHITECTURE.md) - 系统架构
- [DOCUMENTATION_PLAN.md](../DOCUMENTATION_PLAN.md) - 文档整理计划

**GitHub & 版本控制**
- 主分支: `main`
- 开发分支: `develop`
- PR 模板: `.github/pull_request_template.md`

**工具与资源**
- 源代码: `src/`
- 测试: `tests/`, `src/**/*.test.ts`
- Tauri 配置: `src-tauri/`
- 类型定义: `src/types/`

---

**文档导航索引版本**: 1.0
**上次更新**: 2026-01-30
**维护者**: 代码质量团队

