/**
 * 操作日志查询 - 类型定义
 */

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

// 操作类型映射
export const actionTypeLabels: Record<string, { text: string; color: string }> = {
  CREATE_PLAN: { text: '创建方案', color: 'blue' },
  DELETE_PLAN: { text: '删除方案', color: 'red' },
  CREATE_VERSION: { text: '创建版本', color: 'cyan' },
  DELETE_VERSION: { text: '删除版本', color: 'volcano' },
  ACTIVATE_VERSION: { text: '激活版本', color: 'green' },
  ROLLBACK_VERSION: { text: '版本回滚', color: 'volcano' },
  RECALC_FULL: { text: '一键重算', color: 'purple' },
  SIMULATE_RECALC: { text: '试算', color: 'geekblue' },
  APPLY_STRATEGY_DRAFT: { text: '发布策略草案', color: 'purple' },
  // 材料操作（兼容旧/新 action_type 命名）
  BATCH_LOCK: { text: '批量锁定', color: 'orange' },
  LOCK_MATERIALS: { text: '批量锁定', color: 'orange' },
  BATCH_UNLOCK: { text: '批量解锁', color: 'lime' },
  UNLOCK_MATERIALS: { text: '批量解锁', color: 'lime' },
  BATCH_FORCE_RELEASE: { text: '批量强制放行', color: 'volcano' },
  FORCE_RELEASE: { text: '强制放行', color: 'volcano' },
  BATCH_SET_URGENT: { text: '设置紧急标志', color: 'red' },
  SET_URGENT: { text: '设置紧急标志', color: 'red' },

  // 排产操作
  MOVE_ITEMS: { text: '移动排产项', color: 'geekblue' },
  UPDATE_CONFIG: { text: '更新配置', color: 'gold' },
  BATCH_UPDATE_CONFIG: { text: '批量更新配置', color: 'gold' },
  RESTORE_CONFIG: { text: '恢复配置', color: 'gold' },
  SAVE_CUSTOM_STRATEGY: { text: '保存自定义策略', color: 'gold' },
  UPDATE_CAPACITY_POOL: { text: '更新产能池', color: 'gold' },
  CREATE_ROLL_CAMPAIGN: { text: '创建换辊窗口', color: 'magenta' },
  CLOSE_ROLL_CAMPAIGN: { text: '结束换辊窗口', color: 'pink' },
  MANUAL_REFRESH_DECISION: { text: '手动刷新决策数据', color: 'geekblue' },

  // 前端遥测/错误上报（写入 action_log，便于复用现有查询页）
  FRONTEND_ERROR: { text: '前端错误', color: 'red' },
  FRONTEND_WARN: { text: '前端告警', color: 'orange' },
  FRONTEND_INFO: { text: '前端日志', color: 'blue' },
  FRONTEND_DEBUG: { text: '前端调试', color: 'default' },
  FRONTEND_EVENT: { text: '前端事件', color: 'geekblue' },
};
