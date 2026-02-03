/**
 * 策略草案格式化工具
 * 从 StrategyDraftComparison.tsx 提取
 */

import type { Dayjs } from 'dayjs';
import dayjs from 'dayjs';
import type { MaterialDetailPayload, SqueezedHintSection, StrategyDraftSummary } from '../types/strategy-draft';
import { MAX_DAYS } from '../types/strategy-draft';

/**
 * 限制日期范围不超过最大天数
 */
export function clampRange(range: [Dayjs, Dayjs]): [Dayjs, Dayjs] {
  let [start, end] = range;
  start = start.startOf('day');
  end = end.startOf('day');
  if (end.isBefore(start)) {
    const tmp = start;
    start = end;
    end = tmp;
  }
  const days = end.diff(start, 'day') + 1;
  if (days > MAX_DAYS) {
    end = start.add(MAX_DAYS - 1, 'day');
  }
  return [start, end];
}

/**
 * 格式化吨数
 */
export function formatTon(v: unknown): string {
  const n = Number(v);
  if (!Number.isFinite(n)) return '—';
  return n.toFixed(1);
}

/**
 * 格式化百分比
 */
export function formatPercent(v: unknown): string {
  const n = Number(v);
  if (!Number.isFinite(n)) return '—';
  return `${(n * 100).toFixed(1)}%`;
}

/**
 * 计算成熟率
 */
export function getMaturityRate(d: StrategyDraftSummary): number {
  const denom = (d?.mature_count ?? 0) + (d?.immature_count ?? 0);
  if (!denom) return 0;
  return (d.mature_count ?? 0) / denom;
}

/**
 * 格式化位置信息（机组/日期/序号）
 */
export function formatPosition(date?: string | null, machine?: string | null, seq?: number | null): string {
  const d = String(date ?? '').trim();
  const m = String(machine ?? '').trim();
  const s = seq == null ? '' : String(seq);
  if (!d && !m && !s) return '—';
  return `${m || '-'} / ${d || '-'} / #${s || '-'}`;
}

/**
 * 格式化布尔值
 */
export function formatBool(v: unknown): string {
  if (v == null) return '—';
  return v ? '是' : '否';
}

/**
 * 格式化文本（空值处理）
 */
export function formatText(v: unknown): string {
  if (v == null) return '—';
  const s = String(v).trim();
  return s ? s : '—';
}

/**
 * 格式化数字
 */
export function formatNumber(v: unknown, digits = 2): string {
  const n = Number(v);
  if (!Number.isFinite(n)) return '—';
  return n.toFixed(digits);
}

/**
 * 判断两个数字是否近似相等
 */
export function isSameNumber(a: number, b: number): boolean {
  return Math.abs(a - b) < 1e-6;
}

/**
 * 规范化物料详情响应
 */
export function normalizeMaterialDetail(raw: unknown): MaterialDetailPayload | null {
  if (!raw) return null;
  if (Array.isArray(raw) && raw.length >= 2) {
    return { master: raw[0], state: raw[1] };
  }
  if (typeof raw === 'object' && raw !== null) {
    const obj = raw as Record<string, unknown>;
    const maybeMaster = obj?.master ?? obj?.material_master ?? (Array.isArray(raw) ? raw[0] : undefined);
    const maybeState = obj?.state ?? obj?.material_state ?? (Array.isArray(raw) ? raw[1] : undefined);
    if (maybeMaster && maybeState) return { master: maybeMaster, state: maybeState };
  }
  return null;
}

/**
 * 格式化原因文本（支持 JSON 美化）
 */
export function prettyReason(value: unknown): string | null {
  const raw = value == null ? '' : String(value).trim();
  if (!raw) return null;
  if ((raw.startsWith('{') && raw.endsWith('}')) || (raw.startsWith('[') && raw.endsWith(']'))) {
    try {
      return JSON.stringify(JSON.parse(raw), null, 2);
    } catch {
      return raw;
    }
  }
  return raw;
}

/**
 * 辅助函数：将布尔值转换
 */
function asBool(v: unknown): boolean {
  return v === true || v === 1 || v === '1' || String(v).toLowerCase() === 'true';
}

/**
 * 构建挤出物料的提示信息
 */
export function buildSqueezedOutHintSections(
  state: Record<string, unknown> | null,
  windowStartDate: string,
  windowEndDate: string
): SqueezedHintSection[] {
  const sections: SqueezedHintSection[] = [];

  const notes: string[] = [
    `比较窗口：${windowStartDate} ~ ${windowEndDate}`,
    '挤出=草案窗口内未排入（若被排到窗口外，也会显示为挤出）',
    '说明：以下提示仅基于 material_state 字段（未额外查库）',
  ];

  sections.push({ title: '含义/范围', lines: notes });

  if (!state) {
    return sections;
  }

  const key: string[] = [];
  const readyInDays = Number(state?.ready_in_days);
  if (Number.isFinite(readyInDays)) {
    if (readyInDays > 0) {
      const readyDate = dayjs().add(readyInDays, 'day').format('YYYY-MM-DD');
      if (dayjs(readyDate).isValid() && dayjs(windowEndDate).isValid() && dayjs(readyDate).isAfter(dayjs(windowEndDate), 'day')) {
        key.push(`窗口内不可适温：预计适温日 ${readyDate} > 窗口结束 ${windowEndDate}`);
      } else {
        key.push(`距适温：${readyInDays} 天（预计适温日：${readyDate}）`);
      }
    } else {
      key.push('已适温（ready_in_days <= 0）');
    }
  } else if (state?.is_mature != null) {
    key.push(asBool(state?.is_mature) ? '已适温（is_mature=1）' : '未适温（is_mature=0）');
  }

  const earliest = String(state?.earliest_sched_date ?? '').trim();
  if (earliest) {
    const end = dayjs(windowEndDate);
    const d = dayjs(earliest);
    if (d.isValid() && end.isValid() && d.isAfter(end, 'day')) {
      key.push(`窗口内不可排：最早可排 ${earliest} > 窗口结束 ${windowEndDate}`);
    } else {
      key.push(`最早可排：${earliest}`);
    }
  }

  if (key.length) sections.push({ title: '关键判断（窗口相关）', lines: key });

  const constraints: string[] = [];
  if (state?.in_frozen_zone != null)
    constraints.push(`冻结区：${asBool(state?.in_frozen_zone) ? '是' : '否'}${asBool(state?.in_frozen_zone) ? '（红线：冻结区不可变更）' : ''}`);
  if (state?.lock_flag != null) constraints.push(`锁定：${asBool(state?.lock_flag) ? '是' : '否'}`);
  if (state?.force_release_flag != null) constraints.push(`强制放行：${asBool(state?.force_release_flag) ? '是' : '否'}`);
  if (constraints.length) sections.push({ title: '限制/标志', lines: constraints });

  const urgency: string[] = [];
  const urgent = String(state?.urgent_level ?? '').trim();
  if (urgent) urgency.push(`紧急等级：${urgent}`);
  if (state?.manual_urgent_flag != null) urgency.push(`人工紧急：${asBool(state?.manual_urgent_flag) ? '是' : '否'}`);
  const rush = String(state?.rush_level ?? '').trim();
  if (rush) urgency.push(`催料等级：${rush}`);
  if (urgency.length) sections.push({ title: '紧急/优先', lines: urgency });

  const cur: string[] = [];
  const schedState = String(state?.sched_state ?? '').trim();
  if (schedState) cur.push(`排产状态：${schedState}`);
  const scheduledDate = String(state?.scheduled_date ?? '').trim();
  const scheduledMachine = String(state?.scheduled_machine_code ?? '').trim();
  const seqNo = state?.seq_no;
  if (scheduledDate || scheduledMachine) {
    cur.push(`已排（material_state）：${scheduledMachine || '-'} / ${scheduledDate || '-'} / #${seqNo ?? '-'}`);
  } else {
    cur.push('已排（material_state）：未排');
  }
  if (cur.length) sections.push({ title: '当前状态（material_state）', lines: cur });

  return sections;
}
