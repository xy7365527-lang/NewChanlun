"""#1414 方案 A：纯内存对拍旧两次校验与新单次全上下文校验；不作运行验收。"""
import ast
import copy
import hashlib
import importlib.util
import unittest
from pathlib import Path
from unittest.mock import patch

from test_tb02d_contract import samples

ROOT = Path(__file__).resolve().parents[1]
spec = importlib.util.spec_from_file_location("validate_once_audit", ROOT / "s_query_integrity.py")
audit = importlib.util.module_from_spec(spec)
spec.loader.exec_module(audit)
c = audit.tb02


def old_project_fact(row, parse):
    # 方案 A 之前 project_fact 的冻结函数体，用现有未改动 validator 做差异对拍。
    if type(row) is not dict:
        row = dict(zip(c.FACT_COLUMNS, row))
    result = {k: row[k] for k in c.FACT_COLUMNS if k not in ("payload_json", "input_refs_json", "source_coords_json")}
    for name in ("payload", "input_refs", "source_coords"):
        result[name] = parse(row[name + "_json"], dict if name == "payload" else list)
        if row[name + "_json"] != c.canonical(result[name]).decode():
            raise ValueError("事实 JSON 列不是封存规范字节")
    for name in ("object_revision", "first_known_generation", "published_generation", "withdrawn_generation"):
        if result[name] is not None:
            if type(result[name]) is not int:
                raise ValueError("事实持久整数类型不符")
            result[name] = str(result[name])
    result["lifecycle"] = "active" if result["withdrawn_generation"] is None else "withdrawn"
    c.validate_fact(result)
    return result


def old_sequence(row, meta, raw):
    value = old_project_fact(row, audit.parse_json)
    c.validate_fact(value, meta, raw)
    return value


def material(kind="CC-054.knowledge_state", payload=None, withdrawn=False):
    if payload is None:
        payload = dict(scope="TB-02-A", input_frontier="0",
                       known_facts=["raw_ohlc", "inclusion_trace", "group_provenance"],
                       waiting_reasons=[], domain="established_direction_without_extreme_identity_competition", requests=[])
    meta = dict(profile_id=c.PROFILE, rule_revision="s2-axis-quantifiers+new-bi/1+segment-second/1")
    raw = [dict(identity_key="raw-key", payload_hash="a" * 64, receipt_id="receipt",
                event_id="event", input_revision="1", revision="1", seq="0", source_coord="0")]
    slot = payload["slot"] if kind in c.SEGMENT_KINDS else "TB-02-A"
    fact_key = c.canonical([kind, slot]).decode()
    identity = dict(kind=kind, fact_key=fact_key, payload=payload, input_refs=raw, **meta)
    row = dict(object_id="obj-" + hashlib.sha256(c.canonical(identity)).hexdigest(), object_revision=1,
               kind=kind, batch_id="batch-1", fact_key=fact_key,
               payload_json=c.canonical(payload).decode(), input_refs_json=c.canonical(raw).decode(),
               source_coords_json='["0"]', first_known_generation=1, first_known_cut="cut-1", published_generation=1,
               withdrawn_generation=2 if withdrawn else None, withdrawal_reason="fact_removed" if withdrawn else None,
               superseded_by=None)
    return row, meta, raw


class ValidateOnceTests(unittest.TestCase):
    def test_legal_active_withdrawn_dict_tuple_and_second_outputs_exact(self):
        second, segment = samples()
        for kind, payload in [("CC-054.knowledge_state", None), ("CC-012.feature_sequence", second), ("CC-013.segment", segment)]:
            for withdrawn in (False, True):
                row, meta, raw = material(kind, payload, withdrawn)
                for value in (row, tuple(row[k] for k in c.FACT_COLUMNS)):
                    with self.subTest(kind=kind, withdrawn=withdrawn, tuple=type(value) is tuple):
                        before = copy.deepcopy((value, meta, raw))
                        old = old_sequence(value, meta, raw)
                        new = c.project_fact(value, audit.parse_json, meta=meta, raw=raw)
                        self.assertEqual(c.canonical(old), c.canonical(new))
                        self.assertEqual((value, meta, raw), before)

    def test_one_full_validation_receives_original_context(self):
        row, meta, raw = material()
        with patch.object(c, "validate_fact", wraps=c.validate_fact) as validate:
            c.project_fact(row, audit.parse_json, meta=meta, raw=raw)
            self.assertEqual(validate.call_count, 1)
            self.assertIs(validate.call_args.args[1], meta)
            self.assertIs(validate.call_args.args[2], raw)
            validate.reset_mock()
            old_sequence(row, meta, raw)
            self.assertEqual(validate.call_count, 2)

    def test_structure_lifecycle_meta_hash_and_raw_corruption_still_rejected(self):
        mutations = [
            ("wire", lambda r, m, w: r.update(payload_json=r['payload_json'].replace('"input_frontier":"0"', '"input_frontier":0'))),
            ("payload", lambda r, m, w: r.update(payload_json=r['payload_json'].replace('"TB-02-A"', '"wrong"'))),
            ("persistent_integer", lambda r, m, w: r.update(object_revision=True)),
            ("first_known", lambda r, m, w: r.update(first_known_cut="cut-2")),
            ("active_lifecycle", lambda r, m, w: r.update(withdrawal_reason="fact_removed")),
            ("withdrawn_lifecycle", lambda r, m, w: r.update(withdrawn_generation=1, withdrawal_reason="fact_removed")),
            ("fact_key", lambda r, m, w: r.update(fact_key='["wrong","slot"]')),
            ("source_coords", lambda r, m, w: r.update(source_coords_json='["1"]')),
            ("meta", lambda r, m, w: m.update(profile_id="wrong")),
            ("hash", lambda r, m, w: r.update(object_id="obj-" + "0" * 64)),
            ("raw", lambda r, m, w: w[0].update(receipt_id="wrong")),
            ("raw_missing", lambda r, m, w: w.clear()),
            ("noncanonical", lambda r, m, w: r.update(source_coords_json='[ "0" ]')),
            ("duplicate", lambda r, m, w: r.update(payload_json=r['payload_json'].replace('{', '{"scope":"TB-02-A",', 1))),
            ("nonfinite", lambda r, m, w: r.update(payload_json=r['payload_json'].replace('"input_frontier":"0"', '"input_frontier":NaN'))),
        ]
        for name, mutate in mutations:
            with self.subTest(name=name):
                row, meta, raw = material(); mutate(row, meta, raw)
                with self.assertRaises(ValueError) as old:
                    old_sequence(row, meta, raw)
                with self.assertRaises(ValueError) as new:
                    c.project_fact(row, audit.parse_json, meta=meta, raw=raw)
                self.assertEqual(str(old.exception), str(new.exception))

    def test_rule_revision_context_still_rejects_segment_schema_mismatch(self):
        second, _ = samples()
        row, meta, raw = material("CC-012.feature_sequence", second)
        meta["rule_revision"] = "s2-axis-quantifiers+new-bi/1+segment-first/1"
        for call in (lambda: old_sequence(row, meta, raw), lambda: c.project_fact(row, audit.parse_json, meta=meta, raw=raw)):
            with self.assertRaisesRegex(ValueError, "段合同与cut规则版本不符"):
                call()

    def test_legacy_two_argument_call_preserves_no_context_behavior(self):
        row, meta, raw = material()
        row["object_id"] = "unverified-without-meta"
        self.assertEqual(c.project_fact(row, audit.parse_json), old_project_fact(row, audit.parse_json))
        with patch.object(c, "validate_fact", wraps=c.validate_fact) as validate:
            c.project_fact(row, audit.parse_json)
            self.assertEqual(validate.call_count, 1)
            self.assertEqual(validate.call_args.args[1:], (None, None))
        row["withdrawal_reason"] = "fact_removed"
        for call in (lambda: c.project_fact(row, audit.parse_json), lambda: old_project_fact(row, audit.parse_json)):
            with self.assertRaises(ValueError): call()

    def test_optional_context_is_independent_and_keyword_only(self):
        row, meta, raw = material()
        c.project_fact(row, audit.parse_json, meta=meta)
        c.project_fact(row, audit.parse_json, raw=raw)
        with self.assertRaises(TypeError): c.project_fact(row, audit.parse_json, meta, raw)
        row["object_id"] = "wrong"
        with self.assertRaises(ValueError): c.project_fact(row, audit.parse_json, meta=meta)
        raw[0]["receipt_id"] = "wrong"
        with self.assertRaises(ValueError): c.project_fact(row, audit.parse_json, raw=raw)

    def test_same_row_identity_does_not_bypass_revalidation(self):
        row, meta, raw = material()
        c.project_fact(row, audit.parse_json, meta=meta, raw=raw)
        raw[0]["receipt_id"] = "changed"
        with self.assertRaises(ValueError): c.project_fact(row, audit.parse_json, meta=meta, raw=raw)
        raw[0]["receipt_id"] = "receipt"
        row["object_id"] = "changed"
        with self.assertRaises(ValueError): c.project_fact(row, audit.parse_json, meta=meta, raw=raw)

    def test_both_audit_assignments_execute_with_original_context(self):
        tree = ast.parse((ROOT / "s_query_integrity.py").read_text())
        wrapper = next(n for n in tree.body if isinstance(n, ast.FunctionDef) and n.name == "_project_canonical_fact")
        delegate = [n for n in ast.walk(wrapper) if isinstance(n, ast.Call) and isinstance(n.func, ast.Attribute)
                    and isinstance(n.func.value, ast.Name) and n.func.value.id == "tb02" and n.func.attr == "_project_fact"]
        self.assertEqual(len(delegate), 1)
        self.assertEqual({k.arg: ast.unparse(k.value) for k in delegate[0].keywords},
                         {"meta": "meta", "raw": "raw", "parse_canonical": "parse_material", "_identity_hash": "hash_identity"})
        found = []
        for function in tree.body:
            if not isinstance(function, ast.FunctionDef) or function.name not in ("_verify_image", "_sealed_generation"):
                continue
            calls = [n for n in ast.walk(function) if isinstance(n, ast.Call) and isinstance(n.func, ast.Name)
                     and n.func.id == "_project_canonical_fact"]
            self.assertEqual(len(calls), 1)
            self.assertEqual({k.arg: ast.unparse(k.value) for k in calls[0].keywords}, {"_parse_pool": "_parse_pool"})
            self.assertEqual([ast.unparse(arg) for arg in calls[0].args[1:]], ["meta", "raw"])
            self.assertFalse(any(isinstance(n, ast.Call) and isinstance(n.func, ast.Attribute)
                                 and isinstance(n.func.value, ast.Name) and n.func.value.id == "tb02"
                                 and n.func.attr in ("project_fact", "validate_fact", "_validate_fact") for n in ast.walk(function)))
            assignment = next(n for n in ast.walk(function) if isinstance(n, ast.Assign) and calls[0] in ast.walk(n))
            row, meta, raw = material()
            env = dict(_project_canonical_fact=audit._project_canonical_fact, row=row,
                       obj={k: v for k, v in row.items() if k not in
                            ("batch_id", "withdrawn_generation", "withdrawal_reason", "superseded_by")},
                       batch_id=row["batch_id"], meta=meta, raw=raw, table="structure_facts", _parse_pool=audit._CanonicalParsePool())
            module = ast.Module(body=[copy.deepcopy(assignment)], type_ignores=[])
            with patch.object(c, "_validate_fact", wraps=c._validate_fact) as validate:
                exec(compile(ast.fix_missing_locations(module), '<audit assignment>', 'exec'), env)
                self.assertEqual(validate.call_count, 1)
                self.assertIs(validate.call_args.args[1], meta)
                self.assertIs(validate.call_args.args[2], raw)
                self.assertTrue(callable(validate.call_args.kwargs["_identity_hash"]))
                self.assertEqual(env['value'], old_sequence(row, meta, raw))
            found.append(function.name)
        self.assertEqual(set(found), {"_verify_image", "_sealed_generation"})

    def test_multiple_corruptions_remain_fail_closed_but_first_detail_can_change(self):
        # 旧首轮没有 raw，先报坐标；新唯一完整校验在同一 ref 循环先发现 raw 不一致。
        row, meta, raw = material()
        row["source_coords_json"] = '["1"]'
        raw[0]["receipt_id"] = "wrong"
        with self.assertRaisesRegex(ValueError, "事实源坐标与扁平输入引用不一一对应"):
            old_sequence(row, meta, raw)
        with self.assertRaisesRegex(ValueError, "typed input_ref 与完整已接纳账不一致"):
            c.project_fact(row, audit.parse_json, meta=meta, raw=raw)


if __name__ == "__main__":
    unittest.main()
