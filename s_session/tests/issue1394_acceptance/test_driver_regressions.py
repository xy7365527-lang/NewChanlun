"""独审 F1/F2：真实文件改写与真实 gate；只用临时 toy。"""
from pathlib import Path
import unittest
from unittest.mock import patch
import driver as d
import test_driver as baseline


class OwnershipRegressions(unittest.TestCase):
    setUp = baseline.AdapterTests.setUp
    tearDown = baseline.AdapterTests.tearDown
    toy_admission = baseline.AdapterTests.toy_admission

    def prepare(self):
        program = self.root / "toy.sh"
        marker = self.root / "MUST-NOT-EXEC"
        program.write_text("#!/bin/sh\n: > " + str(marker) + "\n")
        program.chmod(0o700)
        argv = [str(program)]
        admission, _ = self.toy_admission(argv, "fake")
        supervisor = d.Supervisor("fake", admission.contract_sha, admission.business_deadline, 8192)
        return program, marker, argv, admission, supervisor

    def start(self, argv, admission, supervisor, output):
        return d.OwnedProcess(argv, output, supervisor, "toy-only", {"scope": "UNIT_TEST_ONLY"},
                              admission.business_deadline, admission=admission)

    def test_real_rewrite_between_admission_and_constructor_rejected_before_spawn(self):
        program, marker, argv, adm, supervisor = self.prepare()
        original_hash = d.bounded_hash
        calls = []
        def read_then_change(path, deadline):
            observed = original_hash(path, deadline)
            calls.append(observed)
            if len(calls) == 1:
                program.write_text(program.read_text() + "# 真正变更字节\n")
            return observed
        with patch.object(d, "bounded_hash", side_effect=read_then_change), \
             patch.object(d.subprocess, "Popen") as popen:
            with self.assertRaises(d.Rejected):
                self.start(argv, adm, supervisor, self.root / "owner")
            popen.assert_not_called()
        self.assertNotEqual(calls[0], calls[1])
        self.assertFalse(marker.exists())
        self.assertEqual(adm.consumed, {("toy-only", "E", "1")})
        with self.assertRaises(d.Rejected):
            adm.check_command(argv, "toy-only", {"scope": "UNIT_TEST_ONLY"}, adm.business_deadline)

    def test_real_rewrite_after_ack_never_releases_gate_and_keeps_frozen_sha(self):
        program, marker, argv, adm, supervisor = self.prepare()
        frozen = d.sha(program.read_bytes())
        save = d.save_new
        def alter(path, value, **kwargs):
            save(path, value, **kwargs)
            if Path(path).name == "ACK.json":
                program.write_text(program.read_text() + "# ACK 后改写\n")
        with patch.object(supervisor, "ack", return_value={"toy": True}), \
             patch.object(d, "save_new", side_effect=alter), \
             patch.object(d, "process_identity", side_effect=lambda p, deadline: {"pid": p.pid, "start_identity": "UNIT_TEST_ONLY", "process_group": p.pid}):
            with self.assertRaises(d.Rejected):
                self.start(argv, adm, supervisor, self.root / "owner")
        owner = next(iter(adm.active_owners.values()))
        self.assertFalse(owner.released)
        self.assertTrue(owner.cleanup_result["reaped"])
        self.assertEqual(owner.binary_before, frozen)
        self.assertTrue(owner.cleanup_result["errors"])
        self.assertFalse(marker.exists())
        self.assertTrue((owner.output / "CLEANUP.json").exists())
        self.assertEqual(adm.consumed, {("toy-only", "E", "1")})

    def test_existing_directory_file_and_symlink_never_receive_cleanup(self):
        for kind in ("empty", "sentinel", "file", "symlink"):
            with self.subTest(kind=kind):
                outer = self.root
                self.root = outer / kind
                self.root.mkdir()
                try:
                    _, marker, argv, adm, sup = self.prepare()
                    output = self.root / "owner"
                    if kind == "file":
                        output.write_bytes(b"ORIGINAL")
                    elif kind == "symlink":
                        target = self.root / "target"
                        target.mkdir()
                        output.symlink_to(target, target_is_directory=True)
                    else:
                        output.mkdir()
                        if kind == "sentinel":
                            (output / "SENTINEL").write_bytes(b"ORIGINAL")
                    before = output.read_bytes() if kind == "file" else sorted(p.name for p in output.iterdir())
                    with patch.object(d.subprocess, "Popen") as popen:
                        with self.assertRaises(FileExistsError) as caught:
                            self.start(argv, adm, sup, output)
                        popen.assert_not_called()
                    after = output.read_bytes() if kind == "file" else sorted(p.name for p in output.iterdir())
                    self.assertEqual(before, after)
                    self.assertFalse(marker.exists())
                    self.assertFalse(caught.exception.cleanup_receipt["reaped"])
                finally:
                    self.root = outer

    def test_replaced_path_is_not_written_gate_is_reaped_error_returned(self):
        _, marker, argv, adm, sup = self.prepare()
        output, moved = self.root / "owner", self.root / "moved"
        def replace(*args):
            output.rename(moved)
            output.mkdir()
            (output / "SENTINEL").write_bytes(b"ORIGINAL")
            return {"toy": True}
        with patch.object(sup, "ack", side_effect=replace), \
             patch.object(d, "process_identity", side_effect=lambda p, deadline: {"pid": p.pid, "start_identity": "UNIT_TEST_ONLY", "process_group": p.pid}):
            with self.assertRaises(d.Rejected) as caught:
                self.start(argv, adm, sup, output)
        self.assertEqual([p.name for p in output.iterdir()], ["SENTINEL"])
        self.assertFalse((moved / "CLEANUP.json").exists())
        self.assertTrue(caught.exception.cleanup_receipt["reaped"])
        self.assertTrue(caught.exception.cleanup_receipt["errors"])
        self.assertFalse(marker.exists())

    def test_ps_failure_after_owned_directory_leaves_cleanup(self):
        _, marker, argv, adm, sup = self.prepare()
        output = self.root / "owner"
        with patch.object(d, "process_identity", side_effect=OSError("toy ps failure")):
            with self.assertRaises(OSError):
                self.start(argv, adm, sup, output)
        receipt = d.strict((output / "CLEANUP.json").read_bytes())
        self.assertTrue(receipt["reaped"])
        self.assertFalse(marker.exists())
