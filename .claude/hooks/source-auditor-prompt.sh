#!/bin/bash
# PostToolUse Hook — source-auditor 提示
# 087号谱系修复（A选项）：实现 docs/** file_write 事件检测
# 消除 dispatch-dag.yaml 中 source-auditor 的声明-能力缺口
#
# 触发：PostToolUse on "Write" | "Edit" tool
# 逻辑：检测写入路径是否匹配 docs/**，若匹配则输出提示（D策略：hooks 提示 + Lead 认领）
# 不阻断，只提示

set -uo pipefail

INPUT=$(cat)

PYTHON_BIN=""
if command -v python >/dev/null 2>&1; then
    PYTHON_BIN="python"
elif command -v python3 >/dev/null 2>&1; then
    PYTHON_BIN="python3"
else
    # 运行环境无 Python 时保持静默，不阻断主流程。
    exit 0
fi

HOOK_INPUT="$INPUT" "$PYTHON_BIN" - <<'PY' 2>/dev/null || true
import json
import os


def pick_file_path(tool_input):
    for key in ("file_path", "path", "target_file"):
        value = tool_input.get(key)
        if isinstance(value, str) and value:
            return value
    return ""


raw = os.environ.get("HOOK_INPUT", "")
if not raw.strip():
    raise SystemExit(0)

try:
    data = json.loads(raw)
except json.JSONDecodeError:
    # Hook 输入异常时保持静默，不阻断主流程。
    raise SystemExit(0)

tool_name = data.get("tool_name", "")
if tool_name not in {"Write", "Edit"}:
    raise SystemExit(0)

tool_input = data.get("tool_input", {})
if not isinstance(tool_input, dict):
    raise SystemExit(0)

file_path = pick_file_path(tool_input)
if not file_path:
    raise SystemExit(0)

cwd = data.get("cwd", ".")
if not isinstance(cwd, str) or not cwd:
    cwd = "."

try:
    rel = os.path.relpath(file_path, cwd)
except Exception:
    rel = file_path

rel = rel.replace("\\", "/")
if rel.startswith("./"):
    rel = rel[2:]

if rel != "docs" and not rel.startswith("docs/"):
    raise SystemExit(0)

print(
    json.dumps(
        {
            "continue": True,
            "systemMessage": (
                f"[source-auditor advisory] 检测到 docs/ 变更（{rel}）。"
                "Lead 可选择：读取 .claude/agents/source-auditor.md 对本次变更执行"
                "溯源标签验证（三级权威链合规性检查）。"
            ),
        },
        ensure_ascii=False,
    )
)
PY

exit 0
