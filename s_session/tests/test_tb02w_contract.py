"""#1420 段合同 v3（s-segment/3）锁：第二种候选三态终点、扫描覆盖/预算、K1 游标与续扫检查点。
人工构造只检合同（Python 与浏览器 JS 同输入同判定），不作段语义或正式验收。"""
import copy
import hashlib
import importlib.util
import json
import subprocess
import unittest
from pathlib import Path

from test_tb02c_contract import c
from test_tb02d_contract import first_sample, samples

ROOT = Path(__file__).resolve().parents[1]
CLIENT = ROOT / "browser/tb01c-client.js"
RULE_V2 = "s2-axis-quantifiers+new-bi/1+segment-second/1"
RULE_V3 = "s2-axis-quantifiers+new-bi/1+segment-second/2"
SEQ, SEG, VER, CHANGE = "CC-012.feature_sequence", "CC-013.segment", "CC-056.segment_version", "CC-055.segment_change"
TERMINALS = {  # 名 → (scan.semantic_state, second_sequence_status, coverage)
    "CONFIRMED": ("CONFIRMED", "CONFIRMED", "COMPLETE"),
    "KILLED": ("KILLED_BY_NEW_EXTREME", "KILLED", "COMPLETE"),
    "OPEN": ("OPEN_AT_PREFIX_END", "PENDING", "COMPLETE"),
    "BUDGET": (None, "PENDING", "INCOMPLETE_RESOURCE_BOUND"),
}
NODE = ("const a=require(process.argv[1]);let s='';process.stdin.on('data',x=>s+=x);process.stdin.on('end',()=>{"
        "try{a.validateSegmentPayload(process.argv[2],JSON.parse(s))}catch(e){process.stdout.write(e.message);process.exit(3)}})")


def v3(p):
    p = copy.deepcopy(p)
    p.update(schema_revision="s-segment/3", semantic_version="segment-second/2")
    return p


def cursor(seg_dir="UP", b_stroke="0", extreme="600", granted="64", consumed="2", sealed=None, kill=None, matched=None, standard=()):
    return dict(schema="second-fold-cursor/1", rule_revision="second-candidate-terminal/1", budget_policy="second-seq-scan-budget/1",
                budget_granted=granted, candidate=dict(seg_dir=seg_dir, b_stroke=b_stroke, candidate_extreme=extreme),
                input_stroke_count="3", from_stroke=str(int(b_stroke) + 1), through_stroke="2", consumed_len=consumed,
                consumed_digest="0123456789abcdef", dir_state=seg_dir, standard=copy.deepcopy(list(standard)), breach_watermark="528",
                semantic_state=sealed, killed_by=copy.deepcopy(kill), matched=copy.deepcopy(matched))


def second_v3(terminal):
    """P＝600 未被越过；KILLED 取 P＝500，被笔 2 高点 528 严格越过（向上段比 >）。"""
    second, _ = samples()
    p = v3(second); d = p["data"]; f = d["facts"]; hit = f["matched"]
    semantic, status, coverage = TERMINALS[terminal]
    extreme = "500" if terminal == "KILLED" else "600"
    kill = dict(stroke_idx="2", price="528", candidate_extreme=extreme, comparison=">") if terminal == "KILLED" else None
    matched = hit if terminal == "CONFIRMED" else None
    if matched is None:
        f["checks"] = [dict(hit, top_fractal=False)]
    truncated = terminal == "BUDGET"
    granted = {"CONFIRMED": "64", "KILLED": "64", "OPEN": None, "BUDGET": "2"}[terminal]
    f.update(matched=matched, resource_truncated=truncated, candidate_extreme=extreme, breach_watermark="528",
             semantic_state=semantic, killed_by=kill, coverage=coverage)
    d["scan"].update(resource_truncated=truncated, semantic_state=semantic, coverage=coverage,
                     budget=dict(policy="second-seq-scan-budget/1", granted=granted),
                     cursor=cursor(extreme=extreme, granted=granted, standard=f["standard"],
                                   sealed=semantic if terminal in ("CONFIRMED", "KILLED") else None, kill=kill, matched=matched))
    d["second_sequence_status"] = status
    return p


def second_down_killed():
    """向下段：P＝150，被 100 严格跌破（比 <）。只检方向分支，不代表真实几何。"""
    p = second_v3("KILLED"); d = p["data"]; f = d["facts"]
    d.update(direction="DOWN", member_direction="DOWN")
    kill = dict(stroke_idx="2", price="100", candidate_extreme="150", comparison="<")
    f.update(candidate_extreme="150", breach_watermark="100", killed_by=kill)
    d["scan"]["cursor"] = cursor(seg_dir="DOWN", extreme="150", standard=f["standard"], sealed="KILLED_BY_NEW_EXTREME", kill=kill)
    d["scan"]["cursor"]["breach_watermark"] = "100"
    return p


def first_v3(status):
    second, _ = samples()
    p = v3(first_sample(second))
    p["data"]["second_sequence_status"] = status
    return p


def segment_v3(result):
    """result ∈ CASE_TWO / KILLED_CASE_TWO / PENDING_CASE_TWO（OPEN）/ PENDING_BUDGET（资源界，事实未知）。"""
    _, segment = samples()
    p = v3(segment); d = p["data"]
    if result == "CASE_TWO":
        return p
    budget = result == "PENDING_BUDGET"
    d.update(result="PENDING_CASE_TWO" if budget else result, geometric_end=None, trigger_stroke=None, terminated_known_at=None)
    d["fact_vector"].update(new_segment_formed=None if budget else False, segment_broken=None if budget else False,
                            scan_complete=not budget, termination_witness=None)
    if result == "KILLED_CASE_TWO":
        d["waiting_reasons"] = ["second_candidate_killed_by_new_extreme"]
    return p


def checkpoint(**kw):
    """续扫检查点：预算耗尽的未决候选（consumed_len == budget_granted）。"""
    elements = [dict(high="400", low="200", stroke_idx=str(i), stroke_indices=[str(i)]) for i in range(1, 3)]
    kw.setdefault("granted", "2")
    kw.setdefault("consumed", "2")
    return cursor(standard=elements, **kw)


def version_v3(round_strokes="2", cursors=None):
    known = dict(generation="1", input_frontier="13", receipt_id="r", received_at="t", semantic_commit_ns=None)
    return dict(schema_revision="s-segment/3", semantic_version="segment-second/2", policy_version="seed_closed_strict_second",
                view_role="main", object_generation="1", slot="TB-02-W",
                data=dict(input_frontier="13", waiting_reasons=[], known_at=known,
                          delivered_scope="seed_strict_first_and_second_terminal",
                          deferred_scope=["general_equal_identity", "global_G006_bridge"],
                          scan_budget=dict(policy="second-seq-scan-budget/1", round_strokes=round_strokes),
                          scan_cursors=[checkpoint()] if cursors is None else cursors))


def records(first, second, segment):
    return [dict(object_id=oid, kind=kind, payload=p, source_coords=["1", "5", "9", "13"]) for oid, kind, p in
            [("first-object", SEQ, first), ("second-object", SEQ, second), ("segment-object", SEG, segment)]]


def mutated(p, change):
    p = copy.deepcopy(p)
    change(p["data"])
    return p


class TB02WContractTest(unittest.TestCase):
    def judge(self, kind, p):
        """同一载荷交 Python 与浏览器 JS 各判一次；返回 (py拒因, js拒因)，接受为 None。"""
        try:
            c.validate_payload(kind, p)
            py = None
        except ValueError as error:
            py = str(error)
        result = subprocess.run(["node", "-e", NODE, str(CLIENT), kind], input=json.dumps(p), text=True, capture_output=True)
        self.assertIn(result.returncode, (0, 3), result.stderr)
        return py, (result.stdout if result.returncode == 3 else None)

    def accept(self, kind, p):
        self.assertEqual(self.judge(kind, p), (None, None))

    def reject(self, kind, p, reason=None):
        py, js = self.judge(kind, p)
        self.assertIsNotNone(py, "Python 误收")
        self.assertIsNotNone(js, "JS 误收")
        if reason is not None:
            self.assertEqual((py, js), (reason, reason))

    def test_three_second_terminals_accept(self):
        for terminal in TERMINALS:
            with self.subTest(terminal):
                self.accept(SEQ, second_v3(terminal))
        self.accept(SEQ, second_down_killed())

    def test_first_mirrors_second_status(self):
        for status in ("NOT_APPLICABLE", "PENDING", "CONFIRMED", "KILLED"):
            self.accept(SEQ, first_v3(status))
        second, _ = samples()
        self.reject(SEQ, mutated(first_sample(second), lambda d: d.update(second_sequence_status="KILLED")))
        self.reject(SEQ, mutated(first_v3("KILLED"), lambda d: d["scan"].update(coverage="COMPLETE")))

    def test_matched_only_when_confirmed(self):
        hit = second_v3("CONFIRMED")["data"]["facts"]["matched"]
        def killed_with_hit(d):
            d["facts"].update(matched=copy.deepcopy(hit), checks=[copy.deepcopy(hit)])
        for p in (mutated(second_v3("KILLED"), killed_with_hit), mutated(second_v3("OPEN"), killed_with_hit),
                  mutated(second_v3("CONFIRMED"), lambda d: d["facts"].update(matched=None))):
            self.reject(SEQ, p, "SECOND命中与状态不符")

    def test_coverage_and_status_binding(self):
        def coverage(value):
            return lambda d: (d["scan"].update(coverage=value), d["facts"].update(coverage=value))
        def truncated(value):
            return lambda d: (d["scan"].update(resource_truncated=value), d["facts"].update(resource_truncated=value))
        def semantic(value):
            return lambda d: (d["scan"].update(semantic_state=value), d["facts"].update(semantic_state=value))
        bound = "SECOND覆盖与语义/截断不一致"
        self.reject(SEQ, mutated(second_v3("CONFIRMED"), coverage("INCOMPLETE_RESOURCE_BOUND")), bound)
        self.reject(SEQ, mutated(second_v3("BUDGET"), truncated(False)), bound)
        self.reject(SEQ, mutated(second_v3("OPEN"), semantic(None)), bound)
        self.reject(SEQ, mutated(second_v3("BUDGET"), semantic("OPEN_AT_PREFIX_END")), bound)
        self.reject(SEQ, mutated(second_v3("OPEN"), coverage("PARTIAL")), "SECOND终态/覆盖未声明")
        self.reject(SEQ, mutated(second_v3("OPEN"), semantic("DONE")), "SECOND终态/覆盖未声明")
        self.reject(SEQ, mutated(second_v3("OPEN"), lambda d: d["facts"].update(coverage="INCOMPLETE_RESOURCE_BOUND")), "SECOND事实终态与扫描不一致")
        self.reject(SEQ, mutated(second_v3("CONFIRMED"), lambda d: (semantic("OPEN_AT_PREFIX_END")(d), d["scan"]["cursor"].update(semantic_state=None, matched=None))), "SECOND状态未绑定语义终点")
        self.reject(SEQ, mutated(second_v3("OPEN"), lambda d: d.update(second_sequence_status="KILLED")), "SECOND状态未绑定语义终点")

    def test_budget(self):
        self.reject(SEQ, mutated(second_v3("BUDGET"), lambda d: d["scan"]["budget"].update(policy="second-seq-scan-budget/2")), "扫描预算政策版本不符")
        self.reject(SEQ, mutated(second_v3("BUDGET"), lambda d: (d["scan"]["budget"].update(granted="0"), d["scan"]["cursor"].update(granted="0"))))
        self.reject(SEQ, mutated(second_v3("BUDGET"), lambda d: d["scan"]["cursor"].update(budget_granted="3")), "K1游标预算未绑定扫描预算")
        self.reject(SEQ, mutated(second_v3("OPEN"), lambda d: d["scan"]["cursor"].update(budget_granted="64")), "K1游标预算未绑定扫描预算")

    def test_kill_witness(self):
        strict = "失效见证未严格越过候选极值"
        both = lambda change: lambda d: (change(d["facts"]["killed_by"]), change(d["scan"]["cursor"]["killed_by"]))
        self.reject(SEQ, mutated(second_v3("KILLED"), both(lambda k: k.update(comparison="<"))), strict)
        self.reject(SEQ, mutated(second_down_killed(), both(lambda k: k.update(comparison=">"))), strict)
        self.reject(SEQ, mutated(second_v3("KILLED"), lambda d: (both(lambda k: k.update(price="500"))(d), d["facts"].update(breach_watermark="500"))), strict)
        self.reject(SEQ, mutated(second_down_killed(), lambda d: (both(lambda k: k.update(price="150"))(d), d["facts"].update(breach_watermark="150"))), strict)
        self.reject(SEQ, mutated(second_v3("KILLED"), both(lambda k: k.update(candidate_extreme="499"))), strict)
        self.reject(SEQ, mutated(second_v3("KILLED"), both(lambda k: k.update(stroke_idx="99"))))
        self.reject(SEQ, mutated(second_v3("KILLED"), lambda d: d["facts"].update(breach_watermark="529")), "越界水位未绑定失效见证")
        self.reject(SEQ, mutated(second_v3("KILLED"), lambda d: d["facts"].update(killed_by=None)), "失效见证与语义终点不符")
        kill = second_v3("KILLED")["data"]["facts"]["killed_by"]
        self.reject(SEQ, mutated(second_v3("OPEN"), lambda d: d["facts"].update(killed_by=copy.deepcopy(kill))), "失效见证与语义终点不符")

    def test_cursor_binding(self):
        cur = lambda **kw: lambda d: d["scan"]["cursor"].update(**kw)
        cand = lambda **kw: lambda d: d["scan"]["cursor"]["candidate"].update(**kw)
        self.reject(SEQ, mutated(second_v3("OPEN"), lambda d: d["scan"]["cursor"].pop("consumed_digest")))
        self.reject(SEQ, mutated(second_v3("OPEN"), cur(extra="1")))
        for change in (cur(schema="second-fold-cursor/2"), cur(rule_revision="second-candidate-terminal/2"), cur(budget_policy="x")):
            self.reject(SEQ, mutated(second_v3("OPEN"), change), "K1游标版本不符")
        self.reject(SEQ, mutated(second_v3("OPEN"), cand(b_stroke="1")), "K1游标起点不是b笔之后")
        self.reject(SEQ, mutated(second_v3("OPEN"), cand(seg_dir="FLAT")), "K1候选方向未声明")
        for change in (cand(seg_dir="DOWN"), cand(candidate_extreme="601"),
                       lambda d: (cand(b_stroke="1")(d), cur(from_stroke="2")(d))):
            self.reject(SEQ, mutated(second_v3("OPEN"), change), "K1候选身份未绑定SECOND扫描")
        for change in (cur(input_stroke_count="4"), cur(consumed_len="3")):
            self.reject(SEQ, mutated(second_v3("OPEN"), change), "K1游标消费范围越界")
        for digest in ("0123456789ABCDEF", "0123456789abcde", "0123456789abcdef\n", None):
            self.reject(SEQ, mutated(second_v3("OPEN"), cur(consumed_digest=digest)), "K1游标指纹不规范")
        self.reject(SEQ, mutated(second_v3("OPEN"), cur(dir_state="SIDE")))
        self.reject(SEQ, mutated(second_v3("OPEN"), cur(semantic_state="OPEN_AT_PREFIX_END")), "K1游标封存了未决终态")
        self.reject(SEQ, mutated(second_v3("CONFIRMED"), cur(matched=None)), "K1游标见证与封存终态不符")
        kill = second_v3("KILLED")["data"]["facts"]["killed_by"]
        self.reject(SEQ, mutated(second_v3("CONFIRMED"), cur(semantic_state="KILLED_BY_NEW_EXTREME", matched=None, killed_by=dict(kill, candidate_extreme="600", price="601"))), "K1游标封存终态与扫描不同")
        self.reject(SEQ, mutated(second_v3("KILLED"), lambda d: d["scan"]["cursor"]["killed_by"].update(price="529")), "K1游标失效见证与事实不同")
        self.reject(SEQ, mutated(second_v3("CONFIRMED"), lambda d: d["scan"]["cursor"]["matched"].update(bottom_fractal=True)), "K1游标命中与事实不同")
        # 游标只封存已消费部分：已确认候选的游标仍可未封存（null）。
        self.accept(SEQ, mutated(second_v3("CONFIRMED"), cur(semantic_state=None, matched=None)))

    def test_v3_shape_is_exact(self):
        second, _ = samples()
        self.reject(SEQ, v3(second))  # v2 形状的 SECOND 换 v3 信封：缺终态/覆盖/预算/游标
        for key in ("semantic_state", "coverage", "budget", "cursor"):
            self.reject(SEQ, mutated(second_v3("OPEN"), lambda d: d["scan"].pop(key)))
        for key in ("candidate_extreme", "breach_watermark", "semantic_state", "killed_by", "coverage"):
            self.reject(SEQ, mutated(second_v3("OPEN"), lambda d: d["facts"].pop(key)))
        v2_with_v3_keys = second_v3("CONFIRMED")
        v2_with_v3_keys.update(schema_revision="s-segment/2", semantic_version="segment-second/1")
        self.reject(SEQ, v2_with_v3_keys)
        self.reject(SEQ, mutated(second, lambda d: d.update(second_sequence_status="KILLED")))
        mixed = second_v3("OPEN"); mixed["semantic_version"] = "segment-second/1"
        self.reject(SEQ, mixed, "段规则/范围版本不符")
        self.reject(SEQ, mutated(first_v3("PENDING"), lambda d: d["scan"].update(cursor=None)))

    def test_segment_results(self):
        for result in ("CASE_TWO", "KILLED_CASE_TWO", "PENDING_CASE_TWO", "PENDING_BUDGET"):
            with self.subTest(result):
                self.accept(SEG, segment_v3(result))
        _, v2_segment = samples()
        killed_v2 = mutated(v2_segment, lambda d: (d.update(result="KILLED_CASE_TWO", geometric_end=None, trigger_stroke=None, terminated_known_at=None),
                                                   d["fact_vector"].update(new_segment_formed=False, segment_broken=False, termination_witness=None)))
        self.reject(SEG, killed_v2, "冒称交付第二种终结")
        self.reject(SEG, mutated(segment_v3("KILLED_CASE_TWO"), lambda d: d.update(second_sequence_id=None, second_sequence_object_id=None)), "第二种缺SECOND引用")
        self.reject(SEG, mutated(segment_v3("KILLED_CASE_TWO"), lambda d: d.update(trigger_stroke="2")), "发展态虚填终结")
        self.reject(SEG, mutated(segment_v3("KILLED_CASE_TWO"), lambda d: d["fact_vector"].update(segment_broken=None)), "段破坏适用性不符")

    def test_segment_change_carries_v3_segment(self):
        seg = segment_v3("KILLED_CASE_TWO")
        known = seg["data"]["formed_known_at"]
        change = dict(seg, slot=[known["generation"], "segment:1:1", "updated"],
                      data=dict(entity_id="segment:1:1", change="updated", before=segment_v3("PENDING_CASE_TWO")["data"],
                                after=seg["data"], known_at=known, object_id="segment-object"))
        self.accept(CHANGE, change)

    def test_version_scope_slot_and_budget(self):
        self.accept(VER, version_v3())
        self.accept(VER, version_v3(round_strokes="0", cursors=[]))
        self.reject(VER, dict(version_v3(), slot="TB-02-D"), "段语义槽身份不符")
        self.reject(VER, mutated(version_v3(), lambda d: d.update(delivered_scope="seed_strict_first_and_finite_second",
                                                               deferred_scope=["general_equal_identity", "general_scan_continuation", "global_G006_bridge"])), "段交付范围不符")
        self.reject(VER, mutated(version_v3(), lambda d: d.pop("scan_budget")))
        self.reject(VER, mutated(version_v3(), lambda d: d.pop("scan_cursors")))
        self.reject(VER, mutated(version_v3(), lambda d: d["scan_budget"].update(policy="second-seq-scan-budget/2")), "扫描预算政策版本不符")
        self.reject(VER, mutated(version_v3(), lambda d: d["scan_budget"].update(round_strokes="-1")))
        v2 = mutated(version_v3(), lambda d: (d.update(delivered_scope="seed_strict_first_and_finite_second",
                                                       deferred_scope=["general_equal_identity", "general_scan_continuation", "global_G006_bridge"])))
        v2.update(schema_revision="s-segment/2", semantic_version="segment-second/1", slot="TB-02-D")
        self.reject(VER, v2)  # v2 不收 scan_budget/scan_cursors
        v2["data"].pop("scan_budget"); v2["data"].pop("scan_cursors")
        self.accept(VER, v2)

    def test_scan_checkpoints(self):
        kill = dict(stroke_idx="2", price="601", candidate_extreme="600", comparison=">")
        hit = second_v3("CONFIRMED")["data"]["facts"]["matched"]
        mixed = [checkpoint(), checkpoint(b_stroke="4", sealed="KILLED_BY_NEW_EXTREME", kill=kill),
                 checkpoint(seg_dir="DOWN", extreme="150", sealed="CONFIRMED", matched=hit), checkpoint(extreme="601")]
        self.accept(VER, version_v3(cursors=mixed))
        self.reject(VER, version_v3(round_strokes="0"), "预算0却登记续扫检查点")
        self.reject(VER, version_v3(cursors=[checkpoint(granted=None)]), "续扫检查点预算不符")
        self.reject(VER, version_v3(cursors=[checkpoint(), checkpoint()]), "续扫检查点候选身份重复")
        self.reject(VER, version_v3(cursors=[checkpoint(consumed="3")]), "续扫检查点预算不符")
        self.reject(VER, version_v3(cursors=[dict(checkpoint(), from_stroke="2")]), "K1游标起点不是b笔之后")
        self.reject(VER, version_v3(cursors=[checkpoint(sealed="OPEN_AT_PREFIX_END")]), "K1游标封存了未决终态")
        self.reject(VER, version_v3(cursors=[checkpoint(sealed="KILLED_BY_NEW_EXTREME")]), "K1游标见证与封存终态不符")
        self.reject(VER, version_v3(cursors=[checkpoint(sealed="KILLED_BY_NEW_EXTREME", kill=dict(kill, comparison="="))]), "失效见证比较未声明")
        self.reject(VER, version_v3(cursors=[dict(checkpoint(), consumed_digest="xyz")]), "K1游标指纹不规范")
        self.reject(VER, version_v3(cursors=[dict(checkpoint(), budget_granted="0")]))
        self.reject(VER, version_v3(cursors=[checkpoint(granted="0", consumed="0")]))
        bad_index = checkpoint(); bad_index["standard"][0]["stroke_indices"] = ["-1"]; bad_index["standard"][0]["stroke_idx"] = "-1"
        self.reject(VER, version_v3(cursors=[bad_index]))
        self.reject(VER, version_v3(cursors={"0": checkpoint()}))

    def test_links(self):
        cases = [("CASE_TWO", "CONFIRMED"), ("KILLED_CASE_TWO", "KILLED"), ("PENDING_CASE_TWO", "OPEN"), ("PENDING_BUDGET", "BUDGET")]
        for result, terminal in cases:
            with self.subTest(result):
                c.validate_segment_links(records(first_v3(TERMINALS[terminal][1]), second_v3(terminal), segment_v3(result)))
        def rejected(rs, reason):
            with self.assertRaises(ValueError) as caught:
                c.validate_segment_links(rs)
            self.assertEqual(str(caught.exception), reason)
        rejected(records(first_v3("PENDING"), second_v3("OPEN"), segment_v3("KILLED_CASE_TWO")), "段结果未绑定SECOND终态")
        rejected(records(first_v3("KILLED"), second_v3("KILLED"), segment_v3("PENDING_CASE_TWO")), "段结果未绑定SECOND终态")
        rejected(records(first_v3("KILLED"), second_v3("KILLED"), segment_v3("CASE_TWO")), "第二种终结证书未绑定同次SECOND命中")
        # 失效是完整扫描的已决结果：段侧把它写成扫描未完成须被 SECOND 的覆盖证据打回。
        unknown = mutated(segment_v3("KILLED_CASE_TWO"), lambda d: d["fact_vector"].update(new_segment_formed=None, segment_broken=None, scan_complete=False))
        self.assertEqual(self.judge(SEG, unknown), (None, None))
        rejected(records(first_v3("KILLED"), second_v3("KILLED"), unknown), "段事实可知性未绑定SECOND扫描证据")
        second, segment = samples()
        rejected(records(first_sample(second), second, v3(segment)), "段引用缺同cut序列对象")
        rejected(records(first_v3("CONFIRMED"), second_v3("CONFIRMED"), segment), "段引用缺同cut序列对象")

    def test_rule_schema_binding(self):
        def fact(kind, payload, rule):
            meta = dict(profile_id="ohlc_integer_tb02a_v1", rule_revision=rule)
            value = dict(object_revision="1", kind=kind, batch_id="batch", fact_key=c.canonical([kind, payload["slot"]]).decode(),
                         payload=payload, input_refs=[], source_coords=[], first_known_generation="1", first_known_cut="cut-1",
                         published_generation="1", withdrawn_generation=None, withdrawal_reason=None, superseded_by=None, lifecycle="active")
            identity = {k: value[k] for k in ("kind", "fact_key", "payload", "input_refs")}
            identity.update(profile_id=meta["profile_id"], rule_revision=rule)
            value["object_id"] = "obj-" + hashlib.sha256(c.canonical(identity)).hexdigest()
            return value, meta
        c.validate_fact(*fact(SEG, segment_v3("KILLED_CASE_TWO"), RULE_V3))
        c.validate_fact(*fact(VER, version_v3(), RULE_V3))
        for kind, payload, rule in [(SEG, segment_v3("KILLED_CASE_TWO"), RULE_V2), (SEG, samples()[1], RULE_V3)]:
            with self.assertRaises(ValueError) as caught:
                c.validate_fact(*fact(kind, payload, rule))
            self.assertEqual(str(caught.exception), "段合同与cut规则版本不符")
        spec = importlib.util.spec_from_file_location("tb02w_audit", ROOT / "s_query_integrity.py")
        audit = importlib.util.module_from_spec(spec); spec.loader.exec_module(audit)
        self.assertEqual(audit._published_axes(RULE_V3), c.AXES)
        self.assertIn("CC-014", audit._published_axes(RULE_V3))


class ScanCatalogTests(unittest.TestCase):
    @staticmethod
    def axes(scope):
        axes = {}
        for cid in c.AXES:
            e = dict(scope="TB-02-A", object_ids=[], waiting_reasons=[])
            if cid in ("CC-011", "CC-012", "CC-013", "CC-014"):
                e.update(scope=scope, delivered_subobligations=["finite_complete_prefix_strict_second", "independent_stroke_and_segment_break_facts"] + (["general_scan_continuation"] if scope == "TB-02-W" else []),
                    deferred_subobligations=["general_equal_identity"] + ([] if scope == "TB-02-W" else ["general_scan_continuation"]) + ["global_G006_bridge"])
            if cid == "CC-056": e.update(raw_revisions=[], withdrawals=[], replaces=[])
            axes[cid] = dict(impl_status="not_implemented", proof_status="not_proved", run_status="not_run", evidence=e)
        return axes

    def test_scope_and_obligations_follow_exact_cut_revision(self):
        for scope, rule in (("TB-02-D", RULE_V2), ("TB-02-W", RULE_V3)):
            with self.subTest(scope=scope):
                axes = self.axes(scope)
                c.validate_axes(axes, rule_revision=rule)
                with self.assertRaises(ValueError): c.validate_axes(axes, rule_revision=RULE_V3 if rule == RULE_V2 else RULE_V2)
                with self.assertRaises(ValueError): c.validate_axes(axes, rule_revision="unknown-rule")
                bad = copy.deepcopy(axes); bad["CC-014"]["impl_status"] = "implemented"
                with self.assertRaises(ValueError): c.validate_axes(bad, rule_revision=rule)
                bad = copy.deepcopy(axes); bad["CC-014"]["evidence"]["deferred_subobligations"] = []
                with self.assertRaises(ValueError): c.validate_axes(bad, rule_revision=rule)
        axes = self.axes("TB-02-W")
        axes["CC-014"]["evidence"]["scope"] = "TB-02-D"
        with self.assertRaises(ValueError): c.validate_axes(axes, rule_revision=RULE_V3)


if __name__ == "__main__":
    unittest.main()
