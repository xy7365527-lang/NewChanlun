#!/usr/bin/env bash
# genealogy-write-guard.sh — 谱系写入守卫
#
# 功能：
# 1. 检查新写入的谱系文件是否 negates 一个 load_bearing: true 的条目
# 2. 如果是，拦截并提示（承重点保护机制，170号谱系）
#
# 用法：
#   bash scripts/hooks/genealogy-write-guard.sh <file_path>
#
# 在 Claude Code hook 中配置为 PreToolUse/PostToolUse 触发。
# 当写入 .chanlun/genealogy/settled/*.md 时自动触发。

set -euo pipefail

FILE="${1:-}"
if [[ -z "$FILE" ]]; then
    echo "用法: genealogy-write-guard.sh <file_path>"
    exit 0
fi

# 只检查 settled 目录下的 .md 文件
if [[ "$FILE" != *".chanlun/genealogy/settled/"* ]]; then
    exit 0
fi

if [[ ! -f "$FILE" ]]; then
    exit 0
fi

ROOT="$(git rev-parse --show-toplevel 2>/dev/null || pwd)"
DAG_FILE="$ROOT/.chanlun/genealogy/dag.yaml"

if [[ ! -f "$DAG_FILE" ]]; then
    exit 0
fi

# 提取文件中的 negates 字段值
NEGATES=""
# 支持 YAML frontmatter 格式: negates: ["xxx", "yyy"]
# 也支持 markdown 格式: **negates**: xxx
while IFS= read -r line; do
    # YAML frontmatter negates
    if echo "$line" | grep -qE '^\s*negates:\s*\['; then
        NEGATES=$(echo "$line" | sed 's/.*negates:\s*\[//;s/\].*//;s/"//g;s/,/ /g' | tr -s ' ')
        break
    fi
    # YAML frontmatter negates (single value)
    if echo "$line" | grep -qE '^\s*negates:\s*[0-9]'; then
        NEGATES=$(echo "$line" | sed 's/.*negates:\s*//' | tr -d '"' | tr -d "'")
        break
    fi
    # Markdown bold format: **negates**: xxx
    if echo "$line" | grep -qE '^\*\*negates\*\*:'; then
        NEGATES=$(echo "$line" | sed 's/.*\*\*negates\*\*:\s*//' | tr -d '"' | tr -d "'")
        break
    fi
done < <(head -30 "$FILE")

if [[ -z "$NEGATES" ]]; then
    # Guard 4（177号）：topo_effect advisory 检测
    # 检查新写入的谱系文件是否含结构化 topo_effect（type:target:scope）
    # 仅 advisory 输出，不执行任何副作用——拓扑操作在 RTAS 循环中由 ceremony_scan 驱动
    TOPO_EFFECT=""
    while IFS= read -r line; do
        if echo "$line" | grep -qE '^\s*topo_effect:\s*"?(freeze|split|sever):[^:]+:[^:"]+'; then
            TOPO_EFFECT=$(echo "$line" | sed 's/.*topo_effect:\s*//;s/"//g' | tr -d "'" | xargs)
            break
        fi
    done < <(head -30 "$FILE")

    if [[ -n "$TOPO_EFFECT" ]]; then
        # 检查是否已有 topo_executed_at（已执行则不提示）
        HAS_EXECUTED=$(head -30 "$FILE" | grep -c 'topo_executed_at:' || true)
        if [[ "$HAS_EXECUTED" -eq 0 ]]; then
            FILE_ID=$(head -30 "$FILE" | grep -oE 'id:\s*["\x27]?[0-9]+' | grep -oE '[0-9]+' | head -1)
            echo "[topology-advisory/${FILE_ID:-unknown}] 谱系含未执行的 topo_effect: ${TOPO_EFFECT}。将在下次 RTAS 循环（ceremony_scan）中自动执行。"
        fi
    fi
    exit 0
fi

# 检查每个 negates 目标是否是 load_bearing
for target in $NEGATES; do
    # 提取编号部分（支持 "158-downstream-1" 格式，取编号部分 158）
    target_id=$(echo "$target" | grep -oE '^[0-9]+' || true)
    if [[ -z "$target_id" ]]; then
        continue
    fi

    # 在 dag.yaml 中查找该节点是否有 load_bearing: true
    # 使用 python 做 YAML 解析（比 grep 更可靠）
    IS_LOAD_BEARING=$(python3 -c "
import yaml, sys
with open('$DAG_FILE', encoding='utf-8') as f:
    dag = yaml.safe_load(f)
for node in dag.get('nodes', []):
    if str(node.get('id', '')) == '$target_id':
        if node.get('load_bearing'):
            print('true')
            sys.exit(0)
print('false')
" 2>/dev/null || echo "false")

    if [[ "$IS_LOAD_BEARING" == "true" ]]; then
        echo "=========================================="
        echo "⚠ 承重点保护拦截"
        echo "=========================================="
        echo ""
        echo "文件: $FILE"
        echo "negates 目标: $target (id=$target_id)"
        echo "该目标在 dag.yaml 中标记为 load_bearing: true"
        echo ""
        echo "承重点条目（170号谱系）是递归产生的不可否定的拓扑产物。"
        echo "否定承重点可能导致结构崩塌。"
        echo ""
        echo "如果确实需要否定承重点，请："
        echo "1. 使用 /escalate 提交矛盾上浮报告"
        echo "2. 经编排者确认后，在谱系中显式记录否定理由"
        echo "3. 在 dag.yaml 中更新 load_bearing 状态"
        echo ""
        echo "=========================================="
        exit 1
    fi
done

exit 0
