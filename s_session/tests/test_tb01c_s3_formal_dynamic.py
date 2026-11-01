"""#1768 正式 S3/Q3 消费臂。原 AuditReader 三动态组件测试单独保留。"""

import contextlib
import http.client
import json
import os
from pathlib import Path
import signal
import socket
import subprocess
import sys
from unittest.mock import patch

import pytest

import tb01c_query_tests as legacy
from r4_anchor_fixture import TestAnchor
import s_service_control as control
import s_socket_client as wire


PRICES = (10000, 11000, 10500, 12000, 11500, 13000, 12000)
SESSION = "tb01c-query-unit"
PROFILE = legacy.ROOT / "s_session/profiles/ohlc_integer_tb02a_v1.json"
CATALOG = legacy.ROOT / "s_session/catalog/signed-catalog.json"
SCOPE = {"structure": "CompleteCut", "economic": "not_started"}


def q3_resources():
    limits = {key: str(value) for key, value in legacy.resources().items()
              if key != "max_atomic_batch_bytes"}
    limits.update(max_atomic_delta_node_bytes="16777216", max_atomic_delta_nodes="100000",
                  max_atomic_delta_records="10000")
    return {"schema_revision": "s-query-resources/3", "limits": limits}


class FormalSession:
    """测试私有新域、逐代正式写收据和同一 Q 控制器的消费记录。"""

    def __init__(self, directory):
        self.root = directory
        self.db = directory / "formal.sqlite"
        self.state = directory / "state"
        self.state.mkdir()
        self.clock = directory / "clock.json"
        self.resources = directory / "q-resources.json"
        self.controller = None
        self.client = None
        self.anchor = None
        self.events = []
        self.serial = 0
        self.pid = None
        self.generation = 0

    def record(self, phase, **values):
        event = {"phase": phase, **values}
        self.events.append(event)
        print("FORMAL_S3_Q3 " + json.dumps(event, ensure_ascii=False, sort_keys=True), flush=True)

    def sidecars(self):
        return {suffix: {"exists": path.exists(),
                         "bytes": path.stat().st_size if path.exists() else None}
                for suffix in ("", "-wal", "-shm", "-journal")
                for path in (Path(str(self.db) + suffix),)}

    def command(self, phase, *args):
        binary = os.environ["S_STRUCTURE_SESSION_BINARY"]
        command = [binary, *map(str, args)]
        environment = dict(os.environ, **(self.anchor.env() if self.anchor else {}))
        self.record(phase + "-before", command=command, sidecars=self.sidecars(), q_pid=self.pid)
        result = subprocess.run(command, capture_output=True, text=True, timeout=60, env=environment)
        self.record(phase, exit=result.returncode, stdout=result.stdout, stderr=result.stderr,
                    sidecars=self.sidecars(), q_pid=self.pid)
        assert result.returncode == 0, f"{phase}: {result.stderr}"
        return wire.decode_frame(result.stdout.encode())

    def initialize(self):
        clock = {"schema_revision": "s-clock-plan/1", "clock_plan_id": "issue1768-formal-three-dynamic",
                 "origin_utc": "2000-01-01T00:00:00Z", "unit": "ns",
                 "events": {f"input-{seq}": {"accept_ns": str(seq * 10 + 1),
                            "attempts": [{"begin_ns": str(seq * 10 + 2),
                                          "commit_ns": str(seq * 10 + 3)}]}
                            for seq in range(len(PRICES))}}
        self.clock.write_bytes(wire.canonical(clock))
        resources = q3_resources()
        assert len(resources["limits"]) == 21
        self.resources.write_bytes(wire.canonical(resources))
        self.record("inputs", clock=clock, resources=resources, binary=os.environ["S_STRUCTURE_SESSION_BINARY"],
                    profile=json.loads(PROFILE.read_text()), db=str(self.db))
        initialized = self.command("init", "init", "--protocol-revision", "s-session/3",
                                   "--db", self.db, "--session", SESSION, "--catalog", CATALOG,
                                   "--profile", PROFILE, "--session-generation", "1",
                                   "--clock-plan", self.clock, "--delivery-retain-generations", "8")
        self.record("initialized", receipt=initialized)
        self.anchor = TestAnchor(self.db, self.state)
        self.record("testonly-anchor", limits_path=str(self.anchor.limits_path),
                    limits=json.loads(self.anchor.raw))
        for seq in range(4):
            self.commit(seq)
        with contextlib.closing(socket.socket()) as reservation:
            reservation.bind(("127.0.0.1", 0))
            self.port = reservation.getsockname()[1]
        self.config = {"schema_revision": "s-launcher/3", "db": str(self.db),
                       "binary": os.environ["S_STRUCTURE_SESSION_BINARY"], "python": sys.executable,
                       "session_id": SESSION, "session_generation": "1", "port": str(self.port),
                       "startup_timeout_ms": "2000", "query_resource_config": str(self.resources)}
        with patch.dict(os.environ, self.anchor.env()):
            self.controller = control.Controller(self.config, self.state)
        self.start_q()
        self.client = http.client.HTTPConnection("127.0.0.1", self.port, timeout=10)

    def envelope(self, payload):
        self.serial += 1
        return {"schema_revision": "s-session/3", "session_id": SESSION, "session_generation": "1",
                "source_namespace": "testonly.tick.ohlc", "source_epoch": "1",
                "producer_id": "issue1768-formal-consumer", "producer_epoch": "1",
                "message_id": f"formal-{self.serial}", "payload_hash": wire.digest(payload),
                "causal_refs": [], "payload": payload}

    def commit(self, seq):
        assert seq == self.generation
        price = str(PRICES[seq])
        event = {"event_id": f"e{seq}", "revision": "1", "seq": str(seq),
                 "received_at": "2000-01-01T00:00:00Z", "raw_text": price,
                 "timestamp": str(seq), "volume": "1", **dict.fromkeys(("open", "high", "low", "close"), price)}
        raw = {"schema_revision": "s-ohlc/1", "session_id": SESSION,
               "source_namespace": "testonly.tick.ohlc", "source_epoch": "1", "instrument": "TEST.TICK",
               "profile": "ohlc_integer_tb02a_v1", "events": [event]}
        request = self.envelope({"op": "ingest", "target_session_id": SESSION,
                                "target_session_generation": "1", "writer_epoch": "1",
                                "clock_event_id": f"input-{seq}", "raw_input": raw})
        path = self.root / f"input-{seq}.json"
        path.write_bytes(wire.canonical(request))
        self.record("one-to-one-crosswalk", old_profile="testonly_tick_1_1_ohlc", old_price=price,
                    old_event_id=f"e{seq}", old_revision="1", old_seq=str(seq),
                    old_timestamp=str(seq), old_source_namespace="testonly.tick.ohlc", old_source_epoch="1",
                    new_event=event, input_path=str(path), request=request)
        before = self.q_identity() if self.pid is not None else None
        result = self.command(f"advance-g{seq + 1}", "advance", "--protocol-revision", "s-session/3",
                              "--db", self.db, "--admission-job-limits", self.anchor.limits_path,
                              "--profile", PROFILE, "--writer-epoch", "1", "--clock-plan", self.clock,
                              "--input", path, "--max-input-bytes", "1048576")
        assert result["publication"]["generation"] == str(seq + 1), result
        self.generation += 1
        if before is not None:
            assert self.q_identity() == before
        self.record("published", generation=str(self.generation), receipt=result, q_identity=before)

    def start_q(self):
        ready = self.controller.start_q("1")
        record = self.controller.status("q")
        assert ready["state"] == "ready" and record["state"] == "running"
        self.pid = record["pid"]
        self.record("q-ready", readiness=ready, process=record, sidecars=self.sidecars())

    def q_identity(self):
        record = self.controller.status("q")
        assert record["state"] == "running" and record["pid"] == self.pid, record
        return {key: record[key] for key in ("pid", "process_identity", "control_instance_id")}

    def post(self, payload):
        before = self.q_identity()
        request = self.envelope(payload)
        self.client.request("POST", "/api/v3/" + payload["op"], wire.canonical(request),
                            {"Content-Type": "application/json"})
        response = self.client.getresponse()
        raw = response.read()
        self.record("q-" + payload["op"], status=response.status, request=request,
                    response=raw.decode(), process=before, sidecars=self.sidecars())
        assert response.status == 200, raw.decode()
        result = wire.validate_reply(request, wire.decode_frame(raw), producer="Q", producer_epoch="1")
        assert result["ok"] is True, result
        assert self.q_identity() == before
        return result

    def snapshot(self, generation="4", mode="AsKnown"):
        return self.post({"op": "snapshot", "session_id": SESSION, "session_generation": "1",
                          "scope": SCOPE, "history_mode": mode, "as_of_generation": generation,
                          "page_size": "2", "order_version": "s-record-order/2"})

    @staticmethod
    def cursor(page):
        fixed = page["fixed_cut"]
        return {"session_id": fixed["session_id"], "session_generation": fixed["session_generation"],
                "catalog_revision": fixed["catalog_revision"], "scope": fixed["scope"],
                "after_generation": fixed["cut_generation"], "base_cut": fixed["structure_cut"],
                "index_frontier": fixed["index_frontier"], "last_seq": fixed["input_frontier"],
                "order_version": page["order_version"]}

    def watch(self, cursor, client):
        return self.post({"op": "watch", "cursor": cursor, "max_batches": "2", "client_id": client})

    def restart_q(self):
        old = self.pid
        self.record("q-stop-after-cross-commit", result=self.controller.stop("q", signal.SIGTERM))
        self.start_q()
        assert self.pid != old

    def close(self):
        if self.client is not None:
            self.client.close()
        if self.controller is not None:
            try:
                self.record("q-cleanup", result=self.controller.stop("q", signal.SIGTERM))
            finally:
                self.controller.close()
        q_log = self.state / "q.log"
        if q_log.exists():
            self.record("q-log", text=q_log.read_text())
        self.record("final-sidecars", sidecars=self.sidecars())


@pytest.fixture
def formal(tmp_path, request):
    session = FormalSession(tmp_path)
    try:
        session.initialize()
        yield session
    finally:
        try:
            session.close()
        finally:
            request.node.user_properties.append(("formal_s3_q3_evidence", json.dumps(session.events)))


def test_formal_old_token_survives_live_q_commit_then_q_restart(formal):
    first = formal.snapshot()
    token = first["next_token"]
    assert token is not None
    original_q, original_client = formal.q_identity(), formal.client
    formal.commit(4)
    page = formal.post({"op": "snapshot", "snapshot_token": token})
    assert page["fixed_cut"]["cut_generation"] == "4"
    assert page["cut_projection_digest"] == first["cut_projection_digest"]
    assert page["validated_capture_digest"] != token["issued_capture_digest"]
    assert formal.q_identity() == original_q and formal.client is original_client
    formal.restart_q()
    assert formal.post({"op": "snapshot", "snapshot_token": token}) == page


def test_formal_fixed_cut_modes_after_live_q_commit(formal):
    first = formal.snapshot()
    token = first["next_token"]
    assert token is not None
    original_q, original_client = formal.q_identity(), formal.client
    formal.commit(4)
    page = formal.post({"op": "snapshot", "snapshot_token": token})
    assert page["fixed_cut"]["cut_generation"] == "4"
    assert page["cut_projection_digest"] == first["cut_projection_digest"]
    assert page["validated_capture_digest"] != token["issued_capture_digest"]
    known = formal.snapshot()
    recomputed = formal.snapshot(mode="RecomputedWithRevision")
    assert known["cut_projection_digest"] != recomputed["cut_projection_digest"]
    assert known["fixed_cut"]["as_of_generation"] == "4"
    assert recomputed["fixed_cut"]["as_of_generation"] is None
    assert formal.q_identity() == original_q and formal.client is original_client


def test_formal_stalled_backlog_is_local_to_live_q_client(formal):
    first = formal.snapshot()
    stalled = formal.cursor(first)
    assert formal.watch(stalled, "stalled")["gap"] is None
    zero = formal.cursor(formal.snapshot(generation="0"))
    fresh = formal.watch(zero, "fresh")
    assert fresh["gap"] is None and fresh["next_cursor"]["after_generation"] == "2"
    fresh = formal.watch(fresh["next_cursor"], "fresh")
    assert fresh["gap"] is None and fresh["next_cursor"]["after_generation"] == "4"
    original_q, original_client = formal.q_identity(), formal.client
    for seq in range(4, 7):
        formal.commit(seq)
        fresh = formal.watch(fresh["next_cursor"], "fresh")
        assert fresh["gap"] is None
        assert fresh["next_cursor"]["after_generation"] == str(seq + 1)
        assert fresh["delivery_frontier"]["first_available_generation"] == "1"
    result = formal.watch(stalled, "stalled")
    assert result["gap"]["reason"] == "client_backlog_overflow"
    assert result["gap"]["missing_range"] == {"from_generation": "5", "to_generation": "7"}
    assert formal.q_identity() == original_q and formal.client is original_client
