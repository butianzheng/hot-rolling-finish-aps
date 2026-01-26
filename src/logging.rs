// ==========================================
// 日志系统初始化
// ==========================================
// 使用 tracing 和 tracing-subscriber
// 支持环境变量配置日志级别
// ==========================================

use tracing_subscriber::{fmt, EnvFilter};

/// 初始化日志系统
///
/// # 环境变量
/// - RUST_LOG: 日志级别过滤器（默认: info）
///   例如: RUST_LOG=debug 或 RUST_LOG=hot_rolling_aps=trace
///
/// # 示例
/// ```no_run
/// use hot_rolling_aps::logging;
/// logging::init();
/// ```
pub fn init() {
    // 从环境变量读取日志级别，默认为 info
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info"));

    // 配置日志格式
    fmt()
        .with_env_filter(filter)
        .with_target(true)
        .with_thread_ids(false)
        .with_line_number(true)
        .init();
}

/// 初始化测试环境的日志系统
///
/// 使用更详细的日志级别，便于调试
pub fn init_test() {
    let _ = fmt()
        .with_env_filter(EnvFilter::new("debug"))
        .with_test_writer()
        .try_init();
}
