#!/bin/bash
# SessionStart Hook — 自动 ceremony（热启动 bootstrap）
#
# 每次会话开始时自动执行。扫描 session 文件和定义/谱系状态，
# 生成差异报告注入系统消息，让 CC 直接进入蜂群循环。
#
# 效果等同于用户手动输入 /ceremony，但零人工干预。

set -euo pipefail

input=$(cat)
cwd=$(echo "$input" | python -c "import sys,json; print(json.loads(sys.stdin.read()).get('cwd', '.'))" 2>/dev/null || echo ".")
cd "$cwd" 2>/dev/null || true

# ─── ceremony 相位标记 ───
# 创建 ceremony-in-progress 标记，由 ceremony-completion-guard (Stop hook) 检查
# 057号谱系：flag 由框架自动管理（stop-guard 第二次 block 时自动清除）
mkdir -p .chanlun 2>/dev/null || true
touch .chanlun/.ceremony-in-progress
rm -f .chanlun/.stop-guard-counter .chanlun/.ceremony-blocked-once 2>/dev/null || true

# ─── 安全 JSON 输出 ───
# 用 python json.dumps 保证转义正确，避免裸拼接导致畸形 JSON
emit_json() {
    local msg="$1"
    python -c "
import json, sys
msg = sys.argv[1]
print(json.dumps({'continue': True, 'suppressOutput': False, 'systemMessage': msg}, ensure_ascii=False))
" "$msg"
}

# ─── 检测模式 ───
SESSION_FILE=""
if [ -d ".chanlun/sessions" ]; then
    SESSION_FILE=$(ls -t .chanlun/sessions/*-session.md 2>/dev/null | head -1 || true)
fi

if [ -z "$SESSION_FILE" ]; then
    # 冷启动：无 session，输出基本状态
    DEF_COUNT=0
    [ -d ".chanlun/definitions" ] && DEF_COUNT=$(ls .chanlun/definitions/*.md 2>/dev/null | wc -l)
    SETTLED=0
    [ -d ".chanlun/genealogy/settled" ] && SETTLED=$(ls .chanlun/genealogy/settled/*.md 2>/dev/null | wc -l)
    PENDING=0
    [ -d ".chanlun/genealogy/pending" ] && PENDING=$(ls .chanlun/genealogy/pending/*.md 2>/dev/null | wc -l)

    MSG="[Ceremony/冷启动] 定义${DEF_COUNT}条 | 谱系${SETTLED}已结算/${PENDING}生成态 | 请执行完整 /ceremony 确认定义基底"
    emit_json "$MSG"
    exit 0
fi

# ─── 热启动：从 session 恢复 ───
SESSION_TIME=$(grep '^\*\*时间\*\*' "$SESSION_FILE" 2>/dev/null | sed 's/.*: *//' || echo "?")
SESSION_COMMIT=$(grep '^\*\*最新提交\*\*' "$SESSION_FILE" 2>/dev/null | sed 's/^\*\*最新提交\*\*: *//' | cut -d' ' -f1 || echo "?")

# 当前状态
CURRENT_COMMIT=$(git log --oneline -1 2>/dev/null || echo "unknown")
GIT_BRANCH=$(git branch --show-current 2>/dev/null || echo "unknown")

# 定义差异（快速版：只比较版本号）
DEF_DIFF=""
CHANGED=0
if [ -d ".chanlun/definitions" ]; then
    for f in .chanlun/definitions/*.md; do
        [ -f "$f" ] || continue
        name=$(basename "$f" .md)
        cur_ver=$(grep -m1 '^\*\*版本\*\*' "$f" 2>/dev/null | sed 's/.*: *//; s/\*\*//g' || echo "?")
        # 从 session 表格提取该定义的版本（第2列）
        ses_ver=$(grep "| ${name} |" "$SESSION_FILE" 2>/dev/null | awk -F'|' '{gsub(/^ *| *$/,"",$3); print $3}' | head -1 || true)
        [ -z "$ses_ver" ] && ses_ver="?"
        if [ "$cur_ver" = "$ses_ver" ]; then
            mark="="
        else
            mark="↑"
            CHANGED=$((CHANGED + 1))
        fi
        DEF_DIFF="${DEF_DIFF} ${name}(${mark}${cur_ver})"
    done
fi

# 谱系统计
SETTLED=0
[ -d ".chanlun/genealogy/settled" ] && SETTLED=$(ls .chanlun/genealogy/settled/*.md 2>/dev/null | wc -l)
PENDING=0
PENDING_LIST=""
if [ -d ".chanlun/genealogy/pending" ]; then
    for f in .chanlun/genealogy/pending/*.md; do
        [ -f "$f" ] || continue
        PENDING=$((PENDING + 1))
        PENDING_LIST="${PENDING_LIST} $(basename "$f" .md)"
    done
fi

# 中断点
INTERRUPTS=$(sed -n '/^## 中断点$/,/^## /{ /^## 中断点$/d; /^## /d; p; }' "$SESSION_FILE" 2>/dev/null | head -5 | tr '\n' ' ' || echo "无")

# 构建消息
MSG="[Ceremony/热启动L2] 恢复自:${SESSION_FILE} (${SESSION_TIME}) | 分支:${GIT_BRANCH} | session提交:${SESSION_COMMIT} | 当前:${CURRENT_COMMIT} | 定义变更:${CHANGED}条 | 谱系:${SETTLED}settled/${PENDING}pending | 中断点:${INTERRUPTS} | ⚡自动进入蜂群循环：先评估可并行工位数(≥2即拉蜂群)，扫描代码/规范/谱系状态确定本轮工作目标"

emit_json "$MSG"
