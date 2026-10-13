"""#1419 TB-02-E 中枢合同（schema v1.2）：共享语料逐条判定，并与浏览器端 JS 逐条对拍结论与原因码。

语料 fixtures/tb02e-contract/corpus.json 由同目录 generate-corpus.py 按定义直算生成（不读生产输出）；本测试只检
合同门，不作中枢语义或正式运行验收。node 不可用时失败，不跳过。
"""
import json
import shutil
import subprocess
import sys
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT))
import s_tb02_contract as c  # noqa: E402

CORPUS = ROOT / "tests" / "fixtures" / "tb02e-contract" / "corpus.json"
JS_JUDGE = r"""
const API = require(process.argv[1]); const corpus = JSON.parse(require('fs').readFileSync(process.argv[2], 'utf8'));
const bases = Object.fromEntries(corpus.cases.filter(c => c.base === null).map(c => [c.name, c]));
const out = {};
for (const c of corpus.cases) {
  let records = c.records, axes = c.axes;
  if (c.base !== null) {
    const b = bases[c.base], seen = new Set(); records = [];
    for (const r of b.records) {
      if (c.remove.includes(r.object_id)) continue;
      records.push(Object.hasOwn(c.replace, r.object_id) && !seen.has(r.object_id) ? c.replace[r.object_id] : r); seen.add(r.object_id);
    }
    records.push(...c.add); axes = c.axes ?? b.axes;
  }
  try { API.validateCenterCut(records, c.rule_revision, axes); out[c.name] = 'ACCEPT'; }
  catch (e) { out[c.name] = e.message.startsWith('TB02E_') ? e.message : 'UNCODED:' + e.message; }
}
process.stdout.write(JSON.stringify(out));
"""


def python_verdicts():
    corpus = json.loads(CORPUS.read_text(encoding="utf-8"))
    out = {}
    for name, _expected, rule, records, axes in c.expand_center_corpus(corpus):
        try:
            c.validate_center_cut(records, rule, axes)
            out[name] = "ACCEPT"
        except ValueError as err:
            text = str(err)
            out[name] = text if text.startswith("TB02E_") else "UNCODED:" + text
    return corpus, out


def code(verdict):
    """完整拒因 `TB02E_<CODE>: <文>` → `<CODE>`；ACCEPT/UNCODED 原样。"""
    return verdict.split(":")[0][len("TB02E_"):] if verdict.startswith("TB02E_") else verdict


def js_verdicts():
    node = shutil.which("node")
    if node is None:
        raise AssertionError("node 不可用：跨语言对拍不得跳过")
    proc = subprocess.run([node, "-e", JS_JUDGE, str(ROOT / "browser" / "tb01c-client.js"), str(CORPUS)],
                          capture_output=True, text=True, timeout=120)
    if proc.returncode != 0:
        raise AssertionError(proc.stderr)
    return json.loads(proc.stdout)


class CenterContractCorpus(unittest.TestCase):
    def test_python_judges_every_case_as_expected(self):
        corpus, got = python_verdicts()
        cases = corpus["cases"]
        self.assertGreaterEqual(sum(case["expected"] == "ACCEPT" for case in cases), 7)
        self.assertGreaterEqual(sum(case["expected"] != "ACCEPT" for case in cases), 20)
        for case in cases:
            with self.subTest(case=case["name"]):
                self.assertEqual(code(got[case["name"]]), case["expected"])

    def test_python_and_js_agree_case_by_case(self):
        """两侧逐条比完整拒因（原因码与拒因文字同文，schema v1.2 §6），不只比原因码。"""
        _corpus, py = python_verdicts()
        js = js_verdicts()
        self.assertEqual(sorted(py), sorted(js))
        for name in py:
            with self.subTest(case=name):
                self.assertEqual(js[name], py[name])

    def test_corpus_covers_required_rejections(self):
        corpus = json.loads(CORPUS.read_text(encoding="utf-8"))
        codes = {case["expected"] for case in corpus["cases"]}
        required = {"CONNECTOR_AS_MEMBER", "CONNECTOR", "PLUS_ONE_REUSE", "RESUME_ANCHOR", "ROLE_VALUE", "ROLES_SHAPE",
                    "CONTRIBUTORS", "CANDIDATE_KIND", "MEMBER_NOT_COMPLETED", "UNIT_SOURCE", "EXCLUSION_REASON", "RULE_MIX",
                    "VERSION", "NOT_ALTERNATING", "BREAK_UP_UNDEFINED", "ASSIGNMENT", "ASSIGNMENT_CONSISTENCY", "RESUME_GAP",
                    "ROLE_NULL", "AXIS_BINDING", "SCAN_CURSOR", "MEMBER_EDGE", "MEMBER_EDGE_COUNT", "BREAK_UP"}
        self.assertEqual(required - codes, set())

    def test_old_rule_axes_and_segment_schema_unchanged(self):
        self.assertEqual(c.SEGMENT_SECOND_AXES[-1], "CC-014")
        self.assertEqual(c.CENTER_AXES, c.SEGMENT_SECOND_AXES + ("CC-016", "CC-017", "CC-022"))
        self.assertTrue(set(c.CENTER_KINDS) <= set(c.KINDS))


if __name__ == "__main__":
    unittest.main()
