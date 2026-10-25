#!/usr/bin/env python3
"""#1488 验收 S1：由冻结 9011 源重建 S86 输入（前 86 条信封 + 86 事件时钟）。

配方与 rust/src/bin/s_session_v3/driver.rs `testonly_s3_prefix_measurement`
（S3_CONTENT_MEASURE_PREFIX）逐字段一致；并逐行核对仓内 receipt
.chanlun/review-results/issue1394-financial-owner-20260930/current-build-inputs/
018-FULL9011-MATERIAL-RECHECK.json.raw.json 的 ordered_request_hashes 前 86 项。

用法：
  build_s86_inputs.py --bars <RW05_...bars.json> [--out DIR] [--check]
--check：只重建并与 DIR 内已入仓文件逐字节比较，不写盘。
"""

import argparse
import hashlib
import json
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
REPO = HERE.parents[3]
PREFIX = 86
SOURCE_SHA256 = "76472995cbf0825ceeeaa94e89bd60d1b1416d6c7e0a7ec3a1b0ec5a90c60087"
RECEIPT = REPO / ".chanlun/review-results/issue1394-financial-owner-20260930/current-build-inputs/018-FULL9011-MATERIAL-RECHECK.json.raw.json"
SESSION = "s-session-testonly-001"
PROFILE = "ohlc_integer_tb02a_v1"


def canonical(value):
    return json.dumps(value, ensure_ascii=False, sort_keys=True, separators=(",", ":"),
                      allow_nan=False)


def sha(data):
    return hashlib.sha256(data).hexdigest()


def num(v):
    # Rust serde_json Value::to_string：整数无小数点，1.0 之类浮点保留 ".0"。
    if isinstance(v, bool):
        raise ValueError("bool")
    if isinstance(v, float):
        return repr(v)
    return str(v)


def envelope(message_id, payload):
    return {"schema_revision": "s-session/3", "session_id": SESSION, "session_generation": "1",
            "source_namespace": "testonly.ohlc", "source_epoch": "1", "message_id": message_id,
            "producer_id": "driver-fixture", "producer_epoch": "1",
            "payload_hash": sha(canonical(payload).encode()), "causal_refs": [], "payload": payload}


def build(bars):
    lines, clock_events = [], {}
    for i, bar in enumerate(bars[:PREFIX]):
        n = i + 1
        ns = 100 + 3 * n
        clock_events[f"input-{n}"] = {"accept_ns": str(ns),
                                      "attempts": [{"begin_ns": str(ns + 1), "commit_ns": str(ns + 2)}]}
        event = {"event_id": f"long-raw-{i}", "revision": "1", "seq": num(bar["source_index"]),
                 "received_at": "2026-09-14T00:00:00Z", "raw_text": canonical(bar),
                 "open": num(bar["open"]), "high": num(bar["high"]), "low": num(bar["low"]),
                 "close": num(bar["close"]), "timestamp": num(bar["timestamp"]), "volume": "1"}
        payload = {"op": "ingest", "target_session_id": SESSION, "target_session_generation": "1",
                   "writer_epoch": "1", "clock_event_id": f"input-{n}",
                   "raw_input": {"schema_revision": "s-ohlc/1", "session_id": SESSION,
                                 "source_namespace": "testonly.ohlc", "source_epoch": "1",
                                 "instrument": "TEST.LONG.TB03B", "profile": PROFILE,
                                 "events": [event]}}
        lines.append(canonical(envelope(f"message-{n}", payload)))
    clock = {"schema_revision": "s-clock-plan/1", "clock_plan_id": "s3-frozen-prefix-cost",
             "origin_utc": "2026-09-18T00:00:00Z", "unit": "ns", "events": clock_events}
    return lines, clock


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--bars", required=True)
    ap.add_argument("--out", default=str(HERE))
    ap.add_argument("--check", action="store_true")
    a = ap.parse_args()
    raw = Path(a.bars).read_bytes()
    if sha(raw) != SOURCE_SHA256:
        sys.exit(f"源哈希不符：{sha(raw)}")
    bars = json.loads(raw)
    assert len(bars) == 9011
    lines, clock = build(bars)
    receipt = json.loads(json.loads(RECEIPT.read_text())["content_utf8"])
    expected = receipt["ordered_request_hashes"][:PREFIX]
    got = [sha(line.encode()) for line in lines]
    if got != expected:
        bad = next(i for i, (x, y) in enumerate(zip(got, expected)) if x != y)
        sys.exit(f"第 {bad + 1} 条信封与仓内 receipt 不符")
    files = {
        "messages.jsonl": "".join(line + "\n" for line in lines).encode(),
        "clock-plan.s86.json": canonical(clock).encode(),
        "bars.s86.json": canonical(bars[:PREFIX]).encode(),
    }
    out = Path(a.out)
    if a.check:
        diff = [name for name, data in files.items() if (out / name).read_bytes() != data]
        if diff:
            sys.exit(f"重建与入仓不一致：{diff}")
        print("CHECK_OK", {k: sha(v) for k, v in files.items()})
        return
    for name, data in files.items():
        (out / name).write_bytes(data)
    print(json.dumps({k: sha(v) for k, v in files.items()}, indent=1))


if __name__ == "__main__":
    main()
