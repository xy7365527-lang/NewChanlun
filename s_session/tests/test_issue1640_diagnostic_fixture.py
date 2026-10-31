"""#1640 1a（gates policy1640-diagnostic）：真实 launcher/4 对 TestOnly 诊断场所写者的 C4 拒绝回执。

只在给出真实写者二进制时执行，没有即跳过，不以假写者冒充：
  ISSUE1640_TESTONLY_BINARY     `s_session,s_capture_native_testonly` 构建；
  ISSUE1640_PRODUCTION_BINARY   `s_session,s_incremental` 生产增量构建（零变化负控）。
每例都走 Controller.start_s 的原路径（R4 锚、运行时探测、资源冻结、socket、子进程、Ready 核验、拒绝善后），
不打桩 launcher 判据，只在 check_build_profile 外包一层记录以留下 Ready payload 原件：
  fixture     S_DIAGNOSTIC_FIXTURE_TESTONLY=venue-profile-diagnostic-fixture ⟹ testonly 写者到 Ready 并自报
              build_profile=testonly、venue_profile=venue-profile-diagnostic-fixture/1、admissible=false，
              launcher/4 在登记与 confirm_ready_process 之前以 TestOnlyBuildError 拒绝并停掉子进程。
  unset       未设开关 ⟹ 同一 testonly 二进制走正式 P0 路径，在 Ready 前因 AwaitingPolicy 退出（入口是 opt-in）。
  invalid     开关取值不是夹具 id ⟹ testonly 写者拒绝启动（DiagnosticFixtureTestOnlyInvalid），不静默忽略。
  production  生产增量二进制带同一开关 ⟹ 不读不生效，仍在 Ready 前因 AwaitingPolicy 退出，日志无夹具自报。
回执落 ISSUE1640_EVIDENCE_DIR（可选）。诊断回执不是生产验收，不计 Destination（#1602:22、#799 限定词②）。
"""

import copy
import json
import os
import sqlite3
import subprocess
import sys
import tempfile
import time
import unittest
from pathlib import Path
from unittest.mock import patch

SESSION = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(SESSION))
sys.path.insert(0, str(SESSION / "tests"))
import s_service_control as control  # noqa: E402
from r4_anchor_fixture import anchored_controller  # noqa: E402

TESTONLY_BINARY = os.environ.get("ISSUE1640_TESTONLY_BINARY")
PRODUCTION_BINARY = os.environ.get("ISSUE1640_PRODUCTION_BINARY")
EVIDENCE = os.environ.get("ISSUE1640_EVIDENCE_DIR")
MARKER = "S_DIAGNOSTIC_FIXTURE_TESTONLY"
FIXTURE_ID = "venue-profile-diagnostic-fixture"
SESSION_ID = "issue1640-diagnostic-39g"
# 与 rust/tests/production_session_entry.rs 的 TestOnly 锚同值；启动审计不得因预算过小失败。
LIMITS = {"max_total_wire_bytes": "1073741824", "max_total_node_bytes": "1073741824",
          "max_total_nodes": "10000000", "max_total_records": "10000000", "max_total_requests": "1000000",
          "max_elapsed_ms": "600000", "max_staged_bytes": "67108864", "max_store_bytes": "1073741824"}
RESOURCES = {"max_frame_bytes": "65536", "read_timeout_ms": "5000", "write_timeout_ms": "5000",
             "response_timeout_ms": "10000", "audit_cache_source_bytes": "1048576", "queue_capacity": "8",
             "max_connections": "4", "incremental_retention_bytes": "268435456"}
CLOCK_PLAN = {"schema_revision": "s-clock-plan/1", "clock_plan_id": "issue1640-diag", "origin_utc": "2000-01-01T00:00:00Z",
              "unit": "ns", "events": {"input-1": {"accept_ns": "10", "attempts": [{"begin_ns": "11", "commit_ns": "12"}]}}}


def raw_rows(db):
    with sqlite3.connect(f"file:{db}?mode=ro&immutable=1", uri=True) as connection:
        return connection.execute("SELECT COUNT(*) FROM raw_events").fetchone()[0]


class Launcher4Drive(unittest.TestCase):
    def init_db(self, binary, root):
        db_dir = root / "s"
        db_dir.mkdir()
        db = db_dir / "s.sqlite"
        clock = root / "clock.json"
        clock.write_text(json.dumps(CLOCK_PLAN))
        command = [binary, "init", "--protocol-revision", "s-session/3", "--db", str(db),
                   "--session", SESSION_ID, "--catalog", str(SESSION / "catalog/signed-catalog.json"),
                   "--profile", str(SESSION / "profiles/ohlc_integer_tb02a_v1.json"),
                   "--session-generation", "1", "--clock-plan", str(clock), "--delivery-retain-generations", "4"]
        result = subprocess.run(command, capture_output=True, text=True, timeout=120)
        self.assertEqual(result.returncode, 0, result.stderr)
        return db, clock

    def config(self, binary, root, db, clock):
        return {"schema_revision": "s-launcher/4", "db": str(db), "socket": str(root / "s.sock"),
                "binary": binary, "python": sys.executable, "catalog": str(SESSION / "catalog/signed-catalog.json"),
                "profile": str(SESSION / "profiles/ohlc_integer_tb02a_v1.json"), "clock_plan": str(clock),
                "browser": str(root / "browser"), "query_resource_config": str(root / "q.json"),
                "session_id": SESSION_ID, "session_generation": "1", "source_namespace": "issue1640.diag",
                "source_epoch": "1", "producer_id": "issue1640-launcher", "producer_epoch": "1",
                "writer_epoch": "1", "query_epoch": "1", "port": "12345", "delivery_retain_generations": "4",
                "startup_timeout_ms": "60000", "s_resources": dict(RESOURCES)}

    def drive(self, binary, case, marker_value):
        """一次真实 start_s；返回 launcher 的异常、Ready 核验前看到的 payload、子进程日志与库行数。"""
        with tempfile.TemporaryDirectory(prefix="i1640-") as directory:
            root = Path(directory).resolve()
            db, clock = self.init_db(binary, root)
            config = self.config(binary, root, db, clock)
            state = root / "state"
            environment = {MARKER: marker_value} if marker_value is not None else {}
            payloads = []
            original = control.check_build_profile

            def recording(launcher_config, payload):
                payloads.append(copy.deepcopy(payload))
                return original(launcher_config, payload)

            with patch.dict(os.environ, environment):
                if marker_value is None:
                    os.environ.pop(MARKER, None)
                controller = anchored_controller(config, state, limits=LIMITS)
                started = time.monotonic()
                try:
                    with patch.object(control, "check_build_profile", side_effect=recording):
                        try:
                            controller.start_s("1")
                            error = None
                        except ValueError as exc:
                            error = exc
                    status = controller.status("s")
                    record_exists = controller.record_path("s").exists()
                    log = (controller.state_dir / "s.log").read_text(errors="replace")
                finally:
                    controller.close()
            outcome = {"case": case, "marker": {MARKER: marker_value}, "schema_revision": config["schema_revision"],
                       "s_resources_fields": sorted(config["s_resources"]), "binary": binary,
                       "elapsed_s": round(time.monotonic() - started, 3),
                       "error_type": type(error).__name__ if error else None,
                       "error": str(error) if error else None, "ready_payloads": payloads,
                       "status_after": status, "record_exists_after": record_exists,
                       "socket_exists_after": os.path.lexists(config["socket"]), "raw_events": raw_rows(db),
                       "s_log": log}
            if EVIDENCE:
                target = Path(EVIDENCE)
                target.mkdir(parents=True, exist_ok=True)
                (target / f"launcher4-{case}.json").write_text(json.dumps(outcome, ensure_ascii=False, indent=2))
            return error, outcome



@unittest.skipUnless(TESTONLY_BINARY, "ISSUE1640_TESTONLY_BINARY 未给出真实 testonly 写者二进制")
class Launcher4DiagnosticFixtureTests(Launcher4Drive):
    def test_fixture_writer_reaches_ready_and_launcher4_rejects_testonly_before_registration(self):
        error, outcome = self.drive(TESTONLY_BINARY, "fixture", FIXTURE_ID)
        self.assertIsInstance(error, control.TestOnlyBuildError, outcome["s_log"])
        self.assertEqual(len(outcome["ready_payloads"]), 1, outcome)
        payload = outcome["ready_payloads"][0]
        self.assertEqual(payload["kind"], "Ready")
        self.assertEqual(payload["build_profile"], "testonly")
        self.assertEqual(payload["venue_profile"], FIXTURE_ID + "/1")
        self.assertIs(payload["admissible"], False)
        diagnostics = [json.loads(line) for line in outcome["s_log"].splitlines()
                       if line.startswith("{") and '"DiagnosticFixtureTestOnly"' in line]
        self.assertEqual(len(diagnostics), 1, outcome["s_log"])
        self.assertEqual(diagnostics[0], {"diagnostic": "DiagnosticFixtureTestOnly", "build_profile": "testonly",
                                          "venue_profile": FIXTURE_ID + "/1", "admissible": False,
                                          "require_admissible": True})
        # 拒绝在登记之前：无 record、状态回到 not_started、socket 已撤、库未推进。
        self.assertFalse(outcome["record_exists_after"])
        self.assertEqual(outcome["status_after"]["state"], "not_started")
        self.assertFalse(outcome["socket_exists_after"])
        self.assertEqual(outcome["raw_events"], 0)

    def test_same_binary_without_switch_exits_on_awaiting_policy_before_ready(self):
        error, outcome = self.drive(TESTONLY_BINARY, "unset", None)
        self.assertIsInstance(error, ValueError, outcome["s_log"])
        self.assertNotIsInstance(error, control.TestOnlyBuildError)
        self.assertEqual(outcome["ready_payloads"], [])
        self.assertIn("AwaitingPolicy", outcome["s_log"])
        self.assertIn("venue_profile", outcome["s_log"])
        self.assertEqual(outcome["raw_events"], 0)

    def test_switch_with_non_fixture_value_refuses_startup(self):
        error, outcome = self.drive(TESTONLY_BINARY, "invalid", "1")
        self.assertIsInstance(error, ValueError, outcome["s_log"])
        self.assertNotIsInstance(error, control.TestOnlyBuildError)
        self.assertEqual(outcome["ready_payloads"], [])
        self.assertIn("DiagnosticFixtureTestOnlyInvalid", outcome["s_log"])
        self.assertEqual(outcome["raw_events"], 0)


@unittest.skipUnless(PRODUCTION_BINARY, "ISSUE1640_PRODUCTION_BINARY 未给出真实生产增量写者二进制")
class Launcher4ProductionBinaryIgnoresSwitchTests(Launcher4Drive):
    def test_production_binary_with_switch_still_exits_on_awaiting_policy_without_fixture_report(self):
        error, outcome = self.drive(PRODUCTION_BINARY, "production", FIXTURE_ID)
        self.assertIsInstance(error, ValueError, outcome["s_log"])
        self.assertNotIsInstance(error, control.TestOnlyBuildError)
        self.assertEqual(outcome["ready_payloads"], [])
        self.assertIn("AwaitingPolicy", outcome["s_log"])
        self.assertNotIn("DiagnosticFixtureTestOnly", outcome["s_log"])
        self.assertNotIn(FIXTURE_ID, outcome["s_log"])
        self.assertEqual(outcome["raw_events"], 0)


if __name__ == "__main__":
    unittest.main()
