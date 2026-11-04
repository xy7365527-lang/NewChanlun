"""#1393 首独审返修：合法几何及重新哈希后的规则、原件、同 cut 绑定反例。"""
import copy
import hashlib
import json
from pathlib import Path
import subprocess
import unittest

from test_tb02u_contract import META, RULE, bar, contract, fact, raw_ref
from test_tb02c_contract import seed_payload
from test_tb02d_contract import samples
import s_query_integrity as integrity


def seal(record, meta=META):
    identity = {key: record[key] for key in ("kind", "fact_key", "payload", "input_refs")}
    record["object_id"] = "obj-" + hashlib.sha256(contract.canonical({**identity, **meta})).hexdigest()
    return record


def raw(coord, low, high, revision=1, seq=None):
    return dict(raw_ref(coord), **bar(coord, low, high), schema_revision="s-ohlc/1",
                ts="0", volume="1") | dict(revision=str(revision), input_revision=str(revision),
                                           seq=str(coord if seq is None else seq))


def evidence(left, right):
    fields = ("source_coord", "open", "high", "low", "close")
    return dict(direction="UP", established_at=right["source_coord"],
                previous_acc={k: left[k] for k in fields}, incoming={k: right[k] for k in fields},
                source_coords=[left["source_coord"], right["source_coord"]])


def initialized():
    raws = [raw(0, 0, 4), raw(1, 1, 5), raw(2, 2, 6)]
    pair = evidence(raws[0], raws[1])
    initial = fact("CC-004.initialization", "W", dict(
        initialization_policy_revision=contract.INITIALIZATION_POLICY,
        direction_state="Established", first_raw_pair=copy.deepcopy(pair)), [0, 1])
    groups = []
    for i in range(2):
        payload = dict(group_index=str(i), group_anchor=str(i), members=[str(i)],
                       **{k: raws[i][k] for k in ("open", "high", "low", "close")},
                       high_sources=[str(i)], low_sources=[str(i)], confirmed=i == 0,
                       direction="UP", direction_evidence=copy.deepcopy(pair),
                       confirmation_evidence=copy.deepcopy(pair) if i == 0 else None,
                       waiting_reasons=[], initialization_policy_revision=contract.INITIALIZATION_POLICY,
                       initial_direction_evidence=copy.deepcopy(pair))
        groups.append(fact("CC-005.inclusion_group", str(i), payload, [0, 1]))
    return dict(generation="1", objects=[initial, *groups], withdrawn_objects=[], raw_history=raws)


def pending():
    initial = fact("CC-004.initialization", "W", dict(
        initialization_policy_revision=contract.INITIALIZATION_POLICY,
        direction_state="Undetermined", first_raw_pair=None), [0])
    item = fact("CC-004.undetermined_raw", "0", dict(raw=bar(0, 0, 10), direction_state="Undetermined",
                initialization_policy_revision=contract.INITIALIZATION_POLICY, pending_since_cut="cut-1"), [0])
    return dict(generation="1", objects=[initial, item], withdrawn_objects=[], raw_history=[raw(0, 0, 10)])


def revised(snapshot):
    """旧组撤回后保留 rev1，新组使用同坐标 rev2；原始修订与对象发生分离。"""
    result = copy.deepcopy(snapshot)
    old = result["objects"]
    new_raw = dict(result["raw_history"][0], high="3", close="3", revision="2", input_revision="2",
                   seq=str(len(result["raw_history"])))
    result["raw_history"].append(new_raw)
    result["objects"] = copy.deepcopy(old)
    result["withdrawn_objects"] = old
    result["generation"] = "2"
    for record in old:
        record.update(withdrawn_generation="2", withdrawal_reason="fact_removed", lifecycle="withdrawn")
    for record in result["objects"]:
        record.update(first_known_generation="2", first_known_cut="cut-2", published_generation="2")
        record["input_refs"] = [dict((k, new_raw[k]) for k in contract.REF_FIELDS) if r["source_coord"] == "0" else r
                                for r in record["input_refs"]]
        p = record["payload"]
        for name in ("first_raw_pair", "initial_direction_evidence", "direction_evidence", "confirmation_evidence"):
            if p.get(name) is not None:
                p[name]["previous_acc"].update(high="3", close="3")
        if record["kind"] == "CC-004.undetermined_raw":
            p["raw"].update(high="3", close="3")
            p["pending_since_cut"] = "cut-2"
        seal(record)
    return result


def valid_snapshots():
    empty = pending()
    empty["objects"] = [fact("CC-004.initialization", "W", empty["objects"][0]["payload"], [])]
    empty["raw_history"] = []
    sparse = initialized()
    mapping = {"0": "10", "1": "100", "2": "1000"}
    def remap(value, key=None):
        if type(value) is dict:
            return {k: remap(v, k) for k, v in value.items()}
        if type(value) is list:
            return [mapping[v] for v in value] if key in ("source_coords", "members", "high_sources", "low_sources") else [remap(v) for v in value]
        return mapping[value] if key in ("source_coord", "group_anchor", "established_at") else value
    sparse = remap(sparse)
    for record in sparse["objects"]:
        if record["kind"] == "CC-005.inclusion_group":
            record["fact_key"] = contract.canonical([record["kind"], record["payload"]["group_anchor"]]).decode()
        seal(record)
    return [initialized(), pending(), revised(initialized()), revised(pending()), empty, sparse]


def schema_cases():
    first = "s2-axis-quantifiers+new-bi/1+segment-first/1"
    second = "s2-axis-quantifiers+new-bi/1+segment-second/1"
    for kind, payload, accepted in (("CC-011.segment_seed", seed_payload(), (first,)),
                                     ("CC-012.feature_sequence", samples()[0], (second, RULE))):
        for rule in (first, second, RULE):
            meta = dict(META, rule_revision=rule)
            yield dict(record=fact(kind, payload["slot"], payload, [], meta=meta), meta=meta,
                       valid=rule in accepted)


def assert_legal_raw_geometry(value):
    if type(value) is dict:
        if {"source_coord", "open", "high", "low", "close"} <= set(value):
            low, high = int(value["low"]), int(value["high"])
            assert low <= int(value["open"]) <= high and low <= int(value["close"]) <= high
        for child in value.values():
            assert_legal_raw_geometry(child)
    elif type(value) is list:
        for child in value:
            assert_legal_raw_geometry(child)


def validate_snapshot(snapshot, meta=META):
    """沿用生产字段/hash、birth 原件比对、每 cut 活动集链接三个实际入口。"""
    records = snapshot["objects"] + snapshot["withdrawn_objects"]
    history = snapshot["raw_history"]
    for record in records:
        contract.validate_fact(record, meta, history)
        # toy 的 rev2 在 cut2 接纳；原件比对仍由生产函数按八键与 seq 执行。
        effective = {}
        for item in history:
            if int(item["revision"]) <= int(record["first_known_generation"]):
                effective[item["source_coord"]] = {k: item[k] for k in contract.REF_FIELDS}
        contract.validate_sources(record, effective, raw=history)
    for cut in range(1, int(snapshot["generation"]) + 1):
        active = [r for r in records if int(r["first_known_generation"]) <= cut
                  and (r["withdrawn_generation"] is None or int(r["withdrawn_generation"]) > cut)]
        contract.validate_initialization_links(active, meta["rule_revision"])


def source_cases():
    result = []
    # 每项独立改变合法 OHLC，两个首对端点、两类证据、活动/撤回均覆盖。
    for withdrawn in (False, True):
        for index, field in ((0, "first_raw_pair"), (1, "initial_direction_evidence"), (2, "initial_direction_evidence")):
            for side in ("previous_acc", "incoming"):
                for axis in ("open", "high", "low", "close"):
                    snap = revised(initialized()) if withdrawn else initialized()
                    target = (snap["withdrawn_objects"] if withdrawn else snap["objects"])[index]
                    item = target["payload"][field][side]
                    item[axis] = str(int(item[axis]) + (-1 if axis in ("low", "close") else 1))
                    seal(target)
                    result.append((f"首对-{withdrawn}-{index}-{side}-{axis}", snap, "原始 OHLC"))
        snap = revised(pending()) if withdrawn else pending()
        target = (snap["withdrawn_objects"] if withdrawn else snap["objects"])[1]
        target["payload"]["raw"]["high"] = "11"  # 原件 high=10；几何仍合法。
        seal(target)
        result.append((f"未定OHLC-{withdrawn}", snap, "原始 OHLC"))
        # 原账有 coord2，但本对象未引用它；不能只查完整 raw_history 是否含此坐标。
        for index, field, side in ((0, "first_raw_pair", "previous_acc"),
                                   (0, "first_raw_pair", "incoming"),
                                   (1, "initial_direction_evidence", "previous_acc"),
                                   (1, "initial_direction_evidence", "incoming")):
            snap = revised(initialized()) if withdrawn else initialized()
            target = (snap["withdrawn_objects"] if withdrawn else snap["objects"])[index]
            target["payload"][field][side] = bar(2, 2, 6)
            if side == "incoming":
                target["payload"][field]["established_at"] = "2"
            seal(target)
            result.append((f"首对坐标-{withdrawn}-{index}-{side}", snap, "未绑定的原始坐标"))
        snap = revised(pending()) if withdrawn else pending()
        snap["raw_history"].append(raw(2, 2, 6, seq=len(snap["raw_history"])))
        target = (snap["withdrawn_objects"] if withdrawn else snap["objects"])[1]
        target["payload"]["raw"] = bar(2, 2, 6)
        target["fact_key"] = contract.canonical([target["kind"], "2"]).decode()
        seal(target)
        result.append((f"未定坐标-{withdrawn}", snap, "未绑定的原始坐标"))
    # 伪原件逐字等于最新 rev2，仍必须按旧对象引用的 rev1 拒绝。
    for snapshot, index, field in ((initialized(), 0, "first_raw_pair"),
                                   (initialized(), 1, "initial_direction_evidence"), (pending(), 1, "raw")):
        snap = revised(snapshot)
        target = snap["withdrawn_objects"][index]
        item = target["payload"][field] if field == "raw" else target["payload"][field]["previous_acc"]
        item.update(high="3", close="3")
        seal(target)
        result.append((f"撤回原件不得借最新修订-{field}", snap, "原始 OHLC"))
    return result


def link_cases():
    result = []
    for withdrawn in (False, True):
        for missing in (False, True):
            snap = revised(initialized()) if withdrawn else initialized()
            records = snap["withdrawn_objects"] if withdrawn else snap["objects"]
            if missing:
                records.pop(0)
            else:
                target = records[2]
                # 真正的 raw1/raw2 原件及引用；但与同 cut 的 raw0/raw1 初始化不符。
                target["payload"]["initial_direction_evidence"] = evidence(snap["raw_history"][1], snap["raw_history"][2])
                target["input_refs"].append(raw_ref(2))
                target["source_coords"].append("2")
                seal(target)
            result.append((f"同cut-{withdrawn}-缺失{missing}", snap, "同 cut"))
        snap = revised(initialized()) if withdrawn else initialized()
        records = snap["withdrawn_objects"] if withdrawn else snap["objects"]
        duplicate = copy.deepcopy(records[0])
        duplicate["input_refs"].append(raw_ref(2))
        duplicate["source_coords"].append("2")
        seal(duplicate)
        records.append(duplicate)
        result.append((f"重复初始化-{withdrawn}", snap, "唯一初始化对象"))
    return result


BAD_RULES = [
    "", "raw-initialization-w/1", "unknown", "unknown+raw-initialization-w/1", RULE + "+raw-initialization-w/1",
    RULE.replace("+raw-initialization", "++raw-initialization"), "+" + RULE, RULE + "+",
    RULE.replace("+new-bi/1", "+new-bi/1+new-bi/1"),
    "s2-axis-quantifiers+new-bi/2+segment-second/2+center-seed/1+center-pair/1",
    "s2-axis-quantifiers+new-bi/2+segment-second/2+center-seed/1+center-pair/1+raw-initialization-w/1",
    # #1419 recompose：CF22 与其 W 后继串随 main 段教义组件合成后，仍归 S3 合同、不进 s-session/1。
    "s2-axis-quantifiers+new-bi/2+segment-second/2+segment-turn/1+second-seq-scan-budget/2+center-seed/1+center-pair/1",
    "s2-axis-quantifiers+new-bi/2+segment-second/2+segment-turn/1+second-seq-scan-budget/2+center-seed/1+center-pair/1+raw-initialization-w/1",
    # #1393×#1419 整合负控：错序、跨 revision 挂 W 后缀、W 版本号变体、重复后缀均不得经剥后缀放行。
    "s2-axis-quantifiers+segment-second/1+new-bi/1+raw-initialization-w/1",
    "s2-axis-quantifiers+new-bi/2+segment-second/1+raw-initialization-w/1",
    "s2-axis-quantifiers+new-bi/2+segment-second/2+raw-initialization-w/1",
    RULE.replace("raw-initialization-w/1", "raw-initialization-w/2"),
    RULE + "+raw-initialization-w/1+raw-initialization-w/1",
]


class ReviewRegressionTests(unittest.TestCase):
    def test_exact_rule_admission_preserves_signed_versions(self):
        # #1393×#1419：白名单含 CF 已签全部串（逐条显式轴数），W 单域历史串映射其基规则。
        expected_axes = {"s2-axis-quantifiers": 9, "s2-axis-quantifiers+new-bi/1": 13, "s2-axis-quantifiers+new-bi/2": 13,
                         "s2-axis-quantifiers+new-bi/1+segment-first/1": 16}
        self.assertEqual(set(contract.RULE_BASES), set(contract.RULE_CONTRACTS) | {RULE})
        for rule in contract.RULE_BASES:
            axes_count = expected_axes.get(contract.base_rule(rule), 17)
            meta = dict(META, rule_revision=rule)
            record = initialized()["objects"][1]
            if rule != RULE:
                record["payload"].pop("initialization_policy_revision")
                record["payload"].pop("initial_direction_evidence")
            seal(record, meta)
            contract.validate_fact(record, meta)
            self.assertEqual(len(integrity._published_axes(rule)), axes_count)
        for rule in BAD_RULES:
            with self.subTest(rule=rule):
                record, meta = pending()["objects"][1], dict(META, rule_revision=rule)
                seal(record, meta)
                contract.validate_fact(record)  # schema 合法；不能借旧 hash 拒绝。
                with self.assertRaisesRegex(ValueError, "完整规则组合"):
                    contract.validate_fact(record, meta)
                with self.assertRaisesRegex(ValueError, "完整规则组合"):
                    integrity._published_axes(rule)

    def test_unknown_cc004_kind_rejected_python_and_javascript(self):
        # #1393×#1419：前缀 CC-004 只是证据归轴，不放行未知 kind。
        record = pending()["objects"][1]
        record["kind"] = "CC-004.initialization_v2"
        record["fact_key"] = contract.canonical([record["kind"], "0"]).decode()
        seal(record)
        with self.assertRaises(ValueError):
            contract.validate_fact(record, META)
        script = r"""
const c = require(process.argv[1]);
const r = JSON.parse(require('fs').readFileSync(0, 'utf8'));
c.validateTyped(r).then(() => { process.stderr.write('未知 CC-004 kind 被放行'); process.exitCode = 1; }, () => process.stdout.write('rejected'));
"""
        client = Path(__file__).resolve().parents[1] / "browser/tb01c-client.js"
        result = subprocess.run(["node", "-e", script, str(client)], input=json.dumps(record), text=True, capture_output=True)
        self.assertEqual((result.returncode, result.stdout), (0, "rejected"), result.stderr)

    def test_legal_active_and_withdrawn_exact_revisions(self):
        for snapshot in valid_snapshots():
            validate_snapshot(snapshot)

    def test_signed_rule_selects_exact_segment_schema(self):
        for case in schema_cases():
            contract.validate_fact(case["record"])
            if case["valid"]:
                contract.validate_fact(case["record"], case["meta"])
            else:
                with self.assertRaisesRegex(ValueError, "段合同与cut规则版本不符"):
                    contract.validate_fact(case["record"], case["meta"])

    def test_rehashed_payload_sources_and_same_cut_links(self):
        for name, snapshot, error in source_cases() + link_cases():
            with self.subTest(name=name):
                assert_legal_raw_geometry(snapshot)
                for record in snapshot["objects"] + snapshot["withdrawn_objects"]:
                    contract.validate_fact(record, META, snapshot["raw_history"])
                with self.assertRaisesRegex(ValueError, error):
                    validate_snapshot(snapshot)

    def test_same_cut_refs_must_match_even_when_ohlc_is_unchanged(self):
        records = initialized()["objects"]
        records[2]["input_refs"][0].update(revision="2", input_revision="2", seq="3")
        seal(records[2])
        with self.assertRaisesRegex(ValueError, "同 cut 原始修订不同"):
            contract.validate_initialization_links(records, RULE)

    def test_javascript_typed_sources_and_production_identity_together(self):
        cases = dict(valid=valid_snapshots(), schemas=list(schema_cases()),
                     invalid=[dict(name=n, snapshot=s, error=e) for n, s, e in source_cases() + link_cases()],
                     bad_rules=BAD_RULES, rules=list(contract.RULE_BASES), meta=META)
        script = r"""
const c = require(process.argv[1]);
const cases = JSON.parse(require('fs').readFileSync(0, 'utf8'));
const all = s => [...s.objects, ...s.withdrawn_objects];
async function seal(r, meta) {
  r.object_id = 'obj-' + await c.sha256({...Object.fromEntries(['kind','fact_key','payload','input_refs'].map(k => [k,r[k]])), ...meta});
}
async function validate(s, meta) {
  for (const r of all(s)) { await c.validateTyped(r); await c.validateTB02Identity(r, meta); }
  await c.validateTB02InitializationSources({...s, rule_revision: meta.rule_revision});
}
(async () => {
  for (const s of cases.valid) await validate(s, cases.meta);
  for (const {record, meta, valid} of cases.schemas) {
    await c.validateTyped(record);
    let accepted = true;
    try { await c.validateTB02Identity(record, meta); }
    catch (e) { if (!e.message.includes('段合同与cut规则版本不符')) throw e; accepted = false; }
    if (accepted !== valid) throw new Error('段 schema 选择错误');
  }
  for (const {name, snapshot, error} of cases.invalid) {
    // 所有反例均先通过实际身份入口，再要求来源/同 cut 门拒绝。
    for (const r of all(snapshot)) { await c.validateTyped(r); await c.validateTB02Identity(r, cases.meta); }
    let rejected = false;
    try { await c.validateTB02InitializationSources({...snapshot, rule_revision: cases.meta.rule_revision}); }
    catch (e) { if (!e.message.includes(error)) throw new Error(name + ': ' + e.message); rejected = true; }
    if (!rejected) throw new Error(name + ': 非法来源被放行');
  }
  for (const rule of cases.rules) {
    const meta = {...cases.meta, rule_revision:rule}, r = structuredClone(cases.valid[0].objects[1]);
    if (rule !== cases.meta.rule_revision) { delete r.payload.initialization_policy_revision; delete r.payload.initial_direction_evidence; }
    await seal(r, meta); await c.validateTyped(r); await c.validateTB02Identity(r, meta);
  }
  for (const rule of cases.bad_rules) {
    const meta = {...cases.meta, rule_revision:rule}, r = structuredClone(cases.valid[1].objects[1]);
    await seal(r, meta); await c.validateTyped(r);
    for (const call of [() => c.validateTB02Identity(r, meta), () => c.validateTB02InitializationSources({generation:'0',objects:[],withdrawn_objects:[],raw_history:[],rule_revision:rule})]) {
      let rejected = false;
      try { await call(); } catch (e) { if (!e.message.includes('完整规则组合')) throw e; rejected = true; }
      if (!rejected) throw new Error('非法完整规则被放行: ' + rule);
    }
  }
  process.stdout.write(JSON.stringify({valid:cases.valid.length,rejected:cases.invalid.length,rules:cases.rules.length,bad_rules:cases.bad_rules.length}));
})().catch(e => { process.stderr.write(e.stack); process.exitCode = 1; });
"""
        client = Path(__file__).resolve().parents[1] / "browser/tb01c-client.js"
        result = subprocess.run(["node", "-e", script, str(client)], input=json.dumps(cases),
                                text=True, capture_output=True)
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(json.loads(result.stdout), dict(valid=len(cases["valid"]), rejected=len(cases["invalid"]),
                                                       rules=len(contract.RULE_BASES), bad_rules=len(BAD_RULES)))


if __name__ == "__main__":
    unittest.main(verbosity=2)
