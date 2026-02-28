#!/usr/bin/env bash
# pre-write-edit-dispatcher.sh — Write/Edit PreToolUse 统一调度器（153号性能优化）
#
# 合并以下 4 个 hook 为单次 Python 调用：
#   - definition-write-guard.sh（定义文件格式校验）
#   - genealogy-write-guard.sh（谱系写入字段+语义校验）
#   - spec-write-guard.sh（核心配置文件修改通知）
#   - hub-node-impact-guard.sh（Hub 节点影响链警告）
#
# 性能改善：4 个 bash 脚本 × 3-5 次 Python 启动 → 1 个 bash + 1 次 Python

set -euo pipefail

INPUT=$(cat)

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

INPUT_JSON="$INPUT" "$PYTHON_BIN" - <<'PY'
import json, os, re, sys, glob

try:
    data = json.loads(os.environ.get("INPUT_JSON", "{}"))
except Exception:
    sys.exit(0)

tool_name = data.get("tool_name", "")
if tool_name not in ("Write", "Edit"):
    sys.exit(0)

tool_input = data.get("tool_input", {})
file_path = tool_input.get("file_path", "")
cwd = data.get("cwd", ".")

if not file_path:
    sys.exit(0)

# --- 公共：路径处理（一次完成） ---
try:
    rel_path = os.path.relpath(file_path, cwd).replace(os.sep, "/")
except ValueError:
    rel_path = file_path.replace("\\", "/")

messages = []

# --- Guard 1: definition-write-guard（定义文件校验） ---
if re.match(r'^(.*/)?\.\s*chanlun/definitions/[^/]+\.md$', rel_path) or \
   re.search(r'[/\\]\.chanlun[/\\]definitions[/\\][^/\\]+\.md$', file_path, re.IGNORECASE):

    # 获取待写入内容
    if tool_name == "Edit":
        old_str = tool_input.get("old_string", "")
        new_str = tool_input.get("new_string", "")
        try:
            with open(file_path, "r", encoding="utf-8") as f:
                content = f.read()
            content = content.replace(old_str, new_str, 1)
        except Exception:
            content = ""
    else:
        content = tool_input.get("content", "")

    if tool_name == "Write":
        missing = []
        # status 字段
        if not re.search(r'(?:^status:\s*(?:生成态|已结算)|^\*\*状态\*\*:\s*(?:生成态|已结算))', content, re.MULTILINE):
            missing.append("status/状态")
        # version 字段
        if not re.search(r'(?:^version:\s*\S|^\*\*版本\*\*:\s*\S)', content, re.MULTILINE):
            missing.append("version/版本")
        if missing:
            messages.append("[definition-write-guard] 定义文件缺少: " + ", ".join(missing))

    elif tool_name == "Edit":
        # 检查是否将 status 改为已结算
        if re.search(r'(?:^status:\s*已结算|^\*\*状态\*\*:\s*已结算)', tool_input.get("new_string", ""), re.MULTILINE):
            basename = os.path.basename(file_path).replace(".md", "")
            settled_dir = os.path.join(cwd, ".chanlun", "genealogy", "settled")
            if os.path.isdir(settled_dir):
                found = any(
                    basename.lower() in os.path.basename(f).lower()
                    for f in glob.glob(os.path.join(settled_dir, "*.md"))
                )
                if not found:
                    messages.append(f"[definition-write-guard] 将 {basename}.md 改为已结算但无对应谱系")

# --- Guard 2: genealogy-write-guard（谱系文件校验） ---
is_genealogy = bool(re.match(r'^\.chanlun/genealogy/(pending|settled)/[^/]+\.md$', rel_path))

if is_genealogy:
    if tool_name == "Edit":
        try:
            with open(file_path, "r", encoding="utf-8") as f:
                content = f.read()
            content = content.replace(
                tool_input.get("old_string", ""),
                tool_input.get("new_string", ""),
                1,
            )
        except Exception:
            content = ""
    else:
        content = tool_input.get("content", "")

    # 强制字段
    required = {"类型": r'(?:\*\*类型\*\*|类型)\s*[:：]',
                "状态": r'(?:\*\*状态\*\*|状态)\s*[:：]',
                "日期": r'(?:\*\*日期\*\*|日期)\s*[:：]',
                "前置": r'(?:\*\*前置\*\*|前置)\s*[:：]'}
    missing = [n for n, p in required.items() if not re.search(p, content)]
    if missing:
        messages.append("[genealogy-write-guard] 缺失字段: " + ", ".join(missing))

    # negation_form 检查（spec-gap-audit 修复：meta-orchestration 要求每条谱系标注 negation_form）
    has_negation_form = bool(re.search(r'(?:negation_form|否定形式)\s*[:：]', content))
    if not has_negation_form:
        messages.append("[genealogy-write-guard] 建议标注 negation_form（否定形式：waiting/expansion/separation/unclassified）")

    # 前置引用验证
    if "前置" not in missing:
        m = re.search(r'(?:\*\*前置\*\*|前置)\s*[:：]\s*(.*)', content)
        if m:
            val = m.group(1).strip()
            if val and val not in ("无", "N/A", "-", "（无）", "(无)", "(none)", "none"):
                refs = re.findall(r'(\d{3}-[a-zA-Z0-9_-]+)', val)
                invalid = []
                for ref in refs:
                    found = False
                    for subdir in ("settled", "pending"):
                        if glob.glob(os.path.join(cwd, ".chanlun", "genealogy", subdir, ref + "*.md")):
                            found = True
                            break
                    if not found:
                        invalid.append(ref)
                if invalid:
                    messages.append("[genealogy-write-guard] 前置引用不存在: " + ", ".join(invalid))

    # topo_effect 检查（147号）
    has_negates = bool(re.search(r'negates:\s*\[(.+?)\]', content)) or \
                  bool(re.search(r'negates:\s*\n\s+-\s+', content))
    has_topo = bool(re.search(r'topo_effect:\s*\S', content))
    if has_negates and not has_topo:
        messages.append("[genealogy-write-guard] negates 存在但缺少 topo_effect 标注")

    # 语义检查（调用外部脚本，可选）
    sem_script = os.path.join(cwd, "scripts", "genealogy_semantic_check.py")
    if os.path.isfile(sem_script) and content:
        try:
            import subprocess
            proc = subprocess.run(
                [sys.executable, sem_script, file_path, "--content-stdin"],
                input=content, capture_output=True, text=True, timeout=5
            )
            if proc.returncode == 0 and proc.stdout.strip():
                sem = json.loads(proc.stdout)
                if sem.get("overall") in ("warn", "fail"):
                    issues = [
                        c["check"] + ": " + c["detail"]
                        for c in sem.get("checks", [])
                        if c.get("status") in ("warn", "fail")
                    ]
                    if issues:
                        messages.append("[genealogy-write-guard] 语义: " + "; ".join(issues))
        except Exception:
            pass

# --- Guard 3: spec-write-guard（核心文件修改通知） ---
core_patterns = [
    (r"(^|/)CLAUDE\.md$", "CLAUDE.md"),
    (r"(^|/)\.chanlun/dispatch-dag\.yaml$", "dispatch-dag.yaml"),
    (r"(^|/)\.chanlun/genealogy/settled/.*\.md$", "已结算谱系"),
    (r"(^|/)\.chanlun/definitions/.*\.md$", "定义文件"),
    (r"(^|/)\.claude/agents/.*\.md$", "agent定义"),
    (r"(^|/)\.claude/hooks/.*\.sh$", "hook脚本"),
    (r"(^|/)\.claude/commands/.*\.md$", "命令定义"),
    (r"(^|/)scripts/ceremony_scan\.py$", "ceremony脚本"),
]
for pattern, label in core_patterns:
    if re.search(pattern, rel_path):
        full = file_path if os.path.isabs(file_path) else os.path.join(cwd, rel_path.replace("/", os.sep))
        if os.path.exists(full):
            messages.append(f"[spec-write-guard] 核心文件修改: {rel_path} ({label})")
        break

# --- Guard 4: hub-node-impact-guard（Hub 节点影响链） ---
hub_match = re.search(r'genealogy/settled/(\d{3}[a-z]?)-', rel_path)
if hub_match:
    node_id = hub_match.group(1)
    hub_nodes = {
        "020": ("构成性矛盾", 21, "019d,020a,021,022,027,028,032,033,035,036,039,041,043,044,052,053,054,056,058,059,060"),
        "016": ("运行时执行层", 19, "017,019,019b,019c,019d,032,033,034,036,038,039,042,043,044,045,048,049,051,053"),
        "005b": ("对象否定对象语法", 17, "004,005,005a,006,010,012,015,016,019c,020,024,041,042,044,046,053,054"),
        "013": ("蜂群结构工位", 14, "014,019,019a,019b,019d,020,030,032,037,039,043,045,046,060"),
        "033": ("声明式dispatch-dag", 9, "034,036,037,038,039,042,043,045,059"),
    }
    if node_id in hub_nodes:
        name, count, chain = hub_nodes[node_id]
        messages.append(f"[hub-node-impact-guard] Hub 节点 {node_id}号（{name}）被 {count} 条谱系引用。影响链: {chain}")

# --- 输出 ---
if not messages:
    sys.exit(0)

combined = "。".join(messages)
print(json.dumps({
    "hookSpecificOutput": {
        "hookEventName": "PreToolUse",
        "permissionDecision": "allow",
        "permissionDecisionReason": combined,
    }
}, ensure_ascii=False))
PY

exit 0
