"""纯离线反例：无真实 SQLite、socket、Controller、服务或 Prime。"""
import copy
import contextlib
import io
import importlib.util
import json
import os
import signal
import sys
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

sys.dont_write_bytecode = True
from evidence import (BINARY_SHA, CLOCK_SHA, COUNT, SOURCE_SHA, SCOPE,
                      Rejected, canonical, cell, completion, decode, native_frame,
                      sha, validate_contract, validate_material, validate_receipt, validate_final,
                      validate_snapshot, table_rows, exact)
import runner


def fixture(count=3):
    """小型协议材料，不使用生产运行默认值；所有证据只在内存。"""
    lines, raws, rows, index, events = [], [], [], [], {}
    for n in range(1, count + 1):
        event = dict(seq=str(n - 1), event_id=f'long-raw-{n - 1}', revision='1',
                     received_at='2026-09-14T00:00:00Z', timestamp=str(1700000000 + n), volume='1',
                     raw_text='原 token ' + str(n), open='1', high='3', low='1', close='2')
        raw = dict(schema_revision='s-ohlc/1', events=[event], session_id='fixture',
                   source_namespace='fixture', source_epoch='1', instrument='FIXTURE.LONG',
                   profile='ohlc_integer_tb02a_v1')
        payload = dict(op='ingest', clock_event_id=f'input-{n}', raw_input=raw,
                       target_session_id='fixture', target_session_generation='1', writer_epoch='1')
        env = dict(schema_revision='s-session/3', message_id=f'message-{n}', session_id='fixture',
                   session_generation='1', source_namespace='fixture', source_epoch='1',
                   producer_id='fixture', producer_epoch='1', payload=payload,
                   payload_hash=sha(canonical(payload)), causal_refs=[])
        line = canonical(env) + b'\n'
        lines.append(line)
        raws.append(canonical(raw) + b'\n')
        rows.append(dict(action='exchange_once_then_validate_committed_before_next', retries='none',
                         ordinal=n, expected_generation=str(n), message_id=env['message_id'],
                         clock_event_id=payload['clock_event_id'], request_sha256=sha(line[:-1])))
        index.append(dict(ordinal=n, message_id=env['message_id'], envelope_sha256=sha(line[:-1]),
                          payload_sha256=env['payload_hash'], raw_input_sha256=sha(canonical(raw)),
                          raw_text_sha256=sha(event['raw_text'].encode()), source_index=str(n - 1),
                          body_bytes=len(line) - 1, lf_frame_bytes=len(line)))
        events[f'input-{n}'] = dict(accept_ns=str(n * 3), attempts=[dict(begin_ns=str(n * 3 + 1), commit_ns=str(n * 3 + 2))])
    clock = canonical(dict(schema_revision='s-clock-plan/1', unit='ns', events=events))
    schedule = dict(clock_sha256=sha(clock), ordinal_order=rows)
    return [b''.join(lines), schedule, clock, b''.join(raws), index]


def contract_fixture():
    contract, _, _ = runner.root_inputs()
    material, _ = runner.material_at(runner.INPUTS)
    return copy.deepcopy(contract), material, contract


def table(rows, names=None):
    names = names or list(rows[0])
    encoded = sorted([[cell(row[key]) for key in names] for row in rows], key=canonical)
    return dict(names=names, rows=encoded, row_count=len(rows), sha256=sha(canonical(encoded)),
                columns=[[n, name, 'fixture', 0, None, 0, 0] for n, name in enumerate(names)])


def committed_fixture():
    data = fixture(1)
    env = decode(data[0])
    payload = dict(kind='Committed', ok=True, original_message_id=env['message_id'], receipt_id='receipt-1', accepted_seq='0',
                   publication=dict(generation='1', index_frontier='batch-1', structure_cut='cut-1', catalog_revision='fixture', input_frontier='0'))
    saved = dict(receipt=table([dict(source_namespace=env['source_namespace'], source_epoch=env['source_epoch'],
                                   message_id=env['message_id'], payload_hash=env['payload_hash'],
                                   canonical_envelope=data[0][:-1].decode(), status='Committed', receipt_id='receipt-1',
                                   first_published_generation=1, clock_event_id='input-1', accepted_seq=0, attempt_count=1)]),
                 publication=table([dict(generation=1, batch_id='batch-1')]),
                 delta=table([dict(generation=1, index_frontier='batch-1', next_cut='cut-1', catalog_revision='fixture', input_frontier=0)]),
                 protocol=table([dict(protocol_revision='s-session/3', session_generation='1',
                                      clock_plan_hash=sha(data[2]), clock_plan_json=data[2].decode(), next_attempt_ordinal=2, logical_phase_frontier='5')]),
                 clock_events=table([dict(clock_event_id='input-1', phase=phase, semantic_ns=str(3 + n), attempt_index=-1 if phase == 'accept' else 0,
                                         attempt_ordinal=None if phase == 'accept' else 1, generation=0 if phase == 'accept' else 1,
                                         message_key=canonical(['fixture', '1', 'message-1']).decode())
                                     for n, phase in enumerate(('accept', 'begin', 'commit'))]))
    return data, payload, saved


def snapshot_metadata(snapshot):
    snapshot['schema'] = table([dict(type='table', name=name, tbl_name=name, rootpage=i + 1, sql='fixture SQL ' + name)
                               for i, name in enumerate(snapshot['tables'])])
    snapshot['db_properties'] = dict(user_version=0, application_id=0, encoding='UTF-8', page_size=4096, auto_vacuum=0)
    return snapshot


class MaterialTests(unittest.TestCase):
    def setUp(self):
        self.data = fixture()

    def validate(self):
        return validate_material(*self.data, count=3)

    def test_complete_valid_and_tail(self):
        self.assertEqual(self.validate()['count'], 3)
        self.data[0] = self.data[0][:-5]
        with self.assertRaises(Rejected):
            self.validate()

    def test_missing_one_request(self):
        self.data[0] = b''.join(self.data[0].splitlines(keepends=True)[:-1])
        with self.assertRaises(Rejected):
            self.validate()

    def test_request_reordered_with_same_clock(self):
        lines = self.data[0].splitlines(keepends=True)
        lines[0], lines[1] = lines[1], lines[0]
        self.data[0] = b''.join(lines)
        with self.assertRaises(Rejected):
            self.validate()

    def test_wrong_request_hash(self):
        self.data[1]['ordinal_order'][1]['request_sha256'] = '0' * 64
        with self.assertRaises(Rejected):
            self.validate()

    def test_same_clock_swapped_schedule(self):
        rows = self.data[1]['ordinal_order']
        rows[0], rows[1] = rows[1], rows[0]
        with self.assertRaises(Rejected):
            self.validate()

    def test_duplicate_json_state_key(self):
        with self.assertRaises(Rejected):
            decode(b'{"state":"running","state":"stopped"}')

    def test_clock_phase_reversal_even_with_new_matching_hash(self):
        clock = decode(self.data[2])
        clock['events']['input-2']['attempts'][0]['begin_ns'] = '1'
        self.data[2] = canonical(clock)
        self.data[1]['clock_sha256'] = sha(self.data[2])
        with self.assertRaises(Rejected):
            self.validate()


class ContractTests(unittest.TestCase):
    def setUp(self):
        self.contract, self.material, self.frozen = contract_fixture()

    def validate(self, contract):
        return validate_contract(contract, self.material, self.frozen)

    def test_frozen_root_accepts_without_capacity_or_peak_certificate(self):
        self.validate(self.contract)
        for key in ('capacity_proof', 'strict_peak', 'budgets'):
            self.assertNotIn(key, self.contract)

    def test_each_S7_missing_or_changed_rejected(self):
        for field in self.contract['s_resources']:
            for operation in ('missing', 'change'):
                broken = copy.deepcopy(self.contract)
                if operation == 'missing':
                    del broken['s_resources'][field]
                else:
                    broken['s_resources'][field] = '999999'
                with self.subTest(field=field, operation=operation), self.assertRaises(Rejected):
                    self.validate(broken)

    def test_all_caller_fields_bound_with_original_types(self):
        for key, value in self.contract['caller_observation'].items():
            broken = copy.deepcopy(self.contract)
            broken['caller_observation'][key] = False if type(value) is str else 'changed'
            with self.subTest(key=key), self.assertRaises(Rejected):
                self.validate(broken)
        for value in (1, 1.0):
            broken = copy.deepcopy(self.contract)
            broken['caller_observation']['no_retries'] = value
            with self.assertRaises(Rejected):
                self.validate(broken)

    def test_duplicate_domains_and_reorder_rejected(self):
        for key in ('label', 'db', 'state_dir', 'socket'):
            broken = copy.deepcopy(self.contract)
            broken['domains'][1][key] = broken['domains'][0][key]
            with self.subTest(key=key), self.assertRaises(Rejected):
                self.validate(broken)
        broken = copy.deepcopy(self.contract)
        broken['domains'].reverse()
        with self.assertRaises(Rejected):
            self.validate(broken)

    def test_wrong_root_SHA_or_Q21_rejected(self):
        for key in ('root_acceptance_sha256', 'caller_observation_sha256', 'resource_sha256', 'q21_sha256'):
            broken = copy.deepcopy(self.contract)
            broken[key] = '0' * 64
            with self.subTest(key=key), self.assertRaises(Rejected):
                self.validate(broken)

    def test_root_input_changed_bytes_cannot_rebind(self):
        original = runner.file_identity
        for target in ('ROOT-ACCEPTANCE.json', 'CALLER-OBSERVATION.json', 'Controller-A.json',
                       'Controller-B.json', 'query-resources.original-Q21.json',
                       'TestOnly-frozen9011-fullclock-serial-first-attempt.s-resources.json'):
            def changed(path):
                value = original(path)
                if Path(path).name == target:
                    value['sha256'] = '0' * 64
                return value
            with self.subTest(target=target), patch.object(runner, 'file_identity', side_effect=changed), self.assertRaises(Rejected):
                runner.root_inputs()

    def test_native_path_resolution_keeps_frozen_values(self):
        config = runner.root_inputs()[1]['A']
        normalized = runner.native_controller_paths(config)
        self.assertEqual(normalized['db'], config['db'])
        self.assertEqual(normalized['socket'], config['socket'])
        self.assertTrue(exact(normalized['s_resources'], config['s_resources']))
        self.assertEqual(normalized['python'], str(Path(config['python']).resolve()))
        self.assertEqual(config['python'], '/Users/silencehan/.local/bin/python3.12')

    def test_exact_root_paths_gate_is_read_only(self):
        with patch.object(Path, 'mkdir', side_effect=AssertionError('不准创建 runtime')):
            output, records = runner.validate_paths(self.contract)
            self.assertEqual(output, runner.RUNTIME_OUTPUT)
            runner.check_directories(records)
        broken = copy.deepcopy(self.contract)
        broken['output_dir'] = str(runner.HERE / 'wrong-domain')
        with self.assertRaises(Rejected):
            runner.validate_paths(broken)
        with patch.object(runner.os.path, 'lexists', return_value=True), self.assertRaises(Rejected):
            runner.validate_paths(self.contract)

    def test_directory_inode_or_owner_change_rejected(self):
        record = runner.directory_identity(runner.HERE)
        for key in ('inode', 'uid', 'device', 'mode'):
            changed = dict(record)
            changed[key] += 1
            with self.subTest(key=key), patch.object(runner, 'directory_identity', return_value=changed), self.assertRaises(Rejected):
                runner.check_directories([record])


class PersistenceTests(unittest.TestCase):
    def test_committed_receipt_valid_then_clock_phase_corrupt(self):
        data, payload, saved = committed_fixture()
        self.assertTrue(validate_receipt(saved, data[0], 1, payload, data[2]))
        broken = copy.deepcopy(saved)
        values = [dict(clock_event_id='input-1', phase=phase, semantic_ns=ns, attempt_index=0)
                  for phase, ns in [('accept', '3'), ('begin', '5'), ('commit', '4')]]
        broken['clock_events'] = table(values)
        with self.assertRaises(Rejected):
            validate_receipt(broken, data[0], 1, payload, data[2])

    def test_accepted_pending_not_committed(self):
        data, payload, saved = committed_fixture()
        saved['receipt'] = table([dict(source_namespace='fixture', source_epoch='1', message_id='message-1',
             payload_hash=decode(data[0])['payload_hash'], canonical_envelope=data[0][:-1].decode(), status='AcceptedPending',
             receipt_id='receipt-1', first_published_generation=None, clock_event_id='input-1')])
        with self.assertRaises(Rejected):
            validate_receipt(saved, data[0], 1, payload, data[2])

    def test_native_reply_validator_corrupt_cause_and_payload(self):
        client = runner.native_client()  # 该原模块只有协议定义；没有 import 启动副作用。
        env = decode(fixture(1)[0])
        response = client.reply_envelope(env, {'kind': 'Committed'}, session_id='fixture',
                      session_generation='1', namespace='s-session/replies', producer_id='S:fixture', producer_epoch='1')
        client.validate_reply(env, response, producer='S', producer_epoch='1')
        response['causal_refs'][0]['message_id'] = 'message-2'
        with self.assertRaises(ValueError):
            client.validate_reply(env, response, producer='S', producer_epoch='1')

    def test_delivery_unknown_stays_failure_even_with_committed_receipt_and_wire(self):
        client = runner.native_client()
        data, payload, saved = committed_fixture()
        env = decode(data[0])
        response = client.reply_envelope(env, payload, session_id='fixture', session_generation='1',
                         namespace='s-session/replies', producer_id='S:fixture', producer_epoch='1')
        raw = canonical(response) + b'\n'
        self.assertTrue(validate_receipt(saved, data[0], 1, payload, data[2]))
        with self.assertRaises(Rejected):
            runner.received_reply(dict(transport='DeliveryUnknown', response=response), data[0], raw, client)
        self.assertEqual(runner.received_reply(dict(transport='Received', response=response), data[0], raw, client), payload)

    def test_truncated_native_reply_rejected(self):
        client = runner.native_client()
        data, payload, _ = committed_fixture()
        response = client.reply_envelope(decode(data[0]), payload, session_id='fixture', session_generation='1',
                         namespace='s-session/replies', producer_id='S:fixture', producer_epoch='1')
        with self.assertRaises(Rejected):
            runner.received_reply(dict(transport='Received', response=response), data[0], canonical(response)[:-2], client)

    def test_raw_native_blob_hash_and_token_not_reencoded(self):
        body = b'\x00\xff\x01\x80raw-token'
        digest = native_frame(7, body)
        snapshot = dict(integrity_check=[['ok']], foreign_key_check=[], tables={
            's_content_meta': table([dict(singleton=1, content_format='s-content/1')]),
            's_content_nodes': table([dict(digest=digest, tag=7, body=body)]),
            's_content_publications': table([dict(generation=1, manifest_root=digest, snapshot_root=digest, delta_root=digest)])})
        snapshot = snapshot_metadata(snapshot)
        self.assertTrue(validate_snapshot(snapshot))
        snapshot['tables']['s_content_nodes'] = table([dict(digest=digest, tag=7, body=body + b'!')])
        with self.assertRaises(Rejected):
            validate_snapshot(snapshot)

    def test_foreign_key_failure_not_pass(self):
        with self.assertRaises(Rejected):
            validate_snapshot(dict(integrity_check=[['ok']], foreign_key_check=[['broken']], tables={}))

    def test_final_receipts_do_not_hide_lost_raw_history(self):
        data, _, saved = committed_fixture()
        final = dict(tables=dict(s_input_messages=saved['receipt'], s_content_publications=saved['publication'],
              structure_deltas=saved['delta'], s_clock_events=saved['clock_events'], s_protocol_meta=saved['protocol'],
              raw_events=table([], names=['seq', 'raw_text', 'event_id', 'volume']),
              raw_ohlc=table([], names=['identity_key', 'revision', 'open', 'high', 'low', 'close'])))
        with self.assertRaises(Rejected):
            validate_final(final, data[0].splitlines(keepends=True), data[2], count=1)

    def test_final_valid_original_then_same_count_wrong_raw_token(self):
        data, _, saved = committed_fixture()
        raw_input = decode(data[0])['payload']['raw_input']
        event = raw_input['events'][0]
        identity = '|'.join(raw_input[k] for k in ('source_namespace', 'source_epoch', 'instrument')) + '|' + event['event_id']
        native_event = {k: event[k] for k in ('event_id', 'revision', 'seq', 'received_at',
                        'raw_text', 'open', 'high', 'low', 'close', 'timestamp', 'volume')}
        event_hash = sha(canonical(native_event))
        raw = dict(seq=0, identity_key=identity, revision=1, input_revision='1',
                   payload_hash=event_hash, receipt_id='rcpt-' + sha((identity + '|' + event_hash).encode())[:16],
                   source_namespace=raw_input['source_namespace'], source_epoch=raw_input['source_epoch'],
                   instrument=raw_input['instrument'], event_id=event['event_id'], received_at=event['received_at'],
                   raw_text=event['raw_text'], price=event['close'], ts=event['timestamp'], volume=event['volume'],
                   source_coord=event['seq'], supersedes_revision=None)
        prices = dict(identity_key=identity, revision=1, schema_revision='s-ohlc/1',
                      **{key: event[key] for key in ('open', 'high', 'low', 'close')})
        final = dict(tables=dict(s_input_messages=saved['receipt'], s_content_publications=saved['publication'],
              structure_deltas=saved['delta'], s_clock_events=saved['clock_events'], s_protocol_meta=saved['protocol'],
              raw_events=table([raw]), raw_ohlc=table([prices])))
        self.assertTrue(validate_final(final, data[0].splitlines(keepends=True), data[2], count=1))
        raw['raw_text'] = '同条数但不同原 token'
        final['tables']['raw_events'] = table([raw])
        with self.assertRaises(Rejected):
            validate_final(final, data[0].splitlines(keepends=True), data[2], count=1)


class ExactMetadataTests(unittest.TestCase):
    def test_one_row_count_bool_or_real_rejected(self):
        for wrong in (True, 1.0, '1'):
            value = table([{'singleton': 1}])
            value['row_count'] = wrong
            with self.subTest(wrong=wrong), self.assertRaises(Rejected):
                table_rows(value)

    def test_each_clock_field_same_wrong_A_B_rejected_independently(self):
        for phase in ('accept', 'begin', 'commit'):
            for key, wrong in (('attempt_index', 99), ('attempt_ordinal', 99), ('generation', 99),
                               ('message_key', '["wrong","1","message-1"]'), ('clock_event_id', 'input-2'),
                               ('semantic_ns', '999')):
                data, payload, saved = committed_fixture()
                rows = table_rows(saved['clock_events'])
                next(row for row in rows if row['phase'] == phase)[key] = wrong
                saved['clock_events'] = table(rows)
                for arm in ('A', 'B'):
                    with self.subTest(arm=arm, phase=phase, key=key), self.assertRaises(Rejected):
                        validate_receipt(copy.deepcopy(saved), data[0], 1, payload, data[2])

    def test_each_receipt_protocol_numeric_and_type_rejected(self):
        for name, field, wrong in (('receipt', 'accepted_seq', 1), ('receipt', 'attempt_count', 2),
                                   ('protocol', 'next_attempt_ordinal', 99), ('protocol', 'logical_phase_frontier', '999'),
                                   ('receipt', 'accepted_seq', 0.0), ('receipt', 'attempt_count', 1.0),
                                   ('protocol', 'next_attempt_ordinal', 2.0), ('protocol', 'logical_phase_frontier', 5)):
            data, payload, saved = committed_fixture()
            rows = table_rows(saved[name])
            rows[0][field] = wrong
            saved[name] = table(rows)
            with self.subTest(name=name, field=field, wrong=wrong), self.assertRaises(Rejected):
                validate_receipt(saved, data[0], 1, payload, data[2])
        data, payload, saved = committed_fixture()
        for name in saved:
            broken = copy.deepcopy(saved)
            if broken[name]['row_count'] == 1:
                broken[name]['row_count'] = True
                with self.subTest(table=name), self.assertRaises(Rejected):
                    validate_receipt(broken, data[0], 1, payload, data[2])

    def test_final_three_messages_uses_final_protocol_and_each_native_event(self):
        data = fixture(3)
        lines = data[0].splitlines(keepends=True)
        names = ('receipt', 'publication', 'delta', 'clock_events')
        collected = {name:[] for name in names}
        raw_events, prices = [], []
        for n, raw in enumerate(lines, 1):
            _, payload, saved = committed_fixture()
            env = decode(raw)
            receipt = table_rows(saved['receipt'])[0]
            receipt.update(message_id=env['message_id'], payload_hash=env['payload_hash'], canonical_envelope=raw[:-1].decode(),
                           receipt_id=f'receipt-{n}', first_published_generation=n, clock_event_id=f'input-{n}', accepted_seq=n-1)
            public = dict(generation=n, batch_id=f'batch-{n}')
            delta = dict(generation=n,index_frontier=f'batch-{n}',next_cut=f'cut-{n}',catalog_revision='fixture',input_frontier=n-1)
            ticks = [dict(clock_event_id=f'input-{n}', phase=phase, semantic_ns=str(3*n+i), attempt_index=-1 if i==0 else 0,
                          attempt_ordinal=None if i==0 else n, generation=n-1 if i==0 else n,
                          message_key=canonical(['fixture','1',f'message-{n}']).decode()) for i,phase in enumerate(('accept','begin','commit'))]
            meta = dict(protocol_revision='s-session/3',session_generation='1',clock_plan_hash=sha(data[2]),
                        clock_plan_json=data[2].decode(),next_attempt_ordinal=n+1,logical_phase_frontier=str(3*n+2))
            payload.update(original_message_id=env['message_id'],receipt_id=f'receipt-{n}',accepted_seq=str(n-1),
                           publication=dict(generation=str(n),index_frontier=f'batch-{n}',structure_cut=f'cut-{n}',catalog_revision='fixture',input_frontier=str(n-1)))
            once = dict(receipt=table([receipt]), publication=table([public]),delta=table([delta]),clock_events=table(ticks),protocol=table([meta]))
            self.assertTrue(validate_receipt(once,raw,n,payload,data[2]))
            for name in names:
                collected[name].extend(table_rows(once[name]))
            raw_input = env['payload']['raw_input']
            event = raw_input['events'][0]
            identity = '|'.join(raw_input[k] for k in ('source_namespace', 'source_epoch', 'instrument')) + '|' + event['event_id']
            native_event = {k: event[k] for k in ('event_id', 'revision', 'seq', 'received_at',
                            'raw_text', 'open', 'high', 'low', 'close', 'timestamp', 'volume')}
            event_hash = sha(canonical(native_event))
            raw_events.append(dict(seq=n-1, identity_key=identity, revision=1, input_revision='1',
                payload_hash=event_hash, receipt_id='rcpt-' + sha((identity + '|' + event_hash).encode())[:16],
                source_namespace=raw_input['source_namespace'], source_epoch=raw_input['source_epoch'],
                instrument=raw_input['instrument'], event_id=event['event_id'], received_at=event['received_at'],
                raw_text=event['raw_text'], price=event['close'], ts=event['timestamp'], volume=event['volume'],
                source_coord=event['seq'], supersedes_revision=None))
            prices.append(dict(identity_key=identity,revision=1,schema_revision='s-ohlc/1',
                               **{k:event[k] for k in ('open','high','low','close')}))
        tables = {out:table(collected[name]) for out,name in (('s_input_messages','receipt'),('s_content_publications','publication'),('structure_deltas','delta'),('s_clock_events','clock_events'))}
        tables.update(s_protocol_meta=table([meta]),raw_events=table(raw_events),raw_ohlc=table(prices))
        final = dict(tables=tables)
        self.assertTrue(validate_final(final,lines,data[2],count=3))
        for field, wrong in (('received_at', '2026-09-15T00:00:00Z'), ('source_coord', '99'),
                             ('payload_hash', '0' * 64), ('receipt_id', 'rcpt-wrong'),
                             ('input_revision', '2'), ('seq', 0.0)):
            broken = copy.deepcopy(final)
            changed = table_rows(broken['tables']['raw_events'])
            changed[0][field] = wrong
            broken['tables']['raw_events'] = table(changed)
            for domain in ('A', 'B'):
                with self.subTest(domain=domain, field=field), self.assertRaises(Rejected):
                    validate_final(broken, lines, data[2], count=3)
        for field,value in (('next_attempt_ordinal',2),('logical_phase_frontier','5')):
            broken=copy.deepcopy(final)
            row=dict(meta)
            row[field]=value
            broken['tables']['s_protocol_meta']=table([row])
            with self.subTest(field=field),self.assertRaises(Rejected):
                validate_final(broken,lines,data[2],count=3)

    def test_xinfo_all_seven_metadata_types_and_schema_checked(self):
        body = b'\x00native'
        digest = native_frame(7, body)
        snapshot = snapshot_metadata(dict(integrity_check=[['ok']], foreign_key_check=[], tables={
            's_content_meta': table([dict(singleton=1, content_format='s-content/1')]),
            's_content_nodes': table([dict(digest=digest, tag=7, body=body)]),
            's_content_publications': table([dict(manifest_root=digest, snapshot_root=digest, delta_root=digest)])}))
        validate_snapshot(snapshot)
        for index, alternate in enumerate((9, 'other', 'INTEGER', 1, '0', 1, 1)):
            different = copy.deepcopy(snapshot)
            different['tables']['s_content_meta']['columns'][0][index] = alternate
            self.assertFalse(exact(snapshot, different), '第'+str(index)+'项元数据差异不得被对拍忽略')
        for index, wrong in enumerate((False, 1, 1, False, 1, 0.0, False)):
            broken = copy.deepcopy(snapshot)
            broken['tables']['s_content_meta']['columns'][0][index] = wrong
            with self.subTest(index=index), self.assertRaises(Rejected):
                validate_snapshot(broken)
        broken = copy.deepcopy(snapshot)
        broken['tables']['s_content_meta']['columns'][0].pop()
        with self.assertRaises(Rejected):
            validate_snapshot(broken)
        for value in (True, 1.0):
            broken = copy.deepcopy(snapshot)
            broken['db_properties']['user_version'] = value
            with self.assertRaises(Rejected):
                validate_snapshot(broken)
        broken = copy.deepcopy(snapshot)
        del broken['tables']['s_content_meta']
        with self.assertRaises(Rejected):
            validate_snapshot(broken)
        self.assertFalse(exact({'v': True}, {'v': 1}))
        self.assertFalse(exact({'v': 1}, {'v': 1.0}))


class RuntimeFailureTests(unittest.TestCase):
    def test_A_final_raw_or_source_failure_stays_partial_with_snapshot(self):
        import runtime
        ready_reply = {'payload': {'control_instance_id': 'offline-nonce'}}
        class FakeWire:
            count = 1
            def __init__(self, directory, label):
                self.directory, self.label = directory, label
            def __enter__(self):
                return self
            def __exit__(self, *_):
                if self.label == 'start-ready':
                    runner.raw_save(self.directory / 'start-ready-0.reply.wire', canonical(ready_reply) + b'\n')
        class FakeClient:
            decode_frame = staticmethod(decode)
        class FakeDB:
            init_observed = False
            def __init__(self, *_):
                pass
            def snapshot(self):
                return {'offline_snapshot': 'retained'}
        class FakeSampler:
            failure = None
            def __init__(self, *_):
                pass
            def start(self):
                pass
            def close(self):
                return {'samples': 'offline'}
        for fault in ('少一条 raw', 'source_coord 两域同错'):
            with self.subTest(fault=fault), tempfile.TemporaryDirectory(dir=runner.HERE, prefix='offline-final-A-') as name:
                output = Path(name)
                resource = output / 'resource.json'
                resource.write_bytes(b'{}')
                resource.chmod(0o600)
                config = {'socket': str(output / 'A.sock'), 's_resources': {'response_timeout_ms': '30000'}}
                identity = dict(state='running', socket_identity='offline-socket',
                    resource_identity={'path': str(resource)}, control_instance_id='offline-nonce')
                class FakeController:
                    def __init__(self, *_):
                        pass
                    def start_s(self, *_):
                        return {'state': 'ready', 'initialization': {}, 'reply': ready_reply}
                    def status(self, *_):
                        return identity
                    def socket_identity(self):
                        return 'offline-socket'
                class FakeControl:
                    Controller = FakeController
                    @staticmethod
                    def read_config(_):
                        return config
                contract = {'caller_observation': {'per_case_observation_deadline_ms': '3600000',
                    'cleanup_timeout_ms': '30000', 'sampling_period_ms': '100'},
                    'domains': [{'label': 'A', 'db': str(output / 'A' / 's.sqlite'),
                                 'state_dir': str(output / 'A' / 'controller')}],
                    'original_checkpoint_every': '100'}
                plan = {'domains': {'A': {'launch_nonce': 'offline-nonce'}},
                    'contract_sha256': 'offline', 'resource_sha256': sha(b'{}')}
                gates = dict(stop_requested=True, status_stopped=True, stop_confirmed=True,
                             same_handles_reaped=True, socket_removed=True, controller_closed=True)
                with patch.object(runtime, 'OwnedDatabase', FakeDB), \
                     patch.object(runtime, 'Sampler', FakeSampler), \
                     patch.object(runtime, 'load_control', return_value=FakeControl), \
                     patch.object(runtime, 'native_client', return_value=FakeClient()), \
                     patch.object(runtime, 'root_inputs', return_value=({}, {'A': config}, [])), \
                     patch.object(runtime, 'native_controller_paths', side_effect=lambda value: value), \
                     patch.object(runtime, 'observe_processes', return_value=contextlib.nullcontext()), \
                     patch.object(runtime, 'Wire', FakeWire), \
                     patch.object(runtime, 'exchange_once', return_value=({}, b'{}\n', {})), \
                     patch.object(runtime, 'validate_receipt'), \
                     patch.object(runtime, 'validate_snapshot'), \
                     patch.object(runtime, 'validate_final', side_effect=Rejected(fault)) as final_check, \
                     patch.object(runtime, 'cleanup_case', return_value=(gates, [])):
                    result = runtime.run_case('A', output, contract, plan, fixture(1)[0].splitlines(keepends=True),
                                              fixture(1)[2], unittest.mock.Mock(), lambda *_: None)
                final_check.assert_called_once()
                self.assertEqual(result['committed_count'], 1)
                self.assertEqual(result['status'], 'PARTIAL')
                self.assertTrue((output / 'A' / 'final.fullDB.json').is_file())
                self.assertEqual(runner.load(output / 'A' / 'CASE-RESULT.json')['status'], 'PARTIAL')

    def test_partial_A_never_calls_B(self):
        import runtime
        with tempfile.TemporaryDirectory(dir=runner.HERE, prefix='offline-serial-A-') as name:
            base = Path(name)
            inputs = base / 'inputs'
            inputs.mkdir()
            (inputs / 'envelopes.draft.jsonl').write_bytes(fixture(1)[0])
            (inputs / 'clock-plan.full9011.draft.json').write_bytes(fixture(1)[2])
            prepared_dir = base / 'prepared'
            prepared_dir.mkdir()
            (prepared_dir / 'PREPARED.json').write_bytes(b'{}\n')
            prepared = {'originals': [], 'inputs': str(inputs)}
            output = base / 'runtime'
            socket_dir = base / 'socket'
            contract = {'socket_dir': str(socket_dir), 'resource_sha256': 'offline',
                        'caller_observation': {'cleanup_timeout_ms': '30000'}}
            case = {'label': 'A', 'status': 'PARTIAL', 'committed_count': 1,
                    'cleanup_deadline_ns': runtime.time.monotonic_ns() + 30_000_000_000}
            with patch.object(runtime, 'gate_files', return_value=(prepared, contract, output, [])), \
                 patch.object(runtime, 'check_directories'), \
                 patch.object(runtime, 'OwnerLimits', return_value=unittest.mock.Mock()), \
                 patch.object(runtime, 'run_case', return_value=case) as run_case, \
                 patch.object(runtime, 'verify_bundle', side_effect=AssertionError('partial 不可最终 PASS')):
                result = runtime.run_ab(prepared_dir)
            self.assertEqual(run_case.call_count, 1)
            self.assertEqual(run_case.call_args.args[0], 'A')
            self.assertEqual(result['status'], 'FAIL_OR_PARTIAL')
            self.assertFalse(result['complete9011'])

    def test_deadline_uses_absolute_remaining_not_per_operation_timeout(self):
        import runtime
        deadline = runtime.Deadline(31_000_000_000)
        with patch.object(runtime.time, 'monotonic_ns', side_effect=[1_000_000_000, 11_000_000_000, 32_000_000_000]), \
             patch.object(runtime.signal, 'setitimer') as timer:
            deadline.arm()
            deadline.arm()
            with self.assertRaises(runtime.AbortRun):
                deadline.arm()
        self.assertEqual([call.args[1] for call in timer.call_args_list], [30, 20])

    def test_blocking_save_or_snapshot_interrupts_at_absolute_deadline(self):
        import runtime
        old = signal.getsignal(signal.SIGALRM)
        try:
            signal.signal(signal.SIGALRM, lambda *_: (_ for _ in ()).throw(runtime.AbortRun('离线限期')))
            deadline = runtime.Deadline(runtime.time.monotonic_ns() + 20_000_000)
            start = runtime.time.monotonic_ns()
            with self.assertRaises(runtime.AbortRun):
                deadline.call(runtime.time.sleep, 0.1)
            self.assertLess(runtime.time.monotonic_ns() - start, 90_000_000)
            with self.assertRaises(runtime.AbortRun):
                deadline.call(lambda: None)
        finally:
            signal.setitimer(signal.ITIMER_REAL, 0)
            signal.signal(signal.SIGALRM, old)

    def test_actual_st_blocks_includes_sidecars_state_and_evidence(self):
        import runtime
        with tempfile.TemporaryDirectory(dir=runner.HERE, prefix='offline-allocation-') as name:
            root = Path(name)
            (root / 'controller').mkdir()
            for path in (root / 's.sqlite', root / 's.sqlite-wal', root / 's.sqlite-shm',
                         root / 'controller' / 'state.json', root / 'evidence.wire'):
                path.write_bytes(b'X' * 8192)
            expected = sum(p.lstat().st_blocks * 512 for p in [root, *root.rglob('*')])
            self.assertEqual(runtime.allocated_bytes([root, root]), expected)
            contract = dict(socket_dir=str(root / 'no-socket'), caller_observation={'max_owned_artifact_allocated_bytes':str(expected-1)})
            limits = runtime.OwnerLimits(contract, root)
            with self.assertRaises(Rejected):
                limits.checkpoint()
            limits.limit = expected + 4096
            with self.assertRaises(Rejected):
                limits.guard_write(root / 'too-large.json', 8192)

    def test_exchange_exception_preserves_existing_wire_and_persistent_state(self):
        import runtime
        data, _, persistent = committed_fixture()
        class FakeClient:
            def exchange(self, *args, **kwargs):
                raise TimeoutError('离线 exchange 异常')
        class FakeDB:
            def persistent(self, *args):
                return persistent
        class FakeWire:
            count = 1
            def __init__(self, directory, label):
                self.directory, self.label = directory, label
            def __enter__(self):
                return self
            def __exit__(self, *_):
                runner.raw_save(self.directory / (self.label + '-0.request.wire'), data[0])
                runner.raw_save(self.directory / (self.label + '-0.reply.wire'), b'{partial')
        parsed = dict(socket='offline', s_resources=dict(response_timeout_ms='30000', max_frame_bytes='4096'))
        with tempfile.TemporaryDirectory(dir=runner.HERE, prefix='offline-exchange-') as name, patch.object(runtime, 'Wire', FakeWire):
            directory = Path(name)
            with self.assertRaises(TimeoutError):
                runtime.exchange_once(FakeClient(), parsed, decode(data[0]), data[0], 1, directory, FakeDB())
            record = runner.load(directory / 'input-1.exchange.json')
            self.assertEqual(record['error']['type'], 'TimeoutError')
            self.assertIsNone(record['result'])
            self.assertEqual((directory / 'input-1-0.reply.wire').read_bytes(), b'{partial')
            self.assertTrue(exact(runner.load(directory / 'input-1.persistent.json'), persistent))
        class FailedDB:
            def persistent(self, *args):
                raise OSError('持久读失败')
        with tempfile.TemporaryDirectory(dir=runner.HERE, prefix='offline-exchange-') as name, patch.object(runtime, 'Wire', FakeWire):
            directory = Path(name)
            with self.assertRaises(TimeoutError):
                runtime.exchange_once(FakeClient(), parsed, decode(data[0]), data[0], 1, directory, FailedDB())
            self.assertEqual(runner.load(directory / 'input-1.persistent.json')['status'], 'UNMEASURED')

    def test_cleanup_or_sampler_failure_still_attempts_snapshot(self):
        import runtime
        class FakeDB:
            init_observed = True
            def __init__(self, *_):
                pass
            def snapshot(self):
                return {'offline_snapshot':'preserved'}
        class FakeSampler:
            def __init__(self, *_):
                pass
            def start(self):
                pass
            def close(self):
                raise OSError('离线采样结束失败')
        class FakeController:
            def __init__(self, *_):
                pass
            def start_s(self, *_):
                raise OSError('离线启动失败，模拟已完成 init')
        class FakeControl:
            Controller = FakeController
            @staticmethod
            def read_config(_):
                return {}
        for snapshot_fails in (False, True):
            with tempfile.TemporaryDirectory(dir=runner.HERE, prefix='offline-preserve-') as name:
                output = Path(name)
                contract = dict(caller_observation=dict(per_case_observation_deadline_ms='3600000',cleanup_timeout_ms='30000',sampling_period_ms='100'),
                    domains=[dict(label='A', db=str(output/'A'/'s.sqlite'), state_dir=str(output/'A'/'controller'))])
                plan = dict(domains={'A':{'launch_nonce':'offline'}},contract_sha256='offline',resource_sha256='offline')
                snapshot = unittest.mock.Mock(side_effect=OSError('snapshot 失败') if snapshot_fails else None,
                                               return_value={'offline_snapshot':'preserved'})
                with patch.object(runtime, 'OwnedDatabase', FakeDB), patch.object(FakeDB, 'snapshot', snapshot), \
                     patch.object(runtime, 'Sampler', FakeSampler), patch.object(runtime, 'load_control', return_value=FakeControl), \
                     patch.object(runtime, 'native_controller_paths', side_effect=lambda value:value), \
                     patch.object(runtime, 'native_client', return_value=None), patch.object(runtime, 'root_inputs', return_value=({}, {'A':{}}, [])), \
                     patch.object(runtime, 'observe_processes', return_value=contextlib.nullcontext()), \
                     patch.object(runtime, 'Wire', return_value=contextlib.nullcontext()), \
                     patch.object(runtime, 'cleanup_case', return_value=({}, [{'error':'离线清理失败'}])), \
                     patch.object(runtime, 'validate_snapshot'):
                    result = runtime.run_case('A', output, contract, plan, [], b'', unittest.mock.Mock(), lambda *_:None)
                snapshot.assert_called_once()
                self.assertEqual(result['status'], 'PARTIAL')
                self.assertEqual(result['preservation']['samples']['status'], 'UNMEASURED')
                if snapshot_fails:
                    self.assertEqual(result['preservation']['final_snapshot']['status'], 'UNMEASURED')
                else:
                    self.assertTrue((output/'A'/'final.fullDB.json').is_file())


class CompletionTests(unittest.TestCase):
    def setUp(self):
        self.cleanup = dict(stop_requested=True, status_stopped=True, stop_confirmed=True,
                            same_handles_reaped=True, socket_removed=True, controller_closed=True)
        self.cases = [dict(label=label, status='VERIFIED', committed_count=COUNT, cleanup=copy.copy(self.cleanup))
                      for label in ('A', 'B')]
        self.comparisons = dict(initialized=True, final=True, all_receipts_and_reply_wire=True)

    def test_full_offline_evidence_predicate_only(self):
        self.assertTrue(completion(self.cases, self.comparisons, True)['complete9011'])
        # 上述仅验证逻辑，不代表实际运行完成。严格未测峰值不影响 S 范围逻辑谓词。

    def test_partial_never_complete_even_if_claimed_verified(self):
        for count in (0, 16, COUNT - 1):
            self.cases[1]['committed_count'] = count
            self.assertFalse(completion(self.cases, self.comparisons, True)['complete9011'])

    def test_each_cleanup_failure_cannot_pass(self):
        for key in self.cleanup:
            with self.subTest(key=key):
                cases = copy.deepcopy(self.cases)
                cases[0]['cleanup'][key] = False
                self.assertNotEqual(completion(cases, self.comparisons, True)['status'], 'PASS')

    def test_S_scope_never_claims_capacity_C_or_graph(self):
        result = completion(self.cases, self.comparisons, True)
        self.assertTrue(result['complete9011'])
        self.assertFalse(result['full_resource_capacity'])
        self.assertEqual(result['strict_RSS_heap_IO_stage_peaks'], 'UNMEASURED')
        self.assertFalse(result['C_complete'])
        self.assertFalse(result['wholegraph_complete'])

    def test_originals_changed_not_pass(self):
        self.assertFalse(completion(self.cases, self.comparisons, False)['complete9011'])


class SideEffectTests(unittest.TestCase):
    def test_production_head_is_separate_from_private_helper_commit(self):
        frozen = json.loads(runner.ROOT_ACCEPTANCE.read_bytes())
        self.assertEqual(runner.HEAD, frozen['root_current_HEAD'])
        self.assertNotEqual(runner.HEAD, '5cc69fa6d08313f2c20f6ba730832e7a39873d35')
        with patch.object(runner, 'git', return_value=b'private-helper-head'), \
             patch.object(runner, 'root_git', return_value=b'0' * 40):
            with self.assertRaises(Rejected):
                runner.binary_binding()

    def test_default_and_missing_run_arguments_do_not_import_runtime(self):
        runtime_before = 'runtime' in sys.modules
        for arguments in ([], ['run']):
            with self.subTest(arguments=arguments), contextlib.redirect_stderr(io.StringIO()), self.assertRaises(SystemExit):
                runner.main(arguments)
        with contextlib.redirect_stdout(io.StringIO()):
            self.assertEqual(runner.main(['run', '--prepared', '/fixture']), 2)
        self.assertEqual('runtime' in sys.modules, runtime_before)

    def test_patch_reconstructs_exact_source_and_rejects_mismatch(self):
        base = b'first\nold\nlast\n'
        section = b'diff --git a/rust/a b/rust/a\nindex 123..456 100644\n--- a/rust/a\n+++ b/rust/a\n@@ -1,3 +1,3 @@\n first\n-old\n+new\n last\n'
        self.assertEqual(runner.patch_result(base, section), b'first\nnew\nlast\n')
        with self.assertRaises(Rejected):
            runner.patch_result(b'first\nwrong\nlast\n', section)

    def test_runtime_direct_entry_rechecks_gate_without_start(self):
        import runtime
        with patch.object(runtime, 'load_control', side_effect=AssertionError('不准导入 Controller')) as native, \
             patch.object(runtime.sqlite3, 'connect', side_effect=AssertionError('不准打开 DB')) as database, \
             patch.object(runtime.socket, 'socket', side_effect=AssertionError('不准创建 socket')) as transport, \
             patch.object(runtime.subprocess, 'Popen', side_effect=AssertionError('不准启动进程')) as process:
            with self.assertRaises(OSError):
                runtime.run_ab('/nonexistent-offline-fixture')
            for mock in (native, database, transport, process):
                mock.assert_not_called()

    def test_import_modules_does_not_open_sqlite_or_start_services(self):
        import runtime
        with patch.object(runtime.sqlite3, 'connect', side_effect=AssertionError('不准打开 DB')) as db, \
             patch.object(runtime.subprocess, 'Popen', side_effect=AssertionError('不准启动进程')) as proc, \
             patch.object(runtime.socket, 'socket', side_effect=AssertionError('不准 socket')) as sock:
            spec = importlib.util.spec_from_file_location('offline_reimport_runtime', runner.HERE / 'runtime.py')
            module = importlib.util.module_from_spec(spec)
            spec.loader.exec_module(module)
            for mock in (db, proc, sock):
                mock.assert_not_called()

    def test_owned_db_rejects_existing_path_without_opening(self):
        import runtime
        with tempfile.TemporaryDirectory(dir=runner.HERE, prefix='offline-test-') as name:
            directory = Path(name)
            (directory / 's.sqlite').write_bytes(b'not a database')
            with patch.object(runtime.sqlite3, 'connect', side_effect=AssertionError('不准打开 DB')) as database:
                with self.assertRaises(Rejected):
                    runtime.OwnedDatabase(directory, str(directory / 's.sqlite'))
                database.assert_not_called()

    def test_actual_cleanup_gates_fail_on_status_stop_or_reap(self):
        import runtime

        class FakeProcess:
            def __init__(self, pid, code, fail=False):
                self.pid, self.code, self.fail = pid, code, fail

            def wait(self, timeout):
                if self.fail:
                    raise TimeoutError('离线 reap 失败反例')
                return self.code

            def poll(self):
                return None if self.fail else self.code

        class FakeController:
            config = {'socket': '/offline-nonexistent-socket'}

            def __init__(self, failure):
                self.failure, self.stopped = failure, False

            def status(self, _, **kwargs):
                if self.failure == 'status':
                    raise ValueError('离线 status 身份失败反例')
                return dict(state='stopped' if self.stopped else 'running', pid=2, control_instance_id='fixture', command=['fixture', 'serve'])

            def stop(self, _, requested_signal):
                if self.failure == 'stop':
                    raise TimeoutError('离线 stop 失败反例')
                self.stopped = True
                return dict(state='stopped', pid=2)

            def close(self):
                pass

            def prepare_socket(self, _):
                raise AssertionError('清理前置失败不准操作 socket')

        for failure in ('status', 'stop', 'reap'):
            with self.subTest(failure=failure), tempfile.TemporaryDirectory(dir=runner.HERE, prefix='offline-cleanup-') as name:
                handles = [(FakeProcess(1, 0), {'op': 'init'}),
                           (FakeProcess(2, -signal.SIGTERM, fail=failure == 'reap'),
                            dict(op='serve', native_nonce='fixture', argv=['fixture', 'serve']))]
                gates, errors = runtime.cleanup_case(FakeController(failure), handles, signal.SIGTERM, runtime.Deadline(runtime.time.monotonic_ns() + 30_000_000_000), Path(name))
                self.assertFalse(all(gates.values()))
                self.assertTrue(errors)
                signal.setitimer(signal.ITIMER_REAL, 0)

    def test_schema_json_is_parseable_has_no_defaults(self):
        schema = runner.load(runner.HERE / 'contract.schema.json')
        self.assertEqual(set(schema['$defs']['resources']['required']), set(contract_fixture()[0]['s_resources']))
        self.assertNotIn('capacity', schema['$defs'])
        self.assertNotIn('"default"', (runner.HERE / 'contract.schema.json').read_text())


if __name__ == '__main__':
    def offline_audit(event, _args):
        if event in ('sqlite3.connect', 'subprocess.Popen', 'socket.__new__'):
            raise AssertionError('纯离线测试禁止副作用：' + event)

    sys.addaudithook(offline_audit)
    unittest.main(verbosity=2)
