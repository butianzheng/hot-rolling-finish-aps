/**
 * MaterialPool 行渲染组件
 */

import { Checkbox, Space, Typography } from 'antd';
import { DownOutlined, RightOutlined } from '@ant-design/icons';
import type { RowComponentProps } from 'react-window';
import { MaterialStatusIcons } from '../MaterialStatusIcons';
import { UrgencyTag } from '../UrgencyTag';
import { FONT_FAMILIES } from '../../theme';
import type { MaterialPoolMaterial, PoolRow, UrgencyBucket } from './types';

const { Text } = Typography;

export type RowData = {
  rows: PoolRow[];
  selected: Set<string>;
  onToggle: (id: string, checked: boolean) => void;
  onInspect?: (material: MaterialPoolMaterial) => void;
  onToggleUrgency: (level: UrgencyBucket) => void;
};

export const MaterialPoolRow = ({
  index,
  style,
  rows,
  selected,
  onToggle,
  onInspect,
  onToggleUrgency,
}: RowComponentProps<RowData>) => {
  const row = rows[index];

  if (row.type === 'header') {
    return (
      <div
        style={{
          ...style,
          display: 'flex',
          alignItems: 'center',
          padding: '0 10px',
          borderBottom: '1px solid rgba(0,0,0,0.06)',
          background: 'rgba(0,0,0,0.02)',
          cursor: 'pointer',
          gap: 8,
        }}
        onClick={() => onToggleUrgency(row.level)}
      >
        <span style={{ width: 16, display: 'flex', justifyContent: 'center' }}>
          {row.collapsed ? <RightOutlined /> : <DownOutlined />}
        </span>
        <UrgencyTag level={row.level} />
        <Text style={{ fontWeight: 600 }}>{row.level}</Text>
        <Text type="secondary">({row.count})</Text>
        <Text type="secondary" style={{ marginLeft: 'auto', fontFamily: FONT_FAMILIES.MONOSPACE }}>
          {row.weight.toFixed(2)}t
        </Text>
      </div>
    );
  }

  const m = row.material;
  const checked = selected.has(m.material_id);

  return (
    <div
      style={{
        ...style,
        display: 'flex',
        alignItems: 'center',
        padding: '0 10px',
        borderBottom: '1px solid rgba(0,0,0,0.06)',
        cursor: 'pointer',
        gap: 8,
      }}
      onClick={() => onInspect?.(m)}
    >
      <Checkbox
        checked={checked}
        onClick={(e) => e.stopPropagation()}
        onChange={(e) => onToggle(m.material_id, e.target.checked)}
      />

      <div style={{ flex: 1, minWidth: 0 }}>
        <Space size={8} style={{ width: '100%', justifyContent: 'space-between' }}>
          <Text style={{ fontFamily: FONT_FAMILIES.MONOSPACE }} ellipsis>
            {m.material_id}
          </Text>
          <UrgencyTag level={m.urgent_level} />
        </Space>

        <Space size={8} style={{ width: '100%', justifyContent: 'space-between' }}>
          <Text type="secondary" style={{ fontSize: 12 }} ellipsis>
            {m.steel_mark || '-'} · {Number(m.weight_t || 0).toFixed(2)}t
          </Text>
          <MaterialStatusIcons
            lockFlag={!!m.lock_flag}
            schedState={String(m.sched_state || '')}
            tempIssue={!!m.temp_issue || m.is_mature === false}
          />
        </Space>
      </div>
    </div>
  );
};
