#!/usr/bin/env python3
"""#1405 专用历史取证：保留原命令退出码，封存实际产物，不推定验收通过。"""

import hashlib
import json
from pathlib import Path
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


def run_test(out, repo, label):
    name = TESTS[label]
    command = ["cargo", "test", "--locked", "--manifest-path", "rust/Cargo.toml",
               "--features", "s_session", "--bin", "s_structure_session", name,
               "--jobs", "1", "--", "--exact", "--nocapture", "--test-threads=1"]
    stdout, stderr = out / f"{label}.stdout.log", out / f"{label}.stderr.log"
    receipt = {"command": command, "head": git(repo, "rev-parse", "HEAD"),
               "cwd": str(repo), "test": name, "status": "started"}
    write_json(out / f"{label}.json", receipt)
    with stdout.open("wb") as so, stderr.open("wb") as se:
        completed = subprocess.run(command, cwd=repo, stdout=so, stderr=se, check=False)
    # The original command status is persisted before any acceptance check.
    receipt.update(status="finished", raw_exit_code=completed.returncode,
                   stdout_sha256=digest(stdout.read_bytes()), stderr_sha256=digest(stderr.read_bytes()))
    write_json(out / f"{label}.json", receipt)
    sys.stdout.buffer.write(stdout.read_bytes())
    sys.stderr.buffer.write(stderr.read_bytes())
    if completed.returncode:
        return completed.returncode if completed.returncode > 0 else 128 - completed.returncode
    text = stdout.read_text(encoding="utf-8")
    # --exact matching zero tests is not success evidence.
    if f"test {name} ... ok" not in text or "1 passed; 0 failed; 0 ignored;" not in text:
        return 97
    return 0


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
    elif mode == "verify":
        verify(out)
    elif mode == "finalize":
        finalize(out)
    else:
        raise ValueError(mode)
    return 0


if __name__ == "__main__":
    sys.exit(main())
