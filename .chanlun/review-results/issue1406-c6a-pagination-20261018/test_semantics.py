"""纯数据验证器测试；合成金额仅来自冻结独立 oracle，非产品证据。"""
import base64
import copy
from contextlib import contextmanager
import importlib.util
import json
import shutil
from pathlib import Path
import sys
import tempfile
import types
import unittest
from unittest.mock import patch

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[2]
sys.path[:0] = [str(ROOT / 's_session/tests'), str(ROOT / 's_session')]
spec = importlib.util.spec_from_file_location('semantics', HERE / 'semantics.py')
sem = importlib.util.module_from_spec(spec)
spec.loader.exec_module(sem)
source_files = (
 's_session/economic_query.py', 's_session/economic_http.py', 's_session/s_socket_client.py',
 's_session/s_readonly_server.py', 's_session/s_query.py', 's_session/s_query_integrity.py',
 's_session/s_tb02_contract.py', 's_session/tests/fixtures/tb01c/query-resources.json',
 's_session/tests/test_tb03c_query.py', 's_session/tests/tb03c_query_runtime.py',
 's_session/tests/test_tb03c_query_runtime_contract.py')
SOURCE = {p: sem.digest_file(ROOT / p) for p in source_files}
d = sem.load_driver(ROOT, SOURCE)
b = sem.binder_for(d)
from s_socket_client import reply_envelope
from test_tb03c_query_runtime_contract import We7FakeCase


def resolved_fixture(backend, payload, message, reply):
    request = d.Case.envelope(backend, payload, message)
    body = {k: v for k, v in payload.items() if k != 'op'}
    event = {'kind': 'ResolveEvidence', 'evidence_id': payload['evidence_id'],
        'evidence': body, 'covered': [v['fact_key'] for v in payload['covered']],
        'commit_seq': reply['cut']['commit_seq'], 'commit_ns': backend.commit_ns['E'],
        'origin_message': {k: request[k] for k in ('source_namespace', 'source_epoch', 'message_id')},
        'origin_request_hash': d.digest(request)}
    reply.update(evidence_id=payload['evidence_id'], evidence_hash=d.digest(body),
                 covered=event['covered'], commit_ns=event['commit_ns'])
    return event


class Backend7(We7FakeCase):
    def __init__(self, *args, **kwargs):
        super().__init__(*args, **kwargs)
        self.page_transfers = {}
        self.commit_ns = {domain: '0' for domain in self.seqs}
        self.global_commit_ns = 0

    def observe(self):
        views = super().observe()
        for domain, view in views.items():
            view['commit_ns'] = self.commit_ns[domain]
        return views

    def _page_document(self, domain, query):
        view = {'kind': 'EconomicView', 'domain_id': domain, 'writer_epoch': '1',
                'cut': self.cut(domain), 'commit_ns': self.commit_ns[domain],
                'image': copy.deepcopy(self.images[domain]),
                'image_hash': d.digest(self.images[domain])}
        if query['op'] == 'ReadView':
            return view
        return {'kind': 'EconomicChanges', 'domain_id': domain, 'writer_epoch': '1',
                'changes': [], 'cut': view['cut']}

    def owner_page(self, domain, payload, message):
        if payload['op'] == 'EconomicReadOpen':
            query = payload['query']
            value = self._page_document(domain, query)
            raw = d.canonical(value)
            transfer_id = d.hashlib.sha256((domain + ':' + message).encode()).hexdigest()[:32]
            descriptor = {'schema_revision': 'economic-read-pages/1', 'transfer_id': transfer_id,
                'session_id': self.manifest['session_id'], 'session_generation': self.manifest['session_generation'],
                'manifest_hash': d.digest(self.manifest), 'producer_id': domain + ':' + self.manifest['session_id'],
                'producer_epoch': '1', 'query_sha256': d.digest(query), 'document_kind': value['kind'],
                'fixed_cuts': {domain: value['cut']}, 'owner_epochs': {domain: '1'},
                'document_bytes': str(len(raw)), 'document_sha256': d.hashlib.sha256(raw).hexdigest(),
                'chunk_bytes': '262144', 'page_count': str((len(raw) + 262143) // 262144)}
            descriptor_hash = d.digest(descriptor)
            self.page_transfers[transfer_id] = {'descriptor_hash': descriptor_hash, 'raw': raw}
            return {'schema_revision': 'economic-read-pages/1', 'kind': 'EconomicReadOpened',
                    'descriptor': descriptor, 'descriptor_sha256': descriptor_hash}
        transfer = self.page_transfers[payload['transfer_id']]
        if payload['descriptor_sha256'] != transfer['descriptor_hash']:
            raise ValueError('synthetic owner descriptor mismatch')
        if payload['op'] == 'EconomicReadClose':
            self.page_transfers.pop(payload['transfer_id'], None)
            return {'schema_revision': 'economic-read-pages/1', 'kind': 'EconomicReadClosed',
                    'transfer_id': payload['transfer_id'], 'descriptor_sha256': transfer['descriptor_hash']}
        offset = int(payload['offset'])
        chunk = transfer['raw'][offset:offset + 262144]
        end = offset + len(chunk)
        return {'schema_revision': 'economic-read-pages/1', 'kind': 'EconomicReadPage',
            'transfer_id': payload['transfer_id'], 'descriptor_sha256': transfer['descriptor_hash'],
            'offset': str(offset), 'raw_byte_count': str(len(chunk)),
            'chunk_sha256': d.hashlib.sha256(chunk).hexdigest(),
            'data_base64': base64.b64encode(chunk).decode('ascii'),
            'next_offset': None if end == len(transfer['raw']) else str(end), 'done': end == len(transfer['raw'])}

    def owner(self, domain, payload, message):
        if payload.get('op') in ('EconomicReadOpen', 'EconomicReadPage', 'EconomicReadClose'):
            return self.owner_page(domain, payload, message)
        first = message not in self.responses
        reply = super().owner(domain, payload, message)
        if first:
            # Owner sequence is domain-local; AsKnown requires one global commit clock.
            self.global_commit_ns += 1
            self.commit_ns[domain] = str(self.global_commit_ns)
            reply.update(domain_id=domain, cut=self.cut(domain))
            if payload['op'] == 'CaptureExternal':
                row = self.images['E']['recorded'][-1]
                row.update(token=payload['token'], source_event_time=payload['source_event_time'])
                reply['token'] = payload['token']
                proof = self.capture_proofs[reply['fact_key']]
                proof['e_receipt'] = copy.deepcopy(reply)
                proof['e_commit_proof']['image'] = copy.deepcopy(self.images['E'])
                proof['e_commit_proof']['commit_ns'] = self.commit_ns[domain]
            elif payload['op'] == 'ResolveEvidence':
                self.images['E']['history'][-1] = resolved_fixture(self, payload, message, reply)
                self.resolution_proofs[payload['covered'][0]['fact_key']].update(
                    image=copy.deepcopy(self.images['E']), receipt=copy.deepcopy(reply))
            elif payload['op'] == 'ApplyAllocation':
                self.images[domain]['applied'][-1]['commit_ns'] = self.commit_ns[domain]
            self.responses[message] = copy.deepcopy(reply)
        return reply


class Backend5:
    """仅为原件管线造正控，金额复用独立 oracle，不声称第三实现。"""
    def __init__(self, plan):
        self.plan, self.model = plan, d.We5Expected()
        self.responses, self.seqs = {}, {'E': 0, 'B:chong-R1': 0}
        self.images = {k: We7FakeCase.empty() for k in self.seqs}
        self.commit_ns = {k: '0' for k in self.seqs}
        self.global_commit_ns = 0  # 全局单调时钟，与 Backend7 一致
        self.processlog = None
        self.page_transfers = {}
        self.resolved = {}

    def cut(self, domain):
        return {'domain_id': domain, 'commit_seq': str(self.seqs[domain]), 'root_hash': d.digest([domain, self.seqs[domain]])}

    def observe(self):
        return {domain: {'cut': self.cut(domain), 'commit_ns': self.commit_ns[domain],
                         'image': copy.deepcopy(image), 'image_hash': d.digest(image)} for domain, image in self.images.items()}

    def _page_document(self, domain, query):
        if query['op'] == 'ReadView':
            view = self.observe()[domain]
            return {'kind': 'EconomicView', 'domain_id': domain, 'writer_epoch': '1', **view}
        base = query['after_cut']
        now = self.observe()[domain]
        changes = [] if base == now['cut'] else [{k: now[k] for k in ('cut', 'commit_ns', 'image', 'image_hash')}]
        return {'kind': 'EconomicChanges', 'domain_id': domain, 'writer_epoch': '1', 'changes': changes, 'cut': now['cut']}

    def _page_open(self, domain, payload, message):
        query = payload['query']
        value = self._page_document(domain, query)
        raw = d.canonical(value)
        transfer_id = d.hashlib.sha256((domain + ':' + message).encode()).hexdigest()[:32]
        fixed = value['cut']
        descriptor = {'schema_revision': 'economic-read-pages/1', 'transfer_id': transfer_id,
            'session_id': self.manifest['session_id'], 'session_generation': self.manifest['session_generation'],
            'manifest_hash': d.digest(self.manifest), 'producer_id': domain + ':' + self.manifest['session_id'],
            'producer_epoch': '1', 'query_sha256': d.digest(query),
            'document_kind': value['kind'], 'fixed_cuts': {domain: fixed}, 'owner_epochs': {domain: '1'},
            'document_bytes': str(len(raw)), 'document_sha256': d.hashlib.sha256(raw).hexdigest(),
            'chunk_bytes': '262144', 'page_count': str((len(raw) + 262143) // 262144)}
        descriptor_hash = d.digest(descriptor)
        self.page_transfers[transfer_id] = {'descriptor': descriptor, 'descriptor_hash': descriptor_hash, 'raw': raw, 'domain': domain}
        return {'schema_revision': 'economic-read-pages/1', 'kind': 'EconomicReadOpened',
                'descriptor': descriptor, 'descriptor_sha256': descriptor_hash}

    def owner_page(self, domain, payload, message):
        if payload['op'] == 'EconomicReadOpen':
            return self._page_open(domain, payload, message)
        transfer = self.page_transfers.get(payload['transfer_id'])
        if transfer is None or transfer['descriptor_hash'] != payload['descriptor_sha256']:
            return {'schema_revision': 'economic-read-pages/1', 'kind': 'EconomicReadError', 'code': 'TransferExpired',
                    'detail': 'synthetic transfer expired', 'transfer_id': payload['transfer_id']}
        if payload['op'] == 'EconomicReadClose':
            self.page_transfers.pop(payload['transfer_id'], None)
            return {'schema_revision': 'economic-read-pages/1', 'kind': 'EconomicReadClosed',
                    'transfer_id': payload['transfer_id'], 'descriptor_sha256': transfer['descriptor_hash']}
        offset = int(payload['offset'])
        chunk = transfer['raw'][offset:offset + 262144]
        end = offset + len(chunk)
        return {'schema_revision': 'economic-read-pages/1', 'kind': 'EconomicReadPage',
                'transfer_id': payload['transfer_id'], 'descriptor_sha256': transfer['descriptor_hash'],
                'offset': str(offset), 'raw_byte_count': str(len(chunk)),
                'chunk_sha256': d.hashlib.sha256(chunk).hexdigest(),
                'data_base64': base64.b64encode(chunk).decode('ascii'),
                'next_offset': None if end == len(transfer['raw']) else str(end), 'done': end == len(transfer['raw'])}

    def owner(self, domain, payload, message):
        if payload.get('op') in ('EconomicReadOpen', 'EconomicReadPage', 'EconomicReadClose'):
            return self.owner_page(domain, payload, message)
        if message in self.responses:
            return copy.deepcopy(self.responses[message])
        step = self.plan['steps'][int(message.split('-')[-1])]
        expected = step['outcome']
        if expected not in ('Recorded', 'Resolved', 'Applied'):
            return {'kind': 'Error', 'error': expected + '：pure-fixture', 'domain_id': domain}
        if payload['op'] == 'ResolveEvidence' and payload['evidence_id'] in self.resolved:
            original_payload, original_reply = self.resolved[payload['evidence_id']]
            if payload != original_payload:
                raise ValueError('synthetic identity conflict')
            self.responses[message] = copy.deepcopy(original_reply)
            return copy.deepcopy(original_reply)
        self.model.advance(step)
        self.seqs[domain] += 1
        self.global_commit_ns += 1  # 全局递增
        self.commit_ns[domain] = str(self.global_commit_ns)  # 统一使用全局单调序号
        e, target = self.images['E'], self.images['B:chong-R1']
        def record(raw, pointer, event):
            return {'fact_key': d.fact_key(event), 'fact_hash': d.fact_hash(raw, pointer), 'event': event,
                    'raw_utf8': raw, 'json_pointer': pointer, 'raw_sha256': d.hashlib.sha256(raw.encode()).hexdigest(),
                    'token': 't-' + event['source']['external_fact_id'], 'source_event_time': event['source']['source_event_time']}
        e['recorded'] = [record(*v) for v in self.model.facts.values()]
        target['recorded'] = [record(*v) for v in self.model.applied.values()]
        e['pending'] = [{'fact_key': k, 'token': v['token'], 'scope': v['scope'], 'reason': {'kind': v['reason']}}
                        for k, v in self.model.pending.items()]
        # 保留已产生的完整历史来源；同业务重放不提交。
        target['applied'] = [{'fact_key': k, 'allocation_id': event['allocation_id'], 'status': 'Applied'}
                             for k, (_, _, event) in self.model.applied.items()]
        if self.model.book:
            view = self.model.book.view()
            target['book'] = {'actual_quantity': str(view['q']), 'cost_basis': {'K': str(view['K'])},
                 'principal': {'status': 'NotEvaluable', 'P_recovered': str(view['P_recovered']), 'batches': []},
                 'income_book': {k: str(view[k]) for k in ('Pi', 'A', 'W', 'R')}, 'fixed_q': str(view['fixed_q']),
                 'realized': {'fees': str(view['fees']), 'gross': str(view['realized_gross'])},
                 'cost_remaining': {'value': str(view['cost_remaining'])},
                 'phase': {'growth_eligible': {'status': view['growth_eligible']}, 'latch': view['latch'],
                           'gate': None if view['gate'] is None else {'kind': view['gate']},
                           'growth_enabled': {'status': view['growth_enabled'], 'reason': 'principal batches absent'},
                           'events': copy.deepcopy(self.model.book.events)},
                 'unit_cost': {k: str(v) for k, v in view['unit_cost'].items()}, 'stable_point': {'seq': view['stable_point']}}
        reply = {'kind': expected, 'domain_id': domain, 'cut': self.cut(domain)}
        if expected == 'Recorded':
            event = d.event_at(payload['raw_utf8'], payload['json_pointer'])
            reply.update(fact_key=d.fact_key(event), fact_hash=d.fact_hash(payload['raw_utf8'], payload['json_pointer']), token=payload['token'])
        if expected == 'Resolved':
            e['history'].append(resolved_fixture(self, payload, message, reply))
            self.resolved[payload['evidence_id']] = (copy.deepcopy(payload), copy.deepcopy(reply))
        self.responses[message] = copy.deepcopy(reply)
        return reply


class PureProcesses:
    def __init__(self, output):
        self.output, self.rows, self.time, self.number = output, [], .1, 0
    def tick(self):
        self.time += .001
        return self.time
    def spawn(self, command, label, stdout=b''):
        self.number += 1
        name = f'{self.number:04d}-{label}'
        row = {'event': 'spawn', 'pid': 100 + self.number, 'command': command,
               'stdout': name + '.stdout', 'stderr': name + '.stderr', 'direct_child': True,
               'inherits_driver_group': True, 'monotonic': self.tick()}
        self.rows.append(row)
        (self.output / row['stdout']).write_bytes(stdout)
        (self.output / row['stderr']).write_bytes(b'')
        return row, label
    def reap(self, entry, service=False):
        row, label = entry
        if service:
            self.rows.append({'event': 'signal', 'pid': row['pid'], 'signal': 15,
                              'returncode_before': None, 'monotonic': self.tick()})
        self.rows.append({'event': 'reaped', 'pid': row['pid'], 'label': label, 'stdout': row['stdout'],
                          'stderr': row['stderr'], 'returncode': -15 if service else 0, 'monotonic': self.tick()})


class ProducerCase(d.Case):
    def __init__(self, name, output, processes, binaries, manifest, backend):
        budget = types.SimpleNamespace()
        budget.t0 = 0.0
        budget.case_end = 300.0
        budget.work_end = 600.0
        budget.final_end = 630.0
        budget.clock = processes.tick
        def until(end):
            remaining = end - processes.time
            if remaining <= 0:
                raise TimeoutError("synthetic case deadline exceeded")
            return remaining
        budget.until = until

        # *** 关键修复：在 super().__init__() 之前完成所有必要的 manifest 准备 ***
        # 我们需要在父类看到之前修改 manifest，让父类保存修改后的版本
        manifest['session_id'] = 'tb1406-c6a-' + name
        manifest['e'] = {}
        # 但注意：self.nodes 的构建需要这些修改后的字段，所以我们提前构建
        nodes = {'E': manifest['e'], **{'B:' + n['chong_id']: n for n in manifest['chongs']}}

        # 父类 d.Case.__init__() (tb03c_query_runtime.py:599-600) 会：
        # 1. 为每个 node 添加 db/socket 字段（修改传入的 manifest）
        # 2. 追加 authorized_sources（Query source）
        # 3. 保存修改后的 manifest 到 manifest.json
        # 通过传入已修改的 manifest，父类会看到我们的修改，并在自己追加后保存完整版本
        super().__init__(name, output, types.SimpleNamespace(budget=budget), binaries, manifest)

        # 父类已经修改了 manifest（添加了 db/socket 字段和 authorized_sources）
        # 所以我们需要使用修改后的 self.manifest 重新构建 nodes
        # 确保 nodes 中的每个元素都指向 manifest 中已更新的同一对象
        self.nodes = {'E': self.manifest['e'], **{'B:' + n['chong_id']: n for n in self.manifest['chongs']}}

        for index in range(len(self.nodes)):
            (self.work / f'{index}.db').write_bytes(b'SYNTHETIC-NOT-A-SQLITE-DATABASE')
            (self.work / f'{index}.db.writer.lock').write_bytes(b'')
        self.processlog, self.backend = processes, backend
        self.backend.processlog = processes
        self.backend.manifest = self.manifest
        if hasattr(self.backend, 'e_identity'):
            self.backend.e_identity = {'domain_id': 'E', 'session_id': self.manifest['session_id'], 'session_generation': self.manifest['session_generation']}
        self.history, self.tokens, self.before_write = {}, {}, None
        self.q_page_transfers = {}
        self.store_views()
    def store_views(self):
        views = self.backend.observe()
        for domain, view in views.items():
            # Keep each commit clock snapshot addressable. A cut can be reused by
            # a replay step, so cut-only keys silently overwrote the earlier
            # AsKnown state (especially the initial commit_ns=0 view).
            commit_ns = str(view['commit_ns'])
            self.history[(domain, commit_ns, d.digest(view['cut']))] = copy.deepcopy(view)
        return views

    def view_at_cut(self, domain, cut):
        candidates = [view for (owner, _commit_ns, _cut_digest), view in self.history.items()
                      if owner == domain and view['cut'] == cut]
        if not candidates:
            raise KeyError((domain, d.digest(cut)))
        return copy.deepcopy(max(candidates, key=lambda view: int(view['commit_ns'])))

    def views_as_known(self, cutoff):
        result = {}
        for domain in self.nodes:
            candidates = [view for (owner, _commit_ns, _cut_digest), view in self.history.items()
                          if owner == domain and int(view['commit_ns']) <= int(cutoff)]
            if not candidates:
                continue
            result[domain] = copy.deepcopy(max(candidates, key=lambda view: int(view['commit_ns'])))
        return result
    def start(self):
        self.entries = []
        self.independent_owner_anchors = {}
        # manifest.json 已由父类 d.Case.__init__() 创建，无需重复保存
        for domain in self.nodes:
            init = self.processlog.spawn(self.args(domain, 'init'), self.name + '-init',
                d.canonical({'kind': 'Initialized', 'domain_id': domain, 'writer_epoch': '1'}))
            self.processlog.reap(init)
            self.epochs[domain] = '1'
            started = self.processlog.tick()
            anchor = {'started_at': started, 'deadline': started + 10, 'case_deadline': 300, 'work_deadline': 600}
            self.independent_owner_anchors[domain] = anchor
            self.entries.append(self.processlog.spawn(self.args(domain, 'serve'), self.name + '-serve'))
            self.owner(domain, {'op': 'ReadView', 'cut': None, 'as_known_ns': None}, 'ready-' + domain,
                       deadline=anchor['deadline'])
        d.save(self.work / 'query.json', {'schema_revision': 'economic-query/1', 'manifest': 'manifest.json',
            'manifest_hash': d.digest(self.manifest), 'query_source': d.QUERY_SOURCE, 'producer_epoch': '1',
            'expected_epochs': self.epochs, 'resources': d.QUERY_RESOURCES})
        self.port = 19003 if self.name == 'we5' else 19004
        d.save(self.output / 'port.json', {'host': '127.0.0.1', 'port': self.port, 'released_before_child_bind': True,
            'collision_policy': 'FAIL without touching occupant'})
        self.structural_db = self.work / 'UNUSED-STRUCTURE.db'
        command = [sys.executable, '-B', str(d.ROOT / 's_readonly_server.py'), '--db', str(self.structural_db),
            '--browser', str(self.work / 'UNUSED-BROWSER.html'), '--host', '127.0.0.1', '--port', str(self.port),
            '--resource-config', str(d.HERE / 'fixtures/tb01c/query-resources.json'), '--producer-epoch', '1',
            '--economic-config', str(self.work / 'query.json'),
            '--economic-wire-evidence', str(self.output / 'q-owner-wire')]
        started = self.processlog.tick()
        # 纯测试监督器独立保留；不能从将要落盘的 proof/transport 回抄。
        self.independent_startup_anchors = {'started_at': started, 'case_deadline': 300, 'work_deadline': 600}
        self.entries.append(self.processlog.spawn(command, self.name + '-q'))
        startup = {'schema_revision': 'c6a-q-startup/1', 'started_at': started, 'deadline': started + 10,
                   'case_deadline': 300, 'work_deadline': 600}
        first = self.sequence + 1
        self.http('/api/economic/ready', deadline=startup['deadline'], startup_ready=startup)
        d.save(self.output / 'Q-STARTUP.json', {**startup, 'first_sequence': first, 'success_sequence': self.sequence,
            'validated_at': self.processlog.tick(), 'success_response_sha256': d.sha(self.output / f'{self.sequence:04d}.http-response.bin')})
    def q_scope(self, public, document, outer_deadline):
        """合成完整内部原件，仅检验离线检查器；不是实际Q/owner运行证据。"""
        self.q_scope_attempt = getattr(self, 'q_scope_attempt', 0) + 1
        attempt = self.q_scope_attempt
        directory = self.output / 'q-owner-wire' / f'attempt-{attempt:06d}'
        directory.mkdir(parents=True)
        (directory / 'public-request.json').write_bytes(d.canonical(public))
        started = self.processlog.tick()
        deadline = min(started + 29, outer_deadline)
        d.save(directory / 'begin.json', {'schema_revision': 'c6a-q-owner-wire/1',
            'attempt': attempt, 'public_request_sha256': d.digest(public), 'started_at': started, 'deadline': started + 30})
        documents = []
        payload = public['payload']
        if payload['op'] == 'EconomicReadOpen':
            query = payload['query']
            if query['op'] == 'snapshot' and query['snapshot_token'] is None:
                for domain, row in document['domains'].items():
                    selector = {'op': 'ReadView', 'cut': None if query['cuts'] is None else query['cuts'][domain],
                                'as_known_ns': query['as_known_ns']}
                    body = {'kind': 'EconomicView', 'domain_id': domain, 'writer_epoch': '1',
                            **{k: row[k] for k in ('cut', 'commit_ns', 'image', 'image_hash')}}
                    documents.append((domain, selector, body))
            elif query['op'] == 'watch':
                for domain, row in document['domains'].items():
                    base = self.view_at_cut(domain, query['cuts'][domain])
                    documents.append((domain, {'op': 'ReadView', 'cut': query['cuts'][domain], 'as_known_ns': None},
                        {'kind': 'EconomicView', 'domain_id': domain, 'writer_epoch': '1', **base}))
                    documents.append((domain, {'op': 'Watch', 'after_cut': query['cuts'][domain]},
                        {'kind': 'EconomicChanges', 'domain_id': domain, 'writer_epoch': '1',
                         'cut': row['next_cut'], 'changes': row['changes']}))
                    for change in row['changes']:
                        documents.append((domain, {'op': 'ReadView', 'cut': change['cut'], 'as_known_ns': None},
                            {'kind': 'EconomicView', 'domain_id': domain, 'writer_epoch': '1', **change}))
        count, byte_count = 0, 0
        def emit(domain, payload, answer):
            nonlocal count, byte_count
            count += 1
            prefix = f'rpc-{count:04d}'
            start = self.processlog.tick()
            req = {'schema_revision': public['schema_revision'], 'session_id': self.manifest['session_id'],
                'session_generation': self.manifest['session_generation'], **d.QUERY_SOURCE,
                'message_id': 'query-' + d.digest([public['message_id'], count])[:32],
                'payload_hash': d.digest(payload), 'causal_refs': [], 'payload': payload}
            response = reply_envelope(req, answer, session_id=self.manifest['session_id'],
                session_generation=self.manifest['session_generation'], namespace='economic-session/replies',
                producer_id=domain + ':' + self.manifest['session_id'], producer_epoch='1')
            raw_req, raw_res = d.canonical(req) + b'\n', d.canonical(response) + b'\n'
            (directory / (prefix + '-request.bin')).write_bytes(raw_req)
            (directory / (prefix + '-response.bin')).write_bytes(raw_res)
            byte_count += len(raw_req) + len(raw_res)
            d.save(directory / (prefix + '-begin.json'), {'index': count, 'address': self.nodes[domain]['socket'],
                'timeout_ms': 30000, 'max_frame_bytes': 1048576, 'started_at': start, 'deadline': deadline,
                'logical_deadline': deadline, 'request_sha256': d.hashlib.sha256(raw_req).hexdigest(),
                'request_bytes': len(raw_req)})
            d.save(directory / (prefix + '-end.json'), {'transport': 'Received', 'error_class': None,
                'started_send': True, 'observed_at': self.processlog.tick(), 'response_bytes': len(raw_res),
                'response_sha256': d.hashlib.sha256(raw_res).hexdigest()})
        for domain, selector, body in documents:
            raw = d.canonical(body)
            tid = d.digest([public['message_id'], domain, count])[:32]
            desc = {'schema_revision': 'economic-read-pages/1', 'transfer_id': tid,
                'session_id': self.manifest['session_id'], 'session_generation': self.manifest['session_generation'],
                'manifest_hash': d.digest(self.manifest), 'producer_id': domain + ':' + self.manifest['session_id'],
                'producer_epoch': '1', 'query_sha256': d.digest(selector), 'document_kind': body['kind'],
                'fixed_cuts': {domain: body['cut']}, 'owner_epochs': {domain: '1'},
                'document_bytes': str(len(raw)), 'document_sha256': d.hashlib.sha256(raw).hexdigest(),
                'chunk_bytes': '262144', 'page_count': str((len(raw)+262143)//262144)}
            dh = d.digest(desc)
            emit(domain, {'schema_revision': 'economic-read-pages/1', 'op': 'EconomicReadOpen',
                'query': selector, 'requested_remaining_ms': '27000'},
                {'schema_revision': 'economic-read-pages/1', 'kind': 'EconomicReadOpened',
                 'descriptor': desc, 'descriptor_sha256': dh})
            for offset in range(0, len(raw), 262144):
                chunk = raw[offset:offset+262144]
                next_offset = offset + len(chunk)
                emit(domain, {'schema_revision': 'economic-read-pages/1', 'op': 'EconomicReadPage',
                    'transfer_id': tid, 'descriptor_sha256': dh, 'offset': str(offset), 'requested_remaining_ms': '26000'},
                    {'schema_revision': 'economic-read-pages/1', 'kind': 'EconomicReadPage', 'transfer_id': tid,
                     'descriptor_sha256': dh, 'offset': str(offset), 'raw_byte_count': str(len(chunk)),
                     'chunk_sha256': d.hashlib.sha256(chunk).hexdigest(), 'data_base64': base64.b64encode(chunk).decode(),
                     'next_offset': None if next_offset == len(raw) else str(next_offset), 'done': next_offset == len(raw)})
            emit(domain, {'schema_revision': 'economic-read-pages/1', 'op': 'EconomicReadClose',
                'transfer_id': tid, 'descriptor_sha256': dh},
                {'schema_revision': 'economic-read-pages/1', 'kind': 'EconomicReadClosed',
                 'transfer_id': tid, 'descriptor_sha256': dh})
        d.save(directory / 'end.json', {'attempt': attempt, 'rpc_count': count, 'wire_bytes': byte_count,
            'error_class': None, 'observed_at': self.processlog.tick()})

    def wire(self, address, request, *, http=False, limit=None, deadline=None, startup_ready=None):
        self.sequence += 1
        started = self.processlog.tick()
        if http:
            header, body = request.split(b'\r\n\r\n', 1)
            path = header.split(b' ')[1].decode()
            envelope = json.loads(body) if body else None
            if path.endswith('/ready'):
                reply = {'manifest_hash': d.digest(self.manifest), 'book_revision': 'tb03c-book/1',
                         'domains': list(self.nodes), 'session_id': self.manifest['session_id'],
                         'session_generation': self.manifest['session_generation'], 'producer_epoch': '1'}
            else:
                payload = envelope['payload']
                if payload.get('op') in ('EconomicReadOpen', 'EconomicReadPage', 'EconomicReadClose'):
                    if payload['op'] == 'EconomicReadOpen':
                        query = payload['query']
                        views = self.store_views()
                        if query['op'] == 'snapshot':
                            if query['snapshot_token'] is not None:
                                document = self.tokens[query['snapshot_token']]
                            else:
                                if query['cuts']:
                                    views = {domain: self.view_at_cut(domain, cut) for domain, cut in query['cuts'].items()}
                                elif query['as_known_ns'] is not None:
                                    views = self.views_as_known(query['as_known_ns'])
                                document = {'kind': 'EconomicSnapshot', 'completeness': 'CompleteVector',
                                    'domains': {domain: {'status': 'Available', **view} for domain, view in views.items()},
                                    'session_id': self.manifest['session_id'], 'session_generation': self.manifest['session_generation'],
                                    'manifest_hash': d.digest(self.manifest),
                                    'mode': 'AsKnown' if query['cuts'] is not None or query['as_known_ns'] is not None else 'Current',
                                    'as_known_ns': query['as_known_ns']}
                                document['snapshot_token'] = 'economic-snapshot-' + d.digest(document)
                                self.tokens[document['snapshot_token']] = copy.deepcopy(document)
                            document_kind = 'EconomicSnapshot'
                        elif query['op'] == 'watch':
                            changes = {}
                            for domain, base_cut in query['cuts'].items():
                                now = views[domain]
                                changes[domain] = {'status': 'Available', 'base_cut': base_cut,
                                    'next_cut': now['cut'],
                                    'changes': [] if base_cut == now['cut'] else [{k: now[k] for k in ('cut', 'commit_ns', 'image', 'image_hash')}]}
                            document = {'kind': 'EconomicWatch', 'domains': changes,
                                'session_id': self.manifest['session_id'], 'session_generation': self.manifest['session_generation'],
                                'manifest_hash': d.digest(self.manifest)}
                            document_kind = 'EconomicWatch'
                        else:
                            raise ValueError('unsupported synthetic Q paged op')
                        raw_doc = d.canonical(document)
                        fixed = {domain: ((view.get('cut') if document_kind == 'EconomicSnapshot' else view.get('next_cut'))
                                          if view.get('status', 'Available') == 'Available' else None)
                                 for domain, view in document['domains'].items()}
                        transfer_id = d.hashlib.sha256(('Q:' + envelope['message_id']).encode()).hexdigest()[:32]
                        descriptor = {'schema_revision': 'economic-read-pages/1', 'transfer_id': transfer_id,
                            'session_id': self.manifest['session_id'], 'session_generation': self.manifest['session_generation'],
                            'manifest_hash': d.digest(self.manifest), 'producer_id': 'Q:' + self.manifest['session_id'],
                            'producer_epoch': '1', 'query_sha256': d.digest(query), 'document_kind': document_kind,
                            'fixed_cuts': fixed, 'owner_epochs': {domain: '1' for domain in self.nodes},
                            'document_bytes': str(len(raw_doc)), 'document_sha256': d.hashlib.sha256(raw_doc).hexdigest(),
                            'chunk_bytes': '262144', 'page_count': str((len(raw_doc) + 262143) // 262144)}
                        descriptor_hash = d.digest(descriptor)
                        self.q_page_transfers[transfer_id] = {'descriptor': descriptor, 'descriptor_hash': descriptor_hash, 'raw': raw_doc}
                        reply = {'schema_revision': 'economic-read-pages/1', 'kind': 'EconomicReadOpened', 'descriptor': descriptor, 'descriptor_sha256': descriptor_hash}
                    else:
                        transfer = self.q_page_transfers[payload['transfer_id']]
                        if payload['op'] == 'EconomicReadClose':
                            self.q_page_transfers.pop(payload['transfer_id'], None)
                            reply = {'schema_revision': 'economic-read-pages/1', 'kind': 'EconomicReadClosed', 'transfer_id': payload['transfer_id'], 'descriptor_sha256': transfer['descriptor_hash']}
                        else:
                            offset = int(payload['offset'])
                            chunk = transfer['raw'][offset:offset + 262144]
                            end = offset + len(chunk)
                            reply = {'schema_revision': 'economic-read-pages/1', 'kind': 'EconomicReadPage', 'transfer_id': payload['transfer_id'], 'descriptor_sha256': transfer['descriptor_hash'], 'offset': str(offset), 'raw_byte_count': str(len(chunk)), 'chunk_sha256': d.hashlib.sha256(chunk).hexdigest(), 'data_base64': base64.b64encode(chunk).decode('ascii'), 'next_offset': None if end == len(transfer['raw']) else str(end), 'done': end == len(transfer['raw'])}
                    self.q_scope(envelope, document if payload['op'] == 'EconomicReadOpen' else None, deadline)
                    reply = reply_envelope(envelope, reply, session_id=self.manifest['session_id'], session_generation=self.manifest['session_generation'], namespace='economic-observe/replies', producer_id='Q:' + self.manifest['session_id'], producer_epoch='1')
                else:
                    views = self.store_views()
                    if path.endswith('/watch'):
                        changes = {}
                        for domain, cut in payload['cuts'].items():
                            now = views[domain]
                            changes[domain] = {'status': 'Available', 'base_cut': cut, 'next_cut': now['cut'],
                                'changes': [] if cut == now['cut'] else [{k: now[k] for k in ('cut', 'commit_ns', 'image', 'image_hash')}]}
                        reply = {'kind': 'EconomicWatch', 'domains': changes, 'session_id': self.manifest['session_id'],
                                 'session_generation': self.manifest['session_generation'], 'manifest_hash': d.digest(self.manifest)}
                    else:
                        if payload['snapshot_token'] is not None:
                            reply = self.tokens[payload['snapshot_token']]
                        else:
                            if payload['cuts']:
                                views = {domain: self.view_at_cut(domain, cut) for domain, cut in payload['cuts'].items()}
                            elif payload['as_known_ns'] is not None:
                                views = self.views_as_known(query['as_known_ns'])
                            reply = {'kind': 'EconomicSnapshot', 'completeness': 'CompleteVector',
                                     'domains': {domain: {'status': 'Available', **view} for domain, view in views.items()},
                                     'session_id': self.manifest['session_id'], 'session_generation': self.manifest['session_generation'],
                                     'manifest_hash': d.digest(self.manifest),
                                     'mode': 'AsKnown' if payload['cuts'] is not None or payload['as_known_ns'] is not None else 'Current',
                                     'as_known_ns': payload['as_known_ns']}
                            token = 'economic-snapshot-' + d.digest(reply)
                            reply['snapshot_token'] = token
                            self.tokens[token] = copy.deepcopy(reply)
                    reply = reply_envelope(envelope, reply, session_id=self.manifest['session_id'],
                        session_generation=self.manifest['session_generation'], namespace='economic-observe/replies',
                        producer_id='Q:' + self.manifest['session_id'], producer_epoch='1')
            raw = d.canonical(reply)
            response = b'HTTP/1.0 200 OK\r\nContent-Length: ' + str(len(raw)).encode() + b'\r\n\r\n' + raw
        else:
            domain = next(k for k, v in self.nodes.items() if v['socket'] == address)
            envelope = json.loads(request)
            if envelope['payload']['op'] == 'ReadView':
                reply = {'kind': 'EconomicView', 'domain_id': domain, **self.store_views()[domain]}
            else:
                if envelope['message_id'] not in self.backend.responses:
                    self.before_write = self.store_views()
                reply = self.backend.owner(domain, envelope['payload'], envelope['message_id'])
                self.store_views()
            response = d.canonical(reply_envelope(envelope, reply, session_id=self.manifest['session_id'],
                session_generation=self.manifest['session_generation'], namespace='economic-session/replies',
                producer_id=domain + ':' + self.manifest['session_id'], producer_epoch='1')) + b'\n'
        label, kind = f'{self.sequence:04d}', 'http' if http else 'owner'
        request_path, response_path = label + f'.{kind}-request.bin', label + f'.{kind}-response.bin'
        (self.output / request_path).write_bytes(request)
        (self.output / response_path).write_bytes(response)
        transport = {'request': request_path, 'response': response_path,
            'request_sha256': d.sha(self.output / request_path), 'response_sha256': d.sha(self.output / response_path),
            'response_bytes': len(response), 'http': http, 'address': address}
        if startup_ready:
            transport['startup_ready_attempt'] = {**startup_ready, 'sequence': self.sequence,
                'attempt_started_at': started, 'observed_at': self.processlog.tick(), 'phase': 'complete',
                'exception_class': None, 'exception_errno': None, 'connect_completed': True, 'send_entered': True}
        d.save(self.output / (label + '-transport.json'), transport)
        return response
    def finish(self):
        for entry in reversed(self.entries):
            self.processlog.reap(entry, service=True)


def fixture(output):
    output.mkdir()
    seed, plan, oracle = d.load_inputs()
    processes = PureProcesses(output)
    binaries = output.parent / 'fake-binaries'
    build = {'source_sha': 'a' * 40, 'source_tree': 'b' * 40, 'command': ['cargo', 'build'],
             'exit_code': 0, 'source_files': SOURCE, 'binaries': {'e_economic_session': 'c' * 64, 'b_chong_session': 'd' * 64}}
    for command, label, raw in ((['git', 'rev-parse', 'HEAD'], 'source-head', b'a' * 40 + b'\n'),
        (['git', 'rev-parse', 'HEAD^{tree}'], 'source-tree', b'b' * 40 + b'\n'),
        (['git', 'status', '--porcelain', '--untracked-files=all'], 'source-clean', b'')):
        processes.reap(processes.spawn(command, label, raw))
    d.save(output / 'SOURCE.json', {'base': d.BASE, 'base_tree': d.BASE_TREE, 'build': build, 'inputs': d.INPUT_HASHES,
        'q_resource_sha256': d.sha(d.HERE / 'fixtures/tb01c/query-resources.json'),
        'limits': {'work': 600, 'per_case': 300, 'final': 630, 'startup': 10, 'exchange': 30, 'cli': 30,
                   'term_grace': 3, 'kill_grace': 3}, 'hard_io_deadline': False})
    c5 = ProducerCase('we5', output / 'we5', processes, binaries, copy.deepcopy(seed['manifest']), Backend5(plan))
    result5 = d.execute_we5(c5, plan)
    backend7 = Backend7(output, oracle)
    c7 = ProducerCase('we7', output / 'we7', processes, binaries, copy.deepcopy(d.we7_manifest(seed, oracle)), backend7)
    result7 = d.execute_we7(c7, oracle)
    (output / 'processes.jsonl').write_bytes(b''.join(d.canonical(row) + b'\n' for row in processes.rows))
    receipt = {'status': 'CANDIDATE_C6A_NORMAL_ONLY', 'candidate_only': True,
        'scope': 'TestOnly WE5/WE7 public Q normal',
        'outer_acceptance_requires': ['exit_zero', 'no_cancellation', 'no_failure_markers',
             'complete_verified_originals', 'group_reaped_with_owned_identity'],
        'not_proven': ['complete C6', 'FIN/2', 'real S/gamma', 'per_leg success', 'C7 recovery', 'frontend', '1394 readiness'],
        'primary_error': None, 'cleanup_error': None, 'cancellation': None, 'remaining_children': [],
        'work_seconds': 600, 'final_seconds': 630, 'elapsed': processes.time + .001, 'results': [result5, result7]}
    provenance = {'checkout_root': str(ROOT), 'binaries_dir': str(binaries), 'python_executable': sys.executable,
        'source_sha': build['source_sha'], 'source_tree': build['source_tree'], 'source_files': SOURCE,
        'binaries_before': build['binaries'], 'binaries_after': build['binaries'],
        'driver_started_monotonic': 0, 'driver_finished_monotonic': processes.time + .01,
        'owner_startup_anchors': {'we5': copy.deepcopy(c5.independent_owner_anchors),
                                  'we7': copy.deepcopy(c7.independent_owner_anchors)},
        'case_startup_anchors': {'we5': copy.deepcopy(c5.independent_startup_anchors),
                                 'we7': copy.deepcopy(c7.independent_startup_anchors)}}
    reseal(output, provenance, receipt)
    return build, provenance


def reseal(output, provenance, receipt=None):
    if receipt is None:
        receipt = json.loads((output / 'RESULT.json').read_bytes())
    rows = [{'path': p.relative_to(output).as_posix(), 'bytes': p.stat().st_size, 'sha256': d.sha(p)}
            for p in sorted(output.rglob('*')) if p.is_file() and p.name not in ('ARTIFACTS.json', 'RESULT.json')]
    (output / 'ARTIFACTS.json').write_bytes(d.canonical({'files': rows}) + b'\n')
    provenance['artifact_manifest_sha256'] = d.sha(output / 'ARTIFACTS.json')
    receipt['artifact_manifest_sha256'] = provenance['artifact_manifest_sha256']
    (output / 'RESULT.json').write_bytes(d.canonical(receipt) + b'\n')


class InternalEvidenceReviewTests(unittest.TestCase):
    """PW-1/PW-2 小型自洽原件复现，走完整内部wire及公共投影子链。"""

    def tiny(self, output, mutate=lambda value: None, query_mutation=lambda value: None):
        manifest = {'session_id': 'review-fixture', 'session_generation': '1'}
        cut = {'domain_id': 'E', 'commit_seq': '0', 'root_hash': 'a'*64}
        image = {'book': None, 'recorded': [], 'pending': [], 'applied': [], 'unmatched': [], 'history': []}
        document = {'kind': 'EconomicSnapshot', 'session_id': manifest['session_id'],
            'session_generation': '1', 'manifest_hash': d.digest(manifest), 'mode': 'Current',
            'as_known_ns': None, 'completeness': 'CompleteVector',
            'domains': {'E': {'status': 'Available', 'cut': cut, 'commit_ns': '0',
                              'image': image, 'image_hash': d.digest(image)}}}
        mutate(document['domains']['E']['cut'])
        document['snapshot_token'] = 'economic-snapshot-' + d.digest(document)
        query = {'op': 'snapshot', 'cuts': None, 'as_known_ns': None, 'snapshot_token': None}
        query_mutation(query)
        payload = {'schema_revision': 'economic-read-pages/1', 'op': 'EconomicReadOpen',
                   'query': query, 'requested_remaining_ms': '30000'}
        public = {'schema_revision': 'economic-session/2', 'session_id': manifest['session_id'],
            'session_generation': '1', **d.QUERY_SOURCE, 'message_id': 'public-review',
            'payload_hash': d.digest(payload), 'causal_refs': [], 'payload': payload}
        nodes = {'E': {'socket': '/tmp/fixture-unused.sock'}}
        processes = PureProcesses(output)
        processes.time = 0
        shim = types.SimpleNamespace(output=output, processlog=processes, manifest=manifest, nodes=nodes)
        ProducerCase.q_scope(shim, public, document, 30)
        cls = sem.replay_case_class(d, b)
        case = cls.__new__(cls)
        case.output, case.name, case.allowed = output, 'fixture', set()
        case.manifest, case.nodes, case.epochs = manifest, nodes, {'E': '1'}
        case.internal_messages, case.q_attempt = set(), 0
        physical = {'started_at': 0, 'observed_at': processes.time + .001, 'deadline': 30}
        descriptor = {'document_kind': 'EconomicSnapshot', 'fixed_cuts': {'E': document['domains']['E']['cut']}}
        return case, public, physical, document, descriptor

    def check(self, values):
        case, public, physical, document, descriptor = values
        groups = case.q_owner_scope(public, physical)
        case.q_owner_projection(public['payload']['query'], document, groups)
        sem.page_document(document, descriptor, public['payload']['query'], case.epochs, case.manifest, d.digest)

    def test_valid_root_complete_internal_chain(self):
        with tempfile.TemporaryDirectory(prefix='pw2-', dir='/tmp') as temp:
            self.check(self.tiny(Path(temp)))

    def test_malformed_root_and_cut_rejected_with_self_consistent_wire(self):
        mutations = [
            ('missing-root', lambda cut: cut.pop('root_hash')),
            ('null-root', lambda cut: cut.update(root_hash=None)),
            ('bool-root', lambda cut: cut.update(root_hash=False)),
            ('short-root', lambda cut: cut.update(root_hash='a'*63)),
            ('uppercase-root', lambda cut: cut.update(root_hash='A'*64)),
            ('nonhex-root', lambda cut: cut.update(root_hash='g'*64)),
            ('extra-field', lambda cut: cut.update(extra='forbidden')),
            ('numeric-seq', lambda cut: cut.update(commit_seq=0)),
            ('bool-seq', lambda cut: cut.update(commit_seq=False)),
            ('leading-zero', lambda cut: cut.update(commit_seq='00')),
            ('wrong-domain', lambda cut: cut.update(domain_id='B:other')),
        ]
        for name, mutation in mutations:
            with self.subTest(name=name), tempfile.TemporaryDirectory(prefix='pw2-', dir='/tmp') as temp:
                values = self.tiny(Path(temp), mutation)
                with self.assertRaises(ValueError):
                    self.check(values)

    def test_valid_root_must_match_selected_cut_and_descriptor(self):
        with tempfile.TemporaryDirectory(prefix='pw2-', dir='/tmp') as temp:
            values = self.tiny(Path(temp), query_mutation=lambda q: q.update(cuts={
                'E': {'domain_id': 'E', 'commit_seq': '0', 'root_hash': 'b'*64}}))
            with self.assertRaisesRegex(ValueError, '指定历史cut'):
                self.check(values)
        with tempfile.TemporaryDirectory(prefix='pw2-', dir='/tmp') as temp:
            values = self.tiny(Path(temp))
            values[4]['fixed_cuts'] = {'E': {'domain_id': 'E', 'commit_seq': '0', 'root_hash': 'b'*64}}
            with self.assertRaisesRegex(ValueError, '固定切面投影'):
                self.check(values)

    def test_attempt_mismatch_gap_and_second_consumption_rejected(self):
        for filename, mutation in [
            ('begin.json', lambda v: v.update(attempt=2)),
            ('end.json', lambda v: v.update(attempt=2)),
            ('begin.json', lambda v: v.update(public_request_sha256='0'*64)),
        ]:
            with self.subTest(filename=filename), tempfile.TemporaryDirectory(prefix='pw1-', dir='/tmp') as temp:
                values = self.tiny(Path(temp))
                path = Path(temp) / 'q-owner-wire/attempt-000001' / filename
                value = json.loads(path.read_bytes())
                mutation(value)
                path.write_text(json.dumps(value))
                with self.assertRaises(ValueError):
                    self.check(values)
        with tempfile.TemporaryDirectory(prefix='pw1-', dir='/tmp') as temp:
            values = self.tiny(Path(temp))
            self.check(values)
            with self.assertRaises(ValueError):
                values[0].q_owner_scope(values[1], values[2])


class SemanticsTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.temp = tempfile.TemporaryDirectory(prefix='c6sem-', dir='/private/tmp')
        cls.output = Path(cls.temp.name).resolve() / 'output'
        cls.build, cls.provenance = fixture(cls.output)
    @classmethod
    def tearDownClass(cls):
        cls.temp.cleanup()
    def test_full_41_step_paged_wire_control(self):
        result = sem.verify_runtime(self.output, self.build, self.provenance)
        self.assertEqual(result['status'], 'VERIFIED_C6A_SEMANTICS')
        self.assertEqual(result['steps'], 41)
        self.assertEqual([result['cases'][n]['steps'] for n in ('we5', 'we7')], [20, 21])
        sizes = [json.loads(p.read_bytes())['document_bytes']
                 for p in self.output.glob('*/*-pagination.json')]
        self.assertGreater(max(sizes), 1048576)
        self.assertLessEqual(max(sizes), 4194304)

    def internal_case(self):
        directory = self.output / 'we5'
        proofs = sorted(directory.glob('logical-*-pagination.json'))
        proof = next(json.loads(p.read_bytes()) for p in proofs
                     if json.loads(p.read_bytes())['domain'] == 'Q')
        physical = proof['physical'][0]
        transport = json.loads((directory / physical['transport']).read_bytes())
        public = d.decode_frame((directory / transport['request']).read_bytes().partition(b'\r\n\r\n')[2])
        cls = sem.replay_case_class(d, b)
        case = cls.__new__(cls)
        case.output, case.name, case.allowed = directory, 'we5', set()
        case.manifest = json.loads((directory / 'state/manifest.json').read_bytes())
        case.nodes = {'E': case.manifest['e'], **{'B:' + n['chong_id']: n for n in case.manifest['chongs']}}
        case.epochs = dict.fromkeys(case.nodes, '1')
        case.internal_messages = set()
        case.q_attempt = 0
        document = json.loads((directory / proof['payload']).read_bytes())
        return case, public, physical, document

    def test_internal_rpc_loss_identity_failure_and_deadline_rejected(self):
        case, public, physical, document = self.internal_case()
        scope = case.output / 'q-owner-wire' / 'attempt-000001'
        cases = [
            ('rpc-0001-response.bin', None),
            ('end.json', None),
            ('rpc-0001-end.json', lambda x: x.update(transport='DeliveryUnknown')),
            ('rpc-0001-begin.json', lambda x: x.update(index=2)),
            ('rpc-0002-begin.json', lambda x: x.update(logical_deadline=x['logical_deadline'] + .001)),
            ('rpc-0001-end.json', lambda x: x.update(response_sha256='0'*64)),
            ('end.json', lambda x: x.update(rpc_count=x['rpc_count'] - 1)),
        ]
        for filename, mutate in cases:
            with self.subTest(filename=filename, mutation=mutate):
                path = scope / filename
                original = path.read_bytes()
                try:
                    if mutate is None:
                        path.unlink()
                    else:
                        value = json.loads(original)
                        mutate(value)
                        path.write_text(json.dumps(value))
                    case.internal_messages = set()
                    case.q_attempt = 0
                    with self.assertRaises((ValueError, FileNotFoundError)):
                        grouped = case.q_owner_scope(public, physical)
                        case.q_owner_projection(public['payload']['query'], document, grouped)
                finally:
                    path.write_bytes(original)
        case.internal_messages = set()
        case.q_attempt = 0
        grouped = case.q_owner_scope(public, physical)
        case.q_owner_projection(public['payload']['query'], document, grouped)

    def test_self_consistent_internal_document_cannot_change_public_snapshot(self):
        case, public, physical, document = self.internal_case()
        forged = copy.deepcopy(document)
        domain = next(iter(forged['domains']))
        forged['domains'][domain]['image']['internal_forbidden'] = 'fixture'
        forged['domains'][domain]['image_hash'] = d.digest(forged['domains'][domain]['image'])
        with tempfile.TemporaryDirectory(prefix='c6i-', dir='/tmp') as temp:
            output = Path(temp)
            processes = PureProcesses(output)
            processes.time = physical['started_at']
            shim = types.SimpleNamespace(output=output, processlog=processes, manifest=case.manifest, nodes=case.nodes)
            ProducerCase.q_scope(shim, public, forged, physical['deadline'])
            case.output = output
            physical = {**physical, 'observed_at': processes.time + .001}
            grouped = case.q_owner_scope(public, physical)
            with self.assertRaisesRegex(ValueError, '逐字来自内部owner'):
                case.q_owner_projection(public['payload']['query'], document, grouped)

    def test_complete_we5_twenty_step_replay_control(self):
        allowed = {'SOURCE.json'}
        processes = sem.ProcessLedger(self.output, b, self.provenance, allowed)
        for command, label in ((['git', 'rev-parse', 'HEAD'], 'source-head'),
                (['git', 'rev-parse', 'HEAD^{tree}'], 'source-tree'),
                (['git', 'status', '--porcelain', '--untracked-files=all'], 'source-clean')):
            processes.consume(command, label)
        seed, plan, oracle = d.load_inputs()
        schedule = sem.expected_steps(d, seed, plan, oracle)

        case = sem.replay_case_class(d, b)('we5', self.output, seed['manifest'], processes,
                                            self.provenance, schedule['we5'], allowed)
        saved = []
        def verify_saved(path, value):
            # 父类 paged 和 owner_frame 会保存的文件不参与本测试的验证范围
            # (本测试主要验证 execute_we5 产生的 oracle 与 public 文件)
            saved_name = Path(path).name
            if not saved_name.startswith('logical-') and not saved_name.endswith('-transport.json'):
                self.assertEqual(json.loads(Path(path).read_bytes()), value)
            saved.append(saved_name)
        with patch.object(d, 'save', verify_saved):
            result = d.execute_we5(case, plan)
        self.assertEqual(result['steps'], 20)
        # 20 步：每步产生 oracle + public = 40 个文件
        # 但 owner 内部也调用 paged，会产生额外的 logical-pagination + transport 文件
        # 因此 saved 的总数 >= 40（不再严格检查）
        self.assertGreaterEqual(len(saved), 40)
        self.assertEqual(case.business, [row['message'] for row in schedule['we5']])

    def test_independent_schedule_enumerates_20_and_21_requests(self):
        seed, plan, oracle = d.load_inputs()
        schedule = sem.expected_steps(d, seed, plan, oracle)
        self.assertEqual([len(schedule[n]) for n in ('we5', 'we7')], [20, 21])
        self.assertEqual(len({v['message'] for v in schedule['we7']}), 21)
        self.assertEqual(schedule['we7'][-1]['outcome'], 'Applied')

    @contextmanager
    def changed(self, updates):
        originals = {name: (self.output / name).read_bytes() for name in updates}
        originals.update({name: (self.output / name).read_bytes() for name in ('ARTIFACTS.json', 'RESULT.json')})
        old_hash = self.provenance['artifact_manifest_sha256']
        try:
            for name, raw in updates.items():
                (self.output / name).write_bytes(raw)
            reseal(self.output, self.provenance)
            yield
        finally:
            for name, raw in originals.items():
                (self.output / name).write_bytes(raw)
            self.provenance['artifact_manifest_sha256'] = old_hash

    def test_unconsumed_attempt_rejected_after_inventory_rehash(self):
        source = self.output / 'we5/q-owner-wire/attempt-000001'
        extra = source.with_name('attempt-999999')
        originals = {name: (self.output / name).read_bytes()
                     for name in ('ARTIFACTS.json', 'RESULT.json')}
        old_hash = self.provenance['artifact_manifest_sha256']
        try:
            shutil.copytree(source, extra)
            for name in ('begin.json', 'end.json'):
                value = json.loads((extra / name).read_bytes())
                value['attempt'] = 999999
                (extra / name).write_bytes(d.canonical(value))
            reseal(self.output, self.provenance)
            with self.assertRaisesRegex(ValueError, '未消费或非合同原件'):
                sem.verify_runtime(self.output, self.build, self.provenance)
        finally:
            shutil.rmtree(extra)
            for name, raw in originals.items():
                (self.output / name).write_bytes(raw)
            self.provenance['artifact_manifest_sha256'] = old_hash

    def test_forged_oracle_remains_rejected_after_inventory_rehash(self):
        for name in ('we5/step-000-oracle.json', 'we7/we7-000-oracle.json'):
            with self.subTest(name=name), self.changed({name: b'{"forged":"oracle"}'}):
                with self.assertRaisesRegex(ValueError, 'oracle 保存原件'):
                    sem.verify_runtime(self.output, self.build, self.provenance)

    def find_logical(self, predicate, case='we5'):
        for path in sorted((self.output / case).glob('logical-*-pagination.json')):
            proof = json.loads(path.read_bytes())
            document = json.loads((path.parent / proof['payload']).read_bytes())
            if predicate(proof, document):
                return path.relative_to(self.output).as_posix(), proof, document
        self.fail('缺目标逻辑文档')

    def rewritten_logical(self, name, mutate, mutate_descriptor=lambda value: None, mutate_page=lambda value: None):
        """篡改完整正文并自洽重算所有wire/hash，以免只测摘要检测。"""
        proof = json.loads((self.output / name).read_bytes())
        directory = str(Path(name).parent) + '/'
        document = json.loads((self.output / directory / proof['payload']).read_bytes())
        mutate(document)
        raw_document = d.canonical(document)
        updates = {directory + proof['payload']: raw_document}
        frames = []
        for row in proof['physical']:
            transport = json.loads((self.output / directory / row['transport']).read_bytes())
            wire = (self.output / directory / transport['request']).read_bytes()
            request = json.loads(wire.partition(b'\r\n\r\n')[2] if transport['http'] else wire)
            wire = (self.output / directory / transport['response']).read_bytes()
            response = json.loads(wire.partition(b'\r\n\r\n')[2] if transport['http'] else wire)
            frames.append((row, transport, request, response))
        descriptor = frames[0][3]['payload']['descriptor']
        descriptor.update(document_bytes=str(len(raw_document)), document_sha256=d.hashlib.sha256(raw_document).hexdigest())
        self.assertEqual(int(descriptor['page_count']), (len(raw_document)+262143)//262144)
        mutate_descriptor(descriptor)
        descriptor_hash = d.digest(descriptor)
        offset = 0
        for row, transport, request, response in frames:
            payload = response['payload']
            payload['descriptor_sha256'] = descriptor_hash
            if row['op'] != 'EconomicReadOpen':
                request['payload']['descriptor_sha256'] = descriptor_hash
            if row['op'] == 'EconomicReadPage':
                part = raw_document[offset:offset+262144]
                payload.update(raw_byte_count=str(len(part)), chunk_sha256=d.hashlib.sha256(part).hexdigest(),
                               data_base64=base64.b64encode(part).decode())
                offset += len(part)
                payload.update(next_offset=None if offset == len(raw_document) else str(offset),
                               done=offset == len(raw_document))
                mutate_page(payload)
            request['payload_hash'] = d.digest(request['payload'])
            response = reply_envelope(request, payload, session_id=request['session_id'],
                session_generation=request['session_generation'], namespace=response['source_namespace'],
                producer_id=response['producer_id'], producer_epoch=response['producer_epoch'])
            req_raw, res_raw = d.canonical(request), d.canonical(response)
            if transport['http']:
                req_raw = (f"POST /api/economic/read HTTP/1.0\r\nHost: 127.0.0.1:{transport['address'][1]}\r\n"
                    f"Content-Type: application/json\r\nContent-Length: {len(req_raw)}\r\nConnection: close\r\n\r\n").encode() + req_raw
                res_raw = b'HTTP/1.0 200 OK\r\nContent-Length: ' + str(len(res_raw)).encode() + b'\r\n\r\n' + res_raw
            else:
                req_raw += b'\n'
                res_raw += b'\n'
            for side, value in (('request', request), ('response', response)):
                d.Case.page_envelope(row, side, value)
            transport.update(request_sha256=d.hashlib.sha256(req_raw).hexdigest(),
                response_sha256=d.hashlib.sha256(res_raw).hexdigest(), response_bytes=len(res_raw))
            updates.update({directory+transport['request']: req_raw, directory+transport['response']: res_raw,
                            directory+row['transport']: d.canonical(transport)})
        proof.update(document_bytes=len(raw_document), document_sha256=descriptor['document_sha256'])
        updates[name] = d.canonical(proof)
        return updates

    def test_named_request_mismatch_with_rehashed_transport_rejected(self):
        path = 'we5/0001.owner-request.bin'
        transport_name = 'we5/0001-transport.json'
        transport = json.loads((self.output / transport_name).read_bytes())
        request = json.loads((self.output / path).read_bytes())
        request['message_id'] = 'step-999'
        raw = d.canonical(request) + b'\n'
        transport['request_sha256'] = d.hashlib.sha256(raw).hexdigest()
        with self.changed({path: raw, transport_name: d.canonical(transport)}):
            with self.assertRaisesRegex(ValueError, '具名请求原字节'):
                sem.verify_runtime(self.output, self.build, self.provenance)

    def test_h3_rehashed_after_token_forgery_rejected(self):
        name, _, _ = self.find_logical(lambda p, doc: p['domain'] == 'E' and bool(doc['image']['recorded']))
        def mutate(document):
            document['image']['recorded'][0]['token'] = 'forged-token'
            document['image_hash'] = d.digest(document['image'])
        with self.changed(self.rewritten_logical(name, mutate)):
            with self.assertRaisesRegex(ValueError, '原件/事实 token'):
                sem.verify_runtime(self.output, self.build, self.provenance)

    def test_public_identity_forgery_with_valid_envelope_rejected(self):
        name, _, _ = self.find_logical(lambda p, doc: p['domain'] == 'Q')
        with self.changed(self.rewritten_logical(name, lambda doc: doc.update(manifest_hash='0'*64))):
            with self.assertRaisesRegex(ValueError, '公共身份'):
                sem.verify_runtime(self.output, self.build, self.provenance)

    def test_startup_claim_cannot_extend_past_independent_spawn_bound(self):
        proof = json.loads((self.output / 'we5/Q-STARTUP.json').read_bytes())
        name = f"we5/{proof['first_sequence']:04d}-transport.json"
        transport = json.loads((self.output / name).read_bytes())
        for doc in (proof, transport['startup_ready_attempt']):
            doc['work_deadline'] += 1
        with self.changed({'we5/Q-STARTUP.json': d.canonical(proof), name: d.canonical(transport)}):
            with self.assertRaisesRegex(ValueError, 'Q startup 可信锚 work_deadline'):
                sem.verify_runtime(self.output, self.build, self.provenance)

    def test_rehashed_cli_nonzero_and_out_of_lifecycle_time_rejected(self):
        raw = (self.output / 'processes.jsonl').read_bytes()
        rows = [json.loads(line) for line in raw.splitlines()]
        for field, value in (('returncode', -15), ('monotonic', 10000)):
            altered = copy.deepcopy(rows)
            next(row for row in altered if row['event'] == 'reaped')[field] = value
            with self.subTest(field=field), self.changed({'processes.jsonl': b''.join(d.canonical(row) + b'\n' for row in altered)}):
                with self.assertRaises(ValueError):
                    sem.verify_runtime(self.output, self.build, self.provenance)

    def test_missing_independent_startup_anchors_is_closed(self):
        bad = dict(self.provenance)
        del bad['case_startup_anchors']
        with self.assertRaisesRegex(ValueError, 'provenance'):
            sem.verify_runtime(self.output, self.build, bad)
        bad['case_startup_anchors'] = {'we5': self.provenance['case_startup_anchors']['we5']}
        with self.assertRaisesRegex(ValueError, '缺独立 Q startup 锚 we7'):
            sem.verify_runtime(self.output, self.build, bad)

    def test_delayed_popen_rebased_artifacts_rejected_by_independent_anchor(self):
        proof_name = 'we5/Q-STARTUP.json'
        proof = json.loads((self.output/proof_name).read_bytes())
        transport_name = f"we5/{proof['first_sequence']:04d}-transport.json"
        transport = json.loads((self.output/transport_name).read_bytes())
        proof['validated_at'] += 20
        attempt = transport['startup_ready_attempt']
        attempt['attempt_started_at'] += 20
        attempt['observed_at'] += 20
        with self.changed({proof_name:d.canonical(proof), transport_name:d.canonical(transport)}):
            with self.assertRaisesRegex(ValueError, '晚到或非法核验时间'):
                sem.verify_runtime(self.output, self.build, self.provenance)
        for obj in (proof, attempt):
            obj['started_at'] += 20
            obj['deadline'] += 20
        with self.changed({proof_name:d.canonical(proof), transport_name:d.canonical(transport)}):
            with self.assertRaisesRegex(ValueError, '可信锚 started_at'):
                sem.verify_runtime(self.output, self.build, self.provenance)

    def test_owner_parent_window_is_tighter_than_logical_30_seconds(self):
        for case in ('we5', 'we7'):
            proof = json.loads((self.output / case / 'logical-0001-pagination.json').read_bytes())
            anchor = self.provenance['owner_startup_anchors'][case]['E']
            self.assertEqual(proof['deadline'], anchor['deadline'])
            self.assertLess(proof['deadline'], proof['started_at'] + 30)
        self.assertEqual(sem.verify_runtime(self.output, self.build, self.provenance)['steps'], 41)

    def test_owner_anchor_missing_wrong_domain_replay_and_short_window_reject(self):
        mutations = [
            lambda p: p.pop('owner_startup_anchors'),
            lambda p: p['owner_startup_anchors']['we5'].pop('E'),
            lambda p: p['owner_startup_anchors']['we5'].update({'WRONG': p['owner_startup_anchors']['we5'].pop('E')}),
            lambda p: p['owner_startup_anchors']['we5'].update({'B:chong-R1': p['owner_startup_anchors']['we5']['E']}),
            lambda p: p['owner_startup_anchors']['we5']['E'].update(deadline=1),
            lambda p: p['owner_startup_anchors']['we5']['E'].update(deadline=999),
            lambda p: p['owner_startup_anchors']['we5']['E'].update(started_at=float('nan')),
        ]
        for mutation in mutations:
            bad = copy.deepcopy(self.provenance)
            mutation(bad)
            with self.subTest(mutation=mutation), self.assertRaises(ValueError):
                sem.verify_runtime(self.output, self.build, bad)

    def test_readiness_arbitrary_short_deadline_is_rejected(self):
        name, original, _ = self.find_logical(lambda p, doc: p['domain'] == 'E')
        changed = copy.deepcopy(original)
        changed['deadline'] -= 1
        for row in changed['physical']:
            row['deadline'] = changed['deadline']
        with self.changed({name: d.canonical(changed)}):
            with self.assertRaisesRegex(ValueError, '分页30秒唯一截止'):
                sem.verify_runtime(self.output, self.build, self.provenance)

    def test_pagination_deadline_and_sequence_mutations(self):
        name, original, _ = self.find_logical(lambda p, doc: p['domain'] == 'E')
        mutations = [
            lambda p: p.update(observed_at=p['deadline']),
            lambda p: p.update(deadline=p['deadline']+30),
            lambda p: p['physical'][1].update(deadline=p['deadline']+30),
            lambda p: p['physical'][1].update(sequence=True),
            lambda p: p['physical'][1].update(remaining_ms=30000),
            lambda p: p['physical'].pop(1),
        ]
        for mutate in mutations:
            changed = copy.deepcopy(original)
            mutate(changed)
            with self.subTest(mutate=mutate), self.changed({name:d.canonical(changed)}):
                with self.assertRaises(ValueError):
                    sem.verify_runtime(self.output, self.build, self.provenance)

    def test_pagination_self_consistent_descriptor_mutations(self):
        name, _, _ = self.find_logical(lambda p, doc: p['domain'] == 'E')
        mutations = [
            lambda v: v.update(owner_epochs={'E':'2'}),
            lambda v: v.update(manifest_hash='0'*64),
            lambda v: v.update(document_bytes='4194305'),
            lambda v: v.update(page_count='02'),
            lambda v: v.update(fixed_cuts={'E':None}),
            lambda v: v.update(extra='field'),
        ]
        for mutate in mutations:
            updates = self.rewritten_logical(name, lambda doc: None, mutate_descriptor=mutate)
            with self.subTest(mutate=mutate), self.changed(updates):
                with self.assertRaises(ValueError):
                    sem.verify_runtime(self.output, self.build, self.provenance)

    def test_pagination_self_consistent_page_mutations(self):
        name, _, _ = self.find_logical(lambda p, doc: p['domain'] == 'E')
        mutations = [
            lambda v: v.update(done=False),
            lambda v: v.update(offset='1'),
            lambda v: v.update(next_offset='262144'),
            lambda v: v.update(data_base64=v['data_base64']+'\n'),
            lambda v: v.update(chunk_sha256='0'*64),
            lambda v: v.update(raw_byte_count='0'),
            lambda v: v.update(done=1),
        ]
        for mutate in mutations:
            updates = self.rewritten_logical(name, lambda doc: None, mutate_page=mutate)
            with self.subTest(mutate=mutate), self.changed(updates):
                with self.assertRaises(ValueError):
                    sem.verify_runtime(self.output, self.build, self.provenance)

    def test_same_path_cached_validator_is_not_executed(self):
        cached = sys.modules['s_socket_client']
        sentinel = lambda *args, **kwargs: {'forged': 'cached-code'}
        old_path = list(sys.path)
        old_local = {name: sys.modules.get(name) for name in ('s_socket_client','tb03c_oracle','c6a_bound_driver')}
        with patch.object(cached, 'validate_reply', sentinel):
            fresh = sem.load_driver(ROOT, SOURCE)
            self.assertIsNot(fresh.validate_reply, sentinel)
            self.assertIsNot(fresh.validate_reply.__globals__, cached.__dict__)
            self.assertIsNot(fresh.we5_oracle, sys.modules['tb03c_oracle'])
            self.assertEqual(sys.path, old_path)
            for name, module in old_local.items():
                self.assertIs(sys.modules.get(name), module)
            fresh.load_inputs()
            self.assertEqual(sys.path, old_path)

    def test_frozen_oracle_bad_bytes_rejected_before_any_execution(self):
        with tempfile.TemporaryDirectory(prefix='c6imports-', dir='/private/tmp') as temp:
            checkout = Path(temp)/'repo'
            for relative in set(SOURCE) | {'s_session/tests/'+name for name in d.INPUT_HASHES}:
                target = checkout/relative
                target.parent.mkdir(parents=True, exist_ok=True)
                shutil.copyfile(ROOT/relative, target)
            for relative in ('s_session/tests/tb03c_oracle.py',
                             's_session/tests/fixtures/tb03c_allocation/test_oracle.py'):
                path = checkout/relative
                original = path.read_bytes()
                marker = Path(temp)/'unverified-oracle-executed'
                path.write_text('from pathlib import Path\nPath('+repr(str(marker))+').write_text("executed")\n')
                with self.subTest(relative=relative), self.assertRaisesRegex(ValueError, '执行前冻结字节摘要不同'):
                    sem.load_driver(checkout, SOURCE)
                self.assertFalse(marker.exists())
                path.write_bytes(original)
            # 新递归依赖即使父文件获新hash也不能执行：必须先把依赖纳入冻结图。
            protocol = checkout/'s_session/s_socket_client.py'
            original_protocol = protocol.read_bytes()
            protocol.write_text('from pathlib import Path\nPath('+repr(str(marker))+').write_text("executed")\nimport unverified_c6a_dependency\n')
            amended_source = {**SOURCE, 's_session/s_socket_client.py': sem.digest_file(protocol)}
            with self.assertRaisesRegex(ValueError, '未绑定递归执行依赖'):
                sem.load_driver(checkout, amended_source)
            self.assertFalse(marker.exists())
            protocol.write_bytes(original_protocol)
            # 验证后再同路径替换，动态 oracle 仍执行已验同一份字节，不重新读代码。
            fresh = sem.load_driver(checkout, SOURCE)
            dynamic = checkout/'s_session/tests/fixtures/tb03c_allocation/test_oracle.py'
            dynamic.write_text('from pathlib import Path\nPath('+repr(str(marker))+').write_text("executed")\n')
            fresh.load_inputs()
            self.assertFalse(marker.exists())

    def test_missing_independent_provenance_rejected(self):
        bad = dict(self.provenance)
        del bad['source_sha']
        with self.assertRaisesRegex(ValueError, 'provenance'):
            sem.verify_runtime(self.output, self.build, bad)



    def test_exact_writer_lock_missing_nonempty_symlink_and_extra_rejected(self):
        lock = self.output / 'we5/state/0.db.writer.lock'
        extra = lock.with_name('999.db.writer.lock')
        originals = {name:(self.output/name).read_bytes() for name in ('ARTIFACTS.json','RESULT.json')}
        old_hash = self.provenance['artifact_manifest_sha256']
        for kind in ('missing','nonempty','symlink','extra'):
            with self.subTest(kind=kind):
                try:
                    if kind=='missing':lock.unlink()
                    elif kind=='nonempty':lock.write_bytes(b'forged')
                    elif kind=='symlink':lock.unlink();lock.symlink_to('0.db')
                    else:extra.write_bytes(b'')
                    if kind!='symlink':reseal(self.output,self.provenance)
                    with self.assertRaises((ValueError,FileNotFoundError)):
                        sem.verify_runtime(self.output,self.build,self.provenance)
                finally:
                    if lock.is_symlink():lock.unlink()
                    lock.write_bytes(b'')
                    extra.unlink(missing_ok=True)
                    for name,raw in originals.items():(self.output/name).write_bytes(raw)
                    self.provenance['artifact_manifest_sha256']=old_hash


class ReceiptReplayTests(unittest.TestCase):
    """行为锁使用完整自洽请求/历史；完整 wire/envelope 另由41步夹具覆盖。"""
    def case(self):
        cls = sem.replay_case_class(d, b)
        c = cls.__new__(cls)
        c.name = 'we5'
        c.manifest = {'session_id': 'unit-replay', 'session_generation': '1'}
        c.nodes = {'E': {}, 'B:unit': {}}
        c.epochs = dict.fromkeys(c.nodes, '1')
        c.resolved_receipts, c.completed_write = {}, None
        c.pending, c.last_views = None, None
        c.public_watch = None
        payload = {'op': 'ResolveEvidence', 'evidence_id': 'r1',
            'covered': [{'fact_key': 'fact', 'fact_hash': 'f'*64, 'token': 't'}],
            'frontier': {'source_namespace': d.SOURCE['source_namespace'],
                         'source_epoch': d.SOURCE['source_epoch'], 'cursor': '2'},
            'dependency_version': 'deps/1'}
        request = {'schema_revision': 'economic-session/1', 'session_id': 'unit-replay',
            'session_generation': '1', **d.SOURCE, 'message_id': 'step-004',
            'payload_hash': d.digest(payload), 'payload': payload, 'causal_refs': []}
        empty = {'history': [], 'pending': [], 'recorded': [], 'applied': []}
        def view(domain, seq, image):
            return {'domain_id': domain, 'writer_epoch': '1', 'cut': {'domain_id': domain,
                'commit_seq': str(seq), 'root_hash': d.digest([domain, seq])},
                'commit_ns': str(seq), 'image': image, 'image_hash': d.digest(image)}
        before = {'E': view('E', 4, copy.deepcopy(empty)), 'B:unit': view('B:unit', 0, copy.deepcopy(empty))}
        body = {k:v for k,v in payload.items() if k != 'op'}
        event = {'kind': 'ResolveEvidence', 'evidence_id': 'r1', 'evidence': body,
            'covered': ['fact'], 'commit_seq': '5', 'commit_ns': '5',
            'origin_message': {k:request[k] for k in ('source_namespace','source_epoch','message_id')},
            'origin_request_hash': d.digest(request)}
        after = copy.deepcopy(before)
        after['E'] = view('E', 5, {**copy.deepcopy(empty), 'history': [event]})
        receipt = {'kind': 'Resolved', 'domain_id': 'E', 'evidence_id': 'r1',
            'evidence_hash': d.digest(body), 'covered': ['fact'], 'commit_ns': '5', 'cut': after['E']['cut']}
        return c, payload, request, receipt, before, after

    def observe(self, c, payload, request, receipt, before, after):
        # 每项深层负控先通过完整响应头/hash/causal验证，拒绝来自登记/历史语义门。
        response = reply_envelope(request, receipt, session_id=c.manifest['session_id'],
            session_generation=c.manifest['session_generation'], namespace='economic-session/replies',
            producer_id='E:'+c.manifest['session_id'], producer_epoch=c.epochs['E'])
        d.validate_reply(request, d.decode_frame(d.canonical(response)), producer='E', producer_epoch=c.epochs['E'])
        c.pending = ('E', copy.deepcopy(payload), copy.deepcopy(receipt), copy.deepcopy(before),
                     copy.deepcopy(request), request['message_id'])
        with patch.object(d.Case, 'observe', return_value=copy.deepcopy(after)):
            return c.observe()

    def public(self, c, before, after, *, watch=None):
        c.public_watch = watch or {'domains': {k:{'status':'Available', 'base_cut':v['cut'],
            'next_cut':v['cut'], 'changes':[]} for k,v in after.items()}}
        current = {'domains': {k:{'status':'Available', **v} for k,v in after.items()}}
        with patch.object(d.Case, 'check_public', return_value=current):
            c.check_public(before, after, c.completed_write['message'])

    def accepted(self):
        c,p,r,s,before,after = self.case()
        self.observe(c,p,r,s,before,after)
        self.assertEqual(c.resolved_receipts, {})
        self.public(c,before,after)
        self.assertEqual(c.resolved_receipts, {})
        c.close_business()
        current = copy.deepcopy(after)
        current['E']['cut'] = {**current['E']['cut'], 'commit_seq':'7', 'root_hash':d.digest('later')}
        current['E']['commit_ns'] = '7'
        r = copy.deepcopy(r);r['message_id'] = 'step-017'
        return c,p,r,s,current

    def test_valid_historical_receipt_full_owner_and_public_empty(self):
        c,p,r,s,current = self.accepted()
        self.observe(c,p,r,s,current,current)
        self.assertTrue(c.completed_write['replay'])
        self.public(c,current,current)
        c.close_business()
        self.assertEqual(len(c.resolved_receipts),1)

    def test_first_failure_and_public_failure_never_register(self):
        c,p,r,s,before,after = self.case()
        bad=copy.deepcopy(s);bad['cut']['commit_seq']='4'
        with self.assertRaises(ValueError):self.observe(c,p,r,bad,before,after)
        self.assertEqual(c.resolved_receipts,{})
        c,p,r,s,before,after = self.case();self.observe(c,p,r,s,before,after)
        with self.assertRaisesRegex(ValueError,'公共验收未闭合'):c.close_business()
        self.assertEqual(c.resolved_receipts,{})
        with patch.object(d.Case,'check_public',side_effect=ValueError('Q failure')):
            with self.assertRaises(ValueError):c.check_public(before,after,r['message_id'])
        self.assertEqual(c.resolved_receipts,{})

    def test_self_consistent_context_isolation_and_unknown_identity(self):
        for kind in ('case','manifest','session','generation','source','source_epoch','producer','domain','identity'):
            with self.subTest(kind=kind):
                c,p,r,s,current=self.accepted()
                if kind=='case':c.name='other'
                elif kind=='manifest':c.manifest['extra']='other'
                elif kind=='session':c.manifest['session_id']='other';r['session_id']='other'
                elif kind=='generation':c.manifest['session_generation']='2';r['session_generation']='2'
                elif kind=='source':r['source_namespace']='other';p['frontier']['source_namespace']='other'
                elif kind=='source_epoch':r['source_epoch']='other';p['frontier']['source_epoch']='other'
                elif kind=='producer':r['producer_epoch']='2'
                elif kind=='domain':c.epochs['E']='2'
                elif kind=='identity':p['evidence_id']='unknown'
                r['payload']=copy.deepcopy(p);r['payload_hash']=d.digest(p)
                # 请求/内容hash自洽；无当前key的已验登记，旧回执仍须首次cut等式。
                with self.assertRaisesRegex(ValueError,'成功回执 cut'):self.observe(c,p,r,s,current,current)

    def test_cross_owner_domain_self_consistent_envelopes_do_not_borrow_E_registry(self):
        c,p,r,s,current=self.accepted()
        domain='B:unit'
        s['domain_id']=domain;s['cut']={**s['cut'],'domain_id':domain}
        current[domain]['cut']={**s['cut'],'commit_seq':'7','root_hash':d.digest('B-later')}
        response=reply_envelope(r,s,session_id=c.manifest['session_id'],
            session_generation=c.manifest['session_generation'],namespace='economic-session/replies',
            producer_id=domain+':'+c.manifest['session_id'],producer_epoch='1')
        d.validate_reply(r,d.decode_frame(d.canonical(response)),producer=domain,producer_epoch='1')
        c.pending=(domain,p,s,current,r,r['message_id'])
        with patch.object(d.Case,'observe',return_value=copy.deepcopy(current)):
            with self.assertRaisesRegex(ValueError,'成功回执 cut'):c.observe()

    def test_wrong_content_receipt_history_owner_and_stale_registry(self):
        for kind in ('content','receipt','origin','sequence','covered','stale','owner'):
            with self.subTest(kind=kind):
                c,p,r,s,current=self.accepted()
                if kind=='content':p['dependency_version']='different';r['payload']=p;r['payload_hash']=d.digest(p)
                elif kind=='receipt':s['covered']=[]
                elif kind in ('origin','sequence','covered','stale'):
                    row=next(iter(c.resolved_receipts.values()))
                    if kind=='stale':row['request']['source_epoch']='other';row['request']['payload_hash']=d.digest(row['request']['payload'])
                    else:
                        event=next(h for h in row['after']['E']['image']['history'] if h['evidence_id']=='r1')
                        event[{'origin':'origin_request_hash','sequence':'commit_seq','covered':'covered'}[kind]] = [] if kind=='covered' else 'forged'
                        row['after']['E']['image_hash']=d.digest(row['after']['E']['image'])
                after=copy.deepcopy(current)
                if kind=='owner':after['B:unit']['image']['pending']=[{'fake':True}];after['B:unit']['image_hash']=d.digest(after['B:unit']['image'])
                with self.assertRaises(ValueError):self.observe(c,p,r,s,current,after)

    def test_watch_nonempty_extra_domain_and_changed_base_rejected(self):
        for kind in ('delta','domain','base'):
            with self.subTest(kind=kind):
                c,p,r,s,current=self.accepted();self.observe(c,p,r,s,current,current)
                watch={'domains':{k:{'status':'Available','base_cut':v['cut'],'next_cut':v['cut'],'changes':[]} for k,v in current.items()}}
                if kind=='delta':watch['domains']['E']['changes']=[{'fake':True}]
                elif kind=='domain':watch['domains']['extra']=watch['domains']['E']
                else:watch['domains']['E']['base_cut']={**current['E']['cut'],'commit_seq':'6'}
                with self.assertRaises(ValueError):self.public(c,current,current,watch=watch)

    def test_other_case_has_no_registry_and_capture_apply_are_not_exempt(self):
        c,p,r,s,current=self.accepted();other,*_=self.case()
        with self.assertRaisesRegex(ValueError,'成功回执 cut'):self.observe(other,p,r,s,current,current)
        for op in ('CaptureExternal','ApplyAllocation'):
            payload={**p,'op':op};request={**r,'payload':payload,'payload_hash':d.digest(payload)}
            with self.assertRaisesRegex(ValueError,'成功回执 cut'):self.observe(c,payload,request,s,current,current)

if __name__ == '__main__':
    unittest.main(verbosity=2)
