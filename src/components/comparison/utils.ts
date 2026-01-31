/**
 * 版本对比的共享辅助函数
 * 包含数据转换、规范化、计算逻辑
 */

import type { PlanItemSnapshot, VersionDiff } from '../../types/comparison';
import { LocalVersionDiffSummary, RETROSPECTIVE_NOTE_KEY_PREFIX, Version } from './types';

/**
 * 规范化日期为仅包含日期部分 (YYYY-MM-DD)
 */
export function normalizeDateOnly(date: string): string {
  const trimmed = String(date || '').trim();
  if (!trimmed) return '';
  if (/^\d{4}-\d{2}-\d{2}/.test(trimmed)) return trimmed.slice(0, 10);
  return trimmed;
}

/**
 * 从版本的配置快照中提取中文名称
 */
export function extractVersionNameCn(version: any): string | null {
  const raw = version?.config_snapshot_json;
  if (raw == null) return null;
  const text = String(raw || '').trim();
  if (!text) return null;
  try {
    const obj = JSON.parse(text);
    const v = obj?.__meta_version_name_cn;
    if (typeof v === 'string' && v.trim()) return v.trim();
    return null;
  } catch {
    return null;
  }
}

/**
 * 格式化版本标签（优先显示中文名称）
 */
export function formatVersionLabel(version: Version): string {
  const nameCn = extractVersionNameCn(version);
  if (nameCn) return nameCn;
  const no = Number(version.version_no ?? 0);
  if (Number.isFinite(no) && no > 0) return `V${no}`;
  // 降级显示：UUID前8位
  return version.version_id.substring(0, 8);
}

/**
 * 格式化版本标签（带完整信息：中文名称 + 版本号）
 *
 * @example
 * - 有中文名称和版本号: "重排产优化 (V10)"
 * - 只有中文名称: "测试方案A"
 * - 只有版本号: "V12"
 * - 都没有: "31c46b4d"（UUID前8位）
 */
export function formatVersionLabelWithCode(version: Version): string {
  const nameCn = extractVersionNameCn(version);
  const no = Number(version.version_no ?? 0);

  if (nameCn && Number.isFinite(no) && no > 0) {
    return `${nameCn} (V${no})`;
  } else if (nameCn) {
    return nameCn;
  } else if (Number.isFinite(no) && no > 0) {
    return `V${no}`;
  } else {
    // 降级显示：UUID前8位
    return version.version_id.substring(0, 8);
  }
}

/**
 * 规范化计划项数据
 */
export function normalizePlanItem(raw: any): PlanItemSnapshot | null {
  const id = String(raw?.material_id ?? '').trim();
  if (!id) return null;
  return {
    material_id: id,
    machine_code: String(raw?.machine_code ?? ''),
    plan_date: normalizeDateOnly(String(raw?.plan_date ?? '')),
    seq_no: Number(raw?.seq_no ?? 0),
    weight_t: raw?.weight_t == null ? undefined : Number(raw.weight_t),
    urgent_level: raw?.urgent_level == null ? undefined : String(raw.urgent_level),
    locked_in_plan: raw?.locked_in_plan == null ? undefined : !!raw.locked_in_plan,
    force_release_in_plan: raw?.force_release_in_plan == null ? undefined : !!raw.force_release_in_plan,
    sched_state: raw?.sched_state == null ? undefined : String(raw.sched_state),
    assign_reason: raw?.assign_reason == null ? undefined : String(raw.assign_reason),
  };
}

/**
 * 计算两个版本之间的物料差异
 */
export function computeVersionDiffs(
  itemsA: PlanItemSnapshot[],
  itemsB: PlanItemSnapshot[]
): { diffs: VersionDiff[]; summary: LocalVersionDiffSummary } {
  const mapA = new Map<string, PlanItemSnapshot>();
  const mapB = new Map<string, PlanItemSnapshot>();
  itemsA.forEach((it) => mapA.set(it.material_id, it));
  itemsB.forEach((it) => mapB.set(it.material_id, it));

  const allIds = new Set<string>([...mapA.keys(), ...mapB.keys()]);
  const diffs: VersionDiff[] = [];

  const isDifferent = (a: unknown, b: unknown) => {
    if (a === b) return false;
    if (a == null && b == null) return false;
    return String(a ?? '') !== String(b ?? '');
  };

  const isWeightDifferent = (a: number | undefined, b: number | undefined) => {
    const na = a == null || !Number.isFinite(a) ? null : Number(a);
    const nb = b == null || !Number.isFinite(b) ? null : Number(b);
    if (na == null && nb == null) return false;
    if (na == null || nb == null) return true;
    return Math.abs(na - nb) > 1e-6;
  };

  Array.from(allIds)
    .sort()
    .forEach((id) => {
      const a = mapA.get(id) ?? null;
      const b = mapB.get(id) ?? null;

      if (!a && b) {
        diffs.push({
          materialId: id,
          changeType: 'ADDED',
          previousState: null,
          currentState: b,
        });
        return;
      }

      if (a && !b) {
        diffs.push({
          materialId: id,
          changeType: 'REMOVED',
          previousState: a,
          currentState: null,
        });
        return;
      }

      if (!a || !b) return;

      const moved =
        isDifferent(a.machine_code, b.machine_code) ||
        isDifferent(a.plan_date, b.plan_date) ||
        Number(a.seq_no ?? 0) !== Number(b.seq_no ?? 0);

      const modified =
        !moved &&
        (isWeightDifferent(a.weight_t, b.weight_t) ||
          isDifferent(a.urgent_level, b.urgent_level) ||
          isDifferent(a.locked_in_plan, b.locked_in_plan) ||
          isDifferent(a.force_release_in_plan, b.force_release_in_plan) ||
          isDifferent(a.sched_state, b.sched_state) ||
          isDifferent(a.assign_reason, b.assign_reason));

      if (!moved && !modified) return;

      diffs.push({
        materialId: id,
        changeType: moved ? 'MOVED' : 'MODIFIED',
        previousState: a,
        currentState: b,
      });
    });

  const summary: LocalVersionDiffSummary = diffs.reduce(
    (acc, d) => {
      acc.totalChanges += 1;
      if (d.changeType === 'ADDED') acc.addedCount += 1;
      else if (d.changeType === 'REMOVED') acc.removedCount += 1;
      else if (d.changeType === 'MOVED') acc.movedCount += 1;
      else acc.modifiedCount += 1;
      return acc;
    },
    { totalChanges: 0, addedCount: 0, removedCount: 0, modifiedCount: 0, movedCount: 0 }
  );

  return { diffs, summary };
}

/**
 * 计算容量使用量映射（机组+日期 -> 总重量）
 */
export function computeCapacityMap(items: PlanItemSnapshot[]): Map<string, number> {
  const map = new Map<string, number>();
  items.forEach((it) => {
    const machine = String(it.machine_code ?? '').trim();
    const date = normalizeDateOnly(String(it.plan_date ?? ''));
    if (!machine || !date) return;
    const weight = Number(it.weight_t ?? 0);
    if (!Number.isFinite(weight) || weight <= 0) return;
    const key = `${machine}__${date}`;
    map.set(key, (map.get(key) ?? 0) + weight);
  });
  return map;
}

/**
 * 计算每日总产量
 */
export function computeDailyTotals(items: PlanItemSnapshot[]): Map<string, number> {
  const map = new Map<string, number>();
  items.forEach((it) => {
    const date = normalizeDateOnly(String(it.plan_date ?? ''));
    if (!date) return;
    const weight = Number(it.weight_t ?? 0);
    if (!Number.isFinite(weight) || weight <= 0) return;
    map.set(date, (map.get(date) ?? 0) + weight);
  });
  return map;
}

/**
 * 生成回顾性笔记的存储键
 */
export function makeRetrospectiveKey(versionIdA: string, versionIdB: string): string {
  const [a, b] = [String(versionIdA || ''), String(versionIdB || '')].sort();
  return `${RETROSPECTIVE_NOTE_KEY_PREFIX}__${a}__${b}`;
}
