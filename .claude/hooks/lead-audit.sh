#!/bin/bash
# lead-audit.sh — Lead 操作拓扑效力触发器（153号修复：黑名单反转）
#
# 发生史：032号→088号→147号→153号
#   032号：lead-permissions.sh deny 列表（导致项目级死锁）
#   088号：拓扑异常对象化审计（从阻断改为记录）
#   147号：权限→拓扑转化（"修改即否定即拓扑操作"）
#   153号：白名单→黑名单反转（白名单无法穷举 Lead 合法操作，导致恶性循环）
#
# 覆盖缺口（407号推论6，平台限制）：
#   本 hook 的 matcher 是 Bash（PostToolUse），只检测 Write/Edit/Bash 操作。
#   Lead 角色越界通常从 Read 代码文件开始（读取→分析→提出修复），
#   但 Read 工具在 Claude Code 平台当前不支持 PostToolUse hook。
#   因此 Lead 的 Read 操作（代码文件阅读）不触发任何 hook 检测。
#   缓解措施：session-start-ceremony.sh 注入角色边界锚点（407号修复1）。
#
# 设计原则（153号修复）：
#   1. 默认放行——Lead 的大部分操作是合法的
#   2. 只有明确的基因组修改才记录拓扑效力
#   3. 记录写入 session 级汇总，不再逐次写入 pattern-buffer（消除恶性循环）
#   4. 子工位操作静默通过（不是 Lead 的拓扑行为）
#
# 黑名单（产生拓扑效力的操作）：
#   - CLAUDE.md 修改（基因组根节点）
#   - .claude/rules/ 修改（蜂群语法规则）
#   - .claude/skills/ 修改（能力声明）
#   - .claude/hooks/ 修改（运行时守卫）
#   - .claude/agents/ 修改（工位定义）
#
# 其他一切操作默认放行，不记录。

set -euo pipefail

resolve_python() {
  local bin
  for bin in python3 python; do
    if command -v "$bin" >/dev/null 2>&1 && "$bin" -c "import sys" >/dev/null 2>&1; then
      echo "$bin"
      return 0
    fi
  done
  return 1
}

PYTHON_BIN="$(resolve_python || true)"
[ -n "$PYTHON_BIN" ] || exit 0

INPUT=$(cat)

# 子工位操作静默通过
if [ -n "${CLAUDE_AGENT_NAME:-}" ]; then
  exit 0
fi

# 单次 Python 调用完成所有逻辑（消除多次进程启动开销）
"$PYTHON_BIN" -c "
import sys, json, re, os, hashlib

data = json.loads(sys.stdin.read())
tool_name = data.get('tool_name', '')

# 只处理 Write/Edit/Bash
if tool_name not in ('Write', 'Edit', 'Bash'):
    sys.exit(0)

ti = data.get('tool_input', {})
file_path = ti.get('file_path', ti.get('command', ''))[:200]
fp = file_path.replace('\\\\', '/').replace('\\\\\\\\', '/')

# --- 黑名单检测：只有基因组修改才产生拓扑效力 ---
is_genome_modification = False

if tool_name in ('Write', 'Edit'):
    # 基因组文件模式
    genome_patterns = [
        r'CLAUDE\.md$',
        r'[\\/]\.claude[\\/]rules[\\/]',
        r'[\\/]\.claude[\\/]skills[\\/]',
        r'[\\/]\.claude[\\/]hooks[\\/]',
        r'[\\/]\.claude[\\/]agents[\\/]',
    ]
    for pat in genome_patterns:
        if re.search(pat, fp, re.IGNORECASE):
            is_genome_modification = True
            break

elif tool_name == 'Bash':
    # Bash 修改基因组文件（sed/写入操作指向基因组路径）
    # 大部分 Bash 是读取/诊断/git 操作，默认放行
    genome_write_patterns = [
        r'>\s*.*CLAUDE\.md',
        r'>\s*.*\.claude/(rules|skills|hooks|agents)/',
        r'sed\s+-i.*CLAUDE\.md',
        r'sed\s+-i.*\.claude/(rules|skills|hooks|agents)/',
    ]
    for pat in genome_write_patterns:
        if re.search(pat, fp):
            is_genome_modification = True
            break

if not is_genome_modification:
    # 非基因组操作，静默放行
    sys.exit(0)

# --- 基因组修改：输出警告（不写 pattern-buffer，只发 systemMessage） ---
msg = f'[lead-audit/153] Lead 直接修改基因组文件: {fp[:100]}。拓扑效力事件，session 结束时汇总审查。'
print(json.dumps({'systemMessage': msg}, ensure_ascii=False))
" <<< "$INPUT" 2>/dev/null

exit 0
