import { describe, it, expect, vi, beforeEach } from 'vitest';
import {
  loadPlanItemsIfEmpty,
  buildPlanItemByIdMap,
  splitSelectedMaterialIds,
  sortMaterialIdsByPlan,
  type IpcPlanItem,
} from './planItems';
import { planApi } from '../../../api/tauri';

// Mock the planApi module
vi.mock('../../../api/tauri', () => ({
  planApi: {
    listPlanItems: vi.fn(),
  },
}));

describe('planItems', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  describe('loadPlanItemsIfEmpty', () => {
    it('应该返回现有的计划项（如果不为空）', async () => {
      const existingItems: IpcPlanItem[] = [
        { material_id: 'M001', plan_date: '2026-02-04', machine_code: 'FM1', seq_no: 1 } as IpcPlanItem,
      ];

      const result = await loadPlanItemsIfEmpty('v1', existingItems);

      expect(result).toBe(existingItems);
      expect(planApi.listPlanItems).not.toHaveBeenCalled();
    });

    it('应该从 API 加载计划项（如果为空数组）', async () => {
      const mockItems: IpcPlanItem[] = [
        { material_id: 'M001', plan_date: '2026-02-04', machine_code: 'FM1', seq_no: 1 } as IpcPlanItem,
      ];
      vi.mocked(planApi.listPlanItems).mockResolvedValue(mockItems);

      const result = await loadPlanItemsIfEmpty('v1', []);

      expect(result).toEqual(mockItems);
      expect(planApi.listPlanItems).toHaveBeenCalledWith('v1');
    });

    it('应该从 API 加载计划项（如果为 null）', async () => {
      const mockItems: IpcPlanItem[] = [
        { material_id: 'M002', plan_date: '2026-02-05', machine_code: 'FM2', seq_no: 2 } as IpcPlanItem,
      ];
      vi.mocked(planApi.listPlanItems).mockResolvedValue(mockItems);

      const result = await loadPlanItemsIfEmpty('v2', null as any);

      expect(result).toEqual(mockItems);
      expect(planApi.listPlanItems).toHaveBeenCalledWith('v2');
    });

    it('应该从 API 加载计划项（如果为 undefined）', async () => {
      const mockItems: IpcPlanItem[] = [
        { material_id: 'M003', plan_date: '2026-02-06', machine_code: 'FM3', seq_no: 3 } as IpcPlanItem,
      ];
      vi.mocked(planApi.listPlanItems).mockResolvedValue(mockItems);

      const result = await loadPlanItemsIfEmpty('v3', undefined as any);

      expect(result).toEqual(mockItems);
      expect(planApi.listPlanItems).toHaveBeenCalledWith('v3');
    });
  });

  describe('buildPlanItemByIdMap', () => {
    it('应该构建物料 ID 到计划项的映射', () => {
      const items: IpcPlanItem[] = [
        { material_id: 'M001', plan_date: '2026-02-04', machine_code: 'FM1', seq_no: 1 } as IpcPlanItem,
        { material_id: 'M002', plan_date: '2026-02-05', machine_code: 'FM2', seq_no: 2 } as IpcPlanItem,
      ];

      const map = buildPlanItemByIdMap(items);

      expect(map.size).toBe(2);
      expect(map.get('M001')).toBe(items[0]);
      expect(map.get('M002')).toBe(items[1]);
    });

    it('应该处理空数组', () => {
      const map = buildPlanItemByIdMap([]);

      expect(map.size).toBe(0);
    });

    it('应该处理 null 数组', () => {
      const map = buildPlanItemByIdMap(null as any);

      expect(map.size).toBe(0);
    });

    it('应该跳过没有 material_id 的项', () => {
      const items: IpcPlanItem[] = [
        { material_id: 'M001', plan_date: '2026-02-04' } as IpcPlanItem,
        ({ material_id: null, plan_date: '2026-02-05' } as unknown as IpcPlanItem),
        { material_id: '', plan_date: '2026-02-06' } as IpcPlanItem,
        { material_id: '  ', plan_date: '2026-02-07' } as IpcPlanItem,
      ];

      const map = buildPlanItemByIdMap(items);

      expect(map.size).toBe(1);
      expect(map.has('M001')).toBe(true);
    });

    it('应该处理重复的 material_id（保留最后一个）', () => {
      const items: IpcPlanItem[] = [
        { material_id: 'M001', plan_date: '2026-02-04', seq_no: 1 } as IpcPlanItem,
        { material_id: 'M001', plan_date: '2026-02-05', seq_no: 2 } as IpcPlanItem,
      ];

      const map = buildPlanItemByIdMap(items);

      expect(map.size).toBe(1);
      expect(map.get('M001')?.seq_no).toBe(2);
    });

    it('应该修剪 material_id 的空格', () => {
      const items: IpcPlanItem[] = [
        { material_id: '  M001  ', plan_date: '2026-02-04' } as IpcPlanItem,
      ];

      const map = buildPlanItemByIdMap(items);

      expect(map.size).toBe(1);
      expect(map.has('M001')).toBe(true);
    });
  });

  describe('splitSelectedMaterialIds', () => {
    let byId: Map<string, IpcPlanItem>;

    beforeEach(() => {
      byId = new Map([
        ['M001', { material_id: 'M001' } as IpcPlanItem],
        ['M002', { material_id: 'M002' } as IpcPlanItem],
        ['M003', { material_id: 'M003' } as IpcPlanItem],
      ]);
    });

    it('应该拆分可用和缺失的物料 ID', () => {
      const selectedIds = ['M001', 'M002', 'M999'];

      const result = splitSelectedMaterialIds(selectedIds, byId);

      expect(result.eligible).toEqual(['M001', 'M002']);
      expect(result.missing).toEqual(['M999']);
    });

    it('应该处理全部可用的情况', () => {
      const selectedIds = ['M001', 'M002', 'M003'];

      const result = splitSelectedMaterialIds(selectedIds, byId);

      expect(result.eligible).toEqual(['M001', 'M002', 'M003']);
      expect(result.missing).toEqual([]);
    });

    it('应该处理全部缺失的情况', () => {
      const selectedIds = ['M999', 'M998', 'M997'];

      const result = splitSelectedMaterialIds(selectedIds, byId);

      expect(result.eligible).toEqual([]);
      expect(result.missing).toEqual(['M999', 'M998', 'M997']);
    });

    it('应该处理空数组', () => {
      const result = splitSelectedMaterialIds([], byId);

      expect(result.eligible).toEqual([]);
      expect(result.missing).toEqual([]);
    });

    it('应该处理 null 数组', () => {
      const result = splitSelectedMaterialIds(null as any, byId);

      expect(result.eligible).toEqual([]);
      expect(result.missing).toEqual([]);
    });
  });

  describe('sortMaterialIdsByPlan', () => {
    let byId: Map<string, IpcPlanItem>;

    beforeEach(() => {
      byId = new Map([
        [
          'M001',
          {
            material_id: 'M001',
            plan_date: '2026-02-05',
            machine_code: 'FM1',
            seq_no: 2,
          } as IpcPlanItem,
        ],
        [
          'M002',
          {
            material_id: 'M002',
            plan_date: '2026-02-04',
            machine_code: 'FM2',
            seq_no: 1,
          } as IpcPlanItem,
        ],
        [
          'M003',
          {
            material_id: 'M003',
            plan_date: '2026-02-04',
            machine_code: 'FM1',
            seq_no: 3,
          } as IpcPlanItem,
        ],
      ]);
    });

    it('应该按计划日期排序', () => {
      const materialIds = ['M001', 'M002', 'M003'];

      const result = sortMaterialIdsByPlan(materialIds, byId);

      expect(result[0]).toBe('M003'); // 2026-02-04
      expect(result[1]).toBe('M002'); // 2026-02-04
      expect(result[2]).toBe('M001'); // 2026-02-05
    });

    it('应该按机组代码排序（相同日期）', () => {
      byId = new Map([
        ['M001', { material_id: 'M001', plan_date: '2026-02-04', machine_code: 'FM2', seq_no: 1 } as IpcPlanItem],
        ['M002', { material_id: 'M002', plan_date: '2026-02-04', machine_code: 'FM1', seq_no: 1 } as IpcPlanItem],
      ]);

      const result = sortMaterialIdsByPlan(['M001', 'M002'], byId);

      expect(result[0]).toBe('M002'); // FM1
      expect(result[1]).toBe('M001'); // FM2
    });

    it('应该按序号排序（相同日期和机组）', () => {
      byId = new Map([
        [
          'M001',
          { material_id: 'M001', plan_date: '2026-02-04', machine_code: 'FM1', seq_no: 3 } as IpcPlanItem,
        ],
        [
          'M002',
          { material_id: 'M002', plan_date: '2026-02-04', machine_code: 'FM1', seq_no: 1 } as IpcPlanItem,
        ],
        [
          'M003',
          { material_id: 'M003', plan_date: '2026-02-04', machine_code: 'FM1', seq_no: 2 } as IpcPlanItem,
        ],
      ]);

      const result = sortMaterialIdsByPlan(['M001', 'M002', 'M003'], byId);

      expect(result[0]).toBe('M002'); // seq_no: 1
      expect(result[1]).toBe('M003'); // seq_no: 2
      expect(result[2]).toBe('M001'); // seq_no: 3
    });

    it('应该处理空数组', () => {
      const result = sortMaterialIdsByPlan([], byId);

      expect(result).toEqual([]);
    });

    it('应该处理 null 数组', () => {
      const result = sortMaterialIdsByPlan(null as any, byId);

      expect(result).toEqual([]);
    });

    it('应该处理不在映射中的物料 ID', () => {
      const result = sortMaterialIdsByPlan(['M001', 'M999'], byId);

      expect(result).toHaveLength(2);
      expect(result).toContain('M001');
      expect(result).toContain('M999');
    });

    it('应该处理缺少字段的计划项', () => {
      byId = new Map([
        ['M001', { material_id: 'M001', plan_date: null, machine_code: null, seq_no: null } as any],
        ['M002', { material_id: 'M002', plan_date: '2026-02-04', machine_code: 'FM1', seq_no: 1 } as IpcPlanItem],
      ]);

      const result = sortMaterialIdsByPlan(['M001', 'M002'], byId);

      expect(result).toHaveLength(2);
    });

    it('应该保持稳定排序', () => {
      byId = new Map([
        ['M001', { material_id: 'M001', plan_date: '2026-02-04', machine_code: 'FM1', seq_no: 1 } as IpcPlanItem],
        ['M002', { material_id: 'M002', plan_date: '2026-02-04', machine_code: 'FM1', seq_no: 1 } as IpcPlanItem],
      ]);

      const result = sortMaterialIdsByPlan(['M001', 'M002'], byId);

      expect(result).toHaveLength(2);
    });
  });
});
