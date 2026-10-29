"""#1768×#1419 P2（issuecomment-6504760769）：九 Python 读者面对 W/2 发布身份的版本分派锁。

Python 公共读者只读 s-session/2 结构库；W/2 完整串是 S3 写者唯一新写身份（含 center/bounded-layout 组件），
不在 v2 合同表内。本文件锁三件事：已登记 v2 历史串逐条保持原轴/合同；W/2 串及其坏序、重复、混版本、
剥后缀变体在两个分派点都被精确拒绝而不回退；同一活 reader 的热 memo 跨规则版本与冷读同判。
"""
import sqlite3
import tempfile
import unittest
from contextlib import closing
from pathlib import Path

from tb01c_query_tests import audit, load_frozen, query, reader, resources

tb02 = audit.tb02

HISTORICAL_V2_RULES = (
    "s2-axis-quantifiers",
    "s2-axis-quantifiers+new-bi/1",
    "s2-axis-quantifiers+new-bi/2",
    "s2-axis-quantifiers+new-bi/1+segment-first/1",
    "s2-axis-quantifiers+new-bi/1+segment-second/1",
    "s2-axis-quantifiers+new-bi/2+segment-second/1",
    "s2-axis-quantifiers+new-bi/2+segment-second/2",
    "s2-axis-quantifiers+new-bi/2+segment-second/2+segment-turn/1",
    "s2-axis-quantifiers+new-bi/2+segment-second/2+segment-turn/1+second-seq-scan-budget/2",
)
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
    W2 + " ",
    "",
    "unknown",
)


class W2ReaderRuleDispatchTests(unittest.TestCase):
    def test_registered_v2_history_keeps_exact_contracts(self):
        self.assertEqual(set(tb02.RULE_CONTRACTS), set(HISTORICAL_V2_RULES))
        for rule in HISTORICAL_V2_RULES:
            with self.subTest(rule=rule):
                self.assertIsInstance(audit._published_axes(rule), tuple)
                self.assertIn(tb02.RULE_CONTRACTS[rule][2],
                              (tb02.KNOWLEDGE_DOMAIN_V1, tb02.KNOWLEDGE_DOMAIN_V2))

    def test_w2_and_malformed_variants_rejected_at_both_dispatch_points(self):
        for rule in W2_VARIANTS:
            with self.subTest(rule=rule):
                self.assertNotIn(rule, tb02.RULE_CONTRACTS)
                with self.assertRaisesRegex(ValueError, "cut规则修订未声明"):
                    audit._published_axes(rule)


class W2ReaderMemoAcrossRuleTests(unittest.TestCase):
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

    def test_hot_memo_from_registered_rule_does_not_admit_w2(self):
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
                self.assertEqual(hot[0], "rejected")
                self.assertEqual(hot, cold)
        self._set_rule(original)
        restored = self._reader(use_memo=False).capture_verified()
        self.assertEqual(restored["capture_digest"], first["capture_digest"])


if __name__ == "__main__":
    unittest.main()
