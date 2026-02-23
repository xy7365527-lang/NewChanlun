#!/bin/bash
# lead-audit.sh — Lead 拓扑异常对象化审计（088号谱系：032号重设计）
#
# 设计原则（Gemini decide 选项D + 173号-1 方法论反转）：
#   1. Lead 保留全权限——不再使用 settings.local.json deny 列表
#   2. Lead 违规直接执行 Write/Edit/Bash 时，不阻断，
#      而是将行为实体化为"拓扑异常对象"写入 pattern-buffer
#   3. meta-observer 异步读取异常后发起结构性否定
#   4. 不使用阈值（005b号合规）——异常对象一旦生成即构成背驰信号
#   5. 视差 Gap 由 git + ESC 兜底（069号结构条件）
#
# 前身：lead-permissions.sh（032号"神圣疯狂"deny 列表——已证明导致项目级死锁）
# 谱系：032→088号→173号-1（白名单→黑名单方法论反转）
#
# 173号-1 方法论反转：
#   旧逻辑："默认否定，白名单放行"——Lead 大部分操作都是合法的，白名单不断膨胀
#   新逻辑："默认放行，黑名单记录"——只有少数操作需要记录为拓扑异常
#
# 黑名单（Lead 直接执行时记录为拓扑异常的操作）：
#   - 直接修改源代码文件（src/ 目录）
#   - 直接修改定义文件（.chanlun/definitions/）
#   - 直接修改测试文件（tests/）
#   - 直接修改 hook 脚本（.claude/hooks/）
#   - 直接修改 agent 配置（.claude/agents/）
#   - 执行破坏性 shell 命令（rm -rf 等）
#
# 触发方式：PostToolUse hook（Lead 层级的 Write/Edit/Bash 调用后）
# 输出：JSON（allow + systemMessage 审计警告）——仅黑名单命中时

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

# --- 黑名单检查（173号-1 方法论反转）---
# 默认放行。只有命中黑名单时才记录拓扑异常。
IS_BLACKLISTED=$(python -c "
import sys, re

tool_name = sys.argv[1]
file_path = sys.argv[2]

fp = file_path.replace('\\\\', '/').replace('\\\\\\\\', '/')

# ---- Bash 黑名单 ----
if tool_name == 'Bash':
    cmd = fp  # file_path 字段存的是命令

    # 破坏性命令（rm -rf, rm -r 非 .venv/.git 目标）
    if re.search(r'\brm\s+-(rf|fr)\s+(?!.*\.(venv|git))', cmd):
        print('yes')
        sys.exit(0)

    # 不匹配黑名单：默认放行
    print('no')
    sys.exit(0)

# ---- Write/Edit 黑名单 ----
if tool_name in ('Write', 'Edit'):
    # 源代码文件（src/ 目录）
    if re.search(r'[\\\\/]src[\\\\/]', fp):
        print('yes')
        sys.exit(0)

    # 定义文件（.chanlun/definitions/）
    if re.search(r'[\\\\/]\.chanlun[\\\\/]definitions[\\\\/]', fp, re.IGNORECASE):
        print('yes')
        sys.exit(0)

    # 测试文件（tests/ 目录）
    if re.search(r'[\\\\/]tests[\\\\/]', fp):
        print('yes')
        sys.exit(0)

    # hook 脚本（.claude/hooks/）
    if re.search(r'[\\\\/]\.claude[\\\\/]hooks[\\\\/]', fp, re.IGNORECASE):
        print('yes')
        sys.exit(0)

    # agent 配置（.claude/agents/）
    if re.search(r'[\\\\/]\.claude[\\\\/]agents[\\\\/]', fp, re.IGNORECASE):
        print('yes')
        sys.exit(0)

# 默认放行
print('no')
" "$TOOL_NAME" "$FILE_PATH" 2>/dev/null || echo "no")

# 未命中黑名单：静默通过
if [ "$IS_BLACKLISTED" = "no" ]; then
  exit 0
fi

# --- 命中黑名单：生成拓扑异常对象（严重性降低——记录而非警报） ---
# 聚合策略：同一 tool_name 类型的 anomaly 聚合为一条记录（按 tool_name 分桶）。
# 不再为每次调用生成独立条目。frequency 递增反映实际触发次数。
# ID 基于 signature hash（不含 timestamp），确保可合并。

CWD=$(echo "$INPUT" | python -c "import sys,json; print(json.load(sys.stdin).get('cwd','.'))" 2>/dev/null || echo ".")
cd "$CWD" 2>/dev/null || true

PATTERN_FILE=".chanlun/pattern-buffer/topo-anomalies.yaml"
TIMESTAMP=$(date -u +"%Y-%m-%dT%H:%M:%SZ" 2>/dev/null || date +"%Y-%m-%dT%H:%M:%S")

# 聚合写入拓扑异常对象到 pattern-buffer
python -c "
import sys, os, hashlib, re

pattern_file = sys.argv[1]
timestamp = sys.argv[2]
tool_name = sys.argv[3]
file_path = sys.argv[4]

# 聚合签名：按 tool_name 分桶（不包含具体路径/命令，避免无限膨胀）
sig = f'lead-direct-{tool_name}'
pat_hash = hashlib.md5(sig.encode()).hexdigest()[:8]
pat_id = f'anomaly-{pat_hash}'

# 确保 pattern-buffer 分片目录和文件存在
os.makedirs(os.path.dirname(pattern_file), exist_ok=True)
if not os.path.isfile(pattern_file):
    with open(pattern_file, 'w', encoding='utf-8') as f:
        f.write('# 模式缓冲区分片——拓扑异常对象（lead-audit 检测）\n')
        f.write('# 088号谱系：Lead 拓扑异常对象化审计\n')
        f.write('# 分片规则：含 anomaly_type 字段的条目\n')
        f.write('version: \"1.0\"\n')
        f.write('patterns: []\n')

# 解析现有条目
with open(pattern_file, 'r', encoding='utf-8') as f:
    content = f.read()

existing = []
current = None
for line in content.split('\n'):
    stripped = line.strip()
    if stripped.startswith('- id:'):
        if current:
            existing.append(current)
        current = {'id': stripped.split(':', 1)[1].strip().strip('\"').strip(\"'\")}
    elif current:
        if stripped.startswith('signature:'):
            current['signature'] = stripped.split(':', 1)[1].strip().strip('\"').strip(\"'\")
        elif stripped.startswith('frequency:'):
            try:
                current['frequency'] = int(stripped.split(':', 1)[1].strip())
            except ValueError:
                current['frequency'] = 0
        elif stripped.startswith('first_seen:'):
            current['first_seen'] = stripped.split(':', 1)[1].strip().strip('\"').strip(\"'\")
        elif stripped.startswith('last_seen:'):
            current['last_seen'] = stripped.split(':', 1)[1].strip().strip('\"').strip(\"'\")
        elif stripped.startswith('sources:'):
            val = stripped.split(':', 1)[1].strip()
            current['sources'] = [s.strip().strip('\"').strip(\"'\") for s in val.strip('[]').split(',') if s.strip()]
        elif stripped.startswith('status:'):
            current['status'] = stripped.split(':', 1)[1].strip().strip('\"').strip(\"'\")
        elif stripped.startswith('description:'):
            current['description'] = stripped.split(':', 1)[1].strip().strip('\"').strip(\"'\")
        elif stripped.startswith('anomaly_type:'):
            current['anomaly_type'] = stripped.split(':', 1)[1].strip().strip('\"').strip(\"'\")
if current:
    existing.append(current)

# 查找是否已有同签名的 anomaly
found = False
promotion_threshold = 3
for p in existing:
    if p.get('signature') == sig:
        p['frequency'] = p.get('frequency', 0) + 1
        p['last_seen'] = timestamp
        # status 升级：frequency >= threshold 时从 observed/candidate → settled
        if p['frequency'] >= promotion_threshold and p.get('status') in ('observed', 'candidate'):
            p['status'] = 'settled'
        found = True
        break

if not found:
    existing.append({
        'id': pat_id,
        'signature': sig,
        'frequency': 1,
        'first_seen': timestamp,
        'last_seen': timestamp,
        'sources': [timestamp],
        'description': f'Lead 直接执行 {tool_name}（应委派工位）——拓扑异常对象',
        'status': 'observed',
        'anomaly_type': 'lead_direct_execution',
    })

# 写回
def yaml_escape(text):
    return str(text).replace('\\\\', '\\\\\\\\').replace('\"', '\\\\\"').replace('\n', ' ')

with open(pattern_file, 'w', encoding='utf-8') as f:
    f.write('# 模式缓冲区分片——拓扑异常对象（lead-audit 检测）\n')
    f.write('# 088号谱系：Lead 拓扑异常对象化审计\n')
    f.write('# 分片规则：含 anomaly_type 字段的条目\n')
    f.write('version: \"1.0\"\n')
    if not existing:
        f.write('patterns: []\n')
    else:
        f.write('patterns:\n')
        for p in existing:
            f.write(f'  - id: \"{yaml_escape(p.get(\"id\", \"?\"))}\"\n')
            f.write(f'    signature: \"{yaml_escape(p.get(\"signature\", \"\"))}\"\n')
            f.write(f'    frequency: {p.get(\"frequency\", 0)}\n')
            f.write(f'    first_seen: \"{yaml_escape(p.get(\"first_seen\", \"\"))}\"\n')
            f.write(f'    last_seen: \"{yaml_escape(p.get(\"last_seen\", \"\"))}\"\n')
            sources = p.get('sources', [])
            sources_str = ', '.join(f'\"{yaml_escape(s)}\"' for s in sources)
            f.write(f'    sources: [{sources_str}]\n')
            desc = p.get('description', '')
            if desc:
                f.write(f'    description: \"{yaml_escape(desc)}\"\n')
            f.write(f'    status: \"{yaml_escape(p.get(\"status\", \"observed\"))}\"\n')
            at = p.get('anomaly_type', '')
            if at:
                f.write(f'    anomaly_type: \"{yaml_escape(at)}\"\n')
" "$PATTERN_FILE" "$TIMESTAMP" "$TOOL_NAME" "$FILE_PATH" 2>/dev/null || true

# 输出审计记录（不阻断，降低严重性——从警报改为记录）
MSG="[lead-audit/088+173] Lead 直接执行 ${TOOL_NAME}（黑名单命中）。拓扑异常已记录至 pattern-buffer。"

MSG="$MSG" python -c "
import json, os
msg = os.environ['MSG']
print(json.dumps({'systemMessage': msg}, ensure_ascii=False))
" 2>/dev/null

exit 0
