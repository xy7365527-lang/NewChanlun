#!/usr/bin/env python3
"""#1393 仅标准库/内存/临时 Python toy；不执行 Rust、网络或产品。"""
import copy
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import time
import unittest
from types import SimpleNamespace
from unittest.mock import patch

SPEC = importlib.util.spec_from_file_location('lock_prepare', Path(__file__).with_name('issue1393_lock_prepare.py'))
m = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(m)
v = m.v


def contract():
    c = copy.deepcopy(m.FIXED)
    c.update({k: 'ab' * 20 for k in v.GIT_FIELDS})
    c.update({k: 'ab' * 32 for k in v.HASH_FIELDS})
    for key in ('approval', 'review'):
        c[key + '_text'] = 'toy 原件，仅溯源，不授予运行许可：' + key
        c[key + '_sha256'] = m.digest(c[key + '_text'].encode())
    c['helper_sha256'] = m.HELPER_SHA256
    return c


def message(c=None):
    return v.MESSAGE_BEGIN + json.dumps(c or contract(), ensure_ascii=False) + v.MESSAGE_END


def github_env(head='cd' * 20):
    return {'GITHUB_ACTIONS': 'true', 'RUNNER_OS': 'Linux', 'GITHUB_REPOSITORY': m.REPOSITORY,
            'GITHUB_REF': m.PUSH_REF, 'GITHUB_EVENT_NAME': 'push', 'GITHUB_RUN_ATTEMPT': '1',
            'GITHUB_JOB': 'lock-prepare', 'GITHUB_SHA': head, 'GITHUB_WORKFLOW_SHA': head,
            'GITHUB_WORKFLOW_REF': m.REPOSITORY + '/' + m.WORKFLOW + '@' + m.PUSH_REF}


def event(c=None, head='cd' * 20):
    c = c or contract()
    return {'repository': {'full_name': m.REPOSITORY}, 'ref': m.PUSH_REF,
            'created': False, 'deleted': False, 'forced': False,
            'before': c['prepared_commit'], 'after': head, 'head_commit': {'id': head, 'message': message(c)}}


def budget(seconds=5):
    return v.Budget({'monotonic': time.monotonic() - 900 + m.FINAL_RESERVE + seconds,
                     'wall': time.time() - 900 + m.FINAL_RESERVE + seconds, 'total_seconds': 900})


def blob(raw):
    return hashlib.sha1(b'blob ' + str(len(raw)).encode() + b'\0' + raw).hexdigest()


class ContractTests(unittest.TestCase):
    def test_false_zero_commands_and_no_event_read(self):
        with patch.dict(os.environ, {'ISSUE1393_LOCK_EXECUTE': 'False'}, clear=True), \
                patch.object(subprocess, 'Popen') as popen, patch.object(v, 'read_push_contract') as read:
            self.assertEqual(m.main(), 1)
            popen.assert_not_called()
            read.assert_not_called()

    def test_draft_is_rejected(self):
        draft = m.ROOT / (m.REPORT + 'CONTRACT-DRAFT.json')
        with self.assertRaises(v.Refused):
            v.validate_contract('True', json.loads(draft.read_text()))

    def test_every_contract_field_required_typed_and_bound(self):
        c = contract()
        v.validate_contract('True', c)
        variants = []
        for k in c:
            item = copy.deepcopy(c)
            del item[k]
            variants.append(item)
            for bad in (None, True, {}, 'UNBOUND'):
                variants.append(dict(c, **{k: bad}))
        variants.extend([dict(c, extra='bad'), dict(c, helper_sha256='0' * 64)])
        with patch.object(subprocess, 'Popen') as popen:
            for bad in variants:
                with self.subTest(bad=bad), self.assertRaises(v.Refused):
                    v.validate_contract('True', bad)
            popen.assert_not_called()

    def test_strict_envelope_duplicates_and_limits(self):
        msg = message()
        self.assertEqual(v.parse_message(msg), contract())
        for bad in ('', '{}', msg + msg, 'title\n' + msg, msg + '\nextra', msg + '\n\n',
                    msg.replace('"schema":', '"schema": "duplicate", "schema":', 1),
                    msg.replace('"total_seconds": 900', '"total_seconds": NaN'),
                    v.MESSAGE_BEGIN + '[' * 1200 + v.MESSAGE_END, 'x' * 32769):
            with self.subTest(bad=bad[:100]), self.assertRaises(v.Refused):
                v.parse_message(bad)

    def test_wrong_hashes_and_provenance(self):
        c = contract()
        for k in v.HASH_FIELDS:
            for value in ('UNBOUND', 'g' * 64, 'a' * 64, 'ab' * 31):
                with self.assertRaises(v.Refused):
                    v.validate_contract('True', dict(c, **{k: value}))
        for key in ('approval', 'review'):
            for text in ('UNBOUND', ' ', 'x' * 8193, c[key + '_text'] + '!'):
                with self.assertRaises(v.Refused):
                    v.validate_contract('True', dict(c, **{key + '_text': text}))

    def test_push_identity_and_attempt(self):
        c, head = contract(), 'cd' * 20
        with patch.dict(os.environ, github_env(), clear=True), patch.object(sys, 'platform', 'linux'):
            m.validate_push(event(c), c, head)
            for key in github_env():
                with patch.dict(os.environ, {key: 'wrong'}), self.assertRaises(v.Refused):
                    m.validate_push(event(c), c, head)
            for key, val in [('before', '0' * 40), ('after', '12' * 20), ('created', True),
                             ('deleted', True), ('forced', True), ('forced', 0), ('ref', 'refs/heads/main')]:
                with self.assertRaises(v.Refused):
                    m.validate_push(dict(event(c), **{key: val}), c, head)

    def test_bad_event_zero_commands_and_keeps_original(self):
        with tempfile.TemporaryDirectory() as d:
            p = Path(d)
            (p / 'issue1393-lock-start.json').write_text(json.dumps({'monotonic': time.monotonic(), 'wall': time.time(), 'total_seconds': 900}))
            raw = json.dumps({'head_commit': {'message': 'bad'}}).encode()
            (p / 'event.json').write_bytes(raw)
            env = dict(github_env(), ISSUE1393_LOCK_EXECUTE='True', RUNNER_TEMP=d, GITHUB_EVENT_PATH=str(p / 'event.json'))
            with patch.dict(os.environ, env, clear=True), patch.object(subprocess, 'Popen') as popen:
                self.assertEqual(m.main(), 1)
                popen.assert_not_called()
            out = p / 'issue1393-lock-evidence'
            self.assertEqual((out / 'push-event.json').read_bytes(), raw)
            self.assertTrue((out / 'evidence.zip').exists())
            self.assertTrue(all(x['status'] == 'NOT_RUN' for x in json.loads((out / 'steps.json').read_text())))

    def test_message_never_becomes_path_or_argv(self):
        c = contract()
        c['approval_text'] = '$(touch /tmp/no) ; `whoami` ../x --build'
        c['approval_sha256'] = m.digest(c['approval_text'].encode())
        v.parse_message(message(c))
        self.assertEqual(m.resolution_argv(Path('/fixed')), ['cargo', '+' + m.TOOLCHAIN, 'generate-lockfile', '--manifest-path', '/fixed/rust/Cargo.toml'])
        with tempfile.TemporaryDirectory() as d:
            s = v.Supervisor(Path(d), budget())
            with patch.object(subprocess, 'Popen') as popen:
                row = s.run('bad-stdin', ['git', 'diff-tree', '--stdin'], d, input_data=b'$(whoami)')
            self.assertEqual(row['status'], 'ERROR')
            popen.assert_not_called()

    def test_fixed_module_hash_checked_before_execution(self):
        with patch.object(m, 'HELPER_SHA256', 'ab' * 32), self.assertRaises(RuntimeError):
            m.load_fixed_helper()


class IdentityTests(unittest.TestCase):
    def test_complete_binding_and_mutations(self):
        with tempfile.TemporaryDirectory() as d:
            root = Path(d)
            c, head = contract(), 'cd' * 20
            false = b'name: toy\nenv:\n' + m.GATE_FALSE
            contents = {m.WORKFLOW: false.replace(m.GATE_FALSE, m.GATE_TRUE), m.RUNNER: b'runner',
                        '.github/workflows/ci.yml': b'ci', m.TELEMETRY: b'telemetry',
                        m.HELPER: (m.ROOT / m.HELPER).read_bytes(), 'rust/Cargo.toml': b'manifest',
                        'rust/Cargo.lock': b'lock', '.github/workflows/issue1393-validation.yml': b'old workflow',
                        m.REPORT + 'RUST-TOOLCHAIN-39091453686.txt': (m.RUSTC_FIRST + '\n' + m.RUSTC_COMMIT + '\n' + m.CARGO_VERSION).encode()}
            for p, raw in contents.items():
                path = root / p
                path.parent.mkdir(parents=True, exist_ok=True)
                path.write_bytes(raw)
            mappings = {m.RUNNER: 'runner_sha256', '.github/workflows/ci.yml': 'ci_sha256', m.TELEMETRY: 'telemetry_sha256',
                        m.HELPER: 'helper_sha256', 'rust/Cargo.toml': 'manifest_sha256', 'rust/Cargo.lock': 'old_lock_sha256',
                        '.github/workflows/issue1393-validation.yml': 'validation_workflow_sha256',
                        m.REPORT + 'RUST-TOOLCHAIN-39091453686.txt': 'toolchain_evidence_sha256'}
            for p, k in mappings.items():
                c[k] = m.digest(contents[p])
            c['workflow_false_sha256'] = m.digest(false)
            tree = b'100644 blob ' + b'ab' * 20 + b'\trust/src/lib.rs\0'
            c['product_manifest_sha256'] = m.digest(v.canonical(v.product_entries(tree)))
            replies = {'git-head': head.encode(), 'git-message': b'tree toy\n\n' + message(c).encode(),
                       'git-parent': (head + ' ' + c['prepared_commit']).encode(),
                       'git-ancestry': (c['source_commit'] + ' ' + c['source_tree']).encode(),
                       'git-prepared-tree': c['prepared_tree'].encode(), 'git-dirty': b'',
                       'git-activation-paths': (':100644 100644 ' + 'ab' * 20 + ' ' + 'cd' * 20 + ' M\0' + m.WORKFLOW + '\0').encode(),
                       'workflow-prepared': false, 'source-prepared-paths': b'',
                       'prepared-ls-tree': tree, 'execution-ls-tree': tree, 'execution-tree': b'ef' * 20}
            s = SimpleNamespace(output=root)
            with patch.dict(os.environ, github_env(), clear=True), patch.object(sys, 'platform', 'linux'), \
                    patch.object(v, 'git_read', side_effect=lambda _s, _r, n, *a: replies[n]):
                m.bind_source(root, c, s, event(c))
                for p in contents:
                    (root / p).write_bytes(contents[p] + b'!')
                    with self.subTest(path=p), self.assertRaises(v.Refused):
                        m.bind_source(root, c, s, event(c))
                    (root / p).write_bytes(contents[p])
                mutations = [('git-ancestry', b''), ('git-prepared-tree', b'12' * 20),
                             ('git-parent', (head + ' ' + c['prepared_commit'] + ' ' + 'ef' * 20).encode()),
                             ('git-message', replies['git-message'] + b'\nextra'), ('git-dirty', b' M rust/Cargo.lock'),
                             ('execution-ls-tree', tree + b'100644 blob changed\trust/extra.rs\0')]
                mutations += [('source-prepared-paths', (p + '\0').encode()) for p in
                              ('rust/Cargo.toml', 'rust/Cargo.lock', m.HELPER, '.github/workflows/ci.yml',
                               '.github/workflows/issue1393-validation.yml', '.github/scripts/test_issue1393_validate.py')]
                mutations += [('git-activation-paths', replies['git-activation-paths'].replace(b':100644', b':100755'))]
                for key, bad in mutations:
                    old = replies[key]
                    replies[key] = bad
                    with self.subTest(key=key), self.assertRaises(v.Refused):
                        m.bind_source(root, c, s, event(c))
                    replies[key] = old

    def test_unique_gate(self):
        false = b'env:\n' + m.GATE_FALSE
        v.verify_activation(false, false.replace(m.GATE_FALSE, m.GATE_TRUE))
        for a, b in ((false, false), (false, false.replace(m.GATE_FALSE, m.GATE_TRUE) + b'x'),
                     (false + m.GATE_FALSE, false + m.GATE_TRUE)):
            with self.assertRaises(v.Refused):
                v.verify_activation(a, b)


class ResolutionTests(unittest.TestCase):
    def test_target_selection_copies_actual_entries_only(self):
        entries = {p: ('100644', 'blob', 'ab' * 20) for p in
                   ('rust/Cargo.toml', 'rust/Cargo.lock', 'rust/src/lib.rs', 'rust/src/bin/a.rs',
                    'rust/src/bin/b/main.rs', 'rust/tests/a.rs', 'rust/src/module.rs',
                    'rust/tests/fixtures/data.json', 'rust/.env', 'rust/.cargo/config.toml')}
        selected = m.target_sources({'package': {}, 'lib': {}, 'bin': [{'path': 'src/bin/a.rs'}]}, entries)
        self.assertEqual(selected, sorted(['rust/Cargo.toml', 'rust/Cargo.lock', 'rust/src/lib.rs',
                                          'rust/src/bin/a.rs', 'rust/src/bin/b/main.rs', 'rust/tests/a.rs']))
        for manifest in ({'workspace': {}}, {'patch': {}}, {'dependencies': {'x': {'path': '../x'}}},
                         {'dependencies': {'x': {'git': 'https://x'}}}, {'bin': [{'path': '../x.rs'}]},
                         {'lib': {'path': '/x.rs'}}, {'package': {'build': '/x'}}):
            with self.assertRaises(v.Refused):
                m.target_sources(manifest, entries)

    def test_source_blob_tamper_and_symlink(self):
        with tempfile.TemporaryDirectory() as d:
            root = Path(d)
            (root / 'a.rs').write_bytes(b'actual source')
            entries = {'a.rs': ('100644', 'blob', blob(b'actual source'))}
            self.assertEqual(m.verified_bytes(root, 'a.rs', entries), b'actual source')
            (root / 'a.rs').write_bytes(b'fake source')
            with self.assertRaises(v.Refused):
                m.verified_bytes(root, 'a.rs', entries)
            (root / 'a.rs').unlink()
            (root / 'b.rs').write_bytes(b'actual source')
            (root / 'a.rs').symlink_to('b.rs')
            with self.assertRaises(v.Refused):
                m.verified_bytes(root, 'a.rs', entries)

    def test_fixed_toolchain_rejects_drift(self):
        good = (m.RUSTC_FIRST + '\ncommit-hash: ' + m.RUSTC_COMMIT + '\nrelease: 1.99.0\nhost: x86_64-unknown-linux-gnu\n').encode()
        cargo = m.CARGO_VERSION.encode()
        m.verify_toolchain(good, cargo)
        for old, new in ((b'1.99.0', b'1.100.0'), (m.RUSTC_COMMIT.encode(), b'ab' * 20),
                         (b'x86_64-unknown-linux-gnu', b'aarch64-apple-darwin')):
            with self.assertRaises(v.Refused):
                m.verify_toolchain(good.replace(old, new), cargo)
        with self.assertRaises(v.Refused):
            m.verify_toolchain(good, b'cargo stable')

    def test_clean_environment_drops_all_injected_settings(self):
        with tempfile.TemporaryDirectory() as d, patch.dict(os.environ, {
                'GITHUB_TOKEN': 'toy', 'CARGO_REGISTRIES_OTHER_TOKEN': 'toy', 'RUSTFLAGS': '--bad',
                'CARGO_HOME': '/bad', 'HTTP_PROXY': 'bad', 'CARGO_TELEMETRY_TEST_SAMPLE_SECONDS': '0.1'}):
            env = m.isolated_environment(Path(d))
            self.assertFalse(any(k in env for k in ('GITHUB_TOKEN', 'CARGO_REGISTRIES_OTHER_TOKEN', 'RUSTFLAGS', 'HTTP_PROXY', 'CARGO_TELEMETRY_TEST_SAMPLE_SECONDS')))
            self.assertEqual(env['CARGO_BUILD_JOBS'], '1')
            self.assertEqual(env['RUSTUP_DIST_SERVER'], 'https://static.rust-lang.org')
            self.assertEqual(env['CARGO_HOME'], d + '/cargo-home')

    def test_shared_deadline_and_failure_no_rerun_preserves_output(self):
        with tempfile.TemporaryDirectory() as d:
            p = Path(d)
            b = budget(1)
            s = v.Supervisor(p, b)
            first = s.run('fail', [sys.executable, '-B', '-c', 'print("raw evidence");raise SystemExit(7)'], p)
            self.assertEqual(first['exit'], 7)
            self.assertTrue(first['cleanup'][-1]['reaped'])
            self.assertIn('raw evidence', (p / 'fail.stdout').read_text())
            with patch.object(subprocess, 'Popen') as popen:
                self.assertEqual(s.run('later', ['forbidden'], p)['status'], 'NOT_RUN')
                popen.assert_not_called()
            self.assertLess(b.remaining(m.FINAL_RESERVE), 1)

    def test_deadline_stops_python_toy_and_reaps_group(self):
        with tempfile.TemporaryDirectory() as d:
            p = Path(d)
            s = v.Supervisor(p, budget(0.12))
            row = s.run('timeout', [sys.executable, '-B', '-c', 'import time;time.sleep(5)'], p)
            self.assertEqual(row['status'], 'TIMEOUT')
            self.assertTrue(row['cleanup'][-1]['group_gone'])
            self.assertTrue(row['cleanup'][-1]['reaped'])

    def test_expired_budget_starts_nothing(self):
        with tempfile.TemporaryDirectory() as d:
            s = v.Supervisor(Path(d), budget(-1))
            with patch.object(subprocess, 'Popen') as popen:
                self.assertEqual(s.run('never', ['forbidden'], d)['status'], 'NOT_RUN')
                popen.assert_not_called()
        for bad in ({'monotonic': float('nan'), 'wall': time.time(), 'total_seconds': 900},
                    {'monotonic': time.monotonic(), 'wall': time.time(), 'total_seconds': 3600}):
            with self.assertRaises(v.Refused):
                v.Budget(bad)

    def test_full_package_and_edge_changes_are_disclosed(self):
        old = b'[[package]]\nname="root"\nversion="1"\ndependencies=["x 1"]\n[[package]]\nname="x"\nversion="1"\n'
        new = b'[[package]]\nname="root"\nversion="1"\ndependencies=["x 2", "tempfile"]\n[[package]]\nname="x"\nversion="2"\n[[package]]\nname="tempfile"\nversion="3"\n'
        result = m.lock_changes(old, new)
        self.assertEqual(len(result['packages_after']), 3)
        self.assertEqual([p['name'] for p in result['removed']], ['x'])
        self.assertEqual(result['version_or_source_changes'][0]['name'], 'x')
        self.assertEqual(len(result['edges_added']), 2)
        self.assertEqual(len(result['edges_removed']), 1)
        self.assertEqual(result['updated_same_identity'][0]['after']['dependencies'], ['x 2', 'tempfile'])

    def test_generated_lock_requires_official_checksum_and_tempfile_edge(self):
        raw = ('[[package]]\nname="newchan_rust"\nversion="0.1.0"\ndependencies=["tempfile"]\n'
               '[[package]]\nname="tempfile"\nversion="3.0.0"\nsource="' + m.REGISTRY + '"\nchecksum="' + 'ab' * 32 + '"\n').encode()
        m.validate_generated_lock(raw)
        for bad in (raw.replace(m.REGISTRY.encode(), b'git+https://other'), raw.replace(b'"tempfile"]', b'"other"]'),
                    raw.replace(b'ab' * 32, b'bad')):
            with self.assertRaises(v.Refused):
                m.validate_generated_lock(bad)

    def test_failed_resolution_keeps_lock_manifest_diff_and_registry(self):
        with tempfile.TemporaryDirectory() as d:
            p = Path(d)
            work, out = p / 'work', p / 'out'
            (work / 'rust').mkdir(parents=True)
            out.mkdir()
            raw = b'[[package]]\nname="old"\nversion="1"\n'
            (work / 'rust/Cargo.lock').write_bytes(raw)
            (out / 'Cargo.lock.old').write_bytes(raw)
            m.preserve_resolution(work, out, budget())
            self.assertEqual((out / 'Cargo.lock').read_bytes(), raw)
            self.assertEqual((out / 'Cargo.lock.patch').read_text(), '')
            self.assertEqual(json.loads((out / 'registry.json').read_text())['files'], [])
            m.save(out / 'result.json', {'status': 'FAILED'})
            m.save(out / 'steps.json', [{'name': 'generate-lockfile', 'status': 'FAILED'}])
            v.package(out, budget())
            self.assertIn('Cargo.lock', json.loads((out / 'FILES.json').read_text()))
            self.assertTrue((out / 'evidence.zip').exists())

    def test_main_resolves_once_in_temp_and_preserves_failure(self):
        for fail in (False, True):
            with self.subTest(fail=fail), tempfile.TemporaryDirectory() as d:
                p = Path(d).resolve()
                root, temp = p / 'checkout', p / 'temp'
                (root / 'rust/src').mkdir(parents=True)
                temp.mkdir()
                manifest = b'[package]\nname="newchan_rust"\nversion="0.1.0"\n[dev-dependencies]\ntempfile="3"\n'
                old = b'[[package]]\nname="newchan_rust"\nversion="0.1.0"\n'
                new = ('[[package]]\nname="newchan_rust"\nversion="0.1.0"\ndependencies=["tempfile"]\n'
                       '[[package]]\nname="tempfile"\nversion="3.0.0"\nsource="' + m.REGISTRY + '"\nchecksum="' + 'ab' * 32 + '"\n').encode()
                inputs = {'rust/Cargo.toml': manifest, 'rust/Cargo.lock': old, 'rust/src/lib.rs': b'// toy real bytes\n'}
                for path, raw in inputs.items():
                    (root / path).write_bytes(raw)
                (temp / 'issue1393-lock-start.json').write_text(json.dumps({'monotonic': time.monotonic(), 'wall': time.time(), 'total_seconds': 900}))
                def bind(_root, _contract, supervisor, _event):
                    tree = b''.join(('100644 blob ' + blob(raw) + '\t' + path + '\0').encode() for path, raw in inputs.items())
                    (supervisor.output / 'execution-ls-tree.stdout').write_bytes(tree)
                calls = []
                def generate(supervisor, _root, name, argv, cwd, env):
                    calls.append(argv)
                    self.assertEqual(name, 'generate-lockfile')
                    self.assertEqual(argv, m.resolution_argv(temp / 'issue1393-lock-work'))
                    self.assertFalse('GITHUB_TOKEN' in env)
                    (cwd / 'Cargo.lock').write_bytes(new)
                    (supervisor.output / (name + '.command.stdout')).write_bytes(b'toy generation raw\n')
                    supervisor.records.append({'name': name, 'status': 'FAILED' if fail else 'PASS', 'exit': 7 if fail else 0})
                    if fail:
                        raise v.Refused('toy 命令失败，不重跑')
                    return b''
                with patch.dict(os.environ, {'ISSUE1393_LOCK_EXECUTE': 'True', 'RUNNER_TEMP': str(temp)}, clear=True), \
                        patch.object(m, 'ROOT', root), patch.object(m, 'bind_source', side_effect=bind), \
                        patch.object(v, 'read_push_contract', return_value=(event(), contract())), \
                        patch.object(m, 'install_toolchain'), patch.object(v, 'telemetry_command', side_effect=generate), \
                        patch.object(m.shutil, 'disk_usage', return_value=SimpleNamespace(free=3 * 1024**3)), \
                        patch.object(m.signal, 'signal'), patch.object(subprocess, 'Popen') as spawn:
                    self.assertEqual(m.main(), 1 if fail else 0)
                    spawn.assert_not_called()
                self.assertEqual(len(calls), 1)
                for path, raw in inputs.items():
                    self.assertEqual((root / path).read_bytes(), raw)
                out = temp / 'issue1393-lock-evidence'
                self.assertEqual((out / 'Cargo.toml').read_bytes(), manifest)
                self.assertEqual((out / 'Cargo.lock.old').read_bytes(), old)
                self.assertEqual((out / 'Cargo.lock').read_bytes(), new)
                self.assertTrue((out / 'evidence.zip').exists())
                self.assertEqual(json.loads((out / 'result.json').read_text())['formal_validation'], 'NOT_RUN')

    def test_invalid_lock_still_keeps_raw_and_registry(self):
        with tempfile.TemporaryDirectory() as d:
            p = Path(d)
            (p / 'work/rust').mkdir(parents=True)
            (p / 'out').mkdir()
            (p / 'work/rust/Cargo.lock').write_bytes(b'invalid TOML!')
            (p / 'out/Cargo.lock.old').write_bytes(b'[[package]]\nname="old"\nversion="1"\n')
            with self.assertRaises(v.Refused):
                m.preserve_resolution(p / 'work', p / 'out', budget())
            self.assertEqual((p / 'out/Cargo.lock').read_bytes(), b'invalid TOML!')
            self.assertTrue((p / 'out/registry.json').exists())


if __name__ == '__main__':
    unittest.main()
