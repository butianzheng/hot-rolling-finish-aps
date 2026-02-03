import { Button, Segmented, Select, Space, Tag } from 'antd';
import { InfoCircleOutlined } from '@ant-design/icons';

import type { WorkbenchViewMode } from '../../stores/use-global-store';
import { formatDate } from '../../utils/formatters';

type ScheduleFocus = { machine?: string; date: string; source?: string } | null;

export default function WorkbenchScheduleViewToolbar(props: {
  machineCode: string | null;
  machineOptions: string[];
  onMachineCodeChange: (machineCode: string | null) => void;
  scheduleFocus: ScheduleFocus;
  pathOverridePendingCount: number;
  pathOverrideContextMachineCode: string | null;
  pathOverrideIsFetching: boolean;
  onOpenPathOverrideModal: () => void;
  viewMode: WorkbenchViewMode;
  onViewModeChange: (mode: WorkbenchViewMode) => void;
}) {
  const {
    machineCode,
    machineOptions,
    onMachineCodeChange,
    scheduleFocus,
    pathOverridePendingCount,
    pathOverrideContextMachineCode,
    pathOverrideIsFetching,
    onOpenPathOverrideModal,
    viewMode,
    onViewModeChange,
  } = props;

  const focusDateLabel = scheduleFocus?.date ? formatDate(scheduleFocus.date) : '';
  const focusMachine = String(scheduleFocus?.machine || '').trim();
  const machineFromSelection = machineCode && machineCode !== 'all' ? String(machineCode) : '';
  const focusLabel = focusDateLabel
    ? focusMachine
      ? `${focusMachine} / ${focusDateLabel}`
      : machineFromSelection
        ? `${machineFromSelection} / ${focusDateLabel}`
        : focusDateLabel
    : '';

  return (
    <Space wrap size={8}>
      <Select
        size="small"
        style={{ width: 148 }}
        value={machineCode ?? 'all'}
        onChange={(value) => onMachineCodeChange(value === 'all' ? null : String(value))}
        options={[
          { label: '全部机组', value: 'all' },
          ...machineOptions.map((code) => ({ label: code, value: code })),
        ]}
      />
      {focusLabel ? <Tag color="blue">聚焦：{focusLabel}</Tag> : null}
      <Button
        size="small"
        icon={<InfoCircleOutlined />}
        type={pathOverridePendingCount > 0 ? 'primary' : 'default'}
        danger={pathOverridePendingCount > 0}
        disabled={!pathOverrideContextMachineCode}
        loading={pathOverrideIsFetching}
        onClick={onOpenPathOverrideModal}
      >
        路径待确认{pathOverridePendingCount > 0 ? ` ${pathOverridePendingCount}` : ''}
      </Button>
      <Segmented
        value={viewMode}
        options={[
          { label: '矩阵', value: 'MATRIX' },
          { label: '甘特图', value: 'GANTT' },
          { label: '卡片', value: 'CARD' },
        ]}
        onChange={(value) => onViewModeChange(value as WorkbenchViewMode)}
      />
    </Space>
  );
}

