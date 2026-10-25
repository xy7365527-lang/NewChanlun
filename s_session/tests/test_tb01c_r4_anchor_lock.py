"""#1527 机械锁：测试专用造锚夹具只在测试可达；生产 CLI 对无锚库必拒且不自签。

主控裁定（#1527 issuecomment-6267327842）：既有服务测试可用 test_prepare 造锚，条件是
生产构建无此入口。Python 侧“生产构建”即 s_session/ 下非 tests 的模块；Rust 侧为 release 构建。
"""

import json
import os
from pathlib import Path
import re
import sqlite3
import subprocess
import sys
import tempfile
import unittest

SESSION = Path(__file__).resolve().parents[1]
REPO = SESSION.parent
FIXTURE_NAMES = ("r4_anchor_fixture", "anchored_controller", "test_prepare", "TestAnchor")


class AnchorLockTests(unittest.TestCase):
    def test_production_python_modules_cannot_reach_the_test_anchor_fixture(self):
        production = [path for path in SESSION.rglob("*.py") if "tests" not in path.relative_to(SESSION).parts]
        self.assertTrue(any(path.name == "s_service_control.py" for path in production))
        for path in production:
            text = path.read_text(encoding="utf-8")
            for name in FIXTURE_NAMES:
                self.assertNotIn(name, text, f"{path.relative_to(REPO)} 引用测试专用造锚夹具 {name}")

    def test_rust_test_prepare_exists_only_under_cfg_test(self):
        source = (REPO / "rust/src/bin/s_session_v3/admission_anchor.rs").read_text(encoding="utf-8")
        definitions = [m.start() for m in re.finditer(r"fn test_prepare\s*\(", source)]
        self.assertEqual(len(definitions), 1)
        preceding = source[:definitions[0]].rstrip().splitlines()[-1]
        self.assertTrue(preceding.startswith("#[cfg(test)]") or "#[cfg(test)]" in source[:definitions[0]].splitlines()[-2],
                        "Rust test_prepare 必须只在 #[cfg(test)] 下编译")
        for path in (REPO / "rust/src").rglob("*.rs"):
            text = path.read_text(encoding="utf-8")
            for match in re.finditer(r"test_prepare\(", text):
                if path.name == "admission_anchor.rs" and match.start() == definitions[0] + 3:
                    continue
                window = text[:match.start()]
                self.assertRegex(window, r"#\[cfg\((all\()?test", f"{path} 在非测试上下文调用 test_prepare")

    def test_production_cli_refuses_an_unanchored_database_without_self_signing(self):
        with tempfile.TemporaryDirectory(prefix="tb01c-r4-lock-") as directory:
            root = Path(directory).resolve()
            db = root / "s.sqlite"
            with sqlite3.connect(db) as connection:
                connection.execute("CREATE TABLE sentinel(value TEXT)")
            state = root / "state"
            state.mkdir()  # S 控制器只采用既存 state 目录
            marker = root / "binary-invoked"
            stub = root / "stub.py"
            stub.write_text("#!" + sys.executable + "\nimport pathlib\npathlib.Path(" + repr(str(marker))
                            + ").write_text('x')\nraise SystemExit(0)\n")
            stub.chmod(0o700)
            fixtures = SESSION / "tests/fixtures/tb01c"
            config = {"schema_revision": "s-launcher/3", "db": str(db), "socket": str(root / "s.sock"),
                      "binary": str(stub), "python": sys.executable, "catalog": str(root / "catalog"),
                      "profile": str(fixtures / "profile.json"), "clock_plan": str(fixtures / "clock-plan.json"),
                      "browser": str(root / "browser"), "query_resource_config": str(fixtures / "query-resources.json"),
                      "session_id": "lock", "session_generation": "1", "source_namespace": "lock",
                      "source_epoch": "1", "producer_id": "lock", "producer_epoch": "1", "writer_epoch": "1",
                      "query_epoch": "1", "port": "12345", "startup_timeout_ms": "10000",
                      "delivery_retain_generations": "1",
                      "s_resources": {key: "1000" for key in ("max_frame_bytes", "read_timeout_ms", "write_timeout_ms",
                                                              "response_timeout_ms", "audit_cache_source_bytes",
                                                              "queue_capacity", "max_connections")}}
            launcher = root / "launcher.json"
            launcher.write_text(json.dumps(config))
            environment = {key: value for key, value in os.environ.items() if not key.startswith("S_CONTROL_")}
            for action in ("start-s", "recover"):
                with self.subTest(action=action):
                    argv = [sys.executable, "-B", str(SESSION / "s_service_control.py"), action,
                            "--config", str(launcher), "--state-dir", str(state)]
                    if action == "recover":
                        argv += ["--new-epoch", "2", "--clock-event-id", "lock-1"]
                    result = subprocess.run(argv, capture_output=True, env=environment, timeout=60)
                    self.assertEqual(result.returncode, 1, result.stderr)
                    error = json.loads(result.stderr)
                    self.assertEqual(error["code"], "AnchorMissing", error)
                    self.assertFalse(Path(str(db) + ".control-owner.json").exists())
                    self.assertFalse((state / "s-admission").exists())
                    self.assertFalse((state / ".s-admission-pending").exists())
                    self.assertFalse(marker.exists(), "无锚时不得调用 Rust 写入口")


if __name__ == "__main__":
    unittest.main()
