import { describe, it, expect } from 'vitest';
import {
  createFrozenZoneViolation,
  createMaturityViolation,
  createCapacityViolation,
  createExplainabilityViolation,
} from './utils';

describe('RedLineGuard 工具函数', () => {
  describe('createFrozenZoneViolation', () => {
    it('应该创建冻结区保护违规对象（使用默认消息）', () => {
      const materialNos = ['MAT001', 'MAT002'];
      const violation = createFrozenZoneViolation(materialNos);

      expect(violation).toEqual({
        type: 'FROZEN_ZONE_PROTECTION',
        message: '该操作涉及冻结材料，已被系统阻止',
        severity: 'error',
        details: '冻结材料不可自动调整或重排（工业红线1）',
        affectedEntities: ['MAT001', 'MAT002'],
      });
    });

    it('应该创建冻结区保护违规对象（使用自定义消息）', () => {
      const materialNos = ['MAT003'];
      const customMessage = '材料 MAT003 已锁定';
      const violation = createFrozenZoneViolation(materialNos, customMessage);

      expect(violation).toEqual({
        type: 'FROZEN_ZONE_PROTECTION',
        message: customMessage,
        severity: 'error',
        details: '冻结材料不可自动调整或重排（工业红线1）',
        affectedEntities: ['MAT003'],
      });
    });

    it('应该处理空材料列表', () => {
      const violation = createFrozenZoneViolation([]);

      expect(violation.type).toBe('FROZEN_ZONE_PROTECTION');
      expect(violation.severity).toBe('error');
      expect(violation.affectedEntities).toEqual([]);
    });

    it('应该处理多个材料编号', () => {
      const materialNos = ['MAT001', 'MAT002', 'MAT003', 'MAT004'];
      const violation = createFrozenZoneViolation(materialNos);

      expect(violation.affectedEntities).toHaveLength(4);
      expect(violation.affectedEntities).toEqual(materialNos);
    });
  });

  describe('createMaturityViolation', () => {
    it('应该创建适温约束违规对象', () => {
      const materialNos = ['MAT005'];
      const daysToReady = 3;
      const violation = createMaturityViolation(materialNos, daysToReady);

      expect(violation).toEqual({
        type: 'MATURITY_CONSTRAINT',
        message: '材料未适温，无法排产',
        severity: 'warning',
        details: '距离适温还需 3 天',
        affectedEntities: ['MAT005'],
      });
    });

    it('应该正确格式化不同天数', () => {
      const materialNos = ['MAT006'];

      const violation1 = createMaturityViolation(materialNos, 1);
      expect(violation1.details).toBe('距离适温还需 1 天');

      const violation7 = createMaturityViolation(materialNos, 7);
      expect(violation7.details).toBe('距离适温还需 7 天');

      const violation0 = createMaturityViolation(materialNos, 0);
      expect(violation0.details).toBe('距离适温还需 0 天');
    });

    it('应该使用 warning 严重性', () => {
      const violation = createMaturityViolation(['MAT007'], 2);
      expect(violation.severity).toBe('warning');
    });

    it('应该处理多个材料编号', () => {
      const materialNos = ['MAT008', 'MAT009', 'MAT010'];
      const violation = createMaturityViolation(materialNos, 5);

      expect(violation.affectedEntities).toHaveLength(3);
      expect(violation.affectedEntities).toEqual(materialNos);
    });
  });

  describe('createCapacityViolation', () => {
    it('应该创建容量约束违规对象（使用默认详情）', () => {
      const message = '当日产能已满';
      const violation = createCapacityViolation(message);

      expect(violation).toEqual({
        type: 'CAPACITY_FIRST',
        message: '当日产能已满',
        severity: 'error',
        details: '容量池约束优先于材料排序（工业红线4）',
      });
    });

    it('应该创建容量约束违规对象（使用自定义详情）', () => {
      const message = '产能溢出 50 吨';
      const details = '当前产能池已超出限制，需要调整计划';
      const violation = createCapacityViolation(message, details);

      expect(violation).toEqual({
        type: 'CAPACITY_FIRST',
        message: '产能溢出 50 吨',
        severity: 'error',
        details: '当前产能池已超出限制，需要调整计划',
      });
    });

    it('应该使用 error 严重性', () => {
      const violation = createCapacityViolation('容量不足');
      expect(violation.severity).toBe('error');
    });

    it('应该处理空消息', () => {
      const violation = createCapacityViolation('');
      expect(violation.message).toBe('');
      expect(violation.type).toBe('CAPACITY_FIRST');
    });
  });

  describe('createExplainabilityViolation', () => {
    it('应该创建可解释性违规对象', () => {
      const message = '缺少决策原因说明';
      const violation = createExplainabilityViolation(message);

      expect(violation).toEqual({
        type: 'EXPLAINABILITY',
        message: '缺少决策原因说明',
        severity: 'warning',
        details: '所有决策必须提供明确原因（工业红线5）',
      });
    });

    it('应该使用 warning 严重性', () => {
      const violation = createExplainabilityViolation('无说明');
      expect(violation.severity).toBe('warning');
    });

    it('应该处理不同的消息内容', () => {
      const messages = [
        '系统推荐结果未提供理由',
        '优先级计算缺少依据',
        '排产顺序调整无说明',
      ];

      messages.forEach((msg) => {
        const violation = createExplainabilityViolation(msg);
        expect(violation.message).toBe(msg);
        expect(violation.type).toBe('EXPLAINABILITY');
      });
    });

    it('应该处理空消息', () => {
      const violation = createExplainabilityViolation('');
      expect(violation.message).toBe('');
      expect(violation.type).toBe('EXPLAINABILITY');
    });
  });

  describe('工业红线类型映射', () => {
    it('应该为所有工业红线提供工具函数', () => {
      const frozenViolation = createFrozenZoneViolation(['M1']);
      expect(frozenViolation.type).toBe('FROZEN_ZONE_PROTECTION');

      const maturityViolation = createMaturityViolation(['M2'], 1);
      expect(maturityViolation.type).toBe('MATURITY_CONSTRAINT');

      const capacityViolation = createCapacityViolation('测试');
      expect(capacityViolation.type).toBe('CAPACITY_FIRST');

      const explainViolation = createExplainabilityViolation('测试');
      expect(explainViolation.type).toBe('EXPLAINABILITY');
    });
  });

  describe('违规对象结构一致性', () => {
    it('所有违规对象都应该有 type, message, severity', () => {
      const violations = [
        createFrozenZoneViolation(['M1']),
        createMaturityViolation(['M2'], 1),
        createCapacityViolation('测试'),
        createExplainabilityViolation('测试'),
      ];

      violations.forEach((v) => {
        expect(v).toHaveProperty('type');
        expect(v).toHaveProperty('message');
        expect(v).toHaveProperty('severity');
        expect(['error', 'warning', 'info']).toContain(v.severity);
      });
    });

    it('所有违规对象都应该有 details', () => {
      const violations = [
        createFrozenZoneViolation(['M1']),
        createMaturityViolation(['M2'], 1),
        createCapacityViolation('测试'),
        createExplainabilityViolation('测试'),
      ];

      violations.forEach((v) => {
        expect(v).toHaveProperty('details');
        expect(typeof v.details).toBe('string');
        expect(v.details!.length).toBeGreaterThan(0);
      });
    });
  });
});
