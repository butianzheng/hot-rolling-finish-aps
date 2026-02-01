#!/bin/bash
# ==========================================
# 数据迁移执行脚本
# ==========================================
# 用途: 执行 capacity_pool 版本化迁移
# 作者: 自动生成
# 日期: 2026-02-01
# ==========================================

set -e  # 遇到错误立即退出

# 颜色定义
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# 配置
DB_PATH="${DB_PATH:-./hot_rolling_aps.db}"
BACKUP_DIR="${BACKUP_DIR:-./backups}"
MIGRATION_DIR="${MIGRATION_DIR:-./scripts/migrations}"
TIMESTAMP=$(date +%Y%m%d_%H%M%S)
BACKUP_FILE="${BACKUP_DIR}/hot_rolling_aps_${TIMESTAMP}.db"

# 打印带颜色的消息
log_info() {
    echo -e "${GREEN}[INFO]${NC} $1"
}

log_warn() {
    echo -e "${YELLOW}[WARN]${NC} $1"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# 检查数据库文件是否存在
check_db_exists() {
    if [ ! -f "$DB_PATH" ]; then
        log_error "数据库文件不存在: $DB_PATH"
        exit 1
    fi
    log_info "数据库文件已找到: $DB_PATH"
}

# 创建备份目录
create_backup_dir() {
    if [ ! -d "$BACKUP_DIR" ]; then
        mkdir -p "$BACKUP_DIR"
        log_info "创建备份目录: $BACKUP_DIR"
    fi
}

# 备份数据库
backup_database() {
    log_info "开始备份数据库..."
    cp "$DB_PATH" "$BACKUP_FILE"

    if [ $? -eq 0 ]; then
        log_info "数据库备份成功: $BACKUP_FILE"
    else
        log_error "数据库备份失败"
        exit 1
    fi
}

# 执行迁移前验证
pre_migration_check() {
    log_info "执行迁移前检查..."

    # 检查当前表结构
    local current_schema=$(sqlite3 "$DB_PATH" "SELECT sql FROM sqlite_master WHERE type='table' AND name='capacity_pool';")

    if echo "$current_schema" | grep -q "version_id"; then
        log_warn "capacity_pool 表已包含 version_id 字段，可能已完成迁移"
        echo -n "是否继续？ (y/N): "
        read -r response
        if [[ ! "$response" =~ ^[Yy]$ ]]; then
            log_info "迁移已取消"
            exit 0
        fi
    else
        log_info "确认需要迁移：capacity_pool 表不包含 version_id 字段"
    fi

    # 统计数据量
    local row_count=$(sqlite3 "$DB_PATH" "SELECT COUNT(*) FROM capacity_pool;")
    log_info "当前 capacity_pool 数据量: $row_count 行"

    # 检查是否有激活版本
    local active_version=$(sqlite3 "$DB_PATH" "SELECT version_id FROM plan_version WHERE status = 'ACTIVE' ORDER BY created_at DESC LIMIT 1;")
    if [ -n "$active_version" ]; then
        log_info "发现激活版本: $active_version (将用于数据迁移)"
    else
        log_warn "未发现激活版本，将使用最新版本或 DEFAULT_VERSION"
    fi
}

# 执行迁移
run_migration() {
    log_info "开始执行迁移..."

    local migration_file="${MIGRATION_DIR}/001_capacity_pool_versioning.sql"

    if [ ! -f "$migration_file" ]; then
        log_error "迁移脚本不存在: $migration_file"
        exit 1
    fi

    sqlite3 "$DB_PATH" < "$migration_file"

    if [ $? -eq 0 ]; then
        log_info "迁移脚本执行成功"
    else
        log_error "迁移脚本执行失败"
        log_error "请使用备份恢复数据库: cp $BACKUP_FILE $DB_PATH"
        exit 1
    fi
}

# 执行迁移后验证
post_migration_check() {
    log_info "执行迁移后验证..."

    # 1. 检查新表结构
    local new_schema=$(sqlite3 "$DB_PATH" "SELECT sql FROM sqlite_master WHERE type='table' AND name='capacity_pool';")

    if echo "$new_schema" | grep -q "version_id TEXT NOT NULL"; then
        log_info "✓ 表结构验证通过: version_id 字段已添加"
    else
        log_error "✗ 表结构验证失败: version_id 字段未找到"
        return 1
    fi

    if echo "$new_schema" | grep -q "PRIMARY KEY (version_id, machine_code, plan_date)"; then
        log_info "✓ 主键验证通过: (version_id, machine_code, plan_date)"
    else
        log_error "✗ 主键验证失败"
        return 1
    fi

    # 2. 检查数据迁移
    local new_row_count=$(sqlite3 "$DB_PATH" "SELECT COUNT(*) FROM capacity_pool;")
    log_info "迁移后数据量: $new_row_count 行"

    # 3. 检查版本分布
    log_info "版本分布统计:"
    sqlite3 -header -column "$DB_PATH" "SELECT version_id, COUNT(*) AS row_count FROM capacity_pool GROUP BY version_id;"

    # 4. 检查索引
    local index_count=$(sqlite3 "$DB_PATH" "SELECT COUNT(*) FROM sqlite_master WHERE type='index' AND tbl_name='capacity_pool';")
    log_info "索引数量: $index_count"

    # 5. 抽查数据完整性
    log_info "抽查前 5 条数据:"
    sqlite3 -header -column "$DB_PATH" "SELECT version_id, machine_code, plan_date, used_capacity_t FROM capacity_pool LIMIT 5;"

    log_info "迁移验证完成"
}

# 更新 schema_version 表
update_schema_version() {
    log_info "更新 schema_version..."

    sqlite3 "$DB_PATH" <<EOF
INSERT OR REPLACE INTO schema_version (version, applied_at)
VALUES (1, datetime('now'));
EOF

    if [ $? -eq 0 ]; then
        log_info "schema_version 已更新为版本 1"
    else
        log_warn "schema_version 更新失败（可忽略）"
    fi
}

# 主流程
main() {
    echo "=========================================="
    echo "  capacity_pool 版本化迁移工具"
    echo "=========================================="
    echo ""

    # 1. 检查数据库
    check_db_exists

    # 2. 创建备份目录
    create_backup_dir

    # 3. 备份数据库
    backup_database

    # 4. 迁移前检查
    pre_migration_check

    echo ""
    log_warn "即将执行数据迁移，这将修改数据库结构"
    echo -n "确认继续？ (y/N): "
    read -r response

    if [[ ! "$response" =~ ^[Yy]$ ]]; then
        log_info "迁移已取消"
        exit 0
    fi

    # 5. 执行迁移
    run_migration

    # 6. 迁移后验证
    if post_migration_check; then
        log_info "✓ 所有验证通过"
    else
        log_error "✗ 验证失败，请检查数据库状态"
        log_error "回滚命令: cp $BACKUP_FILE $DB_PATH"
        exit 1
    fi

    # 7. 更新 schema_version
    update_schema_version

    echo ""
    echo "=========================================="
    log_info "迁移成功完成！"
    echo "=========================================="
    echo ""
    log_info "备份文件: $BACKUP_FILE"
    log_info "如需回滚，执行: cp $BACKUP_FILE $DB_PATH"
}

# 执行主流程
main "$@"
