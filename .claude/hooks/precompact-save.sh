#!/bin/bash
# PreCompact Hook — L1 热启动状态快照
#
# 当 Claude Code 上下文即将被压缩时，自动保存蜂群状态到 session 文件。
# 用于 L1 级别恢复：compact 后从 session 文件重建工作上下文。
#
# 输入：JSON (stdin) — session_id, transcript_path, cwd 等
# 输出：JSON (stdout) — continue=true, systemMessage=状态摘要

set -euo pipefail

# 读取 stdin 的 JSON 输入
input=$(cat)
cwd=$(echo "$input" | python3 -c "import sys,json; print(json.loads(sys.stdin.read()).get('cwd', '.'))" 2>/dev/null || echo ".")

# 确保在项目目录内
cd "$cwd" 2>/dev/null || cd /home/user/NewChanlun

# 确保 sessions 目录存在
mkdir -p .chanlun/sessions

TIMESTAMP=$(date +"%Y-%m-%d-%H%M")
SNAPSHOT_FILE=".chanlun/sessions/${TIMESTAMP}-precompact.md"

# 采集定义状态
DEFINITIONS=""
if [ -d ".chanlun/definitions" ]; then
    for f in .chanlun/definitions/*.md; do
        [ -f "$f" ] || continue
        name=$(basename "$f" .md)
        version=$(grep -m1 '^\*\*版本\*\*' "$f" 2>/dev/null | sed 's/.*: *//' || echo "?")
        status=$(grep -m1 '^\*\*状态\*\*' "$f" 2>/dev/null | sed 's/.*: *//' || echo "?")
        DEFINITIONS="${DEFINITIONS}\n| ${name} | ${version} | ${status} |"
    done
fi

# 采集谱系状态
PENDING_COUNT=0
SETTLED_COUNT=0
PENDING_LIST=""
if [ -d ".chanlun/genealogy/pending" ]; then
    for f in .chanlun/genealogy/pending/*.md; do
        [ -f "$f" ] || continue
        PENDING_COUNT=$((PENDING_COUNT + 1))
        id=$(basename "$f" .md)
        PENDING_LIST="${PENDING_LIST}\n- ${id}"
    done
fi
if [ -d ".chanlun/genealogy/settled" ]; then
    SETTLED_COUNT=$(find .chanlun/genealogy/settled -name "*.md" 2>/dev/null | wc -l)
fi

# 采集 git 状态
GIT_BRANCH=$(git branch --show-current 2>/dev/null || echo "unknown")
GIT_COMMIT=$(git log --oneline -1 2>/dev/null || echo "unknown")
GIT_DIRTY=$(git diff --stat 2>/dev/null | tail -1)
[ -z "$GIT_DIRTY" ] && GIT_DIRTY="clean"

# 查找最近的 session 文件（非 precompact 类型）
LATEST_SESSION=""
for f in $(ls -t .chanlun/sessions/*.md 2>/dev/null | head -5); do
    case "$f" in
        *precompact*) continue ;;
        *) LATEST_SESSION="$f"; break ;;
    esac
done

# 写入快照
cat > "$SNAPSHOT_FILE" << SNAPSHOT_EOF
# PreCompact 状态快照

**时间**: ${TIMESTAMP}
**分支**: ${GIT_BRANCH}
**最新提交**: ${GIT_COMMIT}
**工作树**: ${GIT_DIRTY}

## 定义基底
| 名称 | 版本 | 状态 |
|------|------|------|$(echo -e "$DEFINITIONS")

## 谱系状态
- 生成态: ${PENDING_COUNT} 个${PENDING_LIST:+$(echo -e "$PENDING_LIST")}
- 已结算: ${SETTLED_COUNT} 个

## 上次手动 session
${LATEST_SESSION:-"无"}

## 恢复指引
1. 读取此文件获取状态快照
2. 读取上次手动 session 获取详细中断点
3. 按蜂群循环运作原则（CLAUDE.md 原则9）启动下一轮工位
SNAPSHOT_EOF

# 生成系统消息
MSG="[PreCompact] 状态已保存: ${SNAPSHOT_FILE} | 定义$(echo -e "$DEFINITIONS" | grep -c '|')条 | 谱系${PENDING_COUNT}生成态/${SETTLED_COUNT}已结算"

# 输出 JSON 响应
echo "{\"continue\": true, \"suppressOutput\": false, \"systemMessage\": \"${MSG}\"}"
