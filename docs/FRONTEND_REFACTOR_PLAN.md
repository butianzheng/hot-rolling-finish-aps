# 热轧精整排产决策支持系统 - 前端重构方案

## 一、系统定位重新定义

### 1.1 核心定位

**决策支持系统 (Decision Support System)**，而非执行监控系统。

核心价值：
- 帮助用户**快速识别风险**
- 提供**多策略方案对比**
- 支持**高效批量干预**
- 实现**决策效果评估**

### 1.2 用户画像

| 特征 | 描述 |
|------|-----|
| 角色 | 单人多角色混用（调度员/主管/经理视角灵活切换） |
| 首要任务 | 查看整体风险状态 |
| 典型场景 | 晨会前快速评估 → 识别问题 → 制定方案 → 对比决策 → 执行 → 复盘 |

### 1.3 当前痛点

1. **信息层次不清晰** - 重要信息和次要信息混杂
2. **缺少工作流引导** - 不知道下一步该做什么
3. **操作路径太长** - 完成一个任务需要多次页面跳转
4. **缺失完整决策闭环** - 没有策略对比和复盘功能

---

## 二、核心工作流设计

### 2.1 决策循环 (Decision Cycle)

```
┌─────────────────────────────────────────────────────────────────┐
│                                                                 │
│   ① 风险概览 ──→ ② 问题下钻 ──→ ③ 人工干预 ──→ ④ 策略对比     │
│        ↑                                              │         │
│        │                                              ↓         │
│   ⑦ 复盘分析 ←── ⑥ 方案执行 ←── ⑤ 方案决策                    │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

### 2.2 各环节详细说明

| 环节 | 目标 | 关键操作 | 输出物 |
|------|------|---------|--------|
| **① 风险概览** | 一眼判断系统健康度 | 查看关键KPI、识别红色区域 | 发现需要关注的问题 |
| **② 问题下钻** | 理解问题根因 | 点击红色区域，查看详情 | 明确问题范围和严重程度 |
| **③ 人工干预** | 调整排程参数 | 锁定物料、调整紧急度、移动排程 | 干预后的计划草案 |
| **④ 策略对比** | 评估不同策略效果 | 并排对比4种策略的KPI | 选出最优策略 |
| **⑤ 方案决策** | 确定最终方案 | 选择策略→微调→确认 | 生成正式计划版本 |
| **⑥ 方案执行** | 执行生效 | 激活版本 | 正式生效的排产计划 |
| **⑦ 复盘分析** | 评估决策效果 | 查看KPI变化、影响分析 | 决策效果报告 |

---

## 三、页面结构重构 (3+1+1模式)

### 3.1 新页面架构

```
┌─ 侧边栏 ─────────┬─────────────────────────────────────────────┐
│                  │  顶部: 🏭 热轧精整排产决策支持                 │
│  📊 风险概览     │  [当前版本: v3-紧急优先-0116 ▾] [👤 调度员]    │
│  📋 计划工作台   ├─────────────────────────────────────────────┤
│  ⚖️ 版本对比     │                                             │
│  📥 数据导入     │              页面内容区域                     │
│                  │                                             │
│  ─────────────  │                                             │
│  ⚙️ 设置中心     │                                             │
└──────────────────┴─────────────────────────────────────────────┘
```

### 3.2 页面职责划分

| 页面 | 核心定位 | 主要功能 |
|------|---------|---------|
| **风险概览** | 系统入口 | 多维风险看板 + 问题下钻 + 跳转处理 |
| **计划工作台** | 全功能操作中心 | 查看排程 + 手动调整 + 一键优化(单策略直接执行) + 版本管理 |
| **版本对比** | 统一对比页面 | 策略草案对比(决策) + 历史版本对比(复盘) + 版本回滚 |
| **数据导入** | 数据入口 | CSV导入 + 冲突解决 |
| **设置中心** | 系统配置 | 全局配置 + 操作日志 + 机组配置 + 用户偏好 |

### 3.3 关键概念定义

| 概念 | 定义 | 状态 |
|------|-----|------|
| **策略草案** | 通过策略计算生成的临时方案 | 未保存，选择确认后才成为正式版本 |
| **正式版本** | 保存在系统中的计划版本 | 持久化，可回滚，有完整历史 |
| **一键优化** | 单策略快速执行 | 参数调整 → 预览影响 → 确认生成版本 |
| **策略对比** | 多策略并行计算并对比 | 生成多个草案 → 对比 → 选择 → 确认生成版本 |

### 3.4 页面间交互流程

```
┌─────────────┐                              ┌─────────────┐
│  风险概览   │─── 发现问题，去处理 ───────→│ 计划工作台   │
└─────────────┘                              └─────────────┘
                                                   │
     ┌───────── 手动调整/一键优化(直接生成版本) ────┤
     │                                             │
     │          生成策略对比方案(多策略)            │
     │                    │                        │
     │                    ↓                        │
     │          ┌─────────────────┐                │
     │          │   版本对比       │                │
     │          │ (策略草案对比)   │                │
     │          │ (选择确认→版本)  │                │
     │          └─────────────────┘                │
     │                    │                        │
     │                    ↓                        │
     │          ┌─────────────────┐                │
     │          │   版本对比       │                │
     │          │ (历史版本对比)   │                │
     │          │ (复盘/回滚)      │                │
     │          └─────────────────┘                │
     │                                             │
     └──────────── 返回工作台继续操作 ──────────────┘
```

---

## 四、各页面详细设计

### 4.1 风险概览页 (首页/入口)

**设计理念**: 概览 + 下钻分层递进

#### 4.1.1 页面布局 (上下分区: 概览+详情)

```
┌─────────────────────────────────────────────────────────────────┐
│  顶部KPI带 (一眼判断系统健康度)                                   │
│                                                                │
│  ┌────────┐  ┌────────┐  ┌────────┐  ┌────────┐  ┌────────┐   │
│  │🟡整体风险│  │L3订单:12│  │利用率:78%│  │🔴冷坨压力│  │换辊:正常│   │
│  │  中等   │  │ 3个逾期 │  │ H033偏高│  │  高    │  │剩余320t│   │
│  └────────┘  └────────┘  └────────┘  └────────┘  └────────┘   │
└─────────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────────┐
│  多维度详情 (Tab切换)                                            │
│  [订单维度] [产能维度] [库存维度] [换辊维度] [问题汇总]            │
├─────────────────────────────────────────────────────────────────┤
│  当前Tab: 问题汇总                                               │
│                                                                │
│  排序: [默认按严重度 ▾]  筛选: [全部类型 ▾]                       │
│                                                                │
│  ┌──────────────────────────────────────────────────────────┐ │
│  │ 🔴P0 L3紧急订单未排入 (3个)               影响: 高 | 今日   │ │
│  │    > 合同号: ABC123, DEF456, GHI789                      │ │
│  │    [查看详情] [去计划工作台处理]                           │ │
│  ├──────────────────────────────────────────────────────────┤ │
│  │ 🟠P1 H033明日产能溢出 (+120t)             影响: 中 | 明日   │ │
│  │    > 超出限制6%, 需要调整或接受                            │ │
│  │    [查看详情] [去计划工作台处理]                           │ │
│  ├──────────────────────────────────────────────────────────┤ │
│  │ 🟡P2 冷坨超30天积压 (5批)                影响: 中 | 持续   │ │
│  │    > H032: 2批, H034: 3批                                │ │
│  │    [查看详情] [去计划工作台处理]                           │ │
│  └──────────────────────────────────────────────────────────┘ │
│                                                                │
│  [切换到其他Tab查看: 订单维度 | 产能维度 | 库存维度 | 换辊维度]   │
└─────────────────────────────────────────────────────────────────┘
```

**各维度Tab内容**:

| Tab | 主要展示 | 可视化形式 |
|-----|---------|-----------|
| 订单维度 | L3/L2/L1/L0分布、逾期订单、完成率趋势 | 堆叠柱状图 + 表格 |
| 产能维度 | 机组×日期风险矩阵、产能利用率、溢出预警 | 热力图 + 条形图 |
| 库存维度 | 冷坨账龄分布、压力等级、消化建议 | 饼图 + 表格 |
| 换辊维度 | 各机组换辊进度、预警状态 | 进度条 + 状态卡片 |
| 问题汇总 | 所有问题按严重度排列 | 列表 (默认Tab) |

#### 4.1.2 下钻交互 (混合方式)

| 点击对象 | 下钻内容 | 展示方式 |
|---------|---------|---------|
| 热力图单元格 | 该机组该日期的详细排程 + 风险明细 | **侧边抽屉** |
| 问题列表项 | 问题详情 + 相关物料 + 建议操作 | **弹窗(Modal)** |
| KPI卡片 | 该指标的详细分解 | 弹窗展示指标趋势 |
| 图表数据点 | 该数据点对应的物料/订单详情 | 侧边抽屉 |

#### 4.1.3 下钻抽屉内容示例

```
┌─ 风险详情: H033 / 2024-01-16 ──────────────────────────────────┐
│                                                                │
│  风险等级: 🔴 高风险                                            │
│  风险原因:                                                     │
│    • 产能溢出 +120t (超出限制6%)                                │
│    • 3个L3订单未排入                                            │
│    • 换辊剩余容量仅320t                                         │
│                                                                │
│  ┌──────────────────────────────────────────────────────────┐ │
│  │ 当日排程列表 (ProTable)                                    │ │
│  │ [seq] [物料号] [重量] [紧急度] [状态]                      │ │
│  │  1    M001     15.2t   L3      🔒已锁定                    │ │
│  │  2    M002     12.8t   L2      待排                        │ │
│  │  ...                                                       │ │
│  └──────────────────────────────────────────────────────────┘ │
│                                                                │
│  [去计划工作台处理] [关闭]                                      │
│                                                                │
└────────────────────────────────────────────────────────────────┘
```

---

### 4.2 计划工作台页

**设计理念**: 一站式排产操作中心，支持高效批量/策略性调整

#### 4.2.1 页面布局

```
┌─────────────────────────────────────────────────────────────────┐
│  工具栏                                                         │
│  [当前计划: v3 ▾] [版本: 紧急优先-0116 ▾] [重算] [新建版本]      │
│  [日期范围▾] [📋批量操作▾] [⚡一键优化▾] [📊生成策略对比方案]     │
└─────────────────────────────────────────────────────────────────┘

┌───────────────────────────────────┬───────────────────────────────┐
│  物料池 (左侧, 树形结构)            │  排程时间线 (右侧)             │
│                                   │                               │
│  ┌─ 🔽 H032 (23个物料)            │  产能概览条形图:               │
│  │   ┌─ 🔽 待排 (12)              │  ┌─────────────────────────┐ │
│  │   │   [☐] M001 15.2t L3      │  │H032 ▓▓▓▓▓░░░ 85%        │ │
│  │   │   [☐] M002 12.8t L2      │  │H033 ▓▓▓▓▓▓▓▓ 92% 🔴    │ │
│  │   │   [☐] M003 18.5t L1      │  │H034 ▓▓▓▓▓░░░ 81%        │ │
│  │   │   ...                    │  └─────────────────────────┘ │
│  │   └─ 📦 已排 (8)              │                               │
│  │   └─ 🔒 已锁定 (3)            │  视图切换: [矩阵] [甘特图] [卡片]│
│  │                               │                               │
│  ┌─ ▶️ H033 (18个物料)            │  当前视图: 矩阵                │
│  ┌─ ▶️ H034 (15个物料)            │  ┌─ H032 ────────────────────┐│
│  ...                              │  │ 01/15 │ 01/16 │ 01/17 │..││
│                                   │  │ M001  │ M002  │ M005  │  ││
│                                   │  │ M003  │ M004  │       │  ││
│                                   │  └───────────────────────────┘│
│  [搜索物料...]                     │  ┌─ H033 ────────────────────┐│
│                                   │  │ 01/15 │ 01/16 │ 01/17 │..││
│                                   │  │ M007  │ M008🔴│ M010  │  ││
└───────────────────────────────────┴───────────────────────────────┘

┌─────────────────────────────────────────────────────────────────┐
│  状态栏                                                         │
│  已选: 5个物料 | 总重: 82.3t | [🔒锁定] [⬆️提升紧急度] [➡️移动到...] │
└─────────────────────────────────────────────────────────────────┘
```

**关键特性**:
- **左侧树形**: 机组 >> 状态 >> 物料列表，支持折叠展开
- **右侧多视图**: 矩阵/甘特图/卡片三种视图可切换
- **产能可视化**: 时间线顶部显示各机组产能利用率条形图
- **双重操作**: 拖拽（单个物料）+ 勾选按钮（批量操作）
- **版本管理**: 工具栏内嵌版本切换/创建/重算功能

#### 4.2.2 批量操作功能

| 操作类型 | 触发方式 | 支持的操作 |
|---------|---------|-----------|
| **手动批量** | 勾选多个物料 → 点击批量操作 | 锁定、解锁、调整紧急度、移动到指定日期 |
| **条件批量** | 点击"按条件操作" | 输入条件（如"H032机组的L3物料"）→ 预览 → 确认执行 |
| **一键优化** | 点击"一键优化"菜单 | 预设场景：清理冷坨空闲产能、平衡机组负载、紧急订单前置 |

#### 4.2.3 一键优化选项

```
┌─ 一键优化 ─────────────────────────────────────────────────────┐
│                                                                │
│  📌 紧急订单前置                                                │
│     将所有L3/L2订单尽可能排入最早可用日期                        │
│     [预览影响] [执行]                                           │
│                                                                │
│  📦 冷坨消化                                                    │
│     优先排产账龄>N天的冷坨物料                                   │
│     [设置天数: 30▾] [预览影响] [执行]                            │
│                                                                │
│  ⚖️ 负载均衡                                                    │
│     平衡各机组产能利用率，减少单机组过载                         │
│     [预览影响] [执行]                                           │
│                                                                │
└────────────────────────────────────────────────────────────────┘
```

#### 4.2.4 操作影响预览

每次批量操作前，显示影响摘要：

```
┌─ 操作影响预览 ────────────────────────────────────────────────┐
│                                                                │
│  即将执行: 将 5个L3物料 移动至 H033 / 2024-01-16               │
│                                                                │
│  影响评估:                                                     │
│  ┌──────────────────┬──────────┬──────────┬──────────┐        │
│  │ 指标              │ 操作前   │ 操作后   │ 变化     │        │
│  ├──────────────────┼──────────┼──────────┼──────────┤        │
│  │ H033 01/16产能   │ 1850t    │ 2020t    │ +170t 🔴 │        │
│  │ L3订单完成率     │ 67%      │ 85%      │ +18% 🟢  │        │
│  │ 产能溢出         │ 0t       │ 20t      │ +20t 🟡  │        │
│  └──────────────────┴──────────┴──────────┴──────────┘        │
│                                                                │
│  ⚠️ 警告: H033 01/16 将超出产能限制 1%                          │
│                                                                │
│  [取消] [仍然执行]                                              │
│                                                                │
└────────────────────────────────────────────────────────────────┘
```

---

### 4.3 版本对比页 (合并策略对比+复盘分析)

**设计理念**: 统一的版本对比页面，同时支持策略决策(向前看)和版本复盘(向后看)

**进入方式**:
1. 从计划工作台点击「生成策略对比方案」→ 进入策略草案对比模式
2. 直接导航进入 → 进入历史版本对比模式

#### 4.3.1 页面模式切换

```
┌─────────────────────────────────────────────────────────────────┐
│  对比模式: [◉ 策略草案对比] [○ 历史版本对比]                      │
└─────────────────────────────────────────────────────────────────┘
```

#### 4.3.2 策略草案对比模式 (决策用)

用于在多个策略计算结果中选择最优方案，确认后生成正式版本。

```
┌─────────────────────────────────────────────────────────────────┐
│  策略草案对比                                              [草案] │
│                                                                │
│  基准版本: [v3-紧急优先-0116▾]  计划范围: [2024-01-15] 至 [2024-01-21] │
│  参与对比: [☑️均衡方案] [☑️紧急优先] [☑️产能优先] [☑️冷坨消化]      │
│  [重新计算策略草案]                                              │
└─────────────────────────────────────────────────────────────────┘

┌───────────────┬───────────────┬───────────────┬───────────────┐
│  均衡方案      │  紧急订单优先  │  产能利用率优先 │  冷坨消化优先  │
│  (草案)        │  (草案)        │  (草案)        │  (草案)        │
├───────────────┼───────────────┼───────────────┼───────────────┤
│  订单维度      │  订单维度      │  订单维度      │  订单维度      │
│  L3完成率: 75% │  L3完成率: 95%✓│  L3完成率: 60% │  L3完成率: 70% │
│  L2完成率: 85% │  L2完成率: 90%✓│  L2完成率: 75% │  L2完成率: 80% │
│  逾期订单: 3   │  逾期订单: 1 ✓ │  逾期订单: 5   │  逾期订单: 4   │
│               │               │               │               │
│  产能维度      │  产能维度      │  产能维度      │  产能维度      │
│  利用率: 82%   │  利用率: 78%   │  利用率: 95%✓  │  利用率: 85%   │
│  溢出: 50t     │  溢出: 120t    │  溢出: 0t ✓    │  溢出: 30t     │
│               │               │               │               │
│  库存维度      │  库存维度      │  库存维度      │  库存维度      │
│  冷坨消化: 12批│  冷坨消化: 8批 │  冷坨消化: 10批│  冷坨消化: 18批✓│
├───────────────┼───────────────┼───────────────┼───────────────┤
│  [查看详情▾]  │  [查看详情▾]  │  [查看详情▾]  │  [查看详情▾]  │
│  [选择此方案] │  [选择此方案]  │  [选择此方案]  │  [选择此方案]  │
└───────────────┴───────────────┴───────────────┴───────────────┘

💡 系统建议: 当前场景下，「紧急订单优先」方案综合表现最佳
```

**选择草案后的确认流程**:

```
┌─ 确认生成版本 ─────────────────────────────────────────────────┐
│                                                                │
│  您选择了: 「紧急订单优先」草案                                  │
│                                                                │
│  ┌──────────────────────────────────────────────────────────┐ │
│  │ 策略参数调整 (可选)                                       │ │
│  │ 紧急订单权重:    [高 ▾]   产能溢出上限:    [5% ▾]          │ │
│  │ 冷坨优先阈值:    [30天▾]  负载均衡系数:    [0.8▾]          │ │
│  └──────────────────────────────────────────────────────────┘ │
│                                                                │
│  版本命名: [紧急优先-0116调整]                                  │
│  备注说明: [处理3个逾期L3订单...]                               │
│                                                                │
│  [取消] [预览调整后方案] [确认生成正式版本]                       │
└────────────────────────────────────────────────────────────────┘
```

#### 4.3.3 历史版本对比模式 (复盘用)

用于对比已保存的正式版本，评估决策效果，支持回滚。

```
┌─────────────────────────────────────────────────────────────────┐
│  历史版本对比                                              [版本] │
│                                                                │
│  版本A: [v2-均衡方案-0115 ▾]      版本B: [v3-紧急优先-0116 ▾]     │
│  [对比这两个版本]                                                │
└─────────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────────┐
│  KPI对比                                                        │
│  ┌─────────────────────────────────────────────────────────┐  │
│  │ 指标           │ v2 (基准)  │ v3 (当前)  │ 变化        │  │
│  ├────────────────┼───────────┼───────────┼────────────┤  │
│  │ L3完成率       │ 75%       │ 95%       │ +20% 🟢    │  │
│  │ 产能利用率     │ 82%       │ 87%       │ +5% 🟢     │  │
│  │ 产能溢出       │ 0t        │ 120t      │ +120t 🔴   │  │
│  │ 冷坨消化       │ 12批      │ 8批       │ -4批 🟡    │  │
│  └─────────────────────────────────────────────────────────┘  │
│                                                                │
│  [▾ 展开风险分布对比]                                           │
│  [▾ 展开物料移动明细]                                           │
└─────────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────────┐
│  📝 复盘总结                                                     │
│  ┌──────────────────────────────────────────────────────────┐ │
│  │ 本次决策要点:                                              │ │
│  │ 1. 选择「紧急订单优先」策略，成功解决3个逾期L3订单           │ │
│  │ 2. 代价: H033/01-16产能溢出6%，需关注换辊节点              │ │
│  │ ...                                                       │ │
│  └──────────────────────────────────────────────────────────┘ │
│                                                                │
│  [保存总结] [导出报告(PDF)] [回滚到v2]                           │
└─────────────────────────────────────────────────────────────────┘
```

#### 4.3.4 信息展示层次 (两种模式通用)

通过折叠展开控制信息密度:

| 层级 | 内容 | 默认状态 |
|-----|------|---------|
| 第1层 | KPI数字对比 | 展开 |
| 第2层 | 风险分布热力图变化 | 折叠 |
| 第3层 | 物料级别移动明细 | 折叠 |

---

### 4.4 数据导入页 (保留，优化)

保留现有MaterialImport页面，优化点：
1. 导入成功后自动跳转到计划工作台
2. 冲突解决后自动刷新相关数据
3. 添加导入历史记录查看

### 4.5 设置中心

**入口位置**: 侧边栏底部

**包含内容**:
- **系统配置**: 现有ConfigManagement功能 (season_mode, min_temp_days, roll_threshold等)
- **操作日志**: 现有ActionLogQuery功能 (审计追溯)
- **机组配置**: 机组产能基础数据配置 (target_capacity, limit_capacity等)
- **用户偏好**: 个人偏好设置 (默认视图、主题、默认策略等)

```
┌─ 设置中心 ─────────────────────────────────────────────────────┐
│                                                                │
│  [系统配置] [操作日志] [机组配置] [用户偏好]                     │
│                                                                │
│  ┌──────────────────────────────────────────────────────────┐ │
│  │ 系统配置                                                  │ │
│  │                                                          │ │
│  │ 季节模式:      [自动 ▾]                                   │ │
│  │ 冬季适温天数:  [3 天]                                      │ │
│  │ 夏季适温天数:  [4 天]                                      │ │
│  │ N1紧急阈值:    [2 天]                                      │ │
│  │ N2紧急阈值:    [7 天]                                      │ │
│  │ 换辊建议阈值:  [1500 t]                                    │ │
│  │ 换辊硬限制:    [2500 t]                                    │ │
│  │ ...                                                       │ │
│  │                                                          │ │
│  │ [保存配置] [恢复默认]                                      │ │
│  └──────────────────────────────────────────────────────────┘ │
│                                                                │
└────────────────────────────────────────────────────────────────┘
```

---

## 五、导航与工作流引导

### 5.1 整体导航设计

```
┌─ 侧边栏 ─────────┬─────────────────────────────────────────────┐
│                  │  顶部: 🏭 热轧精整排产决策支持                 │
│  📊 风险概览     │  [当前版本: v3-紧急优先-0116 ▾] [👤 调度员]    │
│  📋 计划工作台    ├─────────────────────────────────────────────┤
│  ⚖️ 策略对比     │                                             │
│  📈 复盘分析     │              页面内容区域                     │
│  📥 数据导入     │                                             │
│                  │                                             │
│  ─────────────  │                                             │
│  ⚙️ 设置中心     │                                             │
└──────────────────┴─────────────────────────────────────────────┘
```

**导航特点**:
- 侧边栏显示5个核心功能入口
- 设置中心放在侧边栏底部 (分隔线下方)
- 顶部显示当前活动版本信息和用户角色

### 5.2 工作流引导 (首次使用/空状态)

当系统没有活动计划时，风险概览页显示引导：

```
┌─────────────────────────────────────────────────────────────────┐
│                                                                │
│                     🏭 欢迎使用排产决策支持系统                   │
│                                                                │
│                      请按以下步骤开始:                          │
│                                                                │
│  ┌──────────┐    ┌──────────┐    ┌──────────┐    ┌──────────┐ │
│  │  Step 1  │ → │  Step 2  │ → │  Step 3  │ → │  Step 4  │ │
│  │ 导入数据  │    │ 查看风险  │    │ 调整排程  │    │ 生成方案  │ │
│  │          │    │          │    │          │    │          │ │
│  │ [开始导入]│    │          │    │          │    │          │ │
│  └──────────┘    └──────────┘    └──────────┘    └──────────┘ │
│                                                                │
└─────────────────────────────────────────────────────────────────┘
```

### 5.3 页面间跳转按钮

在每个页面关键位置添加跳转按钮，引导用户进入下一步：

| 当前页面 | 触发条件 | 引导按钮 |
|---------|---------|---------|
| 风险概览 | 发现问题 | "去计划工作台处理" |
| 计划工作台 | 需要策略对比 | "生成策略对比方案" → 跳转到版本对比页(草案模式) |
| 计划工作台 | 一键优化 | 参数弹窗 → 预览 → 直接生成版本(不跳转) |
| 版本对比(草案模式) | 选择策略 | "确认生成正式版本" |
| 版本对比(版本模式) | 完成复盘 | "返回计划工作台" / "返回风险概览" |

---

## 六、技术实现要点

### 6.1 状态管理重构

```typescript
// 新的全局状态结构
interface GlobalState {
  // 当前活动版本
  activeVersionId: string | null;

  // 当前工作流阶段
  workflowStage: 'overview' | 'planning' | 'comparing' | 'reviewing';

  // 策略对比草案 (临时状态)
  strategyDrafts: StrategyDraft[];

  // 用户偏好
  preferences: {
    defaultStrategy: 'balanced' | 'urgent' | 'capacity' | 'coldStock';
    riskThreshold: 'low' | 'medium' | 'high';
  };
}
```

### 6.2 新增API调用

需要与后端确认以下API是否存在或需要新增：

| API | 用途 | 是否需要新增 |
|-----|------|-------------|
| `generate_strategy_comparison` | 生成多策略对比结果 | 需确认 |
| `apply_strategy` | 应用选定策略并生成版本 | 需确认 |
| `get_version_impact_analysis` | 获取版本影响分析 | 需确认 |
| `compare_versions` | 对比两个版本差异 | 已存在 |
| `batch_move_items` | 批量移动物料 | 已存在 |

### 6.3 组件复用计划

| 新页面 | 可复用组件 | 需新增组件 |
|-------|-----------|-----------|
| 风险概览 | RiskCalendarHeatmap, GlobalKPIDisplay | RiskDrilldownDrawer, ProblemList |
| 计划工作台 | ProTable, UrgencyTag, FrozenZoneBadge | BatchOperationPanel, TimelineView, ImpactPreviewModal |
| 策略对比 | - | StrategyCard, ComparisonTable, StrategySelector |
| 复盘分析 | - | ImpactAnalysisPanel, VersionCompareView, SummaryEditor |

---

## 七、验收标准

### 7.1 功能验收

- [ ] 风险概览页可在5秒内判断系统整体健康状态
- [ ] 从发现问题到开始处理不超过2次点击
- [ ] 批量操作支持按条件筛选 + 一键执行
- [ ] 策略对比可同时展示4种策略的KPI对比
- [ ] 方案决策支持: 选择策略 → 微调 → 确认 → 生成版本
- [ ] 复盘分析可评估决策方案是否达到预期

### 7.2 交互验收

- [ ] 所有下钻操作使用侧边抽屉，无需跳转页面
- [ ] 批量操作前显示影响预览
- [ ] 每个页面有明确的"下一步"引导
- [ ] 空状态有操作引导

### 7.3 性能验收

- [ ] 风险概览页首屏加载 < 2秒
- [ ] 策略对比计算完成 < 30秒 (后端)
- [ ] 表格虚拟滚动支持1000+行流畅滚动

---

## 八、实施计划 (基于3+1+1结构)

### Phase 1: 风险概览重构 (1-2周)
- 合并D1-D6为统一风险看板
- 实现下钻抽屉 (侧边抽屉展示详情)
- 添加问题列表和"去处理"跳转
- 产能概览条形图整合

### Phase 2: 计划工作台整合 (3-4周) **核心阶段**
- 合并物料管理和排程可视化
- 实现左侧树形物料池 (机组>>状态>>物料)
- 实现右侧混合视图 (矩阵/甘特图/卡片切换)
- 实现产能概览条形图 (时间线顶部)
- 实现批量操作 (勾选+按钮, 拖拽单个)
- 实现一键优化 (单策略快速执行)
  - 参数弹窗 → 预览影响 → 确认生成版本
- 内嵌版本管理 (切换/创建/重算)
- 实现"生成策略对比方案"跳转

### Phase 3: 版本对比页 (2-3周) **合并页面**
- 新增版本对比页面 (合并策略对比+复盘分析)
- 实现模式切换 (策略草案对比 / 历史版本对比)
- 策略草案对比模式:
  - 4种预设策略计算 (需后端配合)
  - 并排卡片对比展示
  - 草案选择 → 参数调整 → 确认生成版本
- 历史版本对比模式:
  - 选择两个版本进行对比
  - KPI变化展示
  - 复盘总结保存和导出 (PDF/Word)
  - 版本回滚功能
- 信息分层展开 (KPI / 风险分布 / 物料明细)

### Phase 4: 数据导入优化 (1周)
- 优化导入流程
- 导入成功后自动跳转工作台
- 冲突解决后自动刷新
- 添加导入历史记录

### Phase 5: 设置中心整合 (1周)
- 整合系统配置、操作日志、机组配置、用户偏好
- 侧边栏底部入口设计
- 分Tab展示各类设置

### Phase 6: 工作流引导与整体优化 (1周)
- 页面间跳转按钮完善
- 空状态引导
- 整体测试和优化
- 性能优化 (虚拟滚动、防抖、memo化)

---

## 九、风险与依赖

### 9.1 后端依赖

| 功能 | 后端需求 | 风险等级 |
|------|---------|---------|
| 多策略计算 | 需要支持不同策略的重算 | 高 |
| 影响分析 | 需要返回结构化的影响数据 | 中 |
| 批量操作 | 现有API需要扩展 | 低 |

### 9.2 设计风险

| 风险 | 缓解措施 |
|------|---------|
| 策略计算时间过长 | 添加进度显示、支持取消 |
| 信息密度过高 | 使用折叠/展开控制信息层次 |
| 用户习惯迁移成本 | 提供引导说明、保留关键操作入口 |

---

## 十、技术确认事项 (已确认)

### 10.1 后端策略计算能力

**当前状态**: ❌ 不支持多策略并行计算

**需要新增的后端API**:

```typescript
// 新增: 生成多策略草案 (后端需实现)
generate_strategy_drafts: {
  request: {
    base_version_id: string,
    plan_date_from: string,
    plan_date_to: string,
    strategies: StrategyType[]  // ['balanced', 'urgent_first', 'capacity_first', 'cold_stock_first']
  },
  response: {
    drafts: StrategyDraft[]  // 每个策略一个草案对象
  }
}

// 新增: 应用策略草案生成正式版本 (后端需实现)
apply_strategy_draft: {
  request: {
    draft_id: string,
    version_name: string,  // 系统自动生成中文名
    parameters: StrategyParameters,  // 可调整的策略参数
    note: string
  },
  response: {
    version_id: string,
    version_name: string
  }
}
```

**后端实现建议**:
- 4种预设策略由后端定义默认参数组合
- 前端可通过配置界面扩展/修改策略参数
- 策略草案为临时对象，不持久化，确认后才生成正式版本

### 10.2 影响分析数据结构

**当前状态**: ✅ `impact_summary_json` 字段完整，满足复盘需求

**现有字段**:
```rust
ImpactSummary {
  // 物料影响 - 支持物料移动明细对比
  moved_count: i32,
  squeezed_out_count: i32,
  added_count: i32,
  material_changes: Vec<MaterialChange>,  // {material_no, change_type, from_date, to_date, reason}

  // 产能影响 - 支持产能变化对比
  capacity_delta_t: f64,
  overflow_delta_t: f64,
  capacity_changes: Vec<CapacityChange>,  // {date, machine_code, used_before, used_after, delta}

  // 风险影响 - 支持风险等级变化对比
  risk_level_before: String,
  risk_level_after: String,
  risk_changes: Vec<RiskChange>,  // {date, machine_code, risk_before, risk_after, reason}

  // 换辊影响
  roll_campaign_affected: bool,
  roll_tonnage_delta_t: Option<f64>,

  // 紧急物料影响
  urgent_material_affected: i32,
  l3_critical_count: i32,

  // 冲突检测
  locked_conflicts: Vec<String>,
  frozen_conflicts: Vec<String>,
  structure_suggestions: Vec<String>,
}
```

**复盘功能可直接使用**: 无需额外字段，现有结构支持:
- 物料移动明细展示
- 产能变化对比
- 风险等级变化
- 决策效果评估

### 10.3 预设策略定义

**确认方案**: 后端预设 + 前端可扩展配置

| 策略名称 | 后端预设参数组合 | 前端可调整 |
|---------|----------------|-----------|
| **均衡方案** (balanced) | 各参数默认值，综合平衡 | 可微调权重 |
| **紧急订单优先** (urgent_first) | urgent_weight=高, overflow_tolerance=放宽 | 权重、溢出上限 |
| **产能利用率优先** (capacity_first) | capacity_weight=高, urgent_weight=降低 | 权重比例 |
| **冷坨消化优先** (cold_stock_first) | cold_stock_age_threshold=降低, cold_stock_weight=高 | 账龄阈值、权重 |

**扩展机制**:
- 设置中心提供「策略配置」Tab
- 用户可复制预设策略并修改参数创建自定义策略
- 自定义策略存储在 config_kv 或 strategy_profiles 表

### 10.4 版本命名规范

**确认方案**: 系统自动生成中文名称

**命名规则**:
```
{策略类型中文}-{日期}-{序号}

示例:
- 均衡方案-0116-001
- 紧急优先-0116-002
- 产能优先-0116-001
- 冷坨消化-0117-001
- 手动调整-0117-001  (一键优化或手动调整生成的版本)
```

**策略类型中文映射**:
- balanced → 均衡方案
- urgent_first → 紧急优先
- capacity_first → 产能优先
- cold_stock_first → 冷坨消化
- manual → 手动调整

---

## 十一、后端API改动清单

### 需要新增的API (Phase 3依赖)

| API | 用途 | 优先级 |
|-----|-----|-------|
| `generate_strategy_drafts` | 生成多策略草案 | P0 |
| `apply_strategy_draft` | 应用草案生成版本 | P0 |
| `get_strategy_draft_detail` | 查询草案变更明细（moved/added/squeezed） | P1 |
| `get_strategy_presets` | 获取预设策略列表 | P1 |
| `save_custom_strategy` | 保存自定义策略 | P2 |

### 已有API (可直接使用)

| API | 用途 | 状态 |
|-----|-----|------|
| `compare_versions` | 版本对比 | ✅ 已存在 |
| `recalc_full` | 单策略重算 (一键优化可用) | ✅ 已存在 |
| `get_risk_snapshot` | 风险快照 | ✅ 已存在 |
| `batch_move_items` | 批量移动 | ✅ 已存在 |
| `batch_lock_materials` | 批量锁定 | ✅ 已存在 |

---

## 十二、项目结构改动评估

### 12.1 路由层改动 (src/router/index.tsx)

**新路由结构**:
```
/ (root redirect to /overview)
├── /overview          ← 风险概览 (合并 Dashboard + D1-D6)
├── /workbench         ← 计划工作台 (合并 Material + Plan)
├── /comparison        ← 版本对比 (NEW)
├── /import            ← 数据导入 (保留优化)
├── /settings          ← 设置中心 (合并 Config + Logs)
└── 404
```

**删除的路由**: /material, /visualization, /config, /risk, /logs, /capacity, /decision/*

### 12.2 状态管理改动 (src/stores/)

**use-global-store.ts 新增字段**:
```typescript
// 版本对比
versionComparisonMode: 'DRAFT_COMPARISON' | 'HISTORICAL' | null;
selectedVersionA: string | null;
selectedVersionB: string | null;

// 工作台
workbenchViewMode: 'MATRIX' | 'GANTT' | 'CARD';
workbenchFilters: { machineCode, urgencyLevel, lockStatus };

// 用户偏好
userPreferences: { defaultTheme, autoRefreshInterval, sidebarCollapsed };
```

**use-plan-store.ts 新增**:
```typescript
draftVersions: StrategyDraft[];  // 策略草案
createDraftVersion: (sourceVersionId, note) => Promise<string>;
publishDraft: (draftId) => Promise<string>;
```

### 12.3 类型定义新增 (src/types/)

**新建 src/types/comparison.ts**:
```typescript
interface VersionDiff {
  materialId: string;
  changeType: 'ADDED' | 'REMOVED' | 'MODIFIED' | 'MOVED';
  previousState: PlanItemSnapshot;
  currentState: PlanItemSnapshot;
  reason: string;
}

interface StrategyDraft {
  draftId: string;
  sourceVersionId: string;
  status: 'DRAFT' | 'PUBLISHED';
  strategyType: 'balanced' | 'urgent_first' | 'capacity_first' | 'cold_stock_first';
  parameters: StrategyParameters;
  changes: VersionDiff[];
}

interface VersionComparisonResult {
  versionIdA: string;
  versionIdB: string;
  diffs: VersionDiff[];
  summary: { totalChanges, addedCount, removedCount, modifiedCount, movedCount };
}
```

**新建 src/types/preferences.ts**:
```typescript
interface UserPreferences {
  defaultTheme: 'light' | 'dark';
  autoRefreshInterval: number;
  sidebarCollapsed: boolean;
  defaultStrategy: StrategyType;
}
```

### 12.4 组件结构改动 (src/components/)

**删除的组件** (7个):
- Dashboard.tsx
- MaterialManagement.tsx
- PlanItemVisualization.tsx
- ConfigManagement.tsx
- ActionLogQuery.tsx
- RiskSnapshotCharts.tsx
- RiskDashboard.tsx

**保留并复用** (5个+):
- MaterialImport.tsx → 数据导入页
- CapacityTimeline.tsx → 计划工作台
- MaterialInspector.tsx → 计划工作台
- FrozenZoneBadge.tsx, UrgencyTag.tsx → 通用组件
- ErrorBoundary.tsx, ThemeToggle.tsx → 全局组件

**新建页面组件** (src/pages/):
- RiskOverview.tsx
- PlanningWorkbench.tsx
- VersionComparison.tsx
- DataImport.tsx
- SettingsCenter.tsx

**新建子组件目录**:
```
src/components/
├── workbench/
│   ├── MaterialTreePanel.tsx      // 左侧树形物料池
│   ├── ScheduleTimelinePanel.tsx  // 右侧排程时间线
│   ├── CapacityOverviewBar.tsx    // 产能概览条形图
│   ├── BatchOperationToolbar.tsx  // 批量操作工具栏
│   └── OneClickOptimizeMenu.tsx   // 一键优化菜单
├── comparison/
│   ├── StrategyDraftCards.tsx     // 策略草案卡片
│   ├── VersionDiffTable.tsx       // 版本差异表格
│   ├── KPIComparisonPanel.tsx     // KPI对比面板
│   └── ConfirmPublishModal.tsx    // 确认发布弹窗
├── overview/
│   ├── KPIBand.tsx               // 顶部KPI带
│   ├── DimensionTabs.tsx         // 多维度Tab切换
│   ├── ProblemList.tsx           // 问题列表
│   └── DrilldownDrawer.tsx       // 下钻抽屉
└── settings/
    ├── SystemConfigPanel.tsx     // 系统配置面板
    ├── ActionLogPanel.tsx        // 操作日志面板
    ├── MachineConfigPanel.tsx    // 机组配置面板
    └── UserPreferencesPanel.tsx  // 用户偏好面板
```

### 12.5 Hooks 新增 (src/hooks/)

| Hook | 用途 | 依赖 |
|-----|-----|-----|
| `useRiskOverviewData.ts` | 聚合D1-D6数据 | use-decision-queries |
| `useWorkbenchFilters.ts` | 工作台筛选状态 | use-global-store |
| `useVersionComparison.ts` | 版本对比逻辑 | planApi.compareVersions |
| `useStrategyDraft.ts` | 策略草案管理 | 新API |
| `useUserPreferences.ts` | 用户偏好 | localStorage + store |

### 12.6 改动统计

| 类别 | 删除 | 新增 | 修改 |
|-----|-----|-----|-----|
| 路由 | 7个 | 5个 | 1个(根路由) |
| Store | 0 | 0 | 2个(字段) |
| 类型文件 | 0 | 2个 | 1个 |
| 页面组件 | 7个 | 5个 | 0 |
| 子组件 | 0 | 15个+ | 0 |
| Hooks | 0 | 5个 | 0 |
| API | 0 | 2个 | 0 |

---

---

## 十三、样式体系、性能优化、错误处理评估

### 13.1 样式体系评估

#### 13.1.1 现状总结

**优势**：
- ✅ 完整的设计令牌体系（src/theme/tokens.ts）
- ✅ 双主题支持（暗色/亮色），默认暗色模式
- ✅ Ant Design v5 ConfigProvider 集成
- ✅ 主题持久化（localStorage）
- ✅ 工业级配色方案（紧急度L0-L3、状态颜色、产能颜色）

**不足**：
- ❌ 大量内联样式重复（301处 style={{}}）
- ❌ 无响应式断点定义（固定布局，仅适配桌面）
- ❌ 无CSS变量输出
- ❌ 缺少组件样式隔离机制

#### 13.1.2 核心文件

| 文件 | 用途 | 大小 |
|-----|-----|-----|
| src/theme/tokens.ts | 设计令牌定义 | 5.6KB |
| src/theme/darkTheme.ts | 暗色主题配置 | 7.1KB |
| src/theme/lightTheme.ts | 亮色主题配置 | 7.0KB |
| src/theme/ThemeContext.tsx | 主题上下文 | 2.8KB |

#### 13.1.3 改进建议（与重构同步）

**高优先级**：
1. **响应式断点定义**（虽然当前为桌面系统，但需为未来扩展预留）
   ```typescript
   // 在 tokens.ts 中添加
   export const BREAKPOINTS = {
     SM: 576,   // 小屏幕
     MD: 768,   // 中等屏幕
     LG: 992,   // 大屏幕
     XL: 1200,  // 超大屏幕
     XXL: 1600, // 2K 屏幕
   } as const;
   ```

2. **提取常用样式对象**，减少重复
   ```typescript
   // 新建 src/styles/commonStyles.ts
   export const containerStyles = {
     card: {
       marginBottom: SPACING.BASE,
       borderRadius: BORDER_RADIUS.LG,
       padding: SPACING.BASE,
     },
     flexCenter: {
       display: 'flex',
       alignItems: 'center',
       justifyContent: 'center',
     },
   };
   ```

**中优先级**：
3. 生成CSS变量（可选，当前Ant Design ConfigProvider已足够）

**建议**：样式体系整体良好，仅需在新组件中应用最佳实践，无需大规模重构。

---

### 13.2 性能优化评估

#### 13.2.1 现状总结

**已实现的优化**：
- ✅ 路由懒加载全覆盖（13个页面，React.lazy + Suspense）
- ✅ React Query 缓存配置合理（staleTime: 5min, gcTime: 10min）
- ✅ ECharts 性能工具库完整（src/utils/echarts-performance.ts）
- ✅ 专用性能监控 Hook（src/hooks/use-performance-monitor.ts）
- ✅ 46%组件使用 memo/useMemo/useCallback

**关键性能瓶颈**：
- 🔴 **超大 Chunk 问题**（最高优先级）
  - MaterialManagement.js: 671KB（应 <300KB）
  - RiskDashboard.js: 334KB（应 <300KB）
- 🔴 **缺少 Vendor 分包**（高优先级）
  - antd、echarts、react 等库未分离
- 🟡 **虚拟滚动未使用**（中优先级）
  - react-window 已安装但未使用
  - 大数据表格（>1000行）性能问题

#### 13.2.2 必须改进项（融入重构）

**第一阶段（立即实施）**：

1. **Vite 分包配置**（vite.config.ts）
   ```typescript
   build: {
     rollupOptions: {
       output: {
         manualChunks: {
           'vendor-react': ['react', 'react-dom', 'react-router-dom'],
           'vendor-antd': ['antd', '@ant-design/pro-components'],
           'vendor-chart': ['echarts', 'echarts-for-react'],
           'vendor-query': ['@tanstack/react-query'],
         },
       },
     },
   }
   ```

2. **虚拟滚动应用**
   - 新建组件时，对大数据列表使用 react-window
   - 优先应用于：计划工作台物料列表、版本对比物料明细

**第二阶段（1-2周内）**：

3. **React Query 细粒度缓存**
   ```typescript
   // 静态数据（配置）
   staleTime: 30 * 60 * 1000, gcTime: 60 * 60 * 1000

   // 动态数据（风险）
   staleTime: 1 * 60 * 1000, gcTime: 5 * 60 * 1000

   // 实时数据（库存）
   staleTime: 30 * 1000, gcTime: 1 * 60 * 1000
   ```

4. **增加组件 Memo 覆盖率**至70%+（特别是列表项组件）

**第三阶段（长期优化）**：
5. 图表库按需加载（dynamic imports）
6. Web Worker 处理数据密集操作（可选）

#### 13.2.3 验收指标

| 指标 | 当前 | 目标 |
|-----|-----|-----|
| 最大单个 Chunk | 671KB | <300KB |
| Vendor Chunk 独立 | ❌ | ✅ |
| 虚拟滚动覆盖 | 0% | 100%（大数据列表） |
| 组件 Memo 覆盖率 | 46% | 70%+ |
| 首屏加载时间 | 未测 | <2s |

---

### 13.3 错误处理评估

#### 13.3.1 现状总结

**已实现的错误处理架构**：

**三层API错误处理**：
1. **IpcClient 统一拦截**（src/api/ipcClient.tsx）
   - ✅ 自动重试机制（指数退避）
   - ✅ 超时控制（默认30s）
   - ✅ 统一错误弹窗（可复制错误信息）

2. **DecisionService 验证层**（src/services/decision-service.ts）
   - ✅ Zod schema 运行时验证
   - ✅ 自定义错误类（ValidationError, DecisionApiError）

3. **React Query 全局配置**（src/app/query-client.tsx）
   - ✅ Query 自动重试2次
   - ✅ Mutation 全局错误处理

**UI错误处理**：
- ✅ ErrorBoundary 顶层覆盖（src/components/ErrorBoundary.tsx）
- ✅ Spin + loading 状态（混合使用）
- ✅ CustomEmpty 空状态组件（src/components/CustomEmpty.tsx）
- ✅ LongTaskProgress 长任务进度（src/components/LongTaskProgress.tsx）

**不足**：
- ❌ 缺少离线支持
- ❌ 缺少远程日志上报（无Sentry）
- ❌ 缺少手动重试按钮
- ❌ 缺少骨架屏（Skeleton）
- ⚠️ ErrorBoundary 仅顶层，缺少页面级边界

#### 13.3.2 必须改进项（融入重构）

**立即改进（1-2周）**：

1. **网络状态监听 + 离线提示**
   ```typescript
   // 使用在线状态检测
   const isOnline = navigator.onLine;
   if (!isOnline) {
     return <Alert type="warning" message="当前处于离线状态" />;
   }
   ```

2. **页面级 ErrorBoundary**
   - 为每个新页面组件包装 ErrorBoundary
   - 关键操作区域（批量操作、策略对比）独立边界

3. **手动重试按钮**
   ```typescript
   if (error) {
     return (
       <Alert
         type="error"
         message="数据加载失败"
         action={<Button onClick={refetch}>重试</Button>}
       />
     );
   }
   ```

4. **骨架屏/Placeholder**
   - 使用 Ant Design `Skeleton` 组件
   - 替换部分 Spin 为 Skeleton，提升感知性能

**中期改进（1个月）**：

5. **集成错误日志上报**（Sentry 或 Tauri 自有日志系统）
   ```typescript
   // ErrorBoundary 中添加
   componentDidCatch(error, errorInfo) {
     console.error('ErrorBoundary caught:', error, errorInfo);
     // Sentry.captureException(error);
   }
   ```

6. **断路器模式**
   - 连续失败5次后停止重试，显示"服务不可用"

**长期优化（2个月）**：
7. 本地缓存 + 离线模式（可选，取决于业务需求）
8. 全链路错误追踪（错误上下文、用户行为轨迹）

#### 13.3.3 改进优先级矩阵

| 功能 | 现状 | 问题级别 | 改进优先级 | 实施阶段 |
|-----|------|--------|---------|---------|
| 错误边界 | 仅顶层 | ⚠️ 中 | 高 | Phase 1 |
| 手动重试 | 缺失 | ⚠️ 中 | 高 | Phase 1 |
| 骨架屏 | 缺失 | ⚠️ 中 | 中 | Phase 2 |
| 离线支持 | 缺失 | 🔴 高 | 高 | Phase 2 |
| 日志上报 | 仅console | 🔴 高 | 中 | Phase 3 |
| 网络监听 | 缺失 | 🔴 高 | 高 | Phase 1 |

---

### 13.4 综合改进计划整合

**与页面重构同步进行的改进**：

#### Phase 1-2（风险概览 + 计划工作台，前4周）
- ✅ 新组件使用提取的 commonStyles
- ✅ 新组件应用 React.memo、useMemo
- ✅ 大数据列表使用 react-window
- ✅ 为页面组件添加 ErrorBoundary
- ✅ 添加手动重试按钮
- ✅ 使用 Skeleton 优化加载体验
- ✅ 网络状态监听

#### Phase 3（版本对比页，2-3周）
- ✅ 应用 Vite 分包配置
- ✅ 策略对比数据使用虚拟滚动
- ✅ 离线状态提示

#### Phase 6（整体优化，1周）
- ✅ Bundle 分析和优化
- ✅ 性能测试和验收
- ✅ 错误处理覆盖率检查
- ✅ 集成日志上报（可选）

---

---

## 十四、详细实施指南

### 14.1 Phase 1: 基础设施搭建（第1-2周）

#### 14.1.1 任务清单

**第1天：项目准备**
- [ ] 创建功能分支 `feature/frontend-refactor`
- [ ] 备份当前代码（git tag v0.3-backup）
- [ ] 创建开发计划看板（如Trello/Notion）
- [ ] 准备测试数据

**第2-3天：类型定义和工具**
- [ ] 创建 `src/types/comparison.ts`
- [ ] 创建 `src/types/preferences.ts`
- [ ] 修改 `src/types/index.ts` 导出新类型
- [ ] 创建 `src/styles/commonStyles.ts`
- [ ] 更新 `src/theme/tokens.ts` 添加 BREAKPOINTS

**第4-5天：Vite 配置优化**
```typescript
// vite.config.ts 完整配置
import { defineConfig } from 'vite';
import react from '@vitejs/plugin-react';

export default defineConfig({
  plugins: [react()],
  clearScreen: false,
  server: {
    port: 5173,
    strictPort: true,
  },
  envPrefix: ['VITE_', 'TAURI_'],
  build: {
    target: ['es2021', 'chrome100', 'safari13'],
    minify: !process.env.TAURI_DEBUG ? 'esbuild' : false,
    sourcemap: !!process.env.TAURI_DEBUG,
    rollupOptions: {
      output: {
        manualChunks: {
          'vendor-react': ['react', 'react-dom', 'react-router-dom'],
          'vendor-antd': ['antd', '@ant-design/pro-components'],
          'vendor-chart': ['echarts', 'echarts-for-react'],
          'vendor-query': ['@tanstack/react-query'],
        },
        chunkFileNames: 'assets/[name]-[hash].js',
        entryFileNames: 'assets/[name]-[hash].js',
        assetFileNames: 'assets/[name]-[hash].[ext]',
      },
    },
    chunkSizeWarningLimit: 500, // 500KB 警告阈值
  },
});
```

**第6-7天：状态管理扩展**
```typescript
// src/stores/use-global-store.ts 新增字段
interface GlobalState {
  // 版本对比
  versionComparisonMode: 'DRAFT_COMPARISON' | 'HISTORICAL' | null;
  selectedVersionA: string | null;
  selectedVersionB: string | null;
  setVersionComparisonMode: (mode: 'DRAFT_COMPARISON' | 'HISTORICAL' | null) => void;
  setSelectedVersions: (versionA: string | null, versionB: string | null) => void;

  // 工作台
  workbenchViewMode: 'MATRIX' | 'GANTT' | 'CARD';
  workbenchFilters: {
    machineCode: string | null;
    urgencyLevel: UrgencyLevel | null;
    lockStatus: 'all' | 'locked' | 'unlocked';
  };
  setWorkbenchViewMode: (mode: 'MATRIX' | 'GANTT' | 'CARD') => void;
  setWorkbenchFilters: (filters: Partial<GlobalState['workbenchFilters']>) => void;

  // 用户偏好
  userPreferences: UserPreferences;
  setUserPreferences: (preferences: Partial<UserPreferences>) => void;
}
```

**第8-10天：路由重构**
```typescript
// src/router/index.tsx 新路由结构
import { createBrowserRouter, Navigate } from 'react-router-dom';
import React from 'react';
import App from '../App';

// 懒加载页面组件
const RiskOverview = React.lazy(() => import('../pages/RiskOverview'));
const PlanningWorkbench = React.lazy(() => import('../pages/PlanningWorkbench'));
const VersionComparison = React.lazy(() => import('../pages/VersionComparison'));
const DataImport = React.lazy(() => import('../pages/DataImport'));
const SettingsCenter = React.lazy(() => import('../pages/SettingsCenter'));

export const router = createBrowserRouter([
  {
    path: '/',
    element: <App />,
    errorElement: <ErrorPage />,
    children: [
      {
        index: true,
        element: <Navigate to="/overview" replace />,
      },
      {
        path: 'overview',
        element: (
          <React.Suspense fallback={<PageSkeleton />}>
            <RiskOverview />
          </React.Suspense>
        ),
      },
      {
        path: 'workbench',
        element: (
          <React.Suspense fallback={<PageSkeleton />}>
            <PlanningWorkbench />
          </React.Suspense>
        ),
      },
      {
        path: 'comparison',
        element: (
          <React.Suspense fallback={<PageSkeleton />}>
            <VersionComparison />
          </React.Suspense>
        ),
      },
      {
        path: 'import',
        element: (
          <React.Suspense fallback={<PageSkeleton />}>
            <DataImport />
          </React.Suspense>
        ),
      },
      {
        path: 'settings',
        element: (
          <React.Suspense fallback={<PageSkeleton />}>
            <SettingsCenter />
          </React.Suspense>
        ),
      },
    ],
  },
]);
```

#### 14.1.2 验收标准

- [ ] 所有类型文件编译无错误
- [ ] Vite 构建成功，Bundle 分析符合预期
- [ ] 状态管理字段测试通过
- [ ] 路由跳转正常（虽然页面还是空的）

---

### 14.2 Phase 2: 风险概览页（第3-4周）

#### 14.2.1 组件设计规格

**RiskOverview.tsx（主页面组件）**
```typescript
// src/pages/RiskOverview.tsx
import { FC } from 'react';
import { Layout, Spin } from 'antd';
import { ErrorBoundary } from '@/components/ErrorBoundary';
import { KPIBand } from '@/components/overview/KPIBand';
import { DimensionTabs } from '@/components/overview/DimensionTabs';
import { useRiskOverviewData } from '@/hooks/useRiskOverviewData';

export const RiskOverview: FC = () => {
  const { data, isLoading, error, refetch } = useRiskOverviewData();

  if (isLoading) {
    return <Skeleton active />;
  }

  if (error) {
    return (
      <Alert
        type="error"
        message="数据加载失败"
        description={error.message}
        action={<Button onClick={() => refetch()}>重试</Button>}
      />
    );
  }

  return (
    <ErrorBoundary>
      <Layout style={{ padding: '24px', background: 'transparent' }}>
        {/* 顶部 KPI 带 */}
        <KPIBand data={data.kpi} />

        {/* 多维度详情区 */}
        <DimensionTabs data={data} />
      </Layout>
    </ErrorBoundary>
  );
};
```

**KPIBand.tsx（顶部KPI带组件）**
```typescript
// src/components/overview/KPIBand.tsx
import { FC, useMemo } from 'react';
import { Row, Col, Card, Statistic, Badge } from 'antd';
import { URGENCY_COLORS, STATE_COLORS } from '@/theme/tokens';

interface KPIBandProps {
  data: {
    overallRisk: 'low' | 'medium' | 'high';
    l3Orders: { total: number; overdue: number };
    capacityUtilization: { avg: number; maxMachine: string };
    coldStockPressure: 'low' | 'medium' | 'high';
    rollCampaign: { status: 'normal' | 'warning'; remaining: number };
  };
}

export const KPIBand: FC<KPIBandProps> = React.memo(({ data }) => {
  const riskColor = useMemo(() => {
    switch (data.overallRisk) {
      case 'high': return URGENCY_COLORS.L3_EMERGENCY;
      case 'medium': return URGENCY_COLORS.L2_HIGH;
      default: return URGENCY_COLORS.L0_NORMAL;
    }
  }, [data.overallRisk]);

  return (
    <Row gutter={16} style={{ marginBottom: 24 }}>
      <Col span={4}>
        <Card size="small">
          <Statistic
            title="整体风险"
            value={data.overallRisk === 'high' ? '高' : data.overallRisk === 'medium' ? '中' : '低'}
            valueStyle={{ color: riskColor, fontSize: 24, fontWeight: 'bold' }}
            prefix={<Badge color={riskColor} />}
          />
        </Card>
      </Col>

      <Col span={5}>
        <Card size="small">
          <Statistic
            title="L3订单"
            value={data.l3Orders.total}
            suffix="个"
            valueStyle={{ fontSize: 20 }}
          />
          {data.l3Orders.overdue > 0 && (
            <div style={{ color: URGENCY_COLORS.L3_EMERGENCY, fontSize: 12 }}>
              {data.l3Orders.overdue} 个逾期
            </div>
          )}
        </Card>
      </Col>

      <Col span={5}>
        <Card size="small">
          <Statistic
            title="利用率"
            value={data.capacityUtilization.avg}
            suffix="%"
            precision={1}
            valueStyle={{ fontSize: 20 }}
          />
          <div style={{ fontSize: 12, color: '#8c8c8c' }}>
            {data.capacityUtilization.maxMachine} 偏高
          </div>
        </Card>
      </Col>

      <Col span={5}>
        <Card size="small">
          <Statistic
            title="冷坨压力"
            value={data.coldStockPressure === 'high' ? '高' : '中'}
            valueStyle={{
              color: data.coldStockPressure === 'high'
                ? URGENCY_COLORS.L3_EMERGENCY
                : URGENCY_COLORS.L2_HIGH,
              fontSize: 20,
            }}
          />
        </Card>
      </Col>

      <Col span={5}>
        <Card size="small">
          <Statistic
            title="换辊"
            value={data.rollCampaign.status === 'normal' ? '正常' : '警告'}
            valueStyle={{
              color: data.rollCampaign.status === 'normal'
                ? STATE_COLORS.READY
                : URGENCY_COLORS.L2_HIGH,
              fontSize: 20,
            }}
          />
          <div style={{ fontSize: 12, color: '#8c8c8c' }}>
            剩余 {data.rollCampaign.remaining}t
          </div>
        </Card>
      </Col>
    </Row>
  );
});
```

**DimensionTabs.tsx（多维度Tab组件）**
```typescript
// src/components/overview/DimensionTabs.tsx
import { FC, useState } from 'react';
import { Tabs, Card } from 'antd';
import { ProblemList } from './ProblemList';
import { OrderDimension } from './OrderDimension';
import { CapacityDimension } from './CapacityDimension';
import { StockDimension } from './StockDimension';
import { RollDimension } from './RollDimension';

const { TabPane } = Tabs;

export const DimensionTabs: FC<{ data: any }> = ({ data }) => {
  const [activeKey, setActiveKey] = useState('problems');

  return (
    <Card>
      <Tabs activeKey={activeKey} onChange={setActiveKey}>
        <TabPane tab="问题汇总" key="problems">
          <ProblemList problems={data.problems} />
        </TabPane>

        <TabPane tab="订单维度" key="orders">
          <OrderDimension data={data.orders} />
        </TabPane>

        <TabPane tab="产能维度" key="capacity">
          <CapacityDimension data={data.capacity} />
        </TabPane>

        <TabPane tab="库存维度" key="stock">
          <StockDimension data={data.stock} />
        </TabPane>

        <TabPane tab="换辊维度" key="roll">
          <RollDimension data={data.roll} />
        </TabPane>
      </Tabs>
    </Card>
  );
};
```

**ProblemList.tsx（问题列表组件）**
```typescript
// src/components/overview/ProblemList.tsx
import { FC, useState } from 'react';
import { List, Tag, Button, Space, Select, Drawer } from 'antd';
import { useNavigate } from 'react-router-dom';
import { URGENCY_COLORS } from '@/theme/tokens';

interface Problem {
  id: string;
  level: 'P0' | 'P1' | 'P2';
  title: string;
  description: string;
  impact: '高' | '中' | '低';
  timeframe: '今日' | '明日' | '持续';
  relatedItems: string[];
}

interface ProblemListProps {
  problems: Problem[];
}

export const ProblemList: FC<ProblemListProps> = ({ problems }) => {
  const navigate = useNavigate();
  const [sortBy, setSortBy] = useState<'severity' | 'time'>('severity');
  const [filterType, setFilterType] = useState<string>('all');
  const [selectedProblem, setSelectedProblem] = useState<Problem | null>(null);

  const sortedProblems = useMemo(() => {
    let sorted = [...problems];
    if (sortBy === 'severity') {
      sorted.sort((a, b) => {
        const order = { P0: 0, P1: 1, P2: 2 };
        return order[a.level] - order[b.level];
      });
    }
    return sorted;
  }, [problems, sortBy]);

  const getLevelColor = (level: Problem['level']) => {
    switch (level) {
      case 'P0': return URGENCY_COLORS.L3_EMERGENCY;
      case 'P1': return URGENCY_COLORS.L2_HIGH;
      case 'P2': return URGENCY_COLORS.L1_MEDIUM;
    }
  };

  return (
    <>
      {/* 筛选排序工具栏 */}
      <Space style={{ marginBottom: 16 }}>
        <span>排序:</span>
        <Select value={sortBy} onChange={setSortBy} style={{ width: 150 }}>
          <Select.Option value="severity">默认按严重度</Select.Option>
          <Select.Option value="time">按时间</Select.Option>
        </Select>

        <span style={{ marginLeft: 16 }}>筛选:</span>
        <Select value={filterType} onChange={setFilterType} style={{ width: 120 }}>
          <Select.Option value="all">全部类型</Select.Option>
          <Select.Option value="order">订单问题</Select.Option>
          <Select.Option value="capacity">产能问题</Select.Option>
          <Select.Option value="stock">库存问题</Select.Option>
        </Select>
      </Space>

      {/* 问题列表 */}
      <List
        itemLayout="vertical"
        dataSource={sortedProblems}
        renderItem={(problem) => (
          <List.Item
            actions={[
              <Button type="link" onClick={() => setSelectedProblem(problem)}>
                查看详情
              </Button>,
              <Button type="primary" onClick={() => navigate('/workbench')}>
                去计划工作台处理
              </Button>,
            ]}
          >
            <List.Item.Meta
              title={
                <Space>
                  <Tag color={getLevelColor(problem.level)} style={{ fontWeight: 'bold' }}>
                    {problem.level}
                  </Tag>
                  <span style={{ fontSize: 16 }}>{problem.title}</span>
                  <Tag>影响: {problem.impact}</Tag>
                  <Tag>{problem.timeframe}</Tag>
                </Space>
              }
              description={
                <div style={{ marginLeft: 60 }}>
                  > {problem.description}
                </div>
              }
            />
          </List.Item>
        )}
      />

      {/* 问题详情抽屉 */}
      <Drawer
        title={`问题详情: ${selectedProblem?.title}`}
        placement="right"
        width={480}
        open={!!selectedProblem}
        onClose={() => setSelectedProblem(null)}
      >
        {selectedProblem && (
          <div>
            <p><strong>等级:</strong> {selectedProblem.level}</p>
            <p><strong>影响:</strong> {selectedProblem.impact}</p>
            <p><strong>时间范围:</strong> {selectedProblem.timeframe}</p>
            <p><strong>描述:</strong> {selectedProblem.description}</p>
            <p><strong>相关物料:</strong></p>
            <ul>
              {selectedProblem.relatedItems.map((item) => (
                <li key={item}>{item}</li>
              ))}
            </ul>
            <Button
              type="primary"
              block
              style={{ marginTop: 24 }}
              onClick={() => {
                setSelectedProblem(null);
                navigate('/workbench');
              }}
            >
              去计划工作台处理
            </Button>
          </div>
        )}
      </Drawer>
    </>
  );
};
```

#### 14.2.2 自定义Hook实现

```typescript
// src/hooks/useRiskOverviewData.ts
import { useQuery } from '@tanstack/react-query';
import { decisionApi } from '@/services/decision-service';
import { useActiveVersionId } from '@/stores/use-global-store';

export const useRiskOverviewData = () => {
  const versionId = useActiveVersionId();

  return useQuery({
    queryKey: ['riskOverview', versionId],
    queryFn: async () => {
      // 并行请求多个数据源
      const [kpi, problems, orders, capacity, stock, roll] = await Promise.all([
        decisionApi.getGlobalKPI(versionId),
        decisionApi.getProblems(versionId),
        decisionApi.getOrderSummary(versionId),
        decisionApi.getCapacitySummary(versionId),
        decisionApi.getStockSummary(versionId),
        decisionApi.getRollSummary(versionId),
      ]);

      return {
        kpi,
        problems,
        orders,
        capacity,
        stock,
        roll,
      };
    },
    staleTime: 1 * 60 * 1000, // 1分钟
    gcTime: 5 * 60 * 1000, // 5分钟
    enabled: !!versionId,
  });
};
```

#### 14.2.3 测试用例

```typescript
// src/pages/__tests__/RiskOverview.test.tsx
import { render, screen, waitFor } from '@testing-library/react';
import { QueryClientProvider } from '@tanstack/react-query';
import { RiskOverview } from '../RiskOverview';
import { queryClient } from '@/app/query-client';

describe('RiskOverview', () => {
  it('显示加载状态', () => {
    render(
      <QueryClientProvider client={queryClient}>
        <RiskOverview />
      </QueryClientProvider>
    );
    expect(screen.getByText(/加载中/)).toBeInTheDocument();
  });

  it('加载数据后显示KPI带', async () => {
    render(
      <QueryClientProvider client={queryClient}>
        <RiskOverview />
      </QueryClientProvider>
    );

    await waitFor(() => {
      expect(screen.getByText('整体风险')).toBeInTheDocument();
      expect(screen.getByText('L3订单')).toBeInTheDocument();
      expect(screen.getByText('利用率')).toBeInTheDocument();
    });
  });

  it('点击"去处理"跳转到工作台', async () => {
    // 测试导航功能
  });
});
```

---

### 14.3 Phase 3: 计划工作台页（第5-8周）

#### 14.3.1 核心组件架构

```
PlanningWorkbench (主页面)
├── Toolbar (工具栏)
│   ├── VersionSelector (版本选择器)
│   ├── DateRangePicker (日期范围选择)
│   ├── BatchOperationDropdown (批量操作菜单)
│   └── OneClickOptimizeDropdown (一键优化菜单)
├── WorkbenchLayout (工作区布局)
│   ├── MaterialTreePanel (左侧物料树)
│   │   ├── MachineNode (机组节点)
│   │   │   ├── StatusNode (状态分组)
│   │   │   │   └── MaterialItem (物料项)
│   │   └── SearchBar (搜索栏)
│   └── ScheduleTimelinePanel (右侧时间线)
│       ├── CapacityOverviewBar (产能概览条)
│       ├── ViewModeSwitcher (视图切换器)
│       └── TimelineContent (时间线内容)
│           ├── MatrixView (矩阵视图)
│           ├── GanttView (甘特图视图)
│           └── CardView (卡片视图)
└── StatusBar (状态栏)
    ├── SelectionSummary (选择摘要)
    └── QuickActions (快速操作按钮)
```

#### 14.3.2 MaterialTreePanel 详细设计

```typescript
// src/components/workbench/MaterialTreePanel.tsx
import { FC, useState, useMemo } from 'react';
import { Tree, Input, Checkbox, Badge } from 'antd';
import { FolderOutlined, FolderOpenOutlined } from '@ant-design/icons';
import { FixedSizeList } from 'react-window'; // 虚拟滚动
import type { DataNode } from 'antd/es/tree';

interface Material {
  id: string;
  materialNo: string;
  weight: number;
  urgency: 'L0' | 'L1' | 'L2' | 'L3';
  status: 'pending' | 'scheduled' | 'locked';
  machineCode: string;
}

interface MaterialTreePanelProps {
  materials: Material[];
  selectedKeys: string[];
  onSelect: (keys: string[]) => void;
}

export const MaterialTreePanel: FC<MaterialTreePanelProps> = ({
  materials,
  selectedKeys,
  onSelect,
}) => {
  const [searchText, setSearchText] = useState('');
  const [expandedKeys, setExpandedKeys] = useState<string[]>([]);

  // 构建树形数据结构
  const treeData = useMemo(() => {
    const grouped = materials.reduce((acc, material) => {
      const { machineCode, status } = material;

      if (!acc[machineCode]) {
        acc[machineCode] = { pending: [], scheduled: [], locked: [] };
      }

      acc[machineCode][status].push(material);

      return acc;
    }, {} as Record<string, Record<string, Material[]>>);

    return Object.entries(grouped).map(([machineCode, statusGroups]) => ({
      title: `${machineCode} (${Object.values(statusGroups).flat().length}个物料)`,
      key: `machine-${machineCode}`,
      icon: ({ expanded }) => expanded ? <FolderOpenOutlined /> : <FolderOutlined />,
      children: [
        {
          title: `待排 (${statusGroups.pending.length})`,
          key: `${machineCode}-pending`,
          children: statusGroups.pending.map((mat) => ({
            title: (
              <div style={{ display: 'flex', justifyContent: 'space-between' }}>
                <Checkbox checked={selectedKeys.includes(mat.id)}>
                  {mat.materialNo}
                </Checkbox>
                <span>{mat.weight}t</span>
                <Badge color={getUrgencyColor(mat.urgency)} text={mat.urgency} />
              </div>
            ),
            key: mat.id,
            isLeaf: true,
            checkable: true,
          })),
        },
        {
          title: `已排 (${statusGroups.scheduled.length})`,
          key: `${machineCode}-scheduled`,
          children: statusGroups.scheduled.map((mat) => ({
            title: (
              <div style={{ display: 'flex', justifyContent: 'space-between' }}>
                <Checkbox checked={selectedKeys.includes(mat.id)}>
                  {mat.materialNo}
                </Checkbox>
                <span>{mat.weight}t</span>
                <Badge color={getUrgencyColor(mat.urgency)} text={mat.urgency} />
              </div>
            ),
            key: mat.id,
            isLeaf: true,
            checkable: true,
          })),
        },
        {
          title: `已锁定 (${statusGroups.locked.length})`,
          key: `${machineCode}-locked`,
          children: statusGroups.locked.map((mat) => ({
            title: (
              <div style={{ display: 'flex', justifyContent: 'space-between' }}>
                <Checkbox checked={selectedKeys.includes(mat.id)} disabled>
                  {mat.materialNo}
                </Checkbox>
                <span>{mat.weight}t</span>
                <Badge color={getUrgencyColor(mat.urgency)} text={mat.urgency} />
              </div>
            ),
            key: mat.id,
            isLeaf: true,
            checkable: true,
            disabled: true,
          })),
        },
      ],
    }));
  }, [materials, selectedKeys]);

  // 搜索过滤
  const filteredTreeData = useMemo(() => {
    if (!searchText) return treeData;
    return filterTreeData(treeData, searchText);
  }, [treeData, searchText]);

  return (
    <div style={{ height: '100%', display: 'flex', flexDirection: 'column' }}>
      <Input.Search
        placeholder="搜索物料号..."
        value={searchText}
        onChange={(e) => setSearchText(e.target.value)}
        style={{ marginBottom: 8 }}
      />

      <div style={{ flex: 1, overflow: 'auto' }}>
        <Tree
          checkable
          selectable={false}
          expandedKeys={expandedKeys}
          onExpand={setExpandedKeys}
          checkedKeys={selectedKeys}
          onCheck={(checked) => {
            if (Array.isArray(checked)) {
              onSelect(checked as string[]);
            }
          }}
          treeData={filteredTreeData}
        />
      </div>
    </div>
  );
};

// 辅助函数
function getUrgencyColor(urgency: string) {
  switch (urgency) {
    case 'L3': return 'red';
    case 'L2': return 'orange';
    case 'L1': return 'blue';
    default: return 'gray';
  }
}

function filterTreeData(data: DataNode[], text: string): DataNode[] {
  return data
    .map((node) => {
      const children = node.children ? filterTreeData(node.children, text) : [];
      if (node.title?.toString().toLowerCase().includes(text.toLowerCase()) || children.length > 0) {
        return { ...node, children };
      }
      return null;
    })
    .filter((node): node is DataNode => node !== null);
}
```

#### 14.3.3 ScheduleTimelinePanel 虚拟滚动实现

```typescript
// src/components/workbench/ScheduleTimelinePanel.tsx
import { FC, useState } from 'react';
import { Button, Space, Select } from 'antd';
import { FixedSizeList as List } from 'react-window';
import { MatrixView } from './MatrixView';
import { GanttView } from './GanttView';
import { CardView } from './CardView';

type ViewMode = 'MATRIX' | 'GANTT' | 'CARD';

export const ScheduleTimelinePanel: FC = () => {
  const [viewMode, setViewMode] = useState<ViewMode>('MATRIX');

  return (
    <div style={{ height: '100%', display: 'flex', flexDirection: 'column' }}>
      {/* 产能概览条形图 */}
      <CapacityOverviewBar />

      {/* 视图切换器 */}
      <Space style={{ marginBottom: 16 }}>
        <span>视图切换:</span>
        <Button.Group>
          <Button
            type={viewMode === 'MATRIX' ? 'primary' : 'default'}
            onClick={() => setViewMode('MATRIX')}
          >
            矩阵
          </Button>
          <Button
            type={viewMode === 'GANTT' ? 'primary' : 'default'}
            onClick={() => setViewMode('GANTT')}
          >
            甘特图
          </Button>
          <Button
            type={viewMode === 'CARD' ? 'primary' : 'default'}
            onClick={() => setViewMode('CARD')}
          >
            卡片
          </Button>
        </Button.Group>
      </Space>

      {/* 时间线内容（虚拟滚动） */}
      <div style={{ flex: 1, overflow: 'hidden' }}>
        {viewMode === 'MATRIX' && <MatrixView />}
        {viewMode === 'GANTT' && <GanttView />}
        {viewMode === 'CARD' && <CardView />}
      </div>
    </div>
  );
};
```

---

### 14.4 Phase 4-6 简要说明

**Phase 4（版本对比页）**：
- 实现双模式切换（策略草案对比 / 历史版本对比）
- 使用 Grid 布局实现并排卡片对比
- 集成 ECharts 展示 KPI 变化趋势
- 实现版本回滚功能（需后端支持）

**Phase 5（数据导入优化）**：
- 保留 MaterialImport 组件
- 添加导入后自动跳转逻辑
- 优化冲突解决流程

**Phase 6（整体优化）**：
- Bundle 分析（使用 rollup-plugin-visualizer）
- 性能测试（Lighthouse CI）
- E2E 测试（Playwright）
- 错误处理覆盖率检查

---

## 十五、迁移指南

### 15.1 从旧版本迁移

**迁移策略**：渐进式迁移，保持系统可用

#### 15.1.1 路由兼容方案

```typescript
// 保留旧路由的重定向
{
  path: '/dashboard',
  element: <Navigate to="/overview" replace />,
},
{
  path: '/material',
  element: <Navigate to="/workbench" replace />,
},
{
  path: '/visualization',
  element: <Navigate to="/workbench" replace />,
},
```

#### 15.1.2 用户数据迁移

```typescript
// 将旧的 localStorage 数据迁移到新结构
function migrateUserPreferences() {
  const oldPrefs = localStorage.getItem('user_preferences');
  if (oldPrefs) {
    const parsed = JSON.parse(oldPrefs);
    const newPrefs: UserPreferences = {
      defaultTheme: parsed.theme || 'dark',
      autoRefreshInterval: parsed.refreshInterval || 30000,
      sidebarCollapsed: parsed.sidebarCollapsed || false,
      defaultStrategy: 'balanced',
    };
    localStorage.setItem('user_preferences_v2', JSON.stringify(newPrefs));
  }
}
```

### 15.2 组件替换对照表

| 旧组件 | 新组件 | 迁移难度 |
|-------|-------|---------|
| Dashboard | RiskOverview | 低 |
| MaterialManagement | PlanningWorkbench | 高 |
| PlanItemVisualization | PlanningWorkbench | 高 |
| ConfigManagement | SettingsCenter | 低 |
| ActionLogQuery | SettingsCenter (操作日志Tab) | 低 |
| MaterialImport | DataImport | 低（保留优化） |

---

## 十六、测试计划

### 16.1 单元测试（Jest + React Testing Library）

**覆盖率目标**：70%

**关键测试用例**：
```typescript
// src/components/overview/__tests__/KPIBand.test.tsx
describe('KPIBand', () => {
  it('根据风险等级显示正确颜色', () => {
    const { container } = render(<KPIBand data={mockData} />);
    const riskIndicator = container.querySelector('.risk-indicator');
    expect(riskIndicator).toHaveStyle({ color: URGENCY_COLORS.L3_EMERGENCY });
  });

  it('逾期订单为0时不显示警告', () => {
    const data = { ...mockData, l3Orders: { total: 10, overdue: 0 } };
    const { queryByText } = render(<KPIBand data={data} />);
    expect(queryByText(/逾期/)).toBeNull();
  });
});
```

### 16.2 集成测试（Playwright）

**测试流程**：
```typescript
// e2e/workbench.spec.ts
test('完整的批量操作流程', async ({ page }) => {
  // 1. 导航到工作台
  await page.goto('/workbench');

  // 2. 勾选多个物料
  await page.check('[data-testid="material-M001"]');
  await page.check('[data-testid="material-M002"]');

  // 3. 点击批量操作
  await page.click('[data-testid="batch-operation-btn"]');

  // 4. 选择锁定操作
  await page.click('text=锁定');

  // 5. 确认操作
  await page.click('text=确认');

  // 6. 验证物料状态更新
  await expect(page.locator('[data-testid="material-M001-status"]')).toHaveText('已锁定');
});
```

### 16.3 性能测试（Lighthouse CI）

**性能基准**：
- 首屏加载（FCP）< 1.5s
- 最大内容绘制（LCP）< 2.5s
- 累积布局偏移（CLS）< 0.1
- 首次输入延迟（FID）< 100ms

```yaml
# lighthouserc.yml
ci:
  assert:
    preset: lighthouse:recommended
    assertions:
      first-contentful-paint:
        - error
        - maxNumericValue: 1500
      largest-contentful-paint:
        - error
        - maxNumericValue: 2500
      cumulative-layout-shift:
        - error
        - maxNumericValue: 0.1
```

---

## 十七、上线检查清单

### 17.1 代码质量

- [ ] ESLint 无错误
- [ ] TypeScript 类型检查通过
- [ ] 单元测试覆盖率 ≥ 70%
- [ ] 集成测试全部通过
- [ ] Code Review 完成

### 17.2 性能指标

- [ ] Bundle 大小分析通过（无超大 Chunk）
- [ ] Lighthouse 分数 ≥ 90
- [ ] 虚拟滚动应用到所有大列表
- [ ] 图片资源优化（如有）
- [ ] 树摇（Tree Shaking）效果确认

### 17.3 功能验收

- [ ] 风险概览页所有Tab正常
- [ ] 计划工作台批量操作功能正常
- [ ] 版本对比双模式切换正常
- [ ] 数据导入成功率100%
- [ ] 设置中心配置保存正常

### 17.4 浏览器兼容性

- [ ] Chrome（最新版本）
- [ ] Edge（最新版本）
- [ ] Firefox（最新版本）
- [ ] Safari（如果支持Mac）

### 17.5 用户体验

- [ ] 所有页面有加载状态
- [ ] 错误状态有友好提示
- [ ] 空状态有操作指引
- [ ] 关键操作有二次确认
- [ ] 长任务有进度反馈

---

**下一步**: 方案已全面详细完善，可进入实施阶段。建议按以下顺序执行：

1. **Phase 1（第1-2周）**: 基础设施搭建
   - 创建分支、类型定义、Vite配置、状态管理扩展、路由重构

2. **Phase 2（第3-4周）**: 风险概览页
   - RiskOverview主页面、KPIBand、DimensionTabs、ProblemList、DrilldownDrawer
   - useRiskOverviewData Hook
   - 单元测试

3. **Phase 3（第5-8周）**: 计划工作台页
   - PlanningWorkbench主页面、MaterialTreePanel、ScheduleTimelinePanel
   - 批量操作、一键优化、虚拟滚动
   - 集成测试

4. **Phase 4（第9-11周）**: 版本对比页
   - VersionComparison主页面、StrategyDraftCards、VersionDiffTable
   - 双模式切换、KPI对比、版本回滚
   - 需后端API配合

5. **Phase 5（第12周）**: 数据导入优化 + 设置中心
   - DataImport优化、SettingsCenter整合
   - 迁移旧数据

6. **Phase 6（第13-14周）**: 整体优化与上线
   - Bundle分析、性能测试、E2E测试
   - 错误处理验收、上线检查清单

**预计总工期**: 14周（约3.5个月）
