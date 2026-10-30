import copy
import importlib.util
import json
import os
import re
import tarfile
import textwrap
from pathlib import Path
import shutil
import subprocess
import tempfile
import time
import unittest
from unittest.mock import patch

spec = importlib.util.spec_from_file_location('g77', Path(__file__).with_name('issue1419-g77-ab.py'))
g77 = importlib.util.module_from_spec(spec)
spec.loader.exec_module(g77)


def probe(side='new', trace=False):
    enabled = side == 'new'
    return {'generation': 77, 'side': side, 'base_side': 'new', 'trace': trace,
            'error': None, 'reply': {'generation': 77, 'message_receipt': {'complete': True}},
            'elapsed_ns': '1000000000', 'f_calls': 10, 'work': {}, 'events': [],
            'development': {'sessions': 2, 'eligible_batches': 10, 'current_full': 29,
                'previous_full': 12 if enabled else 120, 'hits': 108 if enabled else 0,
                'misses': 12 if enabled else 120, 'capacity': 16384 if enabled else 0,
                'disabled': not enabled, 'retained_bytes': 0,
                'peak_bytes': 786928 if enabled else 0,
                'allocated_bytes': 1573856 if enabled else 0,
                'released_bytes': 1573856 if enabled else 0,
                'combined_peak_bytes': 2953712 if enabled else 2166784},
            'proofs': {'disabled': False, 'retained_bytes': 2166784,
                'combined_peak_bytes': 2953712 if enabled else 2166784,
                'auxiliary_retained_bytes': 0},
            'material': {'logical_reads': 100, 'sql_material_reads': 90,
                'retained_capacity_highwater': 33554432}}


class FakeProcess:
    def __init__(self, command, env, cwd, *, stderr, timeout=False, exit_code=0, mutate=None):
        assert Path(stderr.name) == Path(env["W2_PENDING_COPY"]).parent.parent / "supervisor.stderr"
        assert not stderr.closed
        self.env, self.command, self.cwd = env, command, cwd
        self.timeout, self.exit_code, self.returncode = timeout, exit_code, None
        self.terminated = False
        self.case = Path(env['W2_PENDING_COPY']).parent.parent
        self.value = probe(env['W2_PREVIOUS_SIDE'], env['W2_TRACE'] == '1')
        if mutate:
            mutate(self.value)
        (self.case / 'run.log').write_text('W2-PENDING-COST ' + json.dumps(self.value) +
                                         '\ntest result: ok. 1 passed;\n')

    def wait(self, timeout):
        if self.timeout and not self.terminated:
            raise subprocess.TimeoutExpired(self.command, timeout)
        self.returncode = 143 if self.terminated else self.exit_code
        (self.case / 'process.json').write_text(json.dumps({
            'stopped': True, 'returncode': self.returncode, 'process_group': 123,
            'signal': 15 if self.terminated else None}))
        return self.returncode

    def poll(self):
        return self.returncode

    def terminate(self):
        self.terminated = True


class MeasurementTest(unittest.TestCase):
    def setUp(self):
        self.print_patch = patch("builtins.print")
        self.print_patch.start()
        self.addCleanup(self.print_patch.stop)
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.out_patch = patch.object(g77, 'out', Path(self.temp.name))
        self.out_patch.start()
        self.addCleanup(self.out_patch.stop)
        with patch.object(g77.subprocess, 'check_output', return_value='candidate-head\n'):
            g77.configure('/tmp/same-release-test-executable', int(time.time()))
        g77.source.mkdir(parents=True)
        (g77.source / 'source-head.txt').write_text('ca887d965a8a30848c4521b5684ae2b046ca5698')
        (g77.source / 'run-id.txt').write_text('41005347604')
        (g77.source / 'a/w2.run').mkdir(parents=True)
        (g77.source / 'a/w2.run/s.sqlite').write_bytes(b'synthetic-copy-only')
        self.processes = []
        self.comparisons = 0
        self.options = {}

    def launch(self, command, **kwargs):
        process = FakeProcess(command, **kwargs, **self.options)
        self.processes.append(process)
        return process

    def command(self, args, **kwargs):
        self.assertGreater(kwargs['timeout'], 0)
        if args[0] == 'cp':
            shutil.copytree(args[2], args[3])
            return subprocess.CompletedProcess(args, 0)
        self.assertEqual(Path(args[1]).name, 'compare-g77-tables.py')
        self.comparisons += 1
        output = {'all_equal': True, 'schema_equal': True,
                  'tables': [{'rows': 1, 'equal': True} for _ in range(24)]}
        return subprocess.CompletedProcess(args, 0, json.dumps(output), '')

    def run_pair(self, index=0, trace=True):
        with patch.object(g77, 'machine', return_value={'synthetic': True}), \
             patch.object(g77.subprocess, 'Popen', side_effect=self.launch), \
             patch.object(g77.subprocess, 'run', side_effect=self.command):
            return g77.pair(index, trace)

    def test_same_binary_closure_and_independent_copies(self):
        self.run_pair()
        self.run_pair(1, False)
        self.assertEqual([p.env['W2_PREVIOUS_SIDE'] for p in self.processes],
                         ['old', 'new', 'new', 'old'])
        self.assertEqual(len({p.env['W2_PENDING_COPY'] for p in self.processes}), 4)
        for p in self.processes:
            self.assertEqual(p.env['W2_BASE_SIDE'], 'new')
            self.assertIn(str(g77.exe), p.command)
            self.assertIn('--test-threads=1', p.command)
        self.assertEqual(self.comparisons, 2)

    def test_resource_caps_release_and_selector_fail_closed(self):
        for section, key, value in [
            ('proofs', 'retained_bytes', 0), ('proofs', 'disabled', True),
            ('proofs', 'combined_peak_bytes', g77.PROOF_CAP + 1),
            ('proofs', 'auxiliary_retained_bytes', 1),
            ('material', 'retained_capacity_highwater', g77.MATERIAL_CAP + 1),
            ('development', 'retained_bytes', 1), ('development', 'released_bytes', 0),
            ('development', 'combined_peak_bytes', g77.PROOF_CAP + 1),
            ('development', 'capacity', 4), ('development', 'disabled', True)]:
            with self.subTest(section=section, key=key):
                p = probe()
                p[section][key] = value
                with self.assertRaises(RuntimeError):
                    g77.validate_resources(p, 'new')
        g77.validate_resources(probe('old'), 'old')
        g77.validate_resources(probe(), 'new')

    def test_actual_work_not_speed_or_logical_read_threshold(self):
        old, new = probe('old'), probe()
        new['elapsed_ns'] = '2000000000'
        new['material']['logical_reads'] = 200
        g77.validate_work(old, new)
        for key, value in [('eligible_batches', 0), ('current_full', 28),
                           ('previous_full', 120), ('hits', 0), ('misses', 1)]:
            changed = copy.deepcopy(new)
            changed['development'][key] = value
            with self.subTest(key=key), self.assertRaises(RuntimeError):
                g77.validate_work(old, changed)

    def test_timeout_stops_supervisor_and_retains_exit_record(self):
        self.options['timeout'] = True
        with self.assertRaises(subprocess.TimeoutExpired):
            self.run_pair()
        self.assertEqual(len(self.processes), 1)
        self.assertTrue(self.processes[0].terminated)
        self.assertEqual(g77.summary['timeouts'], 1)
        self.assertEqual(g77.summary['failed_cases'], 1)
        self.assertEqual(g77.summary['launched'], 1)
        self.assertEqual(g77.summary['exited'], 1)
        self.assertEqual(g77.summary['reclaimed'], 1)
        self.assertEqual(g77.summary['nonzero_exits'], 1)
        saved = json.loads((g77.out / 'summary.json').read_text())
        self.assertTrue(saved['cases'][0]['timed_out'])
        self.assertTrue((g77.out / 'trace/old/run.log').exists())

    def test_error_exit_retains_release_evidence_and_stops_pair(self):
        self.options.update(exit_code=101, mutate=lambda p: p.update(error='audit rejected'))
        with self.assertRaisesRegex(RuntimeError, 'exit/reclamation'):
            self.run_pair()
        self.assertEqual(len(self.processes), 1)
        case = g77.summary['cases'][0]
        self.assertTrue(case['resources_valid'])
        self.assertEqual(case['probe']['error'], 'audit rejected')
        self.assertEqual(g77.summary['reclaimed'], 1)

    def test_resource_failure_stops_before_second_arm(self):
        self.options['mutate'] = lambda p: p['proofs'].update(auxiliary_retained_bytes=1)
        with self.assertRaisesRegex(RuntimeError, 'not released'):
            self.run_pair()
        self.assertEqual(len(self.processes), 1)
        self.assertIn('probe', g77.summary['cases'][0])
        self.assertEqual(g77.summary['pairs'], [])

    def test_reply_mismatch_stops_before_database_comparison(self):
        self.options['mutate'] = lambda p: p.update(reply={'side': p['side']})
        with self.assertRaisesRegex(RuntimeError, 'reply mismatch'):
            self.run_pair()
        self.assertEqual(self.comparisons, 0)
        self.assertEqual(len(g77.summary['cases']), 2)
        self.assertEqual(g77.summary['pairs'], [])

    def test_typed_database_mismatch_is_retained(self):
        real_command = self.command
        def mismatch(args, **kwargs):
            if args[0] == 'cp':
                return real_command(args, **kwargs)
            return subprocess.CompletedProcess(args, 1, '', 'typed row mismatch')
        self.command = mismatch
        with self.assertRaisesRegex(RuntimeError, 'typed DB mismatch'):
            self.run_pair()
        self.assertEqual((g77.out / 'trace/comparison.stderr').read_text(), 'typed row mismatch')
        self.assertEqual(g77.summary['pairs'], [])

    def test_shared_deadline_covers_build_and_copy(self):
        self.assertEqual(g77.deadline - g77.summary['started_epoch'], 35 * 60 - 120)
        with patch.object(g77, 'deadline', time.time() - 1), self.assertRaises(TimeoutError):
            self.run_pair()
        self.assertEqual(len(self.processes), 0)
        self.assertFalse(g77.summary['cases'][0]['launched'])

    def test_trace_first_and_all_five_pairs_kept(self):
        with patch.object(g77, 'machine', return_value={}), \
             patch.object(g77, 'command_output', return_value={'stdout': 'synthetic'}), \
             patch.object(g77.subprocess, 'Popen', side_effect=self.launch), \
             patch.object(g77.subprocess, 'run', side_effect=self.command), \
             patch.dict(g77.os.environ, {'G77_MODE': 'trace-first'}):
            g77.main()
        self.assertEqual([p['trace'] for p in g77.summary['pairs']], [True] + [False] * 5)
        self.assertEqual(g77.summary['successes'], 12)
        self.assertEqual(g77.summary['status'], 'five_measured_pairs_complete')
        g77.summarize_timings()
        self.assertEqual(g77.summary['measured_pairs'], 5)
        self.assertEqual(g77.summary['timings']['old']['values_seconds'], [1.0] * 5)
        self.assertTrue(g77.summary['benefit'].startswith('未测出收益'))

    def test_trace_only_keeps_full_pair_without_measurement_series(self):
        with patch.object(g77, 'machine', return_value={}), \
             patch.object(g77, 'command_output', return_value={'stdout': 'synthetic'}), \
             patch.object(g77.subprocess, 'Popen', side_effect=self.launch), \
             patch.object(g77.subprocess, 'run', side_effect=self.command), \
             patch.dict(g77.os.environ, {'G77_MODE': 'trace-only'}):
            g77.main()
        self.assertEqual([p.env['W2_PREVIOUS_SIDE'] for p in self.processes], ['old', 'new'])
        self.assertTrue(all(p.env['W2_TRACE'] == '1' for p in self.processes))
        self.assertTrue(all(p.env['W2_BASE_SIDE'] == 'new' for p in self.processes))
        self.assertEqual(g77.summary['successes'], 2)
        self.assertEqual(self.comparisons, 1)
        self.assertEqual(g77.summary['mode'], 'trace-only')
        self.assertEqual(g77.summary['max_measured_pairs'], 0)
        self.assertEqual(g77.summary['status'], 'trace_pair_complete')
        self.assertEqual(len(g77.summary['pairs']), 1)
        self.assertTrue(g77.summary['pairs'][0]['trace'])
        self.assertEqual(g77.summary['pairs'][0]['table_count'], 24)
        g77.summarize_timings()
        self.assertEqual(g77.summary['measured_pairs'], 0)
        self.assertNotIn('timings', g77.summary)
        self.assertNotIn('median_saving_seconds', g77.summary)

    def test_insufficient_time_keeps_trace_without_extending_window(self):
        calls = []
        def trace(index, enabled):
            calls.append((index, enabled))
            g77.deadline = time.time()
        with patch.object(g77, 'machine', return_value={}), \
             patch.object(g77, 'command_output', return_value={'stdout': 'synthetic'}), \
             patch.object(g77, 'pair', side_effect=trace), \
             patch.dict(g77.os.environ, {'G77_MODE': 'trace-first'}):
            g77.main()
        self.assertEqual(calls, [(0, True)])
        self.assertEqual(g77.summary['status'], 'bounded_partial_samples')

    def test_old_trace_cannot_replace_this_candidate_trace(self):
        with patch.dict(g77.os.environ, {'G77_MODE': 'untraced'}), \
             self.assertRaisesRegex(RuntimeError, 'trace-first or trace-only required'):
            g77.main()
        self.assertEqual(g77.summary['cases'], [])

    def test_small_savings_are_reported_as_unmeasured(self):
        g77.summary['pairs'] = [
            {'trace': False, 'old_seconds': old, 'new_seconds': old - 0.01}
            for old in [1, 1.1, 1.2, 1.3, 1.4]]
        g77.summarize_timings()
        self.assertTrue(g77.summary['benefit'].startswith('未测出收益'))
        self.assertEqual(len(g77.summary['paired_savings_seconds']), 5)

    def test_cli_records_failure_and_preserves_completed_samples(self):
        g77.summary["pairs"] = [{'trace': False, 'old_seconds': 1.1, 'new_seconds': 1.0}]
        with patch.object(g77, 'configure'), \
             patch.object(g77.signal, 'signal'), \
             patch.object(g77.sys, 'argv', ['g77', '/tmp/same-binary']), \
             patch.dict(g77.os.environ, {'G77_STARTED': '0'}), \
             patch.object(g77, 'main', side_effect=RuntimeError('comparison failed')), \
             self.assertRaisesRegex(RuntimeError, 'comparison failed'):
            g77.cli()
        saved = json.loads((g77.out / 'summary.json').read_text())
        self.assertEqual(saved['status'], 'failed')
        self.assertEqual(saved['errors'], 1)
        self.assertTrue(saved['benefit'].startswith('未判定收益'))
        self.assertEqual(saved['measured_pairs'], 1)
        self.assertEqual(saved['timings']['new']['values_seconds'], [1.0])

    def test_copy_timeout_never_launches_probe(self):
        with patch.object(g77.subprocess, 'run', side_effect=subprocess.TimeoutExpired('cp', 1)), \
             patch.object(g77.subprocess, 'Popen') as launch, \
             self.assertRaises(subprocess.TimeoutExpired):
            g77.run_case(0, 'old', True)
        launch.assert_not_called()
        self.assertEqual(g77.summary['timeouts'], 1)
        self.assertEqual(g77.summary['launched'], 0)
        self.assertTrue((g77.out / 'summary.json').exists())

    def test_workflow_uses_one_build_and_shared_deadline(self):
        text = Path(__file__).parents[1].joinpath('workflows/issue-1419-g77-ab.yml').read_text()
        self.assertEqual(text.count('cargo test --locked'), 1)
        self.assertIn('G77_MODE: "trace-only"', text)
        self.assertIn('timeout-minutes: 35', text)
        self.assertIn('G77_STARTED + 35 * 60 - 120', text)
        self.assertIn('development_measurement_', text)
        self.assertIn('rustfmt --check --edition 2021', text)
        self.assertNotIn('rustfmt --edition 2021', text)
        self.assertNotIn('prior_trace_run', Path(g77.__file__).read_text())


class ArchiveWorkflowTest(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.workspace = Path(self.temp.name)
        self.evidence = self.workspace / 'g77-evidence'
        self.evidence.mkdir()
        self.cases = []
        self.live_groups = set()
        self.scripts = Path(__file__).resolve().parent
        workflow = (self.scripts.parent / 'workflows/issue-1419-g77-ab.yml').read_text()
        step = workflow.split('- name: Retain every generated sample after writer stop', 1)[1]
        self.archive_code = textwrap.dedent(step.split("python3 - <<'PY'\n", 1)[1].split('\n          PY', 1)[0])
        self.upload = workflow.split('- name: Upload exact bounded diagnostic evidence', 1)[1]
        scripts = self.workspace / '.github/scripts'
        scripts.mkdir(parents=True)
        shutil.copyfile(self.scripts / 'w2-runtime-process.py', scripts / 'w2-runtime-process.py')

    def sample(self, name, state=None, launched=True):
        folder = self.evidence / name
        (folder / 'w2.run').mkdir(parents=True)
        for filename in ['s.sqlite', 's.sqlite-wal']:
            (folder / 'w2.run' / filename).write_bytes(b'synthetic original')
        if state is not None:
            (folder / 'process.json').write_text(json.dumps(state))
        self.cases.append({'case': name, 'launched': launched})
        (self.evidence / 'summary.json').write_text(json.dumps({'cases': self.cases}))
        return folder

    def emergency(self, folder, group=123, complete=True):
        package = folder.with_suffix('.stopped-originals.tgz')
        with tarfile.open(package, 'w:gz') as archive:
            archive.add(folder, arcname=folder.name)
        state = {'stopped': True, 'process_group': group, 'receipt_error': 'disk receipt failure',
                 'emergency_archive': str(package), 'emergency_tar_returncode': 0,
                 'emergency_archive_complete': complete}
        (folder / 'supervisor.stderr').write_text(json.dumps(state) + '\n')
        return package

    def archive(self):
        original_spec = importlib.util.spec_from_file_location
        def supervisor_spec(name, location):
            result = original_spec(name, location)
            loader = result.loader.exec_module
            def load(module):
                loader(module)
                module.live_group = lambda group: [group] if group in self.live_groups else []
            result.loader.exec_module = load
            return result
        previous = Path.cwd()
        try:
            os.chdir(self.workspace)
            with patch('importlib.util.spec_from_file_location', side_effect=supervisor_spec), \
                 patch.dict(os.environ, {'G77_STARTED': str(int(time.time()))}):
                exec(compile(self.archive_code, 'workflow-archive', 'exec'), {})
        finally:
            os.chdir(previous)

    def members(self):
        with tarfile.open(self.evidence / 'samples.tgz') as archive:
            return archive.getnames()

    def test_emergency_and_stopped_sample_reach_upload(self):
        for stale in [None, {'stopped': False, 'process_group': 123}]:
            with self.subTest(stale=stale):
                if (self.evidence / 'trace').exists():
                    shutil.rmtree(self.evidence / 'trace')
                self.cases.clear()
                old = self.sample('trace/old', stale)
                self.emergency(old)
                self.sample('trace/new', {'stopped': True, 'process_group': 124})
                self.archive()
                members = self.members()
                self.assertIn('trace/old.stopped-originals.tgz', members)
                with tarfile.open(self.evidence / 'samples.tgz') as outer:
                    with tarfile.open(fileobj=outer.extractfile('trace/old.stopped-originals.tgz')) as inner:
                        self.assertEqual(inner.extractfile('old/w2.run/s.sqlite-wal').read(), b'synthetic original')
                self.assertIn('trace/new/w2.run/s.sqlite-wal', members)
                self.assertNotIn('trace/old/w2.run/s.sqlite', members)
                status = json.loads((self.evidence / 'archive-check.json').read_text())
                self.assertEqual(status['emergency_archives'][0]['case'], 'trace/old')
                self.assertEqual(status['rejected_cases'], [])
                self.assertIn('g77-evidence/samples.tgz', self.upload)

    def test_live_writer_rejected_without_losing_stopped_sample(self):
        self.sample('trace/old', {'stopped': True, 'process_group': 123})
        self.sample('trace/new', {'stopped': True, 'process_group': 999})
        self.live_groups.add(999)
        with self.assertRaisesRegex(AssertionError, 'trace/new'):
            self.archive()
        members = self.members()
        self.assertIn('trace/old/w2.run/s.sqlite', members)
        self.assertFalse(any(name.startswith('trace/new') for name in members))

    def test_missing_or_stale_receipt_is_not_stopped(self):
        old = self.sample('trace/old')
        self.emergency(old)
        (old / 'supervisor.stderr').unlink()
        self.sample('trace/new', {'stopped': False, 'process_group': 123})
        with self.assertRaises(AssertionError):
            self.archive()
        self.assertEqual(self.members(), [])
        status = json.loads((self.evidence / 'archive-check.json').read_text())
        self.assertEqual(len(status['rejected_cases']), 2)

    def test_unstarted_copy_is_retained(self):
        self.sample('trace/old', launched=False)
        self.archive()
        self.assertIn('trace/old/w2.run/s.sqlite', self.members())

    def test_stale_unstarted_summary_cannot_authorize_writer_copy(self):
        old = self.sample('trace/old', launched=False)
        (old / 'supervisor.stderr').write_text('')
        with self.assertRaises(AssertionError):
            self.archive()
        self.assertEqual(self.members(), [])

    def test_incomplete_or_live_emergency_is_rejected(self):
        old = self.sample('trace/old')
        self.emergency(old, complete=False)
        new = self.sample('trace/new', {'stopped': False, 'process_group': 999})
        self.emergency(new, group=999)
        self.live_groups.add(999)
        with self.assertRaises(AssertionError):
            self.archive()
        self.assertEqual(self.members(), [])


class IntegrationWorkflowTest(unittest.TestCase):
    def test_development_count_matches_source_and_gate(self):
        root = Path(__file__).resolve().parents[2]
        source = (root / 'rust/src/bin/s_session_v3/source_audit_development_tests.rs').read_text()
        tests = re.findall(r'(?m)^#\[test\]\nfn (\w+)\(', source)
        self.assertEqual(len(tests), 10)
        self.assertNotIn('#[ignore]', source)
        workflow = (root / '.github/workflows/issue-1419-integration33z.yml').read_text()
        gate = next(line.strip() for line in workflow.splitlines() if 'grep ' in line and 'development35a.log' in line)
        expected = re.search(r"'([^']+)'", gate).group(1)
        self.assertIn(f'{len(tests)} passed; 0 failed', expected)
        for passed, failed, accepted in [(10, 0, True), (8, 0, False), (0, 0, False), (10, 1, False)]:
            result = subprocess.run(['grep', '-F', expected], input=f'test result: ok. {passed} passed; {failed} failed;\n',
                                    text=True, capture_output=True)
            self.assertEqual(result.returncode == 0, accepted)


if __name__ == '__main__':
    unittest.main()
