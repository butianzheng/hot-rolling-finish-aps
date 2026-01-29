import { Card, Checkbox, Space, Tag, Typography } from 'antd';
import type { RowComponentProps } from 'react-window';
import { FONT_FAMILIES } from '../../theme';
import { formatWeight } from '../../utils/formatters';
import type { PlanItemRow } from './types';

const { Text } = Typography;

export type ScheduleCardRowData = {
  items: PlanItemRow[];
  selected: Set<string>;
  onToggle: (id: string, checked: boolean) => void;
  onInspect?: (id: string) => void;
};

export const ScheduleCardRow = ({
  index,
  style,
  items,
  selected,
  onToggle,
  onInspect,
}: RowComponentProps<ScheduleCardRowData>) => {
  const it = items[index];
  const checked = selected.has(it.material_id);
  const urgent = String(it.urgent_level || 'L0');

  const urgentColor =
    urgent === 'L3' ? 'red' : urgent === 'L2' ? 'orange' : urgent === 'L1' ? 'blue' : 'default';

  return (
    <div style={{ ...style, padding: '0 8px' }}>
      <Card
        size="small"
        style={{ cursor: 'pointer' }}
        onClick={() => onInspect?.(it.material_id)}
      >
        <Space align="start" style={{ width: '100%', justifyContent: 'space-between' }}>
          <Space align="start" size={10}>
            <Checkbox
              checked={checked}
              onClick={(e) => e.stopPropagation()}
              onChange={(e) => onToggle(it.material_id, e.target.checked)}
            />
            <Space direction="vertical" size={2} style={{ minWidth: 0 }}>
              <Text style={{ fontFamily: FONT_FAMILIES.MONOSPACE }} strong ellipsis>
                {it.material_id}
              </Text>
              <Text type="secondary" style={{ fontSize: 12 }}>
                {it.machine_code} · {it.plan_date} · 序{it.seq_no}
              </Text>
              <Text type="secondary" style={{ fontSize: 12 }}>
                {formatWeight(it.weight_t)}
              </Text>
            </Space>
          </Space>

          <Space direction="vertical" size={4} align="end">
            <Tag color={urgentColor}>{urgent}</Tag>
            {it.locked_in_plan && <Tag color="purple">冻结</Tag>}
            {it.force_release_in_plan && <Tag color="orange">强制放行</Tag>}
          </Space>
        </Space>
      </Card>
    </div>
  );
};
