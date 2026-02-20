#!/usr/bin/env bash
# double-helix-verify.sh — PreToolUse hook for Bash (git commit)
# 055号谱系：双螺旋架构的运行时实现
#
# 触发：PreToolUse(Bash)，仅拦截 git commit 命令
# 动作：将 staged diff 发送给 Gemini 验证，检查与谱系/定义的一致性
# 输出：allow（放行）或 block + 矛盾对象（打回）
#
# 熔断：Gemini 不可达时放行（052号相变：降级到单核模式）
# 死锁保护：连续 3 次 block 同一 commit → 放行 + 警告

set -euo pipefail

INPUT=$(cat)

# 只处理 Bash 工具
TOOL_NAME=$(echo "$INPUT" | python -c "import sys,json; print(json.load(sys.stdin).get('tool_name',''))" 2>/dev/null || echo "")
if [ "$TOOL_NAME" != "Bash" ]; then
  exit 0
fi

# 提取命令
COMMAND=$(echo "$INPUT" | python -c "import sys,json; print(json.load(sys.stdin).get('tool_input',{}).get('command',''))" 2>/dev/null || echo "")

# 只拦截 git commit（不拦截 git add, git push 等）
if ! echo "$COMMAND" | grep -qE "git commit"; then
  exit 0
fi

# 获取工作目录
CWD=$(echo "$INPUT" | python -c "import sys,json; print(json.load(sys.stdin).get('cwd','.'))" 2>/dev/null || echo ".")
cd "$CWD" 2>/dev/null || exit 0

# ─── 死锁保护 ───
HELIX_COUNTER=".chanlun/.helix-block-counter"
HELIX_LAST_HASH=".chanlun/.helix-last-diff-hash"

STAGED_DIFF=$(git diff --cached --stat 2>/dev/null || echo "")
if [ -z "$STAGED_DIFF" ]; then
  # 没有 staged changes，放行
  exit 0
fi

# 计算 diff 的 hash 用于死锁检测
DIFF_HASH=$(git diff --cached 2>/dev/null | md5sum | cut -d' ' -f1)
LAST_HASH=""
BLOCK_COUNT=0

if [ -f "$HELIX_LAST_HASH" ]; then
  LAST_HASH=$(cat "$HELIX_LAST_HASH" 2>/dev/null || echo "")
fi
if [ -f "$HELIX_COUNTER" ]; then
  BLOCK_COUNT=$(cat "$HELIX_COUNTER" 2>/dev/null || echo "0")
  BLOCK_COUNT=$((BLOCK_COUNT + 0))
fi

# 如果是同一个 diff 被 block 了 3 次，熔断放行
if [ "$DIFF_HASH" = "$LAST_HASH" ] && [ "$BLOCK_COUNT" -ge 3 ]; then
  rm -f "$HELIX_COUNTER" "$HELIX_LAST_HASH" 2>/dev/null || true
  exit 0
fi

# 如果是新的 diff，重置计数器
if [ "$DIFF_HASH" != "$LAST_HASH" ]; then
  BLOCK_COUNT=0
fi

# ─── Gemini 验证 ───
# 获取详细 diff（限制大小防止 token 爆炸）
FULL_DIFF=$(git diff --cached 2>/dev/null | head -500)
COMMIT_MSG=$(echo "$COMMAND" | grep -oP '(?<=-m\s["\x27]).*?(?=["\x27])' || echo "$COMMAND")

# 获取最近的谱系上下文
RECENT_SETTLED=""
if [ -d ".chanlun/genealogy/settled" ]; then
  RECENT_SETTLED=$(ls -t .chanlun/genealogy/settled/*.md 2>/dev/null | head -5 | while read f; do
    basename "$f" .md
  done | tr '\n' ', ' | sed 's/,$//')
fi

# 调用 Gemini verify
VERIFY_RESULT=$(PYTHONPATH=src python -c "
import os, sys, json

# 检查 API key
api_key = os.environ.get('GOOGLE_API_KEY', '')
if not api_key:
    print(json.dumps({'decision': 'allow', 'reason': 'no-api-key'}))
    sys.exit(0)

try:
    from newchan.gemini.modes import decide

    diff_text = '''$FULL_DIFF'''[:3000]
    commit_msg = '''$COMMIT_MSG'''
    recent = '''$RECENT_SETTLED'''

    subject = f'双螺旋验证：git commit 一致性检查'
    context = f'''你是新缠论系统的 pre-commit 验证器。请检查以下 commit 是否与谱系/定义一致。

Commit 消息: {commit_msg}

Staged diff (前500行):
{diff_text}

最近结算的谱系: {recent}

检查项:
1. 是否有与已结算谱系矛盾的变更？（例如：删除了被引用的定义，修改了已结算的原则）
2. 是否有明显的逻辑断裂？（例如：新增代码引用了不存在的概念）
3. 是否违反同一存在论（054号）？

如果没有发现矛盾，回复: CLEAN
如果发现矛盾，回复: CONTRADICTION: [具体描述]

注意：只报告严重的结构性矛盾，不要报告风格问题或小的改进建议。宁可放行也不要误报。'''

    result = decide(subject, context)
    response = result.response

    if 'CONTRADICTION' in response.upper() and 'CLEAN' not in response[:20].upper():
        # 提取矛盾描述
        contradiction = response
        print(json.dumps({
            'decision': 'block',
            'reason': contradiction[:500]
        }, ensure_ascii=False))
    else:
        print(json.dumps({'decision': 'allow', 'reason': 'gemini-verified-clean'}))

except Exception as e:
    # Gemini 不可达 → 052号相变：降级放行
    print(json.dumps({'decision': 'allow', 'reason': f'gemini-unreachable: {str(e)[:100]}'}))
" 2>/dev/null || echo '{"decision":"allow","reason":"script-error"}')

# 解析结果
DECISION=$(echo "$VERIFY_RESULT" | python -c "import sys,json; print(json.load(sys.stdin).get('decision','allow'))" 2>/dev/null || echo "allow")
REASON=$(echo "$VERIFY_RESULT" | python -c "import sys,json; print(json.load(sys.stdin).get('reason',''))" 2>/dev/null || echo "")

if [ "$DECISION" = "block" ]; then
  # 更新死锁计数器
  echo "$DIFF_HASH" > "$HELIX_LAST_HASH"
  echo $((BLOCK_COUNT + 1)) > "$HELIX_COUNTER"

  # 输出矛盾对象
  python -c "
import json
print(json.dumps({
    'decision': 'block',
    'reason': '''[双螺旋] Gemini 发现矛盾，commit 被拦截。\n\n矛盾对象:\n$REASON\n\n请修正后重新 commit。连续 block 3 次后自动熔断放行。'''
}, ensure_ascii=False))
"
  exit 0
fi

# 清理计数器（验证通过）
rm -f "$HELIX_COUNTER" "$HELIX_LAST_HASH" 2>/dev/null || true
exit 0
