"""#1394 对象公开序窄锁；有限toy独立源与可变wire分离，无产品执行。"""
import copy
from dataclasses import replace
import unittest

import captured_cut as x
import closure as c
import driver as d
import test_capture_bridge as cap
import test_product_capture as product


def basis_pair(captures):
    index = next(i for i, item in enumerate(captures) if item.route == 'financial-basis')
    item = captures[index]
    return index, d.strict(item.request), d.strict(item.reply)


def change_basis(data, change):
    """仅改待检wire并重钉toy清单；源nodes始终来自构造时的独立原件。"""
    contract, captures, _, _ = data
    index, request, reply = basis_pair(captures)
    change(request, reply)
    captures[index] = replace(captures[index], request=d.canonical(request),
                              reply=d.canonical(reply) + b'\n')
    contract['captures'] = [item.identity() for item in captures]
    return data


class CaptureOrderTests(unittest.TestCase):
    def test_public_order_201_preserves_all_raw_bytes_and_source(self):
        data = cap.fixture()
        contract, captures, source, _ = data
        index, request, reply = basis_pair(captures)
        self.assertEqual(request['object_ids'], ['carrier', 'source', 'terminal'])
        self.assertEqual([item['snapshot_index'] for item in reply['selected_objects']], ['2', '0', '1'])
        # 非canonical空白是捕获原件；导出不得重编码request/reply迎合接口。
        captures[index] = replace(captures[index], request=b' \n' + captures[index].request + b'\t',
                                  reply=b'  ' + captures[index].reply)
        contract['captures'] = [item.identity() for item in captures]
        before = [(item.request, item.reply, copy.deepcopy(item.binding)) for item in captures]
        original_source = dict(source)
        facts = cap.export(*data)
        self.assertEqual(facts['status'], 'UNAUTHENTICATED_FACTS')
        self.assertEqual(facts['product_status'], 'NOT_RUN')
        self.assertFalse(facts['execution_authorized'])
        self.assertEqual([o['object_id'] for o in facts['snapshot']['objects']], ['source', 'terminal', 'carrier'])
        self.assertEqual(facts['captures'], contract['captures'])
        self.assertEqual(before, [(item.request, item.reply, item.binding) for item in captures])
        self.assertEqual(source, original_source)

    def test_same_public_and_storage_order_still_passes(self):
        data = cap.fixture(storage_order=(2, 0, 1))
        _, request, reply = basis_pair(data[1])
        self.assertEqual(request['object_ids'], ['carrier', 'source', 'terminal'])
        self.assertEqual([item['snapshot_index'] for item in reply['selected_objects']], ['0', '1', '2'])
        self.assertEqual(cap.export(*data)['status'], 'UNAUTHENTICATED_FACTS')

    def test_storage_order_request_even_with_matching_selected_is_rejected(self):
        def change(request, reply):
            request['object_ids'] = ['source', 'terminal', 'carrier']
            reply['selected_objects'].sort(key=lambda item: int(item['snapshot_index']))
        with self.assertRaisesRegex(d.Rejected, '财务点查'):
            cap.export(*change_basis(cap.fixture(), change))

    def test_request_missing_duplicate_extra_and_same_set_wrong_order_rejected(self):
        for wanted in ([], ['carrier', 'source'], ['carrier', 'source', 'source'],
                       ['carrier', 'source', 'terminal', 'extra'], ['carrier', 'source', 'extra'],
                       ['terminal', 'source', 'carrier']):
            with self.subTest(wanted=wanted), self.assertRaisesRegex(d.Rejected, '财务点查'):
                cap.export(*change_basis(cap.fixture(), lambda req, rep: req.update(object_ids=wanted)))

    def test_selected_order_missing_duplicate_and_extra_rejected(self):
        changes = [lambda items: items.reverse(), lambda items: items.pop(),
                   lambda items: items.__setitem__(1, copy.deepcopy(items[0])),
                   lambda items: items.append(copy.deepcopy(items[0]))]
        for change in changes:
            with self.subTest(change=change), self.assertRaisesRegex(d.Rejected, '点查对象'):
                cap.export(*change_basis(cap.fixture(), lambda req, rep: change(rep['selected_objects'])))

    def test_selected_index_invalid_or_outside_snapshot_rejected(self):
        for index in ('3', '999', '-1', '02', '2.0', '', None, 2, True):
            with self.subTest(index=index), self.assertRaises(d.Rejected):
                cap.export(*change_basis(cap.fixture(), lambda req, rep:
                    rep['selected_objects'][0].update(snapshot_index=index)))

    def test_selected_duplicate_index_rejected(self):
        with self.assertRaisesRegex(d.Rejected, '点查对象'):
            cap.export(*change_basis(cap.fixture(), lambda req, rep:
                rep['selected_objects'][1].update(snapshot_index='2')))

    def test_public_rank_cannot_replace_snapshot_index(self):
        def change(request, reply):
            for i, item in enumerate(reply['selected_objects']):
                item['snapshot_index'] = str(i)
        with self.assertRaisesRegex(d.Rejected, '点查对象'):
            cap.export(*change_basis(cap.fixture(), change))

    def test_selected_id_ref_and_hash_each_bind_independent_record(self):
        for field in ('object_id', 'object_ref', 'object_hash'):
            def change(request, reply):
                reply['selected_objects'][0][field] = copy.deepcopy(reply['selected_objects'][1][field])
            with self.subTest(field=field), self.assertRaisesRegex(d.Rejected, '点查对象'):
                cap.export(*change_basis(cap.fixture(), change))

    def test_selected_fields_and_snapshot_ref_remain_exact(self):
        for mode in ('missing', 'extra', 'snapshot_ref'):
            def change(request, reply):
                if mode == 'missing':
                    reply['selected_objects'][0].pop('object_hash')
                elif mode == 'extra':
                    reply['selected_objects'][0]['extra'] = 'unapproved'
                else:
                    reply['snapshot_ref'] = reply['selected_objects'][0]['object_ref']
            with self.subTest(mode=mode), self.assertRaises(d.Rejected):
                cap.export(*change_basis(cap.fixture(), change))

    def test_q_wire_reordering_is_not_repaired_by_request_or_selected(self):
        data = cap.fixture()
        contract, captures, source, _ = data
        original_source = dict(source)
        item = captures[0]
        reply, request = d.strict(item.reply), d.strict(item.request)
        reply['payload']['rows'].reverse()
        reply['payload_hash'] = cap.fp(reply['payload'])
        reply['message_id'] = 'reply-' + cap.fp([request['source_namespace'], request['source_epoch'],
                                                request['message_id'], reply['payload_hash']])
        captures[0] = replace(item, reply=d.canonical(reply))
        def change(request, reply):
            request['object_ids'][:2] = reversed(request['object_ids'][:2])
            reply['selected_objects'][:2] = reversed(reply['selected_objects'][:2])
        change_basis(data, change)
        with self.assertRaisesRegex(d.Rejected, '有序全集'):
            cap.export(*data)
        self.assertEqual(source, original_source)

    def test_independent_source_cannot_be_replaced_with_mutated_wire(self):
        data = cap.fixture()
        contract, captures, source, _ = data
        original_source = dict(source)
        index = next(i for i, item in enumerate(captures) if item.route == 'financial-closure')
        reply = d.strict(captures[index].reply)
        reply['nodes'][0]['body_hex'] = 'ff'
        captures[index] = replace(captures[index], reply=d.canonical(reply))
        contract['captures'] = [item.identity() for item in captures]
        with self.assertRaisesRegex(d.Rejected, '摘要'):
            cap.export(*data)
        self.assertEqual(source, original_source)
        # 反方向也独立：wire正确，独立源错误同样必须拒绝。
        data = cap.fixture()
        root = data[0]['anchor']['basis_ref']['root']
        data[2][root] = c.Node(root, 0, b'')
        with self.assertRaisesRegex(d.Rejected, '持久节点摘要'):
            cap.export(*data)

    def test_basis_proof_remains_bound_to_full_closure(self):
        for foreign in (False, True):
            data = cap.fixture()
            node = next(iter(data[2].values()))
            raw = {'root': node.root, 'tag': f'{node.tag:02x}', 'body_hex': node.body.hex()}
            if foreign:
                raw['body_hex'] = 'ff'
            change_basis(data, lambda req, rep: rep.update(proof_nodes=[raw]))
            with self.subTest(foreign=foreign):
                if foreign:
                    with self.assertRaisesRegex(d.Rejected, 'proof'):
                        cap.export(*data)
                else:
                    self.assertEqual(cap.export(*data)['status'], 'UNAUTHENTICATED_FACTS')

    def test_memory_adapter_capture_exports_without_rewriting_requests(self):
        contract, _, source, input_raw = cap.fixture()
        plan, replies = product.fixture()
        result, machine, _ = product.run(plan, replies)
        self.assertEqual(result['capture_status'], 'UNAUTHENTICATED_RAW_CAPTURE', result['failure'])
        contract['resources'] = copy.deepcopy(plan['resources'])
        contract['captures'] = [item.identity() for item in machine.captures]
        raw_pairs = [(item.request, item.reply) for item in machine.captures]
        facts = cap.export(contract, machine.captures, source, input_raw)
        self.assertEqual(facts['status'], 'UNAUTHENTICATED_FACTS')
        self.assertEqual(facts['captures'], contract['captures'])
        self.assertEqual(raw_pairs, [(item.request, item.reply) for item in machine.captures])
