use crate::decision::use_cases::d4_machine_bottleneck::{
    BottleneckHeatmap, MachineBottleneckProfile,
};
use rusqlite::Connection;
use std::error::Error;
use std::sync::{Arc, Mutex};

/// D4 机组堵塞仓储
///
/// 职责: 查询机组堵塞概况数据
/// 策略: 优先从 decision_machine_bottleneck 读模型表读取，回退到 capacity_pool/plan_item 实时计算
pub struct BottleneckRepository {
    pub(super) conn: Arc<Mutex<Connection>>,
}


impl BottleneckRepository {
    /// 创建新的 BottleneckRepository 实例
    pub fn new(conn: Arc<Mutex<Connection>>) -> Self {
        Self { conn }
    }

    /// 查询机组堵塞概况
    ///
    /// 策略: 优先从 decision_machine_bottleneck 读模型表读取，如果为空则回退到实时计算
    ///
    /// # 参数
    /// - version_id: 方案版本 ID
    /// - machine_code: 机组代码（可选）
    /// - start_date: 开始日期
    /// - end_date: 结束日期
    ///
    /// # 返回
    /// - Ok(Vec<MachineBottleneckProfile>): 机组堵塞概况列表，按堵塞分数降序排列
    /// - Err: 数据库错误
    pub fn get_bottleneck_profile(
        &self,
        version_id: &str,
        machine_code: Option<&str>,
        start_date: &str,
        end_date: &str,
    ) -> Result<Vec<MachineBottleneckProfile>, Box<dyn Error>> {
        // 优先尝试从读模型表读取
        if let Ok(profiles) = self.get_bottleneck_from_read_model(version_id, machine_code, start_date, end_date) {
            if !profiles.is_empty() {
                tracing::debug!(
                    version_id = version_id,
                    count = profiles.len(),
                    "D4: 从 decision_machine_bottleneck 读模型表读取"
                );
                return Ok(profiles);
            }
        }

        // 回退到实时计算
        tracing::debug!(
            version_id = version_id,
            "D4: 回退到 capacity_pool/plan_item 实时计算"
        );
        self.get_bottleneck_realtime(version_id, machine_code, start_date, end_date)
    }

    /// 查询最堵塞的 N 个机组-日组合
    pub fn get_top_bottlenecks(
        &self,
        version_id: &str,
        start_date: &str,
        end_date: &str,
        top_n: usize,
    ) -> Result<Vec<MachineBottleneckProfile>, Box<dyn Error>> {
        let mut profiles = self.get_bottleneck_profile(version_id, None, start_date, end_date)?;
        profiles.truncate(top_n);
        Ok(profiles)
    }

    /// 获取机组堵塞热力图数据
    pub fn get_bottleneck_heatmap(
        &self,
        version_id: &str,
        start_date: &str,
        end_date: &str,
    ) -> Result<BottleneckHeatmap, Box<dyn Error>> {
        let profiles = self.get_bottleneck_profile(version_id, None, start_date, end_date)?;

        let mut heatmap = BottleneckHeatmap::new(
            version_id.to_string(),
            start_date.to_string(),
            end_date.to_string(),
        );

        for profile in profiles {
            heatmap.add_cell(
                profile.machine_code,
                profile.plan_date,
                profile.bottleneck_score,
                profile.bottleneck_level,
            );
        }

        Ok(heatmap)
    }
}
