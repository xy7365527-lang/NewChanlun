#!/usr/bin/env python3
"""#1372：从真实驱动事件量化有限负载，不把采样窗口扩称生产容量。"""

import argparse
import hashlib
import json
from pathlib import Path


def report(path):
    raw = path.read_bytes()
    events = [json.loads(line) for line in raw.splitlines()]
    ingest = [event for event in events if event["kind"] == "ingest"]
    by_operation = {event["operation"]: event for event in ingest}
    if len(by_operation) != len(ingest):
        raise ValueError("存在重复操作记录，不能静默覆盖")
    output = {"status": "NOT_VERIFIED", "events_sha256": hashlib.sha256(raw).hexdigest(),
              "offers": len(ingest), "transport_counts": {}, "windows": [],
              "scope": "160项/500ms具名轨迹的实测窗口；不证明无限输入或生产容量"}
    for event in ingest:
        transport = event["result"]["transport"]
        output["transport_counts"][transport] = output["transport_counts"].get(transport, 0) + 1
    phases = [event for event in events if event["kind"] == "phase_begin" and event["first_operation"] == 96]
    if len(phases) != 1:
        output["reason"] = "缺少唯一live64阶段起点；现有失败原件不补造窗口"
        return output
    anchor = int(phases[0]["phase_anchor_ns"])
    samples = sorted((event for event in events if event["kind"] == "frontier" and event["phase_start"] == 96),
                     key=lambda event: int(event["phase_offset_ns"]))
    if not samples:
        output["reason"] = "缺少live前沿实测"
        return output
    offsets = [int(event["phase_offset_ns"]) for event in samples]
    output["max_observation_interval_ms"] = max((b-a for a, b in zip(offsets, offsets[1:])), default=0) / 1000000
    for begin, end in ((0, 10000), (10000, 20000), (20000, 30000)):
        # 只使用声明窗口内部实测点；不借窗口外的迟到采样证明窗口内前进。
        inside = [event for event in samples
                  if begin * 1000000 <= int(event["phase_offset_ns"]) <= end * 1000000]
        a, b = (inside[0], inside[-1]) if inside else (None, None)
        scheduled = [event for event in ingest if begin*1000000 <= int(event["planned_ns"])-anchor < end*1000000]
        observed = [event for event in ingest if begin*1000000 <= int(event["sent_ns"])-anchor < end*1000000]
        window = {"declared_window_ms": [begin, end], "scheduled_offers": len(scheduled),
                  "observed_offers": len(observed), "first_sample": a, "last_sample": b}
        if a is not None and b is not None and int(a["phase_offset_ns"]) < int(b["phase_offset_ns"]):
            window["evidence_status"] = "OBSERVED_WITHIN_DECLARED_WINDOW"
            window["actual_sample_span_ms"] = [int(event["phase_offset_ns"]) / 1000000 for event in (a, b)]
            window["accepted_progress"] = int(b["accepted"]) - int(a["accepted"])
            window["committed_progress"] = int(b["committed"]) - int(a["committed"])
            window["start_unobserved_ms"] = int(a["phase_offset_ns"]) / 1000000 - begin
            window["end_unobserved_ms"] = end - int(b["phase_offset_ns"]) / 1000000
            window["end_accepted_pending"] = int(b["accepted"]) - int(b["committed"])
        else:
            window["evidence_status"] = "NOT_VERIFIED_INSUFFICIENT_WITHIN_WINDOW_SAMPLES"
        output["windows"].append(window)
    terminals = [event for event in events if event["kind"] == "phase_end" and event["first_operation"] == 96]
    output["last_observed_frontier"] = terminals[0] if len(terminals) == 1 else samples[-1]
    live = [event for event in ingest if 96 <= event["operation"] < 160]
    if live:
        output["live_max_dispatch_delay_ms"] = max(int(event["sent_ns"])-int(event["planned_ns"]) for event in live) / 1000000
        output["live_max_reply_ms"] = max(int(event["received_ns"])-int(event["sent_ns"]) for event in live) / 1000000
        output["live_reply_ms_by_operation"] = {str(event["operation"]): (int(event["received_ns"])-int(event["sent_ns"])) / 1000000 for event in live}
    if set(by_operation) != set(range(160)):
        output["reason"] = "160项实际输送记录不全"
    elif any(by_operation[index]["result"]["transport"] != ("DeliveryUnknown" if index == 95 else "Received") for index in range(160)):
        output.update(status="FAIL", reason="健康输送未完整接收或预声明故障未出现")
    elif any(window["evidence_status"].startswith("NOT_VERIFIED") for window in output["windows"]):
        output["reason"] = "声明窗口内缺少两个不同时间的实测点，不能借窗口外数据补证"
    elif any(window.get("accepted_progress", 0) <= 0 or window.get("committed_progress", 0) <= 0 for window in output["windows"]):
        output.update(status="FAIL", reason="存在没有接纳/提交前进证据的窗口")
    elif any(window["scheduled_offers"] != 20 for window in output["windows"]):
        output.update(status="FAIL", reason="没有按声明2/s调度窗口")
    elif int(output["last_observed_frontier"]["accepted"]) != 160 or int(output["last_observed_frontier"]["committed"]) != 160:
        output["reason"] = "末次实测未覆盖完整160提交，须以另存正式收据和权威库核定"
    else:
        output["status"] = "OBSERVED_BOUNDED_PROGRESS_REQUIRES_TRAJECTORY_AND_CONSUMER_REVIEW"
    return output


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--events", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    result = report(args.events)
    with args.output.open("x") as stream:
        json.dump(result, stream, ensure_ascii=False, indent=2)
        stream.write("\n")
    print(json.dumps({"status": result["status"], "offers": result["offers"], "windows": len(result["windows"])}, ensure_ascii=False))
    return 0 if result["status"].startswith("OBSERVED_") else 2


if __name__ == "__main__":
    raise SystemExit(main())
