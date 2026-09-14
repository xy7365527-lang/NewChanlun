#!/usr/bin/env python3
"""#1373：将独立原始账簿封装成正式消息；不计算任何预期结构。"""

import argparse
import datetime
import hashlib
import json
from pathlib import Path
import sys

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from s_service_control import canonical


def dump(path, value):
    with path.open("x") as stream:
        json.dump(value, stream, ensure_ascii=False, indent=2)
        stream.write("\n")


def prepare(output, sockets, binary, python, node, port_base):
    root = Path(__file__).resolve().parents[1]
    fixtures = root / "tests/fixtures/tb02a"
    ledger_bytes = (fixtures / "raw-ledger.json").read_bytes()
    oracle_bytes = (fixtures / "hand-oracle.json").read_bytes()
    ledger = json.loads(ledger_bytes)
    cases = {case["case_id"]: case["raw_bars"] for case in ledger["cases"]}
    if len(cases) != len(ledger["cases"]) or not cases:
        raise ValueError("原始账簿case身份重复或为空")
    runs = [(key, bars, key, None) for key, bars in cases.items()]
    before, after = cases["high_only_before_revision"], cases["high_only_after_revision"]
    changed = [(old, new) for old, new in zip(before, after) if old != new]
    if len(before) != len(after) or len(changed) != 1 or changed[0][0]["raw_id"] != changed[0][1]["raw_id"]:
        raise ValueError("具名修订必须恰好改变同身份的一根原始K")
    runs.append(("revision_recovery", before + [changed[0][1]], "high_only_after_revision", len(before)))
    output, sockets = output.resolve(), sockets.resolve()
    output.mkdir(parents=True, exist_ok=False)
    sockets.mkdir(parents=True, exist_ok=False)
    plan = json.loads((root / "tests/fixtures/tb01c/runtime-plan.json").read_text())
    origin = datetime.datetime(2000, 1, 1, tzinfo=datetime.timezone.utc)
    origin_ns = 946684800000000000
    configurations = []
    for sequence, (case_id, bars, expected_case, revision_index) in enumerate(runs):
        for arm, fault in (("a", None), ("b", None)):
            # 两次运行采用相同具名故障和语义时钟，目录/进程保持独立。
            fault = revision_index if revision_index is not None else fault
            directory = output / (case_id + "-" + arm)
            directory.mkdir()
            sock = sockets / (str(sequence) + arm + ".sock")
            if len(str(sock).encode()) > 100:
                raise ValueError("Unix socket路径超出本运行声明的100字节界限")
            sid = "tb02a-" + case_id
            namespace = "testonly.ohlc.tb02a." + case_id
            clock_events, messages = {}, []
            for index, bar in enumerate(bars):
                event_id = "op-" + str(index).zfill(4)
                base = origin_ns + index * 10_000_000_000
                attempts = [{"begin_ns": str(base + 100_000_000), "commit_ns": str(base + 200_000_000)}]
                if index == fault:
                    attempts.append({"begin_ns": str(base + 1_100_000_000), "commit_ns": str(base + 1_200_000_000)})
                    clock_events["recover-001"] = {"recover_ns": str(base + 1_000_000_000)}
                clock_events[event_id] = {"accept_ns": str(base), "attempts": attempts}
                event = {"event_id": bar["raw_id"], "revision": "2" if index == revision_index else "1",
                         "seq": str(bar["raw_index"]),
                         "received_at": (origin + datetime.timedelta(seconds=index * 10)).isoformat().replace("+00:00", "Z"),
                         "raw_text": json.dumps(bar, sort_keys=True, separators=(",", ":")),
                         **{field: str(bar[field]) for field in ("open", "high", "low", "close")},
                         "timestamp": str(bar["raw_index"] * 60), "volume": "1"}
                payload = {"op": "ingest", "target_session_id": sid, "target_session_generation": "1",
                           "writer_epoch": "2" if fault is not None and index > fault else "1",
                           "clock_event_id": event_id,
                           "raw_input": {"schema_revision": "s-ohlc/1", "session_id": sid,
                                         "source_namespace": namespace, "source_epoch": "1", "instrument": "TEST-OHLC",
                                         "profile": "ohlc_integer_tb02a_v1", "events": [event]}}
                messages.append({"schema_revision": "s-session/2", "session_id": sid, "session_generation": "1",
                                 "source_namespace": namespace, "source_epoch": "1", "message_id": "tb02a-" + event_id,
                                 "producer_id": "tb02a-input-driver", "producer_epoch": "1", "causal_refs": [],
                                 "payload_hash": hashlib.sha256(canonical(payload)).hexdigest(), "payload": payload})
            with (directory / "messages.jsonl").open("x") as stream:
                for message in messages:
                    stream.write(json.dumps(message, ensure_ascii=False, separators=(",", ":")) + "\n")
            dump(directory / "clock-plan.json", {"schema_revision": "s-clock-plan/1", "clock_plan_id": sid,
                                                  "origin_utc": "2000-01-01T00:00:00Z", "unit": "ns", "events": clock_events})
            port = port_base + len(configurations)
            if not 1024 <= port <= 65535:
                raise ValueError("端口不在本地测试有效范围")
            config = {"schema_revision": "s-launcher/2", "session_id": sid, "session_generation": "1",
                      "source_namespace": namespace, "source_epoch": "1", "producer_id": "tb02a-control",
                      "producer_epoch": "1", "writer_epoch": "1", "query_epoch": "1",
                      "delivery_retain_generations": "32", "port": str(port), "startup_timeout_ms": "10000",
                      "db": str(directory / "session.sqlite"), "socket": str(sock),
                      "binary": str(binary.resolve()), "python": str(python.resolve()),
                      "catalog": str(root / "catalog/signed-catalog.json"),
                      "profile": str(root / "profiles/ohlc_integer_tb02a_v1.json"),
                      "clock_plan": str(directory / "clock-plan.json"), "browser": str(root / "browser/index.html"),
                      "query_resource_config": str(root / "tests/fixtures/tb01c/query-resources.json"),
                      "s_resources": plan["s_resources"]}
            dump(directory / "config.json", config)
            command = [str(python.resolve()), str(root / "tests/tb02a_runtime.py"), "--config", str(directory / "config.json"),
                       "--messages", str(directory / "messages.jsonl"), "--output", str(directory / "evidence"),
                       "--node", str(node.resolve())]
            if fault is not None:
                command += ["--fault-index", str(fault), "--fault-stage", "after_batch"]
            configurations.append({"case_id": case_id, "arm": arm, "expected_case": expected_case,
                                   "input_count": len(messages), "fault_index": fault,
                                   "old_cut_before_revision": str(revision_index) if revision_index else None,
                                   "command": command, "directory": str(directory)})
    if ((fixtures / "raw-ledger.json").read_bytes() != ledger_bytes
            or (fixtures / "hand-oracle.json").read_bytes() != oracle_bytes):
        raise ValueError("包装期间冻结输入或手算原件发生变化")
    dump(output / "RUN-PLAN.json", {"schema": "tb02a-runtime-plan/1", "not_product_default": True,
                                     "fixture_kind": ledger["oracle_kind"],
                                     "source_ledger_sha256": hashlib.sha256(ledger_bytes).hexdigest(),
                                     "hand_oracle_sha256": hashlib.sha256(oracle_bytes).hexdigest(),
                                     "explicit_test_wrapper": {
                                         "timestamp": "raw_index * 60 seconds", "volume": "1",
                                         "instrument": "TEST-OHLC", "received_at": "2000-01-01T00:00:00Z + input_index * 10 seconds",
                                         "price_unit": "1", "original_ohlc": "raw-ledger.json unchanged",
                                         "raw_text": "complete original bar encoded as canonical JSON"},
                                     "runs": configurations})
    return {"planned_runs": len(configurations), "output": str(output)}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    for name in ("output", "sockets", "binary", "python", "node"):
        parser.add_argument("--" + name, type=Path, required=True)
    parser.add_argument("--port-base", type=int, required=True)
    print(json.dumps(prepare(**vars(parser.parse_args())), ensure_ascii=False))


if __name__ == "__main__":
    main()
