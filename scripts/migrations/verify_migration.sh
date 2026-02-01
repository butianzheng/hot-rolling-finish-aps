#!/bin/bash
# ==========================================
# 迁移验证脚本
# ==========================================
# 用途: 独立验证 capacity_pool 版本化迁移结果
# 作者: 自动生成
# 日期: 2026-02-01
# ==========================================

set -e

# 颜色定义
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

# 配置
DB_PATH="${DB_PATH:-./hot_rolling_aps.db}"

# 计数器
PASS_COUNT=0
FAIL_COUNT=0

# 打印函数
log_info() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

log_pass() {
    echo -e "${GREEN}[PASS]${NC} $1"
    ((PASS_COUNT++))
}

log_fail() {
    echo -e "${RED}[FAIL]${NC} $1"
    ((FAIL_COUNT++))
}

log_warn() {
    echo -e "${YELLOW}[WARN]${NC} $1"
}

# 验证 1: 表结构包含 version_id
verify_schema_has_version_id() {
    echo ""
    log_info "验证 1: 检查表结构是否包含 version_id..."

    local schema=$(sqlite3 "$DB_PATH" "SELECT sql FROM sqlite_master WHERE type='table' AND name='capacity_pool';")

    if echo "$schema" | grep -q "version_id TEXT NOT NULL"; then
        log_pass "version_id 字段存在且为 NOT NULL"
    else
        log_fail "version_id 字段缺失或定义不正确"
        echo "当前表结构:"
        echo "$schema"
    fi
}

# 验证 2: 主键是三字段
verify_primary_key() {
    echo ""
    log_info "验证 2: 检查主键定义..."

    local schema=$(sqlite3 "$DB_PATH" "SELECT sql FROM sqlite_master WHERE type='table' AND name='capacity_pool';")

    if echo "$schema" | grep -q "PRIMARY KEY (version_id, machine_code, plan_date)"; then
        log_pass "主键为 (version_id, machine_code, plan_date)"
    else
        log_fail "主键定义不正确"
        echo "当前表结构:"
        echo "$schema"
    fi
}

# 验证 3: 所有行都有 version_id
verify_no_null_version_id() {
    echo ""
    log_info "验证 3: 检查是否所有行都有 version_id..."

    local null_count=$(sqlite3 "$DB_PATH" "SELECT COUNT(*) FROM capacity_pool WHERE version_id IS NULL OR version_id = '';")

    if [ "$null_count" -eq 0 ]; then
        log_pass "所有行都有有效的 version_id"
    else
        log_fail "发现 $null_count 行缺少 version_id"
    fi
}

# 验证 4: 数据完整性（与 plan_item 对比）
verify_data_consistency() {
    echo ""
    log_info "验证 4: 检查数据一致性（与 plan_item 对比）..."

    # 检查 plan_item 中的版本是否在 capacity_pool 中都有对应数据
    local result=$(sqlite3 "$DB_PATH" <<EOF
SELECT COUNT(*) FROM (
    SELECT DISTINCT pi.version_id, pi.machine_code, pi.plan_date
    FROM plan_item pi
    WHERE NOT EXISTS (
        SELECT 1 FROM capacity_pool cp
        WHERE cp.version_id = pi.version_id
          AND cp.machine_code = pi.machine_code
          AND cp.plan_date = pi.plan_date
    )
);
EOF
)

    if [ "$result" -eq 0 ]; then
        log_pass "plan_item 中的所有 (version_id, machine_code, plan_date) 在 capacity_pool 中都有对应记录"
    else
        log_warn "发现 $result 个 plan_item 组合在 capacity_pool 中缺失（可能正常）"
    fi
}

# 验证 5: 版本外键约束
verify_foreign_key() {
    echo ""
    log_info "验证 5: 检查外键约束..."

    # 启用外键检查
    local fk_violations=$(sqlite3 "$DB_PATH" "PRAGMA foreign_key_check(capacity_pool);")

    if [ -z "$fk_violations" ]; then
        log_pass "外键约束检查通过，无违规数据"
    else
        log_fail "发现外键约束违规:"
        echo "$fk_violations"
    fi
}

# 验证 6: 索引存在
verify_indexes() {
    echo ""
    log_info "验证 6: 检查索引..."

    local index_count=$(sqlite3 "$DB_PATH" "SELECT COUNT(*) FROM sqlite_master WHERE type='index' AND tbl_name='capacity_pool';")

    if [ "$index_count" -ge 2 ]; then
        log_pass "索引已创建 (共 $index_count 个)"
        sqlite3 -header -column "$DB_PATH" "SELECT name FROM sqlite_master WHERE type='index' AND tbl_name='capacity_pool';"
    else
        log_fail "索引数量不足 (当前: $index_count, 预期: >= 2)"
    fi
}

# 验证 7: used_capacity_t 聚合正确性
verify_used_capacity() {
    echo ""
    log_info "验证 7: 检查 used_capacity_t 聚合正确性..."

    # 从 plan_item 聚合，与 capacity_pool 对比
    local mismatch_count=$(sqlite3 "$DB_PATH" <<EOF
SELECT COUNT(*) FROM (
    SELECT
        cp.version_id,
        cp.machine_code,
        cp.plan_date,
        cp.used_capacity_t AS pool_used,
        COALESCE(SUM(pi.weight_t), 0) AS actual_used,
        ABS(cp.used_capacity_t - COALESCE(SUM(pi.weight_t), 0)) AS diff
    FROM capacity_pool cp
    LEFT JOIN plan_item pi ON
        cp.version_id = pi.version_id AND
        cp.machine_code = pi.machine_code AND
        cp.plan_date = pi.plan_date
    GROUP BY cp.version_id, cp.machine_code, cp.plan_date
    HAVING ABS(diff) > 0.01
);
EOF
)

    if [ "$mismatch_count" -eq 0 ]; then
        log_pass "used_capacity_t 与 plan_item 聚合一致"
    else
        log_warn "发现 $mismatch_count 个日期的 used_capacity_t 与 plan_item 不一致（可能需要重新计算）"
    fi
}

# 验证 8: 版本分布统计
verify_version_distribution() {
    echo ""
    log_info "验证 8: 版本分布统计..."

    log_info "capacity_pool 版本分布:"
    sqlite3 -header -column "$DB_PATH" "SELECT version_id, COUNT(*) AS row_count, MIN(plan_date) AS min_date, MAX(plan_date) AS max_date FROM capacity_pool GROUP BY version_id ORDER BY row_count DESC;"

    local version_count=$(sqlite3 "$DB_PATH" "SELECT COUNT(DISTINCT version_id) FROM capacity_pool;")
    log_info "共 $version_count 个不同版本"
}

# 验证 9: 数据量一致性
verify_row_count() {
    echo ""
    log_info "验证 9: 数据量统计..."

    local total_rows=$(sqlite3 "$DB_PATH" "SELECT COUNT(*) FROM capacity_pool;")
    log_info "capacity_pool 总行数: $total_rows"

    if [ "$total_rows" -gt 0 ]; then
        log_pass "数据已迁移 ($total_rows 行)"
    else
        log_fail "capacity_pool 表为空"
    fi
}

# 验证 10: 抽样检查
verify_sample_data() {
    echo ""
    log_info "验证 10: 抽样检查前 5 条数据..."

    sqlite3 -header -column "$DB_PATH" <<EOF
SELECT
    version_id,
    machine_code,
    plan_date,
    target_capacity_t,
    used_capacity_t,
    overflow_t
FROM capacity_pool
LIMIT 5;
EOF
}

# 主流程
main() {
    echo "=========================================="
    echo "  capacity_pool 迁移验证工具"
    echo "=========================================="

    if [ ! -f "$DB_PATH" ]; then
        log_fail "数据库文件不存在: $DB_PATH"
        exit 1
    fi

    log_info "数据库路径: $DB_PATH"

    # 执行所有验证
    verify_schema_has_version_id
    verify_primary_key
    verify_no_null_version_id
    verify_data_consistency
    verify_foreign_key
    verify_indexes
    verify_used_capacity
    verify_version_distribution
    verify_row_count
    verify_sample_data

    # 总结
    echo ""
    echo "=========================================="
    echo "  验证总结"
    echo "=========================================="
    log_pass "通过: $PASS_COUNT"
    log_fail "失败: $FAIL_COUNT"

    if [ "$FAIL_COUNT" -eq 0 ]; then
        echo ""
        log_pass "所有关键验证通过！迁移成功。"
        exit 0
    else
        echo ""
        log_fail "发现 $FAIL_COUNT 个失败项，请检查。"
        exit 1
    fi
}

main "$@"
