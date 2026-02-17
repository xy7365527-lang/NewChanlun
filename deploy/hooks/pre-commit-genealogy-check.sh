#!/bin/bash
# pre-commit hook: 概念层变更必须有对应谱系条目
#
# 逻辑：
#   1. 从 .chanlun/concept-paths 读取概念层路径（按需索引，非硬编码）
#   2. 检查 staged files 是否匹配
#   3. 如果概念层变了但谱系没动，阻塞提交
#
# hook 本身 O(1)（永远不变），路径注册 O(n)（在配置文件中声明）
# 跳过：git commit --no-verify

CONCEPT_PATHS_FILE=".chanlun/concept-paths"

# 配置文件不存在则放行（项目尚未初始化）
if [ ! -f "$CONCEPT_PATHS_FILE" ]; then
    exit 0
fi

# 读取概念层路径（忽略空行和注释）
PATTERNS=()
while IFS= read -r line; do
    line=$(echo "$line" | sed 's/#.*//; s/^[[:space:]]*//; s/[[:space:]]*$//')
    [ -n "$line" ] && PATTERNS+=("$line")
done < "$CONCEPT_PATHS_FILE"

# 无注册路径则放行
if [ ${#PATTERNS[@]} -eq 0 ]; then
    exit 0
fi

# 获取所有 staged 文件
STAGED_FILES=$(git diff --cached --name-only)

# 检查是否有概念层变更
CONCEPT_FILES=""
for pattern in "${PATTERNS[@]}"; do
    MATCHES=$(echo "$STAGED_FILES" | grep "^${pattern}" || true)
    [ -n "$MATCHES" ] && CONCEPT_FILES="${CONCEPT_FILES}${MATCHES}\n"
done

# 没有概念层变更，放行
if [ -z "$CONCEPT_FILES" ]; then
    exit 0
fi

# 有概念层变更，检查是否有谱系条目
if echo "$STAGED_FILES" | grep -q "^\.chanlun/genealogy/"; then
    exit 0
fi

# 概念层变更但没有谱系条目，阻塞
echo ""
echo "┌─────────────────────────────────────────────┐"
echo "│  ⚠ 概念层变更检测到，未发现对应谱系条目     │"
echo "└─────────────────────────────────────────────┘"
echo ""
echo "已变更的概念层文件:"
echo -e "$CONCEPT_FILES" | sed '/^$/d' | sed 's/^/  /'
echo ""
echo "请先写入谱系记录到 .chanlun/genealogy/ 再提交。"
echo ""
echo "如果此次变更确认不涉及概念发现（纯格式/纯技术），"
echo "可用 --no-verify 跳过本检查。"
exit 1
