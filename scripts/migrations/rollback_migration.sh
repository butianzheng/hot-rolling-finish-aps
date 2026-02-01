#!/bin/bash
# ==========================================
# 迁移回滚脚本
# ==========================================
# 用途: 回滚 capacity_pool 版本化迁移
# 作者: 自动生成
# 日期: 2026-02-01
# ==========================================

set -e

# 颜色定义
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

# 配置
DB_PATH="${DB_PATH:-./hot_rolling_aps.db}"
BACKUP_DIR="${BACKUP_DIR:-./backups}"

# 打印函数
log_info() {
    echo -e "${GREEN}[INFO]${NC} $1"
}

log_warn() {
    echo -e "${YELLOW}[WARN]${NC} $1"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# 查找最新备份
find_latest_backup() {
    if [ ! -d "$BACKUP_DIR" ]; then
        log_error "备份目录不存在: $BACKUP_DIR"
        exit 1
    fi

    local latest_backup=$(ls -t "$BACKUP_DIR"/hot_rolling_aps_*.db 2>/dev/null | head -1)

    if [ -z "$latest_backup" ]; then
        log_error "未找到备份文件"
        exit 1
    fi

    echo "$latest_backup"
}

# 从备份恢复
restore_from_backup() {
    local backup_file="$1"

    log_info "准备从备份恢复: $backup_file"

    # 验证备份文件
    if ! sqlite3 "$backup_file" "SELECT COUNT(*) FROM capacity_pool;" > /dev/null 2>&1; then
        log_error "备份文件损坏或无效"
        exit 1
    fi

    # 创建当前数据库的备份（以防万一）
    local safety_backup="${DB_PATH}.before_rollback_$(date +%Y%m%d_%H%M%S)"
    cp "$DB_PATH" "$safety_backup"
    log_info "已创建安全备份: $safety_backup"

    # 恢复
    cp "$backup_file" "$DB_PATH"

    if [ $? -eq 0 ]; then
        log_info "恢复成功"
    else
        log_error "恢复失败"
        exit 1
    fi
}

# 验证回滚结果
verify_rollback() {
    log_info "验证回滚结果..."

    # 检查表结构（应该不包含 version_id）
    local schema=$(sqlite3 "$DB_PATH" "SELECT sql FROM sqlite_master WHERE type='table' AND name='capacity_pool';")

    if echo "$schema" | grep -q "PRIMARY KEY (machine_code, plan_date)"; then
        log_info "✓ 表结构已恢复为旧版本 (machine_code, plan_date)"
    else
        log_warn "表结构可能未完全恢复"
    fi

    # 统计数据
    local row_count=$(sqlite3 "$DB_PATH" "SELECT COUNT(*) FROM capacity_pool;")
    log_info "数据量: $row_count 行"
}

# 手动回滚（SQL 方式）
manual_rollback() {
    log_warn "使用 SQL 方式回滚（去除 version_id）"

    sqlite3 "$DB_PATH" <<'EOF'
PRAGMA foreign_keys = OFF;

-- 1. 创建旧表结构
CREATE TABLE IF NOT EXISTS capacity_pool_old (
  machine_code TEXT NOT NULL REFERENCES machine_master(machine_code),
  plan_date TEXT NOT NULL,
  target_capacity_t REAL NOT NULL,
  limit_capacity_t REAL NOT NULL,
  used_capacity_t REAL NOT NULL DEFAULT 0.0,
  overflow_t REAL NOT NULL DEFAULT 0.0,
  frozen_capacity_t REAL NOT NULL DEFAULT 0.0,
  accumulated_tonnage_t REAL NOT NULL DEFAULT 0.0,
  roll_campaign_id TEXT,
  PRIMARY KEY (machine_code, plan_date)
);

-- 2. 迁移数据（合并多版本，保留 used_capacity_t 最大的）
INSERT INTO capacity_pool_old (
  machine_code,
  plan_date,
  target_capacity_t,
  limit_capacity_t,
  used_capacity_t,
  overflow_t,
  frozen_capacity_t,
  accumulated_tonnage_t,
  roll_campaign_id
)
SELECT
  machine_code,
  plan_date,
  MAX(target_capacity_t) AS target_capacity_t,
  MAX(limit_capacity_t) AS limit_capacity_t,
  MAX(used_capacity_t) AS used_capacity_t,
  MAX(overflow_t) AS overflow_t,
  MAX(frozen_capacity_t) AS frozen_capacity_t,
  MAX(accumulated_tonnage_t) AS accumulated_tonnage_t,
  MAX(roll_campaign_id) AS roll_campaign_id
FROM capacity_pool
GROUP BY machine_code, plan_date;

-- 3. 删除新表
DROP TABLE capacity_pool;

-- 4. 重命名
ALTER TABLE capacity_pool_old RENAME TO capacity_pool;

-- 5. 创建索引
CREATE INDEX IF NOT EXISTS idx_pool_machine_date ON capacity_pool(machine_code, plan_date);

PRAGMA foreign_keys = ON;
EOF

    if [ $? -eq 0 ]; then
        log_info "SQL 回滚成功"
    else
        log_error "SQL 回滚失败"
        exit 1
    fi
}

# 主流程
main() {
    echo "=========================================="
    echo "  capacity_pool 迁移回滚工具"
    echo "=========================================="
    echo ""

    log_warn "此操作将回滚 capacity_pool 版本化迁移"
    log_warn "数据库路径: $DB_PATH"

    echo ""
    echo "选择回滚方式:"
    echo "1) 从备份恢复（推荐）"
    echo "2) SQL 回滚（去除 version_id，合并多版本数据）"
    echo "3) 取消"
    echo -n "请选择 (1/2/3): "
    read -r choice

    case $choice in
        1)
            local backup_file=$(find_latest_backup)
            echo ""
            log_info "找到最新备份: $backup_file"
            echo -n "确认恢复？ (y/N): "
            read -r confirm

            if [[ "$confirm" =~ ^[Yy]$ ]]; then
                restore_from_backup "$backup_file"
                verify_rollback
                log_info "回滚完成"
            else
                log_info "已取消"
            fi
            ;;
        2)
            echo ""
            log_warn "SQL 回滚会合并多版本数据，可能导致数据丢失"
            echo -n "确认继续？ (y/N): "
            read -r confirm

            if [[ "$confirm" =~ ^[Yy]$ ]]; then
                manual_rollback
                verify_rollback
                log_info "回滚完成"
            else
                log_info "已取消"
            fi
            ;;
        3)
            log_info "已取消"
            exit 0
            ;;
        *)
            log_error "无效选择"
            exit 1
            ;;
    esac
}

main "$@"
