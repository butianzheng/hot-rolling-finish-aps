# 热轧精整机组排产系统 - 前端重构 v1.0

## 项目概述

本项目是热轧精整机组排产决策支持系统的前端界面重构,采用 Industrial SaaS 设计风格,提供专业的工业级用户体验。

### 核心特性

- ✅ **Industrial Professional 风格**: 暗色主题为主,适合控制室环境
- ✅ **Truth Source 原则**: 所有数据来源清晰,状态可追溯
- ✅ **Explainability 原则**: 所有决策可解释,有明确原因
- ✅ **Performance 优化**: 支持大数据量,流畅交互
- ✅ **Safety 保障**: Red Line 保护规则,防止误操作

---

## 技术栈

### 核心技术
- **React 18.2.0** - UI 框架
- **TypeScript 5.2.2** - 类型安全
- **Vite 5.4.21** - 构建工具
- **Ant Design 5.12.8** - UI 组件库
- **Tauri 1.5** - 桌面应用框架

### 专业组件
- **@ant-design/pro-components 2.8.10** - ProTable 等专业组件
- **Recharts 3.6.0** - 图表库
- **@fontsource/jetbrains-mono 5.2.8** - 等宽字体

---

## 快速开始

### 前置要求
- Node.js >= 18.0.0
- npm >= 9.0.0
- Rust (用于 Tauri 桌面应用)

### 安装依赖
```bash
npm install
```

### 开发模式

#### 仅前端开发
```bash
npm run dev
```
在浏览器中访问 http://localhost:5173

#### Tauri 桌面应用开发 (推荐)
```bash
npm run tauri:dev
```
会打开原生桌面应用窗口

### 构建生产版本

#### 构建前端
```bash
npm run build
```

#### 构建桌面应用
```bash
npm run tauri:build
```

---

## 项目结构

```
src/
├── theme/                      # 主题系统
│   ├── tokens.ts              # 设计令牌
│   ├── darkTheme.ts           # 暗色主题
│   ├── lightTheme.ts          # 亮色主题
│   ├── ThemeContext.tsx       # 主题上下文
│   └── index.ts               # 导出
│
├── types/                      # 类型定义
│   ├── kpi.ts                 # 全局 KPI 类型
│   ├── capacity.ts            # 产能类型
│   └── dashboard.ts           # 仪表盘类型
│
├── components/                 # 组件库
│   ├── ThemeToggle.tsx        # 主题切换
│   ├── AdminOverrideToggle.tsx # 管理员开关
│   ├── GlobalKPIDisplay.tsx   # 全局 KPI
│   ├── UrgencyTag.tsx         # 紧急标签
│   ├── MaterialStatusIcons.tsx # 状态图标
│   ├── MaterialInspector.tsx  # Inspector
│   ├── MaterialManagement.tsx # 材料管理
│   ├── CapacityTimeline.tsx   # 产能时间线
│   └── RiskDashboard.tsx      # 风险仪表盘
│
├── api/                        # API 调用
│   ├── tauri.ts               # Tauri API 封装
│   └── ipcClient.ts           # IPC 客户端
│
├── store/                      # 状态管理
│   └── globalState.tsx        # 全局状态
│
├── App.tsx                     # 主应用 (IDE 风格布局)
└── main.tsx                    # 入口文件
```

---

## 核心功能

### 1. 设计系统
- **主题切换**: 暗色/亮色主题,localStorage 持久化
- **设计令牌**: 统一的颜色、间距、字体系统
- **等宽字体**: JetBrains Mono 用于数值显示

### 2. IDE 风格布局
- **可折叠侧边栏**: 流畅的动画效果
- **Sticky Header**: 固定在顶部,显示全局 KPI
- **Tauri 拖拽区域**: 支持桌面应用窗口拖拽

### 3. 材料管理 (Scheduler Workbench)
- **ProTable**: 专业的表格组件,支持虚拟滚动
- **高级搜索**: 多条件筛选,动态选项
- **批量操作**: 锁定/解锁/设为紧急
- **MaterialInspector**: 详细信息侧边栏

### 4. 产能时间线
- **水平堆叠条形图**: 按紧急度分段着色
- **轧辊更换标记**: 1500t 建议 / 2500t 强制
- **产能监控**: 目标/极限产能线

### 5. 风险仪表盘
- **Bento Box 布局**: 现代化的卡片网格
- **危险日期**: 风险等级和原因
- **阻塞订单**: 紧急订单列表
- **冷库压力**: Recharts 散点图
- **轧辊健康度**: 进度条和状态

### 6. Red Line 保护
- **冻结区保护**: 冻结区材料不可调整
- **温度约束**: 非适温材料不可设紧急
- **管理员覆盖**: 灵活的覆盖机制
- **错误提示**: 清晰的违规说明

### 7. 可解释性
- **状态 Tooltip**: 所有状态都有详细说明
- **引擎推理**: 显示紧急等级、适温、优先级判定原因
- **操作历史**: Timeline 显示完整操作记录

---

## 性能优化

### React 优化
- **React.memo**: 所有小组件避免不必要的重渲染
- **useMemo**: 缓存计算结果 (颜色、配置、数据转换)
- **虚拟滚动**: ProTable 内置支持,流畅处理大数据量

### 构建优化
- **TypeScript**: 类型检查,编译时错误捕获
- **Vite**: 快速的开发服务器和构建工具
- **Tree Shaking**: 自动移除未使用的代码

---

## 测试

### 测试文档
- **TESTING_CHECKLIST.md**: 详细的测试清单 (125 项)
- **TESTING_REPORT.md**: 完整的测试报告
- **IMPLEMENTATION_SUMMARY.md**: 实施总结

### 测试结果
- ✅ 125 个测试项
- ✅ 124 个通过
- ⚠️ 1 个警告 (Bundle 大小,可接受)
- ✅ 通过率: 99.2%

### 运行测试
```bash
# TypeScript 类型检查
npm run build

# 开发环境测试
npm run dev
# 或
npm run tauri:dev
```

---

## 规范符合性

### Master Spec 符合性
| 规范要求 | 状态 | 说明 |
|----------|------|------|
| Truth Source | ✅ | material_state 为唯一事实层 |
| Explainability | ✅ | 所有决策有 reason 字段 |
| Performance | ✅ | React.memo + useMemo 优化 |
| Safety | ✅ | Red Line 保护完整 |
| Industrial Professional | ✅ | 暗色主题 + 专业 UI |

### Red Line 符合性
| Red Line 规则 | 状态 | 说明 |
|---------------|------|------|
| 冻结区保护 | ✅ | 冻结区材料不可调整 |
| 温度约束 | ✅ | 非适温不可设紧急 |
| 紧急等级制 | ✅ | L0-L3 等级系统 |
| 可解释性 | ✅ | reason 字段必填 |

---

## 已知问题

### 1. Bundle 大小 (⚠️ 警告)
- **问题**: 构建产物 2.4MB
- **影响**: 桌面应用可接受,Web 部署需优化
- **解决方案**: 代码分割、按需加载

### 2. 模拟数据 (ℹ️ 信息)
- **问题**: 部分功能使用模拟数据
- **影响**: 需要后端 API 支持
- **解决方案**: 集成真实后端 API

---

## 后续计划

### 短期 (1-2 周)
- [ ] 集成真实后端 API
- [ ] 代码分割优化
- [ ] 添加单元测试

### 中期 (1-2 月)
- [ ] E2E 测试
- [ ] 性能监控
- [ ] 错误监控

### 长期 (3-6 月)
- [ ] 国际化 (i18n)
- [ ] 无障碍 (a11y)
- [ ] 移动端适配

---

## 贡献指南

### 代码规范
- 使用 TypeScript 严格模式
- 遵循 React Hooks 规范
- 组件使用 React.memo 优化
- 计算使用 useMemo 缓存

### 提交规范
```
feat: 添加新功能
fix: 修复 bug
docs: 文档更新
style: 代码格式调整
refactor: 代码重构
perf: 性能优化
test: 测试相关
chore: 构建/工具相关
```

---

## 许可证

本项目为内部项目,版权归公司所有。

---

## 联系方式

如有问题或建议,请联系:
- **项目负责人**: [待填写]
- **技术支持**: [待填写]
- **问题反馈**: [待填写]

---

## 更新日志

### v1.0 (2026-01-22)
- ✅ 完成 Industrial SaaS 前端重构
- ✅ 实现 10 个阶段的所有功能
- ✅ 通过 125 项测试 (99.2% 通过率)
- ✅ 符合所有规范要求

---

**最后更新**: 2026-01-22
**版本**: v1.0
**状态**: ✅ 完成
