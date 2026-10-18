"""#1527 定向收据：仅静态、rustfmt stdin、AST、纯模型和普通小文件。"""

import ast
import hashlib
import json
from pathlib import Path
import re
import subprocess
import sys


ROOT = Path(__file__).resolve().parents[3]
OUT = Path(__file__).resolve().parent
BASE = "f7d34f47cba8a0ea0ceb55499206520f058da61f"
REVIEW = "3dde745d3909f882beac67de106a42d0f097aa65"
PRODUCTS = [
    "rust/src/bin/s_session_v3/admission.rs",
    "rust/src/bin/s_session_v3/admission_provider.rs",
    "s_session/tests/issue1527_recovery/provider_tests.rs",
]
RECEIPTS = []


def sha(data):
    return hashlib.sha256(data).hexdigest()


def command(argv, stdin=None):
    result = subprocess.run(argv, cwd=ROOT, input=stdin, capture_output=True, check=False)
    RECEIPTS.append({"argv": argv, "cwd": str(ROOT), "rc": result.returncode,
                     "stdin_sha256": sha(stdin) if stdin is not None else None,
                     "stdout_sha256": sha(result.stdout), "stdout_bytes": len(result.stdout),
                     "stderr": result.stderr.decode(errors="replace"),
                     "stdout": result.stdout.decode(errors="replace") if len(result.stdout) < 1500 else None})
    if result.returncode:
        raise AssertionError(f"命令失败 rc={result.returncode}: {argv}")
    return result.stdout


def blob(path):
    return command(["git", "show", f"{BASE}:{path}"])


def function(source, signature):
    start = source.index(signature)
    opening = source.index("{", start)
    depth = 0
    for i in range(opening, len(source)):
        depth += (source[i] == "{") - (source[i] == "}")
        if depth == 0:
            return source[start:i + 1]
    raise AssertionError(signature)


def rebuild_patch(patch):
    files = {}
    for section in patch.split(b"diff --git ")[1:]:
        lines = section.splitlines(keepends=True)
        path = lines[0].split(b" b/", 1)[1].strip().decode()
        old = blob(path).splitlines(keepends=True)
        built = []
        cursor = 0
        i = next(i for i, line in enumerate(lines) if line.startswith(b"@@"))
        hunks = 0
        while i < len(lines):
            header = re.match(rb"@@ -(\d+)(?:,(\d+))? \+(\d+)(?:,(\d+))? @@", lines[i])
            assert header, lines[i]
            old_start = int(header[1]) - 1
            old_count = int(header[2] or b"1")
            new_start = int(header[3]) - 1
            new_count = int(header[4] or b"1")
            built.extend(old[cursor:old_start])
            assert len(built) == new_start
            cursor = old_start
            consumed = produced = 0
            i += 1
            while i < len(lines) and not lines[i].startswith(b"@@"):
                kind, data = lines[i][:1], lines[i][1:]
                assert kind in (b" ", b"+", b"-"), lines[i]
                if kind in (b" ", b"-"):
                    assert old[cursor] == data, (path, cursor)
                    cursor += 1
                    consumed += 1
                if kind in (b" ", b"+"):
                    built.append(data)
                    produced += 1
                i += 1
            assert (consumed, produced) == (old_count, new_count)
            hunks += 1
        built.extend(old[cursor:])
        rebuilt = b"".join(built)
        assert rebuilt == (ROOT / path).read_bytes()
        files[path] = {"hunks": hunks, "rebuilt_sha256": sha(rebuilt)}
    assert sorted(files) == sorted(PRODUCTS)
    return files


def checks():
    result = {"issue": 1527, "base": BASE, "review_source": REVIEW,
              "scope": "H-01/M-02 候选返修；非 Rust 运行/完整准入验收",
              "actual": {}, "commands": RECEIPTS}
    actual = result["actual"]
    assert command(["git", "rev-parse", "HEAD"]).decode().strip() == BASE
    branch = command(["git", "branch", "--show-current"]).decode().strip()
    assert branch.startswith("codex/"), branch
    actual["branch"] = branch
    changed = command(["git", "diff", "--name-only", BASE]).decode().splitlines()
    assert sorted(changed) == sorted(PRODUCTS), changed
    untracked = command(["git", "ls-files", "--others", "--exclude-standard"]).decode().splitlines()
    assert all(p.startswith(str(OUT.relative_to(ROOT)) + "/") for p in untracked)
    actual["tracked_write_set"] = changed

    history = command(["git", "ls-tree", "-r", "--name-only", BASE,
                       ".chanlun/review-results/issue1527-formal-i1-20261018",
                       ".chanlun/review-results/issue1527-i1-b2b3-20261018"]).decode().splitlines()
    actual["historical_reports_unchanged"] = {}
    for p in history:
        original = blob(p)
        current = (ROOT / p).read_bytes()
        assert original == current, p
        actual["historical_reports_unchanged"][p] = {"sha256": sha(current), "bytes": len(current)}
    review_path = ".chanlun/review-results/issue1527-formal-i1-review-20261018/REVIEW.md"
    review = command(["git", "-C", str(ROOT.parent / "formal-i1-review-worktree"),
                      "show", f"{REVIEW}:{review_path}"])
    actual["fixed_review"] = {"path": review_path, "sha256": sha(review), "bytes": len(review)}

    # 保 B2/B3 和 #1488 等写集外源码原字节；未运行其测试。
    protected = [
        "rust/src/bin/s_session_v3/admission_anchor.rs",
        "rust/src/bin/s_session_v3/admission_private/mod.rs",
        "rust/src/bin/s_session_v3/admission_private/fd_wal.rs",
        "rust/src/bin/s_session_v3/state.rs", "rust/src/bin/s_session_v3/audit_owner.rs",
        "rust/src/bin/s_session_v3/driver.rs", "rust/src/bin/s_session_v3/writer_cli.rs",
        "rust/src/bin/s_session_v3/mod.rs", "rust/src/bin/s_session_v2/mod.rs",
        "rust/src/bin/s_structure_session.rs", "rust/src/s_content/job.rs",
        "s_session/tests/issue1527_recovery/b2b3_terminal_tests.rs",
    ]
    actual["protected_bytes_unchanged"] = {}
    for p in protected:
        current = (ROOT / p).read_bytes()
        assert current == blob(p), p
        actual["protected_bytes_unchanged"][p] = sha(current)
    old_tests = blob(PRODUCTS[2])
    assert (ROOT / PRODUCTS[2]).read_bytes().startswith(old_tests)
    actual["existing_provider_tests_prefix_unchanged"] = sha(old_tests)

    old_admission = blob(PRODUCTS[0]).decode()
    admission = (ROOT / PRODUCTS[0]).read_text()
    signatures = ["pub(crate) fn write_result(", "pub(crate) fn read_current_text(",
                  "pub(crate) fn read_profile_text(", "pub(crate) fn load_startup_profile(",
                  "pub(crate) fn write_current_result(", "pub(crate) fn read_startup(",
                  "pub(crate) fn require_current("]
    for signature in signatures:
        assert function(old_admission, signature) == function(admission, signature), signature
    actual["b2_original_io_bodies_unchanged"] = signatures
    finish = function(admission, "pub(crate) fn finish_startup(")
    gate = function(admission, "fn startup_gate(")
    assert finish.index("return Err(error.clone())") < finish.index("finish_output()")
    assert finish.index("return Err(error);") < finish.index("state.take()")
    assert finish.index("state.take()") < finish.index("finish_terminal_startup()")
    assert gate.index("startup_failure") < gate.index("check_deadline()")
    assert all(word not in finish for word in ("Job::", "Instant::now", "release(", "reconcile("))
    actual["m02_static_control_flow"] = "首错提前返回；失败在清槽/终端撤账前返回；无换账/续期/释放预留"

    provider = (ROOT / PRODUCTS[1]).read_text().split("#[cfg(test)]", 1)[0]
    old_provider = blob(PRODUCTS[1]).decode()
    pin = function(provider, "fn open(path:")
    assert pin == function(old_provider, "fn open(path:")
    for forbidden in (".try_lock(", ".create(", ".truncate(", "remove_file("):
        assert forbidden not in provider
    for required in (".checked_mul(512)", ".checked_add(physical)",
                     '".control.lock"', '"UnknownDatabaseSidecar"',
                     '"ControlLockNotRegular"', "p.verify(true)?;"):
        assert required in provider
    actual["h01_static_pin_and_accounting"] = "沿原 Pinned 固定FD/身份及 st_blocks×512；无重取控制锁/创建/删除"

    for p in PRODUCTS:
        command(["rustfmt", "--edition", "2021", "--emit", "stdout", "--config", "skip_children=true"],
                (ROOT / p).read_bytes())
    actual["rustfmt_stdin"] = {"files": PRODUCTS, "meaning": "仅语法解析，不证明类型/借用/编译或执行"}
    actual["python_ast"] = []
    for p in (OUT / "models.py", OUT / "verify.py"):
        ast.parse(p.read_bytes(), filename=str(p))
        actual["python_ast"].append({"path": str(p.relative_to(ROOT)), "sha256": sha(p.read_bytes())})

    model_data = command([sys.executable, str((OUT / "models.py").relative_to(ROOT))])
    model = json.loads(model_data)
    assert all(row["candidate_admitted_by_suffix"] for row in model["h01"])
    assert all(row["contract_satisfied_in_model"] for row in model["m02_candidate"]["cases"])
    assert not model["m02_fixed_baseline"]["cases"][2]["contract_satisfied_in_model"]
    assert not any(row["contract_satisfied_in_model"] for row in model["m02_reviewed"]["cases"][:3])
    (OUT / "AFTER-MODELS.json").write_bytes(model_data)
    actual["models"] = {"path": str((OUT / "AFTER-MODELS.json").relative_to(ROOT)),
                        "sha256": sha(model_data), "scope": model["scope"]}
    actual["before_models"] = {"sha256": sha((OUT / "BEFORE-MODELS.json").read_bytes()),
                               "meaning": "返修前普通文件/模型反例；不冒充 Rust 失败运行"}
    command(["git", "diff", "--check"])
    patch = command(["git", "diff", "--binary", BASE, "--", *PRODUCTS])
    (OUT / "FINAL.patch").write_bytes(patch)
    actual["exact_patch_rebuild"] = rebuild_patch(patch)
    actual["final_patch_sha256"] = sha(patch)
    actual["products"] = {p: {"sha256": sha((ROOT / p).read_bytes()), "bytes": (ROOT / p).stat().st_size}
                          for p in PRODUCTS}
    test_names = []
    for p in PRODUCTS:
        before = set(re.findall(r"#\[test\]\s+fn (\w+)", blob(p).decode()))
        after = set(re.findall(r"#\[test\]\s+fn (\w+)", (ROOT / p).read_text()))
        test_names.extend({"file": p, "name": n, "status": "NOT_RUN"} for n in sorted(after - before))
    assert len(test_names) == 8
    result["rust_test_sources"] = test_names
    result["inherited"] = [
        "旧独评 H-01/M-02 及 B1；只取固定 Git 报告，不重写旧结论",
        "f7d34f47 的 B2 同账/终端stderr和 B3 S3原算法审计；本轮核原字节，未重跑",
        "父目录 XERJ 缓存仅作既有检索参考；未安装/索引/联网",
    ]
    result["not_run"] = ["cargo/rustc/build", "全部 Rust 测试/类型/借用/ABI/Drop运行",
                         "SQLite 实际执行/Controller和Rust跨进程联调", "正式S/Q/旧binary/历史重型",
                         "完整9011/资源峰值/恢复/金融隔离独立双跑", "继承模型/测试重跑", "网络"]
    result["open"] = ["B1 D2生产RAM", "S2/legacy旧域计量", "I1最终验收", "I2", "I3", "I4", "I5",
                      "9011", "64GiB/store", "AgentStorage148GiB", "金融/隔离/真实恢复/独立双跑/全部六目标"]
    result["status"] = "PASS_STATIC_AND_MODELS_ONLY"
    return result


def manifest():
    checks_data = json.loads((OUT / "CHECKS.json").read_text())
    for p, record in checks_data["actual"]["products"].items():
        assert sha((ROOT / p).read_bytes()) == record["sha256"], p
    files = [ROOT / p for p in PRODUCTS]
    files += [p for p in sorted(OUT.iterdir()) if p.is_file() and p.name != "MANIFEST.json"]
    data = {"issue": 1527, "base": BASE, "review_source": REVIEW,
            "manager": "/root/manage_1527_sandcastle",
            "parent_thread": "01a12e01-45a1-7173-a3e8-d338d108fb93",
            "scope_status": "completed", "production_status": "BLOCK_NOT_MERGEABLE",
            "description": "仅H-01/M-02候选返修已交付，待经理冻结后独审；未授权后续动作",
            "self_hash_policy": "不含自身，避免递归摘要",
            "files": {str(p.relative_to(ROOT)): {"sha256": sha(p.read_bytes()), "bytes": p.stat().st_size}
                      for p in files},
            "open": checks_data["open"], "not_run": checks_data["not_run"]}
    (OUT / "MANIFEST.json").write_text(json.dumps(data, ensure_ascii=False, indent=2) + "\n")
    print(json.dumps({"files": len(files), "manifest_sha256": sha((OUT / "MANIFEST.json").read_bytes())}))


if __name__ == "__main__":
    if sys.argv[1:] == ["manifest"]:
        manifest()
    else:
        try:
            result = checks()
        except Exception as exc:
            (OUT / "FAILED-CHECKS.json").write_text(json.dumps(
                {"error": str(exc), "commands": RECEIPTS}, ensure_ascii=False, indent=2) + "\n")
            raise
        (OUT / "CHECKS.json").write_text(json.dumps(result, ensure_ascii=False, indent=2) + "\n")
        print(json.dumps({"status": result["status"], "command_receipts": len(RECEIPTS),
                          "historical_reports": len(result["actual"]["historical_reports_unchanged"]),
                          "rust_tests_not_run": len(result["rust_test_sources"])}))
