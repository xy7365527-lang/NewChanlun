"""只运行 UNIT_TEST_ONLY；不创建数据库、不启动进程/产品。"""
from dataclasses import asdict, replace
from pathlib import Path
import unittest

import scheduler as s

PREP = Path(__file__).parent / "fixtures/PREPARATION-CONTRACT.json"
PREP_SHA = "ce719894bcb8aa60e956d723271250d57d892ddbb6c70dea64bd0cf4aaca8fa1"


class Clock:
    value = 1.0

    def __call__(self):
        return self.value


def fixture():
    reqs = s.load_requirements(PREP, PREP_SHA)
    h = lambda v: s.digest("UNIT_TEST_ONLY/" + v)
    steps = []
    for scenario in reqs.scenarios:
        epoch = 1
        request_hash = h(scenario.name + "/original-request")
        receipt_key = "UNIT_TEST_ONLY/" + scenario.name
        domain = scenario.domain if scenario.family == "fault" else "E"
        operation = scenario.operation or "UNIT_TEST_ONLY"
        for kind in s.required_steps(scenario):
            if kind == "recover":
                epoch += 1
            binding = s.Binding("UNIT_TEST_ONLY", h("contract"), PREP_SHA, h("source"), h("input"),
                                h("manifest"), h("voice"), "/UNIT_TEST_ONLY/" + scenario.name + ".db",
                                domain, domain, epoch, "cut-9011",
                                request_hash, receipt_key, operation)
            requests = ()
            if kind in ("business", "resend"):
                # 一个批次两个decision、两个owner；逐项核验而非同owner覆盖。
                requests = (
                    s.Request(binding, "UNIT_TEST_ONLY_EXPECTED_A"),
                    s.Request(replace(binding, owner="B:beta", domain="B:beta",
                                      epoch=1,  # recover 只重启具名目标；beta 未重启。
                                      db=binding.db + ".beta", request_sha=h(receipt_key + "/beta"),
                                      receipt_key=receipt_key + "/beta"), "UNIT_TEST_ONLY_EXPECTED_B"),
                )
            if scenario.family == "normal" and kind == "business":
                requests += tuple(s.Request(replace(binding, receipt_key=receipt_key + "/" + case,
                                                     request_sha=h(receipt_key + "/" + case)),
                                            "UNIT_TEST_ONLY/" + case, case=case)
                                  for case in reqs.negative_domains)
            proofs = s.PROOFS[kind]
            if kind in ("oracle", "recovery_oracle"):
                proofs += tuple("compare/" + key for key in reqs.comparisons)
            steps.append(s.Step(scenario.name, scenario.name + "/" + kind, kind, binding,
                                ("UNIT_TEST_ONLY", scenario.name, kind),
                                h("resources"), 50.0, proofs, requests))
    return s.Plan(reqs, "UNIT_TEST_ONLY", h("contract"), h("seed"), h("source"), h("input"),
                  h("manifest"), h("voice"), h("voice-authority"), h("resources"),
                  h("artifacts"), h("supervisor"), h("policy"),
                  "economic-session/2", "economic-manifest/1", tuple(steps), 60.0, 70.0, 80.0)


class FakeAdapter(s.SimulationAdapter):
    mode = "UNIT_TEST_ONLY"

    def __init__(self, clock=None):
        self.events = []
        self.clock = clock
        self.fail_stage = None
        self.fail_exc = RuntimeError("UNIT_TEST_ONLY failure")
        self.mutate = lambda ev: ev
        self.late = False
        self.bad_cleanup = False
        self.omit_cleanup = False
        self.bad_seal = False
        self.owners = ("UNIT_TEST_ONLY/E", "UNIT_TEST_ONLY/B:alpha", "UNIT_TEST_ONLY/B:beta")

    def perform(self, step, predecessor_sha):
        self.events.append(step.stage)
        if self.fail_stage == step.stage:
            raise self.fail_exc
        if self.late:
            self.clock.value = step.deadline + 0.01
        ev = s.Evidence(step.sha(), step.binding, predecessor_sha, s.digest(step.argv),
                        step.resources_sha, step.deadline, "Received", "INDEPENDENTLY_VERIFIED",
                        s.digest("UNIT_TEST_ONLY/originals"), step.expected_proofs,
                        tuple(s.BusinessReceipt(r, r.binding.request_sha, s.digest(asdict(r)),
                                                "INDEPENDENTLY_VERIFIED", r.expected_outcome)
                              for r in step.requests))
        return self.mutate(ev)

    def active_owner_keys(self):
        return self.owners

    def cleanup_all(self, reason):
        receipts = []
        for index, owner in enumerate(self.owners):
            self.events.append("cleanup/" + owner)
            error = "UNIT_TEST_ONLY cleanup failure" if self.bad_cleanup and index == 0 else ""
            receipts.append(s.CleanupReceipt(owner, not error, s.digest(owner), error))
        return tuple(receipts[:-1] if self.omit_cleanup else receipts)

    def seal(self, outcome, receipts, cleanup, deadline):
        self.events.append("seal/" + outcome)
        return s.SealReceipt(s.digest("UNIT_TEST_ONLY/seal"),
                             "WRONG" if self.bad_seal else outcome,
                             s.digest([asdict(c) for c in cleanup]),
                             s.digest([asdict(r) for r in receipts]))


class SchedulerTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.plan = fixture()

    def execute(self, plan=None, adapter=None, clock=None):
        adapter = adapter or FakeAdapter()
        result = s.Scheduler(plan or self.plan, adapter, clock or (lambda: 1.0)).run()
        return result, adapter

    def test_quantifier_from_original_contract(self):
        reqs = self.plan.requirements
        self.assertEqual(reqs.preparation_sha, PREP_SHA)
        self.assertEqual(len([r for r in reqs.scenarios if r.family == "fault"]), 24)
        self.assertEqual(len([r for r in reqs.scenarios if r.family == "normal"]), 2)
        self.assertEqual(len([r for r in reqs.scenarios if r.family == "extra"]), 3)
        self.assertEqual(len(reqs.scenarios), 30)

    def test_full_order_and_receipt_chain(self):
        result, adapter = self.execute()
        self.assertEqual(result.outcome, "SIMULATED_COMPLETE_NOT_PRODUCT_ACCEPTANCE")
        self.assertEqual(len(result.stage_receipts), len(self.plan.steps))
        previous = self.plan.seed_sha
        for receipt, step in zip(result.stage_receipts, self.plan.steps):
            self.assertEqual(receipt.predecessor_sha, previous)
            self.assertEqual(receipt.step_sha, step.sha())
            previous = receipt.sha()
        self.assertEqual(adapter.events[:len(self.plan.steps)], [p.stage for p in self.plan.steps])
        self.assertFalse(result.execution_authorized)
        self.assertFalse(result.product_executed)
        self.assertIsNotNone(result.seal)

    def test_missing_formal_voice_rejects_before_work_cleanup_still_runs(self):
        result, adapter = self.execute(replace(self.plan, formal_voice_authority_sha=None))
        self.assertEqual(result.outcome, "FAILED")
        self.assertEqual(result.attempted, ())
        self.assertEqual(len(result.cleanup), 3)
        self.assertTrue(adapter.events[-1].startswith("seal/"))

    def test_all_missing_originals_rejected(self):
        for key in ("frozen_resources_sha", "artifact_manifest_sha", "supervisor_sha",
                    "policy_original_sha"):
            with self.subTest(key=key):
                result, _ = self.execute(replace(self.plan, **{key: None}))
                self.assertEqual(result.outcome, "FAILED")
                self.assertFalse(result.attempted)

    def test_product_gate_and_real_adapter_blocked(self):
        result, _ = self.execute(replace(self.plan, execution_authorized=True))
        self.assertEqual(result.outcome, "FAILED")
        adapter = FakeAdapter()
        adapter.mode = "PRODUCT"
        result, _ = self.execute(adapter=adapter)
        self.assertEqual(result.outcome, "FAILED")
        self.assertFalse(result.attempted)

    def test_failure_at_each_kind_cleanup_and_seal(self):
        seen = set()
        for step in self.plan.steps:
            if step.kind in seen:
                continue
            seen.add(step.kind)
            with self.subTest(kind=step.kind):
                adapter = FakeAdapter()
                adapter.fail_stage = step.stage
                result, _ = self.execute(adapter=adapter)
                self.assertEqual(result.outcome, "FAILED")
                self.assertEqual(result.attempted[-1], step.stage)
                self.assertEqual(len(result.cleanup), 3)
                self.assertIsNotNone(result.seal)
                self.assertFalse(any(r.stage == step.stage for r in result.stage_receipts))

    def test_cancel_cleanup_and_seal(self):
        adapter = FakeAdapter()
        adapter.fail_stage = self.plan.steps[4].stage
        adapter.fail_exc = KeyboardInterrupt()
        result, _ = self.execute(adapter=adapter)
        self.assertEqual(result.outcome, "CANCELLED")
        self.assertEqual(len(result.cleanup), 3)
        self.assertEqual(result.seal.final_outcome, "CANCELLED")

    def test_received_is_not_business_success(self):
        adapter = FakeAdapter()
        adapter.mutate = lambda ev: replace(ev, business_status="Received")
        result, _ = self.execute(adapter=adapter)
        self.assertEqual(result.outcome, "FAILED")
        self.assertFalse(result.stage_receipts)

    def test_late_ack_or_persist_stops_before_next_stage(self):
        clock = Clock()
        adapter = FakeAdapter(clock)
        adapter.late = True
        result, _ = self.execute(adapter=adapter, clock=clock)
        self.assertEqual(result.outcome, "FAILED")
        self.assertEqual(len(result.attempted), 1)
        self.assertFalse(result.stage_receipts)
        self.assertEqual(len(result.cleanup), 3)

    def test_duplicate_run_cannot_retry_after_failure(self):
        adapter = FakeAdapter()
        adapter.fail_stage = self.plan.steps[0].stage
        runner = s.Scheduler(self.plan, adapter, lambda: 1.0)
        result = runner.run()
        self.assertEqual(result.outcome, "FAILED")
        with self.assertRaises(s.Rejected):
            runner.run()
        self.assertEqual(len([e for e in adapter.events if e == adapter.fail_stage]), 1)

    def test_stages_cannot_be_missing_reordered_or_repeated(self):
        steps = self.plan.steps
        variants = (steps[1:], (steps[1], steps[0]) + steps[2:], steps + steps[:1])
        for variant in variants:
            with self.subTest(length=len(variant)):
                result, _ = self.execute(replace(self.plan, steps=variant))
                self.assertEqual(result.outcome, "FAILED")
                self.assertFalse(result.attempted)

    def test_binding_corruptions_before_start(self):
        b = self.plan.steps[0].binding
        changes = dict(run_id="another-run", contract_sha="0"*64, preparation_sha="0"*64,
                       source_sha="0"*64, input_sha="0"*64, manifest_sha="0"*64,
                       voice_sha="0"*64, db="relative.db", owner="", domain="",
                       epoch=0, cut="", request_sha="broken", receipt_key="", operation="")
        for name, value in changes.items():
            with self.subTest(name=name):
                bad = replace(self.plan.steps[0], binding=replace(b, **{name: value}))
                result, _ = self.execute(replace(self.plan, steps=(bad,) + self.plan.steps[1:]))
                self.assertEqual(result.outcome, "FAILED")
                self.assertFalse(result.attempted)

    def test_evidence_wrong_binding_predecessor_command_resource_deadline(self):
        changes = dict(step_sha="0"*64, predecessor_sha="0"*64, command_sha="0"*64,
                       resources_sha="0"*64, deadline=123.0, raw_originals_sha="")
        for name, value in changes.items():
            with self.subTest(name=name):
                adapter = FakeAdapter()
                adapter.mutate = lambda ev, name=name, value=value: replace(ev, **{name: value})
                result, _ = self.execute(adapter=adapter)
                self.assertEqual(result.outcome, "FAILED")
                self.assertFalse(result.stage_receipts)

    def test_missing_or_extra_proof_rejected(self):
        for proofs in ((), ("extra-proof",)):
            adapter = FakeAdapter()
            adapter.mutate = lambda ev, proofs=proofs: replace(ev, proofs=proofs)
            result, _ = self.execute(adapter=adapter)
            self.assertEqual(result.outcome, "FAILED")

    def test_cleanup_one_owner_failure_continues_remaining_and_seals(self):
        adapter = FakeAdapter()
        adapter.bad_cleanup = True
        result, _ = self.execute(adapter=adapter)
        self.assertEqual(result.outcome, "INCOMPLETE")
        self.assertEqual(len(result.cleanup), 3)
        self.assertFalse(result.cleanup[0].reaped)
        self.assertTrue(result.cleanup[1].reaped)
        self.assertTrue(result.cleanup[2].reaped)
        self.assertEqual(result.seal.final_outcome, "INCOMPLETE")

    def test_cleanup_missing_owner_fails(self):
        adapter = FakeAdapter()
        adapter.omit_cleanup = True
        result, _ = self.execute(adapter=adapter)
        self.assertEqual(result.outcome, "INCOMPLETE")
        self.assertIn("完整owner", result.error)

    def test_seal_failure_does_not_override_error_with_success(self):
        adapter = FakeAdapter()
        adapter.bad_seal = True
        result, _ = self.execute(adapter=adapter)
        self.assertEqual(result.outcome, "INCOMPLETE")
        self.assertIsNone(result.seal)

    def test_resend_original_identity_required(self):
        index = next(i for i, p in enumerate(self.plan.steps) if p.kind == "resend")
        steps = list(self.plan.steps)
        steps[index] = replace(steps[index], binding=replace(steps[index].binding, receipt_key="wrong"))
        result, _ = self.execute(replace(self.plan, steps=tuple(steps)))
        self.assertEqual(result.outcome, "FAILED")
        self.assertFalse(result.attempted)

    def test_recovery_requires_new_epoch(self):
        index = next(i for i, p in enumerate(self.plan.steps) if p.kind == "recover")
        steps = list(self.plan.steps)
        steps[index] = replace(steps[index], binding=replace(steps[index].binding, epoch=1))
        result, _ = self.execute(replace(self.plan, steps=tuple(steps)))
        self.assertEqual(result.outcome, "FAILED")

    def test_multiple_decision_owner_receipts_cannot_be_dropped_or_extra(self):
        for alter in (lambda receipts: receipts[:-1], lambda receipts: receipts + receipts[:1],
                      lambda receipts: tuple(reversed(receipts))):
            adapter = FakeAdapter()
            adapter.mutate = lambda ev, alter=alter: replace(
                ev, request_receipts=alter(ev.request_receipts)) if ev.request_receipts else ev
            result, _ = self.execute(adapter=adapter)
            self.assertEqual(result.outcome, "FAILED")
            self.assertIn("请求回执全集", result.error)

    def test_request_independent_verdict_required(self):
        adapter = FakeAdapter()
        def mutate(ev):
            if ev.request_receipts:
                first = replace(ev.request_receipts[0], independent_verdict="Received")
                return replace(ev, request_receipts=(first,) + ev.request_receipts[1:])
            return ev
        adapter.mutate = mutate
        result, _ = self.execute(adapter=adapter)
        self.assertEqual(result.outcome, "FAILED")

    def test_same_cut_request_identity_required(self):
        index = next(i for i, p in enumerate(self.plan.steps) if p.requests)
        steps = list(self.plan.steps)
        requests = steps[index].requests
        corrupt = replace(requests[0], binding=replace(requests[0].binding, cut="another-cut"))
        steps[index] = replace(steps[index], requests=(corrupt,) + requests[1:])
        result, _ = self.execute(replace(self.plan, steps=tuple(steps)))
        self.assertEqual(result.outcome, "FAILED")
        self.assertFalse(result.attempted)

    def test_resource_deadline_never_extended(self):
        for deadline in (61.0, float("inf"), float("nan"), True):
            with self.subTest(deadline=deadline):
                bad = replace(self.plan.steps[0], deadline=deadline)
                result, _ = self.execute(replace(self.plan, steps=(bad,) + self.plan.steps[1:]))
                self.assertEqual(result.outcome, "FAILED")

    def test_ab_database_overlap_rejected(self):
        a_db = next(p.binding.db for p in self.plan.steps if p.scenario == "normal/A")
        steps = tuple(replace(p, binding=replace(p.binding, db=a_db))
                      if p.scenario == "normal/B" else p for p in self.plan.steps)
        result, _ = self.execute(replace(self.plan, steps=steps))
        self.assertEqual(result.outcome, "FAILED")


    def test_idempotent_repeat_uses_occurrence_without_changing_identity(self):
        index = next(i for i, p in enumerate(self.plan.steps) if p.requests)
        steps = list(self.plan.steps)
        original = steps[index].requests
        duplicate = replace(original[0], ordinal=1)
        steps[index] = replace(steps[index], requests=original + (duplicate,))
        result, _ = self.execute(replace(self.plan, steps=tuple(steps)))
        self.assertEqual(result.outcome, "SIMULATED_COMPLETE_NOT_PRODUCT_ACCEPTANCE")

    def test_negative_business_response_cannot_turn_into_success(self):
        index = next(i for i, p in enumerate(self.plan.steps) if p.requests)
        steps = list(self.plan.steps)
        original = steps[index].requests
        negative = replace(original[0], expected_outcome="StaleWriter")
        steps[index] = replace(steps[index], requests=(negative,) + original[1:])
        adapter = FakeAdapter()
        def mutate(ev):
            if ev.request_receipts:
                first = replace(ev.request_receipts[0], observed_outcome="Committed")
                return replace(ev, request_receipts=(first,) + ev.request_receipts[1:])
            return ev
        adapter.mutate = mutate
        result, _ = self.execute(replace(self.plan, steps=tuple(steps)), adapter)
        self.assertEqual(result.outcome, "FAILED")
        self.assertIn("预期拒绝结果错绑", result.error)


    def test_original_normal_negative_domain_and_comparison_quantifiers_preserved(self):
        self.assertEqual(len(self.plan.requirements.negative_domains), 5)
        self.assertEqual(len(self.plan.requirements.comparisons), 10)
        index = next(i for i, p in enumerate(self.plan.steps)
                     if p.kind == "business" and p.scenario == "normal/A")
        steps = list(self.plan.steps)
        steps[index] = replace(steps[index], requests=steps[index].requests[:-1])
        result, _ = self.execute(replace(self.plan, steps=tuple(steps)))
        self.assertEqual(result.outcome, "FAILED")
        self.assertIn("负域业务请求缺漏", result.error)
        steps = list(self.plan.steps)
        index = next(i for i, p in enumerate(steps) if p.kind == "oracle")
        steps[index] = replace(steps[index], expected_proofs=s.PROOFS["oracle"])
        result, _ = self.execute(replace(self.plan, steps=tuple(steps)))
        self.assertEqual(result.outcome, "FAILED")
        self.assertIn("步骤判据缺漏", result.error)


if __name__ == "__main__":
    unittest.main()
