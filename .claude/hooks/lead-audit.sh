#!/bin/bash
# lead-audit.sh — Lead 拓扑异常对象化审计（088号谱系：032号重设计）
#
# 设计原则（Gemini decide 选项D）：
#   1. Lead 保留全权限——不再使用 settings.local.json deny 列表
#   2. Lead 违规直接执行 Write/Edit/Bash 时，不阻断，
#      而是将行为实体化为"拓扑异常对象"写入 pattern-buffer
#   3. meta-observer 异步读取异常后发起结构性否定
#   4. 不使用阈值（005b号合规）——异常对象一旦生成即构成背驰信号
#   5. 视差 Gap 由 git + ESC 兜底（069号结构条件）
#
# 前身：lead-permissions.sh（032号"神圣疯狂"deny 列表——已证明导致项目级死锁）
# 谱系：032→088号
#
# 白名单（Lead 在 ceremony/编排中的合法直接操作，不写入 pattern-buffer）：
#   - git 操作（git add/commit/push/fetch/rebase/show/diff/log/status 等）
#   - dag.yaml 修改（.chanlun/genealogy/dag.yaml）
#   - 谱系文件写入（.chanlun/genealogy/ 目录下任意文件）
#   - downstream-action-overrides.yaml 修改
#   - session 文件写入（.chanlun/sessions/）
#   - ceremony_scan.py 运行
#   - dispatch-dag.yaml 修改
#   - 读取/检查 pattern-buffer.yaml 自身的 Bash 命令
#   - 对 scripts/ 目录下脚本的维护操作
#
# 触发方式：PostToolUse hook（Lead 层级的 Write/Edit/Bash 调用后）
# 输出：JSON（allow + systemMessage 审计警告）

set -euo pipefail

INPUT=$(cat)

# 提取 tool 名
TOOL_NAME=$(echo "$INPUT" | python -c "import sys,json; print(json.load(sys.stdin).get('tool_name',''))" 2>/dev/null || echo "")

# 只审计 Write/Edit/Bash（Lead 应委派给工位的操作）
case "$TOOL_NAME" in
  Write|Edit|Bash) ;;
  *) exit 0 ;;
esac

# 检测是否为蜂群内的子工位调用（子工位调用不审计——它们就是执行者）
# 方法：检查环境变量 CLAUDE_AGENT_NAME（子工位有此变量，Lead 没有）
# 如果存在且非空，说明是子工位，静默通过
if [ -n "${CLAUDE_AGENT_NAME:-}" ]; then
  exit 0
fi

# 提取操作目标文件/命令
FILE_PATH=$(echo "$INPUT" | python -c "
import sys,json
d = json.load(sys.stdin)
ti = d.get('tool_input', {})
print(ti.get('file_path', ti.get('command', ''))[:200])
" 2>/dev/null || echo "unknown")

# --- 白名单检查 ---
# 如果操作匹配白名单，静默通过（不写入 pattern-buffer）
IS_WHITELISTED=$(python -c "
import sys, re

tool_name = sys.argv[1]
file_path = sys.argv[2]

fp = file_path.replace('\\\\', '/').replace('\\\\\\\\', '/')

# ---- Bash 白名单 ----
if tool_name == 'Bash':
    cmd = fp  # file_path 字段存的是命令

    # git 操作（git add/commit/push/fetch/rebase/show/diff/log/status/stash 等）
    if re.search(r'\bgit\s+(add|commit|push|fetch|rebase|show|diff|log|status|stash|pull|checkout|branch|merge|tag|remote|reset|clean|describe|rev-parse|ls-files|shortlog|blame)\b', cmd):
        print('yes')
        sys.exit(0)

    # ceremony_scan.py 运行
    if 'ceremony_scan.py' in cmd:
        print('yes')
        sys.exit(0)

    # dag_add_node.py 脚本（维护 dag 的合法工具）
    if 'dag_add_node.py' in cmd or 'dag_add_edge.py' in cmd:
        print('yes')
        sys.exit(0)

    # 读取/检查 pattern-buffer.yaml 自身
    if 'pattern-buffer.yaml' in cmd:
        print('yes')
        sys.exit(0)

    # downstream_audit 脚本
    if 'downstream_audit' in cmd:
        print('yes')
        sys.exit(0)

    # dag.yaml 相关读取
    if 'dag.yaml' in cmd and re.search(r'\byaml\.safe_load\b|\byaml\.load\b', cmd):
        print('yes')
        sys.exit(0)

# ---- Write/Edit 白名单 ----
if tool_name in ('Write', 'Edit'):
    # dag.yaml
    if re.search(r'[\\\\/]genealogy[\\\\/]dag\.yaml', fp, re.IGNORECASE):
        print('yes')
        sys.exit(0)

    # 谱系文件（.chanlun/genealogy/ 目录下任意文件）
    if re.search(r'[\\\\/]\.chanlun[\\\\/]genealogy[\\\\/]', fp, re.IGNORECASE):
        print('yes')
        sys.exit(0)

    # downstream-action-overrides.yaml
    if 'downstream-action-overrides.yaml' in fp:
        print('yes')
        sys.exit(0)

    # session 文件（.chanlun/sessions/）
    if re.search(r'[\\\\/]\.chanlun[\\\\/]sessions[\\\\/]', fp, re.IGNORECASE):
        print('yes')
        sys.exit(0)

    # dispatch-dag.yaml
    if 'dispatch-dag.yaml' in fp:
        print('yes')
        sys.exit(0)

print('no')
" "$TOOL_NAME" "$FILE_PATH" 2>/dev/null || echo "no")

if [ "$IS_WHITELISTED" = "yes" ]; then
  # 白名单操作：静默通过，不写入 pattern-buffer
  python -c "import json; print(json.dumps({'decision': 'allow'}))" 2>/dev/null
  exit 0
fi

# --- Lead 直接执行 Write/Edit/Bash（非白名单）：生成拓扑异常对象 ---

CWD=$(echo "$INPUT" | python -c "import sys,json; print(json.load(sys.stdin).get('cwd','.'))" 2>/dev/null || echo ".")
cd "$CWD" 2>/dev/null || true

PATTERN_FILE=".chanlun/pattern-buffer.yaml"
TIMESTAMP=$(date -u +"%Y-%m-%dT%H:%M:%SZ" 2>/dev/null || date +"%Y-%m-%dT%H:%M:%S")

# 写入拓扑异常对象到 pattern-buffer
python -c "
import sys, os, hashlib

pattern_file = sys.argv[1]
timestamp = sys.argv[2]
tool_name = sys.argv[3]
file_path = sys.argv[4]

# 生成异常对象 ID
sig = f'lead-direct-{tool_name}:{file_path}'
pat_hash = hashlib.md5(f'{timestamp}-{sig}'.encode()).hexdigest()[:8]

# 确保 pattern-buffer 存在
if not os.path.isfile(pattern_file):
    with open(pattern_file, 'w', encoding='utf-8') as f:
        f.write('# 模式缓冲区——谱系的生成态前置\n')
        f.write('# 043号谱系：自生长回路\n')
        f.write('# Status 枚举: observed → candidate → settled → promoted/rejected\n')
        f.write('version: \"1.0\"\n')
        f.write('patterns: []\n')

# 追加拓扑异常对象
with open(pattern_file, 'r', encoding='utf-8') as f:
    content = f.read()

# 检查是否已有 patterns: [] 占位
if 'patterns: []' in content:
    content = content.replace('patterns: []', 'patterns:')

# 兜底：如果没有 patterns 键，补一个
if 'patterns:' not in content:
    if content and not content.endswith('\n'):
        content += '\n'
    content += 'patterns:\n'

if content and not content.endswith('\n'):
    content += '\n'

def yaml_quote(text):
    return str(text).replace('\\\\', '\\\\\\\\').replace('\"', '\\\\\"').replace('\n', ' ')

safe_sig = yaml_quote(sig)
safe_timestamp = yaml_quote(timestamp)
safe_tool_name = yaml_quote(tool_name)

# 追加异常条目
entry = (
    f'  - id: \"anomaly-{pat_hash}\"\n'
    f'    signature: \"{safe_sig}\"\n'
    f'    frequency: 1\n'
    f'    first_seen: \"{safe_timestamp}\"\n'
    f'    last_seen: \"{safe_timestamp}\"\n'
    f'    sources: [\"{safe_timestamp}\"]\n'
    f'    description: \"Lead 直接执行 {safe_tool_name}（应委派工位）——拓扑异常对象\"\n'
    f'    status: \"candidate\"\n'
    f'    anomaly_type: \"lead_direct_execution\"\n'
)

with open(pattern_file, 'w', encoding='utf-8') as f:
    f.write(content + entry)
" "$PATTERN_FILE" "$TIMESTAMP" "$TOOL_NAME" "$FILE_PATH" 2>/dev/null || true

# 输出审计警告（不阻断）
MSG="[lead-audit/088] Lead 直接执行 ${TOOL_NAME}（应委派工位）。拓扑异常对象已写入 pattern-buffer。meta-observer 将异步审查。"

MSG="$MSG" python -c "
import json, os
msg = os.environ['MSG']
print(json.dumps({'decision': 'allow', 'reason': msg}, ensure_ascii=False))
" 2>/dev/null

exit 0
