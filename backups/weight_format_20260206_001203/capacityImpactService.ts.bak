/**
 * 产能影响预测计算服务
 *
 * 负责计算选中物料对产能的影响，支持：
 * - 移除物料的产能降低预测
 * - 添加物料的产能提升预测
 * - 分段影响分析（按紧急度）
 */

import type { CapacityTimelineData } from '../types/capacity';
import type { MaterialPoolMaterial } from '../components/material-pool/types';

/**
 * 产能影响预测结果
 */
export interface CapacityImpactPrediction {
  // 基础信息
  originalCapacity: number;           // 原始实际产能
  affectedWeight: number;              // 选中物料的合计重量

  // 预测结果
  predictedCapacity: number;            // 预测后的产能
  capacityDelta: number;                // 产能变化值（负数=降低，正数=提升）
  utilizationChangePercent: number;     // 利用率变化百分点

  // 限制检查
  exceedsTargetBefore: boolean;         // 原始是否超目标
  exceedsTargetAfter: boolean;          // 预测是否超目标
  exceedsLimitBefore: boolean;          // 原始是否超限制
  exceedsLimitAfter: boolean;           // 预测是否超限制

  // 状态判断
  improves: boolean;                    // 预测是否改善（降低产能溢出或利用率）
  risk: 'LOW' | 'MEDIUM' | 'HIGH';     // 预测风险等级
  message: string;                      // 用户友好的提示信息

  // 详细的物料信息
  materialDetails: Array<{
    material_id: string;
    weight_t: number;
    urgent_level: string;
    status: string;  // "已排" | "待排" | "未成熟" 等
  }>;
}

/**
 * 计算移除选中物料后的产能影响
 *
 * 场景：用户在物料池选中了某些物料，想了解如果把它们移到其他日期，
 * 当前日期的产能会如何变化
 *
 * @param timeline 目标时间线数据
 * @param selectedMaterials 用户选中的物料列表（必须都在该timeline中）
 * @returns 产能影响预测结果
 */
export function predictRemovalImpact(
  timeline: CapacityTimelineData,
  selectedMaterials: MaterialPoolMaterial[]
): CapacityImpactPrediction {
  // Step 1: 计算选中物料的合计重量
  const affectedWeight = selectedMaterials.reduce((sum, m) => sum + Number(m.weight_t || 0), 0);

  // Step 2: 计算预测产能（简化模型：仅考虑重量，不考虑紧急度重分配）
  const predictedCapacity = Math.max(0, timeline.actualCapacity - affectedWeight);
  const capacityDelta = predictedCapacity - timeline.actualCapacity;  // 负数

  // Step 3: 计算利用率变化
  const originalUtilization = (timeline.actualCapacity / timeline.targetCapacity) * 100;
  const predictedUtilization = (predictedCapacity / timeline.targetCapacity) * 100;
  const utilizationChangePercent = predictedUtilization - originalUtilization;

  // Step 4: 检查是否超限
  const exceedsTargetBefore = timeline.actualCapacity > timeline.targetCapacity;
  const exceedsTargetAfter = predictedCapacity > timeline.targetCapacity;
  const exceedsLimitBefore = timeline.actualCapacity > timeline.limitCapacity;
  const exceedsLimitAfter = predictedCapacity > timeline.limitCapacity;

  // Step 5: 判断是否改善
  const improves = exceedsTargetBefore && !exceedsTargetAfter ||
                   exceedsLimitBefore && !exceedsLimitAfter ||
                   utilizationChangePercent < -5;  // 至少改善5个百分点

  // Step 6: 评估风险
  let risk: 'LOW' | 'MEDIUM' | 'HIGH' = 'LOW';
  if (exceedsLimitAfter) {
    risk = 'HIGH';
  } else if (exceedsTargetAfter || affectedWeight === 0) {
    risk = 'MEDIUM';
  }

  // Step 7: 生成提示信息
  let message = '';
  if (affectedWeight === 0) {
    message = '未选中任何物料';
  } else if (improves && !exceedsTargetAfter) {
    message = `✓ 移除后产能将改善至 ${predictedCapacity.toFixed(2)}t（目标:${timeline.targetCapacity.toFixed(2)}t）`;
  } else if (improves) {
    message = `✓ 产能利用率降低 ${Math.abs(utilizationChangePercent).toFixed(1)}% 到 ${predictedUtilization.toFixed(1)}%`;
  } else if (exceedsLimitAfter) {
    message = `✗ 移除后仍超限制产能（${predictedCapacity.toFixed(2)}/${timeline.limitCapacity.toFixed(2)}t）`;
  } else if (exceedsTargetAfter) {
    message = `⚠️  移除后仍超目标产能（${predictedCapacity.toFixed(2)}/${timeline.targetCapacity.toFixed(2)}t）`;
  } else {
    message = `产能将变化 ${capacityDelta.toFixed(2)}t`;
  }

  // Step 8: 收集物料详情
  const materialDetails = selectedMaterials.map(m => ({
    material_id: m.material_id,
    weight_t: Number(m.weight_t || 0),
    urgent_level: String(m.urgent_level || 'L0'),
    status: getStatus(m),
  }));

  return {
    originalCapacity: timeline.actualCapacity,
    affectedWeight,
    predictedCapacity,
    capacityDelta,
    utilizationChangePercent,
    exceedsTargetBefore,
    exceedsTargetAfter,
    exceedsLimitBefore,
    exceedsLimitAfter,
    improves,
    risk,
    message,
    materialDetails,
  };
}

/**
 * 计算添加物料到某个时间线后的产能影响
 *
 * @param timeline 目标时间线
 * @param material 要添加的物料
 * @returns 产能影响预测结果
 */
export function predictAdditionImpact(
  timeline: CapacityTimelineData,
  material: MaterialPoolMaterial
): CapacityImpactPrediction {
  const weight = Number(material.weight_t || 0);
  const predictedCapacity = timeline.actualCapacity + weight;
  const capacityDelta = weight;

  const originalUtilization = (timeline.actualCapacity / timeline.targetCapacity) * 100;
  const predictedUtilization = (predictedCapacity / timeline.targetCapacity) * 100;
  const utilizationChangePercent = predictedUtilization - originalUtilization;

  const exceedsTargetBefore = timeline.actualCapacity > timeline.targetCapacity;
  const exceedsTargetAfter = predictedCapacity > timeline.targetCapacity;
  const exceedsLimitBefore = timeline.actualCapacity > timeline.limitCapacity;
  const exceedsLimitAfter = predictedCapacity > timeline.limitCapacity;

  const worsens = (!exceedsTargetBefore && exceedsTargetAfter) ||
                  (!exceedsLimitBefore && exceedsLimitAfter);

  let risk: 'LOW' | 'MEDIUM' | 'HIGH' = 'LOW';
  if (exceedsLimitAfter) {
    risk = 'HIGH';
  } else if (exceedsTargetAfter) {
    risk = 'MEDIUM';
  }

  let message = '';
  if (exceedsLimitAfter) {
    message = `✗ 添加后将超限制产能（${predictedCapacity.toFixed(2)}/${timeline.limitCapacity.toFixed(2)}t）`;
  } else if (exceedsTargetAfter) {
    message = `⚠️  添加后将超目标产能（${predictedCapacity.toFixed(2)}/${timeline.targetCapacity.toFixed(2)}t）`;
  } else if (predictedUtilization > 90) {
    message = `⚠️  产能利用率较高 ${predictedUtilization.toFixed(1)}%，建议谨慎`;
  } else {
    message = `✓ 可安全添加，产能仍在目标内`;
  }

  return {
    originalCapacity: timeline.actualCapacity,
    affectedWeight: weight,
    predictedCapacity,
    capacityDelta,
    utilizationChangePercent,
    exceedsTargetBefore,
    exceedsTargetAfter,
    exceedsLimitBefore,
    exceedsLimitAfter,
    improves: !worsens,
    risk,
    message,
    materialDetails: [{
      material_id: material.material_id,
      weight_t: weight,
      urgent_level: String(material.urgent_level || 'L0'),
      status: getStatus(material),
    }],
  };
}

/**
 * 获取物料的状态标签
 */
function getStatus(material: MaterialPoolMaterial): string {
  if (material.is_mature === false) {
    return '未成熟';
  }

  const state = String(material.sched_state || '').toUpperCase();
  if (state.includes('SCHEDULED')) {
    return '已排产';
  }
  if (state.includes('READY')) {
    return '待排产';
  }
  if (state.includes('LOCKED')) {
    return '已锁定';
  }

  return '其他';
}

/**
 * 获取影响严重程度的颜色
 */
export function getImpactRiskColor(risk: 'LOW' | 'MEDIUM' | 'HIGH'): string {
  switch (risk) {
    case 'HIGH':
      return '#ff4d4f';
    case 'MEDIUM':
      return '#faad14';
    case 'LOW':
    default:
      return '#52c41a';
  }
}
