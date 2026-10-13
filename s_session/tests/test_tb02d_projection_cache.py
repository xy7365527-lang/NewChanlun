"""#1414 单槽负控：真实小v1 writer材料+局部v2协议模型；不是正式v2验收。"""
import argparse
import copy
import hashlib
import io
from types import SimpleNamespace
import sys
import unittest
from pathlib import Path
from unittest.mock import patch

import tb01c_query_tests as base

audit, query, server = base.audit, base.query, base.reader


class ProjectionCacheTests(base.QueryTests):
    def setUp(self):
        super().setUp()
        original = audit.verify
        # 沿用原查询测试的局部协议模型，仅增加协议字段，底层材料仍经真实verify。
        def protocol_verify(*args, **kwargs):
            return self.protocol_model(original(*args, **kwargs))
        shim = patch.object(audit, 'verify', side_effect=protocol_verify)
        shim.start()
        self.addCleanup(shim.stop)

    def page(self, proof=None, request=None, cached=True):
        proof = proof if proof is not None else self.reader.capture_verified()
        return query.snapshot_page(proof, request or self.request(), self.reader.resources, vars(server),
                                   projection_provider=self.reader._project_fixed_cut if cached else None)

    def test_projection_pages_exact_and_one_build_per_key(self):
        real = query.project_fixed_cut
        with patch.object(query, 'project_fixed_cut', wraps=real) as build:
            proof = self.reader.capture_verified()
            expected = self.page(proof, cached=False)
            build.reset_mock()
            page = self.page(proof)
            self.assertEqual(audit.canonical(page), audit.canonical(expected))
            while not page['done']:
                request = {'op': 'snapshot', 'snapshot_token': page['next_token']}
                proof = self.reader.capture_verified()  # every continuation still clamps DB version
                with patch.object(query, 'project_fixed_cut', real):
                    expected = self.page(proof, request, cached=False)
                page = self.page(proof, request)
                self.assertEqual(audit.canonical(page), audit.canonical(expected))
            self.assertEqual(build.call_count, 1)
            self.assertGreater(self.reader.stats['cache_hits'], 0)
            for generation, mode in [(4, 'RecomputedWithRevision'), (0, 'AsKnown'), (0, 'RecomputedWithRevision')]:
                request = self.request(mode, str(generation))
                with patch.object(query, 'project_fixed_cut', real):
                    expected = self.page(proof, request, cached=False)
                self.assertEqual(self.page(proof, request), expected)
            self.assertEqual(build.call_count, 4)

    def test_projection_returned_nested_values_cannot_poison_cache(self):
        proof = self.reader.capture_verified()
        first = self.page(proof)
        expected = copy.deepcopy(first)
        first['rows'][0]['record']['poison'] = ['mutable']
        first['fixed_cut']['scope']['structure'] = 'poison'
        first['catalog']['items'][0]['evidence']['poison'] = ['mutable']
        first['counts']['total'] = '0'
        first['next_token']['scope']['structure'] = 'poison'
        self.assertEqual(self.page(proof), expected)
        watch = dict(op='watch', cursor=query.cursor_at(proof, 0, vars(server)), max_batches='2', client_id='cache-test')
        changed = copy.deepcopy(watch)
        changed['cursor']['session_id'] = 'other'
        leases = query.ClientLeases(self.reader.resources)
        result = query.watch_page(proof, changed, self.reader.resources, vars(server), leases,
                                  projection_provider=self.reader._project_fixed_cut)
        expected = copy.deepcopy(result)
        result['observed_head']['scope']['structure'] = 'poison'
        result['gap']['rebuild']['scope']['structure'] = 'poison'
        result['gap']['snapshot_token']['scope']['structure'] = 'poison'
        self.assertEqual(query.watch_page(proof, changed, self.reader.resources, vars(server), leases,
                         projection_provider=self.reader._project_fixed_cut), expected)

    def test_projection_joint_capacity_boundary_and_no_time_swallow(self):
        proof = self.reader.capture_verified()
        expected = self.page(proof)
        amount = audit.object_bytes((proof, self.reader.memo, self.reader._projection_slot))
        self.assertGreater(amount, audit.object_bytes(proof))
        self.reader.discard_projection()
        self.reader.resources['max_cache_bytes'] = amount
        self.assertEqual(self.page(proof), expected)
        self.assertIsNotNone(self.reader._projection_slot)
        self.reader.discard_projection()
        self.reader.resources['max_cache_bytes'] = amount - 1
        self.assertEqual(self.page(proof), expected)
        self.assertIsNone(self.reader._projection_slot)
        self.reader.resources['max_cache_bytes'] = amount
        with patch.object(audit, 'object_bytes', side_effect=audit.QueryBudgetExceeded('deadline sentinel')):
            with self.assertRaisesRegex(audit.QueryBudgetExceeded, 'deadline sentinel'):
                self.page(proof)
        self.assertIsNone(self.reader._projection_slot)

    def test_projection_new_capture_same_generation_and_corruption(self):
        old = self.reader.capture_verified()
        self.page(old)
        self.mutate("INSERT INTO meta VALUES('projection_change','new')")
        later = self.reader.capture_verified()
        self.assertIsNone(self.reader._projection_slot)
        self.assertIsNot(old, later)
        self.assertNotEqual(old['capture_digest'], later['capture_digest'])
        self.assertEqual(self.page(later), self.page(later, cached=False))
        self.mutate("UPDATE witnesses SET merged_high='corrupt' WHERE witness_id=(SELECT witness_id FROM witnesses LIMIT 1)")
        with self.assertRaises(ValueError):
            self.reader.capture_verified()
        self.assertIsNone(self.reader._projection_slot)
        self.assertIsNone(self.reader.cached)

    def test_projection_foreign_proof_close_and_worker_isolation(self):
        proof = self.reader.capture_verified()
        expected = self.page(proof)
        # Equal but different proof cannot share a retained slot/cycle.
        self.assertEqual(self.page(copy.deepcopy(proof)), expected)
        self.assertIsNone(self.reader._projection_slot)
        self.page(proof)
        other = query.AuditReader(self.db, base.resources(), vars(server))
        self.addCleanup(other.close)
        other_proof = other.capture_verified()
        other._project_fixed_cut(other_proof, 4, 'AsKnown')
        self.assertIsNot(other._projection_slot, self.reader._projection_slot)
        self.reader.close()
        self.assertIsNone(self.reader._projection_slot)
        self.assertIsNotNone(other._projection_slot)
        self.page(self.reader.capture_verified())
        self.assertIsNotNone(self.reader._projection_slot)

    def test_projection_old_token_new_commit_and_modes(self):
        proof = self.reader.capture_verified()
        first = self.page(proof)
        before = first['next_token']
        self.call(self.db, 'accept', '--input', self.input(4, 11500), '--profile',
                  base.ROOT/'s_session/profiles/testonly_tick_1_1_ohlc.json')
        self.call(self.db, 'advance')
        later = self.reader.capture_verified()
        page = self.page(later, {'op': 'snapshot', 'snapshot_token': before})
        self.assertEqual(page['cut_projection_digest'], first['cut_projection_digest'])
        self.assertNotEqual(page['validated_capture_digest'], before['issued_capture_digest'])
        known = self.page(later)
        recomputed = self.page(later, self.request('RecomputedWithRevision', '4'))
        self.assertNotEqual(known['cut_projection_digest'], recomputed['cut_projection_digest'])
        self.assertEqual(known['fixed_cut']['as_of_generation'], '4')
        self.assertIsNone(recomputed['fixed_cut']['as_of_generation'])

    def test_projection_cached_token_negative_cases_and_page_size(self):
        proof = self.reader.capture_verified()
        first = self.page(proof)
        token = first['next_token']
        for field, value in [('session_id','wrong'), ('session_generation','2'), ('profile_hash','f'*64),
                             ('rule_revision','wrong'), ('order_version','wrong'), ('scope',{}),
                             ('cut_generation','0'), ('cut_projection_digest','f'*64), ('offset','999999')]:
            with self.subTest(field=field):
                bad = copy.deepcopy(token); bad[field] = value
                bad['token_checksum'] = hashlib.sha256(audit.canonical({k:v for k,v in bad.items() if k!='token_checksum'})).hexdigest()
                with self.assertRaises((server.InvalidQuery, ValueError)):
                    self.page(proof, {'op':'snapshot','snapshot_token':bad})
        bad = copy.deepcopy(token);bad['token_checksum']='f'*64
        with self.assertRaises(server.InvalidQuery):
            self.page(proof, {'op':'snapshot','snapshot_token':bad})
        oversized = self.request();oversized['page_size']=str(self.reader.resources['max_page_size']+1)
        with self.assertRaises(audit.QueryBudgetExceeded):
            self.page(proof, oversized)
        smaller = self.request();smaller['page_size']='1'
        self.assertEqual(self.page(proof, smaller)['cut_projection_digest'], first['cut_projection_digest'])

    def test_projection_new_capture_race_and_original_budget_failure_clear(self):
        self.page()
        fired=[]
        def race(name):
            if name == 'after_first_read' and not fired:
                fired.append(name); self.mutate("INSERT INTO meta VALUES('projection_race','yes')")
        self.reader.seam=race
        proof=self.reader.capture_verified()
        self.assertIsNone(self.reader._projection_slot)
        self.assertEqual(proof['meta']['projection_race'], 'yes')
        self.reader.seam=lambda _:None
        self.page(proof)
        self.mutate("INSERT INTO meta VALUES('projection_budget','yes')")
        self.reader.resources['max_capture_bytes']=1
        with self.assertRaises(audit.QueryBudgetExceeded):self.reader.capture_verified()
        self.assertIsNone(self.reader._projection_slot)
        self.assertIsNone(self.reader.cached)

    def test_projection_watch_uses_same_slot_and_original_batch_budget(self):
        proof=self.reader.capture_verified()
        self.page(proof,self.request('RecomputedWithRevision','4'))
        request=dict(op='watch',cursor=query.cursor_at(proof,2,vars(server)),max_batches='2',client_id='watch')
        limits=self.reader.resources
        with patch.object(query,'project_fixed_cut',side_effect=AssertionError('should hit slot')):
            got=query.watch_page(proof,request,limits,vars(server),query.ClientLeases(limits),projection_provider=self.reader._project_fixed_cut)
        expected=query.watch_page(proof,request,limits,vars(server),query.ClientLeases(limits))
        self.assertEqual(audit.canonical(got),audit.canonical(expected))
        limits=dict(limits,max_atomic_batch_bytes=1)
        with self.assertRaises(audit.QueryBudgetExceeded):
            query.watch_page(proof,request,limits,vars(server),query.ClientLeases(limits),projection_provider=self.reader._project_fixed_cut)

    def test_projection_typed_payload_and_envelope_hash_unchanged(self):
        from test_tb02d_contract import samples
        second,segment=samples()
        proof=self.reader.capture_verified()
        # Contract fixture is carried opaquely; no claim of a persisted v2 publication.
        proof['wires']=copy.deepcopy(proof['wires'])
        proof['wires']['objects'][0][1]['typed_test']={'second':second,'segment':segment}
        self.reader.discard_projection()
        expected=self.page(proof,cached=False)
        actual=self.page(proof)
        self.assertEqual(audit.canonical(actual),audit.canonical(expected))
        one=dict(source_namespace='test',source_epoch='1',message_id='one',payload_hash='a'*64,
                 session_id='tb01c-query-unit',session_generation='1')
        two=dict(one,message_id='two')
        a=query.response_envelope(one,actual,proof,1,audit)
        b=query.response_envelope(two,actual,proof,1,audit)
        self.assertEqual(a['payload_hash'],hashlib.sha256(audit.canonical(actual)).hexdigest())
        self.assertNotEqual(a['message_id'],b['message_id'])
        self.assertNotEqual(a['causal_refs'],b['causal_refs'])


    def test_projection_identity_change_and_verify_time_error_clear(self):
        self.page()
        identity = self.reader._identity()
        with patch.object(self.reader, '_identity', return_value=(identity[0], identity[1] + 1)):
            fresh = self.reader.capture_verified()
            self.assertIsNone(self.reader._projection_slot)
            self.page(fresh)
        self.reader.capture_verified()
        self.assertIsNone(self.reader._projection_slot)
        self.page()
        self.mutate("INSERT INTO meta VALUES('verify_budget','yes')")
        with patch.object(audit, 'verify', side_effect=audit.QueryBudgetExceeded('verify deadline sentinel')):
            with self.assertRaisesRegex(audit.QueryBudgetExceeded, 'verify deadline sentinel'):
                self.reader.capture_verified()
        self.assertIsNone(self.reader._projection_slot)
        self.assertIsNone(self.reader.cached)

    def test_projection_all_capture_race_windows_clear(self):
        for seam in ('after_v0', 'after_first_read', 'before_cache_registration'):
            with self.subTest(seam=seam):
                self.reader.seam = lambda _: None
                self.page()
                if seam == 'before_cache_registration':
                    self.mutate("INSERT OR REPLACE INTO meta VALUES('force_capture',?)", (seam,))
                fired = []
                def inject(name):
                    if name == seam and not fired:
                        fired.append(name)
                        self.mutate("INSERT OR REPLACE INTO meta VALUES('projection_race_seam',?)", (seam,))
                self.reader.seam = inject
                proof = self.reader.capture_verified()
                self.assertEqual(fired, [seam])
                self.assertIsNone(self.reader._projection_slot)
                if seam == 'before_cache_registration':
                    self.assertIsNone(self.reader.cached)
                    self.page(proof)
                    self.assertIsNone(self.reader._projection_slot)

    def test_projection_handler_clamps_every_request_and_clears_error(self):
        # Exercise production do_POST without opening a port; only transport is an in-memory sink.
        handler = object.__new__(server.Handler)
        handler.path = '/api/v2/snapshot'
        handler.connection = SimpleNamespace(settimeout=lambda _: None)
        handler.server = SimpleNamespace(query_reader=lambda: self.reader, resources=self.reader.resources,
            producer_epoch=7, leases=query.ClientLeases(self.reader.resources))
        responses = []
        def send(body, status=200, **_):
            responses.append((status, audit.parse_json(body)))
            return True
        handler._send_bytes = send
        def post(payload):
            wire = audit.canonical(self.envelope(payload))
            handler.headers = SimpleNamespace(get_all=lambda key, default: [str(len(wire))], get=lambda key: None)
            handler.rfile = io.BytesIO(wire)
            handler.do_POST()
            return responses[-1]
        with patch.object(self.reader, 'capture_verified', wraps=self.reader.capture_verified) as capture:
            status, first = post(self.request())
            self.assertEqual(status, 200)
            self.assertIsNotNone(self.reader._projection_slot)
            status, second = post(self.request())
            self.assertEqual((status, capture.call_count), (200, 2))
            self.assertEqual(first, second)
        self.reader.resources['max_reply_bytes'] = 4096
        status, error = post(self.request())
        self.assertEqual((status, error['payload']['error']), (503, 'QueryBudgetExceeded'))
        self.assertIsNone(self.reader._projection_slot)
        self.assertNotIn('fixed_cut', error['payload'])
        self.reader.resources['max_reply_bytes'] = base.resources()['max_reply_bytes']
        self.page()
        self.mutate("UPDATE witnesses SET merged_high='corrupt' WHERE witness_id=(SELECT witness_id FROM witnesses LIMIT 1)")
        status, error = post(self.request())
        self.assertEqual((status, error['payload']['error']), (503, 'StorageUnavailable'))
        self.assertIsNone(self.reader._projection_slot)


    def test_projection_rollback_failure_and_get_error_clear(self):
        self.page()
        connection = self.reader.connection
        class RollbackFailure:
            def __getattr__(self, name):
                return getattr(connection, name)
            def rollback(self):
                raise OSError('rollback sentinel')
        self.reader.connection = RollbackFailure()
        def fail(name):
            if name == 'after_first_read':
                raise ValueError('capture sentinel')
        self.reader.seam = fail
        with self.assertRaisesRegex(OSError, 'rollback sentinel'):
            self.reader.capture_verified()
        self.assertIsNone(self.reader._projection_slot)
        self.reader.connection = connection
        connection.rollback()
        self.reader.seam = lambda _: None
        self.page()
        handler = object.__new__(server.Handler)
        handler.server = SimpleNamespace(query_reader=lambda: self.reader, resources=self.reader.resources,
                                         producer_epoch=7, control_instance_id='unit')
        responses=[]
        handler._send_json=lambda payload,status=200: responses.append((status,payload))
        handler._send_bytes=lambda *_: self.fail('oversized GET should not publish bytes')
        with patch.object(audit,'project_state',return_value={'cut':{},'oversized':'x'*self.reader.resources['max_reply_bytes']}):
            handler._verified_read('state',None,None)
        self.assertEqual((responses[-1][0],responses[-1][1]['error']),(503,'QueryBudgetExceeded'))
        self.assertIsNone(self.reader._projection_slot)


if __name__ == '__main__':
    p=argparse.ArgumentParser();p.add_argument('--binary',type=Path,required=True)
    args=p.parse_args();base.BINARY=args.binary.resolve()
    names=[n for n in ProjectionCacheTests.__dict__ if n.startswith('test_projection_')]
    result=unittest.TextTestRunner(verbosity=2).run(unittest.TestSuite(ProjectionCacheTests(n) for n in names))
    raise SystemExit(not result.wasSuccessful())
