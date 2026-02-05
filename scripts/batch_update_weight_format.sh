#!/bin/bash
# ==============================================
# 批量替换重量相关的toFixed精度
# ==============================================
# 将所有重量/产能相关的toFixed(2)改为toFixed(3)
# ==============================================

echo "=== 批量替换重量格式化精度 ==="
echo ""

# 备份列表
BACKUP_DIR="backups/weight_format_change_$(date +%Y%m%d_%H%M%S)"
mkdir -p "$BACKUP_DIR"

# 需要处理的文件模式
FILES=(
  "src/components/**/*.{ts,tsx}"
  "src/pages/**/*.{ts,tsx}"
  "src/utils/**/*.{ts,tsx}"
)

# 统计
TOTAL_CHANGES=0
TOTAL_FILES=0

echo "1️⃣  开始搜索需要修改的文件..."
echo "================================"

# 查找所有包含weight相关toFixed(2)的文件
for pattern in "${FILES[@]}"; do
  find src -type f \( -name "*.ts" -o -name "*.tsx" \) | while read file; do
    # 检查文件是否包含重量相关的toFixed
    if grep -q "weight.*toFixed\|capacity.*toFixed\|吨.*toFixed\|\.toFixed(2).*t\|weightT.*toFixed" "$file"; then
      echo "📝 发现: $file"

      # 备份原文件
      cp "$file" "$BACKUP_DIR/$(basename $file).bak"

      # 执行替换（只替换重量相关的）
      sed -i '' 's/\(weight[^.]*\)\.toFixed(2)/\1.toFixed(3)/g' "$file"
      sed -i '' 's/\(capacity[^.]*\)\.toFixed(2)/\1.toFixed(3)/g' "$file"
      sed -i '' 's/\(weightT[^.]*\)\.toFixed(2)/\1.toFixed(3)/g' "$file"
      sed -i '' 's/\.toFixed(2)t/.toFixed(3)t/g' "$file"
      sed -i '' 's/\.toFixed(2) 吨/.toFixed(3) 吨/g' "$file"
      sed -i '' 's/\.toFixed(2)}吨/.toFixed(3)}吨/g' "$file"
      sed -i '' 's/\.toFixed(2)}t/.toFixed(3)}t/g' "$file"

      ((TOTAL_FILES++))
    fi
  done
done

echo ""
echo "2️⃣  替换统计："
echo "================================"
echo "修改文件数: $TOTAL_FILES"
echo "备份位置: $BACKUP_DIR"
echo ""

echo "3️⃣  验证替换结果（前10个）："
echo "================================"
find src -type f \( -name "*.ts" -o -name "*.tsx" \) -exec grep -l "weight.*toFixed(3)\|capacity.*toFixed(3)" {} \; | head -10

echo ""
echo "=== 替换完成 ==="
echo ""
echo "💡 下一步："
echo "   1. 运行 npm run typecheck 检查类型错误"
echo "   2. 运行 npm run dev 启动前端验证效果"
echo "   3. 如需回滚: cp $BACKUP_DIR/* 到对应位置"
