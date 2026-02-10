use super::RecalcEngine;
use crate::engine::events::{ScheduleEvent, ScheduleEventType};
use chrono::NaiveDate;
use std::error::Error;

impl RecalcEngine {
    /// 触发决策视图刷新
    ///
    /// # 参数
    /// - `version_id`: 版本ID
    /// - `event_type`: 事件类型
    /// - `start_date`: 起始日期 (可选)
    /// - `end_date`: 结束日期 (可选)
    /// - `machine_codes`: 受影响的机组列表 (可选)
    ///
    /// # 返回
    /// - `Ok(())`: 事件发布成功
    /// - `Err`: 发布失败
    ///
    /// # 说明
    /// - 如果 event_publisher 未配置，则跳过刷新
    /// - 如果有 start_date 和 end_date，则进行增量刷新
    /// - 如果有 machine_codes，则刷新指定机组
    pub(super) fn trigger_decision_refresh(
        &self,
        version_id: &str,
        event_type: ScheduleEventType,
        start_date: Option<NaiveDate>,
        end_date: Option<NaiveDate>,
        machine_codes: Option<&[String]>,
    ) -> Result<(), Box<dyn Error>> {
        // 构建事件范围
        let affected_date_range = match (start_date, end_date) {
            (Some(start), Some(end)) => Some((start, end)),
            _ => None,
        };

        let affected_machines = machine_codes.map(|codes| codes.to_vec());

        // 创建排产事件
        let event = ScheduleEvent::incremental(
            version_id.to_string(),
            event_type,
            Some("RecalcEngine triggered refresh".to_string()),
            affected_machines,
            affected_date_range,
        );

        // 发布事件
        match self.event_publisher.publish(event) {
            Ok(task_id) => {
                if !task_id.is_empty() {
                    tracing::info!(
                        "决策视图刷新事件已发布: task_id={}, version_id={}",
                        task_id,
                        version_id
                    );
                }
                Ok(())
            }
            Err(e) => {
                tracing::error!("决策视图刷新事件发布失败: {}", e);
                Err(Box::new(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    e.to_string(),
                )))
            }
        }
    }
}
