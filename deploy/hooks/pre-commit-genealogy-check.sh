#!/bin/bash
# pre-commit hook: 概念层变更必须有对应谱系条目
# 谱系014 runtime 强制层种子
#
# 逻辑：
#   1. 检查 staged files 是否包含概念层文件
#   2. 如果有，检查是否同时 staged 了 .chanlun/genealogy/ 下的文件
#   3. 如果概念层变了但谱系没动，阻塞提交
#
# 跳过：git commit --no-verify（仅用于确认不涉及概念发现的变更）

# 概念层文件模式（变更这些文件意味着概念层有动作）
CONCEPT_PATTERNS=(
    ".chanlun/definitions/"
    ".claude/agents/"
    ".claude/skills/meta-orchestration/SKILL.md"
    "deploy/agents/"
    "deploy/skills/meta-orchestration/SKILL.md"
    "docs/spec/"
)

# 获取所有 staged 文件
STAGED_FILES=$(git diff --cached --name-only)

# 检查是否有概念层变更
CONCEPT_CHANGED=0
CONCEPT_FILES=""
for pattern in "${CONCEPT_PATTERNS[@]}"; do
    MATCHES=$(echo "$STAGED_FILES" | grep "^${pattern}" || true)
    if [ -n "$MATCHES" ]; then
        CONCEPT_CHANGED=1
        CONCEPT_FILES="${CONCEPT_FILES}${MATCHES}\n"
    fi
done

# 没有概念层变更，放行
if [ "$CONCEPT_CHANGED" -eq 0 ]; then
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
