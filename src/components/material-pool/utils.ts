/**
 * MaterialPool 辅助函数
 */

import React from 'react';
import { Space, Typography } from 'antd';
import type { DataNode } from 'antd/es/tree';
import type { MaterialPoolMaterial, MaterialPoolSelection, UrgencyBucket } from './types';
import { normalizeSchedState, getSchedStateLabel } from '../../utils/schedState';

const { Text } = Typography;

export type MaterialPoolSummary = {
  total_count: number;
  machines: Array<{
    machine_code: string;
    total_count: number;
    states: Array<{ sched_state: string; count: number }>;
  }>;
};

/**
 * 构建树形数据结构
 */
export function buildTreeData(materials: MaterialPoolMaterial[]): DataNode[] {
  const machineMap = new Map<string, Map<string, number>>();
  materials.forEach((m) => {
    const machine = String(m.machine_code || '').trim() || 'UNKNOWN';
    const state = normalizeSchedState(m.sched_state);
    if (!machineMap.has(machine)) machineMap.set(machine, new Map());
    const stateMap = machineMap.get(machine)!;
    stateMap.set(state, (stateMap.get(state) ?? 0) + 1);
  });

  const machines = Array.from(machineMap.keys()).sort();
  const preferredStates = ['READY', 'PENDING_MATURE', 'FORCE_RELEASE', 'LOCKED', 'SCHEDULED', 'BLOCKED'];

  return [
    {
      key: 'all',
      title: React.createElement(Space, { size: 8 },
        React.createElement(Text, { strong: true }, '全部机组'),
        React.createElement(Text, { type: 'secondary' }, `(${materials.length})`)
      ),
      isLeaf: true,
    },
    ...machines.map((machine) => {
      const stateMap = machineMap.get(machine)!;
      const states = Array.from(stateMap.keys()).sort((a, b) => {
        const ai = preferredStates.indexOf(a);
        const bi = preferredStates.indexOf(b);
        if (ai !== -1 || bi !== -1) return (ai === -1 ? 99 : ai) - (bi === -1 ? 99 : bi);
        return a.localeCompare(b);
      });

      return {
        key: `machine:${machine}`,
        title: React.createElement(Space, { size: 8 },
          React.createElement(Text, { strong: true }, machine),
          React.createElement(Text, { type: 'secondary' }, `(${states.reduce((sum, s) => sum + (stateMap.get(s) ?? 0), 0)})`)
        ),
        children: states.map((state) => {
          const count = stateMap.get(state) ?? 0;
          return {
            key: `machine:${machine}/state:${state}`,
            title: React.createElement(Space, { size: 8 },
              React.createElement(Text, null, getSchedStateLabel(state)),
              React.createElement(Text, { type: 'secondary' }, `(${count})`)
            ),
            isLeaf: true,
          };
        }),
      };
    }),
  ];
}

/**
 * 从后端汇总数据构建树形数据结构
 *
 * 用途：
 * - 大数据量下避免依赖“当前页 materials”推导树节点数量（会失真）
 */
export function buildTreeDataFromSummary(summary: MaterialPoolSummary | null | undefined): DataNode[] {
  const total = Number(summary?.total_count ?? 0);
  const machinesRaw = Array.isArray(summary?.machines) ? summary!.machines : [];

  const preferredStates = ['READY', 'PENDING_MATURE', 'FORCE_RELEASE', 'LOCKED', 'SCHEDULED', 'BLOCKED'];
  const machines = [...machinesRaw].sort((a, b) => String(a.machine_code).localeCompare(String(b.machine_code)));

  return [
    {
      key: 'all',
      title: React.createElement(
        Space,
        { size: 8 },
        React.createElement(Text, { strong: true }, '全部机组'),
        React.createElement(Text, { type: 'secondary' }, `(${total})`)
      ),
      isLeaf: true,
    },
    ...machines.map((m) => {
      const machineCode = String(m.machine_code || '').trim() || 'UNKNOWN';
      const statesRaw = Array.isArray(m.states) ? m.states : [];
      const states = [...statesRaw].sort((a, b) => {
        const sa = normalizeSchedState(a.sched_state);
        const sb = normalizeSchedState(b.sched_state);
        const ai = preferredStates.indexOf(sa);
        const bi = preferredStates.indexOf(sb);
        if (ai !== -1 || bi !== -1) return (ai === -1 ? 99 : ai) - (bi === -1 ? 99 : bi);
        return sa.localeCompare(sb);
      });

      const machineTotal = Number(m.total_count ?? states.reduce((sum, s) => sum + Number(s.count || 0), 0));

      return {
        key: `machine:${machineCode}`,
        title: React.createElement(
          Space,
          { size: 8 },
          React.createElement(Text, { strong: true }, machineCode),
          React.createElement(Text, { type: 'secondary' }, `(${machineTotal})`)
        ),
        children: states.map((s) => {
          const state = normalizeSchedState(s.sched_state);
          const count = Number(s.count || 0);
          return {
            key: `machine:${machineCode}/state:${state}`,
            title: React.createElement(
              Space,
              { size: 8 },
              React.createElement(Text, null, getSchedStateLabel(state)),
              React.createElement(Text, { type: 'secondary' }, `(${count})`)
            ),
            isLeaf: true,
          };
        }),
      };
    }),
  ];
}

/**
 * 解析树节点 key 为选择状态
 */
export function parseTreeKey(key: string): MaterialPoolSelection {
  if (!key.startsWith('machine:')) return { machineCode: null, schedState: null };
  const rest = key.slice('machine:'.length);
  const [machineCode, statePart] = rest.split('/state:');
  if (!machineCode) return { machineCode: null, schedState: null };
  return { machineCode, schedState: statePart || null };
}

/**
 * 选择状态转换为树节点 key
 */
export function selectionToTreeKey(selection: MaterialPoolSelection): string | null {
  if (!selection.machineCode) return 'all';
  if (selection.schedState) return `machine:${selection.machineCode}/state:${selection.schedState}`;
  return `machine:${selection.machineCode}`;
}

/**
 * 标准化紧急级别
 */
export function normalizeUrgencyLevel(level: string | null | undefined): UrgencyBucket {
  const v = String(level || '').toUpperCase().trim();
  if (v === 'L3') return 'L3';
  if (v === 'L2') return 'L2';
  if (v === 'L1') return 'L1';
  return 'L0';
}
