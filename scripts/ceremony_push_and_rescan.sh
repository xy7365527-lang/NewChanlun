#!/usr/bin/env bash
# ceremony 步骤 7-10 原子链
# 用法：bash scripts/ceremony_push_and_rescan.sh [commit_message]
#
# 执行序列：
# 1. git add 已知文件类型（.py .md .yaml .sh .json .jsonl）
# 2. git commit（如果有 staged changes）
# 3. git push（失败则 fetch+rebase+push）
# 4. python scripts/ceremony_scan.py --phase rescan
# 5. 输出 rescan JSON 结果（供 Lead 解析）
#
# 整个序列在一次 Bash 调用中完成，消除 LLM 决策间隙。
# 不使用 set -e：需要处理 push 失败后的 rebase 重试。

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
cd "$REPO_ROOT" || { echo '{"error": "无法进入仓库根目录"}'; exit 1; }

COMMIT_MSG="${1:-chore: ceremony atomic chain commit}"

# --- Step 1: 写入 ceremony 状态（步骤7：push 阶段）---
python "$SCRIPT_DIR/ceremony_state.py" write 7 push

# --- Step 2: git add 已知文件类型（排除 tmp/）---
git add -- '*.py' '*.md' '*.yaml' '*.sh' '*.json' '*.jsonl' 2>/dev/null
git reset HEAD -- tmp/ 2>/dev/null

# --- Step 2: git commit（仅当有 staged changes 时）---
COMMITTED=false
if ! git diff --cached --quiet 2>/dev/null; then
    if git commit -m "$COMMIT_MSG"; then
        COMMITTED=true
    else
        echo '{"error": "git commit 失败", "phase": "commit"}'
        exit 1
    fi
fi

# --- Step 3: git push（失败则 fetch+rebase+push）---
PUSH_OK=false
if git push 2>/dev/null; then
    PUSH_OK=true
else
    echo "push 失败，尝试 stash+fetch+rebase+push..." >&2
    git stash -u
    if git fetch origin && git rebase "origin/$(git branch --show-current)" && git push; then
        PUSH_OK=true
        git stash pop 2>/dev/null
    else
        git stash pop 2>/dev/null
        echo '{"error": "git push 失败（含 rebase 重试）", "phase": "push"}'
        exit 1
    fi
fi

# --- Step 4: 更新 ceremony 状态（步骤8：rescan 阶段）---
python "$SCRIPT_DIR/ceremony_state.py" write 8 rescan

# --- Step 5: ceremony rescan ---
RESCAN_TMPFILE=$(mktemp)
python "$SCRIPT_DIR/ceremony_scan.py" --phase rescan >"$RESCAN_TMPFILE" 2>&1
RESCAN_EXIT=$?

if [ $RESCAN_EXIT -ne 0 ]; then
    # 用 python 安全地构造错误 JSON（通过 stdin 传入 detail）
    python -c "
import json, sys
detail = sys.stdin.read()
print(json.dumps({'error': 'ceremony_scan.py --phase rescan 失败', 'phase': 'rescan', 'detail': detail}, ensure_ascii=False))
" < "$RESCAN_TMPFILE"
    rm -f "$RESCAN_TMPFILE"
    exit 1
fi

# --- Step 5: 输出结果 ---
# 通过环境变量 + stdin 传递数据给 python，避免 shell 注入
ATOMIC_COMMITTED="$COMMITTED" \
ATOMIC_PUSH_OK="$PUSH_OK" \
ATOMIC_COMMIT_MSG="$COMMIT_MSG" \
python -c "
import json, sys, os

raw = sys.stdin.read()
try:
    data = json.loads(raw)
except Exception:
    data = {'raw_output': raw}

data['atomic_chain'] = {
    'committed': os.environ.get('ATOMIC_COMMITTED') == 'true',
    'push_ok': os.environ.get('ATOMIC_PUSH_OK') == 'true',
    'commit_msg': os.environ.get('ATOMIC_COMMIT_MSG', ''),
}
print(json.dumps(data, ensure_ascii=False, indent=2))
" < "$RESCAN_TMPFILE"

rm -f "$RESCAN_TMPFILE"
