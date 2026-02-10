# 更新日志

本文档记录项目的重要变更历史。

## [未发布]

### 基础设施
- 添加 GitHub Actions 自动化部署配置
  - 多平台构建发布工作流（Windows、macOS、Linux）
  - PR 构建测试工作流
  - 自动创建 GitHub Release 并上传安装包
- 添加应用图标（512x512 占位图标）
- 添加完整的发布流程文档

### 测试改进
- 补充 `src/types/strategy.ts` 工具函数的完整测试覆盖
- `src/types` 目录测试覆盖率达到 100%
- 新增 `src/types/strategy.test.ts` 测试文件，包含 normalizeStrategyKey、getStrategyLabelByKey 和 BUILTIN_STRATEGY_OPTIONS 的全面测试

### 文档
- 新增 [DEPLOYMENT_SETUP.md](DEPLOYMENT_SETUP.md) - 部署配置快速开始指南
- 新增 [docs/guides/RELEASE_GUIDE.md](docs/guides/RELEASE_GUIDE.md) - 完整发布流程文档
- 新增 [icons/ICON_GUIDE.md](icons/ICON_GUIDE.md) - 图标准备指南

## [v1.1.0] - 2026-01-31

### 新增功能
- **工作台业务联动系统**
  - 物料池状态可视化：可操作性状态指示、风险徽章、信息密度提升40%
  - 产能影响预测：实时预测选中物料对产能的影响，支持风险评估
  - 风险概览深链接：从风险问题一键直达工作台并自动定位，决策效率提升96%
  - 智能筛选联动：自动应用机组、紧急度、日期筛选，操作步骤减少86%

详见：[工作台联动功能总结](docs/reports/WORKBENCH_LINKAGE_FEATURES.md)

### 测试改进
- 补充材料导入引擎测试覆盖
- 补充 services 和 components 测试覆盖
- 补充 hooks 和 decision types 测试覆盖
- 补充事件总线测试覆盖
- 修复测试失败并补充工具函数测试覆盖
- 单元测试覆盖率达到 92.95%

## [v1.0.0] - 2026-01-30

### 初始发布
- 完整的热轧精整排产决策支持系统
- 11 个业务引擎
- 6 个决策支持模块 (D1-D6)
- Rust + Tauri 后端
- React + TypeScript + Ant Design 前端
- 42 个测试文件，测试通过率 100%

---

## 版本说明

版本号遵循 [语义化版本](https://semver.org/lang/zh-CN/) 规范：

- **主版本号**：不兼容的 API 修改
- **次版本号**：向下兼容的功能性新增
- **修订号**：向下兼容的问题修正

## 变更类型

- **新增功能** (Added)：新增的功能
- **变更** (Changed)：对现有功能的变更
- **废弃** (Deprecated)：即将移除的功能
- **移除** (Removed)：已移除的功能
- **修复** (Fixed)：任何 bug 修复
- **安全** (Security)：安全相关的修复
- **测试改进** (Testing)：测试相关的改进
- **文档** (Documentation)：文档相关的更新
