"""#1497 同输入完整验证对拍；纯内存，不依赖 S 二进制或计时阈值。"""
import copy
import importlib.util
import json
from pathlib import Path
import unittest
from unittest.mock import patch

from test_tb02d_parse_pool import audit, fixture, outcome, server
from test_tb02d_key_reuse import delayed_birth_fixture, mutate_delta, bad_batch, rows, put


ROOT = Path(__file__).resolve().parents[1]
spec = importlib.util.spec_from_file_location("tb1497_full_control", ROOT / "s_query_integrity.py")
control = importlib.util.module_from_spec(spec)
spec.loader.exec_module(control)


class NoReuse:
    """测试对照臂：所有材料进入原完整 validator，不对生产暴露开关。"""
    def seed(self, *_args):
        pass

    def batch(self, *_args):
        return None

    def delta(self, value):
        return ()


def original_parse(raw, expected=dict, *, require_canonical=False):
    # #1497 前 parse_json 的完整算法。它同时是新编码可表示性证明的 oracle。
    if type(raw) not in (str, bytes):
        raise ValueError("持久 JSON 类型损坏")
    result = json.loads(raw, object_pairs_hook=control._pairs_unique, parse_constant=control._nonfinite)
    if type(result) not in (expected if type(expected) is tuple else (expected,)):
        raise ValueError("持久 JSON 形状损坏")
    encoded = control.canonical(result)
    if require_canonical and raw != encoded.decode("utf-8"):
        raise ValueError("事实 JSON 列不是封存规范字节")
    return result


control._VerifiedFacts = NoReuse
control.parse_json = original_parse


def change_batch(image, mutate, gen=2):
    """修复封存外层 hash/length/证据，让损坏真正到达内容验证。"""
    import hashlib
    image = copy.deepcopy(image)
    bn, batches = rows(image, "batches")
    dn, deltas = rows(image, "structure_deltas")
    target = next(row for row in batches if json.loads(row["canonical_bytes"])["generation"] == gen)
    batch = json.loads(target["canonical_bytes"])
    mutate(batch)
    content = audit.canonical(batch)
    bid = "batch-" + hashlib.sha256(content).hexdigest()
    target.update(batch_id=bid, canonical_bytes=content, byte_len=len(content))
    row = deltas[gen - 1]
    delta, evidence = json.loads(row["delta_json"]), json.loads(row["catalog_evidence_json"])
    evidence["batch_id"] = bid
    delta.update(index_frontier=bid, catalog_evidence=evidence)
    row.update(index_frontier=bid, catalog_evidence_json=audit.canonical(evidence).decode(),
               delta_json=audit.canonical(delta).decode())
    put(image, "batches", bn, batches)
    put(image, "structure_deltas", dn, deltas)
    return image


class ColdVerifyTests(unittest.TestCase):
    def compare(self, image, accepted=True):
        before = copy.deepcopy(image)
        old = outcome(lambda: control.verify(image, vars(server), memo={}))
        new = outcome(lambda: audit.verify(image, vars(server), memo={}))
        self.assertEqual(new, old)
        self.assertEqual(new[0] == "ok", accepted)
        self.assertEqual(image, before)
        if not accepted:
            return
        old, new = old[1], new[1]
        for cut in range(new["generation"] + 1):
            self.assertEqual(audit.canonical(audit.project_state(new, cut, vars(server))),
                             control.canonical(control.project_state(old, cut, vars(server))))
        for index in range(new["generation"]):
            self.assertEqual(audit.canonical(audit.project_delta(new, index)),
                             control.canonical(control.project_delta(old, index)))
        # 同字节 warm memo 与无 memo 路径也必须保留相同结果。
        self.assertEqual(audit.verify(image, vars(server), memo=new["_audit_memo"]),
                         control.verify(image, vars(server), memo=old["_audit_memo"]))
        self.assertEqual(audit.verify(image, vars(server)), control.verify(image, vars(server)))

    def test_birth_repeat_withdrawal_and_interpreter_independent_path(self):
        for enabled in (False, True):
            with patch.object(audit._CanonicalParsePool, "_layout_matches", return_value=enabled):
                for image in (fixture(), delayed_birth_fixture()):
                    with self.subTest(pool=enabled):
                        self.compare(image)
        # 正式 S 在累计 batch 重写本代坐标，投影再恢复首次获知坐标。
        self.compare(change_batch(fixture(), lambda b: [o.update(
            first_known_generation=2, published_generation=2, first_known_cut="cut-2") for o in b["objects"]]))

    def test_same_id_changed_material_and_duplicate_records_fail_as_before(self):
        image = fixture()
        mutations = [
            lambda d: d["upserts"][0].update(object_revision=True),
            lambda d: d["upserts"][0].update(published_generation="2"),
            lambda d: d["upserts"][0]["payload"].update(input_frontier=0),
            lambda d: d["upserts"][0]["payload"].update(input_frontier=True),
            lambda d: d["upserts"][0]["input_refs"][0].update(receipt_id="other"),
            lambda d: d["upserts"][0].update(object_id=[]),
            lambda d: d["upserts"].append(copy.deepcopy(d["upserts"][0])),
            lambda d: d.update(upserts={}),
            lambda d: d["upserts"].append(None),
        ]
        for mutate in mutations:
            self.compare(mutate_delta(image, mutate), accepted=False)
        self.compare(bad_batch(image), accepted=False)
        for field, value in (("object_revision", True), ("first_known_generation", True),
                             ("published_generation", 2.0), ("first_known_cut", "cut-99"),
                             ("payload_json", "{}"), ("payload_json", " " + rows(image, "structure_facts")[1][0]["payload_json"]),
                             ("source_coords_json", '["99"]'), ("object_id", [])):
            with self.subTest(batch_field=field, value=value):
                self.compare(change_batch(image, lambda b: b["objects"][0].update({field: value})), accepted=False)
        self.compare(change_batch(image, lambda b: b["objects"][0].update(extra="unknown")), accepted=False)

    def test_no_cross_verify_trust_and_deadline_checks_remain(self):
        image = fixture()
        audit.verify(image, vars(server))
        self.compare(bad_batch(image), accepted=False)
        counts = []
        for module in (control, audit):
            with patch.object(module, "_deadline", wraps=module._deadline) as check:
                module.verify(image, vars(server))
                counts.append(check.call_count)
            self.assertEqual(outcome(lambda: module.verify(image, vars(server), deadline=0))[0], "QueryBudgetExceeded")
        self.assertEqual(counts[0], counts[1])

    def test_overflow_with_syntax_or_shape_error_preserves_verify_rejection(self):
        for raw in ('{"x":1e999}', '{"x":1e999,}', '[1e999]', '[1e999,]',
                    '1e999', '1e999 trailing', '{"x":1e999,"x":0}'):
            with self.subTest(raw=raw):
                image = fixture()
                names, deltas = rows(image, "structure_deltas")
                deltas[0]["delta_json"] = raw
                put(image, "structure_deltas", names, deltas)
                self.compare(image, accepted=False)

    def test_exact_material_proof_precedes_reuse(self):
        counts = []
        delta_contract = server._tb02_contract()
        image = fixture()
        for module in (control, audit):
            with patch.object(module.tb02, "_validate_fact", wraps=module.tb02._validate_fact) as facts, \
                 patch.object(delta_contract, "validate_fact", wraps=delta_contract.validate_fact) as delta:
                module.verify(image, vars(server))
                counts.append((sum(call.args[1] is not None for call in facts.call_args_list), delta.call_count))
        self.assertEqual(counts, [(6, 4), (2, 0)])


class ParseEquivalenceTests(unittest.TestCase):
    def compare(self, raw, expected=(dict, list)):
        for strict in (False, True):
            old = outcome(lambda: original_parse(raw, expected, require_canonical=strict))
            new = outcome(lambda: audit.parse_json(raw, expected, require_canonical=strict))
            self.assertEqual(old[0], new[0], (raw, strict))
            if old[0] == "ok":
                self.assertEqual(audit.canonical(old[1]), audit.canonical(new[1]))

    def test_overflow_waits_for_syntax_and_top_level_shape_checks(self):
        cases = ('{"x":1e999}', '{"x":-1e999}', '{"x":1e999,}', '[1e999]',
                 '[1e999,]', '1e999', '1e999 trailing', '{"x":1e999,"x":0}')
        for raw in cases:
            for expected in (dict, list, float, (dict, list)):
                for strict in (False, True):
                    with self.subTest(raw=raw, expected=expected, strict=strict):
                        self.assertEqual(
                            outcome(lambda: original_parse(raw, expected, require_canonical=strict)),
                            outcome(lambda: audit.parse_json(raw, expected, require_canonical=strict)))

    def test_json_fixtures_tb01c_through_tb02e(self):
        count = 0
        for folder in ("tb01c", "tb02a", "tb02b", "tb02c", "tb02d", "tb02e", "tb02d-contract", "tb02e-contract"):
            for path in (ROOT / "tests" / "fixtures" / folder).rglob("*.json"):
                with self.subTest(path=path):
                    self.compare(path.read_bytes())
                    count += 1
        self.assertGreater(count, 20)

    def test_duplicate_nonfinite_number_surrogate_and_encoding_cases(self):
        cases = ['{"a":1,"a":2}', '{"a":{"b":1,"b":2}}', '{"a":NaN}',
                 '{"a":Infinity}', '{"a":-Infinity}', '{"a":1e999}', '{"a":-1e999}',
                 '{"a":' + '9' * 500 + '.0}', '{"a":-0.0}', '{"a":1e-999}',
                 r'{"a":"\ud800"}', r'{"a":"\udfff"}', r'{"\ud800":1}',
                 r'{"a":"\uD83D\uDE00"}', r'{"a":"\\ud800"}',
                 '{"a":"\ud800"}', '{"a":"中文😀"}', '[]', 'null', '1', '{} trailing']
        for raw in cases:
            with self.subTest(raw=repr(raw)):
                self.compare(raw)
                for encoding in ("utf-8", "utf-16", "utf-32"):
                    self.compare(raw.encode(encoding, errors="surrogatepass"))


if __name__ == "__main__":
    unittest.main()
