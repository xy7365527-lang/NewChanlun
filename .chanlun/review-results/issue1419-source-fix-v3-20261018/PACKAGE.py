"""#1419 只读封包：生成产品 patch/来源清单，不修改 Git 或产品源码。"""
from pathlib import Path
import hashlib
import json
import subprocess

ROOT = Path(__file__).resolve().parents[3]
OUT = Path(__file__).resolve().parent
HEAD = "2694afcf147f3ba4114fde386470886c2428efc0"
BASE = "5cb7253d46d973669e7d10cd3331e12c9ef313b1"
DONOR = "634f62cddfaddf656a35ca9b9db20cb716b62bff"
commands = []


def run(args):
    return subprocess.run(args, cwd=ROOT, capture_output=True)


def checked(args):
    result = run(args)
    commands.append({"argv": args, "exit_code": result.returncode,
                     "stdout": result.stdout.decode(), "stderr": result.stderr.decode()})
    return result


def sha(data):
    return hashlib.sha256(data).hexdigest()


def identity(data):
    if data is None:
        return None
    return {"bytes": len(data), "sha256": sha(data), "git_blob": hashlib.sha1(
        b"blob " + str(len(data)).encode() + b"\0" + data).hexdigest()}


def at(rev, name):
    result = run(["git", "show", f"{rev}:{name}"])
    return result.stdout if result.returncode == 0 else None


assert checked(["git", "rev-parse", "HEAD"]).stdout.decode().strip() == HEAD
new = checked(["git", "ls-files", "--others", "--exclude-standard", "--", "rust", "s_session"]).stdout.decode().splitlines()
paths = checked(["git", "diff", "--name-only", BASE, "--", "rust", "s_session"]).stdout.decode().splitlines()
delta = checked(["git", "diff", "--name-only", HEAD, "--", "rust", "s_session"]).stdout.decode().splitlines()
paths, delta = sorted(set(paths + new)), sorted(set(delta + new))

manifest = []
for name in paths:
    data = (ROOT / name).read_bytes()
    donor = at(DONOR, name)
    manifest.append({"path": name, "baseline": identity(at(BASE, name)),
                     "repair_head": identity(at(HEAD, name)), "donor": identity(donor),
                     "current": identity(data), "changed_this_round": name in delta,
                     "donor_exact_bytes": data == donor})
(OUT / "SOURCE-MANIFEST.json").write_text(json.dumps({
    "baseline": BASE, "repair_head": HEAD, "donor": DONOR,
    "scope": "产品相对固定 5cb 基线的完整差异集合；含本轮未跟踪新源码，不含报告。非全仓编译封印。",
    "files": manifest,
}, ensure_ascii=False, indent=2) + "\n")
(OUT / "WRITESET.json").write_text(json.dumps({
    "head": HEAD, "product_files": delta,
    "report_directory": str(OUT.relative_to(ROOT)),
    "note": "报告文件逐项见 REPORT-MANIFEST.json；旧 v2 报告及原件未改。",
}, ensure_ascii=False, indent=2) + "\n")

for rev, target in [(BASE, "SOURCE.patch"), (HEAD, "DELTA.patch")]:
    args = ["git", "diff", "--no-ext-diff", "--binary", "--full-index", "--unified=0", rev, "--", "rust", "s_session"]
    result = run(args)
    assert result.returncode == 0
    patch = result.stdout
    for name in new:
        extra_args = ["git", "diff", "--no-index", "--binary", "--full-index", "--unified=0", "--", "/dev/null", name]
        extra = run(extra_args)
        assert extra.returncode == 1, extra.stderr.decode()
        patch += extra.stdout
        commands.append({"argv": extra_args, "exit_code": extra.returncode,
                         "stdout_in": target, "stdout_sha256": sha(extra.stdout),
                         "stderr": extra.stderr.decode(), "note": "--no-index exit 1 表示存在差异。"})
    (OUT / target).write_bytes(patch)
    commands.append({"argv": args, "exit_code": result.returncode, "stdout_in": target,
                     "stdout_sha256": sha(result.stdout), "stderr": result.stderr.decode(),
                     "note": "文件尾追加上述新源码差异，完整 artifact SHA 见报告清单。"})
    verify = checked(["git", "apply", "--reverse", "--check", "--unidiff-zero", str(OUT / target)])
    assert verify.returncode == 0, verify.stderr.decode()

inputs = []
source_paths = [
    "/Volumes/AgentStorage/issue1419-sandcastle-manager-20261018/SOURCE-V2-INDEPENDENT-REVIEW.json",
    "/Volumes/AgentStorage/issue1419-sandcastle-manager-20261018/issue1419-live.json",
    "/Volumes/AgentStorage/issue1419-sandcastle-manager-20261018/run01-artifacts/XERJ-ATTEMPTS.json",
    "/Volumes/AgentStorage/issue1419-sandcastle-manager-20261018/run01-artifacts/REFERENCE-HITS.json",
]
old = ROOT / ".chanlun/review-results/issue1419-native-semantic-v2-20261018"
source_paths += [str(p) for p in sorted(old.iterdir()) if p.is_file()]
for name in source_paths:
    path = Path(name)
    inputs.append({"path": str(path), **identity(path.read_bytes()),
                   "role": "复用来源，不视为已验证新树"})
for name in ["formal/Formal/RecursiveConstruction.lean", "formal/Origin/CenterConstruct.lean",
             "formal/Origin/CenterPairFixtureExport.lean"]:
    inputs.append({"path": name, "donor_revision": DONOR, "donor": identity(at(DONOR, name)),
                   "checkout": identity((ROOT / name).read_bytes()) if (ROOT / name).exists() else None,
                   "role": "只读源码引用，Lean NOT_RUN"})
(OUT / "INPUTS.json").write_text(json.dumps(inputs, ensure_ascii=False, indent=2) + "\n")

fmt = checked(["rustfmt", "--edition", "2021", "--config", "skip_children=true", "--check",
               "rust/src/theta_v0/parser/second_fold/wire_match.rs"])
assert fmt.returncode == 0
product_check = checked(["git", "diff", "--check", BASE, "--", "rust", "s_session"])
assert product_check.returncode == 0
new_artifacts_with_trailing = []
for path in sorted(OUT.rglob("*")):
    if path.is_file() and path.suffix in {".md", ".py", ".json", ".patch"}:
        for line, text in enumerate(path.read_text().splitlines(), 1):
            if text.rstrip() != text:
                new_artifacts_with_trailing.append({"path": str(path.relative_to(ROOT)), "line": line})
(OUT / "PACKAGE-CHECKS.json").write_text(json.dumps({
    "product_files": len(paths), "changed_rust_files": len(delta),
    "source_and_delta_reverse_check": "PASS_READ_ONLY", "new_artifact_trailing_whitespace": new_artifacts_with_trailing,
    "product_diffcheck": "PASS", "full_tree_diffcheck": "FAIL_ARTIFACT_V2_SOURCE_PATCH_EXIT_2",
    "note": "新 patch 用零上下文避免空上下文行混同空白错误；应用时须 --unidiff-zero。不修改旧报告。",
    "rust_compiled": False, "tests_executed": False, "approved": False,
}, ensure_ascii=False, indent=2) + "\n")
(OUT / "PACKAGE-COMMANDS.json").write_text(json.dumps(commands, ensure_ascii=False, indent=2) + "\n")
print(json.dumps({"product_files": len(paths), "changed_rust_files": len(delta),
                  "new_artifact_trailing_whitespace": len(new_artifacts_with_trailing),
                  "patch_checks": "PASS_READ_ONLY"}, ensure_ascii=False, indent=2))
