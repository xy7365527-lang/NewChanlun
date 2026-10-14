#!/usr/bin/env python3
"""#1419 / #1497：TB-02-E S9 浏览器延后开关（--browser-deferred-to 1497）的测试锁。

锁三件：① 不带开关时 harness 计划与命令没有开关痕迹、runtime 照原样起 tb02e_browser.cjs、浏览器门仍只认 passed；
② 带开关时 runtime 不起浏览器、在 browser/RESULT.json 写 DEFERRED 收据，harness 总状态为 PASS_EXCEPT_BROWSER_DEFERRED（不是 passed）；
③ 两种收据互不冒充。不起 S/Q、不起浏览器；依赖同 tb02c（ijson）。
"""
import copy
import json
import shutil
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path
from unittest import mock

TESTS = Path(__file__).resolve().parent
ROOT = TESTS.parent
sys.path.insert(0, str(TESTS))
sys.path.insert(0, str(ROOT))
import tb02e_harness as harness  # noqa: E402
import tb02e_runtime as runtime  # noqa: E402

CUTS = tuple(harness.RECOVERY_CUTS)
INPUTS = {case: len(bars) for case, bars in json.loads((harness.FIXTURE / 'raw-ledger.json').read_text())['cases'].items()
          if case in harness.CASES}
# daac6d8661 版 prepare 写出的计划键与次序；不带开关时必须原样。
PLAN_KEYS = ['schema', 'fixture_sha256', 'oracle_sha256', 'source_sha256', 'binary_sha256', 'normal_recovery',
             'fault_injection', 'operation_deadline_seconds', 'run_deadline_seconds', 'offline_deadlines_seconds', 'scope', 'runs']


def write(path, value):
    path.write_text(json.dumps(value, ensure_ascii=False))


def passed_browser(inputs):
    return {'status': 'passed', 'cuts': [str(CUTS[0]), str(inputs)], 'page_errors': [], 'browser_version': 'stub'}


def deferred_browser(inputs):
    return runtime.deferred_browser_record('1497', (CUTS[0], inputs))


class HarnessSwitch(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.tmp = tempfile.TemporaryDirectory()
        base = Path(cls.tmp.name)
        binary = base / 's_structure_session'
        binary.write_bytes(b'stub writer')
        common = dict(binary=binary, python=Path(sys.executable), node=base / 'node', playwright_module=base / 'pw', port_base=18860)
        cls.default_plan = harness.prepare(base / 'default', **common)
        cls.deferred_plan = harness.prepare(base / 'deferred', browser_deferred_to='1497', **common)

    @classmethod
    def tearDownClass(cls):
        cls.tmp.cleanup()

    def test_default_plan_carries_no_switch(self):
        self.assertEqual(list(self.default_plan), PLAN_KEYS)
        on_disk = json.loads((Path(self.tmp.name) / 'default/RUN-PLAN.json').read_text())
        self.assertEqual(on_disk, self.default_plan)
        for run in self.default_plan['runs']:
            self.assertNotIn('--browser-deferred-to', run['command'])
            self.assertEqual(run['command'][-2:], ['--case', run['case']])

    def test_switch_recorded_in_plan_and_commands(self):
        self.assertEqual(list(self.deferred_plan), PLAN_KEYS + ['browser_deferred_to'])
        self.assertEqual(self.deferred_plan['browser_deferred_to'], '#1497')
        for run in self.deferred_plan['runs']:
            self.assertEqual(run['command'][-4:], ['--case', run['case'], '--browser-deferred-to', '1497'])

    def test_switch_does_not_touch_inputs(self):
        for left, right in zip(self.default_plan['runs'], self.deferred_plan['runs']):
            for name in ('messages.jsonl', 'clock.json'):
                self.assertEqual((Path(left['directory']) / name).read_bytes(), (Path(right['directory']) / name).read_bytes())

    def materialize(self, plan, browser):
        """按计划造四臂已完成原件（只造 compare_completed 读的收据）。"""
        out = Path(tempfile.mkdtemp(dir=self.tmp.name))
        plan = copy.deepcopy(plan)
        for run in plan['runs']:
            directory = out / f"{run['case']}-{run['arm']}"
            run['directory'] = str(directory)
            evidence = directory / 'evidence'
            (evidence / 'browser').mkdir(parents=True)
            inputs = INPUTS[run['case']]
            write(directory / 'EXIT.json', {'exit': 0})
            write(evidence / 'RESULT.json', {'status': 'CAPTURED_REQUIRES_DOUBLE_RUN_ORACLE_AND_BROWSER', 'input_count': inputs,
                                             'core_records': len(harness.expected_layout(inputs)), 'cleanup': {'errors': []}})
            write(evidence / 'query-oracle.json', {'status': 'passed', 'case': run['case'], 'cuts': inputs})
            write(evidence / 'normal-recovery.json', {'equal': True, 'cuts': [CUTS[0], inputs]})
            write(evidence / 'browser/RESULT.json', browser(inputs))
        return plan, out

    def compare(self, plan, out, trace_status='PASS'):
        trace = lambda left, right, layout: {'status': trace_status, 'expected_records': len(layout)}
        with mock.patch.object(harness, 'validate_source_binding', lambda plan, run: None), \
                mock.patch.object(harness, 'compare_exact_traces', trace):
            return harness.compare_completed(plan, out)

    def test_default_pass_is_passed(self):
        plan, out = self.materialize(self.default_plan, passed_browser)
        result = self.compare(plan, out)
        self.assertEqual(result['status'], 'passed')
        self.assertNotIn('browser', result)
        self.assertEqual(json.loads((out / 'COMPARISON-RESULT.json').read_text()), result)

    def test_deferred_pass_is_not_passed(self):
        plan, out = self.materialize(self.deferred_plan, deferred_browser)
        result = self.compare(plan, out)
        self.assertEqual(result['status'], 'PASS_EXCEPT_BROWSER_DEFERRED')
        self.assertEqual(result['browser'], {'status': 'DEFERRED', 'deferred_to': '#1497'})
        self.assertEqual(json.loads((out / 'COMPARISON-RESULT.json').read_text()), result)

    def test_deferred_receipt_rejected_without_switch(self):
        plan, out = self.materialize(self.default_plan, deferred_browser)
        with self.assertRaises((KeyError, ValueError)):
            self.compare(plan, out)
        self.assertFalse((out / 'COMPARISON-RESULT.json').exists())

    def test_passed_receipt_rejected_under_switch(self):
        plan, out = self.materialize(self.deferred_plan, passed_browser)
        with self.assertRaisesRegex(ValueError, '收据不完整'):
            self.compare(plan, out)
        self.assertFalse((out / 'COMPARISON-RESULT.json').exists())

    def test_other_ticket_rejected_under_switch(self):
        plan, out = self.materialize(self.deferred_plan, lambda n: {**deferred_browser(n), 'deferred_to': '#9999'})
        with self.assertRaisesRegex(ValueError, '收据不完整'):
            self.compare(plan, out)

    def test_trace_mismatch_under_switch_not_verified(self):
        plan, out = self.materialize(self.deferred_plan, deferred_browser)
        with self.assertRaisesRegex(ValueError, '尚未验真'):
            self.compare(plan, out, trace_status='NOT_VERIFIED')
        self.assertEqual(json.loads((out / 'COMPARISON-RESULT.json').read_text())['status'], 'NOT_VERIFIED')

    def test_cli_rejects_other_tickets_and_compare_existing(self):
        for script in ('tb02e_harness.py', 'tb02e_runtime.py'):
            run = subprocess.run([sys.executable, '-B', str(TESTS / script), '--browser-deferred-to', '9999'],
                                 capture_output=True, text=True, timeout=120)
            self.assertEqual(run.returncode, 2, script)
            self.assertIn('invalid choice', run.stderr)
        run = subprocess.run([sys.executable, '-B', str(TESTS / 'tb02e_harness.py'), '--compare-existing', self.tmp.name,
                              '--browser-deferred-to', '1497'], capture_output=True, text=True, timeout=120)
        self.assertEqual(run.returncode, 2)
        self.assertIn('只用于新运行', run.stderr)


class RuntimeBrowserStep(unittest.TestCase):
    def make(self, deferred):
        out = Path(tempfile.mkdtemp())
        self.addCleanup(shutil.rmtree, out, True)
        run = object.__new__(runtime.Tb02eRun)
        run.output, run.root, run.case, run.recovery_cuts = out, ROOT, 'min12_up', CUTS
        run.node, run.playwright_module, run.browser_deferred_to = '/stub/node', '/stub/pw', deferred
        run.config = {'port': '18860', 'db': str(out / 'db.sqlite')}
        run.events = []
        run.emit = lambda kind, **fields: run.events.append((kind, fields))
        run.query = lambda command: {'echo': command['id'][len(command['id'].split('-')[0]) + 1:], 'generation': command['generation']}
        run.control = lambda *args: {'control': list(args)}
        return run

    def test_default_launches_browser_as_before(self):
        run = self.make(None)
        calls = []

        class Child:
            pid, returncode = 999999, 0

            def wait(self, timeout=None):
                return 0

        def popen(command, **kwargs):
            calls.append((command, kwargs))
            return Child()
        with mock.patch.object(runtime, 'verify', return_value={'status': 'passed'}), \
                mock.patch.object(runtime.subprocess, 'Popen', popen):
            self.assertEqual(run.complete_after_core(5), 7)
        expected = ['/stub/node', str(ROOT / 'tests/tb02e_browser.cjs'), '--base-url', 'http://127.0.0.1:18860',
                    '--output', str(run.output / 'browser'), '--playwright-module', '/stub/pw',
                    '--final-cut', str(CUTS[1]), '--pending-cut', str(CUTS[0])]
        self.assertEqual(len(calls), 1)
        self.assertEqual(calls[0][0], expected)
        self.assertIs(calls[0][1].get('start_new_session'), True)
        self.assertEqual(json.loads((run.output / 'browser-command.json').read_text()), expected)
        self.assertFalse((run.output / 'browser').exists())
        self.assertNotIn('browser_deferred', [kind for kind, _ in run.events])

    def test_switch_writes_deferred_receipt_without_browser(self):
        run = self.make('1497')
        with mock.patch.object(runtime, 'verify', return_value={'status': 'passed'}), \
                mock.patch.object(runtime.subprocess, 'Popen', side_effect=AssertionError('浏览器不应启动')) as popen:
            self.assertEqual(run.complete_after_core(5), 7)
        popen.assert_not_called()
        record = json.loads((run.output / 'browser/RESULT.json').read_text())
        self.assertEqual(record, runtime.deferred_browser_record('1497', CUTS))
        self.assertEqual((record['status'], record['deferred_to'], record['browser_executed']), ('DEFERRED', '#1497', False))
        self.assertEqual(record['planned_cuts'], [str(CUTS[0]), str(CUTS[1])])
        self.assertEqual([e['round'] for e in record['failure_evidence']], [1, 2])
        self.assertTrue(record['reason'])
        self.assertTrue(harness.browser_receipt_ok(record, '#1497', CUTS[1]))
        self.assertFalse((run.output / 'browser-command.json').exists())
        self.assertIs(json.loads((run.output / 'normal-recovery.json').read_text())['equal'], True)
        self.assertTrue((run.output / 'query-oracle.json').exists())
        self.assertIn(('browser_deferred', {'deferred_to': '#1497'}), run.events)


if __name__ == '__main__':
    unittest.main()
