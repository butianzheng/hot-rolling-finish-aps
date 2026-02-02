// ==========================================
// 热轧精整排产系统 - 事件适配器
// ==========================================
// 职责: 实现 Engine 层定义的 ScheduleEventPublisher trait
// 说明: 将 Engine 层的 ScheduleEvent 转换为 Decision 层的 RefreshTask
// 架构: 依赖倒置 - Decision 层实现 Engine 层定义的接口
// ==========================================

use crate::engine::events::{ScheduleEvent, ScheduleEventPublisher, ScheduleEventType};
use super::{RefreshQueue, RefreshTask, RefreshScope, RefreshTrigger};
use std::error::Error;
use std::sync::Arc;

/// 刷新队列适配器
///
/// 实现 Engine 层定义的 `ScheduleEventPublisher` trait
/// 将 `ScheduleEvent` 转换为 `RefreshTask` 并入队
pub struct RefreshQueueAdapter {
    queue: Arc<RefreshQueue>,
}

impl RefreshQueueAdapter {
    /// 创建新的适配器实例
    pub fn new(queue: Arc<RefreshQueue>) -> Self {
        Self { queue }
    }

    /// 将 Engine 层的事件类型转换为 Decision 层的触发类型
    fn convert_event_type(event_type: &ScheduleEventType) -> RefreshTrigger {
        match event_type {
            ScheduleEventType::PlanItemChanged => RefreshTrigger::PlanItemChanged,
            ScheduleEventType::RiskSnapshotUpdated => RefreshTrigger::RiskSnapshotUpdated,
            ScheduleEventType::MaterialStateChanged => RefreshTrigger::MaterialStateChanged,
            ScheduleEventType::CapacityPoolChanged => RefreshTrigger::CapacityPoolChanged,
            ScheduleEventType::RollCampaignChanged => RefreshTrigger::RollCampaignChanged,
            ScheduleEventType::RhythmTargetChanged => RefreshTrigger::RhythmTargetChanged,
            ScheduleEventType::VersionCreated => RefreshTrigger::VersionCreated,
            ScheduleEventType::ManualTrigger => RefreshTrigger::ManualRefresh,
        }
    }

    /// 将 ScheduleEvent 转换为 RefreshTask
    fn convert_to_refresh_task(event: &ScheduleEvent) -> RefreshTask {
        // 转换日期范围
        let affected_date_range = event.affected_date_range.map(|(start, end)| {
            (start.to_string(), end.to_string())
        });

        // 创建刷新范围
        let scope = RefreshScope {
            version_id: event.version_id.clone(),
            is_full_refresh: event.is_full_scope,
            affected_machines: event.affected_machines.clone(),
            affected_date_range,
        };

        // 转换触发类型
        let trigger = Self::convert_event_type(&event.event_type);

        // 创建刷新任务
        RefreshTask::new(
            scope,
            trigger,
            event.source.clone(),
            3, // 默认最多重试 3 次
        )
    }
}

impl ScheduleEventPublisher for RefreshQueueAdapter {
    fn publish(&self, event: ScheduleEvent) -> Result<String, Box<dyn Error + Send + Sync>> {
        // 转换为刷新任务
        let task = Self::convert_to_refresh_task(&event);

        // 入队
        match self.queue.enqueue(task) {
            Ok(task_id) => {
                tracing::info!(
                    "RefreshQueueAdapter: 事件已转换并入队 - task_id={}, version_id={}, event_type={}",
                    task_id,
                    event.version_id,
                    event.event_type.as_str()
                );

                // 当前系统未启用后台 worker，若仅入队会导致决策读模型长期不刷新，
                // 从而出现“驾驶舱/决策看板数据为空且不联动”的问题。
                // 这里在发布事件后同步处理队列，保证读模型及时可用。
                match self.queue.process_all() {
                    Ok(refresh_ids) => {
                        if !refresh_ids.is_empty() {
                            tracing::info!(
                                "RefreshQueueAdapter: 已同步执行刷新任务 - refreshed_count={}",
                                refresh_ids.len()
                            );
                        }
                    }
                    Err(e) => {
                        tracing::warn!("RefreshQueueAdapter: 同步执行刷新任务失败: {}", e);
                    }
                }

                Ok(task_id)
            }
            Err(e) => {
                tracing::error!(
                    "RefreshQueueAdapter: 事件入队失败 - version_id={}, error={}",
                    event.version_id,
                    e
                );
                Err(Box::new(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    e.to_string(),
                )))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;

    #[test]
    fn test_convert_event_type() {
        assert!(matches!(
            RefreshQueueAdapter::convert_event_type(&ScheduleEventType::PlanItemChanged),
            RefreshTrigger::PlanItemChanged
        ));
        assert!(matches!(
            RefreshQueueAdapter::convert_event_type(&ScheduleEventType::RiskSnapshotUpdated),
            RefreshTrigger::RiskSnapshotUpdated
        ));
        assert!(matches!(
            RefreshQueueAdapter::convert_event_type(&ScheduleEventType::MaterialStateChanged),
            RefreshTrigger::MaterialStateChanged
        ));
        assert!(matches!(
            RefreshQueueAdapter::convert_event_type(&ScheduleEventType::CapacityPoolChanged),
            RefreshTrigger::CapacityPoolChanged
        ));
        assert!(matches!(
            RefreshQueueAdapter::convert_event_type(&ScheduleEventType::RollCampaignChanged),
            RefreshTrigger::RollCampaignChanged
        ));
        assert!(matches!(
            RefreshQueueAdapter::convert_event_type(&ScheduleEventType::VersionCreated),
            RefreshTrigger::VersionCreated
        ));
        assert!(matches!(
            RefreshQueueAdapter::convert_event_type(&ScheduleEventType::ManualTrigger),
            RefreshTrigger::ManualRefresh
        ));
    }

    #[test]
    fn test_convert_to_refresh_task_full_scope() {
        let event = ScheduleEvent::full_scope(
            "V001".to_string(),
            ScheduleEventType::PlanItemChanged,
            Some("TestSource".to_string()),
        );

        let task = RefreshQueueAdapter::convert_to_refresh_task(&event);

        assert_eq!(task.scope.version_id, "V001");
        assert!(task.scope.is_full_refresh);
        assert!(task.scope.affected_machines.is_none());
        assert!(task.scope.affected_date_range.is_none());
        assert_eq!(task.trigger_source, Some("TestSource".to_string()));
    }

    #[test]
    fn test_convert_to_refresh_task_incremental() {
        let start_date = NaiveDate::from_ymd_opt(2026, 1, 24).unwrap();
        let end_date = NaiveDate::from_ymd_opt(2026, 1, 30).unwrap();

        let event = ScheduleEvent::incremental(
            "V001".to_string(),
            ScheduleEventType::CapacityPoolChanged,
            None,
            Some(vec!["H032".to_string(), "H033".to_string()]),
            Some((start_date, end_date)),
        );

        let task = RefreshQueueAdapter::convert_to_refresh_task(&event);

        assert_eq!(task.scope.version_id, "V001");
        assert!(!task.scope.is_full_refresh);
        assert_eq!(task.scope.affected_machines.as_ref().unwrap().len(), 2);
        assert!(task.scope.affected_date_range.is_some());

        let (start, end) = task.scope.affected_date_range.unwrap();
        assert_eq!(start, "2026-01-24");
        assert_eq!(end, "2026-01-30");
    }
}
