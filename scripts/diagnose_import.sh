#!/bin/bash
# ==========================================
# 导入功能诊断脚本
# ==========================================

echo "==========================================
导入功能全面诊断
==========================================
"

# 1. 检查数据库表结构
echo "1. 检查数据库表结构..."
DB_PATH="./hot_rolling_aps.db"
if [ -f "$DB_PATH" ]; then
    echo "  数据库文件存在: $DB_PATH"

    # 检查 import_conflict 表
    SCHEMA=$(sqlite3 "$DB_PATH" ".schema import_conflict" 2>/dev/null)
    if echo "$SCHEMA" | grep -q "source_batch_id"; then
        echo "  ✓ import_conflict 表结构正确 (包含 source_batch_id)"
    else
        echo "  ✗ import_conflict 表结构错误 (缺少 source_batch_id)"
        echo "  需要重建数据库或添加列"
    fi

    # 检查 material_master 表
    MM_COUNT=$(sqlite3 "$DB_PATH" "SELECT COUNT(*) FROM material_master" 2>/dev/null)
    echo "  材料主数据表记录数: $MM_COUNT"

    # 检查 material_state 表
    MS_COUNT=$(sqlite3 "$DB_PATH" "SELECT COUNT(*) FROM material_state" 2>/dev/null)
    echo "  材料状态表记录数: $MS_COUNT"
else
    echo "  ✗ 数据库文件不存在: $DB_PATH"
fi

echo ""

# 2. 检查后端进程
echo "2. 检查后端进程..."
BACKEND_PID=$(pgrep -f "热轧精整排产系统" | head -1)
if [ -n "$BACKEND_PID" ]; then
    echo "  ✓ 后端进程运行中 (PID: $BACKEND_PID)"
else
    echo "  ✗ 后端进程未运行"
fi

# 3. 检查前端进程
echo ""
echo "3. 检查前端进程..."
VITE_PID=$(pgrep -f "vite" | head -1)
if [ -n "$VITE_PID" ]; then
    echo "  ✓ Vite 开发服务器运行中 (PID: $VITE_PID)"
else
    echo "  ✗ Vite 开发服务器未运行"
fi

# 4. 检查测试数据文件
echo ""
echo "4. 检查测试数据文件..."
TEST_FILE="tests/fixtures/datasets/01_normal_data.csv"
if [ -f "$TEST_FILE" ]; then
    LINE_COUNT=$(wc -l < "$TEST_FILE")
    echo "  ✓ 测试文件存在: $TEST_FILE ($LINE_COUNT 行)"
else
    echo "  ✗ 测试文件不存在: $TEST_FILE"
fi

# 5. 运行导入单元测试
echo ""
echo "5. 运行导入相关测试..."
TEST_RESULT=$(cargo test --lib import 2>&1 | grep -E "test result")
echo "  $TEST_RESULT"

# 6. 运行导入 API 测试
echo ""
echo "6. 运行导入 API 端到端测试..."
API_TEST_RESULT=$(cargo test --test import_api_e2e_test 2>&1 | grep -E "test result")
echo "  $API_TEST_RESULT"

# 7. 检查 Tauri 命令注册
echo ""
echo "7. 检查 Tauri 命令配置..."
IMPORT_CMD=$(grep -n "import_materials" src/main.rs | head -1)
if [ -n "$IMPORT_CMD" ]; then
    echo "  ✓ import_materials 命令已注册: $IMPORT_CMD"
else
    echo "  ✗ import_materials 命令未在 main.rs 中注册"
fi

# 8. 检查 rename_all 配置
echo ""
echo "8. 检查 rename_all 配置..."
RENAME_COUNT=$(grep -c 'rename_all = "snake_case"' src/app/tauri_commands.rs)
echo "  已配置 rename_all 的命令数: $RENAME_COUNT"

echo ""
echo "==========================================
诊断完成
==========================================
"
