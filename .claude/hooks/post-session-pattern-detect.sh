#!/bin/bash
# Stop Hook — post-session pattern detect（043号谱系：自生长回路入口）
#
# 触发：Stop 事件（session 即将结束）
# 逻辑：读取最近 session 文件，提取 tool 调用频次和序列模式，
#        写入 .chanlun/pattern-buffer.yaml
#
# 设计原则：
#   - 只追加/更新，不删除已有 pattern
#   - 幂等：同一 session 重复执行不产生重复条目
#   - 跨 session 合并：同签名 pattern 累加 frequency，追加 source
#   - 无外部依赖（纯 bash + python）
#
# Status 枚举（M1 统一）：
#   observed  — 单次出现，无重复序列（仅 session 概要）
#   candidate — session 内重复序列（frequency >= 2）
#   settled   — 跨 session 累积 frequency >= promotion_threshold（3）→ 结晶守卫拦截
#   promoted  — 已结晶为 skill
#   rejected  — 明确不结晶

set -euo pipefail

PROJECT_ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
SESSIONS_DIR="$PROJECT_ROOT/.chanlun/sessions"
PATTERN_FILE="$PROJECT_ROOT/.chanlun/pattern-buffer.yaml"

# --- 找到最新的 session 文件（排除 archive 目录）---
latest_session=""
if [ -d "$SESSIONS_DIR" ]; then
  latest_session=$(find "$SESSIONS_DIR" -maxdepth 1 -name "*.md" -type f 2>/dev/null \
    | sort -r \
    | head -1)
fi

if [ -z "$latest_session" ] || [ ! -f "$latest_session" ]; then
  # 无 session 文件，静默退出
  exit 0
fi

session_id=$(basename "$latest_session" .md)

# --- 提取 tool 调用序列 ---
# session 文件中 tool 调用通常以 "- Tool: XXX" 或 "tool_name" 等形式出现
# 我们提取所有看起来像 tool 名的词（大写开头的单词出现在特定上下文中）
# 简化策略：提取所有 `Read`, `Write`, `Edit`, `Grep`, `Glob`, `Bash`, `Task`, `TaskUpdate` 等关键词

tool_sequence=$(grep -oE '\b(Read|Write|Edit|Grep|Glob|Bash|Task|TaskUpdate|TaskCreate|TaskList|TaskGet|SendMessage|WebFetch|WebSearch|NotebookEdit|Skill)\b' "$latest_session" 2>/dev/null || true)

if [ -z "$tool_sequence" ]; then
  # 无 tool 调用记录，静默退出
  exit 0
fi

# --- 统计频次 ---
freq_table=$(echo "$tool_sequence" | sort | uniq -c | sort -rn)

# --- 检测重复序列（滑动窗口：3-tool 序列）---
tool_array=($tool_sequence)
declare -A trigram_count 2>/dev/null || true

# bash 4+ associative array fallback
if declare -A trigram_count 2>/dev/null; then
  len=${#tool_array[@]}
  if [ "$len" -ge 3 ]; then
    for ((i=0; i<=len-3; i++)); do
      trigram="${tool_array[$i]}→${tool_array[$((i+1))]}→${tool_array[$((i+2))]}"
      trigram_count["$trigram"]=$(( ${trigram_count["$trigram"]:-0} + 1 ))
    done
  fi

  # 过滤频次 >= 2 的序列
  repeated_patterns=""
  for key in "${!trigram_count[@]}"; do
    count=${trigram_count[$key]}
    if [ "$count" -ge 2 ]; then
      repeated_patterns="$repeated_patterns$key|$count\n"
    fi
  done
else
  # fallback：无 associative array 支持，跳过序列检测
  repeated_patterns=""
fi

# --- 确保 pattern-buffer.yaml 存在 ---
if [ ! -f "$PATTERN_FILE" ]; then
  cat > "$PATTERN_FILE" << 'INIT'
# 模式缓冲区——谱系的生成态前置
# 043号谱系：自生长回路
version: "1.0"
patterns: []
INIT
fi

# --- 检查此 session 是否已处理过 ---
# 精确匹配：session_id 必须作为完整的 sources 条目出现（避免子串误匹配）
if grep -qE "sources:.*\"${session_id}\"" "$PATTERN_FILE" 2>/dev/null; then
  # 已处理，幂等退出
  exit 0
fi

# --- 生成 pattern 条目 ---
timestamp=$(date -u +"%Y-%m-%dT%H:%M:%SZ" 2>/dev/null || date +"%Y-%m-%dT%H:%M:%S")

# 从频次表生成 tool 使用概要
top_tools=$(echo "$freq_table" | head -5 | awk '{printf "%s(%s) ", $2, $1}')

# 收集本 session 的 pattern（signature → frequency）
declare -A session_patterns 2>/dev/null || true

if declare -A session_patterns 2>/dev/null; then
  # 从重复序列收集
  if [ -n "$repeated_patterns" ]; then
    while IFS='|' read -r sig freq; do
      [ -z "$sig" ] && continue
      session_patterns["$sig"]="$freq"
    done < <(echo -e "$repeated_patterns")
  fi
fi

# --- M2: 跨 session 合并逻辑 ---
# 使用 python 处理 YAML 合并（bash 处理 YAML 不可靠）
PROMOTION_THRESHOLD=3

python -c "
import sys, re, os

pattern_file = sys.argv[1]
session_id = sys.argv[2]
timestamp = sys.argv[3]
threshold = int(sys.argv[4])
top_tools = sys.argv[5]

# 读取传入的 session patterns（signature|frequency）
session_pats = {}
for line in sys.argv[6].split('\\n'):
    line = line.strip()
    if not line:
        continue
    parts = line.split('|')
    if len(parts) == 2:
        session_pats[parts[0]] = int(parts[1])

# 解析现有 pattern-buffer.yaml
existing = []
if os.path.isfile(pattern_file):
    with open(pattern_file, 'r', encoding='utf-8') as f:
        content = f.read()

    current = None
    for line in content.split('\\n'):
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
                # 简单解析 [\"a\", \"b\"]
                val = stripped.split(':', 1)[1].strip()
                current['sources'] = [s.strip().strip('\"').strip(\"'\") for s in val.strip('[]').split(',') if s.strip()]
            elif stripped.startswith('status:'):
                current['status'] = stripped.split(':', 1)[1].strip().strip('\"').strip(\"'\")
            elif stripped.startswith('description:'):
                current['description'] = stripped.split(':', 1)[1].strip().strip('\"').strip(\"'\")
    if current:
        existing.append(current)

# 构建 signature → existing pattern 索引
sig_index = {}
for i, p in enumerate(existing):
    sig = p.get('signature', '')
    if sig:
        sig_index[sig] = i

# 合并 session patterns 到 existing
merged_sigs = set()
for sig, freq in session_pats.items():
    if sig in sig_index:
        # 合并：累加 frequency，追加 source，更新 last_seen
        idx = sig_index[sig]
        existing[idx]['frequency'] = existing[idx].get('frequency', 0) + freq
        sources = existing[idx].get('sources', [])
        if session_id not in sources:
            sources.append(session_id)
        existing[idx]['sources'] = sources
        existing[idx]['last_seen'] = timestamp
        # M1: 升级 status（observed/candidate → settled 当 frequency >= threshold）
        if existing[idx]['frequency'] >= threshold and existing[idx].get('status') in ('observed', 'candidate'):
            existing[idx]['status'] = 'settled'
        merged_sigs.add(sig)
    else:
        # 新 pattern
        import hashlib
        pat_hash = hashlib.md5(f'{session_id}-{sig}'.encode()).hexdigest()[:8]
        new_pat = {
            'id': f'pat-{pat_hash}',
            'signature': sig,
            'frequency': freq,
            'first_seen': timestamp,
            'last_seen': timestamp,
            'sources': [session_id],
            'status': 'candidate'
        }
        existing.append(new_pat)
        merged_sigs.add(sig)

# 如果没有重复序列但有 tool 使用，记录概要（仅当无 session pattern 时）
if not session_pats and top_tools.strip():
    summary_sig = f'session-summary: {top_tools.strip()}'
    if summary_sig not in sig_index:
        import hashlib
        summary_hash = hashlib.md5(f'{session_id}-summary'.encode()).hexdigest()[:8]
        existing.append({
            'id': f'pat-{summary_hash}',
            'signature': summary_sig,
            'frequency': 1,
            'first_seen': timestamp,
            'last_seen': timestamp,
            'sources': [session_id],
            'status': 'observed'
        })

# 写回 pattern-buffer.yaml
with open(pattern_file, 'w', encoding='utf-8') as f:
    f.write('# 模式缓冲区——谱系的生成态前置\\n')
    f.write('# 043号谱系：自生长回路\\n')
    f.write('# Status 枚举: observed → candidate → settled → promoted/rejected\\n')
    f.write('version: \"1.0\"\\n')
    if not existing:
        f.write('patterns: []\\n')
    else:
        f.write('patterns:\\n')
        for p in existing:
            f.write(f'  - id: \"{p.get(\"id\", \"?\")}\"\\n')
            f.write(f'    signature: \"{p.get(\"signature\", \"\")}\"\\n')
            f.write(f'    frequency: {p.get(\"frequency\", 0)}\\n')
            f.write(f'    first_seen: \"{p.get(\"first_seen\", \"\")}\"\\n')
            f.write(f'    last_seen: \"{p.get(\"last_seen\", \"\")}\"\\n')
            sources = p.get('sources', [])
            sources_str = ', '.join(f'\"{s}\"' for s in sources)
            f.write(f'    sources: [{sources_str}]\\n')
            desc = p.get('description', '')
            if desc:
                f.write(f'    description: \"{desc}\"\\n')
            f.write(f'    status: \"{p.get(\"status\", \"observed\")}\"\\n')
" "$PATTERN_FILE" "$session_id" "$timestamp" "$PROMOTION_THRESHOLD" "$top_tools" "$(echo -e "$repeated_patterns")"

exit 0
