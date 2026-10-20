"""#1493 段合同 v4（s-segment/4，规则 `…+segment-turn/1`，xianduan.md §7.1）锁：FIRST 动作集与作用域、tie_with、
窗成员、第一种情况候选态与 a/b 配套、open_turn、PENDING_CASE_ONE、CC-013 候选态/窗成员与段端门 fail-closed。
人工构造只检合同（Python 与浏览器 JS 同输入同判定），不作段语义或正式验收。

构造取票面义务 7 的链：向上段笔 s0..s7 端点 [0,10,4,20,5,18,7,15,3]——FIRST 为下降笔 s1=[4,10]、s3=[5,20]、
s5=[7,18]、s7=[3,15]。s5 ⊂ s3 且 s3 是峰候选 ⟹ s5 记为 s3 的窗成员（前缀到 s5：PENDING_CASE_ONE）；
s7 在 b 终点一侧严格越过（3 < 5）且 15 ≤ 20 ⟹ 先破判据成立（前缀到 s7：CONFIRMED）。
"""
import copy
import json
import subprocess
import unittest
from pathlib import Path

from test_tb02c_contract import c

ROOT = Path(__file__).resolve().parents[1]
CLIENT = ROOT / "browser/tb01c-client.js"
RULE_V4 = "s2-axis-quantifiers+new-bi/2+segment-second/2+segment-turn/1"
SEQ, SEG, VER = "CC-012.feature_sequence", "CC-013.segment", "CC-056.segment_version"
PRICES = [0, 10, 4, 20, 5, 18, 7, 15, 3]
NODE = ("const a=require(process.argv[1]);let s='';process.stdin.on('data',x=>s+=x);process.stdin.on('end',()=>{"
        "try{a.validateSegmentPayload(process.argv[2],JSON.parse(s))}catch(e){process.stdout.write(e.message);process.exit(3)}})")
KNOWN = dict(generation="1", input_frontier="35", receipt_id="r", received_at="t", semantic_commit_ns=None)


def envelope(data, slot):
    return dict(schema_revision="s-segment/4", semantic_version="segment-second/2", policy_version="seed_closed_strict_second",
                view_role="main", object_generation="1", slot=slot, data=data)


def refs(through):
    return [dict(stroke_index=str(i), entity_id=f"bi:1:{4*i+1}", start_anchor=str(4*i+1), end_anchor=str(4*i+5),
                 start_price=str(PRICES[i]), end_price=str(PRICES[i+1]), start_roots=[str(4*i+1)], end_roots=[str(4*i+5)])
            for i in range(through + 1)]


def el(i, members=()):
    lo, hi = sorted((PRICES[i], PRICES[i+1]))
    return dict(high=str(hi), low=str(lo), stroke_idx=str(i), stroke_indices=[str(i)], window_members=[str(m) for m in members])


def step(incoming, before, after, action, scope="SAME_SEQUENCE", tie_with=None, merge_direction=None):
    return dict(incoming=incoming, before=before, after=after, action=action, merge_direction=merge_direction, scope=scope, tie_with=tie_with)


def first(through):
    """前缀到 s5（窗开）或 s7（先破确认）的 FIRST 序列。"""
    s1, s3 = el(1), el(3)
    steps = [step(el(1), None, el(1), "append"), step(el(3), el(1), el(3), "append"),
             step(el(5), el(3), el(3, [5]), "window_member", "TURN_HYPOTHESIS")]
    standard, checks, open_turn = [s1, el(3, [5])], [], None
    if through == 5:
        open_turn = dict(elements=[s1, el(3, [5])], turn_hypothesis_pair=["1", "3"], turn_hypothesis_state="OPEN_AT_PREFIX_END", killed_by=None)
    else:
        steps.append(step(el(7), el(3, [5]), el(7), "append"))
        standard.append(el(7))
        checks.append(dict(elements=[s1, el(3, [5]), el(7)], first_fractal=True, gap=False, gap_closed_by_c=None, provisional=False,
                           turn_hypothesis_pair=["1", "3"], turn_hypothesis_state="CONFIRMED"))
    raw = [el(i) for i in range(1, through + 1, 2)]
    return envelope(dict(entity_id="first:segment:1:1", segment_id="segment:1:1", role="FIRST_SEQUENCE", direction="UP",
                         member_direction="DOWN", stroke_refs=refs(through),
                         scan=dict(from_stroke="0", through_stroke=str(through), input_stroke_count=str(through + 1),
                                   resource_truncated=False, tail_cache_window="0"),
                         facts=dict(raw_elements=raw, standard=standard, steps=steps, checks=checks, open_turn=open_turn),
                         second_sequence_status="NOT_APPLICABLE"), "first:segment:1:1")


def construction(n):
    built = dict(start_stroke="0", stroke_indices=[str(i) for i in range(n)], length=f"n={n}" if n < 3 else "even_ge3" if n % 2 == 0 else "odd_ge3",
                 vector=None, overlap=None, overlap_relation=None, direction=None, failed_conditions=[])
    if n >= 3:
        built.update(overlap=["4", "10"], overlap_relation="OVERLAP")
    if n >= 3 and n % 2:
        built.update(vector=[True] * 4, direction="UP")
    return built


def segment_pending():
    """前缀到 s5：开窗候选 b=s3 有窗成员 s5 ⟹ PENDING_CASE_ONE。"""
    return envelope(dict(entity_id="segment:1:1", entity_revision="1", seed_id="seed:1:1", sequence_id="first:segment:1:1",
        sequence_object_id="first-object", second_sequence_id=None, second_sequence_object_id=None,
        construction=construction(6), result="PENDING_CASE_ONE", waiting_reasons=["first_kind_third_stroke_within_first"],
        termination_attempts=[], geometric_end=None, trigger_stroke=None, formed_known_at=KNOWN, terminated_known_at=None,
        stroke_refs=refs(5), strict=None,
        fact_vector=dict(stroke_broken=False, new_segment_formed=False, segment_broken=False, stroke_break_witness=None,
                         scan_from_stroke="0", scan_through_stroke="5", scan_complete=True, termination_witness=None),
        turn_hypothesis_state="OPEN_AT_PREFIX_END", window_members=["5"], parse_invalid_from=None, parse_invalid=None), "segment:1:1")


def segment_confirmed():
    """前缀到 s7：先破判据确认 ⟹ CASE_ONE，段 s0–s2，触发笔 s7，候选态 CONFIRMED、窗成员 [s5]。"""
    p = segment_pending(); d = p["data"]
    check = first(7)["data"]["facts"]["checks"][0]
    strict = dict(elements=copy.deepcopy(check["elements"]), raw_gap=False, gap_closed_by_c=None, effective_case="CASE_ONE",
                  closure_operator="<=", closure_left="3", closure_right="10")
    attempt = dict(end_stroke="2", trigger_stroke="7", minimum_count_met=True, construction=construction(3))
    d.update(construction=construction(3), result="CASE_ONE", waiting_reasons=[], termination_attempts=[attempt],
             geometric_end=dict(stroke_index="2", group_anchor="13", extreme_roots=["13"], price="20"), trigger_stroke="7",
             terminated_known_at=KNOWN, stroke_refs=refs(7), strict=strict, turn_hypothesis_state="CONFIRMED", window_members=["5"])
    d["fact_vector"] = dict(stroke_broken=True, new_segment_formed=True, segment_broken=True,
        stroke_break_witness=dict(earlier_stroke="0", later_stroke="3", earlier_price="10", later_price="5", comparison="<="),
        scan_from_stroke="0", scan_through_stroke="7", scan_complete=True,
        termination_witness=dict(kind="CASE_ONE", strict=copy.deepcopy(strict), second_match=None, construction=construction(3),
                                 minimum_count_met=True, through_stroke="7"))
    return p


def invalid_repro(start="0"):
    return dict(seg_start=start, end_stroke="4", construction_vector=[True, True, False, True], failed_conditions=["top_not_above_bottom"],
                elements=[dict(high=h, low=l, stroke_idx=i, stroke_indices=[i], window_members=[]) for h, l, i in (("10", "4", "1"), ("20", "5", "3"), ("18", "7", "5"))])


def segment_invalid():
    """段端门失败（fail-closed）：末次终结尝试满足最少笔数但构造不完整 ⟹ FAILED_GEOMETRY，附最小复现。"""
    p = segment_pending(); d = p["data"]
    bad = construction(5); bad.update(vector=[True, True, False, True], direction=None, failed_conditions=["top_not_above_bottom"])
    d.update(result="FAILED_GEOMETRY", waiting_reasons=["top_not_above_bottom"], turn_hypothesis_state=None, window_members=[],
             termination_attempts=[dict(end_stroke="4", trigger_stroke="5", minimum_count_met=True, construction=bad)],
             parse_invalid_from="0", parse_invalid=invalid_repro())
    return p


def version(waiting=()):
    return envelope(dict(input_frontier="35", waiting_reasons=list(waiting), known_at=KNOWN,
                         delivered_scope="seed_strict_first_and_second_terminal", deferred_scope=["general_equal_identity", "global_G006_bridge"],
                         scan_budget=dict(policy="second-seq-scan-budget/1", round_strokes="0"), scan_cursors=[]), "TB-02-W")


def records(first_payload, segment_payload, version_payload=None):
    out = [dict(object_id="first-object", kind=SEQ, payload=first_payload, source_coords=["1"]),
           dict(object_id="segment-object", kind=SEG, payload=segment_payload, source_coords=["1"])]
    if version_payload is not None:
        out.append(dict(object_id="version-object", kind=VER, payload=version_payload, source_coords=["1"]))
    return out


def mutated(p, change):
    p = copy.deepcopy(p)
    change(p["data"])
    return p


class TB02XContractTest(unittest.TestCase):
    def judge(self, kind, p):
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

    def reject(self, kind, p, reason):
        self.assertEqual(self.judge(kind, p), (reason, reason))

    def test_rule_binds_v4_schema(self):
        self.assertEqual(c.RULE_CONTRACTS[RULE_V4], ("s-new-bi/2", "s-segment/4", c.KNOWLEDGE_DOMAIN_V2))

    def test_v4_samples_accept(self):
        for kind, p in ((SEQ, first(5)), (SEQ, first(7)), (SEG, segment_pending()), (SEG, segment_confirmed()), (SEG, segment_invalid()), (VER, version())):
            with self.subTest(kind=kind, result=p["data"].get("result")):
                self.accept(kind, p)

    def test_missing_v4_keys_rejected(self):
        cases = [
            (SEQ, first(5), lambda d: d["facts"]["standard"][0].pop("window_members"), "FIRST元素"),
            (SEQ, first(5), lambda d: d["facts"]["steps"][0].pop("tie_with"), "包含步骤"),
            (SEQ, first(7), lambda d: d["facts"]["checks"][0].pop("turn_hypothesis_state"), "分型见证"),
            (SEQ, first(5), lambda d: d["facts"].pop("open_turn"), "序列事实"),
            (SEG, segment_pending(), lambda d: d.pop("parse_invalid"), SEG),
            (SEG, segment_pending(), lambda d: d.pop("turn_hypothesis_state"), SEG),
        ]
        for kind, p, change, name in cases:
            with self.subTest(name):
                py, js = self.judge(kind, mutated(p, change))
                self.assertEqual(py, name + " 字段缺失/多余")
                self.assertEqual(js, name + " 完整字段不符")

    def test_v3_shape_is_not_v4(self):
        """旧代（无窗成员键、旧 turn_boundary）不得冒充 v4；v4 也不收旧动作。"""
        def old_action(d):
            d["facts"]["steps"][2].update(action="turn_boundary")
        self.assertEqual(self.judge(SEQ, mutated(first(5), old_action)), ("包含跨作用域或动作未声明", "包含跨作用域/动作未声明"))

    def test_action_scope_and_merge_direction(self):
        cases = [
            (lambda d: d["facts"]["steps"][2].update(scope="SAME_SEQUENCE"), "包含跨作用域或动作未声明", "包含跨作用域/动作未声明"),
            (lambda d: d["facts"]["steps"][1].update(scope="TURN_HYPOTHESIS"), "包含跨作用域或动作未声明", "包含跨作用域/动作未声明"),
            (lambda d: d["facts"]["steps"][2].update(merge_direction="UP"), "方向性包含缺见证", "方向性包含缺见证"),
        ]
        for change, py, js in cases:
            self.assertEqual(self.judge(SEQ, mutated(first(5), change)), (py, js))

    def test_tie_with_binding(self):
        def tie(value):
            def change(d):
                d["facts"]["steps"][1].update(action="tie_pair", scope="TURN_HYPOTHESIS", tie_with=value)
            return change
        self.accept(SEQ, mutated(first(5), tie("1")))
        self.reject(SEQ, mutated(first(5), tie("3")), "tie_with未绑定末标准元素")
        self.reject(SEQ, mutated(first(5), tie(None)), "tie_with只属并列对")
        self.reject(SEQ, mutated(first(5), lambda d: d["facts"]["steps"][1].update(tie_with="1")), "tie_with只属并列对")

    def test_window_member_step_keeps_b(self):
        def moved(d):
            d["facts"]["steps"][2]["after"]["low"] = "7"
        def unrecorded(d):
            d["facts"]["steps"][2]["after"]["window_members"] = []
        for change in (moved, unrecorded):
            self.reject(SEQ, mutated(first(5), change), "窗成员步改写了b或未记入成员")
        def before_member(d):
            d["facts"]["standard"][1]["window_members"] = ["1"]
            d["facts"]["open_turn"]["elements"][1]["window_members"] = ["1"]
        self.reject(SEQ, mutated(first(5), before_member), "窗成员不在b之后或混入合并来源")

    def test_turn_state_pair_binding(self):
        def state(value, fractal=None, pair="keep"):
            def change(d):
                check = d["facts"]["checks"][0]
                check["turn_hypothesis_state"] = value
                if fractal is not None:
                    check["first_fractal"] = fractal
                    if not fractal:
                        check["gap"] = None
                if pair != "keep":
                    check["turn_hypothesis_pair"] = pair
            return change
        self.accept(SEQ, mutated(first(7), state("KILLED_BY_NEW_EXTREME", False)))
        self.accept(SEQ, mutated(first(7), state("NOT_CASE_ONE", False, None)))
        self.reject(SEQ, mutated(first(7), state("KILLED_BY_NEW_EXTREME", False, None)), "假设转折点两侧未绑定a/b元素")
        self.reject(SEQ, mutated(first(7), state("NOT_CASE_ONE", False)), "假设转折点两侧未绑定a/b元素")
        self.reject(SEQ, mutated(first(7), state("CONFIRMED", False)), "候选态与分型事实不配套")
        self.reject(SEQ, mutated(first(7), state("OPEN_AT_PREFIX_END")), "第一种情况候选态未声明")
        self.reject(SEQ, mutated(first(7), lambda d: d["facts"]["checks"][0].update(provisional=True)), "候选态与分型事实不配套")

    def test_open_turn_binding(self):
        self.reject(SEQ, mutated(first(5), lambda d: d["facts"]["open_turn"].update(turn_hypothesis_state="CONFIRMED")), "开窗候选未绑定末两标准元素")
        self.reject(SEQ, mutated(first(5), lambda d: d["facts"]["open_turn"].update(turn_hypothesis_pair=["1", "5"])), "开窗候选未绑定末两标准元素")
        self.reject(SEQ, mutated(first(5), lambda d: d["facts"]["open_turn"]["elements"][1].update(window_members=[])), "开窗候选未绑定末两标准元素")

    def test_open_turn_kill_witness(self):
        # 审查发现：段方向笔越过 P 而下一反向笔未进序列时，开窗候选即作废并带见证。
        def kill(price, comparison=">", idx="4", state="KILLED_BY_NEW_EXTREME"):
            return lambda d: d["facts"]["open_turn"].update(turn_hypothesis_state=state, killed_by=dict(stroke_idx=idx, price=price, comparison=comparison))
        self.accept(SEQ, mutated(first(5), kill("21")))
        self.reject(SEQ, mutated(first(5), kill("20")), "开窗作废见证未严格越过P")
        self.reject(SEQ, mutated(first(5), kill("21", "<")), "开窗作废见证未严格越过P")
        self.reject(SEQ, mutated(first(5), kill("21", idx="2")), "开窗作废见证未严格越过P")
        self.reject(SEQ, mutated(first(5), kill("21", state="OPEN_AT_PREFIX_END")), "开窗候选作废见证与状态不符")
        self.reject(SEQ, mutated(first(5), lambda d: d["facts"]["open_turn"].update(turn_hypothesis_state="KILLED_BY_NEW_EXTREME")), "开窗候选作废见证与状态不符")

    def test_pending_case_one_requires_window(self):
        self.reject(SEG, mutated(segment_pending(), lambda d: d.update(window_members=[])), "PENDING_CASE_ONE缺开窗候选或窗成员")
        self.reject(SEG, mutated(segment_pending(), lambda d: d.update(waiting_reasons=[])), "候选态/窗成员与等待原因不配套")
        self.reject(SEG, mutated(segment_pending(), lambda d: d.update(turn_hypothesis_state=None)), "候选态/窗成员与等待原因不配套")
        self.reject(SEG, mutated(segment_pending(), lambda d: d.update(turn_hypothesis_state="CONFIRMED")), "PENDING_CASE_ONE缺开窗候选或窗成员")
        self.reject(SEG, mutated(segment_confirmed(), lambda d: d.update(turn_hypothesis_state="KILLED_BY_NEW_EXTREME")), "终结与第一种情况候选态不符")
        self.reject(SEG, mutated(segment_pending(), lambda d: d.update(turn_hypothesis_state="NOT_CASE_ONE")), "段候选态未声明")

    def test_minimum_length_is_not_geometry_failure(self):
        def short(d):
            d["termination_attempts"] = [dict(end_stroke="2", trigger_stroke="5", minimum_count_met=False, construction=construction(3))]
        self.reject(SEG, mutated(segment_pending(), short), "最少笔数等待原因未绑定末次终结尝试")
        def short_waiting(d):
            short(d); d["waiting_reasons"].append("minimum_segment_length_not_met")
        self.accept(SEG, mutated(segment_pending(), short_waiting))
        def short_failed(d):
            short(d); d.update(result="FAILED_GEOMETRY", waiting_reasons=["minimum_segment_length_not_met"], turn_hypothesis_state=None, window_members=[])
        self.reject(SEG, mutated(segment_pending(), short_failed), "最少笔数不足或构造完整却称几何失败")

    def test_parse_invalid_repro_shape(self):
        self.reject(SEG, mutated(segment_invalid(), lambda d: d.update(parse_invalid_from="1")), "段端门失败复现未绑定候选起点")
        self.reject(SEG, mutated(segment_invalid(), lambda d: d["parse_invalid"].update(construction_vector=[True] * 4)), "段端门失败复现构造完整")
        self.reject(SEG, mutated(segment_invalid(), lambda d: d["parse_invalid"]["elements"].pop()), "段端门失败复现缺分型三元素")

    def test_links_bind_pending_window_to_first(self):
        c.validate_segment_links(records(first(5), segment_pending()))
        c.validate_segment_links(records(first(7), segment_confirmed()))
        with self.assertRaisesRegex(ValueError, "PENDING_CASE_ONE未绑定FIRST开窗候选"):
            c.validate_segment_links(records(first(7), segment_pending()))
        with self.assertRaisesRegex(ValueError, "段候选态/窗成员未绑定FIRST检查"):
            c.validate_segment_links(records(first(7), mutated(segment_confirmed(), lambda d: d.update(window_members=[]))))

    def test_parse_invalid_blocks_in_version_and_axes(self):
        axes = {cid: dict(evidence=dict(waiting_reasons=["segment_parse_invalid"])) for cid in ("CC-011", "CC-012", "CC-013", "CC-014")}
        c.validate_segment_links(records(first(5), segment_invalid(), version(["segment_parse_invalid", "top_not_above_bottom"])))
        with self.assertRaisesRegex(ValueError, "段端门失败未在段版本标阻断"):
            c.validate_segment_links(records(first(5), segment_invalid(), version(["top_not_above_bottom"])))
        with self.assertRaisesRegex(ValueError, "段端门失败未在段版本标阻断"):
            c.validate_segment_links(records(first(5), segment_invalid()))
        quiet = copy.deepcopy(axes); quiet["CC-013"]["evidence"]["waiting_reasons"] = []
        with self.assertRaisesRegex(ValueError, "段端门失败未在逐轴证据标阻断"):
            c.validate_segment_links(records(first(5), segment_invalid(), version(["segment_parse_invalid"])), {**quiet})


if __name__ == "__main__":
    unittest.main()
