"""#1394 独审05采样边界锁：连续推进内存时钟，不运行产品或真实进程。"""
from dataclasses import replace
import unittest

import test_review04 as previous


class Review05Tests(unittest.TestCase):
    def advancing_clock(self, state):
        state['clock_reads'] = []

        def clock():
            state['now'][0] += .000002
            state['clock_reads'].append(state['now'][0])
            return state['now'][0]

        state['core'].clock = clock

    def queued_case(self, count, delay):
        case = previous.Review04Tests()

        def effect(state):
            case.queue_descendants(state)
            self.advancing_clock(state)

        state = case.scenario(count=count, delay=delay, queued=True, effect=effect)
        case.accepted(state, count)
        owners = {101, 103, 104} | ({102} if count == 2 else set())
        self.assertEqual(set(state['core'].owners), owners)
        self.assertEqual(set(state['samples'][-1][1]), owners | {100})
        self.assertEqual(len(state['samples']), 2)
        self.assertFalse(state['queue'])
        self.assertEqual([pid for pid, _ in state['probe'].signals], sorted(owners, reverse=True))
        self.assertTrue(all(row['reaped'] for row in state['outputs'][-1][1]['owners']))
        self.assertEqual(state['outputs'][-1][1]['product_status'], 'NOT_RUN')
        self.assertLess(len(state['calls']), 12)
        self.assertGreater(len(state['clock_reads']), 2)
        self.assertTrue(all(a < b for a, b in zip(state['clock_reads'], state['clock_reads'][1:])))
        self.assertLess(state['now'][0], state['start']+1)
        self.assertEqual(state['core'].limits.sample_interval, .125)
        self.assertEqual(state['core'].limits.work_deadline, state['start']+10)
        self.assertEqual(state['core'].limits.heartbeat_timeout, 1)
        self.assertTrue(all(stage['deadline'] == state['start']+9
                            for stage in state['core'].stages.values()))

    def test_R_F5_advancing_clock_last_emit_return_drains_before_sample(self):
        self.queued_case(1, .124995)

    def test_R_F5_advancing_clock_before_second_ACK_drains_before_sample(self):
        self.queued_case(2, .124985)

    def test_R_F5_advancing_clock_near_boundary_legal_controls(self):
        for count in (1, 2):
            with self.subTest(count=count):
                self.queued_case(count, .1245)

    def test_R_F5_not_due_tick_still_checks_worker_identity(self):
        case = previous.Review04Tests()

        def effect(state):
            self.advancing_clock(state)
            state['probe'].identities[100] = replace(state['worker'], start='reused')

        state = case.scenario(count=1, delay=.005, effect=effect)
        case.rejected(state, '漂移')
        self.assertEqual(len(state['samples']), 1)

    def test_R_F5_tick_clock_crossing_original_deadline_rejects_without_sample(self):
        case = previous.Review04Tests()

        def effect(state):
            state['now'][0] = state['core'].last_heartbeat+1-.000003
            self.advancing_clock(state)

        state = case.scenario(count=1, delay=.005, effect=effect)
        case.rejected(state, '心跳')
        self.assertEqual(len(state['samples']), 1)
