/**
 * workbench/move/impact.ts 单元测试
 */

import dayjs from 'dayjs';
import { describe, it, expect } from 'vitest';
import { computeMoveImpactBase } from './impact';

describe('workbench/move/impact', () => {
  it('AUTO_FIX：跳过 locked_in_plan 项，影响仅按可移动项计算', () => {
    const res = computeMoveImpactBase({
      moveModalOpen: true,
      moveTargetMachine: 'M2',
      moveTargetDate: dayjs('2026-02-02'),
      moveValidationMode: 'AUTO_FIX',
      planItems: [
        { material_id: 'A', machine_code: 'M1', plan_date: '2026-02-01', weight_t: 10, locked_in_plan: true },
        { material_id: 'B', machine_code: 'M1', plan_date: '2026-02-01', weight_t: 20, locked_in_plan: false },
      ] as any,
      selectedMaterialIds: ['A', 'B'],
    });

    expect(res?.rows).toHaveLength(2);
    const byKey = new Map(res?.rows.map((r) => [`${r.machine_code}__${r.date}`, r] as const));
    expect(byKey.get('M1__2026-02-01')?.delta_t).toBe(-20);
    expect(byKey.get('M2__2026-02-02')?.delta_t).toBe(20);
  });

  it('STRICT：包含 locked_in_plan 项，影响按全量计算', () => {
    const res = computeMoveImpactBase({
      moveModalOpen: true,
      moveTargetMachine: 'M2',
      moveTargetDate: dayjs('2026-02-02'),
      moveValidationMode: 'STRICT',
      planItems: [
        { material_id: 'A', machine_code: 'M1', plan_date: '2026-02-01', weight_t: 10, locked_in_plan: true },
        { material_id: 'B', machine_code: 'M1', plan_date: '2026-02-01', weight_t: 20, locked_in_plan: false },
      ] as any,
      selectedMaterialIds: ['A', 'B'],
    });

    expect(res?.rows).toHaveLength(2);
    const byKey = new Map(res?.rows.map((r) => [`${r.machine_code}__${r.date}`, r] as const));
    expect(byKey.get('M1__2026-02-01')?.delta_t).toBe(-30);
    expect(byKey.get('M2__2026-02-02')?.delta_t).toBe(30);
  });

  it('目标机组/日期与原位置相同：净变化为 0 时 rows 为空', () => {
    const res = computeMoveImpactBase({
      moveModalOpen: true,
      moveTargetMachine: 'M1',
      moveTargetDate: dayjs('2026-02-01'),
      moveValidationMode: 'STRICT',
      planItems: [
        { material_id: 'A', machine_code: 'M1', plan_date: '2026-02-01', weight_t: 10, locked_in_plan: false },
      ] as any,
      selectedMaterialIds: ['A'],
    });

    expect(res?.rows).toEqual([]);
    expect(res?.affectedMachines).toEqual(['M1']);
    expect(res?.dateFrom).toBe('2026-02-01');
    expect(res?.dateTo).toBe('2026-02-01');
  });
});

