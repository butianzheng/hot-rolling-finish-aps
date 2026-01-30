/**
 * MaterialInspector 类型定义
 */

export interface Material {
  material_id: string;
  machine_code: string;
  weight_t: number;
  steel_mark: string;
  sched_state: string;
  urgent_level: string;
  lock_flag: boolean;
  manual_urgent_flag: boolean;
  is_frozen?: boolean;
  is_mature?: boolean;
  temp_issue?: boolean;
  urgent_reason?: string; // 紧急等级判定原因
  eligibility_reason?: string; // 适温判定原因
  priority_reason?: string; // 优先级排序原因
}

export interface ActionLog {
  action_id: string;
  version_id: string | null;
  action_type: string;
  action_ts: string;
  actor: string;
  payload_json?: any;
  impact_summary_json?: any;
  machine_code?: string | null;
  date_range_start?: string | null;
  date_range_end?: string | null;
  detail?: string | null;
}

export interface MaterialInspectorProps {
  visible: boolean;
  material: Material | null;
  onClose: () => void;
  onLock?: (materialId: string) => void;
  onUnlock?: (materialId: string) => void;
  onSetUrgent?: (materialId: string) => void;
  onClearUrgent?: (materialId: string) => void;
}
