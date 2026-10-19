"""#1527 工作草稿：固定基线和窄差异核验，不运行 Rust 或产品。"""

import ast
import hashlib
import json
from pathlib import Path
import subprocess

ROOT = Path(__file__).resolve().parents[3]
BASE = "5cb7253d46d973669e7d10cd3331e12c9ef313b1"
FILES = ["s_session/recovery_admission.py", "s_session/s_service_control.py",
         "s_session/tests/issue1527_recovery/test_python_r4.py"]


def git(*args):
    return subprocess.run(["git", *args], cwd=ROOT, check=True, capture_output=True).stdout


def definitions(raw):
    tree = ast.parse(raw)
    return {node.name: ast.dump(node, include_attributes=False)
            for node in ast.walk(tree) if isinstance(node, (ast.FunctionDef, ast.AsyncFunctionDef))}


assert git("rev-parse", "HEAD").decode().strip() == BASE
assert git("branch", "--show-current").decode().strip().startswith("codex/")
for filename in FILES:
    ast.parse((ROOT / filename).read_bytes())
old = git("show", BASE + ":s_session/s_service_control.py")
before, after = definitions(old), definitions((ROOT / FILES[1]).read_bytes())
unchanged = ["validate_s_resources", "freeze_s_resources", "validate_resource_identity",
             "process_identity", "runtime_environment", "record", "status", "confirm_ready_process",
             "prepare_socket", "start_q", "stop", "wait_ready"]
for name in unchanged:
    assert before[name] == after[name], name
launcher = "s_session/launch_s.sh"
assert git("show", BASE + ":" + launcher) == (ROOT / launcher).read_bytes()
assert b'"$HERE/s_service_control.py" "$@"' in (ROOT / launcher).read_bytes()
raw = (ROOT / FILES[0]).read_text()
assert 'PENDING = ".s-admission-pending"' in raw
assert 'os.unlink(' not in raw and 'os.replace(' not in raw
status = git("status", "--porcelain", "--untracked-files=all").decode().splitlines()
for line in status:
    name = line[3:]
    assert name in FILES or name.startswith(".chanlun/review-results/issue1527-python-r4-20261018/"), name
print(json.dumps({"结论": "通过", "基线": BASE, "语法文件数": len(FILES),
                  "AST未变函数": unchanged, "launcher": "原 service 转接逐字未变",
                  "当前文件sha256": {p: hashlib.sha256((ROOT / p).read_bytes()).hexdigest() for p in FILES}},
                 ensure_ascii=False, indent=2))
