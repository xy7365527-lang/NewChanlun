#!/usr/bin/env bash
# post-write-edit-dispatcher.sh — Write/Edit PostToolUse 统一调度器（153号性能优化）
#
# 合并以下 5 个 hook 为单次 Python 调用：
#   - result-package-guard.sh（谱系结果包六要素检查）
#   - downstream-action-guard.sh（下游推论追踪）
#   - dag-validation-guard.sh（DAG 验证）
#   - topology-mutator-prompt.sh（topo_effect 提示）
#   - lead-audit.sh（基因组修改检测）
#
# 性能改善：5 个 bash 脚本 × 2-4 次 Python 启动 → 1 个 bash + 1 次 Python

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
import json, os, re, sys, subprocess

try:
    data = json.loads(os.environ.get("INPUT_JSON", "{}"))
except Exception:
    sys.exit(0)

tool_name = data.get("tool_name", "")
if tool_name not in ("Write", "Edit"):
    sys.exit(0)

file_path = data.get("tool_input", {}).get("file_path", "")
cwd = data.get("cwd", ".")

if not file_path:
    sys.exit(0)

# --- 公共：路径处理 ---
try:
    rel_path = os.path.relpath(file_path, cwd).replace(os.sep, "/")
except ValueError:
    rel_path = file_path.replace("\\", "/")

try:
    os.chdir(cwd)
except Exception:
    pass

messages = []

is_genealogy_any = bool(re.match(r'^\.chanlun/genealogy/(pending|settled)/[^/]+\.md$', rel_path))
is_genealogy_settled = bool(re.match(r'^\.chanlun/genealogy/settled/[^/]+\.md$', rel_path))

# --- Guard 1: result-package-guard（谱系结果包六要素） ---
if is_genealogy_any:
    try:
        with open(file_path, "r", encoding="utf-8") as f:
            content = f.read()
        required = {
            "边界条件": r'(?:##\s*边界条件|\*\*边界条件\*\*|boundary)',
            "下游推论": r'(?:##\s*下游推论|\*\*下游推论\*\*|downstream)',
            "影响声明": r'(?:##\s*影响声明|\*\*影响声明\*\*|impact)',
        }
        missing = [n for n, p in required.items() if not re.search(p, content, re.IGNORECASE)]
        if missing:
            messages.append("[result-package-guard] 缺少: " + ", ".join(missing))
    except Exception:
        pass

# --- Guard 2: downstream-action-guard（下游推论追踪） ---
if is_genealogy_settled:
    try:
        with open(file_path, "r", encoding="utf-8") as f:
            content = f.read()
        m = re.search(r'##\s*下游推论\s*\n(.*?)(?=\n##\s|\Z)', content, re.DOTALL)
        local_count = len(re.findall(r'^\d+\.', m.group(1), re.MULTILINE)) if m else 0

        audit_script = os.path.join(cwd, "scripts", "downstream_audit.py")
        audit = ""
        if os.path.isfile(audit_script):
            try:
                proc = subprocess.run(
                    [sys.executable, audit_script, "--summary"],
                    capture_output=True, text=True, timeout=5
                )
                audit = proc.stdout.strip()
            except Exception:
                pass

        if local_count > 0 or (audit and "0 未解决" not in audit):
            parts = []
            if local_count > 0:
                parts.append(f"本谱系含 {local_count} 条下游推论")
            if audit:
                parts.append(f"全局: {audit}")
            messages.append("[downstream-action-guard] " + "。".join(parts))
    except Exception:
        pass

# --- Guard 3: dag-validation-guard（DAG 验证） ---
if is_genealogy_any:
    dag_file = os.path.join(cwd, ".chanlun", "genealogy", "dag.yaml")
    validate_script = os.path.join(cwd, "scripts", "chanlun", "validate_dag.py")
    sync_script = os.path.join(cwd, "scripts", "dag_sync.py")

    if os.path.isfile(dag_file) and os.path.isfile(validate_script):
        try:
            proc = subprocess.run(
                [sys.executable, validate_script],
                capture_output=True, text=True, timeout=5
            )
            output = proc.stdout + proc.stderr
            if "DAG validation passed" not in output:
                # 尝试自动修复
                if os.path.isfile(sync_script):
                    subprocess.run(
                        [sys.executable, sync_script],
                        capture_output=True, text=True, timeout=5
                    )
                    proc2 = subprocess.run(
                        [sys.executable, validate_script],
                        capture_output=True, text=True, timeout=5
                    )
                    output2 = proc2.stdout + proc2.stderr
                    if "DAG validation passed" not in output2:
                        # 仍然失败——报告但不阻断（PostToolUse 不能 block）
                        messages.append(f"[dag-validation-guard] DAG 验证失败: {output2[:200]}")
                else:
                    messages.append(f"[dag-validation-guard] DAG 验证失败: {output[:200]}")
        except Exception:
            pass

# --- Guard 4: topology-mutator-prompt（topo_effect 提示） ---
if is_genealogy_settled:
    try:
        with open(file_path, "r", encoding="utf-8") as f:
            content = f.read()
        topo_match = re.search(r"topo_effect:\s*[\"']*(\S+)[\"']*", content)
        if topo_match:
            topo_effect = topo_match.group(1).strip().strip('"').strip("'")
            parts = topo_effect.split(":")
            if len(parts) == 3 and parts[0] in ("freeze", "split", "sever"):
                effect_type, target_id, scope = parts
                id_match = re.search(r"^id:\s*[\"']*(\S+)[\"']*", content, re.MULTILINE)
                gid = id_match.group(1).strip('"').strip("'") if id_match else "unknown"
                type_names = {"freeze": "冻结", "split": "分裂", "sever": "切断"}
                messages.append(
                    f"[topology-mutator/147] 谱系 {gid} 拓扑效果: {type_names.get(effect_type, effect_type)}"
                    f" 目标: {target_id}, 范围: {scope}"
                )
    except Exception:
        pass

# --- Guard 5: lead-audit（基因组修改检测，153号黑名单模式） ---
if not os.environ.get("CLAUDE_AGENT_NAME"):
    fp = file_path.replace("\\", "/")
    genome_patterns = [
        r'CLAUDE\.md$',
        r'[\\/]\.claude[\\/]rules[\\/]',
        r'[\\/]\.claude[\\/]skills[\\/]',
        r'[\\/]\.claude[\\/]hooks[\\/]',
        r'[\\/]\.claude[\\/]agents[\\/]',
    ]
    for pat in genome_patterns:
        if re.search(pat, fp, re.IGNORECASE):
            messages.append(f"[lead-audit/153] 基因组文件修改: {rel_path[:100]}")
            break

# --- 输出 ---
if not messages:
    sys.exit(0)

combined = "。".join(messages)
print(json.dumps({"systemMessage": combined}, ensure_ascii=False))
PY

exit 0
