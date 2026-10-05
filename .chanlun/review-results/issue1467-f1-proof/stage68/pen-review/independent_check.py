#!/usr/bin/env python3
"""独立离线审核；只复用已审身份，不导入作者函数，不运行 R_W。"""
from __future__ import annotations

import ast
import hashlib
import importlib.util
import json
import resource
import sys
import time
from collections import Counter
from fractions import Fraction
from pathlib import Path

OUT = Path(__file__).resolve().parent
E = OUT.parents[1]
AUTHOR = OUT.parent / "pen-author"
R = Path("/Users/silencehan/.codex/worktrees/order-event-base-research/NewChanlun")
FIELDS = ("start", "end", "start_price", "end_price", "up")
NAMES = ("P0", "P1", "P2", "c0")


def load(path: Path) -> object:
    return json.loads(path.read_text())


def digest(path: Path) -> str:
    result = hashlib.sha256()
    with path.open("rb") as handle:
        for block in iter(lambda: handle.read(65536), b""):
            result.update(block)
    return result.hexdigest()


def write(name: str, value: object) -> None:
    (OUT / name).write_text(json.dumps(value, ensure_ascii=False, indent=2) + "\n")


def identity(pen: dict) -> tuple:
    return tuple(pen[name] for name in FIELDS)


def ratio(value: Fraction | None) -> dict | None:
    return None if value is None else {"num": value.numerator, "den": value.denominator}


def unpack(value: dict) -> Fraction:
    return Fraction(value["num"], value["den"])


def main() -> None:
    began = time.perf_counter()
    assert digest(AUTHOR / "manifest.json") == "cb6b1ea7ec569768ce7cc45f4390ff13ba51bff3731c055131de440803d65a3d"
    assert digest(AUTHOR / "FINAL.json") == "232dcdad4242be696cfe11deb9fae890e9adb3a0d40d8a6a64f8de996b3328c9"
    manifest = load(AUTHOR / "manifest.json")
    inherited = load(AUTHOR / "run/inputhash-before.json")
    source_refs = load(AUTHOR / "source-evidence.json")
    paths = {AUTHOR / item["path"] for item in manifest["files"]}
    paths |= {AUTHOR / "manifest.json", AUTHOR / "FINAL.json"}
    paths |= {Path(path) for path in inherited}
    paths |= {Path(item["path"]) for item in source_refs}
    paths.add(R / "docs/chanlun/text/blog/084-第84课.md")
    before = {str(path): digest(path) for path in sorted(paths)}
    write("inputhash-before.json", before)
    for item in manifest["files"]:
        path = AUTHOR / item["path"]
        assert before[str(path)] == item["sha256"]
        assert path.stat().st_size == item["bytes"]
    assert all(before[path] == expected for path, expected in inherited.items())
    source_readback = []
    for item in source_refs:
        path = Path(item["path"])
        lines = path.read_text().splitlines()
        assert digest(path) == item["sha256"]
        assert all(lines[line["line"] - 1] == line["text"] for line in item["lines"])
        source_readback.append({"path": str(path), "sha256": digest(path), "lines_checked": [line["line"] for line in item["lines"]], "literal_matches": True})
    write("source-literal-check.json", source_readback)

    source = load(E / "stage66/nested-author/run/source.json")
    prices = [item["price"] for item in source["observations"]]
    assert len(prices) == 443
    assert all(item["index"] == item["time"] == i for i, item in enumerate(source["observations"]))
    assert all(item["untradable"] and item["trade_volume"] == 0 for item in source["observations"])
    del source
    # 审核者为离线全域核对，会整体加载输入；不作在线物理隔离主张。
    prefixes = load(E / "stage66/nested-review/independent-prefixes.json")
    certs = load(E / "stage67/live-review/independent-certificates.json")
    versions = {}
    signatures = {}
    active_first = {}
    stable_first = {}
    families = {}
    states = Counter()
    retracted = []
    geometry = []
    active_retracted = []
    rows = []
    previous_stable = []
    previous_current = None
    previous_active = None
    for cut, prefix in enumerate(prefixes):
        assert prefix["cut"] == cut and prefix["stable_count"] == len(prefix["stable"])
        stable = prefix["stable"]
        active = prefix["active"]
        anchors = prefix["anchors"]
        assert stable[:len(previous_stable)] == previous_stable
        for pen in stable[len(previous_stable):]:
            sig = identity(pen)
            assert sig in active_first
            stable_first[sig] = cut
        if previous_active and identity(previous_active) not in stable_first and (not active or identity(previous_active) != identity(active)):
            active_retracted.append(cut)
        if active and identity(active) not in active_first:
            sig = identity(active)
            assert sig in signatures
            version = versions[signatures[sig]]
            assert version["evidenceState"] == "pending-opposite-fractal"
            active_first[sig] = cut
            a, z = active["start"], active["end"]
            support = prices[:cut + 1]
            start_index = next(i for i, x in enumerate(anchors) if x["raw"] == a)
            assert anchors[start_index + 1]["raw"] == z
            assert all((support[x] < support[x-1] and support[x] < support[x+1]) if bottom else (support[x] > support[x-1] and support[x] > support[x+1]) for x, bottom in [(a, active["up"]), (z, not active["up"])])
            assert z - a >= 4 and len(set((a-1, a, a+1)) & set((z-1, z, z+1))) == 0
            assert all(support[i] != support[i-1] for i in range(a, z+2))
            assert min(support[a:z+1]) == min(active["start_price"], active["end_price"])
            assert max(support[a:z+1]) == max(active["start_price"], active["end_price"])
            geometry.append({"version": version["id"], "knownAt": cut, "start": a, "end": z, "pointbar_shape": True})
        if previous_current and identity(previous_current) not in active_first:
            retracted.append({"version": previous_current["id"], "knownAt": cut})
        current = None
        if anchors:
            anchor = anchors[-1]
            a = anchor["raw"]
            assert a + 1 <= anchor["known_at"] <= cut
            assert anchor["price"] == prices[a]
            assert (prices[a] > prices[a-1] and prices[a] > prices[a+1]) if anchor["top"] else (prices[a] < prices[a-1] and prices[a] < prices[a+1])
            up = not anchor["top"]
            displacement = prices[cut] - prices[a]
            duration = cut - a
            legal_direction = displacement > 0 if up else displacement < 0
            extreme = prices[cut] == (max(prices[a:cut+1]) if up else min(prices[a:cut+1]))
            state = "zero-duration" if duration <= 0 else "direction-invalid" if not legal_direction else "endpoint-not-current-extreme" if not extreme else "waiting-spacing" if duration < 4 else "pending-opposite-fractal"
            family = f"DP:{a}@{anchor['known_at']}"
            version_id = f"{family}:v{cut}"
            families.setdefault(family, {"knownAt": cut, "anchor": a, "end": a, "state": "zero-duration", "v": None})
            current = {"id": version_id, "family": family, "version": cut, "observedAt": cut, "anchorKnownAt": anchor["known_at"], "startFractal": anchor, "start": a, "end": cut, "start_price": prices[a], "end_price": prices[cut], "up": up, "direction": "Up" if up else "Down", "evidenceState": state, "extendable": state in ("waiting-spacing", "pending-opposite-fractal"), "terminalFractalKnownAt": None, "duration": duration, "v": ratio(Fraction(displacement, duration)) if duration else None, "StructuralPenNow_original": None, "source_pointbar_mapping_proven": None}
            versions[version_id] = current
            signatures[identity(current)] = version_id
            states[state] += 1
        rows.append({"cut": cut, "current": current, "sameActive": bool(active and current and identity(active) == identity(current)), "sameStable": bool(current and any(identity(current) == identity(pen) for pen in stable)), "slot": len(anchors)-1 if anchors else None})
        previous_stable, previous_active, previous_current = stable, active, current
    assert len(rows) == 443
    pens = prefixes[-1]["stable"] + [prefixes[-1]["active"]]

    def speed(pen: dict) -> Fraction:
        return Fraction(pen["end_price"] - pen["start_price"], pen["end"] - pen["start"])

    cert_clocks = {}
    for cert in certs:
        witness_clock = max(stable_first[identity(pens[slot])] for slot in cert["witness_pen_slots"])
        assert witness_clock == cert["known_at"]
        cert_clocks[cert["block"]] = witness_clock
    whole = {}
    for g, name in enumerate(NAMES):
        first, last = 5+25*g, 29+25*g
        terminal = pens[last]
        whole[name] = {"start": pens[first]["start"], "end": terminal["end"], "endpointKnownAt": active_first[identity(terminal)], "frozenAt": stable_first[identity(terminal)], "rawEvidenceKnownAt": cert_clocks[5+5*g], "cFrozenL": ratio(speed(terminal)-speed(pens[last-4])), "wholeFrozenL": ratio(speed(terminal)-speed(pens[first]))}
    expected_weak = []
    previous_weak = {}
    comparisons = Counter()
    key_traces = []
    with (AUTHOR / "run/all-prefixes.jsonl").open() as observed, (E / "stage67/live-review/independent-prefix-results.jsonl").open() as earlier:
        for cut, expected in enumerate(rows):
            actual = json.loads(next(observed))
            old = {item["id"]: item for item in (json.loads(next(earlier)) for _ in range(5))}
            current = expected["current"]
            if current:
                assert actual["current"] == current
                assert actual["activeCandidateVersion"] == (signatures[identity(actual["active"])] if actual["active"] else None)
            else:
                assert actual["current"]["state"] == "no-known-anchor" and actual["current"]["v"] is None
            assert actual["currentSameAsActive"] == expected["sameActive"]
            assert actual["currentSameAsAnyStable"] == expected["sameStable"]
            assert actual["templateCurrentRawSlot"] == expected["slot"]
            comparisons["prefixes"] += 1
            force_trace = []
            for g, name in enumerate((*NAMES, "parent")):
                target = whole["c0" if name == "parent" else name]
                base = 80 if name == "parent" else 25+25*g
                anchor = prefixes[cut]["anchors"][base] if len(prefixes[cut]["anchors"]) > base else None
                first = pens[base] if anchor else None
                first_at = stable_first.get(identity(first)) if first else None
                ended = cut >= target["endpointKnownAt"]
                if not anchor:
                    reason, value = "start-anchor-not-known", None
                elif first_at is None or cut < first_at:
                    reason, value = "first-pen-not-stable", None
                elif ended:
                    reason, value = "old-span-ended-new-residual-has-separate-template-slot", None
                else:
                    reason, value = None, unpack(current["v"]) - speed(first)
                eligible = bool(old[name]["local_role_gate"] and not ended and value is not None and old[name]["extreme"] is True and value < unpack(old[name]["b_L"]))
                selected = actual["force"][g]
                assert selected["id"] == name
                assert selected["proposedL"] == ratio(value) and selected["reason"] == reason
                assert selected["jointCandidate"] == eligible
                assert selected["firstStableAt"] == (first_at if first_at is not None and cut >= first_at else None)
                assert selected["oldSpanEnd"] == (target["end"] if ended else None)
                assert selected["frozenAt"] == (target["frozenAt"] if cut >= target["frozenAt"] else None)
                assert selected["rawEvidenceKnownAt"] == (target["rawEvidenceKnownAt"] if cut >= target["rawEvidenceKnownAt"] else None)
                frozen = target["wholeFrozenL"] if name == "parent" else target["cFrozenL"]
                assert selected["frozenL"] == (frozen if cut >= target["frozenAt"] else None)
                assert selected["sourceL"] is None and selected["StructuralPenNow_original"] is None and selected["Completed_original"] is None
                now_id = current["id"] if value is not None else None
                previous = previous_weak.get(name)
                if previous and (not eligible or previous["version"] != now_id):
                    expected_weak.append({"cut": cut, "id": name, "kind": "cancel-endpoint-completion-candidate", "candidate": previous, "reason": "selected-span-ended" if ended else "endpoint-version-superseded" if eligible else "predicate-no-longer-true"})
                    previous_weak.pop(name, None)
                if eligible:
                    candidate = {"version": now_id, "end": cut, "knownAt": cut, "L": ratio(value), "Completed_original": None}
                    previous_weak[name] = candidate
                    expected_weak.append({"cut": cut, "id": name, "kind": "propose-endpoint-completion-candidate", "candidate": candidate})
                force_trace.append({"id": name, "L": ratio(value), "reason": reason, "jointCandidate": eligible, "frozenL": selected["frozenL"], "rawEvidenceKnownAt": selected["rawEvidenceKnownAt"]})
                comparisons["force_rows"] += 1
            if cut in (0, 1, 2, 102, 109, 110, 117, 118, 119, 120, 121, 122, 125, 126, 137, 138, 217, 221, 222, 226, 238, 321, 322, 326, 338, 417, 421, 422, 426, 438, 442):
                key_traces.append({**expected, "primitivePenSlot": expected["slot"], "rawBlockSlot": expected["slot"]//5 if expected["slot"] is not None else None, "force": force_trace})
        assert not observed.read().strip() and not earlier.read().strip()
    assert list(versions.values()) == load(AUTHOR / "run/versions.json")
    assert expected_weak == load(AUTHOR / "run/completion-candidates.json")
    author_events = load(AUTHOR / "run/events.json")
    actual_retracted = [{"version": item["version"], "knownAt": item["knownAt"]} for item in author_events if item["kind"] == "retracted-proposal"]
    assert retracted == actual_retracted
    for kind, table in (("confirmed-rw-active", active_first), ("stable-rw", stable_first)):
        events = [item for item in author_events if item["kind"] == kind]
        assert len(events) == len(table)
        assert all(table[identity(item["pen"])] == item["knownAt"] and signatures[identity(item["pen"])] == item["candidateVersion"] for item in events)
    init = load(AUTHOR / "run/initialization.json")
    assert [x["cut"] for x in init["noAnchor"]] == [0, 1]
    assert [{k: x[k] for k in ("knownAt", "anchor", "end", "state", "v")} for x in init["zeroDurationBirths"]] == list(families.values())
    author_wholes = load(AUTHOR / "run/whole-spans.json")
    for name, expectation in whole.items():
        assert all(author_wholes[name][field] == value for field, value in expectation.items())
    raw_spans = load(AUTHOR / "run/raw-spans.json")
    for raw in raw_spans:
        block = int(raw["id"].split(":")[1])
        first, last = block*5, block*5+4
        assert raw["start"] == pens[first]["start"]
        if last < len(pens):
            terminal = pens[last]
            assert raw["end"] == terminal["end"] and raw["endpointKnownAt"] == active_first[identity(terminal)]
            if identity(terminal) in stable_first:
                assert raw["frozenAt"] == stable_first[identity(terminal)]
                assert raw["frozenL"] == ratio(speed(terminal)-speed(pens[first]))
    assert raw_spans[0]["frozenL"] == {"num": -1000, "den": 1}
    a, r, t = 113, 117, 121
    u = Fraction(prices[r]-prices[a], r-a)
    v = Fraction(prices[t]-prices[r], t-r)
    average = Fraction(prices[t]-prices[a], t-a)
    assert average == Fraction(r-a, t-a)*u + Fraction(t-r, t-a)*v
    assert average-v == Fraction(r-a, t-a)*(u-v)
    assert (u, v, average) == (Fraction(-300), Fraction(500), Fraction(100))
    code = (AUTHOR / "check_pen.py").read_text()
    ast.parse(code)
    compile(code, str(AUTHOR / "check_pen.py"), "exec")
    after = {str(path): digest(path) for path in sorted(paths)}
    write("inputhash-after.json", after)
    assert before == after
    rss = resource.getrusage(resource.RUSAGE_SELF).ru_maxrss * (1 if sys.platform == "darwin" else 1024)
    assert rss <= 96*1024*1024, rss
    summary = {"status": "independent-finite-crosscheck-pass", "one_existing_history": True, "new_histories": 0, "rw_runs": 0, "author_functions_called": 0, "prefixes": len(rows), "versions": len(versions), "families": len(families), "active_matches": len(active_first), "stable_matches": len(stable_first), "endpoint_proposal_retractions": len(retracted), "active_retractions": len(active_retracted), "states": dict(states), "no_anchor_prefixes": 2, "zero_duration_birth_microsteps": len(families), "same_active": sum(x["sameActive"] for x in rows), "same_stable": sum(x["sameStable"] for x in rows), "equality_domain": {"all_prefixes": 443, "current_exists": sum(x["current"] is not None for x in rows), "current_and_active_exist": sum(bool(x["current"] and p["active"]) for x, p in zip(rows, prefixes)), "current_and_nonempty_stable_exist": sum(bool(x["current"] and p["stable"]) for x, p in zip(rows, prefixes)), "identity_fields": FIELDS, "missing_objects": "excluded from substantive equality; output boolean false"}, "comparisons": dict(comparisons), "pointbar_geometry_matches": len(geometry), "raw_certificate_witness_clocks": cert_clocks, "whole": whole, "lemma_121": {"u": ratio(u), "R": ratio(v), "A": ratio(average), "L_R": ratio(v-1000), "L_A": ratio(average-1000)}, "static_tools_available": {name: importlib.util.find_spec(name) is not None for name in ("ruff", "mypy", "pylint", "black")}, "ast_compile_without_pyc": "pass", "input_hashes_unchanged": True, "peak_rss_bytes": rss, "elapsed_seconds": time.perf_counter()-began, "reviewer_read_scope": "offline whole input preload and postcomputed crosscheck; no physical future isolation claim", "source_StructuralPenNow": None, "RootArm67": None, "Completed": None, "Owner": None, "Next": None, "F2": None, "full_F1": None}
    write("independent-results.json", summary)
    write("independent-key-trajectories.json", key_traces)
    write("independent-pointbar-geometry.json", geometry)
    write("independent-completion-candidates.json", expected_weak)
    print(json.dumps(summary, ensure_ascii=False))


if __name__ == "__main__":
    main()
