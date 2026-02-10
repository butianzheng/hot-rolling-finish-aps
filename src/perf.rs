use rusqlite::Connection;
use std::cell::Cell;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::{Duration, Instant};

static PERF_SQL_ENABLED: AtomicBool = AtomicBool::new(false);
static SLOW_SQL_THRESHOLD_MS: AtomicU64 = AtomicU64::new(0);

thread_local! {
    static PERF_DEPTH: Cell<u32> = Cell::new(0);
    static SQL_COUNT: Cell<u64> = Cell::new(0);
    static SLOW_SQL_COUNT: Cell<u64> = Cell::new(0);
}

fn is_true(v: &str) -> bool {
    matches!(
        v.trim().to_lowercase().as_str(),
        "1" | "true" | "yes" | "y" | "on"
    )
}

fn truncate_sql(sql: &str, max_len: usize) -> String {
    let s = sql.trim().replace('\n', " ");
    if s.len() <= max_len {
        return s;
    }
    format!("{}…", &s[..max_len])
}

/// 安装 SQLite 语句 trace/profile（用于 SQL 计数 + 慢查询日志）
///
/// 开关：
/// - Debug 默认开启；Release 默认关闭（可通过环境变量开启）
/// - `HOT_ROLLING_APS_PERF_SQL=1` 强制开启
/// - `HOT_ROLLING_APS_SLOW_SQL_MS=50` 配置慢 SQL 阈值（毫秒）
pub fn install_sqlite_tracing(conn: &mut Connection) {
    let enabled = match std::env::var("HOT_ROLLING_APS_PERF_SQL") {
        Ok(v) => is_true(&v),
        Err(_) => cfg!(debug_assertions),
    };

    PERF_SQL_ENABLED.store(enabled, Ordering::Relaxed);

    if !enabled {
        // 显式清理，避免复用连接导致残留 callback
        conn.trace(None);
        conn.profile(None);
        return;
    }

    let slow_ms = std::env::var("HOT_ROLLING_APS_SLOW_SQL_MS")
        .ok()
        .and_then(|v| v.trim().parse::<u64>().ok())
        .unwrap_or(if cfg!(debug_assertions) { 50 } else { 200 });
    SLOW_SQL_THRESHOLD_MS.store(slow_ms, Ordering::Relaxed);

    conn.trace(Some(sql_trace_callback));
    conn.profile(Some(sql_profile_callback));
}

fn sql_trace_callback(_sql: &str) {
    if !PERF_SQL_ENABLED.load(Ordering::Relaxed) {
        return;
    }
    let active = PERF_DEPTH.with(|d| d.get() > 0);
    if !active {
        return;
    }
    SQL_COUNT.with(|c| c.set(c.get().saturating_add(1)));
}

fn sql_profile_callback(sql: &str, duration: Duration) {
    if !PERF_SQL_ENABLED.load(Ordering::Relaxed) {
        return;
    }

    let ms = duration.as_millis() as u64;
    let threshold = SLOW_SQL_THRESHOLD_MS.load(Ordering::Relaxed);
    if threshold > 0 && ms >= threshold {
        let sql_short = truncate_sql(sql, 420);
        tracing::warn!(
            target: "slow_sql",
            duration_ms = ms,
            sql = %sql_short,
            "slow sql"
        );
        let active = PERF_DEPTH.with(|d| d.get() > 0);
        if active {
            SLOW_SQL_COUNT.with(|c| c.set(c.get().saturating_add(1)));
        }
    }
}

/// 性能统计 Guard：记录 elapsed_ms + SQL 语句数 + 慢 SQL 数
///
/// 使用方式：
/// ```ignore
/// let _perf = hot_rolling_aps::perf::PerfGuard::new("list_plan_items");
/// // do work...
/// ```
pub struct PerfGuard {
    op: &'static str,
    start: Instant,
    sql_start: u64,
    slow_sql_start: u64,
}

impl PerfGuard {
    pub fn new(op: &'static str) -> Self {
        PERF_DEPTH.with(|d| d.set(d.get().saturating_add(1)));
        let sql_start = SQL_COUNT.with(|c| c.get());
        let slow_sql_start = SLOW_SQL_COUNT.with(|c| c.get());
        Self {
            op,
            start: Instant::now(),
            sql_start,
            slow_sql_start,
        }
    }
}

impl Drop for PerfGuard {
    fn drop(&mut self) {
        let elapsed_ms = self.start.elapsed().as_millis() as u64;
        let sql_end = SQL_COUNT.with(|c| c.get());
        let slow_sql_end = SLOW_SQL_COUNT.with(|c| c.get());
        let sql_count = sql_end.saturating_sub(self.sql_start);
        let slow_sql_count = slow_sql_end.saturating_sub(self.slow_sql_start);

        tracing::info!(
            target: "perf",
            op = self.op,
            elapsed_ms,
            sql_count,
            slow_sql_count,
            "done"
        );

        PERF_DEPTH.with(|d| d.set(d.get().saturating_sub(1)));
    }
}
