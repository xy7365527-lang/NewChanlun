"""#1419 封包：只读 Git 检查、写本报告目录；不执行工具链或修改 Git 状态。"""
import difflib
import hashlib
import json
from pathlib import Path
import subprocess

OUT = Path(__file__).resolve().parent
ROOT = OUT.parents[2]
BASE = "545e9f75faec49be39de7799e0aaf3f6c8276de9"
DONOR = "634f62cddfaddf656a35ca9b9db20cb716b62bff"
FILES = [r["path"] for r in json.loads((OUT / "SOURCE-INDEX.json").read_text())["files"]]
COMMANDS = []


def sha(data):
    return hashlib.sha256(data).hexdigest()


def write(name, data):
    (OUT / name).write_text(json.dumps(data, ensure_ascii=False, indent=2) + "\n")


def run(args, expected=0):
    p = subprocess.run(args, cwd=ROOT, text=True, capture_output=True)
    COMMANDS.append({"argv": args, "exit_code": p.returncode, "stdout": p.stdout, "stderr": p.stderr})
    assert p.returncode == expected, COMMANDS[-1]
    return p.stdout


def git_bytes(ref, path, absent=False):
    if absent:
        return b""
    return subprocess.check_output(["git", "show", ref + ":" + path], cwd=ROOT)


def patch(name, old_ref):
    chunks = []
    for path in FILES:
        absent = old_ref == BASE and path.endswith("/CenterPairFixtureExport.lean")
        before = git_bytes(old_ref, path, absent).decode()
        after = (ROOT / path).read_text()
        if before == after:
            continue
        chunks.append(f"diff --git a/{path} b/{path}\n")
        if absent:
            chunks.append("new file mode 100644\n")
        chunks.extend(difflib.unified_diff(before.splitlines(keepends=True), after.splitlines(keepends=True),
                      fromfile="/dev/null" if absent else "a/" + path, tofile="b/" + path, n=0))
    (OUT / name).write_text("".join(chunks))
    run(["git", "apply", "--reverse", "--check", "--unidiff-zero", str(OUT / name)])


try:
    assert run(["git", "rev-parse", "HEAD"]).strip() == BASE
    branch = run(["git", "branch", "--show-current"]).strip()
    assert branch == "codex/1419-lean-v4-20261018"
    run(["git", "diff", "--check", "--", "formal"])
    # --no-index 有新文件即 exit=1；无诊断不记成 exit=0。
    run(["git", "diff", "--no-index", "--check", "/dev/null", "formal/Origin/CenterPairFixtureExport.lean"], 1)
    assert COMMANDS[-1]["stdout"] == COMMANDS[-1]["stderr"] == ""
    tracked = set(run(["git", "diff", "--name-only", BASE]).splitlines())
    untracked = set(run(["git", "ls-files", "--others", "--exclude-standard"]).splitlines())
    report = str(OUT.relative_to(ROOT)) + "/"
    assert all(p in FILES or p.startswith(report) for p in tracked | untracked)
    assert (tracked | untracked) & set(FILES) == set(FILES)
    assert not run(["git", "diff", BASE, "--", "rust", "formal/lakefile.toml", "formal/lean-toolchain"])
    for item in json.loads((OUT / "INPUTS.json").read_text())["inputs"]:
        path = Path(item["path"])
        assert sha(path.read_bytes()) == item["sha256"], str(path)
    manifest = json.loads((OUT / "SOURCE-MANIFEST.json").read_text())
    for row in manifest:
        data = (ROOT / row["path"]).read_bytes()
        row["final_sha256"] = sha(data)
        row["final_bytes"] = len(data)
        row["donor_identical"] = row["final_sha256"] == row["donor_sha256"]
    write("SOURCE-MANIFEST.json", manifest)
    patch("DELTA.patch", BASE)
    patch("ADAPTATIONS.patch", DONOR)
    write("PACKAGE.json", {"classification": "#1419 工作草稿；未提交的 Lean 候选与静查证据",
          "base": BASE, "donor": DONOR, "branch": branch, "formal_files": FILES,
          "patches": {p: sha((OUT / p).read_bytes()) for p in ["DELTA.patch", "ADAPTATIONS.patch"]},
          "model_run": "CHECK-RUN-1.json", "final_edit": "模型核验后只订正说明注释，封包更新文件哈希；不重复模型",
          "lean_rust_fixture_runtime": "NOT_RUN", "review": "NOT_RUN",
          "hash_rule": "SHA256SUMS 覆盖全部 11 项 formal 及本目录全部其他文件，自身除外；路径相对 checkout 根。"})
finally:
    write("PACKAGE-COMMANDS.json", COMMANDS)

paths = [ROOT / p for p in FILES] + [p for p in OUT.iterdir() if p.is_file() and p.name != "SHA256SUMS"]
paths.sort(key=lambda p: str(p.relative_to(ROOT)))
lines = [sha(p.read_bytes()) + "  " + str(p.relative_to(ROOT)) for p in paths]
(OUT / "SHA256SUMS").write_text("\n".join(lines) + "\n")
print(f"封包通过：11 项 formal，{len(paths) - 11} 份报告/补丁/检查文件；SHA256SUMS 共 {len(paths)} 项。")
print("两个零上下文补丁反向只读检查均 exit=0；HEAD/分支/写集/22 项输入身份保持。")
print("DELTA.patch SHA256=" + sha((OUT / "DELTA.patch").read_bytes()))
print("SHA256SUMS SHA256=" + sha((OUT / "SHA256SUMS").read_bytes()))
