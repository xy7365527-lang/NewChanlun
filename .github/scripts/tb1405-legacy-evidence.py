#!/usr/bin/env python3
"""#1405 专用历史取证：保留原命令退出码，封存实际产物，不推定验收通过。"""

import hashlib
import json
from pathlib import Path
import re
import sqlite3
import subprocess
import sys

BASE = "b46ba296993371cb56b2dff7ef878647dc0665ad"
RULE = "s2-axis-quantifiers+new-bi/1"
ALLOWED = {
    ".github/workflows/ci-1405-legacy-evidence.yml",
    ".github/scripts/tb1405-legacy-evidence.py",
    "rust/src/bin/s_session_v3/source_audit.rs",
    "rust/src/bin/s_session_v3/legacy_v1_export_tests.rs",
}
TESTS = {
    "old-lock": "v3::source_audit::tests::full_some_audit_keeps_frozen_read_and_budget_trace",
    "source": "v3::source_audit::tests::legacy_v1_export::tb1405_export_legacy_v1_source_nodes",
    "database": "v3::source_audit::tests::legacy_v1_export::tb1405_export_legacy_v1_persistent_database",
}


def digest(data):
    return hashlib.sha256(data).hexdigest()


def write_json(path, value):
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(value, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")


def git(repo, *args):
    return subprocess.check_output(["git", "-C", str(repo), *args], text=True).strip()


def provenance(out, baseline, candidate):
    assert git(baseline, "rev-parse", "HEAD") == BASE
    assert not git(baseline, "status", "--porcelain", "--untracked-files=no")
    assert not git(candidate, "status", "--porcelain", "--untracked-files=no")
    changed = set(git(candidate, "diff", "--name-only", BASE, "HEAD").splitlines())
    assert changed == ALLOWED, changed
    old_source = baseline / "rust/src/bin/s_session_v3/source_audit.rs"
    adapted_source = candidate / "rust/src/bin/s_session_v3/source_audit.rs"
    insertion = (b'    mod legacy_v1_export {\n'
                 b'        include!("legacy_v1_export_tests.rs");\n'
                 b'    }\n\n')
    assert adapted_source.read_bytes().count(insertion) == 1
    assert adapted_source.read_bytes().replace(insertion, b"", 1) == old_source.read_bytes()
    result = {"base": BASE, "changed_paths": sorted(changed), "checkouts": {}}
    for name, repo in (("baseline", baseline), ("adapter", candidate)):
        files = subprocess.check_output([
            "git", "-C", str(repo), "ls-files", "-z", "rust", "s_session/catalog",
            "s_session/profiles", ".github/scripts/tb1405-legacy-evidence.py",
            ".github/workflows/ci-1405-legacy-evidence.yml",
        ]).decode().split("\0")
        hashes = {p: digest((repo / p).read_bytes()) for p in files if p and (repo / p).is_file()}
        write_json(out / f"{name}-source-sha256.json", hashes)
        result["checkouts"][name] = {
            "head": git(repo, "rev-parse", "HEAD"),
            "tree": git(repo, "rev-parse", "HEAD^{tree}"),
            "cargo_lock_sha256": hashes["rust/Cargo.lock"],
            "source_manifest_sha256": digest((out / f"{name}-source-sha256.json").read_bytes()),
        }
    result["production_sources_identical"] = True
    result["old_lock_source_identical"] = True
    write_json(out / "PROVENANCE.json", result)


def run_command(out, repo, label, command, *, echo_stdout=True):
    stdout, stderr = out / f"{label}.stdout.log", out / f"{label}.stderr.log"
    receipt = {"command": command, "head": git(repo, "rev-parse", "HEAD"),
               "cwd": str(repo), "status": "started"}
    write_json(out / f"{label}.json", receipt)
    with stdout.open("wb") as so, stderr.open("wb") as se:
        completed = subprocess.run(command, cwd=repo, stdout=so, stderr=se, check=False)
    # The original command status is persisted before any acceptance check.
    receipt.update(status="finished", raw_exit_code=completed.returncode,
                   stdout_sha256=digest(stdout.read_bytes()), stderr_sha256=digest(stderr.read_bytes()))
    write_json(out / f"{label}.json", receipt)
    if echo_stdout:
        sys.stdout.buffer.write(stdout.read_bytes())
    sys.stderr.buffer.write(stderr.read_bytes())
    return receipt


def test_binary(text, repo, target):
    messages = [json.loads(line) for line in text.splitlines() if line.strip()]
    finished = [m for m in messages if m.get("reason") == "build-finished"]
    if len(finished) != 1 or finished[0].get("success") is not True:
        raise ValueError("missing successful cargo build-finished identity")
    artifacts = [m for m in messages if m.get("reason") == "compiler-artifact"
                 and m.get("target", {}).get("name") == "s_structure_session"
                 and m.get("profile", {}).get("test") is True
                 and m.get("executable") is not None]
    if len(artifacts) != 1:
        raise ValueError("expected exactly one s_structure_session test executable")
    artifact = artifacts[0]
    executable = Path(artifact["executable"]).resolve()
    if (Path(artifact["manifest_path"]).resolve() != repo / "rust/Cargo.toml"
            or Path(artifact["target"]["src_path"]).resolve()
            != repo / "rust/src/bin/s_structure_session.rs"
            or artifact["target"].get("kind") != ["bin"]
            or "s_session" not in artifact.get("features", [])
            or not executable.is_relative_to(target)
            or not executable.is_file()):
        raise ValueError("test artifact does not match checkout, target directory or feature")
    return executable, artifact


def exact_test_list(text, name):
    # The same executable will run with this exact filter; a bench or zero match is rejected.
    return [line.strip() for line in text.splitlines() if line.strip()] == [
        f"{name}: test", "1 test, 0 benchmarks"]


def one_test_passed(text, name):
    # Identity comes from the exact list plus the sole named invocation, not an arbitrary
    # substring. Logs may split the named status line; the sole harness summary must
    # independently prove one executed test, no failure, no ignored test and no bench.
    starts = re.findall(r"^test ([\w:]+) \.\.\. ", text, re.MULTILINE)
    running = re.findall(r"^running (\d+) tests?$", text, re.MULTILINE)
    summaries = re.findall(
        r"^test result: (\w+)\. (\d+) passed; (\d+) failed; (\d+) ignored; "
        r"(\d+) measured; \d+ filtered out; finished in [0-9.]+s$", text, re.MULTILINE)
    return (starts == [name] and running == ["1"]
            and summaries == [("ok", "1", "0", "0", "0")]
            and re.search(r"(?:^|\.\.\. )ok\n\s*\ntest result:", text, re.MULTILINE) is not None)


def run_test(out, repo, label):
    name, head = TESTS[label], git(repo, "rev-parse", "HEAD")
    role = "baseline" if label == "old-lock" else "adapter"
    target = (out.parent / "cargo-targets" / f"{role}-{head}").resolve()
    validation = {"test": name, "head": head, "cwd": str(repo), "role": role,
                  "target_dir": str(target), "accepted": False, "phases": {}}

    def finish(code, reason):
        validation.update(wrapper_exit_code=code, reason=reason, accepted=code == 0)
        write_json(out / f"{label}-validation.json", validation)
        return code

    def phase(suffix, command, **kwargs):
        # Cargo runs tests in the package root, even with --manifest-path elsewhere.
        cwd = repo if command[0] == "cargo" else repo / "rust"
        receipt = run_command(out, cwd, suffix, command, **kwargs)
        validation["phases"][suffix] = receipt
        write_json(out / f"{label}-validation.json", validation)
        return receipt["raw_exit_code"]

    def raw_failure(code, reason):
        return finish(code if code > 0 else 128 - code, reason)

    write_json(out / f"{label}-validation.json", validation)
    command = ["cargo", "test", "--locked", "--manifest-path", "rust/Cargo.toml",
               "--features", "s_session", "--bin", "s_structure_session", "--jobs", "1",
               "--target-dir", str(target), "--no-run", "--message-format=json"]
    code = phase(f"{label}-build", command, echo_stdout=False)
    if code:
        return raw_failure(code, "cargo build failed")
    try:
        executable, artifact = test_binary(
            (out / f"{label}-build.stdout.log").read_text(encoding="utf-8"), repo, target)
        binary_hash = digest(executable.read_bytes())
        validation["binary"] = {"path": str(executable), "sha256": binary_hash,
                                "sha256_before_list": binary_hash, "cargo_artifact": artifact}
        code = phase(f"{label}-list", [str(executable), name, "--exact", "--list", "--format=pretty"])
        if code:
            return raw_failure(code, "exact test list failed")
        validation["binary"]["sha256_after_list"] = digest(executable.read_bytes())
        if not exact_test_list((out / f"{label}-list.stdout.log").read_text(encoding="utf-8"), name):
            return finish(97, "exact list is not the unique requested test")
        if validation["binary"]["sha256_after_list"] != binary_hash:
            return finish(97, "test executable changed during list")
        # Keep libtest capture enabled, preventing normal test prints from splitting
        # its named status line. Failure output still remains in the original logs.
        validation["binary"]["sha256_before_run"] = digest(executable.read_bytes())
        if validation["binary"]["sha256_before_run"] != binary_hash:
            return finish(97, "test executable changed before run")
        code = phase(label, [str(executable), name, "--exact", "--format=pretty", "--test-threads=1"])
        if code:
            return raw_failure(code, "test executable failed")
        validation["binary"]["sha256_after_run"] = digest(executable.read_bytes())
        if validation["binary"]["sha256_after_run"] != binary_hash:
            return finish(97, "test executable changed during run")
        if not one_test_passed((out / f"{label}.stdout.log").read_text(encoding="utf-8"), name):
            return finish(97, "run did not prove exactly one named, passed, nonignored test")
    except (KeyError, ValueError, OSError) as error:
        return finish(97, f"evidence identity rejected: {error}")
    return finish(0, "same binary uniquely listed and executed requested test")


def self_test(out):
    """Synthetic command-boundary controls only; never invokes cargo or a Rust binary."""
    import io
    from unittest.mock import patch

    head, repo = "a" * 40, (out / "synthetic-checkout").resolve()
    name = TESTS["source"]
    summary = "test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 530 filtered out; finished in 0.01s\n"
    passed = f"\nrunning 1 test\ntest {name} ... ok\n\n{summary}"
    cases = {
        "normal": {},
        "baseline_role_isolation": {"label": "old-lock"},
        "stdout_interleaved": {"run_text": passed.replace("... ok", '... {"ok":true}\nok')},
        "extra_stdout": {"run_text": "before test\n" + passed + "after test\n"},
        "zero_run": {"run_text": passed.replace("running 1 test", "running 0 tests").replace("1 passed", "0 passed"), "expect": 97},
        "wrong_run_name": {"run_text": passed.replace(name, "wrong::test"), "expect": 97},
        "ignored": {"run_text": passed.replace("... ok", "... ignored").replace("1 passed", "0 passed").replace("0 ignored", "1 ignored"), "expect": 97},
        "failed_summary_exit_zero": {"run_text": passed.replace("result: ok", "result: FAILED").replace("1 passed", "0 passed").replace("0 failed", "1 failed"), "expect": 97},
        "missing_named_finish": {"run_text": passed.replace("... ok", "... arbitrary log"), "expect": 97},
        "duplicate_summary": {"run_text": passed + summary, "expect": 97},
        "multiple_named_tests": {"run_text": passed.replace("\n\ntest result:", "\ntest other::test ... ok\n\ntest result:"), "expect": 97},
        "zero_list": {"list_text": "0 tests, 0 benchmarks\n", "expect": 97},
        "wrong_list_name": {"list_text": "wrong::test: test\n\n1 test, 0 benchmarks\n", "expect": 97},
        "bench_list": {"list_text": f"{name}: benchmark\n\n0 tests, 1 benchmark\n", "expect": 97},
        "extra_list_entry": {"list_text": f"{name}: test\nother: test\n\n2 tests, 0 benchmarks\n", "expect": 97},
        "build_exit_7": {"build_exit": 7, "expect": 7},
        "build_signal": {"build_exit": -9, "expect": 137},
        "list_exit_8": {"list_exit": 8, "expect": 8},
        "run_exit_9_with_pass_text": {"run_exit": 9, "expect": 9},
        "run_signal": {"run_exit": -15, "expect": 143},
        "wrong_manifest": {"artifact_error": "manifest_path", "expect": 97},
        "wrong_source": {"artifact_error": "src_path", "expect": 97},
        "missing_feature": {"artifact_error": "features", "expect": 97},
        "wrong_role_target": {"artifact_error": "executable", "expect": 97},
        "duplicate_artifact": {"artifact_error": "duplicate", "expect": 97},
        "unsuccessful_build_identity": {"artifact_error": "build_success", "expect": 97},
        "changed_after_list": {"change_at": "list", "expect": 97},
        "changed_after_run": {"change_at": "run", "expect": 97},
    }
    results = []
    for case, settings in cases.items():
        test_label = settings.get("label", "source")
        test_name = TESTS[test_label]
        destination = out / case
        destination.mkdir(parents=True, exist_ok=True)
        commands = []
        executable = None

        def fake_run(command, *, cwd, stdout, stderr, check):
            nonlocal executable
            commands.append(command)
            if command[0] == "cargo":
                stage = "build"
                assert cwd == repo
                target = Path(command[command.index("--target-dir") + 1])
                role = "baseline" if test_label == "old-lock" else "adapter"
                assert target.name == f"{role}-{head}" and target.is_absolute()
                assert "--no-run" in command and "--message-format=json" in command
                assert "--locked" in command and command[command.index("--features") + 1] == "s_session"
                executable = target / "debug/deps/s_structure_session-synthetic"
                executable.parent.mkdir(parents=True, exist_ok=True)
                executable.write_bytes(b"synthetic binary identity; not executable")
                artifact = {"reason": "compiler-artifact", "manifest_path": str(repo / "rust/Cargo.toml"),
                            "target": {"name": "s_structure_session", "kind": ["bin"],
                                       "src_path": str(repo / "rust/src/bin/s_structure_session.rs")},
                            "profile": {"test": True}, "features": ["s_session"], "executable": str(executable)}
                error = settings.get("artifact_error")
                if error == "manifest_path":
                    artifact["manifest_path"] = str(repo / "wrong/Cargo.toml")
                elif error == "src_path":
                    artifact["target"]["src_path"] = str(repo / "wrong.rs")
                elif error == "features":
                    artifact["features"] = []
                elif error == "executable":
                    artifact["executable"] = str(out / "wrong-role-binary")
                messages = [artifact] * (2 if error == "duplicate" else 1)
                messages.append({"reason": "build-finished", "success": error != "build_success"})
                text = "\n".join(json.dumps(m) for m in messages) + "\n"
            else:
                assert cwd == repo / "rust"
                assert command[0] == str(executable) and test_name in command and "--exact" in command
                assert "--nocapture" not in command
                stage = "list" if "--list" in command else "run"
                text = settings.get(f"{stage}_text", f"{test_name}: test\n\n1 test, 0 benchmarks\n" if stage == "list" else passed.replace(name, test_name))
                if settings.get("change_at") == stage:
                    executable.write_bytes(b"changed synthetic identity")
            stdout.write(text.encode())
            stderr.write(b"synthetic stderr\n")
            return subprocess.CompletedProcess(command, settings.get(f"{stage}_exit", 0))

        capture = type("Capture", (), {"buffer": io.BytesIO()})
        with patch.dict(globals(), {"git": lambda *_: head}), patch.object(subprocess, "run", side_effect=fake_run), patch.object(sys, "stdout", capture()), patch.object(sys, "stderr", capture()):
            code = run_test(destination, repo, test_label)
        validation = json.loads((destination / f"{test_label}-validation.json").read_text())
        assert code == settings.get("expect", 0), (case, code, validation)
        assert validation["accepted"] == (code == 0)
        for phase_name, receipt in validation["phases"].items():
            stage = "run" if phase_name == test_label else phase_name.removeprefix(f"{test_label}-")
            assert receipt["raw_exit_code"] == settings.get(f"{stage}_exit", 0), case
            assert receipt["stdout_sha256"] == digest((destination / f"{phase_name}.stdout.log").read_bytes())
            assert receipt["stderr_sha256"] == digest((destination / f"{phase_name}.stderr.log").read_bytes())
        if "list" in case and code:
            assert len(commands) == 2, case
        results.append({"case": case, "wrapper_exit": code, "commands": len(commands), "passed": True})
    write_json(out / "RESULT.json", {"synthetic_only": True, "cargo_executed": False,
                                     "tests": results, "passed": len(results)})
    print(f"#1405 synthetic command/evidence controls: {len(results)} passed; no Rust executed")


def verify(out):
    source, persistent = out / "originals/source-audit", out / "originals/persistent"
    manifest = json.loads((source / "COMPLETE.json").read_text())
    assert manifest["producer_base"] == BASE and manifest["rule_revision"] == RULE
    assert manifest["raw_count"] > 0 and manifest["counts"]["objects"] > 0
    bodies = (source / "node-bodies.bin").read_bytes()
    assert digest(bodies) == manifest["node_bodies_sha256"]
    nodes = json.loads((source / "nodes.json").read_text())
    assert len(nodes) == manifest["node_count"] > 0
    seen, next_offset = set(), 0
    for node in nodes:
        offset, length = int(node["offset"]), int(node["length"])
        assert offset == next_offset and length >= 0
        body = bodies[offset:offset + length]
        assert len(body) == length and digest(body) == node["body_sha256"]
        framed = b"NewChanlun/SContent/1\0" + bytes([node["tag"]]) + length.to_bytes(8, "big") + body
        assert node["root"] == "sc1-" + digest(framed) and node["root"] not in seen
        seen.add(node["root"])
        next_offset += length
    assert next_offset == len(bodies)
    assert json.loads((source / "state-ref.json").read_text())["root"] in seen
    database = persistent / "s.sqlite"
    info = json.loads((persistent / "COMPLETE.json").read_text())
    assert info["producer_base"] == BASE and info["rule_revision"] == RULE
    original_hash = digest(database.read_bytes())
    assert original_hash == info["database_sha256"]
    # immutable=1 prevents sidecar creation or mutations in the frozen original directory.
    connection = sqlite3.connect(database.resolve().as_uri() + "?mode=ro&immutable=1", uri=True)
    try:
        assert connection.execute("PRAGMA integrity_check").fetchone() == ("ok",)
        assert connection.execute("SELECT value FROM meta WHERE key='rule_revision'").fetchone() == (RULE,)
        for table in ("s_content_nodes", "s_content_publications", "s_content_objects", "s_content_witnesses"):
            assert connection.execute(f"SELECT COUNT(*) FROM {table}").fetchone()[0] > 0
    finally:
        connection.close()
    assert digest(database.read_bytes()) == original_hash
    write_json(out / "ORIGINALS-VALIDATION.json", {
        "exact_frames": len(nodes), "database_readonly_integrity": "ok",
        "database_unchanged": True, "current_reader_acceptance": False,
    })


def finalize(out):
    files = {str(p.relative_to(out)): digest(p.read_bytes()) for p in sorted(out.rglob("*"))
             if p.is_file() and p.name != "ARTIFACTS.json"}
    write_json(out / "ARTIFACTS.json", {"sha256": files, "formal_product_acceptance": False})


def main():
    mode, destination, *args = sys.argv[1:]
    out = Path(destination).resolve()
    out.mkdir(parents=True, exist_ok=True)
    if mode == "provenance":
        provenance(out, *(Path(p).resolve() for p in args))
    elif mode == "test":
        return run_test(out, Path(args[0]).resolve(), args[1])
    elif mode == "self-test":
        self_test(out)
    elif mode == "verify":
        verify(out)
    elif mode == "finalize":
        finalize(out)
    else:
        raise ValueError(mode)
    return 0


if __name__ == "__main__":
    sys.exit(main())
