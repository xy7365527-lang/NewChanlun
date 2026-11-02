"""#1657：合同门真实经过 launcher，拒绝不发布、不启动、不损害 stop/status。"""
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest
from unittest.mock import patch
sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import s_service_control as control
from r4_anchor_fixture import test_prepare

class AdmissionContractTests(unittest.TestCase):
    def build(self, root, *, pending=True, version="test-only/1", response=None):
        db=root/"source.sqlite"; state=root/"state"
        test_prepare(db,state,pending=pending)
        binary=root/"contract-probe"
        binary.write_text("#!"+sys.executable+"\nimport json,sys\n"+(response or "print('denied',file=sys.stderr)\nsys.exit(7)\n"))
        binary.chmod(0o700)
        config={"schema_revision":"s-launcher/3","db":str(db),"binary":str(binary),
                "startup_timeout_ms":"10000","admission_contract_version":version}
        return control.Controller(config,state)

    def test_contract_rejection_preserves_pending_and_originals_and_releases_locks(self):
        for action in ("start_s","recover"):
            with self.subTest(action=action), tempfile.TemporaryDirectory() as d:
                root=Path(d).resolve(); c=self.build(root)
                original={p:p.read_bytes() for p in root.rglob("*.json")}
                try:
                    with patch.object(c,"runtime_environment",return_value={}), patch.object(c.admission,"publish") as publish, patch.object(c,"start_process") as spawn:
                        with self.assertRaises(subprocess.CalledProcessError) as caught:
                            getattr(c,action)(*(('1',) if action=='start_s' else ('2','event')))
                        publish.assert_not_called(); spawn.assert_not_called()
                        self.assertEqual(caught.exception.control_receipt['action_stage'],'contract')
                        self.assertGreater(caught.exception.control_receipt['wire_bytes'],0)
                    self.assertEqual(original,{p:p.read_bytes() for p in root.rglob("*.json")})
                    self.assertEqual(c.status('s')['state'],'not_started')
                finally: c.close()
                # 失败后的下一生命周期仍能取得原控制锁。
                again=control.Controller(c.config,root/'state'); again.close()

    def test_missing_version_does_not_prevent_status_or_invoke_binary(self):
        with tempfile.TemporaryDirectory() as d:
            root=Path(d).resolve(); c=self.build(root,version=None)
            try:
                self.assertEqual(c.status('s')['state'],'not_started')
                with patch.object(control.subprocess,'run') as run:
                    with self.assertRaisesRegex(ValueError,'AdmissionContractVersionMissing'): c.start_s('1')
                    run.assert_not_called()
                self.assertEqual(c.status('s')['state'],'not_started')
            finally: c.close()

    def test_contract_checker_binds_verified_sha_and_original_deadline(self):
        script="a=dict(zip(sys.argv[2::2],sys.argv[3::2]))\nprint(json.dumps({'ok':True,'contract_version':a['--contract-version'],'sha256':a['--limits-sha']}))\n"
        with tempfile.TemporaryDirectory() as d:
            root=Path(d).resolve(); c=self.build(root,response=script)
            try:
                c.admission.prepare(); deadline=c.admission_deadline
                with patch.object(c,'runtime_environment',return_value={}) as environment:
                    c.check_admission_contract()
                    environment.assert_called_once_with(deadline)
                self.assertTrue((root/'state'/'.s-admission-pending').exists())
                self.assertFalse((root/'state'/'s-admission').exists())
            finally: c.close()

    def test_contract_success_cannot_mask_substituted_reply(self):
        with tempfile.TemporaryDirectory() as d:
            root=Path(d).resolve(); c=self.build(root,response="print('{}')\n")
            try:
                c.admission.prepare()
                with patch.object(c,'runtime_environment',return_value={}):
                    with self.assertRaisesRegex(ValueError,'AdmissionContractReplyInvalid'): c.check_admission_contract()
            finally: c.close()

    def test_contract_child_uses_remaining_not_new_timeout(self):
        with tempfile.TemporaryDirectory() as d:
            root=Path(d).resolve(); c=self.build(root)
            try:
                c.admission.prepare()
                with patch.object(c,'runtime_environment',return_value={}), patch.object(control.time,'monotonic',return_value=c.admission_deadline+1), patch.object(control.subprocess,'run') as run:
                    with self.assertRaises((ValueError,TimeoutError)): c.check_admission_contract()
                    run.assert_not_called()
            finally: c.close()

if __name__=='__main__': unittest.main()
