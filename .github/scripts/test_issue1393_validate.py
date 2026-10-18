#!/usr/bin/env python3
"""#1393 仅标准库纯模型与临时 Python toy；从不调用产品或编译器。"""
import copy
import importlib.util
import json
import os
from pathlib import Path
import signal
import sys
import tempfile
import time
import unittest
from unittest.mock import patch

SPEC = importlib.util.spec_from_file_location('issue1393_validate', Path(__file__).with_name('issue1393_validate.py'))
v = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(v)


def contract():
    data = {'schema': 'issue1393-validation/1', 'status': 'AUTHORIZED_ONCE',
            'run_attempt': '1', 'lib_features': ['default'], 's_features': ['default', 's_session'],
            'total_seconds': 3600, 'approval_text': 'toy 根批准原件', 'review_text': 'toy 独审原件'}
    for key in ('source_commit', 'source_tree', 'prepared_commit', 'prepared_tree'):
        data[key] = 'ab' * 20
    for key in ('product_manifest_sha256', 'runner_sha256', 'workflow_false_sha256', 'ci_sha256', 'telemetry_sha256'):
        data[key] = 'ab' * 32
    for key in ('approval', 'review'):
        data[key + '_sha256'] = v.digest(data[key + '_text'].encode())
    return data


def budget(seconds=10):
    return v.Budget({'monotonic': time.monotonic() - v.TOTAL_SECONDS + v.FINAL_RESERVE + seconds,
                     'wall': time.time() - v.TOTAL_SECONDS + v.FINAL_RESERVE + seconds,
                     'total_seconds': 3600})


class GateTests(unittest.TestCase):
    def test_false_main_never_starts_any_external_command(self):
        with tempfile.TemporaryDirectory() as directory:
            p = Path(directory)
            (p / 'contract.json').write_text(json.dumps(contract()))
            with patch.dict(os.environ, {'ISSUE1393_EXECUTE': 'False'}), patch.object(v.subprocess, 'Popen') as spawn:
                rc = v.main(['--contract', str(p / 'contract.json'), '--start', str(p / 'MISSING'), '--output', str(p / 'out')])
            self.assertNotEqual(rc, 0)
            spawn.assert_not_called()
            result = json.loads((p / 'out/result.json').read_text())
            self.assertEqual(result['real_execution'], 'NOT_RUN')
            rows = json.loads((p / 'out/steps.json').read_text())
            self.assertTrue(all(row['status'] == 'NOT_RUN' for row in rows))

    def test_unknown_or_unbound_identity_rejected_before_spawn(self):
        variants = []
        for key in ('source_commit', 'source_tree', 'runner_sha256', 'workflow_false_sha256', 'product_manifest_sha256'):
            for value in (None, '', 'UNBOUND', '0' * (64 if 'sha256' in key else 40)):
                item = contract()
                item[key] = value
                variants.append(item)
        item = contract()
        item['approval_text'] += '被改'
        variants.append(item)
        with patch.object(v.subprocess, 'Popen') as spawn:
            for item in variants:
                with self.subTest(item=item), self.assertRaises(v.Refused):
                    v.validate_contract('True', item)
            for gate in ('', 'true', 'False', None):
                with self.assertRaises(v.Refused):
                    v.validate_contract(gate, contract())
            spawn.assert_not_called()

    def test_source_tree_and_hash_mismatch_stop_at_binding(self):
        from types import SimpleNamespace
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory).resolve()
            c = contract()
            head = 'cd' * 20
            workflow = b'name: toy\nenv:\n' + v.GATE_FALSE
            for relative, content in ((v.WORKFLOW, workflow.replace(v.GATE_FALSE, v.GATE_TRUE)),
                                      (v.RUNNER, b'toy runner'), ('.github/workflows/ci.yml', b'toy ci'),
                                      ('.github/scripts/run-cargo-with-telemetry.sh', b'toy telemetry')):
                path = root / relative
                path.parent.mkdir(parents=True, exist_ok=True)
                path.write_bytes(content)
            for relative, key in ((v.RUNNER, 'runner_sha256'), ('.github/workflows/ci.yml', 'ci_sha256'),
                                  ('.github/scripts/run-cargo-with-telemetry.sh', 'telemetry_sha256')):
                c[key] = v.digest((root / relative).read_bytes())
            c['workflow_false_sha256'] = v.digest(workflow)
            tree = b'100644 blob abc\trust/src/lib.rs\0'
            c['product_manifest_sha256'] = v.digest(v.canonical(v.product_entries(tree)))
            replies = {'git-head': head.encode(), 'git-parent': (head + ' ' + c['prepared_commit']).encode(),
                       'git-source-tree': c['source_tree'].encode(), 'git-prepared-tree': c['prepared_tree'].encode(),
                       'git-dirty': b'', 'git-activation-paths': (v.WORKFLOW + '\0').encode(),
                       'workflow-prepared': workflow, 'source-ls-tree': tree, 'execution-ls-tree': tree,
                       'execution-tree': b'ef' * 20}
            fake = SimpleNamespace(output=root, run=lambda *a: {'name': 'git-ancestor', 'status': 'PASS'})
            with patch.dict(os.environ, {'GITHUB_SHA': head, 'GITHUB_REPOSITORY': 'toy/repo',
                                         'GITHUB_REF': 'refs/heads/toy', 'GITHUB_WORKFLOW_SHA': head,
                                         'GITHUB_WORKFLOW_REF': 'toy/repo/' + v.WORKFLOW + '@refs/heads/toy'}), patch.object(v, 'git_read', side_effect=lambda s, r, n, *a: replies[n]):
                v.bind_source(root, c, fake)
                for name, wrong in (('git-source-tree', b'12' * 20), ('git-head', b'12' * 20),
                                    ('execution-ls-tree', b'100644 blob changed\trust/src/lib.rs\0'),
                                    ('git-dirty', b' M rust/src/lib.rs')):
                    original = replies[name]
                    replies[name] = wrong
                    with self.subTest(name=name), self.assertRaises(v.Refused):
                        v.bind_source(root, c, fake)
                    replies[name] = original
                for key in ('runner_sha256', 'workflow_false_sha256', 'product_manifest_sha256'):
                    bad = dict(c, **{key: '12' * 32})
                    with self.subTest(key=key), self.assertRaises(v.Refused):
                        v.bind_source(root, bad, fake)

    def test_only_one_activation_change_allowed(self):
        prepared = b'name: toy\nenv:\n' + v.GATE_FALSE
        executed = prepared.replace(v.GATE_FALSE, v.GATE_TRUE)
        v.verify_activation(prepared, executed)
        for bad in (prepared, executed + b'# changed\n', executed.replace(b'toy', b'other')):
            with self.assertRaises(v.Refused):
                v.verify_activation(prepared, bad)
        with self.assertRaises(v.Refused):
            v.verify_activation(prepared + v.GATE_FALSE, executed)

    def test_product_digest_cannot_hide_product_change(self):
        source = b'100644 blob abc\trust/src/lib.rs\0'
        changed = b'100644 blob def\trust/src/lib.rs\0'
        control = b'100644 blob control\t.github/scripts/issue1393_validate.py\0'
        self.assertNotEqual(v.product_entries(source), v.product_entries(changed))
        self.assertEqual(v.product_entries(source), v.product_entries(source + control))


class BinaryTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name).resolve()
        self.target_dir = self.root / 'one-run-target'
        self.binary = self.target_dir / 'debug/deps/lib-toy'
        self.binary.parent.mkdir(parents=True)
        self.binary.write_text('#!/bin/sh\nexit 0\n')
        self.binary.chmod(0o700)
        self.target = v.TARGETS['lib']
        self.artifact = {'reason': 'compiler-artifact', 'profile': {'test': True},
                         'target': {'name': self.target['name'], 'kind': self.target['kind'],
                                    'src_path': str(self.root / self.target['src'])},
                         'manifest_path': str(self.root / 'rust/Cargo.toml'), 'features': ['default'],
                         'executable': str(self.binary), 'filenames': [str(self.binary)], 'fresh': False}

    def raw(self, *artifacts):
        return b'\n'.join(json.dumps(x).encode() for x in [*artifacts, {'reason': 'build-finished', 'success': True}])

    def test_current_cargo_artifact_and_replacement_detection(self):
        self.assertEqual(v.select_binary(self.raw(self.artifact), self.target, self.root, self.target_dir), self.binary)
        before = v.file_identity(self.binary)
        replacement = self.binary.with_suffix('.replacement')
        replacement.write_bytes(self.binary.read_bytes())
        replacement.chmod(0o700)
        replacement.replace(self.binary)
        self.assertNotEqual(before, v.file_identity(self.binary))

    def test_stale_foreign_missing_and_duplicate_artifacts_rejected(self):
        variants = [self.raw(), self.raw(self.artifact, self.artifact), b'', b'{malformed}']
        for key, value in (('fresh', True), ('features', ['default', 'backtest_bin']),
                           ('executable', ''), ('executable', '/tmp/another-target/bin'),
                           ('manifest_path', '/tmp/other/Cargo.toml'), ('filenames', [])):
            item = copy.deepcopy(self.artifact)
            item[key] = value
            variants.append(self.raw(item))
        item = copy.deepcopy(self.artifact)
        item['target']['src_path'] = '/tmp/foreign/lib.rs'
        variants.append(self.raw(item))
        variants.append(json.dumps(self.artifact).encode())
        for raw in variants:
            with self.subTest(raw=raw), self.assertRaises((v.Refused, ValueError)):
                v.select_binary(raw, self.target, self.root, self.target_dir)

    def test_list_missing_zero_duplicate_and_short_name_ambiguity_rejected(self):
        expected = {'module::wanted': 'source.rs'}
        self.assertEqual(v.verify_list(b'module::wanted: test\n1 test, 0 benchmarks\n', expected), ['module::wanted'])
        for raw in (b'', b'other: test\n', b'module::wanted: test\nmodule::wanted: test\n',
                    b'module::wanted: test\nother::wanted: test\n', b'module::wanted: benchmark\n'):
            with self.subTest(raw=raw), self.assertRaises(v.Refused):
                v.verify_list(raw, expected)

    def test_zero_test_exit_zero_and_ignored_are_not_passes(self):
        good = b'test module::wanted ... ok\n\ntest result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 40 filtered out; finished in 0.01s\n'
        v.verify_exact(good, 'module::wanted')
        for bad in (good.replace(b'1 passed', b'0 passed'), good.replace(b'0 ignored', b'1 ignored'),
                    good.replace(b' ... ok', b' ... ignored'), good.replace(b'module::wanted', b'other::wanted')):
            with self.assertRaises(v.Refused):
                v.verify_exact(bad, 'module::wanted')

    def test_real_source_contains_all_nine_declared_tests(self):
        root = Path(__file__).resolve().parents[2]
        v.verify_source_names(root, v.LIB_CASES)
        v.verify_source_names(root, v.S_CASES)
        self.assertEqual((len(v.LIB_CASES), len(v.S_CASES)), (7, 2))


class SupervisionTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)

    def run_toy(self, supervisor, name, code):
        return supervisor.run(name, [sys.executable, '-c', code], self.root)

    def test_nonzero_exit_stops_following_work(self):
        supervisor = v.Supervisor(self.root, budget())
        first = self.run_toy(supervisor, 'failure', 'import sys; print("original"); sys.exit(7)')
        second = self.run_toy(supervisor, 'later', 'raise AssertionError("must not run")')
        self.assertEqual(first['exit'], 7)
        self.assertEqual(first['status'], 'FAILED')
        self.assertTrue(first['cleanup'][-1]['reaped'])
        self.assertEqual(second['status'], 'NOT_RUN')
        self.assertFalse((self.root / 'later.stdout').exists())

    def test_single_deadline_is_not_reset_after_success(self):
        supervisor = v.Supervisor(self.root, budget(0.4))
        first = self.run_toy(supervisor, 'first', 'import time; time.sleep(0.12)')
        second = self.run_toy(supervisor, 'second', 'import time; time.sleep(2)')
        third = self.run_toy(supervisor, 'third', 'print("forbidden")')
        self.assertEqual(first['status'], 'PASS')
        self.assertLess(second['remaining_seconds'], first['remaining_seconds'] - 0.08)
        self.assertEqual(second['status'], 'TIMEOUT')
        self.assertEqual(third['status'], 'NOT_RUN')
        self.assertLess(second['ended_monotonic'] - first['started_monotonic'], 1.2)

    def test_term_ignored_escalates_to_kill_and_is_reaped(self):
        supervisor = v.Supervisor(self.root, budget(0.4))
        row = self.run_toy(supervisor, 'ignore-term',
                           'import signal,time; signal.signal(signal.SIGTERM, signal.SIG_IGN); print("ready", flush=True); time.sleep(4)')
        self.assertEqual(row['status'], 'TIMEOUT')
        self.assertEqual(row['exit'], -signal.SIGKILL)
        self.assertEqual([x['signal'] for x in row['cleanup'] if 'signal' in x], ['SIGTERM', 'SIGKILL'])
        self.assertTrue(row['cleanup'][-1]['reaped'])
        self.assertTrue(row['cleanup'][-1]['group_gone'])

    def test_signal_is_recorded_and_later_work_not_run(self):
        supervisor = v.Supervisor(self.root, budget())
        old = signal.signal(signal.SIGTERM, supervisor.handle_signal)
        try:
            row = self.run_toy(supervisor, 'signal',
                               'import os,signal,time; os.kill(os.getppid(), signal.SIGTERM); time.sleep(3)')
        finally:
            signal.signal(signal.SIGTERM, old)
        self.assertEqual(row['status'], 'SIGNALLED')
        self.assertEqual(row['received_signal'], signal.SIGTERM)
        self.assertEqual(row['exit'], -signal.SIGTERM)
        self.assertEqual(self.run_toy(supervisor, 'later', 'print(1)')['status'], 'NOT_RUN')

    def test_spawn_failure_and_cleanup_failure_remain_failures(self):
        supervisor = v.Supervisor(self.root, budget())
        row = supervisor.run('missing', ['/nonexistent/issue1393-toy'], self.root)
        self.assertEqual(row['status'], 'ERROR')
        self.assertIsNone(row['exit'])
        supervisor = v.Supervisor(self.root, budget())
        with patch.object(supervisor, 'cleanup', return_value=([{'error': 'toy reap denied'}], False)):
            row = self.run_toy(supervisor, 'cleanup', 'print("done")')
        self.assertEqual(row['status'], 'CLEANUP_FAILED')
        self.assertEqual(row['exit'], 0)
        self.assertTrue(supervisor.stopped)

    def test_unknown_process_group_is_never_reported_reclaimed(self):
        supervisor = v.Supervisor(self.root, budget())
        with patch.object(v.os, 'killpg', side_effect=PermissionError('toy probe denied')):
            with patch.object(supervisor.budget, 'remaining', return_value=0.02):
                row = self.run_toy(supervisor, 'denied-group', 'pass')
        self.assertEqual(row['status'], 'CLEANUP_FAILED')
        self.assertTrue(row['cleanup'][-1]['reaped'])
        self.assertFalse(row['cleanup'][-1]['group_gone'])
        self.assertTrue(any('probe_error' in event for event in row['cleanup']))

    def test_telemetry_and_fixture_keep_actual_exit_codes_separate(self):
        wrapper = self.root / '.github/scripts/run-cargo-with-telemetry.sh'
        wrapper.parent.mkdir(parents=True)
        wrapper.write_text('#!/bin/bash\n"$@"\n')
        wrapper.chmod(0o700)
        for code, meaning in v.FIXTURE_EXITS.items():
            out = self.root / ('exit-' + str(code))
            out.mkdir()
            supervisor = v.Supervisor(out, budget())
            command = [sys.executable, '-c', 'import sys; print("fixture toy"); sys.exit(' + str(code) + ')']
            if code == 0:
                v.telemetry_command(supervisor, self.root, 'fixture', command, self.root, os.environ.copy())
            else:
                with self.assertRaises(v.Refused):
                    v.telemetry_command(supervisor, self.root, 'fixture', command, self.root, os.environ.copy())
            self.assertEqual(supervisor.records[-1]['command_exit'], code)
            self.assertEqual(supervisor.records[-1]['fixture_meaning'], meaning)
        wrapper.write_text('#!/bin/bash\n"$@"\nexit 9\n')
        out = self.root / 'wrapper-failure'
        out.mkdir()
        supervisor = v.Supervisor(out, budget())
        with self.assertRaises(v.Refused):
            v.telemetry_command(supervisor, self.root, 'toy', [sys.executable, '-c', 'print("ok")'], self.root, os.environ.copy())
        self.assertEqual(supervisor.records[-1]['command_exit'], 0)
        self.assertEqual(supervisor.records[-1]['exit'], 9)
        wrapper.write_text('#!/bin/bash\nexit 8\n')
        out = self.root / 'before-command-failure'
        out.mkdir()
        supervisor = v.Supervisor(out, budget())
        with self.assertRaises(v.Refused):
            v.telemetry_command(supervisor, self.root, 'toy', [sys.executable, '-c', 'print("forbidden")'], self.root, os.environ.copy())
        self.assertEqual(supervisor.records[-1]['command_status'], 'NOT_RUN')
        self.assertIsNone(supervisor.records[-1]['command_exit'])

    def test_wall_clock_can_shorten_but_not_extend_monotonic_deadline(self):
        b = budget(1)
        mono, wall = time.monotonic(), time.time()
        with patch.object(v.time, 'monotonic', return_value=mono), patch.object(v.time, 'time', return_value=wall - 100):
            remaining = b.remaining(v.FINAL_RESERVE)
            self.assertLessEqual(remaining, 1)
        with patch.object(v.time, 'time', return_value=wall + 100):
            self.assertLess(b.remaining(v.FINAL_RESERVE), 0)

    def test_package_missing_evidence_and_zip_failure_are_errors(self):
        with self.assertRaises(v.Refused):
            v.package(self.root, budget())
        v.save(self.root / 'result.json', {'status': 'FAILED'})
        v.save(self.root / 'steps.json', [{'status': 'NOT_RUN'}])
        with patch.object(v.zipfile, 'ZipFile', side_effect=OSError('toy full disk')):
            with self.assertRaisesRegex(OSError, 'toy full disk'):
                v.package(self.root, budget())
        v.package(self.root, budget())
        self.assertEqual(json.loads((self.root / 'PACKAGE.json').read_text())['status'], 'PACKAGED')
        with v.zipfile.ZipFile(self.root / 'evidence.zip') as archive:
            self.assertEqual(json.loads(archive.read('result.json'))['status'], 'FAILED')


if __name__ == '__main__':
    unittest.main()
