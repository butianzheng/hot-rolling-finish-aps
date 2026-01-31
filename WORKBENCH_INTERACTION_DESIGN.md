# 规划工作台交互设计 - 业务联动和状态可视化方案

## 执行摘要

**核心问题**：
1. ❌ 四个视图（物料池、产能概览、排程视图、风险概览）缺乏有机的业务流联动
2. ❌ 材料状态信息散落，用户难以直观理解材料的**当前状态** → **可操作性** → **风险影响**
3. ❌ 待处理问题与具体的材料/日期/机组聚合视图脱离，跳转后缺乏上下文

**改进方向**：
1. ✅ 建立"风险 → 诊断 → 决策 → 执行"的完整业务流
2. ✅ 设计**分层状态标签系统**，让每个材料的可操作性一目了然
3. ✅ 实现**深链接和上下文保留**，使风险问题可直接导航到具体操作点

---

## 第一部分：业务联动流程再设计

### 1.1 当前问题分析

#### 问题 1：视图间缺乏清晰的业务流关系

**现状**:
```
风险概览 (RiskDashboard)
  ├─ 孤立的信息卡片
  ├─ 点击可跳转到工作台
  └─ 但缺乏具体的"下一步该做什么"提示

物料池 (MaterialPool)
  ├─ 树形展示所有物料
  ├─ 可按机组/状态筛选
  └─ 但不清楚"哪些物料是问题物料"

产能概览 (CapacityTimelineContainer)
  ├─ 堆叠条形图展示紧急度分布
  ├─ 高亮超产能日期
  └─ 但用户看不到具体是哪些物料造成的超产能

排程视图 (ScheduleCardView/GanttView)
  ├─ 列出所有排程项
  └─ 但缺乏"这个排程是否最优"的判断
```

**本质问题**：这些视图像"四个独立的仪表盘"，而不是"一个完整的诊疗工作台"。

---

#### 问题 2：材料可操作性不清晰

**当前状态**：
```
material_state 有 7 个值：
  'PENDING_MATURE' / 'READY' / 'LOCKED' / 'FORCE_RELEASE' / 'BLOCKED' / 'SCHEDULED' / 'UNKNOWN'

但用户看物料列表时不知道：
  ❓ 这个物料能不能移动到其他日期？(取决于 lock_flag + sched_state + capacity)
  ❓ 这个物料为什么没被排程？(原因是冷料? 产能不足? 被锁定?)
  ❓ 强制放行这个冷料会不会导致轧辊更换？(需要计算)
  ❓ 如果调整这个物料，哪些日期的产能会改变？
```

**根本问题**：状态信息是**被动展示**的，而不是**主动引导**用户做决策。

---

#### 问题 3：风险问题和操作场景脱离

**跳转流程的问题**：
```
1. 风险概览看到"M3 2024-02-01 产能溢出 5t"
2. 点击跳转到工作台
3. 工作台打开，但没有给出任何上下文：
   ├─ M3 这天哪些物料造成了溢出？
   ├─ 能否通过移动某些物料到前后天来解决？
   ├─ 是否应该拆分大件或调整优先级？
   └─ 操作后的产能影响预测在哪里？
```

---

### 1.2 改进方案：建立"事件驱动"的业务流

#### 核心设计原则

1. **风险是触发点**：风险概览是"监测仪表盘"，一旦发现异常，就主动推送到工作台
2. **工作台是诊疗中心**：物料池是"选择器"，产能概览是"预测器"，排程视图是"执行器"
3. **决策有反馈**：每一个操作都要即时显示影响（产能变化、风险等级降低等）

#### 重新定义四个视图的职责

| 视图 | 旧职责 | 新职责 | 核心交互 |
|------|--------|--------|---------|
| **风险概览** | 展示指标 | **风险诊断** | 点击具体问题 → 跳转到工作台（带上下文） |
| **物料池** | 物料列表 | **问题隔离** | 筛选问题物料 → 显示可操作项 |
| **产能概览** | 产能趋势 | **影响预测** | 实时显示选中物料对产能的影响 |
| **排程视图** | 排程查看 | **决策执行** | 拖拽、批量操作，实时预测后续风险 |

#### 建议的业务流：风险驱动工作流

```
┌─────────────────────────────────────────────────────────┐
│  1. 问题发现 (Risk Detection)                           │
│     风险概览显示异常：                                   │
│     ├─ 危险日期 (产能溢出/L3积压)                       │
│     ├─ 冻结材料异常                                      │
│     └─ 轧辊换辊预警                                      │
└────────┬────────────────────────────────────────────────┘
         │ 点击"诊断此问题"
         ↓
┌─────────────────────────────────────────────────────────┐
│  2. 问题诊断 (Root Cause Analysis)                      │
│     工作台自动打开，并：                                  │
│     ├─ 高亮问题所在的机组和日期                         │
│     ├─ 列出该日期的所有物料（按紧急度排序）             │
│     ├─ 标记"可操作物料"（可移动/可强制放行等）          │
│     └─ 显示"若移动该物料，产能影响预测"                 │
└────────┬────────────────────────────────────────────────┘
         │ 用户选中某个物料
         ↓
┌─────────────────────────────────────────────────────────┐
│  3. 方案决策 (Decision Making)                          │
│     产能概览实时显示：                                   │
│     ├─ "若移动到 2024-01-30，产能变化：-5t / +3t"      │
│     ├─ "操作后是否继续超产能？"                         │
│     └─ "操作的风险等级：低 / 中 / 高"                   │
│                                                          │
│     排程视图显示：                                       │
│     ├─ 目标日期的当前排程                               │
│     ├─ 拖拽预览（拖到新位置，产能色变）                 │
│     └─ "此操作后的新风险等级"                           │
└────────┬────────────────────────────────────────────────┘
         │ 确认操作（或批量操作）
         ↓
┌─────────────────────────────────────────────────────────┐
│  4. 决策执行 (Execution)                                │
│     ├─ 批量操作（移动/锁定/强制放行）                   │
│     ├─ 排程重算（后端 re-sched）                         │
│     └─ 即时反馈（ActionLog 记录，审计追踪）             │
└────────┬────────────────────────────────────────────────┘
         │ 操作完成
         ↓
┌─────────────────────────────────────────────────────────┐
│  5. 结果验证 (Verification)                             │
│     ├─ 工作台刷新，风险指标更新                         │
│     ├─ 风险概览卡片绿化（风险降低）                     │
│     └─ 用户可自动导航回风险概览查看改进效果             │
└─────────────────────────────────────────────────────────┘
```

---

### 1.3 具体交互设计

#### A. 风险概览 → 工作台的深链接

**当前**:
```typescript
// 简单跳转，无上下文
onClick={() => navigate('/workbench')}
```

**改进**:
```typescript
// 带完整上下文的深链接
onClick={() => {
  navigate('/workbench', {
    state: {
      triggerContext: {
        type: 'CAPACITY_OVERFLOW',          // 问题类型
        machineCode: data.machine_code,     // 具体机组
        planDate: data.date,                // 具体日期
        affectedMaterials: [
          'MAT-001', 'MAT-003'              // 造成超产能的物料
        ],
        recommendation: 'MOVE_FORWARD',     // 建议操作
        expectedImpact: {
          currentOverflow: 5.2,             // 当前溢出 5.2t
          targetOverflow: 0.8,              // 目标溢出 0.8t
        }
      }
    }
  });
}}
```

**工作台接收上下文后**:
```typescript
// PlanningWorkbench.tsx 中
const location = useLocation();
const context = location.state?.triggerContext;

useEffect(() => {
  if (!context) return;

  // 1. 自动选中问题机组
  setWorkbenchFilters({ machineCode: context.machineCode });

  // 2. 自动选中相关物料
  setSelectedMaterialIds(context.affectedMaterials);

  // 3. 自动滚动到问题日期
  scrollToDate(context.planDate);

  // 4. 显示诊断面板
  setDiagnosisPanelOpen(true);
  setDiagnosisPanelContext(context);
}, [context]);
```

---

#### B. 物料池的"可操作性标记"

**当前**:
```
□ MAT-001   M1   25t   L2    [lock]
□ MAT-002   M2   15t   L0
□ MAT-003   M3   8t    L3    [lock]
```

**改进**:
```
□ MAT-001   M1   25t   L2
  ├─ 状态：已排产 (2024-01-31)
  ├─ 操作：[可移动] [可强制放行] [不可锁定]
  └─ 提示：若移动到2024-02-01，产能从 39/40 → 29/40 (改善) ✓

□ MAT-002   M2   15t   L0
  ├─ 状态：冷料未排 (库龄 12d)
  ├─ 操作：[强制放行] [不可移动] [不可锁定]
  └─ 提示：强制放行需管理员批准 ⚠️

□ MAT-003   M3   8t    L3
  ├─ 状态：冻结已排 (冻结中)
  ├─ 操作：[不可移动] [不可锁定] [解冻需审批]
  └─ 提示：冻结区保护，不建议调整 🔒
```

---

#### C. 产能概览的"影响预测"

**当前**:
```
[M1 产能条] 占用: 39.5t / 目标: 40t / 限制: 45t
[M2 产能条] 占用: 42t / 目标: 40t / 限制: 45t ⚠️ 超产能
```

**改进**:
```
┌─ M1 产能分配 (2024-01-31) ─────────────────────────┐
│ 占用: 39.5t / 目标: 40t / 限制: 45t              │
│ ├─ L3 紧急: [████████] 2t (2件)                  │
│ ├─ L2 高:   [████████████████] 5.5t (3件)        │
│ ├─ L1 中:   [████████████████████████] 12t (5件) │
│ └─ L0 正常: [██████████████████████████████] 20t  │
│                                                    │
│ 当前选中: MAT-001 (25t L2) [highlight]           │
│ 若移动：                                           │
│   → 移到 2024-01-30  产能: 29/40 (改善 ✓)         │
│   → 移到 2024-02-01  产能: 42/40 (加剧 ✗)         │
│   → 强制放行        产能: +8t (警告 ⚠️)           │
│                                                    │
│ [建议] 移动到 2024-01-30 (产能最优)              │
└────────────────────────────────────────────────────┘
```

---

#### D. 排程视图的"操作反馈"

**当前（甘特图）**:
```
       2024-01-30        2024-01-31        2024-02-01
M1  [████████████]   [████████████]   [████████████]
M2  [█████████]      [██████████████] [████████]
M3  [████████]       [██████]         [███████████]
```

**改进（拖拽预览）**:
```
用户拖动 MAT-001 (25t) 从 M1/2024-01-31 到 M1/2024-01-30

实时预览：
       2024-01-30           2024-01-31
M1  [████████████████]   [████████]
    39→64 (超过50?) → 警告! ⚠️

建议位置：2024-02-02 [████████████] 产能 40/40 (最优) ✓

确认后：
  ├─ API 调用 planApi.moveItems()
  ├─ 后端排程重算
  ├─ ActionLog 记录："将 MAT-001 从 01-31 移到 02-02 (改善产能溢出)"
  └─ 工作台自动刷新，风险卡片绿化
```

---

### 1.4 核心交互流程模式

#### 模式 1：问题驱动的诊断流

```
用户场景：看到某日期产能溢出

步骤：
1. 风险概览点击 → 获得上下文（机组、日期、原因）
2. 工作台自动跳转 + 高亮问题区域
3. 物料池显示该日期的所有物料 + 可操作性标记
4. 用户勾选可移动物料 → 产能概览实时预测
5. 确认最优方案 → 执行操作
6. 工作台刷新，返回风险概览验证改进
```

#### 模式 2：关键日期的放大镜

```
用户场景：聚焦某个特定日期（如交期、危险日期）

设计：
1. 点击产能概览某日期 → 放大显示该日期的所有物料和产能
2. 显示该日期的：
   ├─ 所有排程物料（按紧急度排序）
   ├─ 产能占用情况
   ├─ 相关的轧辊换辊
   ├─ 后续日期的容量预测
   └─ 可操作的建议（移动、调整、强制放行等）
3. 允许拖拽调整排程顺序（预览产能变化）
4. 批量操作确认后执行
```

#### 模式 3：物料的完整生命周期视图

```
用户场景：追踪某个物料的整个排程过程

设计：
1. 点击物料 → 打开"物料追踪面板"
2. 显示该物料的：
   ├─ 当前状态（已排/冷料/锁定等）
   ├─ 当前排程日期和序号
   ├─ 历史操作记录（ActionLog）
   ├─ 对应的产能占用
   ├─ 相关的风险（轧辊更换/交期风险等）
   └─ 建议操作（移动/强制放行等）
3. 在这个面板内可以：
   ├─ 直接执行操作（移动/锁定等）
   ├─ 查看操作的影响预测
   ├─ 比较不同操作方案
   └─ 追踪后续影响
```

---

## 第二部分：材料状态可视化设计

### 2.1 问题根源：状态信息分散

**当前问题**：
```
用户看到物料列表时，需要同时理解：
1. material_state (7个值)
2. lock_flag
3. manual_urgent_flag
4. is_frozen
5. is_mature
6. temp_issue
7. plan_item 中的 locked_in_plan
8. plan_item 中的 force_release_in_plan
9. 与当前版本的配置对应关系
10. 与后续日期的产能关系

信息太散，用户难以形成"这个物料现在能干什么"的直观判断。
```

### 2.2 改进方案：分层状态标签系统

#### 第一层：主状态（可操作性）

用户一眼看到这个物料**现在能不能动**：

```
┌─ 🟢 就绪可动 (READY_TO_OPERATE)
│  └─ material_state = READY
│     lock_flag = false
│     not in plan
│     → 用户可以：移动、排程、强制放行

├─ 🟡 已排需谨慎 (SCHEDULED_CAUTION)
│  └─ material_state = SCHEDULED
│     locked_in_plan = false
│     → 用户可以：移动（可能影响风险）、解锁强制放行
│     → 警告：移动会重排排程

├─ 🔴 已锁冻结 (LOCKED_FROZEN)
│  └─ lock_flag = true
│     OR is_frozen = true
│     → 用户：不可操作、需解冻或管理员干预

├─ 🟠 冷料需审批 (COLD_NEEDS_APPROVAL)
│  └─ is_mature = false
│     sched_state != FORCE_RELEASE
│     → 用户可以：申请强制放行（需管理员审批）
│     → 显示库龄和温度

└─ ⚪ 未知状态 (UNKNOWN)
   └─ 需要后端核对数据一致性
```

#### 第二层：风险标记（为什么这样）

为每个物料标记它**为什么处于这个状态**：

```
🟢 就绪可动
  ├─ 标签：新入库
  │   └─ 适温温且未排程
  │
  └─ 标签：待排序
      └─ 库龄 5d，建议优先排程

🟡 已排需谨慎
  ├─ 标签：超产能日期
  │   └─ 该日期已超产能 5t，移动需谨慎
  │
  ├─ 标签：L3 紧急排
  │   └─ 紧急订单，不建议移动
  │
  └─ 标签：轧辊换辊前
      └─ 在轧辊更换前，可能影响质量

🔴 已锁冻结
  ├─ 标签：手动锁定
  │   └─ 操作人: 张三 (2024-01-30 14:23)
  │       原因：等待客户确认
  │
  └─ 标签：冻结区保护
      └─ Red Line 5: 不可调整

🟠 冷料需审批
  ├─ 标签：库龄 18d
  │   └─ 需强制放行，库龄超过 14d
  │
  └─ 标签：温度异常
      └─ 当前温度 180°C，低于目标 200°C
```

#### 第三层：可操作建议（现在能干什么）

针对每个物料的当前状态，给出**最相关的操作建议**：

```
🟢 就绪可动
  建议操作：
  ├─ 【排程到...】选择日期和机组
  ├─ 【快速排序】使用 AI 推荐位置
  ├─ 【设为紧急】升级紧急度 (L0 → L1/L2/L3)
  └─ 【锁定】防止后续自动调整

🟡 已排需谨慎
  建议操作：
  ├─ 【移动到...】确认后重排（预览风险）
  ├─ 【强制锁定】避免优化重排时移动
  ├─ 【撤销放行】如果是强制放行，可撤销
  └─ 【查看影响】分析移动对后续日期的影响

🔴 已锁冻结
  建议操作：
  ├─ 【解锁】(需权限) - 解除手动锁定
  ├─ 【查看锁定原因】 - 显示历史记录
  ├─ 【申请解冻】- 如果是冻结区，申请审核
  └─ 【管理员覆盖】(仅管理员) - 强制调整

🟠 冷料需审批
  建议操作：
  ├─ 【申请强制放行】- 需管理员审批
  ├─ 【加热】- 如果有加热设施 (可选)
  ├─ 【等待成熟】- 估算成熟时间
  └─ 【通知客户】- 延期通知
```

### 2.3 UI 设计：分层显示

#### 物料行的详细设计

```
┌─ 【状态徽章】【物料号】     【机组】【重量】【紧急度】    【库龄】
│
│  🟢 就绪可动      MAT-001      M1    25t    L2        5d
│
│  ├─ 细节行 1 (第二层 - 风险标记):
│  │  标签：[超产能日期] [L3紧急排] [轧辊换辊前]
│  │
│  ├─ 细节行 2 (第三层 - 可操作建议):
│  │  操作: [排程到...] [设为紧急] [锁定]
│  │  预览: 若排程到 2024-02-01 → 产能 39/40 ✓
│  │
│  └─ 收起/展开 按钮

│
│  🔴 已锁冻结      MAT-002      M3    15t    L0        12d
│
│  ├─ 细节行:
│  │  原因: 手动锁定 (张三, 2024-01-30)
│  │  理由: 等待客户确认
│  │
│  └─ 操作: [解锁] [查看记录] [申请异常处理]

│
│  🟠 冷料需审批    MAT-003      M2    8t     L3        18d
│
│  ├─ 细节行:
│  │  温度: 180°C (目标 200°C) | 库龄: 18d | 优先级: 高
│  │  风险: 客户最晚交期还有 3 天
│  │
│  └─ 操作: [申请强制放行] [加热] [通知客户]
│
└─ 底部操作栏 (高度 48px):
   显示该行当前的核心操作，以及操作后的影响预测
```

### 2.4 材料状态的规范化定义

#### 新增"操作状态"概念

```typescript
/**
 * 操作状态：描述当前物料"能做什么"，而不仅仅是"处于什么状态"
 */
export type OperabilityStatus =
  | 'READY_TO_OPERATE'        // 🟢 就绪可动
  | 'SCHEDULED_CAUTION'        // 🟡 已排需谨慎
  | 'LOCKED_FROZEN'            // 🔴 已锁冻结
  | 'COLD_NEEDS_APPROVAL'      // 🟠 冷料需审批
  | 'UNKNOWN';

/**
 * 计算材料的操作状态
 */
export function computeOperabilityStatus(material: Material): OperabilityStatus {
  // 优先级：冻结 > 锁定 > 冷料 > 排程 > 就绪

  if (material.is_frozen) {
    return 'LOCKED_FROZEN';
  }

  if (material.lock_flag) {
    return 'LOCKED_FROZEN';
  }

  if (!material.is_mature && material.sched_state !== 'FORCE_RELEASE') {
    return 'COLD_NEEDS_APPROVAL';
  }

  if (material.sched_state === 'SCHEDULED') {
    return 'SCHEDULED_CAUTION';
  }

  if (material.sched_state === 'READY') {
    return 'READY_TO_OPERATE';
  }

  return 'UNKNOWN';
}

/**
 * 计算材料的风险标记
 */
export function computeRiskBadges(
  material: Material,
  context: WorkbenchContext  // 当前日期、机组、产能数据
): RiskBadge[] {
  const badges: RiskBadge[] = [];

  // 检测超产能日期
  if (context.capacityOverflow > 0) {
    badges.push({
      type: 'CAPACITY_OVERFLOW',
      label: `超产能日期 (+${context.capacityOverflow}t)`,
      severity: 'HIGH'
    });
  }

  // 检测L3紧急订单
  if (material.urgent_level === 'L3') {
    badges.push({
      type: 'L3_URGENT',
      label: 'L3 紧急排',
      severity: 'CRITICAL'
    });
  }

  // 检测轧辊换辊风险
  if (context.isNearRollChange) {
    badges.push({
      type: 'ROLL_CHANGE_RISK',
      label: '轧辊换辊前',
      severity: 'MEDIUM'
    });
  }

  // ... 其他风险

  return badges;
}

/**
 * 生成可操作建议
 */
export function suggestOperations(
  material: Material,
  operability: OperabilityStatus
): Operation[] {
  switch (operability) {
    case 'READY_TO_OPERATE':
      return [
        {
          type: 'SCHEDULE_TO',
          label: '排程到...',
          icon: 'calendar',
          priority: 'primary'
        },
        {
          type: 'SET_URGENT',
          label: '设为紧急',
          icon: 'alert',
          priority: 'default'
        },
        {
          type: 'LOCK',
          label: '锁定',
          icon: 'lock',
          priority: 'default'
        }
      ];

    case 'SCHEDULED_CAUTION':
      return [
        {
          type: 'MOVE_TO',
          label: '移动到...',
          icon: 'arrow-right',
          priority: 'primary',
          warning: '会重新排程'
        },
        {
          type: 'LOCK_IN_PLAN',
          label: '锁定在排程',
          icon: 'lock',
          priority: 'default'
        }
      ];

    // ... 其他情况
  }
}
```

---

## 第三部分：风险概览与工作台的整合设计

### 3.1 当前问题

```
风险概览和工作台缺乏**因果关系**：

风险概览显示：
  "M3 2024-02-01 产能溢出 5t"

工作台显示：
  "M3 所有物料的完整排程"

缺失的是：
  "这 5t 的溢出具体是哪些物料造成的？"
  "哪些物料可以移动来解决这个溢出？"
```

### 3.2 改进设计：建立"风险 → 原因物料 → 可操作物料"的追踪链

#### A. 风险卡片的改进

**当前**:
```
危险日期：2024-02-01
产能溢出：5t
L3积压：2件
【跳转到工作台】
```

**改进**:
```
┌─ 危险日期：2024-02-01 (M3) ──────────────────┐
│                                                  │
│ 产能溢出：5t  (41/40t)                         │
│ L3积压：2件                                     │
│                                                  │
│ 溢出物料分析：                                  │
│ ├─ MAT-001 (25t L2) 可移动 ✓ → 移到 02-02    │
│ ├─ MAT-003 (8t L3) 可强制放行 ✓ → 前移到 02-01 前  │
│ └─ MAT-005 (13t L0) 已锁 ✗ → 无法调整        │
│                                                  │
│ 推荐操作：【诊断并修复】 [打开工作台]         │
│                                                  │
│ 修复方案：                                      │
│ 方案 A: 移动 MAT-001 到 02-02                  │
│   └─ 预期结果：M3 产能 31/40 ✓                │
│ 方案 B: 强制放行 MAT-003 到 02-01 前          │
│   └─ 预期结果：M3 产能 33/40 ✓                │
│                                                  │
│ 【一键修复方案A】 【手动调整】              │
└──────────────────────────────────────────────┘
```

#### B. 工作台的"问题诊断面板"

**新增组件**：`DiagnosisPanelContent`

```typescript
interface DiagnosisPanelProps {
  context: RiskContext;  // 来自风险概览的上下文
  workbenchState: WorkbenchState;  // 工作台当前状态
}

// 显示内容：
const DiagnosisPanelContent = ({ context, workbenchState }: DiagnosisPanelProps) => {
  return (
    <Segmented defaultValue="overview" options={[
      { label: '问题概览', value: 'overview' },
      { label: '根因分析', value: 'rootcause' },
      { label: '修复方案', value: 'solutions' },
      { label: '操作记录', value: 'history' },
    ]}>

      {/* Tab 1: 问题概览 */}
      <div>
        <Alert
          type="error"
          message={`${context.problemType} - ${context.location}`}
          description={`严重程度: ${context.severity} | 影响范围: ${context.impactScope}`}
          showIcon
        />

        <Timeline>
          <Timeline.Item>
            问题检测时间：{context.detectedAt}
          </Timeline.Item>
          <Timeline.Item>
            根本原因：{context.rootCause.description}
          </Timeline.Item>
          <Timeline.Item>
            关键物料：
            <List>
              {context.rootCause.affectedMaterials.map(mat => (
                <List.Item
                  key={mat.id}
                  onClick={() => setSelectedMaterialIds([mat.id])}
                  style={{ cursor: 'pointer' }}
                >
                  {mat.id} ({mat.weight}t {mat.urgency}) - {mat.operability}
                </List.Item>
              ))}
            </List>
          </Timeline.Item>
        </Timeline>
      </div>

      {/* Tab 2: 根因分析 */}
      <div>
        {context.rootCause.contributingFactors.map(factor => (
          <Card>
            <Row>
              <Col span={2}>{getRiskIcon(factor.type)}</Col>
              <Col span={22}>
                <Title level={5}>{factor.title}</Title>
                <Text>{factor.description}</Text>
                <br/>
                <Text type="secondary">
                  影响：{factor.impact}
                </Text>
              </Col>
            </Row>
          </Card>
        ))}
      </div>

      {/* Tab 3: 修复方案 */}
      <div>
        {context.suggestedSolutions.map((solution, idx) => (
          <Card title={`方案 ${String.fromCharCode(65 + idx)}`}>
            <Description
              items={[
                { label: '操作', value: solution.operations.join(' → ') },
                { label: '影响', value: solution.expectedImpact },
                { label: '风险', value: solution.residualRisk },
                { label: '成功率', value: `${solution.successRate}%` },
              ]}
            />
            <Space>
              <Button
                type="primary"
                onClick={() => applySolution(solution)}
              >
                应用此方案
              </Button>
              <Button onClick={() => previewSolution(solution)}>
                预览影响
              </Button>
            </Space>
          </Card>
        ))}
      </div>

      {/* Tab 4: 操作记录 */}
      <div>
        <ActionLogTimeline
          logs={context.relatedOperations}
          onRevert={(log) => revertOperation(log)}
        />
      </div>
    </Segmented>
  );
};
```

#### C. "关闭问题"流程

当用户修复了风险问题后，工作台应该提供"返回风险概览并验证"的流程：

```typescript
// 操作完成后
const onOperationCompleted = async () => {
  // 1. 刷新工作台数据
  await refreshWorkbenchData();

  // 2. 显示成功提示
  message.success('操作已完成，风险指标已更新');

  // 3. 提供"返回风险概览"按钮
  Modal.success({
    title: '修复成功',
    content: '该问题的相关指标已更新，返回风险概览查看改进效果？',
    okText: '返回风险概览',
    cancelText: '继续调整',
    onOk: () => {
      navigate('/risk-dashboard');
    }
  });
};
```

---

## 第四部分：实施路线图

### 优先级 1（必做，立即实施）

1. **增强物料池的状态显示**
   - 添加"操作状态徽章"（🟢🟡🔴🟠）
   - 添加"风险标记"标签
   - 添加"可操作建议"操作按钮
   - 预期工时：2-3 小时

2. **改进产能概览的交互反馈**
   - 实时显示"选中物料的产能影响预测"
   - 支持鼠标悬停查看详情
   - 预期工时：1-2 小时

3. **完善风险概览的深链接**
   - 风险卡片跳转带上完整上下文
   - 工作台接收并应用上下文（自动筛选、高亮）
   - 预期工时：1-2 小时

### 优先级 2（重要，后续实施）

4. **设计和实施"问题诊断面板"**
   - 在工作台左侧显示诊断面板
   - 分 Tab 显示：问题概览、根因分析、修复方案、操作记录
   - 预期工时：3-4 小时

5. **实现物料的"影响分析"**
   - 点击物料时显示：该物料对产能、轧辊、交期的影响
   - 预期工时：2-3 小时

6. **建立"问题关闭"流程**
   - 操作完成后提示返回风险概览
   - 自动验证问题是否已解决
   - 预期工时：1-2 小时

### 优先级 3（优化，逐步迭代）

7. **"一键修复"建议**
   - AI 分析问题，提出最优修复方案
   - 用户确认后一键执行
   - 预期工时：4-6 小时

8. **排程视图的拖拽预览增强**
   - 拖拽时实时显示产能、风险等级、轧辊影响
   - 预期工时：2-3 小时

---

## 第五部分：设计总结

### 核心改进点

| 当前问题 | 改进方向 | 预期效果 |
|---------|---------|---------|
| 四个视图相对独立 | 建立风险驱动的业务流 | 用户可清晰追踪"问题→诊断→决策→执行→验证" |
| 材料状态信息散落 | 设计分层状态标签系统 | 用户一眼看出"这个物料现在能干什么" |
| 无法追踪问题原因 | 实现风险→根因物料→可操作物料的追踪链 | 用户可以直接看到解决问题的操作建议 |
| 缺乏决策支持 | 添加产能影响预测和方案对比 | 用户可以比较操作方案，选择最优解 |
| 无法验证修复效果 | 完善"问题关闭"流程 | 操作完成后自动返回验证，形成完整闭环 |

### 预期用户体验提升

```
当前流程（无方向）：
  看风险卡片 → 不知所措 → 随意调整 → 可能改善或恶化

改进流程（有方向）：
  看风险卡片 → 点击"诊断此问题"
  → 自动跳转工作台，高亮问题区域
  → 显示根因物料和可操作建议
  → 预览修复后的效果
  → 执行操作
  → 自动验证问题是否解决
  → 返回风险概览查看改进
```

---

## 下一步建议

1. **确认优先级** - 您认为哪些功能对当前工作最急需？
2. **细化设计** - 对某个具体模块（如诊断面板、状态标签）进行更详细的设计
3. **原型验证** - 如果可能，制作一个简单的交互原型验证设计方向
4. **实施计划** - 根据优先级制定详细的开发计划和时间表
