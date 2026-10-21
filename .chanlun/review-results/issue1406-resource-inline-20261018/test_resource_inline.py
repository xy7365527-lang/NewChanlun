"""#1406 resources inline纯替身；禁止产品、网络、真实进程和信号。"""
import ast
from contextlib import ExitStack
import hashlib
import importlib.util
import json
from pathlib import Path
import signal
import subprocess
import sys
import tempfile
from types import SimpleNamespace
import unittest
from unittest.mock import Mock, patch

ROOT = Path(__file__).resolve().parents[3]
SOURCE = ROOT / ".github/scripts/run-1406-c6a.py"
spec = importlib.util.spec_from_file_location("resource_inline_runner", SOURCE)
m = importlib.util.module_from_spec(spec)
sys.modules[spec.name] = m
spec.loader.exec_module(m)


def request():
    return {"groups": [123], "filesystem": "/tmp/probe-volume", "output": "/tmp/probe-output.json"}


class InlineCodecTests(unittest.TestCase):
    def test_roundtrip_canonical_schema_and_empty_groups(self):
        for groups in ([], [123]):
            value = {**request(), "groups": groups}
            raw = m.resource_inline_encode(value)
            self.assertEqual(m.resource_inline_decode(raw), value)
            self.assertLessEqual(len(raw.encode()), 16384)
            self.assertEqual(json.loads(raw)["schema"], "c6a-resource-probe/1")

    def test_invalid_schema_fields_domain_paths_and_noncanonical_json(self):
        original = json.loads(m.resource_inline_encode(request()))
        mutations = [lambda x: x.update(schema="wrong"), lambda x: x.update(extra=1),
                     lambda x: x.pop("output"), lambda x: x.update(groups=[True]),
                     lambda x: x.update(groups=[0]), lambda x: x.update(groups=[-1]),
                     lambda x: x.update(groups=[2**31]), lambda x: x.update(groups=[1, 2]),
                     lambda x: x.update(groups="1"), lambda x: x.update(output="relative"),
                     lambda x: x.update(output="/tmp/../escape"), lambda x: x.update(output="/tmp//double"),
                     lambda x: x.update(output="/tmp/nul\x00"), lambda x: x.update(output="/" + "x"*4096),
                     lambda x: x.update(filesystem="/" + "界"*1400)]
        for mutate in mutations:
            value = dict(original)
            mutate(value)
            raw = json.dumps(value, sort_keys=True, separators=(",", ":"), ensure_ascii=False)
            with self.subTest(value=value), self.assertRaises(RuntimeError):
                m.resource_inline_decode(raw)
        for raw in (" " + m.resource_inline_encode(request()), '{"schema":1,"schema":2}',
                    m.resource_inline_encode(request()).replace('"groups":[123]', '"groups":[NaN]')):
            with self.subTest(raw=raw), self.assertRaises((RuntimeError, ValueError)):
                m.resource_inline_decode(raw)

    def test_size_checked_before_parse_and_encode_rejects_expansion(self):
        with patch.object(m, "parse_json", side_effect=AssertionError("must reject before parsing")):
            for raw in ("x"*16385, "界"*6000):
                with self.subTest(raw_len=len(raw)), self.assertRaisesRegex(RuntimeError, "16 KiB"):
                    m.resource_inline_decode(raw)
        with self.assertRaisesRegex(RuntimeError, "16 KiB"):
            m.resource_inline_encode({"groups": [], "filesystem": "/" + "\t"*4095, "output": "/" + "\t"*4095})

    def test_inline_entry_runs_resource_worker_with_validated_request(self):
        raw = m.resource_inline_encode(request())
        sample = {"rss": 1, "free": 100, "available": 100, "live": False}
        with patch.object(sys, "argv", ["runner", "--resource-inline", raw]), \
             patch.object(sys, "platform", "linux"), patch.dict(m.os.environ, {"GITHUB_ACTIONS": "true"}), \
             patch.object(m, "snapshot", return_value=sample) as snapshot, patch.object(m, "save") as save:
            self.assertEqual(m.main(), 0)
            snapshot.assert_called_once_with({123}, "/tmp/probe-volume")
            save.assert_called_once_with("/tmp/probe-output.json", sample)

    def test_inline_entry_dispatch_and_illegal_mode_combinations(self):
        raw = m.resource_inline_encode(request())
        with patch.object(sys, "argv", ["runner", "--resource-inline", raw]), patch.object(m, "worker") as worker:
            self.assertEqual(m.main(), 0)
            worker.assert_called_once_with("resources", request())
        for args in (["--resource-inline", raw, "--run"], ["--resource-inline"], ["--resource-inline", "{}"]):
            with self.subTest(args=args), patch.object(sys, "argv", ["runner", *args]), \
                 patch.object(m, "worker") as worker, patch.object(sys, "stderr"):
                with self.assertRaises((SystemExit, RuntimeError)):
                    m.main()
                worker.assert_not_called()
        with patch.object(sys, "argv", ["runner", "--worker", "resources", "/tmp/untrusted"]), \
             patch.object(m, "small_json", side_effect=AssertionError("legacy file must not be read")):
            with self.assertRaisesRegex(RuntimeError, "requires inline"):
                m.main()


class ProbeTests(unittest.TestCase):
    def simulate(self, preparation=0., spawn_delay=.01, duration=.05, deadline=100., initial=0.):
        now, processes, events = [initial], {}, []
        real_encode = m.resource_inline_encode
        real_save = m.save
        with tempfile.TemporaryDirectory(prefix="inline-probe-") as tmp:
            supervisor = m.LinuxSupervisor(tmp, tmp, tmp, {}, 0.)
            def encode(value):
                events.append(("prepare-begin", now[0]))
                now[0] += preparation
                events.append(("prepare-end", now[0]))
                return real_encode(value)
            def popen(argv, **kwargs):
                events.append(("popen-begin", now[0]))
                self.assertEqual(argv[-2], "--resource-inline")
                value = m.resource_inline_decode(argv[-1])
                now[0] += spawn_delay
                process = SimpleNamespace(pid=91001, returncode=None, request=value,
                                          completes_at=now[0] + duration)
                processes[process.pid] = process
                events.append(("popen-end", now[0]))
                return process
            def observe(owned):
                p = owned.process
                if now[0] >= p.completes_at:
                    if owned.exited is None:
                        owned.exited = now[0]
                        real_save(p.request["output"], {"rss": 1, "free": 10**12, "available": 10**12, "live": False})
                    return SimpleNamespace(si_pid=p.pid)
                return None
            def reap(owned, limit, *, graceful):
                events.append(("mock-reap", now[0], limit, graceful))
                owned.process.returncode = 0 if graceful else -signal.SIGTERM
                supervisor.owned.pop(owned.process.pid)
                return {"complete": True, "forced": not graceful, "deadline": limit}
            def save(path, value):
                self.assertFalse(Path(path).name.startswith("request-"))
                return real_save(path, value)
            def sleep(duration):
                self.assertGreaterEqual(duration, 0)
                now[0] += duration
            with ExitStack() as stack:
                for obj, key, value in ((m, "clock", lambda: now[0]), (m, "resource_inline_encode", encode),
                        (m.subprocess, "Popen", popen), (m.time, "sleep", sleep), (m, "save", save),
                        (supervisor, "observe_exit", observe), (supervisor, "reap", reap)):
                    stack.enter_context(patch.object(obj, key, side_effect=value))
                for obj, key in ((supervisor, "worker_command"), (Path, "resolve"), (m.os, "killpg"), (m.os, "waitid")):
                    stack.enter_context(patch.object(obj, key, side_effect=AssertionError("forbidden " + key)))
                error = None
                try:
                    supervisor.probe([123], deadline)
                except RuntimeError as exc:
                    error = str(exc)
            names = [p.name for p in Path(tmp).iterdir()]
            self.assertFalse(any(name.startswith("request-") for name in names))
            return error, supervisor.receipts, events

    def test_unbounded_group_iterable_is_rejected_before_iteration(self):
        with tempfile.TemporaryDirectory() as tmp:
            supervisor = m.LinuxSupervisor(tmp, tmp, tmp, {}, 0.)
            groups = Mock()
            groups.__iter__ = Mock(side_effect=AssertionError("must not iterate"))
            with patch.object(m, "clock", return_value=0.), self.assertRaisesRegex(RuntimeError, "inline groups"):
                supervisor.probe(groups, 100.)
            groups.__iter__.assert_not_called()

    def test_slow_preparation_is_not_charged_to_probe_activity_and_no_request_io(self):
        error, receipts, events = self.simulate(preparation=.240089513, spawn_delay=.012)
        self.assertIsNone(error)
        self.assertAlmostEqual(receipts[0]["started"], .240089513)
        self.assertAlmostEqual(receipts[0]["deadline"], .240089513 + 6.5)
        self.assertLess(receipts[0]["finished"], receipts[0]["deadline"] - 6.25)
        self.assertTrue(receipts[0]["pass"])

    def test_slow_popen_and_probe_still_fail_without_rebasing(self):
        for kwargs in ({"spawn_delay": .3}, {"duration": .4}):
            with self.subTest(kwargs=kwargs):
                error, receipts, _ = self.simulate(**kwargs)
                self.assertIsNotNone(error)
                self.assertEqual(receipts[0]["deadline"], 6.5)
                self.assertEqual(receipts[0]["failure"], "phase active deadline")
                self.assertFalse(receipts[0]["pass"])

    def test_preparation_crossing_parent_active_end_never_spawns(self):
        error, receipts, events = self.simulate(preparation=.24, deadline=6.45)
        self.assertIn("preparation reached stage active deadline", error)
        self.assertEqual(receipts, [])
        self.assertFalse(any(row[0].startswith("popen") for row in events))
        error, receipts, events = self.simulate(initial=.201, deadline=6.45)
        self.assertIn("preparation reached stage active deadline", error)
        self.assertEqual(events, [])

    def test_tight_parent_D_caps_local_deadline_and_late_probe_fails(self):
        error, receipts, _ = self.simulate(preparation=.24, spawn_delay=.012, deadline=6.55)
        self.assertIsNotNone(error)
        self.assertEqual(receipts[0]["deadline"], 6.55)
        self.assertEqual(receipts[0]["failure"], "phase active deadline")

    def test_parent_clamped_fast_probe_passes_within_original_D(self):
        error, receipts, _ = self.simulate(preparation=.24, spawn_delay=.001, duration=.001, deadline=6.55)
        self.assertIsNone(error)
        self.assertEqual(receipts[0]["deadline"], 6.55)
        self.assertLess(receipts[0]["finished"], 6.30)


if __name__ == "__main__":
    unittest.main(verbosity=2)
