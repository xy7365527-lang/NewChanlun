"""#1424 TB-02-F 中枢对合同（schema s-center-pair/1）：共享语料逐条判定，并与浏览器端 JS 逐条对拍完整拒因。

语料 fixtures/tb02f-contract/corpus.json 由同目录 generate-corpus.py 按定义直算生成（不读生产输出）；本测试只检
合同门，不作中枢对语义或正式运行验收。node 不可用时失败，不跳过。
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

CORPUS = ROOT / "tests" / "fixtures" / "tb02f-contract" / "corpus.json"
PREFIXES = ("TB02E_", "TB02F_")
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
  try { API.validatePairCut(records, c.rule_revision, axes); out[c.name] = 'ACCEPT'; }
  catch (e) { out[c.name] = /^TB02[EF]_/.test(e.message) ? e.message : 'UNCODED:' + e.message; }
}
process.stdout.write(JSON.stringify(out));
"""


def python_verdicts():
    corpus = json.loads(CORPUS.read_text(encoding="utf-8"))
    out = {}
    for name, _expected, rule, records, axes in c.expand_center_corpus(corpus):
        try:
            c.validate_pair_cut(records, rule, axes)
            out[name] = "ACCEPT"
        except ValueError as err:
            text = str(err)
            out[name] = text if text.startswith(PREFIXES) else "UNCODED:" + text
    return corpus, out


def code(verdict):
    """对门用短码；中枢门保留 TB02E_ 前缀，区分整 cut 在哪一层拒绝。"""
    return verdict.split(":")[0][len("TB02F_"):] if verdict.startswith("TB02F_") else verdict.split(":")[0]


def js_verdicts():
    node = shutil.which("node")
    if node is None:
        raise AssertionError("node 不可用：跨语言对拍不得跳过")
    proc = subprocess.run([node, "-e", JS_JUDGE, str(ROOT / "browser" / "tb01c-client.js"), str(CORPUS)],
                          capture_output=True, text=True, timeout=120)
    if proc.returncode != 0:
        raise AssertionError(proc.stderr)
    return json.loads(proc.stdout)


class PairContractCorpus(unittest.TestCase):
    def test_python_judges_every_case_as_expected(self):
        corpus, got = python_verdicts()
        cases = corpus["cases"]
        self.assertGreaterEqual(sum(case["expected"] == "ACCEPT" for case in cases), 11)
        self.assertGreaterEqual(sum(case["expected"] != "ACCEPT" for case in cases), 30)
        for case in cases:
            with self.subTest(case=case["name"]):
                self.assertEqual(code(got[case["name"]]), case["expected"])

    def test_python_and_js_agree_case_by_case(self):
        """两侧逐条比完整拒因（原因码与拒因文字同文），不只比原因码。"""
        _corpus, py = python_verdicts()
        js = js_verdicts()
        self.assertEqual(sorted(py), sorted(js))
        for name in py:
            with self.subTest(case=name):
                self.assertEqual(js[name], py[name])

    def test_corpus_covers_required_rejections(self):
        corpus = json.loads(CORPUS.read_text(encoding="utf-8"))
        codes = {case["expected"] for case in corpus["cases"]}
        required = {"LEAF", "PAIR_INPUTS", "DOMAIN", "DIRECTION", "MATRIX", "WITNESS", "REALIZATION", "SHAPE", "PROVISIONAL",
                    "REVISION", "CONNECTORS", "PENDING", "PAIR_COUNT", "ADJACENCY", "EDGE", "VERSION_COUNT", "SCOPE", "CHANGE",
                    "POSITION", "ROLE", "DESTRUCTION", "UCP_COUNT", "CORE", "RULE_MIX", "CANDIDATE_KIND", "AXIS_BINDING"}
        self.assertEqual(required - codes, set())

    def test_accepted_bases_cover_all_five_leaves_and_pending(self):
        """合法 cut 覆盖全部五叶外加 ≥9 待定，每种状态至少一个对象经两侧门接受。"""
        corpus = json.loads(CORPUS.read_text(encoding="utf-8"))
        seen = set()
        for case in corpus["cases"]:
            if case["base"] is None:
                for r in case["records"]:
                    if r["kind"] == "CC-020.center_pair":
                        d = r["payload"]["data"]
                        seen.add(d["leaf"] if d["status"] == "EVALUATED" else d["status"])
        self.assertEqual(seen, set(c.PAIR_LEAVES) | {c.PAIR_PENDING})

    def test_real_pair_history_transitions(self):
        corpus = json.loads(CORPUS.read_text(encoding="utf-8"))
        expanded = {name: records for name, _, _, records, _ in c.expand_center_corpus(corpus)}
        script = r"""
const A=require(process.argv[1]), input=JSON.parse(require('fs').readFileSync(0,'utf8'));
try { A.validatePairTransition(input.before,input.after); process.stdout.write('ACCEPT'); }
catch(e) { process.stdout.write(e.message); }
"""
        self.assertGreaterEqual(len(corpus["transitions"]), 10)
        for case in corpus["transitions"]:
            with self.subTest(case=case["name"]):
                before = [] if case["before"] is None else expanded[case["before"]]
                after = expanded[case["after"]]
                try:
                    c.validate_pair_transition(before, after)
                    py = "ACCEPT"
                except ValueError as error:
                    py = str(error)
                js = subprocess.run(["node", "-e", script, str(ROOT / "browser" / "tb01c-client.js")],
                                    input=json.dumps(dict(before=before, after=after)), capture_output=True, text=True, check=True, timeout=20).stdout
                self.assertEqual(py, js)
                self.assertEqual(code(py), case["expected"])

    def test_publication_generation_binds_full_typed_pair_and_change(self):
        import copy
        import hashlib

        corpus = json.loads(CORPUS.read_text(encoding="utf-8"))
        bases = {case["name"]: case for case in corpus["cases"] if case["base"] is None}
        meta = {"profile_id": c.PROFILE, "rule_revision": c.PAIR_RULE}

        def envelope(record, generation):
            record = copy.deepcopy(record)
            record.update(object_revision="1", batch_id="publication-test", fact_key=c.canonical([record["kind"], record["payload"]["slot"]]).decode(),
                          input_refs=[], source_coords=[], first_known_generation=str(generation), first_known_cut=f"cut-{generation}",
                          published_generation=str(generation), withdrawn_generation=None, withdrawal_reason=None, superseded_by=None, lifecycle="active")
            identity = {key: record[key] for key in ("kind", "fact_key", "payload", "input_refs")}
            identity.update(meta)
            record["object_id"] = "obj-" + hashlib.sha256(c.canonical(identity)).hexdigest()
            return record

        def typed_rows(name, offset=0):
            rows = bases[name]["records"]
            pair = next((r for r in rows if r["kind"] == "CC-020.center_pair"), None)
            event = copy.deepcopy(next(r for r in rows if r["kind"] == "CC-055.pair_change"))
            generation = int(event["payload"]["data"]["known_at"]["generation"]) + offset
            if pair is not None:
                pair = envelope(pair, generation)
                event["payload"]["data"]["object_id"] = pair["object_id"]
            return pair, envelope(event, generation), str(generation)

        js = r"""
const A=require(process.argv[1]), x=JSON.parse(require('fs').readFileSync(0,'utf8'));
function verdict(f) { try {f(); return 'ACCEPT';} catch(e) {return e.message;} }
process.stdout.write(JSON.stringify({typed: x.rows.map(r=>verdict(()=>A.validateTyped(r))),
 transition: verdict(()=>A.validatePairTransition(x.before,x.after,x.events,x.generation)),
 history: verdict(()=>A.validatePairHistory(x.history))}));
"""
        for offset in (0, 100):
            history, before = [], []
            for name in ("history_w12_k11", "history_w12_k12", "history_w12_k13", "history_withdrawn"):
                with self.subTest(name=name, offset=offset):
                    pair, event, generation = typed_rows(name, offset)
                    if before:
                        before[0].update(withdrawn_generation=generation, lifecycle="withdrawn",
                                         withdrawal_reason="fact_removed" if pair is None else "superseded_by_revision",
                                         superseded_by=None if pair is None else pair["object_id"])
                    rows = [r for r in (pair, event) if r is not None]
                    history.extend(rows)
                    after = [] if pair is None else [pair]
                    typed = []
                    for r in rows:
                        try:
                            c.validate_fact(r, meta)
                            typed.append("ACCEPT")
                        except ValueError as error:
                            typed.append(str(error))
                    try:
                        c.validate_pair_transition(before, after, [event], generation=generation)
                        transition = "ACCEPT"
                    except ValueError as error:
                        transition = str(error)
                    result = json.loads(subprocess.run(["node", "-e", js, str(ROOT / "browser" / "tb01c-client.js")],
                        input=json.dumps(dict(rows=rows, before=before, after=after, events=[event], generation=generation, history=history)),
                        capture_output=True, text=True, check=True, timeout=20).stdout)
                    self.assertEqual(result["typed"], typed)
                    self.assertEqual(result["transition"], transition)
                    expected = "ACCEPT" if offset == 0 else "PUBLICATION"
                    self.assertTrue(all(code(v) == expected for v in typed))
                    self.assertEqual(code(transition), expected)
                    self.assertEqual(code(result["history"]), expected)
                    if pair is not None and offset == 0:
                        # 当前代前移而对象没有变化：沿用原 known_at，不得要求每代重发。
                        c.validate_pair_transition([pair], [pair], [], generation="999")
                        stable = json.loads(subprocess.run(["node", "-e", js, str(ROOT / "browser" / "tb01c-client.js")],
                            input=json.dumps(dict(rows=[pair], before=[pair], after=[pair], events=[], generation="999", history=history)),
                            capture_output=True, text=True, check=True, timeout=20).stdout)
                        self.assertEqual(stable["transition"], "ACCEPT")
                    before = after
        # 外层/载荷一起改为 112，仍不能冒充实际传入的第 12 代。
        pair, event, _ = typed_rows("history_w12_k11")
        pair["payload"]["data"]["known_at"]["generation"] = "112"
        pair["payload"]["data"]["formed_known_at"]["generation"] = "112"
        pair = envelope(pair, 112)
        event["payload"]["data"].update(after=copy.deepcopy(pair["payload"]["data"]), known_at=copy.deepcopy(pair["payload"]["data"]["known_at"]), object_id=pair["object_id"])
        event["payload"]["slot"][0] = "112"
        event = envelope(event, 112)
        c.validate_fact(pair, meta); c.validate_fact(event, meta)
        with self.assertRaisesRegex(ValueError, "TB02F_PUBLICATION"):
            c.validate_pair_transition([], [pair], [event], generation="12")
        got = json.loads(subprocess.run(["node", "-e", js, str(ROOT / "browser" / "tb01c-client.js")],
            input=json.dumps(dict(rows=[pair,event], before=[], after=[pair], events=[event], generation="12", history=[pair,event])),
            capture_output=True, text=True, check=True, timeout=20).stdout)
        self.assertEqual(got["typed"], ["ACCEPT", "ACCEPT"])
        self.assertEqual(code(got["transition"]), "PUBLICATION")

    def test_center_family_schema_unchanged(self):
        self.assertEqual(c.PAIR_RULE, c.CENTER_RULE + "+center-pair/1")
        self.assertTrue(set(c.PAIR_KINDS) <= set(c.KINDS))
        self.assertFalse(set(c.PAIR_KINDS) & set(c.CENTER_KINDS))

    def test_ge9_colluding_forgery_and_legal_pending(self):
        """#1424 C1：成员/状态不变且角色与 pair 自洽，也不得越过待升级边界。"""
        corpus, py = python_verdicts()
        js = js_verdicts()
        bad = "ge9_roles_and_pair_forged_evaluated"
        for verdicts in (py, js):
            self.assertEqual(verdicts["w9_k15"], "ACCEPT")
            self.assertEqual(code(verdicts[bad]), "TB02E_ROLE_NULL")
        expanded = {name: records for name, _, _, records, _ in c.expand_center_corpus(corpus)}
        for name in ("w9_k15", bad):
            records = expanded[name]
            center = next(r["payload"]["data"] for r in records if r["kind"] == "CC-016.center")
            self.assertEqual((len(center["members"]), center["status"]), (12, "UPGRADE_PENDING_TB02H"))
            pair = next(r["payload"]["data"] for r in records if r["kind"] == "CC-020.center_pair")
            self.assertEqual(pair["status"], c.PAIR_PENDING if name == "w9_k15" else "EVALUATED")


if __name__ == "__main__":
    unittest.main()
