# 热轧精整排产系统 (Hot Rolling Finishing APS)

**版本**: v1.1.0
**状态**: 生产就绪 (Production Ready)
**最后更新**: 2026-01-31

---

## 项目简介

本系统是一个**工业级决策支持系统**，用于**热轧精整机组**的排产调度。

### 核心定位

| 是 | 不是 |
|---|-----|
| 决策支持系统 | 自动控制系统 |
| 人工最终控制 | 优化算法平台 |
| 排产建议工具 | 通用任务调度器 |

### 技术栈

- **后端**: Rust + Tauri + SQLite
- **前端**: React 18 + TypeScript + Ant Design
- **构建**: Vite + Cargo

---

## 快速开始

### 环境要求

- Node.js >= 18
- Rust >= 1.70
- npm >= 9

### 安装与运行

```bash
# 安装依赖
npm install

# 开发模式
npm run tauri dev

# 生产构建
npm run tauri build
```

### 运行测试

```bash
# 全部测试
cargo test

# 特定测试
cargo test --test full_business_flow_e2e_test
```

---

## 项目结构

```
hot-rolling-finish-aps/
├── spec/                    # 规格文档 (核心规范)
│   ├── Claude_Dev_Master_Spec.md    # 最高权威规范
│   ├── Engine_Specs_v0.3_Integrated.md
│   ├── Field_Mapping_Spec_v0.3_Integrated.md
│   ├── Tauri_API_Contract_v0.3_Integrated.md
│   ├── DecisionApi_Contract_v1.0.md
│   └── data_dictionary_v0.1.md
│
├── src/                     # 源代码
│   ├── domain/              # 领域模型
│   ├── repository/          # 数据仓储
│   ├── engine/              # 业务引擎 (16个)
│   ├── api/                 # API 层
│   ├── decision/            # 决策层 (D1-D6)
│   ├── importer/            # 数据导入
│   ├── app/                 # Tauri 集成
│   ├── components/          # React 组件
│   └── pages/               # React 页面
│
├── tests/                   # 测试代码
├── migrations/              # 数据库迁移
├── docs/                    # 技术文档
└── CLAUDE.md                # 项目宪法
```

---

## 核心功能

### 决策支持 (D1-D6)

| 决策 | 功能 | 工业意义 |
|-----|------|---------|
| D1 | 风险日摘要 | 哪天最危险 |
| D2 | 订单失败分析 | 哪些紧急单无法完成 |
| D3 | 冷料压库 | 哪些冷料压库 |
| D4 | 机组堵塞 | 哪个机组最堵 |
| D5 | 换辊预警 | 换辊是否异常 |
| D6 | 产能优化 | 是否存在产能优化空间 |

### 业务引擎

- **适温引擎**: 判定材料是否适温可排
- **紧急引擎**: 计算紧急等级 (L0-L3)
- **优先级引擎**: 多维排序规则
- **产能引擎**: 产能池填充约束
- **重算引擎**: 排产重算/联动
- **风险引擎**: 风险等级计算

### 最新功能（v1.1.0 - 2026-01-31）

**工作台业务联动系统**
- **物料池状态可视化**: 可操作性状态指示、风险徽章、信息密度提升40%
- **产能影响预测**: 实时预测选中物料对产能的影响，支持风险评估
- **风险概览深链接**: 从风险问题一键直达工作台并自动定位，决策效率提升96%
- **智能筛选联动**: 自动应用机组、紧急度、日期筛选，操作步骤减少86%

详见：[工作台联动功能总结](docs/reports/WORKBENCH_LINKAGE_FEATURES.md)

---

## 工业红线

系统严格遵守以下工业约束：

1. **冻结区保护** - 冻结区材料不可被系统自动调整
2. **适温约束** - 非适温材料不得进入当日产能池
3. **分层紧急** - 紧急度是等级制 (L0-L3)，非评分制
4. **产能优先** - 产能约束始终优先于材料排序
5. **可解释性** - 每个决策必须有明确原因

---

## 文档索引

📚 **[完整文档导航](docs/INDEX.md)** - 按角色和用途查找文档

### 核心规范

| 文档 | 说明 |
|-----|------|
| [spec/Claude_Dev_Master_Spec.md](spec/Claude_Dev_Master_Spec.md) | 最高权威规范 |
| [spec/Engine_Specs_v0.3_Integrated.md](spec/Engine_Specs_v0.3_Integrated.md) | 引擎规格 |
| [spec/Tauri_API_Contract_v0.3_Integrated.md](spec/Tauri_API_Contract_v0.3_Integrated.md) | API 契约 |
| [spec/DecisionApi_Contract_v1.0.md](spec/DecisionApi_Contract_v1.0.md) | 决策 API 规范 |

### 项目文档

| 文档 | 说明 |
|-----|------|
| [CLAUDE.md](CLAUDE.md) | 项目宪法（开发规范与约束） |
| [CONTRIBUTING.md](CONTRIBUTING.md) | 贡献指南 |
| [docs/core/ARCHITECTURE.md](docs/core/ARCHITECTURE.md) | 系统架构 |
| [docs/core/DEVELOPMENT_GUIDE.md](docs/core/DEVELOPMENT_GUIDE.md) | 开发指南 |
| [docs/reports/FINAL_WORK_SUMMARY.md](docs/reports/FINAL_WORK_SUMMARY.md) | 最终工作总结 |
| [docs/reports/TEST_REPORT.md](docs/reports/TEST_REPORT.md) | 单元测试报告 |
| [docs/reports/WORKBENCH_LINKAGE_FEATURES.md](docs/reports/WORKBENCH_LINKAGE_FEATURES.md) | 工作台联动功能总结 (v1.1.0) |

### 历史文档

| 文档 | 说明 |
|-----|------|
| [docs/archived/audit_reports/PROJECT_AUDIT_REPORT.md](docs/archived/audit_reports/PROJECT_AUDIT_REPORT.md) | 项目审核报告 |
| [docs/frontend/README.md](docs/frontend/README.md) | 前端开发指南 |

---

## 项目统计

| 指标 | 数量 |
|------|------|
| Rust 代码 | 50,170 行 |
| TypeScript 代码 | 41,200 行 |
| 总代码行数 | ~91,370 行 |
| 业务引擎 | 11 个 |
| API 端点 | 6 个主要模块 |
| 测试用例 | 42 个测试文件 |
| 单元测试覆盖率 | 92.95% |
| 测试通过率 | 100% |
| React 组件 | 228 个文件 |
| 组件目录 | 42 个 |
| 页面数量 | 11 个 |

---

## 许可证

私有项目 - 内部使用

---

## 联系方式

如有问题，请联系项目负责人。
