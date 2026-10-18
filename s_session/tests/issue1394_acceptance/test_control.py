from dataclasses import replace
from pathlib import Path
import time
import unittest
from unittest.mock import patch
import driver as d
import control
import journal
import scheduler as s
import test_driver as baseline
import test_scheduler as simulation


class ControlTests(unittest.TestCase):
    setUp = baseline.AdapterTests.setUp
    tearDown = baseline.AdapterTests.tearDown
    toy_admission = baseline.AdapterTests.toy_admission

    def setup_bridge(self):
        program=self.root/"toy.sh"
        program.write_text("#!/bin/sh\nexit 0\n");program.chmod(0o700)
        admission,_=self.toy_admission([str(program)],"fake")
        # 重新制作具名 toy 合同及新的 single-use 路径；不把字符串改名当产品授权。
        contract={**admission.contract,"scope":"UNIT_TEST_ONLY","single_use_receipt":str(self.root/"SECOND-USE")}
        cp,gp=self.root/"NAMED-TOY-CONTRACT",self.root/"NAMED-TOY-GRANT"
        d.save_new(cp,contract)
        d.save_new(gp,{"execution_authorized":True,"contract_sha256":d.sha(cp.read_bytes()),
                      "driver_sha256":d.sha(Path(d.__file__).read_bytes()),"nonce":"named-toy"})
        admission=d.ExecutionAdmission(*({"path":str(p),"sha256":d.sha(p.read_bytes())} for p in (cp,gp)))
        supervisor=d.Supervisor("fake",admission.contract_sha,admission.business_deadline,8192)
        fixture=simulation.fixture()
        caps={"scope":"UNIT_TEST_ONLY"}
        b=replace(fixture.steps[0].binding,contract_sha=admission.contract_sha,domain="E",owner="E",epoch=1)
        step=replace(fixture.steps[0],stage="toy-only",binding=b,argv=(str(program),),
                     resources_sha=s.digest(caps),deadline=admission.business_deadline)
        plan=replace(fixture,contract_sha=admission.contract_sha,steps=(step,),work_deadline=admission.business_deadline,
                     cleanup_deadline=admission.cleanup_deadline,seal_deadline=admission.seal_deadline)
        log=journal.Journal.create(self.root/"journal",plan,max_bytes=100000,max_records=10)
        budget=d.ReadBudget(admission.business_deadline,100,10000)
        bridge=control.ControlBridge(admission,supervisor,log,budget)
        return bridge,step,plan,caps

    def test_toy_real_gate_p0_once_journal_and_cleanup(self):
        bridge,step,plan,caps=self.setup_bridge()
        try:
            with patch.object(bridge.supervisor,"ack",return_value={"toy":True}), \
                 patch.object(d,"process_identity",side_effect=lambda p, deadline:{"pid":p.pid,"start_identity":"UNIT_TEST_ONLY","process_group":p.pid}):
                owner=bridge.start_toy(step,plan.seed_sha,self.root/"owner",caps)
                result=bridge.finish_toy(step,owner)
                self.assertEqual(result["receipt"]["status"],"CONTROL_ONLY")
                self.assertEqual(bridge.admission.consumed,{("toy-only","E","1")})
                with self.assertRaises(d.Rejected):bridge.start_toy(step,plan.seed_sha,self.root/"again",caps)
            self.assertTrue(bridge.cleanup_all("COMPLETE")[0].reaped)
        finally:
            bridge.admission.cleanup_all("test-finally")
            bridge.journal.close()

    def test_failed_start_consumes_durable_stage_and_reaps_registered_owner(self):
        bridge,step,plan,caps=self.setup_bridge()
        try:
            with patch.object(d,"process_identity",side_effect=OSError("toy identity error")):
                with self.assertRaises(OSError):bridge.start_toy(step,plan.seed_sha,self.root/"owner",caps)
            self.assertTrue(next(iter(bridge.admission.active_owners.values())).cleanup_result["reaped"])
            with self.assertRaises(d.Rejected):bridge.start_toy(step,plan.seed_sha,self.root/"again",caps)
        finally:bridge.journal.close()

    def test_mode_rename_never_produces_simulated_business_proof(self):
        bridge,step,plan,caps=self.setup_bridge()
        try:
            bridge.mode="UNIT_TEST_ONLY"
            with self.assertRaises(d.Rejected):bridge.perform(step,plan.seed_sha)
            self.assertNotIsInstance(bridge,s.SimulationAdapter)
        finally:bridge.journal.close()

    def test_shared_read_budget_not_renewed_for_second_original(self):
        bridge,step,plan,caps=self.setup_bridge()
        try:
            original=self.root/"original";original.write_bytes(b"123456")
            bridge.budget.max_bytes=10
            bridge.read_original(original)
            with self.assertRaises(d.Rejected):bridge.hash_original(original)
            self.assertGreater(bridge.budget.bytes_read,10)
        finally:bridge.journal.close()

    def test_control_seal_owns_directory_and_failure_cannot_become_product_pass(self):
        bridge,step,plan,caps=self.setup_bridge()
        bridge.seal_directory=d.OwnedDirectory(self.root/"seal")
        bridge.seal_budget=d.ReadBudget(bridge.admission.seal_deadline,100,100000)
        try:
            result=bridge.seal("FAILED",(),(),bridge.admission.seal_deadline)
            self.assertEqual(result.final_outcome,"FAILED")
            raw=(self.root/"seal/SEAL.json").read_bytes()
            self.assertEqual(result.original_sha,d.sha(raw))
            self.assertEqual(d.strict(raw)["product_status"],"NOT_RUN")
            with self.assertRaises(d.Rejected):bridge.seal("CONTROL_COMPLETE",(),(),bridge.admission.seal_deadline)
        finally:bridge.journal.close()
