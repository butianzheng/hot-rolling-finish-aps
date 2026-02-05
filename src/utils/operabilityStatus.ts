/**
 * 物料可操作性状态分析
 *
 * 这个模块负责计算物料的"可操作状态"，帮助用户理解：
 * - 这个物料当前能做什么操作
 * - 为什么处于这个状态
 * - 存在哪些风险
 */

import type { MaterialPoolMaterial } from '../components/material-pool/types';
import { normalizeSchedState } from './schedState';

/**
 * 操作状态：描述物料"能做什么"
 */
export type OperabilityStatus =
  | 'READY_TO_OPERATE'      // 🟢 就绪可动 - 适温未排程，可以自由操作
  | 'SCHEDULED_CAUTION'     // 🟡 已排需谨慎 - 已排程但未锁定，操作需预览影响
  | 'LOCKED_FROZEN'         // 🔴 已锁冻结 - 锁定或冻结，需解锁或管理员干预
  | 'COLD_NEEDS_APPROVAL'   // 🟠 冷料需审批 - 未适温，需申请强制放行
  | 'UNKNOWN';              // ⚪ 未知状态 - 需检查数据

/**
 * 风险级别
 */
export type RiskSeverity = 'LOW' | 'MEDIUM' | 'HIGH' | 'CRITICAL';

/**
 * 风险标记类型
 */
export type RiskBadgeType =
  | 'CAPACITY_OVERFLOW'     // 超产能日期
  | 'L3_URGENT'            // L3紧急订单
  | 'L2_URGENT'            // L2紧急订单
  | 'ROLL_CHANGE_RISK'     // 轧辊换辊风险
  | 'TEMP_ISSUE'           // 温度问题
  | 'AGE_WARNING'          // 库龄告警
  | 'LOCKED_MANUAL'        // 手动锁定
  | 'FROZEN_ZONE';         // 冻结区保护

/**
 * 风险标记
 */
export interface RiskBadge {
  type: RiskBadgeType;
  label: string;
  severity: RiskSeverity;
  tooltip?: string;
}

/**
 * 操作建议类型
 */
export type OperationType =
  | 'SCHEDULE_TO'         // 排程到...
  | 'MOVE_TO'            // 移动到...
  | 'SET_URGENT'         // 设为紧急
  | 'LOCK'               // 锁定
  | 'UNLOCK'             // 解锁
  | 'FORCE_RELEASE'      // 强制放行
  | 'CANCEL_FORCE'       // 撤销强制放行
  | 'VIEW_DETAILS';      // 查看详情

/**
 * 操作建议
 */
export interface Operation {
  type: OperationType;
  label: string;
  icon?: string;
  priority?: 'primary' | 'default' | 'danger';
  disabled?: boolean;
  warning?: string;
  tooltip?: string;
}

/**
 * 计算物料的操作状态
 *
 * 优先级顺序（从高到低）：
 * 1. 冻结区保护 → LOCKED_FROZEN
 * 2. 手动锁定 → LOCKED_FROZEN
 * 3. 冷料未适温 → COLD_NEEDS_APPROVAL
 * 4. 已排产 → SCHEDULED_CAUTION
 * 5. 就绪待排 → READY_TO_OPERATE
 */
export function computeOperabilityStatus(material: MaterialPoolMaterial): OperabilityStatus {
  // 1. 冻结区保护（最高优先级，工业红线）
  if (material.is_frozen) {
    return 'LOCKED_FROZEN';
  }

  // 2. 手动锁定
  if (material.lock_flag) {
    return 'LOCKED_FROZEN';
  }

  // 3. 冷料未适温（需特殊处理）
  const schedState = normalizeSchedState(material.sched_state);
  if (material.is_mature === false && schedState !== 'FORCE_RELEASE') {
    return 'COLD_NEEDS_APPROVAL';
  }

  // 4. 已排产（需谨慎操作）
  if (schedState === 'SCHEDULED') {
    return 'SCHEDULED_CAUTION';
  }

  // 5. 就绪可操作
  if (schedState === 'READY') {
    return 'READY_TO_OPERATE';
  }

  // 6. 其他状态
  if (schedState === 'BLOCKED') {
    return 'LOCKED_FROZEN'; // 阻断状态视为不可操作
  }

  return 'UNKNOWN';
}

/**
 * 计算物料的风险标记
 *
 * @param material 物料数据
 * @param context 可选的上下文信息（如产能数据）
 */
export function computeRiskBadges(
  material: MaterialPoolMaterial,
  context?: {
    capacityOverflow?: number;      // 产能溢出吨数
    isNearRollChange?: boolean;     // 是否接近轧辊更换
  }
): RiskBadge[] {
  const badges: RiskBadge[] = [];

  // 1. 紧急度标记
  const urgency = String(material.urgent_level || '').toUpperCase();
  if (urgency === 'L3') {
    badges.push({
      type: 'L3_URGENT',
      label: 'L3紧急',
      severity: 'CRITICAL',
      tooltip: '最高优先级订单，不建议调整',
    });
  } else if (urgency === 'L2') {
    badges.push({
      type: 'L2_URGENT',
      label: 'L2高优',
      severity: 'HIGH',
      tooltip: '高优先级订单，调整需谨慎',
    });
  }

  // 2. 冻结区保护
  if (material.is_frozen) {
    badges.push({
      type: 'FROZEN_ZONE',
      label: '冻结区',
      severity: 'CRITICAL',
      tooltip: '冻结区保护（工业红线），不可调整',
    });
  }

  // 3. 手动锁定
  if (material.lock_flag) {
    badges.push({
      type: 'LOCKED_MANUAL',
      label: '已锁定',
      severity: 'HIGH',
      tooltip: '手动锁定，需解锁后才能操作',
    });
  }

  // 4. 温度问题
  if (material.temp_issue || material.is_mature === false) {
    badges.push({
      type: 'TEMP_ISSUE',
      label: '冷料',
      severity: 'MEDIUM',
      tooltip: material.is_mature === false ? '未适温，需强制放行' : '存在温度问题',
    });
  }

  // 5. 产能溢出（需要上下文信息）
  if (context?.capacityOverflow && context.capacityOverflow > 0) {
    badges.push({
      type: 'CAPACITY_OVERFLOW',
      label: `超产能 +${context.capacityOverflow.toFixed(2)}t`,
      severity: 'HIGH',
      tooltip: '该日期产能已溢出，建议移动',
    });
  }

  // 6. 轧辊换辊风险（需要上下文信息）
  if (context?.isNearRollChange) {
    badges.push({
      type: 'ROLL_CHANGE_RISK',
      label: '近换辊期',
      severity: 'MEDIUM',
      tooltip: '接近轧辊更换阈值，可能影响质量',
    });
  }

  return badges;
}

/**
 * 生成操作建议
 *
 * 根据物料的操作状态，提供可执行的操作列表
 */
export function suggestOperations(
  material: MaterialPoolMaterial,
  operability: OperabilityStatus
): Operation[] {
  const schedState = normalizeSchedState(material.sched_state);

  switch (operability) {
    case 'READY_TO_OPERATE':
      return [
        {
          type: 'SCHEDULE_TO',
          label: '排程到...',
          icon: 'calendar',
          priority: 'primary',
          tooltip: '将物料排程到指定日期和机组',
        },
        {
          type: 'SET_URGENT',
          label: '设为紧急',
          icon: 'alert',
          priority: 'default',
          tooltip: '提升物料的紧急度等级',
        },
        {
          type: 'LOCK',
          label: '锁定',
          icon: 'lock',
          priority: 'default',
          tooltip: '锁定物料，防止自动调整',
        },
      ];

    case 'SCHEDULED_CAUTION':
      return [
        {
          type: 'MOVE_TO',
          label: '移动到...',
          icon: 'arrow-right',
          priority: 'primary',
          warning: '移动会触发排程重算',
          tooltip: '移动物料到其他日期或机组',
        },
        {
          type: 'LOCK',
          label: '锁定在排程',
          icon: 'lock',
          priority: 'default',
          tooltip: '锁定物料，防止优化时被移动',
        },
        {
          type: 'VIEW_DETAILS',
          label: '查看影响',
          icon: 'eye',
          priority: 'default',
          tooltip: '查看移动对产能的影响',
        },
      ];

    case 'LOCKED_FROZEN':
      const operations: Operation[] = [];

      if (material.lock_flag) {
        operations.push({
          type: 'UNLOCK',
          label: '解锁',
          icon: 'unlock',
          priority: 'primary',
          tooltip: '解除手动锁定',
        });
      }

      if (material.is_frozen) {
        operations.push({
          type: 'VIEW_DETAILS',
          label: '查看冻结原因',
          icon: 'eye',
          priority: 'default',
          tooltip: '查看为何物料在冻结区',
        });
      }

      if (schedState === 'BLOCKED') {
        operations.push({
          type: 'VIEW_DETAILS',
          label: '查看阻断原因',
          icon: 'eye',
          priority: 'default',
          tooltip: '查看物料被阻断的原因',
        });
      }

      return operations;

    case 'COLD_NEEDS_APPROVAL':
      return [
        {
          type: 'FORCE_RELEASE',
          label: '申请强制放行',
          icon: 'check-circle',
          priority: 'primary',
          warning: '需管理员审批',
          tooltip: '申请强制放行未适温物料',
        },
        {
          type: 'VIEW_DETAILS',
          label: '查看温度信息',
          icon: 'eye',
          priority: 'default',
          tooltip: '查看物料的温度和库龄信息',
        },
      ];

    case 'UNKNOWN':
    default:
      return [
        {
          type: 'VIEW_DETAILS',
          label: '查看详情',
          icon: 'eye',
          priority: 'default',
          tooltip: '查看物料的详细信息',
        },
      ];
  }
}

/**
 * 获取操作状态的显示配置
 */
export function getOperabilityConfig(status: OperabilityStatus): {
  color: string;
  emoji: string;
  label: string;
  description: string;
} {
  switch (status) {
    case 'READY_TO_OPERATE':
      return {
        color: '#52c41a',
        emoji: '🟢',
        label: '就绪可动',
        description: '适温未排程，可以自由操作',
      };

    case 'SCHEDULED_CAUTION':
      return {
        color: '#faad14',
        emoji: '🟡',
        label: '已排需谨慎',
        description: '已排程但未锁定，操作需预览影响',
      };

    case 'LOCKED_FROZEN':
      return {
        color: '#ff4d4f',
        emoji: '🔴',
        label: '已锁冻结',
        description: '锁定或冻结，需解锁或管理员干预',
      };

    case 'COLD_NEEDS_APPROVAL':
      return {
        color: '#fa8c16',
        emoji: '🟠',
        label: '冷料需审批',
        description: '未适温，需申请强制放行',
      };

    case 'UNKNOWN':
    default:
      return {
        color: '#d9d9d9',
        emoji: '⚪',
        label: '未知状态',
        description: '需检查数据一致性',
      };
  }
}

/**
 * 获取风险级别的颜色
 */
export function getRiskSeverityColor(severity: RiskSeverity): string {
  switch (severity) {
    case 'CRITICAL':
      return '#ff4d4f';
    case 'HIGH':
      return '#fa8c16';
    case 'MEDIUM':
      return '#faad14';
    case 'LOW':
    default:
      return '#1677ff';
  }
}
