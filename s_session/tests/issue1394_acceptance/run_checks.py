"""本目录定向验证；--offline 显式列出宿主沙箱阻断的三项原测试，不伪报全量通过。"""
import argparse
import json
from pathlib import Path
import unittest


BLOCKED_HERE = {
    "test_driver.AdapterTests.test_owned_toy_marker_sigkill_and_reap": "沙箱禁止 Unix socket bind 与 /bin/ps",
    "test_driver.AdapterTests.test_supervisor_real_unix_ack_and_mismatch": "沙箱禁止 Unix socket bind 与 /bin/ps",
    "test_driver.AdapterTests.test_real_gate_ack_loss_and_late_disk_reap_without_release": "沙箱禁止 /bin/ps",
}


def flatten(suite):
    for test in suite:
        if isinstance(test, unittest.TestSuite):
            yield from flatten(test)
        else:
            yield test


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--offline", action="store_true")
    args = parser.parse_args()
    tests = list(flatten(unittest.defaultTestLoader.discover(str(Path(__file__).parent))))
    excluded = {t.id(): BLOCKED_HERE[t.id()] for t in tests if args.offline and t.id() in BLOCKED_HERE}
    selected = [t for t in tests if t.id() not in excluded]
    result = unittest.TextTestRunner(verbosity=2).run(unittest.TestSuite(selected))
    print("CHECK_RESULT_JSON=" + json.dumps({"scope": "OFFLINE_SUBSET" if args.offline else "ALL_TARGETED",
          "discovered": len(tests), "run": result.testsRun, "success": result.wasSuccessful(),
          "errors": [t.id() for t, _ in result.errors], "failures": [t.id() for t, _ in result.failures],
          "excluded": excluded, "execution_authorized": False, "product_status": "NOT_RUN"}, ensure_ascii=False))
    raise SystemExit(0 if result.wasSuccessful() else 1)
