#!/bin/bash
# 验证 v0.7 迁移数据完整性

DB_PATH=${1:-"hot_rolling_aps.db"}

echo "=== v0.7 迁移验证脚本 ==="
echo "数据库路径: $DB_PATH"
echo ""

# 检查 schema_version
echo "1. 检查 schema_version..."
sqlite3 "$DB_PATH" "SELECT version, applied_at FROM schema_version ORDER BY version DESC LIMIT 1;"
echo ""

# 检查字段是否存在
echo "2. 检查 rolling_output_date 字段..."
sqlite3 "$DB_PATH" "PRAGMA table_info(material_master);" | grep rolling_output_date
if [ $? -eq 0 ]; then
  echo "✅ rolling_output_date 字段已添加"
else
  echo "❌ rolling_output_date 字段缺失"
fi
echo ""

# 检查回填覆盖率
echo "3. 检查回填覆盖率..."
sqlite3 "$DB_PATH" <<EOF
SELECT
  COUNT(*) AS total_materials,
  SUM(CASE WHEN output_age_days_raw IS NOT NULL THEN 1 ELSE 0 END) AS has_output_age,
  SUM(CASE WHEN rolling_output_date IS NOT NULL THEN 1 ELSE 0 END) AS has_rolling_date,
  ROUND(100.0 * SUM(CASE WHEN rolling_output_date IS NOT NULL THEN 1 ELSE 0 END) /
    NULLIF(SUM(CASE WHEN output_age_days_raw IS NOT NULL THEN 1 ELSE 0 END), 0), 2) AS coverage_pct
FROM material_master;
EOF
echo ""

# 检查数据合理性
echo "4. 检查数据合理性（产出日期不应晚于创建日期）..."
ANOMALY_COUNT=$(sqlite3 "$DB_PATH" "
  SELECT COUNT(*)
  FROM material_master
  WHERE rolling_output_date IS NOT NULL
    AND rolling_output_date > date(created_at);
")
if [ "$ANOMALY_COUNT" -eq 0 ]; then
  echo "✅ 无异常数据"
else
  echo "⚠️  发现 $ANOMALY_COUNT 条异常数据（产出日期晚于创建日期）"
  echo "异常数据示例（前5条）:"
  sqlite3 "$DB_PATH" "
    SELECT material_id, rolling_output_date, date(created_at) AS created_date, output_age_days_raw
    FROM material_master
    WHERE rolling_output_date IS NOT NULL
      AND rolling_output_date > date(created_at)
    LIMIT 5;
  "
fi
echo ""

echo "=== 验证完成 ==="
