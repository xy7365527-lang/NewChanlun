"""#1768×#1419 P2（issuecomment-6504760769）：旧 SQLite 校验链（s-session/2 只读历史）的规则分派锁。

覆盖面只限 s_query_integrity 的两个真实分派入口：目录轴入口 _published_axes 与事实合同入口
_project_canonical_fact（经 s_tb02_contract._validate_fact 消费 meta.rule_revision）。
1. 已登记历史串：期望轴与（新笔合同, 段合同, 知识域）三元组逐条显式写在本文件，不从被测表生成；
   两入口都以真实输入执行，合法输入通过、错合同输入以对应原因拒绝。
2. S3 的 W/1、W/2 发布身份及其坏序、重复、混版本、剥后缀变体：两入口都以「cut规则修订未声明」拒绝，不回退。
3. #1393 v2 W/1 历史读域（PR1793 RULE_BASES/base_rule）：未落地时两入口精确拒绝；落地后按下方显式期望读。
4. 篡改 meta.rule_revision 后热 memo 与冷读同拒（batch 根绑定），恢复原串摘要恢复。
S3 socket/controller → 正式 Q3 路线不在本文件；那条路线的版本兼容由 S3 公共验证器及三动态承接。
"""
import hashlib
import sqlite3
import tempfile
import unittest
from contextlib import closing
from pathlib import Path

from tb01c_query_tests import audit, load_frozen, query, reader, resources
from test_tb02b_contract import V1 as BI_V1, V2 as BI_V2, as_v1, endpoint
from test_tb02d_contract import samples

tb02 = audit.tb02

LEGACY = ("CC-001", "CC-002", "CC-003", "CC-004", "CC-005", "CC-006", "CC-007", "CC-054", "CC-056")
BI = LEGACY + ("CC-008", "CC-009", "CC-010", "CC-055")
FIRST_SEGMENT = BI + ("CC-011", "CC-012", "CC-013")
FULL = FIRST_SEGMENT + ("CC-014",)
DOMAIN_V1 = "established_direction_without_extreme_identity_competition"
DOMAIN_V2 = "established_direction_with_extreme_root_sets"
DOMAIN_W = "raw_adjacent_W_initialization"

# 规则串 → (目录轴, 新笔合同, 段合同, TB-02-A 知识域)。来源：#1405 知识域两代与新笔合同、#1420 s-segment/3、
# #1493 xianduan §7.1 第 7 条 s-segment/4、#1604 second-seq-scan-budget/2 → s-segment/5；轴取 #1405/#1493/#1604 已签目录。
HISTORY = {
    "s2-axis-quantifiers": (LEGACY, None, None, DOMAIN_V1),
    "s2-axis-quantifiers+new-bi/1": (BI, "s-new-bi/1", None, DOMAIN_V1),
    "s2-axis-quantifiers+new-bi/2": (BI, "s-new-bi/2", None, DOMAIN_V2),
    "s2-axis-quantifiers+new-bi/1+segment-first/1": (FIRST_SEGMENT, "s-new-bi/1", "s-segment/1", DOMAIN_V1),
    "s2-axis-quantifiers+new-bi/1+segment-second/1": (FULL, "s-new-bi/1", "s-segment/2", DOMAIN_V1),
    "s2-axis-quantifiers+new-bi/2+segment-second/1": (FULL, "s-new-bi/2", "s-segment/2", DOMAIN_V2),
    "s2-axis-quantifiers+new-bi/2+segment-second/2": (FULL, "s-new-bi/2", "s-segment/3", DOMAIN_V2),
    "s2-axis-quantifiers+new-bi/2+segment-second/2+segment-turn/1": (FULL, "s-new-bi/2", "s-segment/4", DOMAIN_V2),
    "s2-axis-quantifiers+new-bi/2+segment-second/2+segment-turn/1+second-seq-scan-budget/2":
        (FULL, "s-new-bi/2", "s-segment/5", DOMAIN_V2),
}
HISTORICAL_V2_RULES = tuple(HISTORY)
# #1393 v2 W/1 历史读域：PR1793@6da53d1 s_tb02_contract.py RULE_BASES 显式映射到 segment-second/1 基规则，
# 知识域取 W 冻结源 W_KNOWLEDGE_DOMAIN，CC-004/005/054 带 raw-initialization-w/1 初始化字段（#1393 合同 test_tb02u_contract）。
W1_V2_HISTORY = "s2-axis-quantifiers+new-bi/1+segment-second/1+raw-initialization-w/1"
W1_V2_EXPECTED = (FULL, "s-new-bi/1", "s-segment/2", DOMAIN_W)
W1_V2_LANDED = hasattr(tb02, "RULE_BASES")

S3_BASE = ("s2-axis-quantifiers+new-bi/2+segment-second/2+segment-turn/1+second-seq-scan-budget/2"
           "+center-seed/1+center-pair/1")
W1_S3 = S3_BASE + "+raw-initialization-w/1"
W2 = S3_BASE + "+raw-initialization-w/2+bounded-layout/1"
W2_VARIANTS = (
    W2,
    W1_S3,
    S3_BASE,
    S3_BASE + "+raw-initialization-w/2",
    S3_BASE + "+bounded-layout/1+raw-initialization-w/2",
    S3_BASE + "+raw-initialization-w/2+bounded-layout/2",
    S3_BASE + "+raw-initialization-w/1+bounded-layout/1",
    S3_BASE + "+raw-initialization-w/1+raw-initialization-w/2+bounded-layout/1",
    W2 + "+bounded-layout/1",
    W2 + "+raw-initialization-w/2",
    W2.replace("center-seed/1+center-pair/1", "center-pair/1+center-seed/1"),
    HISTORICAL_V2_RULES[-1] + "+raw-initialization-w/2+bounded-layout/1",
    HISTORICAL_V2_RULES[-1] + "+raw-initialization-w/2",
    "s2-axis-quantifiers+new-bi/2+segment-second/1+raw-initialization-w/1",
    W1_V2_HISTORY + "+bounded-layout/1",
    W2 + " ",
    "",
    "unknown",
)
RAW = [dict(identity_key="raw-key", payload_hash="a" * 64, receipt_id="receipt", event_id="event",
            input_revision="1", revision="1", seq="0", source_coord="0")]
UNDECLARED = "cut规则修订未声明"
BI_MISMATCH = "新笔合同与cut规则版本不符"
SEGMENT_MISMATCH = "段合同与cut规则版本不符"
DOMAIN_MISMATCH = "TB-02-A 知识域与cut规则版本不符"


def knowledge(domain, w_fields=False):
    payload = dict(scope="TB-02-A", input_frontier="0", known_facts=["raw_ohlc", "inclusion_trace", "group_provenance"],
                   waiting_reasons=[], domain=domain, requests=[])
    if w_fields:
        payload.update(initialization_policy_revision="raw-initialization-w/1", direction_state="Undetermined")
    return "CC-054.knowledge_state", "TB-02-A", payload, RAW


def bi_endpoint(schema):
    top = endpoint("TOP", "5", "13", ["5", "6"], "5", "6", ["4", "5", "6", "7"])
    version = BI_V2 if schema == "s-new-bi/2" else BI_V1
    data = top if schema == "s-new-bi/2" else as_v1(top, "5")
    payload = dict(**version, view_role="main", object_generation="1", slot="fx:1:5", data=data)
    return "CC-008.endpoint", payload["slot"], payload, []


def segment_s2():
    payload = samples()[1]
    return "CC-013.segment", payload["slot"], payload, RAW


def row_for(rule, material):
    """真实 structure_facts 行；内容身份按该 cut 规则闭合，使规则合同成为唯一可能拒因。"""
    kind, slot, payload, refs = material
    fact_key = tb02.canonical([kind, slot]).decode()
    meta = dict(profile_id=tb02.PROFILE, rule_revision=rule)
    identity = dict(kind=kind, fact_key=fact_key, payload=payload, input_refs=refs, **meta)
    row = dict(object_id="obj-" + hashlib.sha256(tb02.canonical(identity)).hexdigest(), object_revision=1,
               kind=kind, batch_id="batch-1", fact_key=fact_key, payload_json=tb02.canonical(payload).decode(),
               input_refs_json=tb02.canonical(refs).decode(), source_coords_json=tb02.canonical(
                   [ref["source_coord"] for ref in refs]).decode(),
               first_known_generation=1, first_known_cut="cut-1", published_generation=1,
               withdrawn_generation=None, withdrawal_reason=None, superseded_by=None)
    return row, meta


def project(rule, material):
    row, meta = row_for(rule, material)
    return audit._project_canonical_fact(row, meta, RAW)


class W2ReaderRuleDispatchTests(unittest.TestCase):
    def assert_contract(self, rule, expected):
        axes, bi_schema, segment_schema, domain = expected
        self.assertEqual(audit._published_axes(rule), axes)
        w_fields = domain == DOMAIN_W
        self.assertEqual(project(rule, knowledge(domain, w_fields))["payload"]["domain"], domain)
        # 载荷层只认与初始化字段同族的知识域；跨族在载荷层先拒，不是规则分派证据。
        for other in ({DOMAIN_W} if w_fields else {DOMAIN_V1, DOMAIN_V2}) - {domain}:
            with self.assertRaisesRegex(ValueError, DOMAIN_MISMATCH):
                project(rule, knowledge(other, w_fields))
        for schema in ("s-new-bi/1", "s-new-bi/2"):
            if schema == bi_schema:
                self.assertEqual(project(rule, bi_endpoint(schema))["payload"]["schema_revision"], schema)
            else:
                with self.assertRaisesRegex(ValueError, BI_MISMATCH):
                    project(rule, bi_endpoint(schema))
        if segment_schema == "s-segment/2":
            self.assertEqual(project(rule, segment_s2())["payload"]["schema_revision"], "s-segment/2")
        else:
            with self.assertRaisesRegex(ValueError, SEGMENT_MISMATCH):
                project(rule, segment_s2())

    def assert_undeclared(self, rule):
        with self.assertRaisesRegex(ValueError, UNDECLARED):
            audit._published_axes(rule)
        materials = [knowledge(DOMAIN_V1), knowledge(DOMAIN_V2), bi_endpoint("s-new-bi/1"),
                     bi_endpoint("s-new-bi/2"), segment_s2()]
        if W1_V2_LANDED:
            materials.append(knowledge(DOMAIN_W, True))
        for material in materials:
            with self.assertRaisesRegex(ValueError, UNDECLARED):
                project(rule, material)

    def test_registered_v2_history_keeps_exact_contracts(self):
        self.assertEqual({rule: tb02.RULE_CONTRACTS[rule] for rule in tb02.RULE_CONTRACTS},
                         {rule: value[1:] for rule, value in HISTORY.items()})
        for rule, expected in HISTORY.items():
            with self.subTest(rule=rule):
                self.assert_contract(rule, expected)

    def test_w1_v2_history_read_domain(self):
        if W1_V2_LANDED:
            self.assertEqual(tb02.RULE_BASES, {**{rule: rule for rule in HISTORY},
                                               W1_V2_HISTORY: "s2-axis-quantifiers+new-bi/1+segment-second/1"})
            self.assert_contract(W1_V2_HISTORY, W1_V2_EXPECTED)
        else:
            self.assert_undeclared(W1_V2_HISTORY)

    def test_w2_and_malformed_variants_rejected_at_both_dispatch_points(self):
        for rule in W2_VARIANTS:
            with self.subTest(rule=rule):
                self.assert_undeclared(rule)


class W2ReaderMemoAcrossRuleTests(unittest.TestCase):
    """篡改已封存版本绑定后热缓存不得放行：g4 为 tick profile，拒因是 batch 根绑定，不是跨版本正向读。"""

    def setUp(self):
        directory = tempfile.TemporaryDirectory(prefix="tb02w2-memo-")
        self.addCleanup(directory.cleanup)
        self.base = Path(directory.name)
        self.db = self.base / "g4.sqlite"
        load_frozen("g4", self.db)

    def _reader(self, **kwargs):
        value = query.AuditReader(self.db, resources(), vars(reader), **kwargs)
        self.addCleanup(value.close)
        return value

    def _set_rule(self, rule):
        with closing(sqlite3.connect(self.db)) as conn, conn:
            conn.execute("UPDATE meta SET value=? WHERE key='rule_revision'", (rule,))

    def _verdict(self, live, memo=None):
        if memo is not None:
            live.memo = memo
        try:
            return "ok", live.capture_verified()["capture_digest"]
        except ValueError as error:
            return "rejected", str(error)

    def test_hot_memo_rejects_tampered_rule_binding_like_cold_read(self):
        with closing(reader.open_readonly(self.db)) as conn:
            original = conn.execute("SELECT value FROM meta WHERE key='rule_revision'").fetchone()[0]
        self.assertIn(original, HISTORICAL_V2_RULES)
        live = self._reader()
        first = live.capture_verified()
        self.assertGreater(first["_audit_memo"]["misses"], 0)
        for rule in (W2, W1_S3):
            with self.subTest(rule=rule):
                self._set_rule(rule)
                hot = self._verdict(live, first["_audit_memo"])
                cold = self._verdict(self._reader(use_memo=False))
                self.assertEqual(hot, ("rejected", "batch 封存坐标/规则与根不一致"))
                self.assertEqual(hot, cold)
        self._set_rule(original)
        restored = self._reader(use_memo=False).capture_verified()
        self.assertEqual(restored["capture_digest"], first["capture_digest"])


if __name__ == "__main__":
    unittest.main()
