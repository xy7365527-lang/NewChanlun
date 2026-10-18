"""#1394 独审04发送边界锁：有限内存队列/时钟/Probe，不执行产品。"""
from dataclasses import asdict
from pathlib import Path
import time
import unittest

import driver as d
import supervisor as sup
from test_supervisor import Probe


class Review04Tests(unittest.TestCase):
    def scenario(self, *, count=2, delay=.005, delay_on=1, terminal='COMPLETE',
                 second_deadline=9, effect=None, queued=False, sample_effect=None):
        now = [time.monotonic()]
        start = now[0]
        worker = sup.Identity(100, 'worker', 100, 99)
        children = [sup.Identity(101, 'E', 101, 100), sup.Identity(102, 'B', 102, 100)]
        probe = Probe(worker)
        probe.identities.update((x.pid, x) for x in children)
        limits = sup.Limits(start+10, start+12, start+14, .125, 1, .1, .2, 4, 100, 100, 100)
        stages = {name: {'resource_caps': {'scope': 'UNIT_TEST_ONLY'},
                        'deadline': start+(second_deadline if name == 'B' else 9),
                        'domain_id': name, 'writer_epoch': '1'} for name in ('E', 'B')}
        script = Path(sup.__file__)
        digest = d.sha(script.read_bytes())
        core = sup.SupervisorCore(contract_sha='a'*64, script=script, script_sha=digest,
            worker=worker, worker_script=script, worker_script_sha=digest, limits=limits,
            stages=stages, probe=probe, clock=lambda: now[0])
        bindings = [{'contract_sha256': 'a'*64, 'worker_pid': 100,
            'worker_start_identity': 'worker', 'worker_process_group': 100,
            'nonce': child.start, 'stage': child.start,
            'owner': {'pid': child.pid, 'start_identity': child.start,
                      'process_group': child.group, 'domain_id': child.start, 'writer_epoch': '1'},
            'resource_caps': stages[child.start]['resource_caps'],
            'absolute_deadline_monotonic': stages[child.start]['deadline']} for child in children]
        end = {'kind': terminal, 'worker': asdict(worker)}
        queue = [*({'kind': 'ACK_REQUEST', 'binding': b} for b in bindings[:count]),
                 None if queued else end]
        state = dict(now=now, start=start, worker=worker, probe=probe, core=core,
                     bindings=bindings, queue=queue, end=end, outputs=[], samples=[], calls=[])

        def receive(timeout):
            state['calls'].append(timeout)
            self.assertLessEqual(len(state['calls']), 32, '有限fake禁止无限排队')
            self.assertTrue(queue, 'fake必须显式提供空队列或结束消息')
            now[0] += .001
            message = queue.pop(0)
            if message is not None and message['kind'] == 'ACK_REQUEST':
                probe.desc.add(message['binding']['owner']['pid'])
            return message

        def sample(identities):
            state['samples'].append((now[0]-start, tuple(x.pid for x in identities)))
            if sample_effect is not None:
                sample_effect(state)
            return probe.readings

        def emit(value):
            state['outputs'].append((now[0]-start, value))
            if value.get('kind') == 'ACK':
                if len(self.acks(state)) == delay_on:
                    now[0] += delay
                    if effect is not None:
                        effect(state)

        probe.sample = sample
        try:
            sup.monitor(core, receive, emit)
        except d.Rejected as error:
            state['error'] = str(error)
        else:
            state['error'] = None
        return state

    def acks(self, state):
        return [(at, value) for at, value in state['outputs'] if value.get('kind') == 'ACK']

    def rejected(self, state, reason, ack_count=1):
        self.assertIsNotNone(state['error'], state['outputs'])
        self.assertIn(reason, state['error'])
        self.assertEqual(len(self.acks(state)), ack_count)
        result = state['outputs'][-1][1]
        self.assertEqual(result['status'], 'INCOMPLETE')
        self.assertEqual(result['product_status'], 'NOT_RUN')
        self.assertTrue(result['failures'])
        self.assertEqual({x['identity']['pid'] for x in result['owners']}, set(state['core'].owners))
        self.assertTrue(all(x['reaped'] for x in result['owners']))
        self.assertNotIn(999, state['core'].owners)
        self.assertNotIn(999, [pid for pid, _ in state['probe'].signals])
        self.assertLess(state['now'][0], state['core'].limits.cleanup_deadline)

    def accepted(self, state, ack_count):
        self.assertIsNone(state['error'])
        self.assertEqual(len(self.acks(state)), ack_count)
        self.assertEqual(state['outputs'][-1][1]['status'], 'CONTROL_REAPED')
        self.assertFalse(state['core'].failures)

    def test_R_F5_formal_second_stage_expires_between_sends(self):
        state = self.scenario(delay=.05, second_deadline=.02)
        self.rejected(state, 'ACK', 1)
        self.assertAlmostEqual(self.acks(state)[0][0], .003, places=5)
        self.assertEqual(set(state['core'].owners), {101, 102})

    def test_R_F5_formal_two_ACK_work_and_heartbeat_exhaustion(self):
        for delay, reason in ((10.1, '截止'), (1.1, '心跳')):
            with self.subTest(delay=delay):
                self.rejected(self.scenario(delay=delay), reason)

    def test_R_F5_single_ACK_COMPLETE_checks_returned_work_deadline(self):
        self.rejected(self.scenario(count=1, delay=10.1), '截止')

    def test_R_F5_single_ACK_CANCEL_checks_returned_work_deadline(self):
        self.rejected(self.scenario(count=1, delay=10.1, terminal='CANCEL'), '截止')

    def test_R_F5_last_ACK_checks_returned_heartbeat(self):
        for terminal in ('COMPLETE', 'CANCEL'):
            with self.subTest(terminal=terminal):
                self.rejected(self.scenario(count=1, delay=1.1, terminal=terminal), '心跳')

    def test_R_F5_formal_single_ACK_due_stranger_before_terminal(self):
        for terminal in ('COMPLETE', 'CANCEL'):
            with self.subTest(terminal=terminal):
                state = self.scenario(count=1, delay=.2, terminal=terminal,
                    effect=lambda s: s['probe'].desc.add(999))
                self.rejected(state, '未登记后代')

    def test_R_F5_two_ACK_last_return_due_RSS_over_cap(self):
        for terminal in ('COMPLETE', 'CANCEL'):
            with self.subTest(terminal=terminal):
                state = self.scenario(delay=.2, delay_on=2, terminal=terminal,
                    effect=lambda s: s['probe'].readings.update(rss_bytes=101))
                self.rejected(state, 'rss_bytes', 2)
                self.assertEqual(len(state['samples']), 2)

    def test_R_F5_first_return_due_RSS_stops_remaining_ACK(self):
        state = self.scenario(delay=.2,
            effect=lambda s: s['probe'].readings.update(rss_bytes=101))
        self.rejected(state, 'rss_bytes')

    def test_R_F5_exact_original_deadline_is_exhausted(self):
        for deadline in ('work', 'heartbeat', 'stage'):
            def expire(s):
                s['now'][0] = (s['core'].limits.work_deadline if deadline == 'work' else
                    s['core'].last_heartbeat+1 if deadline == 'heartbeat' else
                    s['core'].stages['B']['deadline'])
            with self.subTest(deadline=deadline):
                state = self.scenario(second_deadline=.02, effect=expire)
                self.rejected(state, 'ACK' if deadline == 'stage' else '截止' if deadline == 'work' else '心跳')

    def test_R_F5_due_sample_can_exhaust_next_stage_before_send(self):
        def sample(s):
            if len(s['samples']) == 2:
                s['now'][0] = s['core'].stages['B']['deadline']
        self.rejected(self.scenario(delay=.2, second_deadline=.3, sample_effect=sample), 'ACK')

    def test_R_F5_due_sample_can_exhaust_heartbeat_before_terminal(self):
        def sample(s):
            if len(s['samples']) == 2:
                s['now'][0] = s['core'].last_heartbeat+1
        self.rejected(self.scenario(count=1, delay=.2, sample_effect=sample), '心跳')

    def test_R_F5_legal_send_delays_keep_original_deadlines_and_due_samples(self):
        for count in (1, 2):
            for delay in (.005, .125, .2):
                for terminal in ('COMPLETE', 'CANCEL'):
                    with self.subTest(count=count, delay=delay, terminal=terminal):
                        state = self.scenario(count=count, delay=delay, terminal=terminal)
                        self.accepted(state, count)
                        self.assertEqual(len(state['samples']), 1 if delay < .125 else 2)
                        self.assertEqual(state['core'].last_heartbeat, state['start'])
                        self.assertEqual(state['core'].limits.work_deadline, state['start']+10)

    def queue_descendants(self, state):
        state['queue'].append({'kind': 'HEARTBEAT', 'worker': asdict(state['worker'])})
        for pid in (103, 104):
            identity = sup.Identity(pid, str(pid), 101, 101)
            state['probe'].identities[pid] = identity
            state['probe'].desc.add(pid)
            state['queue'].append({'kind': 'REGISTER_DESCENDANT',
                'worker': asdict(state['worker']), 'contract_sha256': 'a'*64,
                'parent': 101, 'identity': asdict(identity)})
        state['queue'].append(state['end'])

    def test_R_F5_send_due_sample_drains_queued_heartbeat_and_multiple_REGISTERs(self):
        state = self.scenario(count=1, delay=.2, queued=True, effect=self.queue_descendants)
        self.accepted(state, 1)
        self.assertEqual(set(state['core'].owners), {101, 103, 104})
        self.assertEqual(set(state['samples'][-1][1]), {100, 101, 103, 104})

    def test_R_F5_send_due_sample_drains_queued_ACK_internal_path(self):
        def enqueue(s):
            s['probe'].desc.add(102)
            s['queue'].extend([{'kind': 'HEARTBEAT', 'worker': asdict(s['worker'])},
                              {'kind': 'ACK_REQUEST', 'binding': s['bindings'][1]}])
            self.queue_descendants(s)
        state = self.scenario(count=1, delay=.2, queued=True, effect=enqueue)
        self.accepted(state, 2)
        self.assertEqual(set(state['core'].owners), {101, 102, 103, 104})

    def test_R_F5_send_queue_does_not_hide_stranger_or_extend_original_deadline(self):
        for violation in ('stranger', 'heartbeat'):
            def enqueue(s):
                self.queue_descendants(s)
                if violation == 'stranger':
                    s['probe'].desc.add(999)
                else:
                    s['now'][0] = s['start']+.9985
            with self.subTest(violation=violation):
                state = self.scenario(count=1, delay=.2, queued=True, effect=enqueue)
                self.rejected(state, '未登记后代' if violation == 'stranger' else '心跳')

    def test_R_F5_send_queue_sampling_boundary_is_finite(self):
        def enqueue(s):
            s['probe'].desc.add(999)
            s['queue'].extend({'kind': 'HEARTBEAT', 'worker': asdict(s['worker'])} for _ in range(200))
        # 每次接收推进0.001，再模拟每条处理0.02；原采样窗内不足32次读取。
        def shorten(s):
            enqueue(s)
            # 此处不改Limits；让接收字典读取推进时钟，模拟有界处理耗时。
            class TimedMessage(dict):
                def __getitem__(self, key):
                    if key == 'kind': s['now'][0] += .02
                    return super().__getitem__(key)
            s['queue'][:] = [TimedMessage(x) for x in s['queue']]
        state = self.scenario(count=1, delay=.2, queued=True, effect=shorten)
        self.rejected(state, '未登记后代')
        self.assertLess(len(state['calls']), 12)
        self.assertTrue(state['queue'])
