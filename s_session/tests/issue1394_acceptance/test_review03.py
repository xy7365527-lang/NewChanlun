"""独审03的两个残余锁；仅内存Probe与短小toy原字节，不执行产品。"""
import copy
from dataclasses import asdict, replace
import unittest

import driver as d
import supervisor as sup
import test_capture_bridge as cap
import test_review01 as prior


class Review03Tests(unittest.TestCase):
    def setup_core(self):
        return prior.Review01Tests().setup_core()

    def heartbeat(self, worker):
        return {'kind': 'HEARTBEAT', 'worker': asdict(worker)}

    def complete(self, worker):
        return {'kind': 'COMPLETE', 'worker': asdict(worker)}

    def descendant(self, worker, identity):
        return {'kind': 'REGISTER_DESCENDANT', 'worker': asdict(worker),
                'contract_sha256': 'a'*64, 'parent': identity.parent,
                'identity': asdict(identity)}

    def queue(self, messages):
        calls = []
        def receive(timeout):
            calls.append(timeout)
            self.assertTrue(messages, 'fake队列必须显式结束，不允许意外无限读取')
            return messages.pop(0)
        return receive, calls

    def test_R_F5_heartbeat_then_first_ACK_formal_probe(self):
        now, worker, child, probe, _, core, binding = self.setup_core()
        probe.desc.add(child.pid); now[0] += .25
        receive, calls = self.queue([self.heartbeat(worker),
            {'kind': 'ACK_REQUEST', 'binding': binding}, self.complete(worker)])
        outputs = []
        sup.monitor(core, receive, outputs.append)
        self.assertEqual(len(calls), 3)
        self.assertEqual(calls[1:], [0, 0])
        self.assertEqual(outputs[0]['kind'], 'ACK')
        self.assertEqual(outputs[-1]['status'], 'CONTROL_REAPED')
        self.assertEqual(probe.waits, [101])

    def test_R_F5_two_REGISTERs_formal_probe(self):
        now, worker, _, probe, _, core, binding = self.setup_core()
        core.ack(binding)
        descendants = [sup.Identity(pid, str(pid), 101, 101) for pid in (102, 103)]
        for identity in descendants:
            probe.identities[identity.pid] = identity; probe.desc.add(identity.pid)
        now[0] += .25
        receive, calls = self.queue([*(self.descendant(worker, x) for x in descendants),
                                     self.complete(worker)])
        outputs = []
        sup.monitor(core, receive, outputs.append)
        self.assertEqual(len(calls), 3)
        self.assertEqual(set(core.owners), {101, 102, 103})
        self.assertEqual(set(probe.waits), {101, 102, 103})
        self.assertEqual(outputs[-1]['status'], 'CONTROL_REAPED')

    def test_R_F5_ACK_force_sample_waits_for_queued_descendants(self):
        _, worker, child, probe, _, core, binding = self.setup_core()
        descendants = [sup.Identity(pid, str(pid), 101, 101) for pid in (102, 103)]
        for identity in [child, *descendants]:
            probe.identities[identity.pid] = identity; probe.desc.add(identity.pid)
        receive, calls = self.queue([{'kind': 'ACK_REQUEST', 'binding': binding},
            *(self.descendant(worker, x) for x in descendants), self.complete(worker)])
        outputs = []
        def emit(value):
            if value.get('kind') == 'ACK':
                self.assertEqual(set(core.owners), {101, 102, 103})
                self.assertTrue(any(e['kind'] == 'SAMPLE' for e in core.events))
            outputs.append(value)
        sup.monitor(core, receive, emit)
        self.assertEqual(len(calls), 4)
        self.assertEqual(outputs[-1]['status'], 'CONTROL_REAPED')

    def test_R_F5_empty_queue_boundary_and_no_queue_legal(self):
        for heartbeat_first in (False, True):
            _, worker, child, probe, _, core, binding = self.setup_core()
            probe.desc.add(child.pid); outputs = []
            messages = ([self.heartbeat(worker)] if heartbeat_first else [])
            messages += [{'kind': 'ACK_REQUEST', 'binding': binding}, None, self.complete(worker)]
            receive, calls = self.queue(messages)
            def checked_receive(timeout):
                if len(messages) == 1:
                    self.assertEqual(outputs[0]['kind'], 'ACK')
                    self.assertGreater(timeout, 0)
                return receive(timeout)
            sup.monitor(core, checked_receive, outputs.append)
            self.assertEqual(outputs[-1]['status'], 'CONTROL_REAPED')
            self.assertIn(0, calls)

    def test_R_F5_stranger_at_empty_or_COMPLETE_boundary_rejected(self):
        for complete in (False, True):
            _, worker, child, probe, _, core, binding = self.setup_core()
            probe.desc.update((child.pid, 999)); outputs = []
            receive, _ = self.queue([self.heartbeat(worker),
                {'kind': 'ACK_REQUEST', 'binding': binding},
                self.complete(worker) if complete else None])
            with self.subTest(complete=complete), self.assertRaisesRegex(d.Rejected, '未登记后代'):
                sup.monitor(core, receive, outputs.append)
            self.assertFalse(any(x.get('kind') == 'ACK' for x in outputs))
            self.assertNotIn(999, core.owners)
            self.assertNotIn(999, [pid for pid, _ in probe.signals])
            self.assertEqual(outputs[-1]['status'], 'INCOMPLETE')

    def test_R_F5_forged_queued_registration_never_acknowledged(self):
        for forged in ('ack_identity', 'worker', 'parent', 'identity', 'contract', 'nonce'):
            _, worker, child, probe, _, core, binding = self.setup_core()
            grandchild = sup.Identity(102, 'child102', 101, 101)
            probe.identities[102] = grandchild; probe.desc.update((101, 102))
            bad = self.descendant(worker, grandchild)
            if forged == 'ack_identity': binding['owner']['start_identity'] = 'forged'
            if forged == 'worker': bad['worker']['start'] = 'forged'
            if forged == 'parent': bad['parent'] = 999
            if forged == 'identity': bad['identity']['start'] = 'forged'
            if forged == 'contract': bad['contract_sha256'] = 'b'*64
            if forged == 'nonce': bad = {'kind': 'ACK_REQUEST', 'binding': binding}
            receive, _ = self.queue([self.heartbeat(worker),
                {'kind': 'ACK_REQUEST', 'binding': binding}, bad, self.complete(worker)])
            outputs = []
            with self.subTest(forged=forged), self.assertRaises(d.Rejected):
                sup.monitor(core, receive, outputs.append)
            self.assertFalse(any(x.get('kind') == 'ACK' for x in outputs))
            self.assertNotIn(102, core.owners)

    def test_R_F5_original_deadlines_not_extended_by_queue_or_heartbeat(self):
        for deadline in ('work', 'heartbeat'):
            for late in ('entry', 'first_receive', 'drain'):
                now, worker, child, probe, limits, core, binding = self.setup_core()
                original = limits.work_deadline if deadline == 'work' else core.last_heartbeat + limits.heartbeat_timeout
                probe.desc.add(child.pid); outputs = []; calls = []
                if late == 'entry': now[0] = original
                def receive(timeout):
                    calls.append(timeout)
                    if late == 'first_receive' or len(calls) == 2: now[0] = original
                    return self.heartbeat(worker) if len(calls) == 1 else {'kind': 'ACK_REQUEST', 'binding': binding}
                with self.subTest(deadline=deadline, late=late), self.assertRaises(d.Rejected):
                    sup.monitor(core, receive, outputs.append)
                self.assertFalse(core.owners)
                self.assertFalse(any(x.get('kind') == 'ACK' for x in outputs))
                self.assertEqual(len(calls), 0 if late == 'entry' else (1 if late == 'first_receive' else 2))

    def test_R_F5_heartbeat_stream_has_fixed_sampling_boundary(self):
        for stranger in (False, True):
            now, worker, _, probe, limits, core, binding = self.setup_core()
            core.ack(binding)
            if stranger: probe.desc.add(999)
            calls = []; outputs = []; samples = len(core.events)
            def receive(timeout):
                calls.append(timeout); now[0] += limits.sample_interval / 4
                return self.heartbeat(worker) if len(calls) <= 20 else self.complete(worker)
            if stranger:
                with self.assertRaisesRegex(d.Rejected, '未登记后代'):
                    sup.monitor(core, receive, outputs.append)
                self.assertLessEqual(len(calls), 6)
                self.assertNotIn(999, core.owners)
            else:
                sup.monitor(core, receive, outputs.append)
                self.assertGreater(len(core.events) - samples, 3)
                self.assertEqual(outputs[-1]['status'], 'CONTROL_REAPED')

    def test_R_F5_COMPLETE_before_registration_is_not_a_grace_period(self):
        _, worker, child, probe, _, core, binding = self.setup_core()
        probe.desc.add(child.pid)
        messages = [self.complete(worker), {'kind': 'ACK_REQUEST', 'binding': binding}]
        receive, calls = self.queue(messages)
        with self.assertRaisesRegex(d.Rejected, '未登记后代'):
            sup.monitor(core, receive, lambda value: None)
        self.assertEqual(len(calls), 1)
        self.assertEqual(len(messages), 1)
        self.assertFalse(core.owners)

    def test_R_F5_queued_ACK_still_requires_resource_sample(self):
        _, worker, child, probe, _, core, binding = self.setup_core()
        probe.desc.add(child.pid); probe.readings['rss_bytes'] = None
        receive, _ = self.queue([self.heartbeat(worker),
            {'kind': 'ACK_REQUEST', 'binding': binding}, None])
        outputs = []
        with self.assertRaisesRegex(d.Rejected, '监督采样未知'):
            sup.monitor(core, receive, outputs.append)
        self.assertFalse(any(x.get('kind') == 'ACK' for x in outputs))

    def test_N2_raw_5940_positive_6040_negative_and_exact_padded_cap(self):
        for extra, allowance in ((0, 0), (100, 0), (100, 100)):
            contract, captures, nodes, raw = cap.fixture()
            i = next(i for i, x in enumerate(captures) if x.route == 'financial-closure')
            old = captures[i]; request = d.strict(old.request)
            self.assertEqual(len(old.reply), 5940)
            request['max_reply_bytes'] = str(len(old.reply) + allowance)
            reply = b' '*extra + old.reply
            self.assertTrue(reply.endswith(b'\n')); self.assertEqual(reply.count(b'\n'), 1)
            self.assertEqual(d.strict(reply), d.strict(old.reply))
            captures[i] = replace(old, request=d.canonical(request), reply=reply)
            contract['captures'] = [x.identity() for x in captures]
            if extra > allowance:
                with self.assertRaisesRegex(d.Rejected, '闭包回复超过请求帽'):
                    cap.export(contract, captures, nodes, raw)
            else:
                result = cap.export(contract, captures, nodes, raw)
                self.assertEqual(result['status'], 'UNAUTHENTICATED_FACTS')
                self.assertEqual(result['captures'][i]['reply_sha256'], d.sha(reply))
            self.assertEqual(captures[i].reply, reply)

    def test_N2_each_financial_page_checks_its_own_raw_limit(self):
        contract, captures, nodes, raw = cap.fixture()
        indices = [i for i, x in enumerate(captures) if x.route == 'financial-closure']
        for i in indices:
            old = captures[i]; request = d.strict(old.request)
            request['max_reply_bytes'] = str(len(old.reply))
            captures[i] = replace(old, request=d.canonical(request))
        contract['captures'] = [x.identity() for x in captures]
        self.assertEqual(cap.export(contract, captures, nodes, raw)['status'], 'UNAUTHENTICATED_FACTS')
        for i in indices:
            changed = captures[:]; changed[i] = replace(changed[i], reply=b' '*100 + changed[i].reply)
            trial = copy.deepcopy(contract); trial['captures'] = [x.identity() for x in changed]
            with self.subTest(page=i), self.assertRaisesRegex(d.Rejected, '闭包回复超过请求帽'):
                cap.export(trial, changed, nodes, raw)

    def test_N2_basis_raw_object_caps_and_job_wire_remain_independent(self):
        contract, captures, nodes, raw = cap.fixture()
        i = next(i for i, x in enumerate(captures) if x.route == 'financial-basis')
        captures[i] = replace(captures[i], reply=b' '*7000 + captures[i].reply)
        contract['captures'] = [x.identity() for x in captures]
        length = len(captures[i].reply); resources = contract['resources']
        self.assertEqual(length, max(len(x.reply) for x in captures if x.route != 'snapshot'))
        for role in ('session', 'reader'): resources[role]['max_reply_bytes'] = str(length)
        wire = sum(len(x.request)+len(x.reply) for x in captures if x.route != 'snapshot')
        resources['reader']['job_limits']['max_total_wire_bytes'] = str(wire)
        self.assertEqual(cap.export(contract, captures, nodes, raw)['status'], 'UNAUTHENTICATED_FACTS')
        for scope in ('objects', 'job'):
            trial = copy.deepcopy(contract)
            if scope == 'objects':
                for role in ('session', 'reader'): trial['resources'][role]['max_reply_bytes'] = str(length-1)
            else: trial['resources']['reader']['job_limits']['max_total_wire_bytes'] = str(wire-1)
            with self.subTest(scope=scope), self.assertRaises(d.Rejected):
                cap.export(trial, captures, nodes, raw)
