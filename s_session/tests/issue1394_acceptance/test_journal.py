from dataclasses import replace
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import time
import unittest
from unittest.mock import patch
import journal as j
import scheduler as s
import test_scheduler as simulation
from driver import Rejected


def small_plan():
    plan = simulation.fixture()
    now = time.monotonic()
    return replace(plan, steps=tuple(replace(step, deadline=now + 10) for step in plan.steps[:2]),
                   work_deadline=now + 10, cleanup_deadline=now + 12, seal_deadline=now + 14)


class JournalTests(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory(dir="/tmp", prefix="j1394-")
        self.path = Path(self.tmp.name) / "journal"
        self.plan = small_plan()
        self.journal = j.Journal.create(self.path, self.plan, max_bytes=100000, max_records=20)

    def tearDown(self):
        self.journal.close()
        self.tmp.cleanup()

    def test_cross_process_consume_only_one_wins(self):
        # 两个短命独立解释器争同一 inode；不是两个内存对象冒充跨进程。
        args = json.dumps({"identity": self.journal.identity, "header": self.journal.header,
                           "step": self.plan.steps[0].__dict__}, default=lambda x: x.__dict__)
        code = """
import json,sys
from journal import Journal
from scheduler import Step, Binding
x=json.loads(sys.argv[2]); z=x['step'];z['binding']=Binding(**z['binding']);z['argv']=tuple(z['argv']);z['requests']=();z['expected_proofs']=tuple(z['expected_proofs'])
with Journal(sys.argv[1],x['identity'],x['header'],100000,20) as journal:
 try: journal.consume(Step(**z),x['header']['seed']);print('CONSUMED')
 except ValueError: print('REFUSED')
"""
        children = []
        try:
            for _ in range(2):
                children.append(subprocess.Popen([sys.executable, "-B", "-c", code, str(self.path), args],
                                                  cwd=Path(__file__).parent, stdout=subprocess.PIPE,
                                                  stderr=subprocess.PIPE, env={}))
            results = [p.communicate(timeout=5) for p in children]
            self.assertTrue(all(p.returncode == 0 for p in children), results)
            self.assertEqual(sorted(out.strip() for out, _ in results), [b"CONSUMED", b"REFUSED"])
        finally:
            for process in children:
                if process.poll() is None:
                    process.kill()
                process.wait(timeout=2)

    def test_failed_stage_stays_consumed_after_reopen(self):
        step = self.plan.steps[0]
        self.journal.consume(step, self.plan.seed_sha)
        with j.Journal(self.path, self.journal.identity, self.journal.header, 100000, 20) as other:
            with self.assertRaises(Rejected):
                other.consume(step, self.plan.seed_sha)

    def test_reopen_cannot_expand_frozen_caps(self):
        with self.assertRaises(Rejected):
            j.Journal(self.path,self.journal.identity,self.journal.header,200000,20)

    def test_predecessor_order_finish_and_receipt_binding(self):
        a, b = self.plan.steps
        with self.assertRaises(Rejected):
            self.journal.consume(b, self.plan.seed_sha)
        self.journal.consume(a, self.plan.seed_sha)
        receipt = s.StageReceipt(0, a.stage, a.scenario, a.sha(), self.plan.seed_sha, "a"*64, "CONTROL_ONLY")
        with self.assertRaises(Rejected):
            self.journal.finish(a, replace(receipt, predecessor_sha="b"*64))
        self.journal.finish(a, receipt)
        with self.assertRaises(Rejected):
            self.journal.consume(b, self.plan.seed_sha)
        self.journal.consume(b, receipt.sha())

    def test_corrupt_tail_and_replaced_inode_fail_closed(self):
        with self.path.open("ab") as out:
            out.write(b'{"partial":')
        with self.assertRaises(Rejected):
            self.journal.consume(self.plan.steps[0], self.plan.seed_sha)
        self.path.rename(self.path.with_suffix(".old"))
        self.path.write_bytes(b"foreign\n")
        with self.assertRaises(Rejected):
            self.journal.consume(self.plan.steps[0], self.plan.seed_sha)
        self.assertEqual(self.path.read_bytes(), b"foreign\n")

    def test_fsync_failure_does_not_allow_retry(self):
        with patch.object(j.os, "fsync", side_effect=OSError("toy fsync failure")):
            with self.assertRaises(OSError):
                self.journal.consume(self.plan.steps[0], self.plan.seed_sha)
        with self.assertRaises(Rejected):
            self.journal.consume(self.plan.steps[0], self.plan.seed_sha)

    def test_mode_change_cannot_admit_control_or_duck_adapter(self):
        adapter = type("NotSimulation", (), {"mode": "UNIT_TEST_ONLY", "active_owner_keys": lambda _: (),
                    "cleanup_all": lambda _, reason: (), "seal": lambda *args: None,
                    "perform": lambda *args: self.fail("不应调用真实桥")})()
        result = s.Scheduler(simulation.fixture(), adapter, simulation.Clock()).run()
        self.assertEqual(result.attempted, ())
        self.assertIn("mode", result.error)
