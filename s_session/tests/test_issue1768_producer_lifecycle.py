"""#1768 组件臂 FrozenProducer 生命周期负控：替身子进程只扮演握手协议，不启动 S/Q、不写库。"""
import json
import os
import stat
import sys
import tempfile
import unittest
from pathlib import Path
from types import SimpleNamespace
from unittest.mock import patch

import tb01c_query_tests as base

FAKE = r'''#!{python}
import json, os, sys, time
from pathlib import Path
d, mode = Path(os.environ["S1768_PRODUCER_HANDSHAKE"]), os.environ["S1768_FAKE_MODE"]
def publish(name, value):
    (d / f"{name}.json.tmp").write_text(json.dumps(value)); (d / f"{name}.json.tmp").rename(d / f"{name}.json")
if mode == "ready-exit": sys.exit(3)
if mode == "ready-hang":
    while True: time.sleep(1)
publish("ready", {"producer_pid": os.getpid()})
while True:
    if (d / "request-0.json").exists():
        publish("receipt-0", {"ok": False, "error": "E6"}); sys.exit(101)
    if (d / "stop.json").exists():
        if mode == "stop-hang":
            while True: time.sleep(1)
        publish("stopped", {"requests": 0})
        sys.exit(7 if mode == "stop-nonzero" else 0)
    time.sleep(0.05)
'''


class ProducerLifecycleTests(unittest.TestCase):
    def setUp(self):
        directory = tempfile.TemporaryDirectory(prefix="producer-lifecycle-")
        self.addCleanup(directory.cleanup)
        self.base = Path(directory.name)
        script = self.base / "fake-producer"
        script.write_text(FAKE.replace("{python}", sys.executable), encoding="utf-8")
        script.chmod(script.stat().st_mode | stat.S_IXUSR)
        self.cleanups = []
        self.host = SimpleNamespace(base=self.base, id=lambda: "lifecycle.case",
                                    reader=SimpleNamespace(cached={"generation": "4"}, connection=None, cached_version=None),
                                    addCleanup=self.cleanups.append)
        patches = [patch.object(base, "PRODUCER", str(script)), patch.object(base, "PRODUCER_EVIDENCE", None),
                   patch.object(base.FrozenProducer, "READY_TIMEOUT", 2),
                   patch.object(base.FrozenProducer, "STOP_TIMEOUT", 1)]
        for p in patches:
            p.start()
            self.addCleanup(p.stop)

    def producer(self, mode):
        with patch.dict(os.environ, S1768_FAKE_MODE=mode):
            producer = base.FrozenProducer(self.host, self.base / "db.sqlite")
        self.assertEqual(self.cleanups, [producer.finish], "cleanup 在第一次阻塞等待前已归属")
        return producer

    def cleanup(self):
        for fn in reversed(self.cleanups):
            fn()

    def events(self, producer):
        return [r["event"] for r in json.loads((producer.log / "records.json").read_text(encoding="utf-8"))["records"]]

    def assertReaped(self, producer, code=None):
        self.assertIsNotNone(producer.proc.poll(), "子进程已回收")
        if code is not None:
            self.assertEqual(producer.proc.returncode, code)
        self.assertTrue(producer.out.closed and producer.err.closed, "日志句柄已关闭")

    def test_normal_stop_passes(self):
        producer = self.producer("normal").start()
        self.cleanup()
        self.assertReaped(producer, 0)
        self.assertEqual(self.events(producer), ["ready", "stop"])
        producer.finish()

    def test_ready_exit_keeps_primary_and_reaps(self):
        producer = self.producer("ready-exit")
        with self.assertRaisesRegex(AssertionError, "ready 前退出 exit=3"):
            producer.start()
        self.cleanup()
        self.assertReaped(producer, 3)
        self.assertEqual(self.events(producer), ["ready-failed", "exited"])

    def test_ready_timeout_kills_and_keeps_primary(self):
        producer = self.producer("ready-hang")
        with self.assertRaisesRegex(AssertionError, "等待 ready 超时"):
            producer.start()
        self.cleanup()
        self.assertReaped(producer)
        self.assertEqual(self.events(producer), ["ready-failed", "stop"])

    def test_stop_timeout_fails_cleanup(self):
        producer = self.producer("stop-hang").start()
        with self.assertRaisesRegex(AssertionError, "producer 未正常停止"):
            self.cleanup()
        self.assertReaped(producer)

    def test_nonzero_exit_after_stop_fails_cleanup(self):
        producer = self.producer("stop-nonzero").start()
        with self.assertRaisesRegex(AssertionError, "producer 退出码 7"):
            self.cleanup()
        self.assertReaped(producer, 7)

    def test_request_failure_stays_primary_cause(self):
        producer = self.producer("normal").start()
        with self.assertRaisesRegex(AssertionError, '"error": "E6"'):
            producer.request(5, 11500)
        self.cleanup()
        self.assertReaped(producer, 101)
        self.assertEqual(self.events(producer), ["ready", "request-0", "exited"])


if __name__ == "__main__":
    unittest.main()
