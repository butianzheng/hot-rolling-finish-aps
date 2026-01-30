# 🤝 贡献指南

感谢您对热轧精整排产系统的贡献！本指南说明如何参与项目开发。

---

## 📖 前置阅读

在开始贡献前，**必须阅读以下文档**：

1. **[CLAUDE.md](./CLAUDE.md)** - 项目开发宪法和工业约束
   - ⚠️ 特别注意「工业红线」章节
   - 冻结区保护、成熟度约束、分层紧急度等

2. **[docs/INDEX.md](./docs/INDEX.md)** - 文档导航
   - 找到适合您角色的文档

3. **[spec/Claude_Dev_Master_Spec.md](./spec/Claude_Dev_Master_Spec.md)** - Master Spec
   - 系统设计最高权威

---

## 🛠️ 开发环境

### 环境要求
- Node.js 18+
- npm 9+
- Rust（如需修改 Tauri 后端）
- TypeScript 5.3+

### 初始化
```bash
# 克隆仓库
git clone <repository-url>
cd hot-rolling-finish-aps

# 安装依赖
npm install

# 启动开发服务
npm run dev          # 前端开发
npm run tauri:dev    # Tauri 应用开发
```

### 编译验证
```bash
# TypeScript 类型检查
npx tsc --noEmit

# 运行测试
npm run test         # 运行所有测试
npm run test:ui      # 打开测试 UI
npm run test:coverage  # 生成覆盖率报告

# 构建
npm run build        # 构建前端
npm run tauri:build  # 构建 Tauri 应用
```

---

## 📋 代码审查流程

所有代码变更都必须经过审查。详见 [docs/process/CODE_REVIEW_PROCESS.md](./docs/process/CODE_REVIEW_PROCESS.md)

### 简化流程

1. **创建分支**
   ```bash
   git checkout -b feature/your-feature-name
   ```

2. **提交代码**
   - 遵循 [CLAUDE.md](./CLAUDE.md) 的工业约束
   - 添加必要的测试和文档
   - 清晰的 commit 信息

3. **提交 PR**
   - PR 标题: `[type]: description`（例: `feat: 添加版本对比功能`）
   - 填写 PR 模板中的所有信息
   - 关联相关 Issue

4. **审查与修改**
   - 至少需要 1 个批准
   - 修改意见后，push 新 commit（不要 squash）
   - 确保 CI 检查通过

5. **合并**
   - Squash and merge 进 main 分支
   - 删��功能分支

---

## 🎯 分支命名规范

```
feature/        - 新功能:          feature/add-version-comparison
fix/            - 问题修复:        fix/memory-leak-in-table
refactor/       - 重构:            refactor/extract-plan-management
docs/           - 文档:            docs/update-api-reference
test/           - 测试:            test/add-export-helpers-tests
chore/          - 维护:            chore/update-dependencies
perf/           - 性能优化:        perf/optimize-capacity-calculation
```

---

## 💾 Commit 信息规范

```
<type>(<scope>): <subject>

<body>

<footer>
```

### 示例

**特性提交**
```
feat(comparison): 实���版本对比查询

- 添加 computeVersionDiffs 函数
- 支持 ADDED/REMOVED/MOVED/MODIFIED 四种差异类型
- 添加 10 个新测试用例

Fixes #123
```

**修复提交**
```
fix(export): 修复 HTML 导出的 XSS 漏洞

- 添加 HTML 特殊字符转义
- 转义所有用户输入字段
- 添加 XSS 防护单元测试

Fixes #456
```

**文档提交**
```
docs: 更新 API 参考文档

- 添加新 API 端点说明
- 更新字段映射表
- 修正过期示例代码
```

### Type 说明

| Type | 说明 | 示例 |
|------|------|------|
| feat | 新功能 | 添加导出功能 |
| fix | 问题修复 | 修复内存泄漏 |
| refactor | 重构 | 提取组件 |
| perf | 性能优化 | 优化排序算法 |
| test | 添加测试 | 补充单元测试 |
| docs | 文档更新 | 更新 README |
| chore | 维护任务 | 升级依赖 |
| ci | CI 配置 | 修改 workflow |

---

## ✅ 提交前检查清单

在提交 PR 前，请完成以下检查：

- [ ] 代码遵循 [CLAUDE.md](./CLAUDE.md) 的约束
- [ ] 所有工业红线都得到保护
- [ ] TypeScript 编译通过: `npx tsc --noEmit`
- [ ] 所有测试通过: `npm run test -- --run`
- [ ] 新代码有对应的单元测试
- [ ] 测试覆盖率不低于 85%（关键模块 90%+）
- [ ] 添加或更新了相关文档
- [ ] Commit 信息清晰、符合规范
- [ ] 没有 console.log 或 debugger 语句
- [ ] 代码格式化已运行

### 代码风格

- 使用 Prettier 格式化代码
- 遵循 ESLint 配置
- TypeScript strict 模式
- 使用有意义的变量名
- 添加必要的注释（特别是业务逻辑）

---

## 🧪 测试指南

### 单元测试

创建 `.test.ts` 文件测试工具函数和组件逻辑。

```typescript
import { describe, it, expect } from 'vitest';
import { computeVersionDiffs } from './utils';

describe('computeVersionDiffs', () => {
  it('应该检测新增项目', () => {
    const result = computeVersionDiffs([], [{ material_id: 'M1' }]);
    expect(result.diffs[0].changeType).toBe('ADDED');
  });
});
```

### 测试覆盖率

运行覆盖率报告:
```bash
npm run test:coverage
```

覆盖率目标:
- 一般模块: 85%+
- 关键业务逻辑: 90%+
- UI 组件: 70%+（接受度较低）

---

## 📝 文档贡献

### 文档位置

- **规范文档**: `spec/` - 系统设计最高权威
- **核心文档**: `docs/core/` - 架构、API、开发指南
- **工作流程**: `docs/process/` - 审查、测试、重构流程
- **项目报告**: `docs/reports/` - 进度、质量评估
- **使用指南**: `docs/guides/` - 组件、工具使用说明

### 文档标准

1. **使用 Markdown** - GFM 语法
2. **包含标题** - 清晰的章节层级
3. **提供示例** - 代码示例应该可运行
4. **保持更新** - 与代码保持同步
5. **添加链接** - 交叉引用相关文档

---

## 🐛 Bug 报告

发现 bug 时，请创建 Issue 并提供：

- **问题描述**: 清晰说明现象
- **重现步骤**: 最小化重现步骤
- **期望行为**: 应该发生什么
- **实际行为**: 实际发生了什么
- **环境信息**: 操作系统、Node 版本等
- **截图/日志**: 如果可能的话

示例：
```
**描述**
点击「导出」按钮时，HTML 导出出现 XSS 注入。

**重现步骤**
1. 打开版本对比界面
2. 在「操作人」字段输入: `<img src=x>`
3. 点击「导出（HTML）」按钮
4. 打开导出的 HTML 文件

**期望行为**
HTML 中的特殊字符应该被转义

**实际行为**
HTML 中包含未转义的脚本标签

**环境**
- OS: macOS 12.6
- Node: 18.17.1
- Browser: Chrome 119
```

---

## 💡 建议和讨论

有改进建议？

1. 查看现有 Issues 和 Discussions
2. 如果还没有相关讨论，开一个新 Discussion
3. 社区成员会参与讨论和评估

---

## 🏆 贡献者指南

### 获得提交权限

1. 通过 2 个有意义的 PR 审查
2. 遵循所有贡献指南和代码标准
3. 联系项目维护者申请提交权限

### 成为维护者

维护者享有更多权限并负责：
- 代码审查和合并
- Issue 和 PR 管理
- 发布新版本
- 社区管理

申请条件：
- 3+ 个月的贡献记录
- 10+ 个已合并的有意义 PR
- 深入理解项目设计和约束
- 承诺长期维护

---

## 📚 相关资源

- [开发指南](./docs/core/DEVELOPMENT_GUIDE.md)
- [系统架构](./docs/core/ARCHITECTURE.md)
- [代码审查流程](./docs/process/CODE_REVIEW_PROCESS.md)
- [工业防护指南](./docs/guides/GUARDS_COMPONENT_GUIDE.md)
- [Master Spec](./spec/Claude_Dev_Master_Spec.md)

---

## 🤔 常见问题

**Q: 我改了代码但测试失败？**
A: 运行 `npm run test -- --ui` 查看测试详情，确保新代码有对应的测试用例。

**Q: PR 被要求更改，如何继续？**
A: 在原分支上修改代码并 push，GitHub 会自动更新 PR。不需要重新创建 PR。

**Q: 我可以并行提交多个 PR 吗？**
A: 可以，但建议先完成一个再开始下一个，避免分散注意力。

**Q: 如何添加新依赖？**
A: 先讨论是否真正需要，然后运行 `npm install <package>`，在 PR 中说明理由。

---

## 📞 联系方式

- **提问**: 创建 Discussion 或 Issue
- **报告 Bug**: 创建有详细信息的 Issue
- **安全问题**: 不要公开，联系维护者私下讨论

---

**版本**: 1.0
**上次更新**: 2026-01-30
**维护者**: 代码质量团队

