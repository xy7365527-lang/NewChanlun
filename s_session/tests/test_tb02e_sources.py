"""#1419 X1：来源/实体成员边界；完整轻量 SQLite → 公共 Q，无校验替身。

段/中枢由既有合同样本组装，补成完整 typed 形状并封存内容 hash；中枢变化使用
Rust tb02e.rs 的完整 before/after 布局。它检验读取入口，不是 S 结构计算或正式运行证据。
"""
import copy
from contextlib import closing
import hashlib
import importlib.util
import json
from pathlib import Path
import sqlite3
import sys
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT))
import s_readonly_server as server
from test_tb02d_contract import first_sample, samples
from tb01c_query_tests import resources

audit = server._query_integrity()
query = server._query_module("s_query")
c = audit.tb02
spec = importlib.util.spec_from_file_location("center_source_corpus", ROOT / "tests/fixtures/tb02e-contract/generate-corpus.py")
generator = importlib.util.module_from_spec(spec)
spec.loader.exec_module(generator)


def records():
    """三个完成段、一个中枢与完整 formed 事件；所有段补齐 typed 字段及序列引用。"""
    generator.STREAMS["SOURCE_LOCK"] = [10, 40, 20, 35]
    case = generator.cut("SOURCE_LOCK", 2)
    out = []
    for record in case["records"]:
        if record["kind"] != "CC-013.segment":
            out.append(copy.deepcopy(record))
            continue
        old = record["payload"]["data"]
        k = int(old["construction"]["start_stroke"]) // 3
        second, segment = samples()
        first = first_sample(second)
        anchor = 10 + 4 * k
        identity = generator.uid(k)
        replacements = {"segment:1:1": identity, "seed:1:1": f"seed:1:{anchor}",
                        "first:segment:1:1": "first:" + identity, "second:segment:1:1": "second:" + identity,
                        "first-object": f"first-{k}", "second-object": f"second-{k}"}
        index_keys = {"stroke_index", "stroke_idx", "start_stroke", "end_stroke", "trigger_stroke", "from_stroke",
                      "through_stroke", "scan_from_stroke", "scan_through_stroke", "earlier_stroke", "later_stroke"}
        def remap(value, key=None):
            if isinstance(value, dict):
                return {name: remap(child, name) for name, child in value.items()}
            if isinstance(value, list):
                if key in ("stroke_indices", "turn_hypothesis_pair"):
                    return [str(int(x) + 3 * k) for x in value]
                if key in ("start_roots", "end_roots", "extreme_roots"):
                    return [str(anchor + (int(x) - 1) // 4) for x in value]
                return [remap(x) for x in value]
            if isinstance(value, str):
                if key in index_keys:
                    return str(int(value) + 3 * k)
                if key in ("start_anchor", "end_anchor", "group_anchor"):
                    return str(anchor + (int(value) - 1) // 4)
                return replacements.get(value, value)
            return value
        second, first, segment = [remap(p) for p in (second, first, segment)]
        a, b = generator.STREAMS["SOURCE_LOCK"][k:k + 2]
        direction = "UP" if b > a else "DOWN"
        for p in (second, first, segment):
            d = p["data"]
            for j, ref in enumerate(d["stroke_refs"]):
                ref.update(entity_id=f"bi:1:{ref['start_anchor']}", start_price=str(a if j % 2 == 0 else b), end_price=str(b if j % 2 == 0 else a))
            if "role" in d:
                d.update(direction=direction, member_direction=direction if d["role"] == "SECOND_SEQUENCE" else "DOWN" if direction == "UP" else "UP")
        segment["data"]["construction"]["direction"] = direction
        segment["data"]["geometric_end"]["price"] = str(b)
        segment["data"]["fact_vector"]["termination_witness"]["construction"] = copy.deepcopy(segment["data"]["construction"])
        out.extend([dict(kind="CC-012.feature_sequence", object_id=f"first-{k}", payload=first),
                    dict(kind="CC-012.feature_sequence", object_id=f"second-{k}", payload=second),
                    dict(kind=record["kind"], object_id=record["object_id"], payload=segment)])
    center = next(r for r in out if r["kind"] == "CC-016.center")
    change = next(r for r in out if r["kind"] == "CC-055.center_change")
    change["payload"]["data"]["after"] = copy.deepcopy(center["payload"]["data"])
    return out


def database(path, mutate=None):
    """规范封存一个有限 typed cut，允许在 hash 前变异以免哈希门掩盖来源拒因。"""
    rs = records()
    if mutate:
        mutate(rs)
    with closing(sqlite3.connect(path)) as conn:
        conn.execute('PRAGMA journal_mode=WAL')
        conn.executescript(audit.LEGACY_SCHEMA + c.SCHEMA)
        def insert(table, row):
            names = list(row)
            conn.execute('INSERT INTO ' + table + ' (' + ','.join(names) + ') VALUES (' + ','.join('?' for _ in names) + ')', tuple(row.values()))
        scope = dict(structure='CompleteCut', economic='not_started')
        profile = json.loads((ROOT / 'profiles/ohlc_integer_tb02a_v1.json').read_text())
        phash = hashlib.sha256(c.canonical(profile)).hexdigest()
        meta = dict(session_id='center-source-lock', catalog_revision='source-lock', rule_revision=c.CENTER_RULE,
                    writer_epoch='1', generation='1', structure_cut='cut-1', index_frontier='pending', last_advance_frontier='31',
                    scope=c.canonical(scope).decode(), profile_id=c.PROFILE, profile_hash=phash,
                    profile_definition=c.canonical(profile).decode(), input_profile=c.PROFILE)
        raw = []
        for i in range(32):
            row = dict(identity_key=f'ns|1|TEST|e{i}', revision=1, input_revision='1', payload_hash='', receipt_id='', seq=i,
                       source_namespace='ns', source_epoch='1', instrument='TEST', event_id=f'e{i}', received_at='t', raw_text='source-lock',
                       price='1', ts=str(i), volume='1', source_coord=str(i), supersedes_revision=None,
                       schema_revision=c.RAW_SCHEMA, open='1', high='2', low='0', close='1')
            row['payload_hash'] = hashlib.sha256(c.canonical(audit._raw_event_content(row))).hexdigest()
            row['receipt_id'] = 'rcpt-' + hashlib.sha256((row['identity_key'] + '|' + row['payload_hash']).encode()).hexdigest()[:16]
            insert('raw_events', {k: v for k, v in row.items() if k not in ('schema_revision', 'open', 'high', 'low', 'close')})
            insert('raw_ohlc', {k: row[k] for k in ('identity_key', 'revision', 'schema_revision', 'open', 'high', 'low', 'close')})
            raw.append(row)
        wires = [audit._raw_wire(r) for r in raw]
        all_refs = [{k: r[k] for k in c.REF_FIELDS} for r in wires]
        by_id = {r['object_id']: r for r in rs}
        facts = {}
        def seal(oid):
            if oid in facts:
                return facts[oid]['object_id']
            record = by_id[oid]
            def bind(value):
                if isinstance(value, dict):
                    return {k: bind(v) for k, v in value.items()}
                if isinstance(value, list):
                    return [bind(v) for v in value]
                return seal(value) if isinstance(value, str) and value in by_id else value
            payload = bind(record['payload'])
            kind = record['kind']
            slot = payload['group_anchor'] if kind == 'CC-005.inclusion_group' else payload['slot']
            key = c.canonical([kind, slot]).decode()
            sources = record.get('source_coords', [str(i) for i in range(32)])
            refs = [all_refs[int(i)] for i in sources]
            identity = dict(kind=kind, fact_key=key, payload=payload, input_refs=refs, profile_id=c.PROFILE, rule_revision=c.CENTER_RULE)
            fact = dict(object_id='obj-' + hashlib.sha256(c.canonical(identity)).hexdigest(), object_revision=1, kind=kind,
                        batch_id='pending', fact_key=key, payload_json=c.canonical(payload).decode(), input_refs_json=c.canonical(refs).decode(),
                        source_coords_json=c.canonical(sources).decode(), first_known_generation=1, first_known_cut='cut-1', published_generation=1,
                        withdrawn_generation=None, withdrawal_reason=None, superseded_by=None)
            facts[oid] = fact
            return fact['object_id']
        for record in rs:
            seal(record['object_id'])
        fs = list(facts.values())
        axes = {}
        for axis in c.CENTER_AXES:
            scope_name = 'TB-02-E' if axis in ('CC-016', 'CC-017', 'CC-022') else 'TB-02-D' if axis in ('CC-011', 'CC-012', 'CC-013', 'CC-014') else 'TB-02-A'
            object_axis = 'CC-013' if axis == 'CC-014' else axis
            evidence = dict(scope=scope_name, object_ids=[f['object_id'] for f in fs if f['kind'].startswith(object_axis + '.')], waiting_reasons=[])
            if scope_name == 'TB-02-E':
                evidence.update(delivered_subobligations=c.TB02E_DELIVERED, deferred_subobligations=c.TB02E_DEFERRED)
            if scope_name == 'TB-02-D':
                evidence.update(delivered_subobligations=['finite_complete_prefix_strict_second', 'independent_stroke_and_segment_break_facts'],
                                deferred_subobligations=['general_equal_identity', 'general_scan_continuation', 'global_G006_bridge'])
            if axis == 'CC-056':
                evidence.update(raw_revisions=[], withdrawals=[], replaces=[])
            axes[axis] = dict(impl_status='not_implemented', proof_status='not_proved', run_status='not_run', evidence=evidence)
        internal = [{k: v for k, v in f.items() if k not in ('batch_id', 'withdrawn_generation', 'withdrawal_reason', 'superseded_by')} for f in fs]
        batch = dict(generation=1, structure_cut='cut-1', input_frontier=31, catalog_revision=meta['catalog_revision'],
                     rule_revision=c.CENTER_RULE, profile_id=c.PROFILE, profile_hash=phash, scope=scope,
                     catalog_axes=axes, objects=internal, witnesses=[], relations=[], observations=[], raw_events=raw)
        content = c.canonical(batch)
        bid = 'batch-' + hashlib.sha256(content).hexdigest()
        insert('batches', dict(batch_id=bid, canonical_bytes=content, byte_len=len(content)))
        upserts = []
        for fact in fs:
            fact['batch_id'] = bid
            insert('structure_facts', fact)
            upserts.append(c.project_fact(fact, audit.parse_json, meta=meta, raw=wires))
        evidence = dict(scope=scope, profile_id=c.PROFILE, profile_hash=phash, rule_revision=c.CENTER_RULE, batch_id=bid, structure_cut='cut-1', axes=axes)
        evidence.update({k: '0' for k in ('classified_objects', 'windows_total', 'merged_bars', 'effective_source_positions', 'withdrawals', 'replaces', 'insufficient_knowledge', 'domain_not_satisfied')})
        seq_range = {'from': '0', 'to': '31'}
        header = dict(session_id=meta['session_id'], catalog_revision=meta['catalog_revision'], base_cut='cut-0', next_cut='cut-1',
                      index_frontier=bid, catalog_run_status='run', generation='1', input_frontier='31', seq_range=seq_range, catalog_evidence=evidence)
        delta = dict(header, upserts=upserts, withdrawals=[], replaces=[], witnesses=[], relations=[], observations=[], raw_history_added=wires)
        insert('structure_deltas', dict(generation=1, session_id=meta['session_id'], catalog_revision=meta['catalog_revision'], base_cut='cut-0', next_cut='cut-1',
               index_frontier=bid, input_frontier=31, catalog_run_status='run', seq_range_json=c.canonical(seq_range).decode(),
               catalog_evidence_json=c.canonical(evidence).decode(), delta_json=c.canonical(delta).decode()))
        meta['index_frontier'] = bid
        for axis, item in axes.items():
            insert('catalog', dict(catalog_id=axis, kind='source-lock', title=axis, domain='source-lock', branches_json='[]',
                   impl_status=item['impl_status'], proof_status=item['proof_status'], run_status=item['run_status'], evidence_json=c.canonical(item['evidence']).decode()))
        for key, value in meta.items():
            insert('meta', dict(key=key, value=value))
        conn.commit()


class SourceMemberTests(unittest.TestCase):
    def verify(self, mutate=None):
        with tempfile.TemporaryDirectory(prefix='tb1419-sources-') as directory:
            db = Path(directory) / 'source.sqlite'
            database(db, mutate)
            reader = query.AuditReader(db, resources(), vars(server))
            try:
                return reader.capture_verified()
            finally:
                reader.close()

    def test_public_q_accepts_center_and_full_formed_snapshot(self):
        proof = self.verify()
        objects = audit.project_state(proof, 1, vars(server))['snapshot']['objects']
        center = next(r for r in objects if r['kind'] == 'CC-016.center')
        change = next(r for r in objects if r['kind'] == 'CC-055.center_change')
        self.assertEqual(len(center['payload']['data']['members']), 3)
        self.assertEqual(change['payload']['data']['after'], center['payload']['data'])
        self.assertTrue(all(x.isdecimal() for x in center['source_coords']))

    @staticmethod
    def group(rs, sources=None):
        rs.append(dict(kind='CC-005.inclusion_group', object_id='raw-group',
                       source_coords=sources if sources is not None else ['0', '31'],
                       payload=dict(group_index='0', group_anchor='0', members=['0', '31'],
                                    open='1', high='2', low='0', close='1', high_sources=['0'], low_sources=['0'],
                                    confirmed=False, direction=None, direction_evidence=None,
                                    confirmation_evidence=None, waiting_reasons=[])))

    def test_public_q_raw_group_members_remain_bound(self):
        self.verify(self.group)
        with self.assertRaisesRegex(ValueError, '未绑定的原始坐标'):
            self.verify(lambda rs: self.group(rs, ['0']))

    def test_public_q_center_entity_must_resolve_to_completed_segment(self):
        def mutate(rs):
            center = next(r for r in rs if r['kind'] == 'CC-016.center')['payload']['data']
            old = center['members'][-1]
            for key in ('members', 'seed_units', 'zn_members'):
                center[key] = ['segment:1:999' if x == old else x for x in center[key]]
        with self.assertRaisesRegex(ValueError, 'TB02E_MEMBER_NOT_COMPLETED'):
            self.verify(mutate)

    def test_source_gate_complete_before_after_and_withdrawn_snapshots(self):
        # 仅锁快照的来源类型，不把尚未构造逐代账的修订/撤回冒充合法 Q 历史。
        for change in ('member_added', 'withdrawn'):
            with self.subTest(change=change):
                record = next(r for r in records() if r['kind'] == 'CC-055.center_change')
                p = record['payload']
                d = p['data']
                d.update(before=copy.deepcopy(d['after']), change=change)
                if change == 'withdrawn':
                    d.update(after=None, object_id=None)
                p['slot'][-1] = change
                ref = {'source_coord': '0', 'revision': '1'}
                record.update(source_coords=['0'], input_refs=[ref])
                by_source, bars = c.validate_sources(record, {'0': ref})
                self.assertEqual(by_source, {'0': ref})
                self.assertEqual(bars, [])

    def test_public_q_nested_same_name_never_gets_entity_exemption(self):
        for nested in ({'nested': {'members': ['0']}}, {'nested': [{'members': ['segment:1:10']}]},
                       {'nested': {'members': {'members': ['0']}}}):
            with self.subTest(nested=nested):
                def mutate(rs):
                    p = next(r for r in rs if r['kind'] == 'CC-055.center_change')['payload']
                    p['data'].update(before=nested, change='member_added')
                    p['slot'][-1] = 'member_added'
                with self.assertRaisesRegex(ValueError, 'members 不属于已声明'):
                    self.verify(mutate)

    def test_public_q_entity_snapshot_requires_complete_type_and_identity(self):
        for bad in ({'members': ['segment:1:10']}, {'entity_id': 'center:1:999', 'members': ['segment:1:10']}):
            with self.subTest(snapshot=bad):
                def mutate(rs):
                    p = next(r for r in rs if r['kind'] == 'CC-055.center_change')['payload']
                    p['data'].update(before=bad, change='member_added')
                    p['slot'][-1] = 'member_added'
                with self.assertRaisesRegex(ValueError, 'TB02E_IDENTITY'):
                    self.verify(mutate)
        def partial(rs):
            p = next(r for r in rs if r['kind'] == 'CC-055.center_change')['payload']
            p['data']['after'] = {'entity_id': p['data']['entity_id'], 'members': ['segment:1:10']}
        with self.assertRaisesRegex(ValueError, 'TB02E_SHAPE'):
            self.verify(partial)

    def test_source_gate_does_not_exempt_wrong_kind_or_numeric_entity_members(self):
        center = next(r for r in records() if r['kind'] == 'CC-016.center')
        center.update(source_coords=['0'], input_refs=[])
        center['payload']['data']['members'][1] = '0'
        with self.assertRaisesRegex(ValueError, 'TB02E_IDENTITY'):
            c.validate_sources(center, {})
        center['kind'] = 'CC-054.knowledge_state'
        with self.assertRaisesRegex(ValueError, 'members 不属于已声明'):
            c.validate_sources(center, {})

    def test_source_gate_retains_scalar_array_and_effective_revision_checks(self):
        center = next(r for r in records() if r['kind'] == 'CC-016.center')
        center.update(source_coords=['0'], input_refs=[{'source_coord': '0', 'revision': '1'}])
        with self.assertRaisesRegex(ValueError, '不是该发布 cut 的有效源修订'):
            c.validate_sources(center, {'0': {'source_coord': '0', 'revision': '2'}})
        effective = {'0': center['input_refs'][0]}
        for key in ('source_coords', 'window', 'high_sources', 'low_sources', 'source_coord', 'group_anchor', 'established_at'):
            with self.subTest(field=key):
                bad = copy.deepcopy(center)
                bad['payload']['extra'] = {key: ['999'] if key in ('source_coords', 'window', 'high_sources', 'low_sources') else '999'}
                with self.assertRaisesRegex(ValueError, '未绑定的原始坐标'):
                    c.validate_sources(bad, effective)


if __name__ == '__main__':
    unittest.main()
