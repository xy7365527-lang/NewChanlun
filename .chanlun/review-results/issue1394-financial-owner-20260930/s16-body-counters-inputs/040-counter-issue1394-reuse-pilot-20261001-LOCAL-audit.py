"""#1394 same-host, read-only S16 audit pilot. Run prepare, build, run, report in order.

Only LOCAL-output/ and the pre-existing owned target/ may be written. The frozen
external-volume scripts and their artifacts are intentionally not imported.
"""

from __future__ import annotations

import hashlib
import json
import os
from pathlib import Path
import re
import shutil
import signal
import sqlite3
import subprocess
import sys
import time
import traceback

if sys.flags.optimize:
    raise SystemExit("LOCAL-audit.py requires Python assertions; optimized mode is forbidden")


P = Path(__file__).resolve().parent
WT = P.parents[1]
OUT = P / "LOCAL-output"
CAND = Path("/Users/silencehan/Projects/NewChanlun-1394-same-material-20261001")
SHADOW = Path("/Users/silencehan/Projects/NewChanlun-1394-audit-shadow-20261001/.scratch/issue1394-audit-readonly-stage-probe-20261001")
ROOT = Path("/Users/silencehan/Projects/NewChanlun-1394-content-v3")
FULL_DB = ROOT / ".scratch/issue1394-formal-fullclock-prefix16-20261001/release1/A/final.fullDB.json"
OLD_BIN = SHADOW / "target/release/deps/s_structure_session-f691a9b42625f576"
NEW_BIN = P / "target/release/deps/s_structure_session-f691a9b42625f576"
ORIGINAL_SOURCE = SHADOW / "source-A"
SOURCE_NAMES = ("s.sqlite", "s.sqlite-wal", "s.sqlite-shm")
HEAD = "c6255d308fc69d94178916a1dc5534546fb77591"
TREE = "7196a77fef56cf87cbe91dc4b135326a142e2395"
PATCH_SHA = "e18ffbaefceb9c577f00c92bfed9edb8821e498afce481f9a90011f0f8d13ff3"
OLD_BIN_SHA = "3ea943907f9dc045c93ba8f5cbf2e7957e7fdf99cc6bf1c55273d843ef56e61d"
OLD_PROBE_SHA = "e5081eb994bbd3109d341292b0816d75e24d60aaabc802030efbdb4cf09643b9"
FULL_DB_SHA = "8a7f8f248d1ffad4081555d2900bfd9bc73fb32971167edb544cb15292ea8174"
SOURCE_SHA = {
    "s.sqlite": "1bb9fb2c1660501bc570a52ccb61aa4c89cbe806fd9fc3bea0cfb550de5423fb",
    "s.sqlite-wal": "89032325fe6a2dbf4b3b97e2f335072ce0d7cc5a634c81e0411ebc4dfefe843b",
    "s.sqlite-shm": "757d2d7f80f325accd4a68f0a776bef0d31db8f1ecf0d7ce486bfefb5d616193",
}


def sha(path: Path) -> str:
    with path.open("rb") as stream:
        return hashlib.file_digest(stream, "sha256").hexdigest()


def canon(value: object) -> str:
    return json.dumps(value, sort_keys=True, separators=(",", ":"), ensure_ascii=False)


def json_sha(value: object) -> str:
    return hashlib.sha256(canon(value).encode()).hexdigest()


def checked_artifact_sha(path: Path, expected: object) -> str:
    assert type(expected) is str and re.fullmatch(r"[0-9a-f]{64}", expected), (str(path), expected)
    actual = sha(path)
    assert actual == expected, (str(path), expected, actual)
    return actual


def save(name: str, value: object) -> None:
    path = OUT / name
    with path.open("x", encoding="utf-8") as stream:
        stream.write(canon(value) + "\n")


def read(name: str) -> object:
    return json.loads((OUT / name).read_text())


def ident(path: Path) -> dict:
    stat = path.stat()
    return dict(path=str(path), bytes=stat.st_size, dev=stat.st_dev,
                inode=stat.st_ino, mtime_ns=stat.st_mtime_ns, sha256=sha(path))


def source_idents(directory: Path) -> list[dict]:
    assert sorted(x.name for x in directory.iterdir()) == sorted(SOURCE_NAMES)
    return [ident(directory / name) for name in SOURCE_NAMES]


def owned_source_equal(expected: list[dict], actual: list[dict]) -> bool:
    """SQLite's read-only WAL lock can advance owned -shm mtime without byte changes."""
    if len(expected) != 3 or len(actual) != 3:
        return False
    for old, new in zip(expected, actual):
        if any(old[key] != new[key] for key in ("path", "bytes", "dev", "inode", "sha256")):
            return False
        if Path(old["path"]).name != "s.sqlite-shm" and old["mtime_ns"] != new["mtime_ns"]:
            return False
    return True


def unsigned(value: object, label: str) -> int:
    assert type(value) is int and value >= 0, (label, value)
    return value


def decimal(value: object, label: str) -> int:
    assert type(value) is str and re.fullmatch(r"(?:0|[1-9][0-9]*)", value), (label, value)
    return int(value)


OLD_COUNT_KEYS = frozenset(("parse_calls", "parse_event_checks", "parse_ns", "stored_clock_calls",
                            "stored_clock_ns", "source_read_calls", "source_body_bytes",
                            "source_framed_bytes", "unique_source_digests", "s_content_f_calls"))
NEW_COUNT_KEYS = OLD_COUNT_KEYS | frozenset(("reuse_hits", "reuse_misses", "reuse_mismatches",
    "admission_reject_entries", "admission_reject_bytes", "admission_reject_capacity",
    "retained_entries_peak", "retained_capacity_peak", "reuse_marker_calls",
    "reuse_marker_inner_ns_lower_bound", "public_role_success_roots", "template_role_success_roots"))
DECIMAL_COUNT_KEYS = frozenset(("parse_ns", "stored_clock_ns", "reuse_marker_inner_ns_lower_bound"))


def checked_counts(value: object, candidate: bool) -> dict:
    assert type(value) is dict
    assert set(value) == (NEW_COUNT_KEYS if candidate else OLD_COUNT_KEYS)
    for key, number in value.items():
        (decimal if key in DECIMAL_COUNT_KEYS else unsigned)(number, key)
    return value


def target_idents(directory: Path) -> list[dict]:
    return [ident(x) for x in sorted(directory.rglob("*")) if x.is_file()]


def git(directory: Path, *args: str) -> str:
    return subprocess.check_output(["git", *args], cwd=directory, text=True).strip()


def static_gate() -> dict:
    assert git(WT, "rev-parse", "HEAD") == git(CAND, "rev-parse", "HEAD") == HEAD
    assert git(WT, "rev-parse", "HEAD:rust") == git(CAND, "rev-parse", "HEAD:rust") == TREE
    assert sha(P / "DIAGNOSTIC.patch") == PATCH_SHA
    binding = json.loads((P / "SOURCE-BINDING.json").read_text())
    assert binding["diagnostic_patch_sha256"] == PATCH_SHA
    assert binding["candidate_head"] == binding["pilot_head"] == HEAD
    assert binding["candidate_rust_tree"] == binding["pilot_base_rust_tree"] == TREE
    for relative, record in binding["source"].items():
        assert sha(WT / relative) == record["sha256"], relative
        if record["candidate_sha256"]:
            assert sha(CAND / relative) == record["candidate_sha256"], relative
    # Reconstruct the exact frozen diagnostic patch without writing it.
    diff = subprocess.check_output(["git", "diff", "--binary", "--", *binding["tracked_changes"]], cwd=WT)
    module = (WT / binding["new_test_observer"]).read_bytes()
    assert hashlib.sha256(diff + b"\n# Untracked cfg(test) observer follows verbatim:\n" + module).hexdigest() == PATCH_SHA
    assert sha(OLD_BIN) == OLD_BIN_SHA
    assert sha(SHADOW / "PROBE.json") == OLD_PROBE_SHA
    assert sha(FULL_DB) == FULL_DB_SHA
    for name, expected in SOURCE_SHA.items():
        assert sha(ORIGINAL_SOURCE / name) == expected
    original_target = json.loads((P / "TARGET-ORIGINAL-BEFORE.json").read_text())
    assert len(original_target) == 589
    assert target_idents(CAND / ".scratch/issue1394-same-material-20261001/target") == [
        {**item, "dev": Path(item["path"]).stat().st_dev} for item in original_target
    ]
    return dict(head=HEAD, rust_tree=TREE, patch_sha256=PATCH_SHA,
                binding_sha256=sha(P / "SOURCE-BINDING.json"),
                old_bin_sha256=OLD_BIN_SHA, old_probe_sha256=OLD_PROBE_SHA,
                full_db_sha256=FULL_DB_SHA, original_target_files=len(original_target),
                free_bytes=shutil.disk_usage(P).free)


def original_gate() -> None:
    recorded = json.loads((P / "SOURCE-ORIGINAL-BEFORE.json").read_text())
    for item in recorded:
        actual = ident(Path(item["path"]))
        assert {key: actual[key] for key in item} == item, item["path"]
    static_gate()


def prepare() -> None:
    assert not OUT.exists(), "LOCAL-output already exists; inspect before rerun"
    assert shutil.disk_usage(P).free >= 300 * 1024 * 1024
    gate = static_gate()
    for path in (ORIGINAL_SOURCE / name for name in SOURCE_NAMES):
        assert path.is_file() and not path.is_symlink()
        opened = subprocess.run(["lsof", "-F", "p", "--", str(path)], capture_output=True, text=True)
        assert opened.returncode == 1 and not opened.stdout, path
    before = source_idents(ORIGINAL_SOURCE)
    time.sleep(0.1)
    assert source_idents(ORIGINAL_SOURCE) == before
    OUT.mkdir(mode=0o700)
    (OUT / "tmp").mkdir(mode=0o700)
    for name in ("baseline-source", "candidate-source"):
        subprocess.run(["cp", "-cR", str(ORIGINAL_SOURCE), str(OUT / name)], check=True)
        copied = source_idents(OUT / name)
        assert all(a["sha256"] == b["sha256"] and a["inode"] != b["inode"] and
                   a["dev"] == b["dev"] and not Path(b["path"]).is_symlink()
                   for a, b in zip(before, copied))
    (OUT / "baseline-bin").mkdir()
    subprocess.run(["cp", "-c", str(OLD_BIN), str(OUT / "baseline-bin/s_structure_session")], check=True)
    assert sha(OUT / "baseline-bin/s_structure_session") == OLD_BIN_SHA
    assert ident(OUT / "baseline-bin/s_structure_session")["inode"] != ident(OLD_BIN)["inode"]
    own_target = target_idents(P / "target")
    original_target = json.loads((P / "TARGET-ORIGINAL-BEFORE.json").read_text())
    relative = lambda xs, base: {str(Path(x["path"]).relative_to(base)): (x["bytes"], x["sha256"]) for x in xs}
    assert relative(own_target, P / "target") == relative(original_target, CAND / ".scratch/issue1394-same-material-20261001/target")
    assert all(a["inode"] != b["inode"] for a, b in zip(own_target, original_target))
    assert source_idents(ORIGINAL_SOURCE) == before
    original_gate()
    save("PREPARE.json", dict(**gate, original_source=before,
                              baseline_source=source_idents(OUT / "baseline-source"),
                              candidate_source=source_idents(OUT / "candidate-source"),
                              own_target_before=own_target,
                              baseline_bin=ident(OUT / "baseline-bin/s_structure_session"),
                              clone_method="APFS cp -cR/cp -c; separate inode, no writable hardlink",
                              wrapper_sha256=sha(Path(__file__)),
                              free_after_bytes=shutil.disk_usage(P).free))
    print(canon(dict(event="LOCAL_PREPARED", output=str(OUT), free_bytes=shutil.disk_usage(P).free)), flush=True)


def rebind() -> None:
    """Bind the completed wrapper to an earlier, still immutable prepare result."""
    assert (OUT / "PREPARE.json").exists() and not (OUT / "REBOUND.json").exists()
    assert not (OUT / "BUILD.json").exists()
    prepared = read("PREPARE.json")
    gate = static_gate()
    assert gate["head"] == prepared["head"] and gate["rust_tree"] == prepared["rust_tree"]
    assert gate["patch_sha256"] == prepared["patch_sha256"]
    assert source_idents(ORIGINAL_SOURCE) == prepared["original_source"]
    assert source_idents(OUT / "baseline-source") == prepared["baseline_source"]
    assert source_idents(OUT / "candidate-source") == prepared["candidate_source"]
    assert target_idents(P / "target") == prepared["own_target_before"]
    assert ident(OUT / "baseline-bin/s_structure_session") == prepared["baseline_bin"]
    save("REBOUND.json", dict(prepare_sha256=sha(OUT / "PREPARE.json"),
                              original_prepare_wrapper_sha256=prepared.get("wrapper_sha256", "UNRECORDED"),
                              current_wrapper_sha256=sha(Path(__file__)),
                              source_binding_sha256=sha(P / "SOURCE-BINDING.json"),
                              diagnostic_patch_sha256=PATCH_SHA,
                              target_files=len(prepared["own_target_before"]),
                              clone_source="unchanged SHA/dev/inode/mtime"))
    print(canon(dict(event="LOCAL_REBOUND", wrapper_sha256=sha(Path(__file__)))), flush=True)


def rebind_after_build() -> None:
    """Preserve the reviewed build and bind a corrected owned-SHM metadata gate."""
    assert (OUT / "REBOUND.json").exists() and not (OUT / "REBOUND-2.json").exists()
    assert not any((OUT / name).exists() for name in ("baseline-PLAN.json", "baseline-PROBE.json",
                                                 "candidate-PLAN.json", "candidate-PROBE.json"))
    original_gate()
    prepared = read("PREPARE.json")
    assert owned_source_equal(prepared["baseline_source"], source_idents(OUT / "baseline-source"))
    assert owned_source_equal(prepared["candidate_source"], source_idents(OUT / "candidate-source"))
    build_result = read("BUILD.json")
    assert unsigned(build_result["exit"], "build.exit") == 0 and build_result["reaped"] is True
    assert build_result["test_binary"] == ident(NEW_BIN)
    save("REBOUND-2.json", dict(previous_rebound_sha256=sha(OUT / "REBOUND.json"),
        previous_wrapper_sha256=read("REBOUND.json")["current_wrapper_sha256"],
        prepare_sha256=sha(OUT / "PREPARE.json"), build_result_sha256=sha(OUT / "BUILD.json"),
        current_wrapper_sha256=sha(Path(__file__)), source_binding_sha256=sha(P / "SOURCE-BINDING.json"),
        diagnostic_patch_sha256=PATCH_SHA,
        correction="owned s.sqlite-shm mtime excluded; SHA, bytes, dev, inode and original triplet remain exact"))
    print(canon(dict(event="LOCAL_REBOUND_AFTER_BUILD", wrapper_sha256=sha(Path(__file__)))), flush=True)


def binding_path() -> Path:
    return OUT / ("REBOUND-2.json" if (OUT / "REBOUND-2.json").exists() else "REBOUND.json")


def bound_gate() -> None:
    bound = json.loads(binding_path().read_text())
    assert bound["current_wrapper_sha256"] == sha(Path(__file__))
    assert bound["prepare_sha256"] == sha(OUT / "PREPARE.json")
    assert bound["source_binding_sha256"] == sha(P / "SOURCE-BINDING.json")
    assert bound["diagnostic_patch_sha256"] == PATCH_SHA
    if binding_path().name == "REBOUND-2.json":
        assert bound["previous_rebound_sha256"] == sha(OUT / "REBOUND.json")
        assert bound["build_result_sha256"] == sha(OUT / "BUILD.json")
        assert bound["previous_wrapper_sha256"] == read("REBOUND.json")["current_wrapper_sha256"]
    else:
        assert unsigned(bound["target_files"], "target_files") == 589


def reap(child: subprocess.Popen, terminate: bool) -> int:
    if terminate and child.poll() is None:
        try:
            os.killpg(child.pid, signal.SIGTERM)
        except ProcessLookupError:
            pass
    try:
        return child.wait(timeout=10)
    except subprocess.TimeoutExpired:
        os.killpg(child.pid, signal.SIGKILL)
        return child.wait()


def build() -> None:
    assert (OUT / "PREPARE.json").exists()
    bound_gate()
    assert not (OUT / "BUILD.json").exists() and not (OUT / "build.log").exists()
    original_gate()
    assert source_idents(OUT / "baseline-source") == read("PREPARE.json")["baseline_source"]
    assert source_idents(OUT / "candidate-source") == read("PREPARE.json")["candidate_source"]
    for _ in range(2):
        assert shutil.disk_usage(P).free >= 300 * 1024 * 1024, "300 MiB build start gate"
        time.sleep(0.5)
    env = os.environ.copy()
    for key in list(env):
        if key.startswith("CARGO_PROFILE_") or key in ("CARGO_ENCODED_RUSTFLAGS", "RUSTFLAGS", "CARGO_INCREMENTAL", "CARGO_TARGET_DIR"):
            env.pop(key)
    explicit = dict(CARGO_TARGET_DIR=str(P / "target"), CARGO_INCREMENTAL="0",
                    PYO3_PYTHON="/Users/silencehan/.local/bin/python3.12",
                    RUSTFLAGS="-L native=/Users/silencehan/.local/share/uv/python/cpython-3.12.13-macos-aarch64-none/lib",
                    TMPDIR=str(OUT / "tmp"))
    env.update(explicit)
    command = ["cargo", "test", "--release", "--locked", "--manifest-path", "rust/Cargo.toml",
               "--features", "s_session", "--bin", "s_structure_session", "--no-run", "-vv"]
    previous = sha(NEW_BIN) if NEW_BIN.exists() else None
    save("BUILD-PLAN.json", dict(command=command, explicit_env=explicit, target_before_sha256=previous,
                                 diagnostic_patch_sha256=PATCH_SHA, free_before_bytes=shutil.disk_usage(P).free,
                                 start_gate_bytes=300 * 1024 * 1024, stop_gate_bytes=250 * 1024 * 1024))
    child = None
    samples = []
    stopped_for_space = False
    started = time.monotonic_ns()
    try:
        with (OUT / "build.log").open("x") as log:
            child = subprocess.Popen(command, cwd=WT, env=env, stdin=subprocess.DEVNULL,
                                     stdout=log, stderr=subprocess.STDOUT, start_new_session=True)
            print(canon(dict(event="LOCAL_BUILD_STARTED", pid=child.pid)), flush=True)
            while child.poll() is None:
                free = shutil.disk_usage(P).free
                samples.append(dict(elapsed_ns=time.monotonic_ns() - started, free_bytes=free))
                if free < 250 * 1024 * 1024:
                    stopped_for_space = True
                    break
                time.sleep(0.25)
            code = reap(child, stopped_for_space)
    except BaseException:
        if child is not None:
            reap(child, True)
        save("BUILD-FAILURE.json", dict(pid=child.pid if child else None, reaped=child is None or child.poll() is not None,
                                        error=traceback.format_exc()))
        raise
    save("BUILD-SAMPLES.json", samples)
    original_gate()
    result = dict(pid=child.pid, exit=code, reaped=True, space_gate=stopped_for_space,
                  elapsed_ns=time.monotonic_ns() - started, free_after_bytes=shutil.disk_usage(P).free,
                  plan_sha256=sha(OUT / "BUILD-PLAN.json"), raw_log_sha256=sha(OUT / "build.log"),
                  diagnostic_patch_sha256=PATCH_SHA)
    if code == 0 and not stopped_for_space:
        assert NEW_BIN.exists() and sha(NEW_BIN) != previous
        result["test_binary"] = ident(NEW_BIN)
    save("BUILD.json", result)
    print(canon(dict(event="LOCAL_BUILD_FINISHED", pid=child.pid, exit=code, space_gate=stopped_for_space)), flush=True)
    assert code == 0 and not stopped_for_space


def cell(value: object) -> object:
    return {"blob_hex": value.hex()} if isinstance(value, bytes) else value


def db_dump(database: Path) -> dict:
    # Only owned clones reach SQLite; the original A triplet is hash/stat only.
    assert database.parent in (OUT / "baseline-source", OUT / "candidate-source")
    connection = sqlite3.connect(f"file:{database}?mode=ro", uri=True)
    try:
        connection.execute("PRAGMA query_only=ON")
        assert connection.execute("PRAGMA query_only").fetchone()[0] == 1
        schema = [list(x) for x in connection.execute("SELECT type,name,tbl_name,rootpage,sql FROM sqlite_schema ORDER BY type,name")]
        tables = {}
        for (name,) in connection.execute("SELECT name FROM sqlite_schema WHERE type='table' AND name NOT LIKE 'sqlite_%' ORDER BY name"):
            quoted = '"' + name.replace('"', '""') + '"'
            columns = [list(x) for x in connection.execute("PRAGMA table_xinfo(" + quoted + ")")]
            rows = [[cell(value) for value in row] for row in connection.execute("SELECT * FROM " + quoted)]
            rows.sort(key=canon)
            tables[name] = dict(columns=columns, row_count=len(rows), rows=rows, sha256=json_sha(rows))
        expected = json.loads(FULL_DB.read_text())
        properties = {key: connection.execute("PRAGMA " + key).fetchone()[0] for key in expected["db_properties"]}
        return dict(schema=schema, db_properties=properties, tables=tables)
    finally:
        connection.close()


def db_summary(database: Path) -> dict:
    actual = db_dump(database)
    expected = json.loads(FULL_DB.read_text())
    assert canon(actual) == canon(expected), "24-table canonical source differs from committed final.fullDB"
    return dict(canonical_sha256=json_sha(actual), tables=len(actual["tables"]),
                rows=sum(x["row_count"] for x in actual["tables"].values()),
                blob_cells=sum(isinstance(v, dict) and "blob_hex" in v for table in actual["tables"].values()
                               for row in table["rows"] for v in row),
                native_blob_cells=sum(isinstance(v, dict) and "blob_hex" in v for table in actual["tables"].values()
                                      for row in table["rows"] for v in row if table is actual["tables"].get("s_content_nodes")))


def run_one(role: str) -> dict:
    assert role in ("baseline", "candidate")
    prepared = read("PREPARE.json")
    source = OUT / (role + "-source")
    before = source_idents(source)
    assert owned_source_equal(prepared[role + "_source"], before)
    if role == "baseline":
        exe = OUT / "baseline-bin/s_structure_session"
        assert ident(exe) == prepared["baseline_bin"] and sha(exe) == OLD_BIN_SHA
        env_name = "S3_READONLY_PROBE_DB"
        output_name = "S3_READONLY_PROBE_OUTPUT"
        test_name = "v3::state::readonly_shadow::readonly_audit_stage_probe"
    else:
        exe = NEW_BIN
        assert ident(exe) == read("BUILD.json")["test_binary"]
        env_name = "S3_REUSE_PILOT_DB"
        output_name = "S3_REUSE_PILOT_OUTPUT"
        test_name = "v3::state::readonly_reuse_pilot::readonly_audit_reuse_pilot"
    output = OUT / (role + "-PROBE.json")
    log_path = OUT / (role + ".log")
    assert not output.exists() and not log_path.exists()
    before_db = db_summary(source / "s.sqlite")
    assert before_db["tables"] == 24 and before_db["rows"] == 48403 and before_db["blob_cells"] == 96397
    after_static = source_idents(source)
    assert owned_source_equal(before, after_static)
    env = os.environ.copy()
    env.update({env_name: str(source / "s.sqlite"), output_name: str(output), "TMPDIR": str(OUT / "tmp")})
    command = [str(exe), test_name, "--exact", "--ignored", "--nocapture", "--test-threads=1"]
    save(role + "-PLAN.json", dict(command=command, explicit_env={k: env[k] for k in (env_name, output_name, "TMPDIR")},
                                     binary_sha256=sha(exe), source_before=before,
                                     source_after_static=after_static, logical_before=before_db,
                                     old_probe_sha256=OLD_PROBE_SHA, diagnostic_patch_sha256=PATCH_SHA,
                                     free_before_bytes=shutil.disk_usage(P).free))
    samples = []
    child = None
    started = time.monotonic_ns()
    try:
        with log_path.open("x") as log:
            child = subprocess.Popen(command, env=env, stdin=subprocess.DEVNULL, stdout=log,
                                     stderr=subprocess.STDOUT, start_new_session=True)
            print(canon(dict(event="LOCAL_AUDIT_STARTED", role=role, pid=child.pid)), flush=True)
            while child.poll() is None:
                sample = subprocess.run(["ps", "-o", "rss=", "-p", str(child.pid)], capture_output=True, text=True)
                if sample.stdout.strip():
                    samples.append(dict(elapsed_ns=time.monotonic_ns() - started,
                                        rss_bytes=int(sample.stdout.strip()) * 1024,
                                        free_bytes=shutil.disk_usage(P).free))
                time.sleep(0.1)
            code = reap(child, False)
    except BaseException:
        if child is not None:
            reap(child, True)
        save(role + "-FAILURE.json", dict(pid=child.pid if child else None,
                                         reaped=child is None or child.poll() is not None,
                                         error=traceback.format_exc()))
        raise
    assert code == 0 and output.exists(), (role, code)
    assert owned_source_equal(before, source_idents(source)), role + " DB/WAL/SHM bytes or inode changed"
    after_db = db_summary(source / "s.sqlite")
    assert after_db == before_db
    original_gate()
    save(role + "-SAMPLES.json", samples)
    result = dict(pid=child.pid, exit=code, reaped=True, process_wall_ns=time.monotonic_ns() - started,
                  rss_sampled_peak_bytes=max((s["rss_bytes"] for s in samples), default=None),
                  rss_sampling_interval_seconds=0.1, heap_peak="UNMEASURED",
                  source_before=before, source_after_static=after_static,
                  source_after=source_idents(source),
                  logical_before=before_db, logical_after=after_db,
                  plan_sha256=sha(OUT / (role + "-PLAN.json")), raw_log_sha256=sha(log_path),
                  probe_sha256=sha(output))
    save(role + "-RESULT.json", result)
    return result


def returned_check(old: dict, baseline: dict, candidate: dict) -> list[dict]:
    assert old["kind"] == baseline["kind"] == candidate["kind"] == "READONLY_DIAGNOSTIC_ONLY"
    assert len(old["repeats"]) == len(baseline["repeats"]) == len(candidate["repeats"]) == 2
    comparison = []
    for index, (a, b, c) in enumerate(zip(old["repeats"], baseline["repeats"], candidate["repeats"])):
        assert all(unsigned(repeat["repeat"], "repeat") == index for repeat in (a, b, c))
        for repeat in (a, b, c):
            decimal(repeat["elapsed_ns"], "elapsed_ns")
            for event in repeat["events"]:
                assert type(event["phase"]) is str
                assert event["generation"] is None or type(event["generation"]) is int
                decimal(event["elapsed_ns"], "event.elapsed_ns")
        assert canon(a["full_returned_state"]) == canon(b["full_returned_state"]) == canon(c["full_returned_state"])
        assert a["fingerprint_sha256"] == b["fingerprint_sha256"] == c["fingerprint_sha256"]
        state = c["full_returned_state"]
        assert set(state) == {"raw", "cuts", "objects", "auxiliary"}
        assert len(state["cuts"]) == 16
        assert all(set(cut) == {"metadata", "roots", "batch_id", "snapshot", "delta"} for cut in state["cuts"])
        marker = lambda r: [(event["phase"], event["generation"]) for event in r["events"]]
        assert marker(a) == marker(b) == marker(c) and len(marker(c)) == 578
        old_count = checked_counts(a["counters"], False)
        base_count = checked_counts(b["counters"], False)
        cand_count = checked_counts(c["counters"], True)
        stable = ("parse_calls", "parse_event_checks", "stored_clock_calls", "source_read_calls",
                  "source_body_bytes", "source_framed_bytes", "unique_source_digests", "s_content_f_calls")
        assert all(old_count[key] == base_count[key] for key in stable)
        assert all(old_count[key] == cand_count[key] for key in stable if key != "s_content_f_calls")
        assert old_count["parse_calls"] == old_count["stored_clock_calls"] == 1
        assert old_count["parse_event_checks"] == 9011
        assert old_count["source_read_calls"] == cand_count["source_read_calls"] == 3082463
        assert old_count["unique_source_digests"] == cand_count["unique_source_digests"] == 35272
        assert cand_count["s_content_f_calls"] < base_count["s_content_f_calls"]
        assert cand_count["public_role_success_roots"] == 32
        assert cand_count["template_role_success_roots"] == 16
        assert cand_count["reuse_hits"] + cand_count["reuse_misses"] > 0
        comparison.append(dict(fingerprint_sha256=c["fingerprint_sha256"], markers=578,
                               baseline_audit_ns=decimal(b["elapsed_ns"], "baseline_audit_ns"),
                               candidate_audit_ns=decimal(c["elapsed_ns"], "candidate_audit_ns"),
                               baseline_counters=base_count, candidate_counters=cand_count,
                               f_reduction=base_count["s_content_f_calls"] - cand_count["s_content_f_calls"],
                               returned_four_members_and_cut_five_members_exact=True))
    return comparison


def current_comparison(old: dict, baseline: dict, candidate: dict, stored: object) -> list[dict]:
    comparison = returned_check(old, baseline, candidate)
    assert canon(comparison) == canon(stored), "stale or altered COMPARISON.json"
    return comparison


def run() -> None:
    bound_gate()
    assert unsigned(read("BUILD.json")["exit"], "build.exit") == 0
    assert not (OUT / "LOCAL-REPORT.json").exists()
    original_gate()
    run_one("baseline")
    run_one("candidate")
    old = json.loads((SHADOW / "PROBE.json").read_text())
    baseline = json.loads((OUT / "baseline-PROBE.json").read_text())
    candidate = json.loads((OUT / "candidate-PROBE.json").read_text())
    comparison = returned_check(old, baseline, candidate)
    original_gate()
    save("COMPARISON.json", comparison)
    print(canon(dict(event="LOCAL_AUDITS_FINISHED", repeats=len(comparison),
                     f_reduction=[item["f_reduction"] for item in comparison])), flush=True)


def report_evidence() -> tuple[list[dict], dict, dict, dict, dict]:
    bound_gate()
    original_gate()
    prepared = read("PREPARE.json")
    build_plan, build_result = read("BUILD-PLAN.json"), read("BUILD.json")
    checked_artifact_sha(OUT / "BUILD-PLAN.json", build_result["plan_sha256"])
    checked_artifact_sha(OUT / "build.log", build_result["raw_log_sha256"])
    assert build_result["diagnostic_patch_sha256"] == build_plan["diagnostic_patch_sha256"] == PATCH_SHA
    assert unsigned(build_result["exit"], "build.exit") == 0 and build_result["reaped"] is True
    assert build_result["space_gate"] is False
    assert unsigned(build_result["elapsed_ns"], "build.elapsed_ns") > 0
    assert build_result["test_binary"] == ident(NEW_BIN)
    assert build_plan["explicit_env"] == dict(CARGO_TARGET_DIR=str(P / "target"), CARGO_INCREMENTAL="0",
        PYO3_PYTHON="/Users/silencehan/.local/bin/python3.12",
        RUSTFLAGS="-L native=/Users/silencehan/.local/share/uv/python/cpython-3.12.13-macos-aarch64-none/lib",
        TMPDIR=str(OUT / "tmp"))
    assert build_plan["command"] == ["cargo", "test", "--release", "--locked", "--manifest-path",
        "rust/Cargo.toml", "--features", "s_session", "--bin", "s_structure_session", "--no-run", "-vv"]
    assert unsigned(build_plan["start_gate_bytes"], "start_gate") == 300 * 1024 * 1024
    assert unsigned(build_plan["stop_gate_bytes"], "stop_gate") == 250 * 1024 * 1024
    before_test = next((item for item in prepared["own_target_before"] if item["path"] == str(NEW_BIN)), None)
    assert build_plan["target_before_sha256"] == (before_test["sha256"] if before_test else None)
    assert build_result["test_binary"]["sha256"] != build_plan["target_before_sha256"]
    assert sha(OUT / "baseline-bin/s_structure_session") == OLD_BIN_SHA
    assert ident(OUT / "baseline-bin/s_structure_session") == prepared["baseline_bin"]
    result = {}
    for role in ("baseline", "candidate"):
        plan, run_result = read(role + "-PLAN.json"), read(role + "-RESULT.json")
        exe = OUT / "baseline-bin/s_structure_session" if role == "baseline" else NEW_BIN
        test = "v3::state::readonly_shadow::readonly_audit_stage_probe" if role == "baseline" else "v3::state::readonly_reuse_pilot::readonly_audit_reuse_pilot"
        env_name = "S3_READONLY_PROBE_DB" if role == "baseline" else "S3_REUSE_PILOT_DB"
        output_name = "S3_READONLY_PROBE_OUTPUT" if role == "baseline" else "S3_REUSE_PILOT_OUTPUT"
        source = OUT / (role + "-source")
        probe = OUT / (role + "-PROBE.json")
        assert plan["command"] == [str(exe), test, "--exact", "--ignored", "--nocapture", "--test-threads=1"]
        assert plan["explicit_env"] == {env_name: str(source / "s.sqlite"), output_name: str(probe), "TMPDIR": str(OUT / "tmp")}
        assert plan["binary_sha256"] == sha(exe)
        assert plan["diagnostic_patch_sha256"] == PATCH_SHA and plan["old_probe_sha256"] == OLD_PROBE_SHA
        assert owned_source_equal(prepared[role + "_source"], plan["source_before"])
        assert owned_source_equal(plan["source_before"], source_idents(source))
        assert run_result["source_before"] == plan["source_before"]
        assert run_result["source_after_static"] == plan["source_after_static"]
        assert owned_source_equal(run_result["source_before"], run_result["source_after_static"])
        assert owned_source_equal(run_result["source_before"], run_result["source_after"])
        assert run_result["logical_before"] == run_result["logical_after"] == plan["logical_before"] == db_summary(source / "s.sqlite")
        checked_artifact_sha(OUT / (role + "-PLAN.json"), run_result["plan_sha256"])
        checked_artifact_sha(OUT / (role + ".log"), run_result["raw_log_sha256"])
        checked_artifact_sha(probe, run_result["probe_sha256"])
        assert unsigned(run_result["exit"], role + ".exit") == 0 and run_result["reaped"] is True
        assert unsigned(run_result["process_wall_ns"], role + ".process_wall_ns") > 0
        rss = run_result["rss_sampled_peak_bytes"]
        assert rss is None or unsigned(rss, role + ".rss_sampled_peak_bytes") > 0
        assert run_result["heap_peak"] == "UNMEASURED"
        result[role] = run_result
    old = json.loads((SHADOW / "PROBE.json").read_text())
    baseline = json.loads((OUT / "baseline-PROBE.json").read_text())
    candidate = json.loads((OUT / "candidate-PROBE.json").read_text())
    assert sha(SHADOW / "PROBE.json") == OLD_PROBE_SHA
    assert baseline["db"] == str(OUT / "baseline-source/s.sqlite")
    assert candidate["db"] == str(OUT / "candidate-source/s.sqlite")
    comparison = current_comparison(old, baseline, candidate, read("COMPARISON.json"))
    original_after = dict(source=source_idents(ORIGINAL_SOURCE),
                          target=target_idents(CAND / ".scratch/issue1394-same-material-20261001/target"))
    recorded_source = json.loads((P / "SOURCE-ORIGINAL-BEFORE.json").read_text())
    recorded_target = json.loads((P / "TARGET-ORIGINAL-BEFORE.json").read_text())
    assert len(original_after["source"]) == 3 and len(original_after["target"]) == 589
    assert [{key: item[key] for key in old_item} for item, old_item in zip(original_after["source"], recorded_source)] == recorded_source
    assert [{key: item[key] for key in old_item} for item, old_item in zip(original_after["target"], recorded_target)] == recorded_target
    return comparison, build_result, result["baseline"], result["candidate"], original_after


def report() -> None:
    assert not (OUT / "LOCAL-REPORT.json").exists() and not (OUT / "LOCAL-REPORT.md").exists()
    try:
        comparison, build_result, baseline_result, candidate_result, original_after = report_evidence()
        original_bytes = (canon(original_after) + "\n").encode()
        original_sha = hashlib.sha256(original_bytes).hexdigest()
        for item in comparison:
            for key in ("baseline_audit_ns", "candidate_audit_ns", "f_reduction"):
                unsigned(item[key], key)
            checked_counts(item["baseline_counters"], False)
            checked_counts(item["candidate_counters"], True)
        assert len(comparison) == 2
        out = dict(kind="READONLY_DIAGNOSTIC_ONLY", status="PASS", same_host=True,
                   measurements_separate_from_q=True, candidate_head=HEAD, candidate_rust_tree=TREE,
                   diagnostic_patch_sha256=PATCH_SHA, test_binary_sha256=sha(NEW_BIN),
                   old_test_binary_sha256=OLD_BIN_SHA, full_db_sha256=FULL_DB_SHA,
                   old_probe_sha256=OLD_PROBE_SHA, source_triplet_sha256=SOURCE_SHA,
                   original_target_files=589, original_target_unchanged=True,
                   original_source_unchanged=True, all_24_tables_48403_rows_96397_blob_cells_exact=True,
                   originals_after_sha256=original_sha,
                   wrapper_sha256=sha(Path(__file__)), rebound_sha256=sha(binding_path()),
                   prepare_sha256=sha(OUT / "PREPARE.json"),
                   build_plan_sha256=sha(OUT / "BUILD-PLAN.json"),
                   baseline_plan_sha256=sha(OUT / "baseline-PLAN.json"),
                   candidate_plan_sha256=sha(OUT / "candidate-PLAN.json"),
                   baseline_probe_sha256=sha(OUT / "baseline-PROBE.json"),
                   candidate_probe_sha256=sha(OUT / "candidate-PROBE.json"),
                   comparisons=comparison, build=build_result,
                   owned_shm_mtime_observations={role: dict(
                       prepared_ns=next(x["mtime_ns"] for x in read("PREPARE.json")[role + "_source"] if Path(x["path"]).name == "s.sqlite-shm"),
                       before_static_ns=next(x["mtime_ns"] for x in read(role + "-RESULT.json")["source_before"] if Path(x["path"]).name == "s.sqlite-shm"),
                       after_static_ns=next(x["mtime_ns"] for x in read(role + "-RESULT.json")["source_after_static"] if Path(x["path"]).name == "s.sqlite-shm"),
                       after_audit_ns=next(x["mtime_ns"] for x in read(role + "-RESULT.json")["source_after"] if Path(x["path"]).name == "s.sqlite-shm"),
                       at_report_ns=(OUT / (role + "-source/s.sqlite-shm")).stat().st_mtime_ns)
                       for role in ("baseline", "candidate")},
                   baseline_rss_sampled_peak_bytes=baseline_result["rss_sampled_peak_bytes"],
                   candidate_rss_sampled_peak_bytes=candidate_result["rss_sampled_peak_bytes"],
                   heap_peak="UNMEASURED", rss_scope="whole test process sampled at 0.1s, not true peak",
                   full9011="NOT_RUN", whole_graph="NOT_RUN")
        lines = ["# #1394 本机 same-material 只读审计收益验收", "", "状态：PASS；完整 A DB/WAL/SHM 的两份独立 APFS 克隆，各执行两次 state::audit。",
                 f"候选 {HEAD} / Rust tree {TREE}；诊断 patch SHA256 {PATCH_SHA}；testbin SHA256 {sha(NEW_BIN)}。",
                 "24 表/48,403 行/96,397 BLOB、AuditedState 四成员、每 cut 五成员、578 marker、SQL found 3,082,463 / unique 35,272 均逐项对拍；原 source 与 589 个原 target 保持 SHA/inode/mtime。", ""]
        for index, item in enumerate(comparison):
            b, c = item["baseline_counters"], item["candidate_counters"]
            lines.append(f"第 {index + 1} 次：同机旧版 {item['baseline_audit_ns']/1e9:.6f}s，候选 {item['candidate_audit_ns']/1e9:.6f}s；F {b['s_content_f_calls']:,} → {c['s_content_f_calls']:,}（减少 {item['f_reduction']:,}）；reuse hit/miss {c['reuse_hits']:,}/{c['reuse_misses']:,}；retained peak {c['retained_entries_peak']:,} entries / {c['retained_capacity_peak']:,} B。")
        lines += ["", f"RSS 0.1s 采样：旧版 {baseline_result['rss_sampled_peak_bytes']} B、候选 {candidate_result['rss_sampled_peak_bytes']} B；heap peak UNMEASURED。", "owned -shm 的 mtime 逐段记在 JSON；各 clone 的 DB/WAL/SHM 字节、SHA、设备与 inode 均保持一致，原 source 与原 target 的 mtime 仍严格不变。", "walltime 为本机该次独立审计，不能并入 Q 正文 fixture 采集；full9011 与整图未由本 pilot 证明。"]
        md_bytes = ("\n".join(lines) + "\n").encode()
        json_bytes = (canon(out) + "\n").encode()
        for name, data in (("ORIGINALS-AFTER.json.tmp", original_bytes),
                           ("LOCAL-REPORT.md.tmp", md_bytes), ("LOCAL-REPORT.json.tmp", json_bytes)):
            with (OUT / name).open("xb") as stream:
                stream.write(data)
                stream.flush()
                os.fsync(stream.fileno())
        os.replace(OUT / "ORIGINALS-AFTER.json.tmp", OUT / "ORIGINALS-AFTER.json")
        os.replace(OUT / "LOCAL-REPORT.md.tmp", OUT / "LOCAL-REPORT.md")
        os.replace(OUT / "LOCAL-REPORT.json.tmp", OUT / "LOCAL-REPORT.json")
    except BaseException:
        for name in ("ORIGINALS-AFTER.json.tmp", "LOCAL-REPORT.md.tmp", "LOCAL-REPORT.json.tmp",
                     "ORIGINALS-AFTER.json", "LOCAL-REPORT.md"):
            (OUT / name).unlink(missing_ok=True)
        if not (OUT / "LOCAL-REPORT.json").exists():
            save("LOCAL-REPORT-FAILURE.json", dict(status="FAIL", error=traceback.format_exc(),
                                                   wrapper_sha256=sha(Path(__file__))))
        raise
    print(canon(dict(event="LOCAL_REPORT_PASS", report=str(OUT / "LOCAL-REPORT.json"))), flush=True)


if __name__ == "__main__":
    commands = {"prepare": prepare, "rebind": rebind, "rebind-after-build": rebind_after_build,
                "build": build, "run": run, "report": report}
    if len(sys.argv) != 2 or sys.argv[1] not in commands:
        raise SystemExit("usage: python3 LOCAL-audit.py {prepare|rebind|rebind-after-build|build|run|report}")
    commands[sys.argv[1]]()
