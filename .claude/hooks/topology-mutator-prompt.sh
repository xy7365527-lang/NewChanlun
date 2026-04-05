#!/usr/bin/env bash
# [已废弃] PostToolUse Hook — topology-mutator 触发提示
#
# 功能已内联到 post-write-edit-dispatcher.sh（Guard 4: topology-mutator-prompt）。
# 本文件保留仅作历史参考。不再由 settings.json 注册。
#
# 原始描述：
# 触发：PostToolUse on Write/Edit
# 逻辑：settled 谱系写入后，检测 topo_effect 字段，
#       若存在则通过 systemMessage 提示 Lead 执行 topology_operator.py
# 设计：D策略（082号）——hooks 提示 + Lead 认领
# 原则0：只提示，不阻断
#
# 异质审计缺口13修复：标注废弃状态，消除与 dispatcher 的功能重复歧义。
exit 0

set -euo pipefail

resolve_python() {
  local bin
  for bin in python python3; do
    if command -v "$bin" >/dev/null 2>&1 && "$bin" -c "import sys" >/dev/null 2>&1; then
      echo "$bin"
      return 0
    fi
  done
  return 1
}

PYTHON_BIN="$(resolve_python || true)"
[ -n "$PYTHON_BIN" ] || exit 0

python() { command "$PYTHON_BIN" "$@"; }

INPUT=$(timeout 3 cat 2>/dev/null || echo "{}")

TOOL_NAME=$(echo "$INPUT" | python -c "
import sys, json
data = json.loads(sys.stdin.read())
print(data.get('tool_name', ''))
" 2>/dev/null || echo "")

if [ "$TOOL_NAME" != "Write" ] && [ "$TOOL_NAME" != "Edit" ]; then
  exit 0
fi

FILE_PATH=$(echo "$INPUT" | python -c "
import sys, json
data = json.loads(sys.stdin.read())
print(data.get('tool_input', {}).get('file_path', ''))
" 2>/dev/null || echo "")

CWD=$(echo "$INPUT" | python -c "
import sys, json
data = json.loads(sys.stdin.read())
print(data.get('cwd', '.'))
" 2>/dev/null || echo ".")

cd "$CWD" 2>/dev/null || true

REL_PATH=$(python -c "
import sys, os
try:
    rel = os.path.relpath(sys.argv[1], sys.argv[2])
except ValueError:
    rel = sys.argv[1]
print(rel.replace(os.sep, '/'))
" "$FILE_PATH" "$CWD" 2>/dev/null || echo "$FILE_PATH")

# 只处理 settled/ 下的谱系文件
MATCH=$(python -c "
import re, sys
path = sys.argv[1]
print('yes' if re.match(r'^\\.chanlun/genealogy/settled/[^/]+\\.md$', path) else 'no')
" "$REL_PATH" 2>/dev/null || echo "no")

if [ "$MATCH" != "yes" ]; then
  exit 0
fi

# 提取 topo_effect 和谱系 id，生成 topology_operator.py 命令提示
python - "$FILE_PATH" <<'PY' 2>/dev/null || true
import json
import re
import sys

file_path = sys.argv[1]

try:
    with open(file_path, encoding="utf-8") as f:
        content = f.read()
except (IOError, FileNotFoundError):
    sys.exit(0)

topo_match = re.search(r"topo_effect:\s*[\"']*(\S+)[\"']*", content)
if not topo_match:
    sys.exit(0)

topo_effect = topo_match.group(1).strip().strip('"').strip("'")
parts = topo_effect.split(":")
if len(parts) != 3:
    sys.exit(0)

effect_type, target_id, scope = parts
if effect_type not in ("freeze", "split", "sever"):
    sys.exit(0)

id_match = re.search(r"^id:\s*[\"']*(\S+)[\"']*", content, re.MULTILINE)
genealogy_id = id_match.group(1).strip('"').strip("'") if id_match else "unknown"

negates_items = []
neg_match = re.search(r"negates:\s*\[(.+?)\]", content)
if neg_match:
    negates_items = [
        x.strip().strip('"').strip("'")
        for x in neg_match.group(1).split(",")
        if x.strip()
    ]

type_names = {
    "freeze": "冻结路径（等待型否定）",
    "split": "分裂节点（扩张型否定）",
    "sever": "切断连接（分离型否定）",
}
type_name = type_names.get(effect_type, effect_type)
negates_str = ", ".join(negates_items) if negates_items else "(见谱系文件)"

msg = (
    f"[topology-mutator/147+178] 谱系 {genealogy_id} 携带拓扑效果: {type_name}。"
    f" 目标: {target_id}, 范围: {scope}, 否定: {negates_str}。"
    f" Lead 应执行: python scripts/topology_operator.py --genealogy {file_path}"
    f" --base .chanlun/block-topology"
)
print(json.dumps({"systemMessage": msg}, ensure_ascii=False))
PY

