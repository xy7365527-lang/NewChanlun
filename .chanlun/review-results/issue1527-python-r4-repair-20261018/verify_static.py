"""#1527 返修静态验收：AST、历史原件、写集、精确 patch；不执行产品。"""

import ast
import hashlib
import json
from pathlib import Path
import subprocess

REPORT = Path(__file__).resolve().parent
ROOT = REPORT.parents[2]
BASE = "561dd8725436077ce870d8189a583b5727d5d7d4"
SOURCES = ("s_session/recovery_admission.py", "s_session/s_service_control.py",
           "s_session/tests/issue1527_recovery/test_python_r4.py")


def git(*args):
    return subprocess.check_output(["git", *args], cwd=ROOT)


def main():
    assert git("rev-parse", "HEAD").decode().strip() == BASE
    assert git("branch", "--show-current").decode().strip().startswith("codex/")
    frozen = json.loads((ROOT.parent / "PYTHON-R4-FROZEN-R1.json").read_text())
    assert git("rev-parse", "HEAD^{tree}").decode().strip() == frozen["tree"]
    for name, expected in frozen["files"].items():
        assert hashlib.sha256(git("show", BASE + ":" + name)).hexdigest() == expected
        if name not in SOURCES:
            assert hashlib.sha256((ROOT / name).read_bytes()).hexdigest() == expected
    changed = git("diff", "--name-only", BASE).decode().splitlines()
    assert set(changed) == set(SOURCES), changed
    report_prefix = str(REPORT.relative_to(ROOT)) + "/"
    untracked = git("ls-files", "--others", "--exclude-standard").decode().splitlines()
    assert all(name.startswith(report_prefix) for name in untracked), untracked
    for name in SOURCES:
        ast.parse((ROOT / name).read_bytes(), filename=name)
    for path in REPORT.glob("*.py"):
        ast.parse(path.read_bytes(), filename=str(path))
    git("diff", "--check", BASE)
    exact = git("diff", "--binary", BASE, "--", *SOURCES)
    patch = REPORT / "python-r4-repair.patch"
    assert patch.read_bytes() == exact
    git("apply", "--reverse", "--check", str(patch))
    print(json.dumps(dict(静态检查="完成", 固定基线=BASE, 原件核对条数=len(frozen["files"]),
                         历史报告未改=True, AST文件数=len(SOURCES), 写集=changed,
                         patch_sha256=hashlib.sha256(exact).hexdigest(),
                         diff_check_rc=0, patch_reverse_check_rc=0), ensure_ascii=False, indent=2))


if __name__ == "__main__":
    main()
