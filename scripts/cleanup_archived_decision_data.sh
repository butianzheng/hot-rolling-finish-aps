#!/bin/bash
# ==============================================
# 清理归档版本的决策数据
# ==============================================
# 目的：删除V001（ARCHIVED）和V002（DRAFT）的过期决策数据
#      这些数据占用空间且可能引起混淆
#
# 使用方法：
#   ./scripts/cleanup_archived_decision_data.sh
# ==============================================

DB_PATH="$HOME/Library/Application Support/hot-rolling-aps-dev/hot_rolling_aps.db"

echo "=== 清理归档版本决策数据 ==="
echo "数据库路径: $DB_PATH"
echo ""

# 检查数据库是否存在
if [ ! -f "$DB_PATH" ]; then
  echo "❌ 错误：数据库文件不存在"
  exit 1
fi

# 备份数据库
BACKUP_FILE="${DB_PATH}.bak.$(date +%Y%m%d_%H%M%S)"
echo "1. 创建备份: $BACKUP_FILE"
cp "$DB_PATH" "$BACKUP_FILE"

if [ $? -eq 0 ]; then
  echo "✅ 备份成功"
else
  echo "❌ 备份失败，操作中止"
  exit 1
fi
echo ""

# 查询当前决策表的记录数
echo "2. 清理前统计："
sqlite3 "$DB_PATH" <<EOF
.mode column
.headers on

SELECT
  'decision_machine_bottleneck' as table_name,
  version_id,
  COUNT(*) as record_count
FROM decision_machine_bottleneck
GROUP BY version_id
ORDER BY version_id;

SELECT
  'decision_day_summary' as table_name,
  version_id,
  COUNT(*) as record_count
FROM decision_day_summary
GROUP BY version_id
ORDER BY version_id;

SELECT
  'capacity_pool' as table_name,
  version_id,
  COUNT(*) as record_count
FROM capacity_pool
GROUP BY version_id
ORDER BY version_id;
EOF
echo ""

# 清理操作
echo "3. 执行清理（删除V001和V002的决策数据）..."
sqlite3 "$DB_PATH" <<'EOF'
BEGIN TRANSACTION;

-- 清理decision_machine_bottleneck
DELETE FROM decision_machine_bottleneck
WHERE version_id IN ('V001', 'V002');

-- 清理decision_day_summary
DELETE FROM decision_day_summary
WHERE version_id IN ('V001', 'V002');

-- 清理decision_cold_stock_profile
DELETE FROM decision_cold_stock_profile
WHERE version_id IN ('V001', 'V002');

-- 清理decision_capacity_opportunity
DELETE FROM decision_capacity_opportunity
WHERE version_id IN ('V001', 'V002');

-- 清理decision_roll_campaign_alert
DELETE FROM decision_roll_campaign_alert
WHERE version_id IN ('V001', 'V002');

-- 清理decision_order_failure_set（注意：这个表没有version_id列，需要检查结构）
-- DELETE FROM decision_order_failure_set
-- WHERE version_id IN ('V001', 'V002');

-- 清理capacity_pool（V001和V002的产能池数据）
DELETE FROM capacity_pool
WHERE version_id IN ('V001', 'V002');

COMMIT;

-- 执行VACUUM优化数据库文件大小
VACUUM;
EOF

if [ $? -eq 0 ]; then
  echo "✅ 清理成功"
else
  echo "❌ 清理失败，请检查错误信息"
  echo "如需恢复，请使用备份文件: $BACKUP_FILE"
  exit 1
fi
echo ""

# 查询清理后的记录数
echo "4. 清理后统计："
sqlite3 "$DB_PATH" <<EOF
.mode column
.headers on

SELECT
  'decision_machine_bottleneck' as table_name,
  version_id,
  COUNT(*) as record_count
FROM decision_machine_bottleneck
GROUP BY version_id
ORDER BY version_id;

SELECT
  'decision_day_summary' as table_name,
  version_id,
  COUNT(*) as record_count
FROM decision_day_summary
GROUP BY version_id
ORDER BY version_id;

SELECT
  'capacity_pool' as table_name,
  version_id,
  COUNT(*) as record_count
FROM capacity_pool
GROUP BY version_id
ORDER BY version_id;
EOF
echo ""

# 统计释放的空间
ORIGINAL_SIZE=$(stat -f%z "$BACKUP_FILE")
NEW_SIZE=$(stat -f%z "$DB_PATH")
SAVED_MB=$(echo "scale=2; ($ORIGINAL_SIZE - $NEW_SIZE) / 1024 / 1024" | bc)

echo "5. 空间释放："
echo "   清理前: $(echo "scale=2; $ORIGINAL_SIZE / 1024 / 1024" | bc) MB"
echo "   清理后: $(echo "scale=2; $NEW_SIZE / 1024 / 1024" | bc) MB"
echo "   节省: ${SAVED_MB} MB"
echo ""

echo "=== 清理完成 ==="
echo "备份文件: $BACKUP_FILE"
echo "如需回滚: cp \"$BACKUP_FILE\" \"$DB_PATH\""
