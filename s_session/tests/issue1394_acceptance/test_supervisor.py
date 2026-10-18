from dataclasses import replace, asdict
import os
from pathlib import Path
import signal
import subprocess
import sys
import tempfile
import time
import unittest
import supervisor as s
from driver import sha, Rejected


class Probe:
    def __init__(self, worker):
        self.identities = {worker.pid: worker}
        self.dead, self.signals, self.waits = set(), [], []
        self.desc = set()
        self.readings = {"rss_bytes": 10, "disk_bytes": 10, "output_bytes": 0}
        self.term_exits = True

    def identity(self, pid): return self.identities[pid]
    def alive(self, identity): return identity.pid not in self.dead
    def descendants(self, worker): return self.desc - self.dead
    def sample(self, identities): return self.readings
    def signal(self, identity, signum):
        self.signals.append((identity.pid, signum))
        if signum == signal.SIGKILL or self.term_exits: self.dead.add(identity.pid)
    def wait(self, identity, deadline):
        self.waits.append(identity.pid)
        return identity.pid in self.dead


class SupervisorTests(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory(dir="/tmp", prefix="s1394-")
        self.script = Path(self.tmp.name) / "toy-script"
        self.script.write_bytes(b"UNIT_TEST_ONLY")
        self.now = time.monotonic()
        self.clock = lambda: self.now
        self.worker = s.Identity(100, "toy-worker", 100, 99)
        self.owner = s.Identity(101, "toy-owner", 101, 100)
        self.probe = Probe(self.worker)
        self.probe.identities[101] = self.owner
        self.probe.desc.add(101)
        self.limits = s.Limits(self.now+10,self.now+12,self.now+14,.1,1,.1,.2,4,100,100,100)
        self.stages = {"toy": {"resource_caps": {"scope":"UNIT_TEST_ONLY"}, "deadline":self.now+9,
                                "domain_id":"E", "writer_epoch":"1"}}
        self.core = self.create()

    def tearDown(self): self.tmp.cleanup()

    def create(self, **kwargs):
        args = dict(contract_sha="a"*64,script=self.script,script_sha=sha(self.script.read_bytes()),
                    worker=self.worker,worker_script=self.script,worker_script_sha=sha(self.script.read_bytes()),
                    limits=self.limits,stages=self.stages,probe=self.probe,clock=self.clock)
        args.update(kwargs)
        return s.SupervisorCore(**args)

    def binding(self):
        return {"contract_sha256":"a"*64,"worker_pid":100,"worker_start_identity":"toy-worker",
                "worker_process_group":100,"nonce":"toy-nonce","stage":"toy",
                "owner":{"pid":101,"start_identity":"toy-owner","process_group":101,"domain_id":"E","writer_epoch":"1"},
                "resource_caps":{"scope":"UNIT_TEST_ONLY"},"absolute_deadline_monotonic":self.now+9}

    def test_ack_registers_same_worker_caps_deadline_and_nonce_once(self):
        binding=self.binding()
        self.assertEqual(self.core.ack(binding), {"kind":"ACK","binding":binding})
        with self.assertRaises(Rejected): self.core.ack(binding)
        self.assertTrue(self.core.stopped)
        self.assertEqual(self.core.cleanup_result["status"], "INCOMPLETE")

    def test_null_or_missing_caps_and_script_change_refused(self):
        with self.assertRaises(Rejected): self.create(limits=replace(self.limits,max_rss_bytes=None))
        with self.assertRaises(Rejected): self.create(script_sha="b"*64)
        binding=self.binding();binding["resource_caps"]={}
        with self.assertRaises(Rejected): self.core.ack(binding)

    def test_script_change_after_registration_stops_at_next_sample(self):
        self.core.ack(self.binding())
        self.script.write_bytes(b"CHANGED_UNIT_TEST_ONLY")
        with self.assertRaises(Rejected):self.core.tick(force=True)
        self.assertEqual(self.core.cleanup_result["status"],"INCOMPLETE")

    def test_registered_descendants_cancel_reaps_leaf_then_parent(self):
        self.core.ack(self.binding())
        child=s.Identity(102,"child",101,101)
        self.probe.identities[102]=child;self.probe.desc.add(102)
        self.core.register(child,parent=101)
        result=self.core.stop("CANCEL")
        self.assertEqual([pid for pid,_ in self.probe.signals], [102,101])
        self.assertEqual(result["status"],"CONTROL_REAPED")

    def test_unregistered_descendant_is_not_adopted_or_signalled(self):
        self.core.ack(self.binding())
        self.probe.desc.add(102)
        with self.assertRaises(Rejected): self.core.tick(force=True)
        self.assertFalse(any(pid==102 for pid,_ in self.probe.signals))
        self.assertEqual(self.core.cleanup_result["status"],"INCOMPLETE")

    def test_heartbeat_loss_deadline_and_resource_unknown_stop(self):
        self.tearDown()
        for case in ("heartbeat","deadline","rss","disk","output","unknown"):
            with self.subTest(case=case):
                self.setUp()
                try:
                    self.core.ack(self.binding())
                    if case=="heartbeat":self.now+=1.1
                    if case=="deadline":self.now+=11
                    if case in ("rss","disk","output"):self.probe.readings[case+"_bytes"]=101
                    if case=="unknown":self.probe.readings["rss_bytes"]=None
                    with self.assertRaises(Rejected):self.core.tick(force=True)
                    self.assertTrue(self.core.stopped)
                    self.assertEqual(self.core.cleanup_result["status"],"INCOMPLETE")
                finally:self.tearDown()

    def test_pid_reuse_does_not_signal_foreign_process(self):
        self.core.ack(self.binding())
        self.probe.identities[101]=replace(self.owner,start="reused")
        result=self.core.stop("CANCEL")
        self.assertFalse(self.probe.signals)
        self.assertEqual(result["status"],"INCOMPLETE")

    def test_term_timeout_escalation_and_no_restart(self):
        self.core.ack(self.binding());self.probe.term_exits=False
        result=self.core.stop("CANCEL")
        self.assertEqual(self.probe.signals,[(101,signal.SIGTERM),(101,signal.SIGKILL)])
        self.assertTrue(result["owners"][0]["reaped"])
        with self.assertRaises(Rejected):self.core.heartbeat(self.worker)

    def test_continuous_monitor_samples_and_handles_transport_loss(self):
        self.core.ack(self.binding())
        seen=[]
        def receive(timeout):
            self.now+=timeout
            if len(seen)>3:raise EOFError("toy worker pipe lost")
            seen.append(timeout)
            return None
        outputs=[]
        with self.assertRaises(EOFError):s.monitor(self.core,receive,outputs.append)
        self.assertGreaterEqual(len([e for e in self.core.events if e["kind"]=="SAMPLE"]),3)
        self.assertTrue(outputs[-1]["owners"][0]["reaped"])

    def test_real_short_toy_child_cancel_wait_reap_with_explicit_fake_identity(self):
        # 实际 Popen/信号/wait；身份与资源 Probe 明确是假值，非 OS 监督证明。
        process=subprocess.Popen([sys.executable,"-B","-c","import time;time.sleep(5)"],
                                 stdout=subprocess.DEVNULL,stderr=subprocess.DEVNULL,env={},start_new_session=True)
        identity=s.Identity(process.pid,"UNIT_TEST_ONLY",process.pid,100)
        self.probe.identities[process.pid]=identity;self.probe.desc={process.pid}
        self.core.register(identity,parent=100)
        self.probe.alive=lambda ident:process.poll() is None
        self.probe.signal=lambda ident,sig:process.send_signal(sig)
        def wait(ident,deadline):
            try:process.wait(timeout=max(.001,deadline-time.monotonic()));return True
            except subprocess.TimeoutExpired:return False
        self.probe.wait=wait
        try:
            self.assertTrue(self.core.stop("CANCEL")["owners"][0]["reaped"])
        finally:
            if process.poll() is None:process.kill()
            process.wait(timeout=2)
