#!/bin/bash
# ==============================================
# 测试产能池API
# ==============================================

DB_PATH="$HOME/Library/Application Support/hot-rolling-aps-dev/hot_rolling_aps.db"

echo "=== 产能池API诊断 ==="
echo ""

# 1. 检查当前激活版本
echo "1️⃣  检查当前激活版本："
echo "===================="
sqlite3 "$DB_PATH" <<'EOF'
.mode column
.headers on
SELECT version_id, plan_id, version_no, status, frozen_from_date, created_at
FROM plan_version
WHERE status = 'ACTIVE'
ORDER BY created_at DESC;
EOF
echo ""

# 2. 检查该版本的产能数据
echo "2️⃣  检查产能数据（2026-02-05至2026-02-10）："
echo "===================="
ACTIVE_VERSION=$(sqlite3 "$DB_PATH" "SELECT version_id FROM plan_version WHERE status = 'ACTIVE' ORDER BY created_at DESC LIMIT 1;")
echo "激活版本: $ACTIVE_VERSION"
echo ""

sqlite3 "$DB_PATH" <<EOF
.mode column
.headers on
SELECT
  machine_code,
  plan_date,
  target_capacity_t,
  limit_capacity_t,
  used_capacity_t,
  ROUND(limit_capacity_t - used_capacity_t, 2) as available_capacity_t
FROM capacity_pool
WHERE version_id = '$ACTIVE_VERSION'
  AND plan_date BETWEEN '2026-02-05' AND '2026-02-10'
ORDER BY plan_date, machine_code
LIMIT 20;
EOF
echo ""

# 3. 统计数据量
echo "3️⃣  数据统计："
echo "===================="
sqlite3 "$DB_PATH" <<EOF
SELECT
  COUNT(*) as total_records,
  COUNT(DISTINCT machine_code) as machine_count,
  MIN(plan_date) as min_date,
  MAX(plan_date) as max_date
FROM capacity_pool
WHERE version_id = '$ACTIVE_VERSION';
EOF
echo ""

# 4. 检查机组列表
echo "4️⃣  可用机组列表："
echo "===================="
sqlite3 "$DB_PATH" <<EOF
SELECT DISTINCT machine_code
FROM capacity_pool
WHERE version_id = '$ACTIVE_VERSION'
ORDER BY machine_code;
EOF
echo ""

echo "=== 诊断完成 ==="
