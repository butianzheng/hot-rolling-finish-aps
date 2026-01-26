// ==========================================
// 热轧精整排产系统 - 引擎层事件发布
// ==========================================
// 职责: 定义排产事件发布 trait，实现依赖倒置
// 说明: Engine 层定义 trait，Decision 层实现适配器
// 优势: Engine 不再依赖 Decision，遵循依赖倒置原则
// ==========================================

use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use std::error::Error;
use std::sync::Arc;

// ==========================================
// 排产事件类型
// ==========================================

/// 排产事件触发类型
///
/// Engine 层定义的事件类型，用于通知下游系统
/// Decision 层的 RefreshTrigger 可以从此类型转换
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ScheduleEventType {
    /// 排产项变更
    PlanItemChanged,
    /// 风险快照更新
    RiskSnapshotUpdated,
    /// 材料状态变更
    MaterialStateChanged,
    /// 产能池变更
    CapacityPoolChanged,
    /// 换辊活动变更
    RollCampaignChanged,
    /// 版本创建
    VersionCreated,
    /// 手动触发
    ManualTrigger,
}

impl ScheduleEventType {
    /// 转换为字符串标识
    pub fn as_str(&self) -> &str {
        match self {
            ScheduleEventType::PlanItemChanged => "PlanItemChanged",
            ScheduleEventType::RiskSnapshotUpdated => "RiskSnapshotUpdated",
            ScheduleEventType::MaterialStateChanged => "MaterialStateChanged",
            ScheduleEventType::CapacityPoolChanged => "CapacityPoolChanged",
            ScheduleEventType::RollCampaignChanged => "RollCampaignChanged",
            ScheduleEventType::VersionCreated => "VersionCreated",
            ScheduleEventType::ManualTrigger => "ManualTrigger",
        }
    }
}

/// 排产事件
///
/// Engine 层发布的事件，包含版本ID、触发类型和影响范围
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScheduleEvent {
    /// 方案版本 ID
    pub version_id: String,
    /// 事件类型
    pub event_type: ScheduleEventType,
    /// 事件来源描述
    pub source: Option<String>,
    /// 受影响的机组列表（None 表示全部）
    pub affected_machines: Option<Vec<String>>,
    /// 受影响的日期范围
    pub affected_date_range: Option<(NaiveDate, NaiveDate)>,
    /// 是否需要全量处理
    pub is_full_scope: bool,
}

impl ScheduleEvent {
    /// 创建全量事件
    pub fn full_scope(version_id: String, event_type: ScheduleEventType, source: Option<String>) -> Self {
        Self {
            version_id,
            event_type,
            source,
            affected_machines: None,
            affected_date_range: None,
            is_full_scope: true,
        }
    }

    /// 创建增量事件
    pub fn incremental(
        version_id: String,
        event_type: ScheduleEventType,
        source: Option<String>,
        machines: Option<Vec<String>>,
        date_range: Option<(NaiveDate, NaiveDate)>,
    ) -> Self {
        Self {
            version_id,
            event_type,
            source,
            affected_machines: machines,
            affected_date_range: date_range,
            is_full_scope: false,
        }
    }
}

// ==========================================
// 事件发布 Trait
// ==========================================

/// 排产事件发布者 Trait
///
/// Engine 层定义，Decision 层实现
/// 通过 trait 实现依赖倒置，解除 Engine → Decision 的直接依赖
///
/// # 实现说明
/// - Decision 层的 `RefreshQueueAdapter` 实现此 trait
/// - 将 `ScheduleEvent` 转换为 `RefreshTask` 并入队
pub trait ScheduleEventPublisher: Send + Sync {
    /// 发布排产事件
    ///
    /// # 参数
    /// - `event`: 排产事件
    ///
    /// # 返回
    /// - `Ok(task_id)`: 任务 ID（如果支持）或空字符串
    /// - `Err`: 发布失败
    fn publish(&self, event: ScheduleEvent) -> Result<String, Box<dyn Error + Send + Sync>>;
}

/// 空操作事件发布者
///
/// 用于不需要事件发布的场景（如单元测试）
#[derive(Debug, Clone, Default)]
pub struct NoOpEventPublisher;

impl ScheduleEventPublisher for NoOpEventPublisher {
    fn publish(&self, event: ScheduleEvent) -> Result<String, Box<dyn Error + Send + Sync>> {
        tracing::debug!(
            "NoOpEventPublisher: 跳过事件发布 - version_id={}, event_type={}",
            event.version_id,
            event.event_type.as_str()
        );
        Ok(String::new())
    }
}

/// 可选的事件发布者包装
///
/// 简化 Option<Arc<dyn ScheduleEventPublisher>> 的使用
pub struct OptionalEventPublisher {
    inner: Option<Arc<dyn ScheduleEventPublisher>>,
}

impl OptionalEventPublisher {
    /// 创建带发布者的实例
    pub fn with_publisher(publisher: Arc<dyn ScheduleEventPublisher>) -> Self {
        Self {
            inner: Some(publisher),
        }
    }

    /// 创建空实例（不发布事件）
    pub fn none() -> Self {
        Self { inner: None }
    }

    /// 发布事件（如果有发布者）
    pub fn publish(&self, event: ScheduleEvent) -> Result<String, Box<dyn Error + Send + Sync>> {
        match &self.inner {
            Some(publisher) => publisher.publish(event),
            None => {
                tracing::debug!(
                    "OptionalEventPublisher: 未配置发布者，跳过事件 - version_id={}, event_type={}",
                    event.version_id,
                    event.event_type.as_str()
                );
                Ok(String::new())
            }
        }
    }

    /// 检查是否配置了发布者
    pub fn is_configured(&self) -> bool {
        self.inner.is_some()
    }
}

impl Default for OptionalEventPublisher {
    fn default() -> Self {
        Self::none()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_schedule_event_full_scope() {
        let event = ScheduleEvent::full_scope(
            "V001".to_string(),
            ScheduleEventType::PlanItemChanged,
            Some("RecalcEngine".to_string()),
        );

        assert_eq!(event.version_id, "V001");
        assert!(event.is_full_scope);
        assert!(event.affected_machines.is_none());
        assert!(event.affected_date_range.is_none());
    }

    #[test]
    fn test_schedule_event_incremental() {
        let start_date = NaiveDate::from_ymd_opt(2026, 1, 24).unwrap();
        let end_date = NaiveDate::from_ymd_opt(2026, 1, 30).unwrap();

        let event = ScheduleEvent::incremental(
            "V001".to_string(),
            ScheduleEventType::CapacityPoolChanged,
            None,
            Some(vec!["H032".to_string(), "H033".to_string()]),
            Some((start_date, end_date)),
        );

        assert_eq!(event.version_id, "V001");
        assert!(!event.is_full_scope);
        assert_eq!(event.affected_machines.as_ref().unwrap().len(), 2);
        assert!(event.affected_date_range.is_some());
    }

    #[test]
    fn test_noop_publisher() {
        let publisher = NoOpEventPublisher;
        let event = ScheduleEvent::full_scope(
            "V001".to_string(),
            ScheduleEventType::ManualTrigger,
            None,
        );

        let result = publisher.publish(event);
        assert!(result.is_ok());
        assert!(result.unwrap().is_empty());
    }

    #[test]
    fn test_optional_publisher_none() {
        let publisher = OptionalEventPublisher::none();
        assert!(!publisher.is_configured());

        let event = ScheduleEvent::full_scope(
            "V001".to_string(),
            ScheduleEventType::ManualTrigger,
            None,
        );

        let result = publisher.publish(event);
        assert!(result.is_ok());
    }

    #[test]
    fn test_optional_publisher_with_noop() {
        let noop = Arc::new(NoOpEventPublisher) as Arc<dyn ScheduleEventPublisher>;
        let publisher = OptionalEventPublisher::with_publisher(noop);
        assert!(publisher.is_configured());

        let event = ScheduleEvent::full_scope(
            "V001".to_string(),
            ScheduleEventType::VersionCreated,
            None,
        );

        let result = publisher.publish(event);
        assert!(result.is_ok());
    }
}
