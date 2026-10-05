#!/usr/bin/env python3
"""Stage67 独立有理数复算。只复用已审前缀，不导入作者代码或重跑 R_W。"""
from __future__ import annotations

import hashlib
import json
import resource
import sys
import time
from collections import Counter
from fractions import Fraction
from pathlib import Path
from typing import Any, Iterator

OUT = Path(__file__).resolve().parent
E = OUT.parents[1]
AUTHOR = OUT.parent / "live-author"
PRIOR = E / "stage66/nested-review"
NAMES = ("P0", "P1", "P2", "c0", "parent")


def dump(name: str, data: Any) -> None:
    (OUT / name).write_text(json.dumps(data, ensure_ascii=False, indent=2) + "\n")


def sha(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(65536), b""):
            digest.update(chunk)
    return digest.hexdigest()


def array_items(path: Path) -> Iterator[dict[str, Any]]:
    """逐项解码数组，最多预读 64 KiB；不物化未来前缀或未来结果。"""
    decoder = json.JSONDecoder()
    with path.open() as handle:
        buffer = ""
        started = False
        eof = False
        while True:
            buffer = buffer.lstrip()
            if not started and buffer:
                assert buffer[0] == "["
                buffer = buffer[1:]
                started = True
            buffer = buffer.lstrip().lstrip(",").lstrip()
            if buffer.startswith("]"):
                return
            try:
                value, end = decoder.raw_decode(buffer)
            except json.JSONDecodeError:
                if eof:
                    raise
                chunk = handle.read(65536)
                eof = not chunk
                buffer += chunk
                continue
            buffer = buffer[end:]
            yield value


def packed(value: Fraction | None) -> dict[str, int] | None:
    if value is None:
        return None
    return {"num": value.numerator, "den": value.denominator}


def unpack(value: dict[str, int] | None) -> Fraction | None:
    return None if value is None else Fraction(value["num"], value["den"])


def overlap(intervals: list[list[int]]) -> list[int]:
    return [max(a for a, _ in intervals), min(b for _, b in intervals)]


def pen_range(pen: dict[str, Any]) -> list[int]:
    return sorted((pen["start_price"], pen["end_price"]))


class Auditor:
    """状态只包含已到达观察、当前前缀及已经在过去成立的证书。"""

    def __init__(self) -> None:
        self.prices: list[int] = []
        self.times: list[int] = []
        self.previous_stable: list[dict[str, Any]] = []
        self.certs: dict[int, dict[str, Any]] = {}
        self.wholes: dict[str, dict[str, Any]] = {}
        self.clock: dict[str, dict[str, Any]] = {name: {} for name in NAMES}
        self.anchor_first: dict[int, int] = {}
        self.stable_first: dict[int, int] = {}
        self.narrow_containment: list[int] = []
        self.firsts = {
            name: {mode: {key: None for key in ("weak", "extreme", "complete_candidate")}
                   for mode in ("A", "R")}
            for name in NAMES
        }
        self.last_truth: dict[tuple[str, str], bool] = {}
        self.last_anchor: dict[tuple[str, str], int] = {}
        self.transitions: list[dict[str, Any]] = []
        self.withdrawals: list[dict[str, Any]] = []
        self.nulls: Counter[str] = Counter()
        self.bounded_rows = 0
        self.last_prefix: dict[str, Any] = {}

    def span(self, start: int, end: int) -> list[int]:
        assert 0 <= start <= end < len(self.prices)
        return [min(self.prices[start:end + 1]), max(self.prices[start:end + 1])]

    def velocity(self, pen: dict[str, Any]) -> Fraction:
        start, end = pen["start"], pen["end"]
        return Fraction(self.prices[end] - self.prices[start], self.times[end] - self.times[start])

    def force(self, stable: list[dict[str, Any]], start: int, end: int) -> Fraction:
        first = next(p for p in stable if p["start"] == start)
        last = next(p for p in stable if p["end"] == end)
        assert first["end"] <= end and start <= last["start"]
        return self.velocity(last) - self.velocity(first)

    def narrow(self, stable: list[dict[str, Any]], block: int) -> tuple[dict[str, Any] | None, str]:
        """用方向归一化后的严格顶分型重新验证窄证，含包含失败。"""
        start_slot = block * 5
        group = stable[start_slot:start_slot + 5]
        if len(group) != 5:
            return None, "insufficient-stable"
        sign = 1 if group[0]["up"] else -1
        features = []
        for slot in range(start_slot, len(stable)):
            pen = stable[slot]
            if pen["up"] == group[0]["up"]:
                continue
            interval = sorted((sign * pen["start_price"], sign * pen["end_price"]))
            features.append((slot, interval))
        for index in range(1, len(features)):
            left, right = features[index - 1][1], features[index][1]
            contained = ((left[0] <= right[0] and left[1] >= right[1]) or
                         (right[0] <= left[0] and right[1] >= left[1]))
            if contained:
                return None, "containment"
            if index < 2:
                continue
            x, y, z = [entry[1] for entry in features[index - 2:index + 1]]
            if not (y[1] > x[1] and y[1] > z[1]):
                continue
            first3 = overlap([pen_range(p) for p in group[:3]])
            start, end = group[0]["start"], group[-1]["end"]
            valid = (y[0] > x[0] and y[0] > z[0] and
                     max(x[0], y[0]) < min(x[1], y[1]) and
                     first3[0] < first3[1] and
                     self.span(start, end) == sorted((self.prices[start], self.prices[end])) and
                     sign * self.prices[end] == y[1])
            if not valid:
                return None, "geometry-false"
            return {"block": block, "start": start, "end": end, "up": sign == 1,
                    "range": self.span(start, end), "evidence": "narrow",
                    "witness_pen_slots": [entry[0] for entry in features[index - 2:index + 1]]}, "valid"
        return None, "no-selected-fractal"

    def selected15(self, stable: list[dict[str, Any]]) -> dict[str, Any] | None:
        """只检查已冻结第 15 块的选定 065/071 条件，不实现通用包含解析。"""
        if len(stable) < 83 or 14 not in self.certs:
            return None
        old, continuation = stable[75:80], stable[80:83]
        assert [p["up"] for p in old] == [True, False, True, False, True]
        assert [p["up"] for p in continuation] == [False, True, False]
        lows = [old[0]["start_price"], old[1]["end_price"], old[3]["end_price"]]
        highs = [old[0]["end_price"], old[2]["end_price"], old[4]["end_price"]]
        violations_before = {(i + 1, j + 1) for i, high in enumerate(highs)
                             for j, low in enumerate(lows) if j - i >= 2 and low <= high}
        extended = lows + [continuation[0]["end_price"]]
        violations_after = {(i + 1, j + 1) for i, high in enumerate(highs)
                            for j, low in enumerate(extended) if j - i >= 2 and low <= high}
        first, latter = overlap([pen_range(p) for p in old[:3]]), overlap([pen_range(p) for p in continuation])
        valid = (not violations_before and violations_after == {(2, 4)} and
                 first[0] < first[1] and latter[0] < latter[1] and
                 continuation[2]["end_price"] < continuation[0]["end_price"] and
                 continuation[1]["end_price"] < continuation[0]["start_price"] and
                 self.certs[14]["end"] == old[0]["start"])
        if not valid:
            return None
        return {"block": 15, "start": old[0]["start"], "end": old[-1]["end"], "up": True,
                "range": self.span(old[0]["start"], old[-1]["end"]),
                "evidence": "selected-source071", "witness_pen_slots": list(range(75, 83))}

    def mark(self, name: str, kind: str, t: int, **details: Any) -> None:
        self.clock[name].setdefault(kind, {"first_known_at": t, **details})

    def reading(self, prefix: dict[str, Any], start: int | None, mode: str, closed: bool) -> dict[str, Any]:
        if closed:
            return {"L": None, "reason": "local-closed-certificate-no-live-domain"}
        if start is None:
            return {"L": None, "reason": "c-role-start-not-known"}
        first = next((p for p in prefix["stable"] if p["start"] == start), None)
        if first is None:
            return {"L": None, "reason": "first-pen-not-stable"}
        if mode == "R":
            if not prefix["anchors"]:
                return {"L": None, "reason": "no-known-anchor"}
            anchor = prefix["anchors"][-1]["raw"]
            kind = "unconfirmed-residual"
        else:
            pen = prefix["active"]
            if pen is None:
                pen = prefix["stable"][-1] if prefix["stable"] else None
            if pen is None:
                return {"L": None, "reason": "no-active-or-stable-pen"}
            anchor = pen["start"]
            kind = "real-active-start" if prefix["active"] else "recent-stable-start"
        if anchor < start:
            return {"L": None, "reason": "selected-pen-anchor-precedes-c", "anchor": anchor}
        elapsed = self.times[-1] - self.times[anchor]
        if elapsed <= 0:
            return {"L": None, "reason": "zero-duration", "anchor": anchor}
        v = Fraction(self.prices[-1] - self.prices[anchor], elapsed)
        v0 = self.velocity(first)
        return {"L": packed(v - v0), "v": packed(v), "first_v": packed(v0),
                "anchor": anchor, "kind": kind, "duration": elapsed,
                "anchor_price": self.prices[anchor], "first_pen": first, "signed": True}

    def row(self, name: str, prefix: dict[str, Any], start: int | None, ready: bool,
            b_l: Fraction | None, extreme: bool | None, closed: bool, extra: dict[str, Any]) -> dict[str, Any]:
        t = prefix["cut"]
        out = {"id": name, "cut": t, "price": self.prices[-1], "c_start": start,
               "local_role_gate": ready, "source_role_gate": None, "source_complete": None,
               "extreme": extreme, "lifecycle": "closed-local-certificate" if closed else "candidate-open",
               "eligible_now": ready and not closed, **extra}
        for mode in ("A", "R"):
            r = self.reading(prefix, start, mode, closed)
            value = unpack(r["L"])
            weak = None if value is None or b_l is None else value < b_l
            joint = bool(ready and not closed and weak is True and extreme is True)
            r.update(weak=weak, complete_candidate=joint)
            for key, predicate in (("weak", weak), ("extreme", extreme), ("complete_candidate", joint)):
                if ready and not closed and predicate is True and self.firsts[name][mode][key] is None:
                    self.firsts[name][mode][key] = t
            identity = (name, mode)
            if self.last_truth.get(identity, False) and not joint:
                reason = "local-certificate-closes-domain" if closed else "predicate-no-longer-true"
                r.update(candidate_withdrawn_at_this_prefix=True, withdrawal_reason=reason)
                self.withdrawals.append({"id": name, "mode": mode, "cut": t, "reason": reason})
            self.last_truth[identity] = joint
            if "anchor" in r:
                previous = self.last_anchor.get(identity)
                if previous is not None and previous != r["anchor"]:
                    self.transitions.append({"id": name, "mode": mode, "cut": t,
                                             "old_anchor": previous, "new_anchor": r["anchor"], "L": r["L"]})
                self.last_anchor[identity] = r["anchor"]
            if value is None:
                self.nulls[r["reason"]] += 1
            out[mode] = r
        self.bounded_rows += 1
        return out

    def step(self, observation: dict[str, Any], prefix: dict[str, Any]) -> list[dict[str, Any]]:
        t = len(self.prices)
        assert prefix["cut"] == observation["index"] == observation["time"] == t
        self.prices.append(observation["price"])
        self.times.append(observation["time"])
        stable = prefix["stable"]
        assert len(stable) == prefix["stable_count"]
        assert stable[:len(self.previous_stable)] == self.previous_stable
        pens = stable + ([prefix["active"]] if prefix["active"] else [])
        for slot, pen in enumerate(pens):
            start, end = pen["start"], pen["end"]
            assert 0 <= start < end <= t
            assert (pen["start_price"], pen["end_price"]) == (self.prices[start], self.prices[end])
            assert pen["up"] == (self.prices[end] > self.prices[start])
            if slot:
                assert pens[slot - 1]["end"] == start and pens[slot - 1]["up"] != pen["up"]
        for anchor in prefix["anchors"]:
            assert 0 <= anchor["raw"] <= anchor["known_at"] <= t
            assert anchor["price"] == self.prices[anchor["raw"]]
            self.anchor_first.setdefault(anchor["raw"], t)
        for pen in stable:
            self.stable_first.setdefault(pen["start"], t)
        self.previous_stable = stable
        for block in range(21):
            cert, status = self.narrow(stable, block)
            if block == 15 and status == "containment":
                self.narrow_containment.append(t)
            if cert is not None:
                self.certs.setdefault(block, {**cert, "known_at": t})
        special = self.selected15(stable)
        if special:
            self.certs.setdefault(15, {**special, "known_at": t})
        rows = []
        for group_index, name in enumerate(NAMES[:4]):
            bslot = 1 + group_index * 5
            for slot, anchor_key, stable_key in ((5 * bslot, "start_anchor", "b_first_pen_stable"),
                                                  (5 * (bslot + 4), "c_anchor_observed", "c_first_pen_stable")):
                if len(prefix["anchors"]) > slot:
                    at = prefix["anchors"][slot]["raw"]
                    self.mark(name, anchor_key, t, at=at)
                    if at in self.stable_first:
                        self.mark(name, stable_key, t, at=at)
            if bslot - 1 in self.certs:
                self.mark(name, "template_start_role", t, at=self.certs[bslot - 1]["end"])
            if bslot in self.certs:
                self.mark(name, "b_closed", t, start=self.certs[bslot]["start"], end=self.certs[bslot]["end"])
            cstart, ready, b_l, extreme, extra = None, False, None, None, {}
            if set(range(bslot, bslot + 4)) <= self.certs.keys():
                b = self.certs[bslot]
                k = [self.certs[j] for j in range(bslot + 1, bslot + 4)]
                core = overlap([cert["range"] for cert in k])
                cstart = k[-1]["end"]
                sequence = [b] + k
                ready = core[0] < core[1] and all(sequence[j]["up"] != sequence[j + 1]["up"] for j in range(3))
                b_l = self.force(stable, b["start"], b["end"])
                prior = self.span(b["start"], cstart)[1 if b["up"] else 0]
                extreme = self.prices[t] > prior if b["up"] else self.prices[t] < prior
                extra = {"K": core, "b_L": packed(b_l), "prior_extreme": prior,
                         "object_start": b["start"], "direction": "Up" if b["up"] else "Down"}
                self.mark(name, "K_and_c_role", t, c_start=cstart, K=core)
            closed = bslot + 4 in self.certs
            rows.append(self.row(name, prefix, cstart, ready, b_l, extreme, closed, extra))
            if name not in self.wholes and set(range(bslot, bslot + 5)) <= self.certs.keys():
                b, c = self.certs[bslot], self.certs[bslot + 4]
                c_l = self.force(stable, c["start"], c["end"])
                endpoint_extreme = self.prices[c["end"]] > prior if b["up"] else self.prices[c["end"]] < prior
                self.wholes[name] = {"id": name, "start": b["start"], "end": c["end"],
                                     "whole": self.span(b["start"], c["end"]), "known_at": t,
                                     "b_L": packed(b_l), "c_L": packed(c_l),
                                     "endpoint_local": bool(c_l < b_l and endpoint_extreme)}
                self.mark(name, "whole_endpoint_certificate", t, end=c["end"], is_original=False)
        cstart, ready, b_l, extreme, extra = None, False, None, None, {}
        if set(NAMES[:3]) <= self.wholes.keys() and 0 in self.certs:
            members = [self.wholes[name] for name in NAMES[:3]]
            core = overlap([whole["whole"] for whole in members])
            b = self.certs[0]
            cstart = members[-1]["end"]
            b_l = self.force(stable, b["start"], b["end"])
            ready = core[0] < core[1] and all(whole["endpoint_local"] for whole in members)
            prior = self.span(b["start"], cstart)[0]
            extreme = self.prices[t] < prior
            extra = {"K": core, "b_L": packed(b_l), "prior_extreme": prior,
                     "object_start": b["start"], "direction": "Down"}
            self.mark("parent", "K_and_c_role", t, c_start=cstart, K=core)
            if cstart in self.stable_first:
                self.mark("parent", "c_first_pen_stable", t, at=cstart, actual_first_stable=self.stable_first[cstart])
        rows.append(self.row("parent", prefix, cstart, ready, b_l, extreme, "c0" in self.wholes, extra))
        self.last_prefix = prefix
        return rows


def main() -> None:
    started = time.perf_counter()
    source = json.loads((E / "stage66/nested-author/run/source.json").read_text())
    auditor = Auditor()
    observed: dict[tuple[str, int], dict[str, Any]] = {}
    count = 0
    bid, ask = int(source["initial"]["bid_quantity"]), int(source["initial"]["ask_quantity"])
    with (OUT / "independent-prefix-results.jsonl").open("w") as handle:
        for t, prefix in enumerate(array_items(PRIOR / "independent-prefixes.json")):
            observation = source["observations"][t]
            if t:
                event = source["events"][t - 1]
                assert event["id"] == event["time"] == t and event["side"] == "bid"
                assert event["price"] == source["bid"] and int(event["quantity_before"]) == bid
                assert event["action"] in ("add", "cancel") and int(event["amount"]) > 0
                bid += (1 if event["action"] == "add" else -1) * int(event["amount"])
                assert bid == int(event["quantity_after"]) > 0
            assert int(observation["bid_quantity"]) == bid and int(observation["ask_quantity"]) == ask
            assert Fraction(source["ask"] * bid + source["bid"] * ask, ask + bid) == observation["price"]
            assert observation["trade_volume"] == 0
            for row in auditor.step(observation, prefix):
                handle.write(json.dumps(row, ensure_ascii=False, separators=(",", ":")) + "\n")
                observed[(row["id"], t)] = row
            count += 1
    assert count == 443 and auditor.bounded_rows == 2215
    dump("independent-online-seal.json", {
        "rows": auditor.bounded_rows, "prefixes": count,
        "prefix_results_sha256": sha(OUT / "independent-prefix-results.jsonl"),
        "reference_stdout_read": False, "final_objects_read": False,
        "final_pens_read": False, "final_parent_read": False,
        "author_calculation_or_outputs_read": False,
        "input_driver_scope": "source observation/event list; prefix array streamed; Auditor.step receives only one observed row and current prefix",
    })
    # 到此后才读取作者结果与前轮最终对象，作为交叉核对的目标。
    del source
    mismatches = []
    for wanted in array_items(AUTHOR / "prefix-results.json"):
        got = observed[(wanted["id"], wanted["cut"])]
        if got != wanted:
            mismatches.append({"id": wanted["id"], "cut": wanted["cut"], "independent": got, "author": wanted})
    dump("row-mismatches.json", mismatches)
    assert not mismatches
    assert auditor.firsts == json.loads((AUTHOR / "first-triggers.json").read_text())
    assert auditor.transitions == json.loads((AUTHOR / "active-reanchoring.json").read_text())
    author_certs = json.loads((AUTHOR / "online-raw-certificates.json").read_text())
    assert sorted(auditor.certs.values(), key=lambda c: c["block"]) == sorted(author_certs, key=lambda c: c["block"])
    assert auditor.narrow_containment == [item["cut"] for item in json.loads((AUTHOR / "S15-narrow-status.json").read_text())]
    c0, b0 = auditor.wholes["c0"], auditor.certs[0]
    parent_clock = auditor.clock["parent"]
    parent_clock.update({
        "start_anchor": {"first_known_at": auditor.anchor_first[b0["start"]], "at": b0["start"]},
        "b_first_pen_stable": {"first_known_at": auditor.stable_first[b0["start"]], "at": b0["start"]},
        "b_raw_closed": {"first_known_at": b0["known_at"], "start": b0["start"], "end": b0["end"]},
        "c_anchor_observed": {"first_known_at": auditor.anchor_first[c0["start"]], "at": c0["start"]},
        "c_first_pen_stable_primitive": {"first_known_at": auditor.stable_first[c0["start"]], "at": c0["start"]},
        "whole_endpoint_certificate": {"first_known_at": c0["known_at"], "end": c0["end"], "is_original": False},
    })
    clocks = json.loads((AUTHOR / "role-first-known.json").read_text())
    for name, entries in auditor.clock.items():
        assert entries.keys() == clocks[name].keys()
        for key, value in entries.items():
            assert value == {k: v for k, v in clocks[name][key].items() if k != "basis"}
    final_objects = json.loads((PRIOR / "independent-objects.json").read_text())
    final_parent = json.loads((PRIOR / "independent-parent.json").read_text())
    final_pens = json.loads((PRIOR / "independent-pens.json").read_text())
    assert len(final_pens) == 110
    endpoints = []
    for old in final_objects + [{"id": "parent", "end": final_parent["local_happened_at"], "c_force": final_parent["c_force"]}]:
        row = observed[(old["id"], old["end"])]
        endpoints.append({"id": old["id"], "cut": old["end"], "old_L": old["c_force"]["L"],
                          "A_L": row["A"]["L"], "R_L": row["R"]["L"],
                          "A_equal": unpack(row["A"]["L"]) == old["c_force"]["L"],
                          "R_equal": unpack(row["R"]["L"]) == old["c_force"]["L"]})
    full_c = auditor.force(auditor.last_prefix["stable"], c0["start"], c0["end"])
    raw_b = auditor.force(auditor.last_prefix["stable"], b0["start"], b0["end"])
    assert full_c == -3500 and raw_b == -1000
    dump("independent-certificates.json", list(auditor.certs.values()))
    dump("independent-role-clocks.json", auditor.clock)
    dump("independent-first-triggers.json", auditor.firsts)
    dump("independent-endpoints.json", endpoints)
    dump("independent-wholes.json", auditor.wholes)
    dump("independent-withdrawals.json", auditor.withdrawals)
    dump("independent-reanchoring.json", auditor.transitions)
    dump("independent-witnesses.json", [observed[(name, t)] for name, t in
         [("P0", 109), ("P0", 118), ("P0", 121), ("P0", 122), ("P0", 126),
          ("P1", 218), ("P1", 220), ("P1", 222), ("c0", 418), ("c0", 420),
          ("c0", 422), ("parent", 338), ("parent", 342), ("parent", 421),
          ("parent", 422), ("parent", 423), ("parent", 438)]])
    rss = resource.getrusage(resource.RUSAGE_SELF).ru_maxrss * (1 if sys.platform == "darwin" else 1024)
    assert rss <= 96 * 1024 * 1024
    result = {"status": "all-2215-rows-and-clocks-exactly-match", "prefixes": count,
              "objects": 5, "rows": auditor.bounded_rows, "row_mismatches": len(mismatches),
              "peak_rss_bytes": rss, "elapsed_seconds": time.perf_counter() - started,
              "certificates": len(auditor.certs), "firsts": auditor.firsts,
              "endpoints": endpoints, "full_c0_L": packed(full_c), "raw_b_L": packed(raw_b),
              "null_reasons_two_modes": dict(auditor.nulls),
              "S15_containment_first": min(auditor.narrow_containment),
              "S15_selected_first": auditor.certs[15]["known_at"]}
    dump("independent-result.json", result)
    print(json.dumps(result, ensure_ascii=False))


if __name__ == "__main__":
    main()
