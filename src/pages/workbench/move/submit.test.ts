/**
 * workbench/move/submit.ts 单元测试
 */

import { describe, it, expect } from 'vitest';
import { computeMoveStartSeq, validateMoveSubmitParams } from './submit';

describe('workbench/move/submit', () => {
  describe('computeMoveStartSeq', () => {
    it('START_SEQ 模式：返回 moveStartSeq（最小为 1，向下取整）', () => {
      expect(
        computeMoveStartSeq({
          moveSeqMode: 'START_SEQ',
          moveStartSeq: 5.8,
          planItems: [],
          moveTargetMachine: 'M1',
          targetDate: '2026-02-01',
        })
      ).toBe(5);

      expect(
        computeMoveStartSeq({
          moveSeqMode: 'START_SEQ',
          moveStartSeq: 0,
          planItems: [],
          moveTargetMachine: 'M1',
          targetDate: '2026-02-01',
        })
      ).toBe(1);
    });

    it('APPEND 模式：按目标日期/机组的最大 seq_no + 1', () => {
      const planItems: any[] = [
        { material_id: 'a', machine_code: 'M1', plan_date: '2026-02-01', seq_no: 3 },
        { material_id: 'b', machine_code: 'M1', plan_date: '2026-02-01', seq_no: 10 },
        { material_id: 'c', machine_code: 'M1', plan_date: '2026-02-02', seq_no: 99 },
        { material_id: 'd', machine_code: 'M2', plan_date: '2026-02-01', seq_no: 88 },
      ];

      expect(
        computeMoveStartSeq({
          moveSeqMode: 'APPEND',
          moveStartSeq: 123,
          planItems,
          moveTargetMachine: 'M1',
          targetDate: '2026-02-01',
        })
      ).toBe(11);
    });
  });

  describe('validateMoveSubmitParams', () => {
    it('参数缺失时返回提示', () => {
      expect(
        validateMoveSubmitParams({
          activeVersionId: null,
          moveTargetMachine: 'M1',
          moveTargetDateValid: true,
          moveReason: 'ok',
        })
      ).toBe('请先激活一个版本');

      expect(
        validateMoveSubmitParams({
          activeVersionId: 'v1',
          moveTargetMachine: null,
          moveTargetDateValid: true,
          moveReason: 'ok',
        })
      ).toBe('请选择目标机组');

      expect(
        validateMoveSubmitParams({
          activeVersionId: 'v1',
          moveTargetMachine: 'M1',
          moveTargetDateValid: false,
          moveReason: 'ok',
        })
      ).toBe('请选择目标日期');

      expect(
        validateMoveSubmitParams({
          activeVersionId: 'v1',
          moveTargetMachine: 'M1',
          moveTargetDateValid: true,
          moveReason: '   ',
        })
      ).toBe('请输入移动原因');
    });

    it('参数完整时返回 null', () => {
      expect(
        validateMoveSubmitParams({
          activeVersionId: 'v1',
          moveTargetMachine: 'M1',
          moveTargetDateValid: true,
          moveReason: 'test',
        })
      ).toBeNull();
    });
  });
});

