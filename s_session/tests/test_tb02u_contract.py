"""#1393：用 toy 原件验证查询契约，不启动产品、服务或浏览器。"""
import copy
import hashlib
import json
import subprocess
from pathlib import Path
import sys
import unittest

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import s_tb02_contract as contract

RULE = "s2-axis-quantifiers+new-bi/1+segment-second/1+raw-initialization-w/1"
META = {"profile_id": contract.PROFILE, "rule_revision": RULE}


def raw_ref(coord):
    return dict(identity_key=f"raw-{coord}", payload_hash="a" * 64,
                receipt_id=f"receipt-{coord}", event_id=f"r{coord}",
                input_revision="1", revision="1", seq=str(coord), source_coord=str(coord))


def bar(coord, low, high):
    return dict(source_coord=str(coord), open=str(low), low=str(low), high=str(high), close=str(high))


def fact(kind, slot, payload, sources, cut=1, meta=META):
    identity = dict(kind=kind, fact_key=contract.canonical([kind, slot]).decode(),
                    payload=payload, input_refs=[raw_ref(i) for i in sources], **meta)
    return dict(object_id="obj-" + hashlib.sha256(contract.canonical(identity)).hexdigest(),
                object_revision="1", batch_id="batch-toy", kind=kind, fact_key=identity["fact_key"],
                payload=payload, input_refs=identity["input_refs"], source_coords=list(map(str, sources)),
                first_known_generation=str(cut), first_known_cut=f"cut-{cut}", published_generation=str(cut),
                withdrawn_generation=None, withdrawal_reason=None, superseded_by=None, lifecycle="active")


class ContractTests(unittest.TestCase):
    def pending(self, cut=1):
        return fact("CC-004.undetermined_raw", "0", dict(raw=bar(0, 0, 10), direction_state="Undetermined",
                    initialization_policy_revision=contract.INITIALIZATION_POLICY, pending_since_cut=f"cut-{cut}"), [0], cut)

    def test_pending_identity_policy_and_source_binding(self):
        first = self.pending()
        contract.validate_fact(first, META)
        refs, bars = contract.validate_sources(first, {"0": raw_ref(0)})
        self.assertEqual(refs, {"0": raw_ref(0)})
        self.assertEqual(bars, [first["payload"]["raw"]])
        later = self.pending(3)
        contract.validate_fact(later, META)
        self.assertEqual(first["input_refs"], later["input_refs"])
        self.assertNotEqual(first["object_id"], later["object_id"])
        for mutate in (
            lambda x: x["payload"].update(direction_state="UP"),
            lambda x: x["payload"].update(pending_since_cut="cut-3"),
            lambda x: x["payload"].update(initialization_policy_revision="raw-initialization-w/0"),
            lambda x: x["payload"]["raw"].update(high="9"),
            lambda x: x["payload"].update(envelope={}),
        ):
            bad = copy.deepcopy(first)
            mutate(bad)
            with self.assertRaises(ValueError):
                contract.validate_fact(bad, META)
        with self.assertRaises(ValueError):
            contract.validate_fact(first, {**META, "rule_revision": contract.base_rule(RULE)})

    def test_empty_pending_and_established_first_pair(self):
        pending = fact("CC-004.initialization", "W", dict(initialization_policy_revision=contract.INITIALIZATION_POLICY,
                       direction_state="Undetermined", first_raw_pair=None), [])
        contract.validate_fact(pending, META)
        evidence = dict(direction="UP", established_at="2", previous_acc=bar(1, 2, 8), incoming=bar(2, 3, 11), source_coords=["0", "1", "2"])
        established = fact("CC-004.initialization", "W", dict(initialization_policy_revision=contract.INITIALIZATION_POLICY,
                           direction_state="Established", first_raw_pair=evidence), [0, 1, 2])
        contract.validate_fact(established, META)
        _, bars = contract.validate_sources(established, {str(i): raw_ref(i) for i in range(3)})
        self.assertEqual(bars, [bar(1, 2, 8), bar(2, 3, 11)])
        bad = copy.deepcopy(established)
        bad["payload"]["first_raw_pair"] = None
        with self.assertRaises(ValueError):
            contract.validate_fact(bad, META)

    def test_group_versions_are_bound_to_cut_rule(self):
        evidence = dict(direction="UP", established_at="1", previous_acc=bar(0, 0, 4), incoming=bar(1, 1, 5), source_coords=["0", "1"])
        payload = dict(group_index="0", group_anchor="0", members=["0"], open="0", low="0", high="4", close="4",
                       high_sources=["0"], low_sources=["0"], confirmed=True, direction="UP", direction_evidence=evidence,
                       confirmation_evidence=evidence, waiting_reasons=[], initialization_policy_revision=contract.INITIALIZATION_POLICY,
                       initial_direction_evidence=evidence)
        current = fact("CC-005.inclusion_group", "0", payload, [0, 1])
        contract.validate_fact(current, META)
        with self.assertRaises(ValueError):
            contract.validate_fact(current, {**META, "rule_revision": contract.base_rule(RULE)})
        legacy_payload = {k: v for k, v in payload.items() if k not in ("initialization_policy_revision", "initial_direction_evidence")}
        legacy_meta = {**META, "rule_revision": contract.base_rule(RULE)}
        legacy = fact("CC-005.inclusion_group", "0", legacy_payload, [0, 1], meta=legacy_meta)
        contract.validate_fact(legacy, legacy_meta)
        with self.assertRaises(ValueError):
            contract.validate_fact(legacy, META)

    def test_javascript_toy_contract_matches_pending_and_initialization(self):
        initial = fact("CC-004.initialization", "W", dict(initialization_policy_revision=contract.INITIALIZATION_POLICY,
                       direction_state="Undetermined", first_raw_pair=None), [])
        valid = [self.pending(), self.pending(3), initial]
        invalid = []
        for key, value in (("direction_state", "UP"), ("pending_since_cut", "cut-9"),
                           ("initialization_policy_revision", "raw-initialization-w/0"), ("envelope", {})):
            bad = copy.deepcopy(valid[0])
            bad["payload"][key] = value
            invalid.append(bad)
        client = Path(__file__).resolve().parents[1] / "browser/tb01c-client.js"
        script = """
const c = require(process.argv[1]);
const cases = JSON.parse(require('fs').readFileSync(0, 'utf8'));
(async () => {
  // CF #1394 起 typed 校验为异步 Q3 入口（判据不变）。
  for (const value of cases.valid) await c.validateTyped(value);
  for (const value of cases.invalid) {
    let rejected = false;
    try { await c.validateTyped(value); } catch (_) { rejected = true; }
    if (!rejected) throw new Error('非法初始化对象被放行');
  }
  process.stdout.write(JSON.stringify({valid:cases.valid.length,rejected:cases.invalid.length}));
})().catch(e => { process.stderr.write(e.stack); process.exitCode = 1; });
"""
        result = subprocess.run(["node", "-e", script, str(client)],
                                input=json.dumps(dict(valid=valid, invalid=invalid)),
                                text=True, capture_output=True, check=True)
        self.assertEqual(json.loads(result.stdout), dict(valid=3, rejected=4))

    def test_existing_withdrawal_contract_carries_many_to_one_replaces(self):
        old = self.pending()
        old.update(withdrawn_generation="2", withdrawal_reason="superseded_by_revision", superseded_by="obj-new-group", lifecycle="withdrawn")
        contract.validate_fact(old, META)
        bad = copy.deepcopy(old)
        bad["superseded_by"] = None
        with self.assertRaises(ValueError):
            contract.validate_fact(bad, META)


if __name__ == "__main__":
    unittest.main(verbosity=2)
