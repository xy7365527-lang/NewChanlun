"""Bounded no-audit counterexamples for LOCAL-audit.py validation gates."""

from copy import deepcopy
import hashlib
import json
from pathlib import Path
import runpy
import tempfile


HERE = Path(__file__).resolve().parent
api = runpy.run_path(str(HERE / "LOCAL-audit.py"))
old_count = dict(parse_calls=1, parse_event_checks=9011, parse_ns="1",
                 stored_clock_calls=1, stored_clock_ns="1", source_read_calls=3082463,
                 source_body_bytes=137305042, source_framed_bytes=232861395,
                 unique_source_digests=35272, s_content_f_calls=4406786)
new_count = dict(old_count, s_content_f_calls=2000000, reuse_hits=100,
                 reuse_misses=35272, reuse_mismatches=0, admission_reject_entries=0,
                 admission_reject_bytes=0, admission_reject_capacity=0,
                 retained_entries_peak=200, retained_capacity_peak=100000,
                 reuse_marker_calls=100, reuse_marker_inner_ns_lower_bound="1",
                 public_role_success_roots=32, template_role_success_roots=16)
state = dict(raw=[], cuts=[dict(metadata="m", roots="r", batch_id=i,
                                snapshot={}, delta={}) for i in range(16)],
             objects="o", auxiliary="a")
events = [dict(phase="marker", generation=None, elapsed_ns=str(i)) for i in range(578)]


def arm(count):
    return dict(kind="READONLY_DIAGNOSTIC_ONLY", repeats=[dict(
        repeat=i, elapsed_ns="4000000000", events=events,
        full_returned_state=state, fingerprint_sha256="f" * 64,
        counters=deepcopy(count)) for i in range(2)])


old, baseline, candidate = arm(old_count), arm(old_count), arm(new_count)
accepted = api["returned_check"](old, baseline, candidate)
assert len(accepted) == 2


def rejects(label, callback):
    try:
        callback()
    except (AssertionError, KeyError, TypeError, ValueError):
        return label
    raise AssertionError(label + " unexpectedly accepted")


checks = []
tampered = deepcopy(old)
tampered["repeats"][0]["counters"]["parse_calls"] = True
checks.append(rejects("old_probe_bool_count", lambda: api["returned_check"](tampered, baseline, candidate)))
tampered = deepcopy(candidate)
tampered["repeats"][0]["counters"]["retained_entries_peak"] = True
checks.append(rejects("candidate_bool_capacity", lambda: api["returned_check"](old, baseline, tampered)))
tampered = deepcopy(candidate)
tampered["repeats"][0]["counters"].pop("reuse_hits")
checks.append(rejects("missing_reuse_field", lambda: api["returned_check"](old, baseline, tampered)))
tampered = deepcopy(candidate)
tampered["repeats"][0]["elapsed_ns"] = 4.5
checks.append(rejects("float_elapsed", lambda: api["returned_check"](old, baseline, tampered)))
tampered = deepcopy(accepted)
tampered[0]["f_reduction"] += 1
checks.append(rejects("stale_comparison", lambda: api["current_comparison"](old, baseline, candidate, tampered)))
checks.append(rejects("false_exit", lambda: api["unsigned"](False, "exit")))
original_triplet = [dict(path=str(HERE / "LOCAL-output/baseline-source" / name), bytes=10,
                         dev=1, inode=index + 1, mtime_ns=100, sha256="a" * 64)
                    for index, name in enumerate(("s.sqlite", "s.sqlite-wal", "s.sqlite-shm"))]
shm_time_changed = deepcopy(original_triplet)
shm_time_changed[2]["mtime_ns"] += 1
assert api["owned_source_equal"](original_triplet, shm_time_changed)
checks.append("owned_shm_mtime_only_allowed")
shm_bytes_changed = deepcopy(shm_time_changed)
shm_bytes_changed[2]["sha256"] = "b" * 64
assert not api["owned_source_equal"](original_triplet, shm_bytes_changed)
checks.append("owned_shm_byte_change_rejected")
db_time_changed = deepcopy(original_triplet)
db_time_changed[0]["mtime_ns"] += 1
assert not api["owned_source_equal"](original_triplet, db_time_changed)
checks.append("owned_db_mtime_change_rejected")
with tempfile.NamedTemporaryFile(dir=HERE / "LOCAL-output", prefix="LOCAL-selftest-", delete=True) as stream:
    stream.write(b"original log")
    stream.flush()
    path = Path(stream.name)
    original_sha = hashlib.sha256(b"original log").hexdigest()
    assert api["checked_artifact_sha"](path, original_sha) == original_sha
    stream.write(b"altered")
    stream.flush()
    checks.append(rejects("modified_raw_log", lambda: api["checked_artifact_sha"](path, original_sha)))

result = dict(status="PASS", wrapper_sha256=api["sha"](HERE / "LOCAL-audit.py"),
              cases=checks, positive_repeats=len(accepted), audit_executed=False)
(HERE / "LOCAL-output/LOCAL-SELFTEST.json").write_text(json.dumps(result, sort_keys=True, indent=2) + "\n")
print(json.dumps(result, sort_keys=True))
