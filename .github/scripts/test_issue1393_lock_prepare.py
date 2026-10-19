#!/usr/bin/env python3
"""#1393 仅标准库/内存/临时 Python toy；不执行 Rust、网络或产品。"""
import copy
import contextlib
import hashlib
import importlib.util
import io
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
    c.update(source_commit='8e50d40933c2a6a25f882481f3c46aa885ce6407',
             source_tree='0a2e04a4ce22a6b8732ad3b55f57446ee3f066f0')
    c.update({k: 'ab' * 32 for k in v.HASH_FIELDS})
    c['helper_sha256'] = m.HELPER_SHA256
    c['manifest_sha256'] = m.MANIFEST_SHA256
    c['old_lock_sha256'] = m.OLD_LOCK_SHA256
    return provenance(c)


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


TOY_CAPS = {'single_bytes': 4 * 1024**2, 'total_bytes': 32 * 1024**2,
            'files': 512, 'items': 100000, 'upload_bytes': 32 * 1024**2}


class ToyCapsTest(unittest.TestCase):
    def setUp(self):
        self.enterContext(patch.dict(m.RESOURCE_CAPS, TOY_CAPS, clear=True))


def provenance(c):
    for kind in ('approval', 'review'):
        c[kind + '_text'] = json.dumps({'schema': 'issue1393-lock-preserve-' + kind + '/2',
                                       'scope_sha256': m.contract_scope(c), 'text': 'toy 原件，不授予运行许可'}, ensure_ascii=False)
        c[kind + '_sha256'] = m.digest(c[kind + '_text'].encode())
    return c


def cache_record(name, version, deps=None):
    return {'name': name, 'vers': version, 'deps': deps or [], 'cksum': 'ab' * 32,
            'features': {}, 'yanked': False, 'rust_version': '1.63'}


def write_cache(work, records):
    base = work / m.SPARSE_CACHE
    base.mkdir(parents=True, exist_ok=True)
    (base / 'config.json').write_text(json.dumps({'dl': 'https://static.crates.io/crates', 'api': 'https://crates.io'}))
    by_name = {}
    for r in records:
        by_name.setdefault(r['name'], []).append(r)
    for name, rows in by_name.items():
        p = base / '.cache' / m.sparse_path(name)
        p.parent.mkdir(parents=True, exist_ok=True)
        p.write_bytes(b'\x03\x02\0\0\0etag: "toy"\0' + b''.join(
            r['vers'].encode() + b'\0' + json.dumps(r).encode() + b'\0' for r in rows))


def lock(rows, root_deps=None):
    text = '[[package]]\nname="newchan_rust"\nversion="0.1.0"\ndependencies=' + json.dumps(root_deps or []) + '\n'
    for name, version, deps in rows:
        text += ('[[package]]\nname=' + json.dumps(name) + '\nversion=' + json.dumps(version)
                 + '\nsource=' + json.dumps(m.REGISTRY) + '\nchecksum="' + 'ab' * 32
                 + '"\ndependencies=' + json.dumps(deps) + '\n')
    return text.encode()


class ContractTests(ToyCapsTest):
    def test_false_zero_commands_and_no_event_read(self):
        with patch.dict(os.environ, {'ISSUE1393_LOCK_EXECUTE': 'False'}, clear=True), \
                patch.object(subprocess, 'Popen') as popen, patch.object(v, 'read_push_contract') as read:
            self.assertEqual(m.main(), 1)
            popen.assert_not_called()
            read.assert_not_called()

    def test_draft_is_rejected(self):
        for folder, name in ((m.REPORT, 'CONTRACT-DRAFT.json'), (m.LEGACY_REPORT, 'CONTRACT-DRAFT.json'), (m.LEGACY_REPORT, 'FIX-CONTRACT-DRAFT.json')):
            draft = json.loads((m.ROOT / (folder + name)).read_text())
            with self.subTest(draft=name), self.assertRaises(v.Refused):
                v.validate_contract('True', draft)
            with self.assertRaises(v.Refused):
                v.validate_contract('True', dict(draft, status='BOUND_FOR_ROOT_REVIEW'))

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
        receipt = json.loads(c['approval_text'])
        receipt['text'] = '$(touch /tmp/no) ; `whoami` ../x --build'
        c['approval_text'] = json.dumps(receipt)
        c['approval_sha256'] = m.digest(c['approval_text'].encode())
        v.parse_message(message(c))
        self.assertEqual(m.resolution_argv(Path('/fixed')), ['cargo', '+' + m.TOOLCHAIN, 'update', '--workspace', '--manifest-path', '/fixed/rust/Cargo.toml'])
        with tempfile.TemporaryDirectory() as d:
            s = v.Supervisor(Path(d), budget())
            with patch.object(subprocess, 'Popen') as popen:
                row = s.run('bad-stdin', ['git', 'diff-tree', '--stdin'], d, input_data=b'$(whoami)')
            self.assertEqual(row['status'], 'ERROR')
            popen.assert_not_called()

    def test_fixed_module_hash_checked_before_execution(self):
        with patch.object(m, 'HELPER_SHA256', 'ab' * 32), self.assertRaises(RuntimeError):
            m.load_fixed_helper()


class IdentityTests(ToyCapsTest):
    def test_complete_binding_and_mutations(self):
        with tempfile.TemporaryDirectory() as d:
            root = Path(d)
            c, head = contract(), 'cd' * 20
            false = b'name: toy\nenv:\n' + m.GATE_FALSE
            contents = {m.WORKFLOW: false.replace(m.GATE_FALSE, m.GATE_TRUE), m.RUNNER: b'runner',
                        '.github/workflows/ci.yml': b'ci', m.TELEMETRY: b'telemetry',
                        m.HELPER: (m.ROOT / m.HELPER).read_bytes(), 'rust/Cargo.toml': (m.ROOT / 'rust/Cargo.toml').read_bytes(),
                        'rust/Cargo.lock': (m.ROOT / 'rust/Cargo.lock').read_bytes(), '.github/workflows/issue1393-validation.yml': b'old workflow',
                        m.LEGACY_REPORT + 'RUST-TOOLCHAIN-39091453686.txt': (m.RUSTC_FIRST + '\n' + m.RUSTC_COMMIT + '\n' + m.CARGO_VERSION).encode()}
            for p, raw in contents.items():
                path = root / p
                path.parent.mkdir(parents=True, exist_ok=True)
                path.write_bytes(raw)
            mappings = {m.RUNNER: 'runner_sha256', '.github/workflows/ci.yml': 'ci_sha256', m.TELEMETRY: 'telemetry_sha256',
                        m.HELPER: 'helper_sha256', 'rust/Cargo.toml': 'manifest_sha256', 'rust/Cargo.lock': 'old_lock_sha256',
                        '.github/workflows/issue1393-validation.yml': 'validation_workflow_sha256',
                        m.LEGACY_REPORT + 'RUST-TOOLCHAIN-39091453686.txt': 'toolchain_evidence_sha256'}
            for p, k in mappings.items():
                c[k] = m.digest(contents[p])
            c['workflow_false_sha256'] = m.digest(false)
            legacy_tree = ('100644 blob ' + 'ab' * 20 + '\t' + m.LEGACY_REPORT + 'RUST-TOOLCHAIN-39091453686.txt\0').encode()
            tree = b'100644 blob ' + b'ab' * 20 + b'\trust/src/lib.rs\0' + legacy_tree
            (root / 'execution-ls-tree.stdout').write_bytes(tree)
            c['product_manifest_sha256'] = m.digest(v.canonical(v.product_entries(tree)))
            provenance(c)
            replies = {'legacy-ls-tree': legacy_tree, 'git-head': head.encode(), 'git-message': b'tree toy\n\n' + message(c).encode(),
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
                mutations = [('legacy-ls-tree', b''), ('legacy-ls-tree', legacy_tree.replace(b'abab', b'cdcd', 1)),
                             ('git-ancestry', b''),
                             ('git-ancestry', (c['source_commit'] + ' ' + '56' * 20).encode()),
                             ('git-prepared-tree', b'12' * 20),
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

                # 自报身份与 Git 回复一起伪换：旧通用绑定会接受，具名源必须先拒绝。
                for source, source_tree in ((c['prepared_commit'], c['prepared_tree']),
                                            ('12' * 20, '34' * 20),
                                            (c['source_commit'], '56' * 20)):
                    bad = dict(c, source_commit=source, source_tree=source_tree)
                    forged = dict(replies, **{'git-ancestry': (source + ' ' + source_tree).encode(),
                                              'git-message': b'tree toy\n\n' + message(bad).encode()})
                    with self.subTest(source=source, tree=source_tree), \
                            patch.object(v, 'git_read', side_effect=lambda _s, _r, n, *a: forged[n]) as read, \
                            patch.object(subprocess, 'Popen') as popen:
                        with self.assertRaises(v.Refused):
                            m.bind_source(root, bad, s, event(bad))
                        read.assert_not_called()
                        popen.assert_not_called()

                # E 可继续准备，但不能退成 S；正确声明仍要接受 Git 全树 identity 核验。
                with self.assertRaises(v.Refused), patch.object(v, 'git_read') as read:
                    m.bind_source(root, dict(c, prepared_commit=c['source_commit']), s, event(c))
                read.assert_not_called()

    def test_unique_gate(self):
        false = b'env:\n' + m.GATE_FALSE
        v.verify_activation(false, false.replace(m.GATE_FALSE, m.GATE_TRUE))
        for a, b in ((false, false), (false, false.replace(m.GATE_FALSE, m.GATE_TRUE) + b'x'),
                     (false + m.GATE_FALSE, false + m.GATE_TRUE)):
            with self.assertRaises(v.Refused):
                v.verify_activation(a, b)


class ResolutionTests(ToyCapsTest):
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
        old = lock([('x', '1.0.0', [])], ['x 1.0.0'])
        new = lock([('x', '2.0.0', []), ('tempfile', '3.0.0', [])], ['x 2.0.0', 'tempfile'])
        result = m.lock_changes(old, new)
        self.assertEqual(len(result['packages_after']), 3)
        self.assertEqual([p['name'] for p in result['removed']], ['x'])
        self.assertEqual(result['version_or_source_changes'][0]['name'], 'x')
        self.assertEqual(len(result['edges_added']), 2)
        self.assertEqual(len(result['edges_removed']), 1)
        self.assertEqual(result['updated_same_identity'][0]['after']['dependencies'], ['x 2.0.0', 'tempfile'])

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
            raw = lock([])
            (work / 'rust/Cargo.lock').write_bytes(raw)
            (out / 'Cargo.lock.old').write_bytes(raw)
            (out / 'Cargo.toml').write_bytes(b'[dev-dependencies]\ntempfile="3"\n')
            write_cache(work, [])
            m.preserve_resolution(work, out, budget())
            self.assertEqual((out / 'Cargo.lock').read_bytes(), raw)
            self.assertEqual((out / 'Cargo.lock.patch').read_text(), '')
            self.assertEqual(json.loads((out / 'registry.json').read_text())['files'], [])
            m.save(out / 'result.json', {'status': 'FAILED'})
            m.save(out / 'steps.json', [{'name': 'update-workspace', 'status': 'FAILED'}])
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
                    self.assertEqual(name, 'update-workspace')
                    self.assertEqual(argv, m.resolution_argv(temp / 'issue1393-lock-work'))
                    self.assertFalse('GITHUB_TOKEN' in env)
                    self.assertEqual((cwd / 'Cargo.lock').read_bytes(), old)
                    write_cache(cwd.parent, [cache_record('tempfile', '3.0.0')])
                    (cwd / 'Cargo.lock').write_bytes(new)
                    (supervisor.output / (name + '.command.stdout')).write_bytes(b'toy generation raw\n')
                    supervisor.records.append({'name': name, 'status': 'FAILED' if fail else 'PASS', 'exit': 7 if fail else 0})
                    if fail:
                        raise v.Refused('toy 命令失败，不重跑')
                    return b''
                with patch.dict(os.environ, {'ISSUE1393_LOCK_EXECUTE': 'True', 'RUNNER_TEMP': str(temp)}, clear=True), \
                        patch.object(m, 'ROOT', root), patch.object(m, 'MANIFEST_SHA256', m.digest(manifest)), \
                        patch.object(m, 'OLD_LOCK_SHA256', m.digest(old)), patch.object(m, 'bind_source', side_effect=bind), \
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
            self.assertTrue((p / 'out/registry-failure.json').exists())


class PreserveContractTests(ToyCapsTest):
    def test_new_contract_and_old_generation_rejected(self):
        c = contract()
        v.validate_contract('True', c)
        for k, value in (('schema', 'issue1393-lock-prepare/1'), ('command', 'GENERATE_LOCKFILE_ONLY'),
                         ('operation', 'GENERATE_LOCKFILE_ONLY'), ('ref', 'refs/heads/codex/1393-lock-once-20261018'),
                         ('old_lock_sha256', 'ce204e12ec8b8d25b41668fb3c466315a309252d873c6dbb89bdf64ba97f6ab6'),
                         ('manifest_sha256', 'cd' * 32), ('registry_network_fallback', 'ALLOW')):
            with self.subTest(field=k), self.assertRaises(v.Refused):
                v.validate_contract('True', provenance(dict(c, **{k: value})))
        for kind in ('approval', 'review'):
            bad = dict(c)
            bad[kind + '_text'] = '旧 generate 批准，即使重新计算摘要也不能使用'
            bad[kind + '_sha256'] = m.digest(bad[kind + '_text'].encode())
            with self.assertRaises(v.Refused):
                v.validate_contract('True', bad)
        for key in ('prepared_commit', 'prepared_tree', 'runner_sha256'):
            bad = dict(c, **{key: 'cd' * (20 if key.startswith('prepared_') else 32)})
            with self.assertRaises(v.Refused):
                v.validate_contract('True', bad)
        with self.assertRaises(v.Refused):
            v.parse_message(message(c).replace('LOCK-PRESERVE-CONTRACT', 'LOCK-CONTRACT'))

    def test_unbound_caps_fail_closed(self):
        with patch.dict(m.RESOURCE_CAPS, dict.fromkeys(TOY_CAPS, 'UNBOUND'), clear=True):
            c = contract()
            with patch.object(subprocess, 'Popen') as spawn, self.assertRaises(v.Refused):
                v.validate_contract('True', c)
            spawn.assert_not_called()
            with tempfile.TemporaryDirectory() as d, self.assertRaises(v.Refused):
                m.EvidenceIO(Path(d), budget())

    def test_original_source_hashes_are_actual(self):
        self.assertEqual(m.digest((m.ROOT / 'rust/Cargo.toml').read_bytes()), m.MANIFEST_SHA256)
        self.assertEqual(m.digest((m.ROOT / 'rust/Cargo.lock').read_bytes()), m.OLD_LOCK_SHA256)

    def test_ast_has_only_update_workspace_no_fallback_or_lock_delete(self):
        import ast
        tree = ast.parse((m.ROOT / m.RUNNER).read_text())
        self.assertEqual(m.resolution_argv(Path('/exclusive')),
                         ['cargo', '+' + m.TOOLCHAIN, 'update', '--workspace', '--manifest-path', '/exclusive/rust/Cargo.toml'])
        forbidden = {'generate-lockfile', 'metadata', 'build', 'check', 'test'}
        calls = [n for n in ast.walk(tree) if isinstance(n, ast.Call) and isinstance(n.func, ast.Attribute)
                 and n.func.attr == 'telemetry_command']
        self.assertEqual(len(calls), 4)
        for call in calls:
            # 固定安装/版本读取与唯一解析，无 second Cargo resolution。
            arg = call.args[3]
            if isinstance(arg, ast.List):
                words = [n.value for n in arg.elts if isinstance(n, ast.Constant)]
                self.assertFalse(forbidden.intersection(words))
        self.assertFalse(any(isinstance(n, ast.Attribute) and n.attr in ('unlink', 'rmtree', 'Popen') for n in ast.walk(tree)))


    def test_missing_or_changed_lock_never_calls_resolver(self):
        for mode in ('missing', 'changed'):
            with self.subTest(mode=mode), tempfile.TemporaryDirectory() as d:
                temp = Path(d).resolve()
                (temp / 'issue1393-lock-start.json').write_text(json.dumps({'monotonic': time.monotonic(), 'wall': time.time(), 'total_seconds': 900}))
                manifest = b'[dev-dependencies]\ntempfile="3"\n'
                old = lock([])
                def copy_source(_root, work, out, _budget):
                    (work / 'rust').mkdir()
                    (work / 'rust/Cargo.toml').write_bytes(manifest)
                    (work / 'rust/Cargo.lock').write_bytes(old)
                    (out / 'Cargo.lock.old').write_bytes(old)
                    (out / 'Cargo.toml').write_bytes(manifest)
                    write_cache(work, [])
                    return []
                def install(_root, work, _supervisor, _env):
                    path = work / 'rust/Cargo.lock'
                    if mode == 'missing':
                        path.unlink()
                    else:
                        path.write_bytes(old + b'\n# tampered')
                with contextlib.ExitStack() as stack:
                    for cm in (patch.dict(os.environ, {'ISSUE1393_LOCK_EXECUTE': 'True', 'RUNNER_TEMP': str(temp)}, clear=True),
                               patch.object(m, 'bind_source'), patch.object(v, 'read_push_contract', return_value=(event(), contract())),
                               patch.object(m, 'copy_resolution_source', side_effect=copy_source),
                               patch.object(m, 'install_toolchain', side_effect=install), patch.object(m.signal, 'signal'),
                               patch.object(m.shutil, 'disk_usage', return_value=SimpleNamespace(free=3 * 1024**3)),
                               patch.object(m, 'MANIFEST_SHA256', m.digest(manifest)), patch.object(m, 'OLD_LOCK_SHA256', m.digest(old))):
                        stack.enter_context(cm)
                    cargo = stack.enter_context(patch.object(v, 'telemetry_command'))
                    spawn = stack.enter_context(patch.object(subprocess, 'Popen'))
                    self.assertEqual(m.main(), 1)
                    cargo.assert_not_called()
                    spawn.assert_not_called()
                result = json.loads((temp / 'issue1393-lock-evidence/result.json').read_text())
                self.assertNotEqual(result['status'], 'PASS')
                self.assertEqual(result['resolution'], 'NOT_RUN')
                self.assertEqual((temp / 'issue1393-lock-evidence/Cargo.lock.old').read_bytes(), old)

    def test_wrong_input_lock_rejected_before_transport(self):
        with tempfile.TemporaryDirectory() as d:
            root = Path(d)
            work, out = root / 'work', root / 'out'
            out.mkdir()
            work.mkdir()
            manifest = (m.ROOT / 'rust/Cargo.toml').read_bytes()
            wrong = b'wrong generated lock'
            inputs = {'rust/Cargo.toml': manifest, 'rust/Cargo.lock': wrong}
            (root / 'rust').mkdir()
            for name, raw in inputs.items():
                (root / name).write_bytes(raw)
            (out / 'execution-ls-tree.stdout').write_bytes(b''.join(
                ('100644 blob ' + blob(raw) + '\t' + name + '\0').encode() for name, raw in inputs.items()))
            with patch.object(subprocess, 'Popen') as spawn, self.assertRaises(v.Refused):
                m.copy_resolution_source(root, work, out, budget())
            spawn.assert_not_called()
            self.assertEqual(list(work.iterdir()), [])

    def test_archive_upload_cap_blocks_packaging(self):
        with tempfile.TemporaryDirectory() as d:
            out = Path(d)
            m.save(out / 'result.json', {'status': 'FAILED'})
            m.save(out / 'steps.json', [])
            with patch.dict(m.RESOURCE_CAPS, upload_bytes=1), self.assertRaises(v.Refused):
                m.package(out, budget())
            self.assertFalse((out / 'evidence.zip').exists())


class RegistryTests(ToyCapsTest):
    def setUp(self):
        super().setUp()
        self.temp = self.enterContext(tempfile.TemporaryDirectory())
        self.base = Path(self.temp).resolve()
        self.work, self.out = self.base / 'work', self.base / 'out'
        (self.work / 'rust').mkdir(parents=True)
        self.out.mkdir()
        self.b = budget()
        self.old = lock([('parent', '1.0.0', ['child']), ('child', '1.0.0', [])], ['parent'])
        self.new = lock([('parent', '1.0.0', ['child']), ('child', '2.0.0', []), ('tempfile', '3.0.0', [])], ['parent', 'tempfile'])
        (self.out / 'Cargo.toml').write_bytes(b'[dependencies]\nparent="1"\n[dev-dependencies]\ntempfile="3"\n')
        (self.out / 'Cargo.lock.old').write_bytes(self.old)
        (self.work / 'rust/Cargo.lock').write_bytes(self.new)
        dep = {'name': 'alias', 'package': 'child', 'req': '>=1, <3', 'features': ['fast'],
               'optional': True, 'default_features': False, 'target': 'cfg(unix)', 'kind': 'normal', 'registry': None}
        self.records = [cache_record('parent', '1.0.0', [dep]), cache_record('child', '1.0.0'),
                        cache_record('child', '2.0.0'), cache_record('tempfile', '3.0.0')]
        write_cache(self.work, self.records)
        self.spawn = self.enterContext(patch.object(subprocess, 'Popen', side_effect=AssertionError('不准真实 Cargo/网络子进程')))

    def tearDown(self):
        self.spawn.assert_not_called()
        super().tearDown()

    def preserve(self):
        return m.preserve_resolution(self.work, self.out, self.b)

    def test_complete_records_parents_features_and_retargeted_text_edge(self):
        self.preserve()
        changes = json.loads((self.out / 'lock-changes.json').read_text())
        self.assertEqual(changes['text_unchanged_target_changed'], [
            {'parent': ['parent', '1.0.0', m.REGISTRY], 'dependency': 'child',
             'before': ['child', '1.0.0', m.REGISTRY], 'after': ['child', '2.0.0', m.REGISTRY]}])
        self.assertFalse(any(e[-1] == 'child' for e in changes['edges_added'] + changes['edges_removed']))
        evidence = json.loads((self.out / 'registry.json').read_text())
        self.assertTrue(evidence['complete'])
        self.assertEqual(len(evidence['records']), 4)
        files = {f['evidence_artifact']: f for f in evidence['files']}
        for row in evidence['records']:
            extract = (self.out / row['evidence_artifact']).read_bytes()
            raw = extract[row['evidence_offset']:row['evidence_offset'] + row['raw_json_bytes']]
            self.assertEqual(m.digest(raw), row['raw_json_sha256'])
            self.assertEqual(extract[row['evidence_offset'] + row['raw_json_bytes']], 0)
            parsed = json.loads(raw)
            self.assertEqual([parsed['name'], parsed['vers']], row['identity'][:2])
            self.assertTrue(row['raw_complete'])
            cache = (self.work / row['cache_path']).read_bytes()
            self.assertEqual(cache[row['cache_offset']:row['cache_offset'] + row['raw_json_bytes']], raw)
            self.assertEqual(m.digest(cache), files[row['evidence_artifact']]['sha256'])
            self.assertEqual(m.digest(extract), files[row['evidence_artifact']]['evidence_sha256'])
            self.assertEqual(m.sparse_records(extract, parsed['name'])[parsed['vers']][1], raw)
        parents = [e for e in evidence['constraint_parents'] if e['target'][0] == 'child']
        self.assertEqual({e['graph'] for e in parents}, {'before', 'after'})
        self.assertTrue(all(e['constraints'][0]['req'] == '>=1, <3' and e['constraints'][0]['features'] == ['fast'] for e in parents))
        roots = [e for e in evidence['constraint_parents'] if e['target'][0] == 'tempfile']
        self.assertEqual(roots[0]['constraints'][0]['declaration']['version'], '3')
        self.assertEqual(evidence['semver_solution'], 'NOT_SOLVED_PENDING_INDEPENDENT_REVIEW')

    def test_null_package_alias_is_normal_dependency(self):
        self.records[0]['deps'][0].update(name='child', package=None)
        write_cache(self.work, self.records)
        self.preserve()
        self.assertTrue(json.loads((self.out / 'registry.json').read_text())['complete'])

    def test_missing_old_version_is_proof_gap(self):
        write_cache(self.work, [r for r in self.records if not (r['name'] == 'child' and r['vers'] == '1.0.0')])
        with self.assertRaises(v.Refused):
            self.preserve()
        proof = json.loads((self.out / 'registry.json').read_text())
        self.assertFalse(proof['complete'])
        self.assertTrue(any(g.get('identity') == ['child', '1.0.0', m.REGISTRY] for g in proof['proof_gap']))
        self.assertEqual((self.out / 'Cargo.lock').read_bytes(), self.new)

    def test_missing_cache_and_no_scan_or_network(self):
        path = self.work / m.SPARSE_CACHE / '.cache' / m.sparse_path('child')
        path.unlink()
        with patch.object(Path, 'rglob', side_effect=AssertionError('不准扫描 registry')), self.assertRaises(v.Refused):
            self.preserve()
        proof = json.loads((self.out / 'registry.json').read_text())
        self.assertFalse(proof['complete'])
        self.assertTrue(any(g.get('name') == 'child' for g in proof['proof_gap']))

    def test_unknown_format_source_checksum_and_missing_parent_constraint(self):
        variants = [('deps', []), ('cksum', 'cd' * 32), ('deps', [dict(self.records[0]['deps'][0], registry='https://other')])]
        for key, value in variants:
            with self.subTest(key=key, value=value):
                for p in self.out.iterdir():
                    if p.name not in ('Cargo.toml', 'Cargo.lock.old'):
                        p.unlink()
                modified = copy.deepcopy(self.records)
                modified[0][key] = value
                write_cache(self.work, modified)
                with self.assertRaises(v.Refused):
                    self.preserve()
                self.assertFalse(json.loads((self.out / 'registry.json').read_text())['complete'])
        path = self.work / m.SPARSE_CACHE / '.cache' / m.sparse_path('child')
        with self.assertRaises(v.Refused):
            m.sparse_records(path.read_bytes().replace(b'\x03\x02', b'\x04\x02', 1), 'child')

    def test_registry_duplicate_versions_and_truncated_json(self):
        raw = (self.work / m.SPARSE_CACHE / '.cache' / m.sparse_path('child')).read_bytes()
        for bad in (raw[:-1], raw + raw[raw.index(b'1.0.0\0'):], raw.replace(b'"vers":', b'"vers":"x","vers":', 1)):
            with self.assertRaises(v.Refused):
                m.sparse_records(bad, 'child')

    def test_graph_rejects_ambiguity_dangling_and_unknown_source(self):
        rows = [('child', '1.0.0', []), ('child', '2.0.0', [])]
        for raw in (lock(rows, ['child']), lock(rows, ['missing']),
                    lock(rows, ['child 1.0.0 (git+https://other)']),
                    lock(rows, ['child 1.0.0']).replace(m.REGISTRY.encode(), b'git+https://other')):
            with self.assertRaises(v.Refused):
                m.lock_changes(raw, raw)
        for text in ('child 1.0.0', 'child 1.0.0 (' + m.REGISTRY + ')'):
            raw = lock(rows, [text])
            self.assertEqual(m.lock_changes(raw, raw)['resolved_edges_before'][0]['target'][1], '1.0.0')

    def test_deleted_package_requires_old_record_and_parent(self):
        (self.work / 'rust/Cargo.lock').write_bytes(lock([('tempfile', '3.0.0', [])], ['tempfile']))
        self.preserve()
        proof = json.loads((self.out / 'registry.json').read_text())
        self.assertIn(['child', '1.0.0', m.REGISTRY], [r['identity'] for r in proof['records']])
        self.assertTrue(any(e['graph'] == 'before' and e['target'][0] == 'parent' for e in proof['constraint_parents']))

    def test_caps_preflight_before_read_and_registry_total_files_items(self):
        io_guard = m.EvidenceIO(self.out, self.b)
        path = self.work / 'large.raw'
        path.write_bytes(b'x' * 600)
        with patch.object(Path, 'open') as opened, self.assertRaises(v.Refused):
            io_guard.read(path, self.work, 512)
        opened.assert_not_called()
        self.assertEqual(io_guard.read(path, self.work, 600), b'x' * 600)
        state = io_guard.check()
        with patch.dict(m.RESOURCE_CAPS, total_bytes=state['registry_bytes'] + 10), \
                patch.object(Path, 'open') as opened, self.assertRaises(v.Refused):
            io_guard.write('registry-x.raw', b'y' * 11)
        opened.assert_not_called()
        with patch.dict(m.RESOURCE_CAPS, total_bytes=state['registry_bytes'] + 10):
            io_guard.write('registry-x.raw', b'y' * 10)
        with patch.dict(m.RESOURCE_CAPS, items=1):
            io_guard.take_items(1)
            with self.assertRaises(v.Refused):
                io_guard.take_items(1)
        registry_files = io_guard.check()['registry_files']
        with patch.dict(m.RESOURCE_CAPS, files=registry_files), self.assertRaises(v.Refused):
            io_guard.write('registry-y.raw', b'z')
        with patch.dict(m.RESOURCE_CAPS, files=registry_files - 1), patch.object(Path, 'open') as opened, \
                self.assertRaises(v.Refused):
            m.EvidenceIO(self.out, self.b)
        opened.assert_not_called()

    def test_non_registry_originals_not_counted_in_registry_caps(self):
        guard = m.EvidenceIO(self.out, self.b)
        with patch.dict(m.RESOURCE_CAPS, single_bytes=16, total_bytes=16, files=1):
            guard.write('toolchain-log.stdout', b'L' * 4096)
            state = guard.check()
        self.assertEqual(state['registry_files'], 0)
        self.assertEqual(state['registry_bytes'], 0)

    def test_registry_read_parse_hash_write_expiry_and_exception(self):
        for stage in ('read', 'parse', 'hash', 'write'):
            for fault in ('deadline', 'exception'):
                with self.subTest(stage=stage, fault=fault):
                    for p in self.out.iterdir():
                        if p.name not in ('Cargo.toml', 'Cargo.lock.old'):
                            p.unlink()
                    b = budget()
                    changes = m.lock_changes(self.old, self.new)
                    guard = m.EvidenceIO(self.out, b)
                    fn = {'read': guard.read, 'parse': m.scan_sparse_cache, 'hash': m.digest, 'write': guard.write}[stage]
                    target, attr = {'read': (guard, 'read'), 'parse': (m, 'scan_sparse_cache'),
                                    'hash': (m, 'digest'), 'write': (guard, 'write')}[stage]
                    fired = []
                    def corrupt(*args, **kwargs):
                        val = fn(*args, **kwargs)
                        if not fired:
                            fired.append(True)
                            if fault == 'exception':
                                raise OSError('toy 取证失败')
                            b.mono -= 1000
                        return val
                    with patch.object(target, attr, side_effect=corrupt), self.assertRaises((v.Refused, OSError)):
                        m.snapshot_registry(self.work, self.out, b, changes, guard)
                    self.assertTrue(fired)
                    self.assertFalse(json.loads((self.out / 'registry-failure.json').read_text())['complete'])
                    if (self.out / 'registry.json').exists():
                        self.assertFalse(json.loads((self.out / 'registry.json').read_text())['complete'])

    def test_symlink_and_summary_write_expiry_never_complete(self):
        path = self.work / m.SPARSE_CACHE / '.cache' / m.sparse_path('child')
        raw = path.read_bytes()
        target = self.work / 'foreign'
        target.write_bytes(raw)
        path.unlink()
        path.symlink_to(target)
        with self.assertRaises(v.Refused):
            self.preserve()
        self.assertFalse(json.loads((self.out / 'registry.json').read_text())['complete'])
        path.unlink()
        path.write_bytes(raw)
        for p in self.out.iterdir():
            if p.name not in ('Cargo.toml', 'Cargo.lock.old'):
                p.unlink()
        b = budget()
        guard = m.EvidenceIO(self.out, b)
        real_json = guard.json
        def expire(name, value):
            real_json(name, value)
            if name == 'registry.json':
                b.wall -= 1000
        with patch.object(guard, 'json', side_effect=expire), self.assertRaises(v.Refused):
            m.snapshot_registry(self.work, self.out, b, m.lock_changes(self.old, self.new), guard)
        self.assertFalse(json.loads((self.out / 'registry.json').read_text())['complete'])


class PackageTailTests(ToyCapsTest):
    STAGES = ('zip-open', 'zip-read', 'zip-eof', 'zip-close', 'zip-update',
              'zip-hexdigest', 'receipt-before', 'receipt-after', 'package-return',
              'print-serialize', 'print-after')

    def run_tail(self, stage=None, fault=None):
        """真实 main/Budget/Supervisor/ZIP，替换外部执行；只在收尾边界推进时钟/发信号。"""
        with tempfile.TemporaryDirectory() as d:
            temp = Path(d).resolve()
            clocks = {'monotonic': 1000.0, 'wall': 10000.0}
            start = dict(clocks, total_seconds=900)
            (temp / 'issue1393-lock-start.json').write_text(json.dumps(start))
            original = b'toy generation raw\n'
            raw = ('[[package]]\nname="newchan_rust"\nversion="0.1.0"\ndependencies=["tempfile"]\n'
                   '[[package]]\nname="tempfile"\nversion="3.0.0"\nsource="' + m.REGISTRY
                   + '"\nchecksum="' + 'ab' * 32 + '"\n').encode()
            out = temp / 'issue1393-lock-evidence'
            fired, final_digest, read_sizes, supervisors = [], [False], [], []
            real_open, real_sha, real_save = Path.open, hashlib.sha256, m.save
            real_package, real_supervisor, real_dumps = v.package, v.Supervisor, json.dumps

            def trip(at):
                if stage == at and not fired:
                    fired.append(at)
                    if fault == 'signal':
                        m.signal.raise_signal(m.signal.SIGTERM)
                    else:
                        clocks[fault] += 901

            class ZipReader(io.BytesIO):
                def read(self, size=-1):
                    read_sizes.append(size)
                    value = super().read(size)
                    trip('zip-read' if value else 'zip-eof')
                    return value

                def __exit__(self, *args):
                    result = super().__exit__(*args)
                    trip('zip-close')
                    return result

            def open_file(path, mode='r', *args, **kwargs):
                if path == out / 'evidence.zip' and mode == 'rb':
                    with real_open(path, mode, *args, **kwargs) as f:
                        reader = ZipReader(f.read())
                    final_digest[0] = True
                    trip('zip-open')
                    return reader
                return real_open(path, mode, *args, **kwargs)

            class Hash:
                def __init__(self, *args, **kwargs):
                    self.value = real_sha(*args, **kwargs)

                def update(self, data):
                    self.value.update(data)
                    if final_digest[0]:
                        trip('zip-update')

                def hexdigest(self):
                    value = self.value.hexdigest()
                    if final_digest[0]:
                        trip('zip-hexdigest')
                        final_digest[0] = False
                    return value

            def save_receipt(path, value):
                success = path.name == 'PACKAGE.json' and value.get('status') == 'PACKAGED'
                if success:
                    trip('receipt-before')
                real_save(path, value)
                if success:
                    trip('receipt-after')

            def finish_package(*args, **kwargs):
                real_package(*args, **kwargs)
                trip('package-return')

            def serialize(value, *args, **kwargs):
                text = real_dumps(value, *args, **kwargs)
                if isinstance(value, dict) and value.get('resolution') == 'UPDATED_PENDING_ROOT_REVIEW' \
                        and value.get('status') == 'PASS' and 'sort_keys' not in kwargs:
                    trip('print-serialize')
                return text

            class Output(io.StringIO):
                def write(self, text):
                    count = super().write(text)
                    if '"status": "PASS"' in text:
                        trip('print-after')
                    return count

            def supervise(*args):
                s = real_supervisor(*args)
                supervisors.append(s)
                return s

            def copy_source(_root, work, output, b):
                self.assertIs(b, supervisors[0].budget)
                (work / 'rust').mkdir()
                (work / 'rust/Cargo.lock').write_bytes(raw)
                (output / 'Cargo.lock.old').write_bytes(raw)
                manifest = b'[dev-dependencies]\ntempfile="3"\n'
                (output / 'Cargo.toml').write_bytes(manifest)
                (work / 'rust/Cargo.toml').write_bytes(manifest)
                write_cache(work, [])
                (output / 'original.stdout').write_bytes(original)
                return []

            previous = {sig: m.signal.getsignal(sig) for sig in (m.signal.SIGTERM, m.signal.SIGINT)}
            captured = Output()
            try:
                with contextlib.ExitStack() as stack:
                    for cm in (
                        patch.dict(os.environ, {'ISSUE1393_LOCK_EXECUTE': 'True', 'RUNNER_TEMP': str(temp)}, clear=True),
                        patch.object(m.time, 'monotonic', side_effect=lambda: clocks['monotonic']),
                        patch.object(m.time, 'time', side_effect=lambda: clocks['wall']),
                        patch.object(m, 'bind_source'),
                        patch.object(m, 'OLD_LOCK_SHA256', m.digest(raw)),
                        patch.object(m, 'MANIFEST_SHA256', m.digest(b'[dev-dependencies]\ntempfile="3"\n')),
                        patch.object(v, 'read_push_contract', return_value=(event(), contract())),
                        patch.object(v, 'Supervisor', side_effect=supervise),
                        patch.object(m, 'copy_resolution_source', side_effect=copy_source),
                        patch.object(m, 'isolated_environment', return_value={}),
                        patch.object(m, 'install_toolchain'), patch.object(v, 'telemetry_command', return_value=b''),
                        patch.object(m.shutil, 'disk_usage', return_value=SimpleNamespace(free=3 * 1024**3)),
                        patch.object(Path, 'open', open_file), patch.object(m.hashlib, 'sha256', Hash),
                        patch.object(m, 'save', side_effect=save_receipt),
                        patch.object(v, 'package', side_effect=finish_package),
                        patch.object(m.json, 'dumps', side_effect=serialize),
                        contextlib.redirect_stdout(captured),
                    ):
                        stack.enter_context(cm)
                    spawn = stack.enter_context(patch.object(subprocess, 'Popen'))
                    code = m.main()
                    spawn.assert_not_called()
            finally:
                for sig, handler in previous.items():
                    m.signal.signal(sig, handler)
            result = json.loads((out / 'result.json').read_text())
            receipt = json.loads((out / 'PACKAGE.json').read_text())
            printed = json.loads(captured.getvalue().splitlines()[-1])
            self.assertEqual(printed, result)
            self.assertEqual((out / 'original.stdout').read_bytes(), original)
            self.assertEqual((out / 'Cargo.lock').read_bytes(), raw)
            self.assertTrue((out / 'evidence.zip').is_file())
            self.assertIsInstance(supervisors[0].budget, v.Budget)
            self.assertEqual((supervisors[0].budget.mono, supervisors[0].budget.wall), (1000.0, 10000.0))
            if fault:
                self.assertEqual(fired, [stage])
                self.assertEqual(code, 1)
                self.assertEqual(result['status'], 'FAILED')
                self.assertEqual(receipt['status'], 'FAILED')
                self.assertNotIn('sha256', receipt)
                if fault == 'signal':
                    self.assertEqual(result['received_signal'], m.signal.SIGTERM)
                # ZIP 保留初始快照；散件和最后输出才是最终结果，不能回写包伪装成功。
                with v.zipfile.ZipFile(out / 'evidence.zip') as archive:
                    self.assertEqual(json.loads(archive.read('result.json'))['status'], 'PASS')
            else:
                self.assertEqual(code, 0)
                self.assertEqual(result['status'], 'PASS')
                self.assertEqual(receipt['status'], 'PACKAGED')
                self.assertEqual(receipt['sha256'], real_sha((out / 'evidence.zip').read_bytes()).hexdigest())
                self.assertTrue(read_sizes)
            self.assertTrue(all(0 < n <= 1024**2 for n in read_sizes))

    def test_main_tail_monotonic_deadline(self):
        for stage in self.STAGES:
            with self.subTest(stage=stage):
                self.run_tail(stage, 'monotonic')

    def test_main_tail_wall_deadline(self):
        for stage in self.STAGES:
            with self.subTest(stage=stage):
                self.run_tail(stage, 'wall')

    def test_main_tail_sigterm(self):
        for stage in self.STAGES:
            with self.subTest(stage=stage):
                self.run_tail(stage, 'signal')

    def test_main_tail_within_window(self):
        self.run_tail()

    def test_digest_streams_multiple_chunks_and_refuses_expired_before_io(self):
        raw = b'x' * (2 * 1024**2 + 17)
        sizes = []

        class Reader(io.BytesIO):
            def read(self, size=-1):
                sizes.append(size)
                return super().read(size)

        b = budget()
        with patch.object(Path, 'open', return_value=Reader(raw)), patch.object(subprocess, 'Popen') as spawn:
            self.assertEqual(m.file_digest(Path('memory-only'), b), hashlib.sha256(raw).hexdigest())
            spawn.assert_not_called()
        self.assertEqual(sizes, [1024**2] * 4)
        with patch.object(Path, 'open') as opened, patch.object(m.hashlib, 'sha256') as hashed:
            with self.assertRaises(v.Refused):
                m.file_digest(Path('memory-only'), budget(-m.FINAL_RESERVE - 1))
            opened.assert_not_called()
            hashed.assert_not_called()


ROOT_CAPS_LITERAL = {'single_bytes': 65536, 'total_bytes': 2097152, 'files': 128, 'items': 512,
                     'upload_bytes': 33554432}


class NewCapsContractTests(ToyCapsTest):
    def test_real_caps_are_root_named_values_and_preserved_limits(self):
        with patch.dict(m.RESOURCE_CAPS, ROOT_CAPS_LITERAL, clear=True):
            pass
        source = (m.ROOT / m.RUNNER).read_text()
        import ast
        tree = ast.parse(source)
        literal = next(ast.literal_eval(n.value) for n in tree.body if isinstance(n, ast.Assign)
                       and any(isinstance(t, ast.Name) and t.id == 'RESOURCE_CAPS' for t in n.targets))
        self.assertEqual(literal, ROOT_CAPS_LITERAL)
        caps = m.FIXED['resource_caps']
        reg = caps['registry_evidence']
        self.assertEqual({k: reg[k] for k in ('single_bytes', 'total_bytes', 'files', 'items')},
                         {k: ROOT_CAPS_LITERAL[k] for k in ('single_bytes', 'total_bytes', 'files', 'items')})
        self.assertIs(reg['additional_network_for_evidence'], False)
        self.assertEqual(reg['mode'], 'EXISTING_EXCLUSIVE_CARGO_CACHE_ONLY')
        self.assertEqual(caps['upload_bytes'], 33554432)
        self.assertEqual(caps['upload_action'], 'actions/upload-artifact@ea165f8d65b6e75b540449e92b4886f43607fa02')
        self.assertEqual(caps['source'], 'ROOT_NEW_NARROWER_CAPS_20261018')
        kept = caps['existing_preserved']
        self.assertEqual(kept, {'MAX_MESSAGE_BYTES': v.MAX_MESSAGE_BYTES, 'MAX_EVENT_BYTES': v.MAX_EVENT_BYTES,
                                'total_seconds': m.TOTAL_SECONDS, 'final_reserve_seconds': m.FINAL_RESERVE,
                                'jobs': m.FIXED['jobs'], 'telemetry_seconds': m.FIXED['telemetry_seconds'],
                                'initial_disk_bytes': m.FIXED['initial_disk_bytes']})
        self.assertEqual((v.MAX_MESSAGE_BYTES, v.MAX_EVENT_BYTES, m.TOTAL_SECONDS, m.FINAL_RESERVE),
                         (32768, 1048576, 900, 90))
        self.assertEqual((m.FIXED['jobs'], m.FIXED['telemetry_seconds'], m.FIXED['initial_disk_bytes']),
                         (1, 10, 2147483648))
        self.assertEqual(m.FIXED['schema'], 'issue1393-lock-preserve/2')
        self.assertIn('本次新定的更窄上限（不是「既有帽」）', source)

    def test_caps_match_in_repo_root_original(self):
        # 根原件副本随源入仓并钉摘要；字面量或合同任一与根原件分叉即红（CI 可见，不依赖外盘）。
        path = m.ROOT / m.REPORT / 'ROOT-REGISTRY-CAPS-20261018.json'
        raw = path.read_bytes()
        self.assertEqual(m.digest(raw), 'd4c6e15423f79acbecc2d52dd5aaacb33d5e5cd96c65d7b0f149ab785025600e')
        root = json.loads(raw)
        reg = root['registry_evidence']
        import ast
        tree = ast.parse((m.ROOT / m.RUNNER).read_text())
        literal = next(ast.literal_eval(n.value) for n in tree.body if isinstance(n, ast.Assign)
                       and any(isinstance(t, ast.Name) and t.id == 'RESOURCE_CAPS' for t in n.targets))
        expected = {k: reg[k] for k in ('single_bytes', 'total_bytes', 'files', 'items')}
        expected['upload_bytes'] = root['upload_bytes']
        self.assertEqual(literal, expected)
        caps = m.FIXED['resource_caps']
        for k in ('single_bytes', 'total_bytes', 'files', 'items', 'mode', 'additional_network_for_evidence'):
            self.assertTrue(m.strict_equal(caps['registry_evidence'][k], reg[k]), k)
        self.assertTrue(m.strict_equal(caps['upload_bytes'], root['upload_bytes']))
        self.assertEqual(caps['upload_counting'], root['upload_counting'])
        self.assertTrue(m.strict_equal(caps['existing_preserved'], root['existing_preserved']))
        draft = json.loads((m.ROOT / m.REPORT / 'CONTRACT-DRAFT.json').read_text())
        self.assertTrue(m.strict_equal(draft['resource_caps'], caps))

    def test_wrong_caps_values_types_unknown_fields_and_old_schema_rejected(self):
        c = contract()
        v.validate_contract('True', c)
        def mutated(fn):
            bad = copy.deepcopy(c)
            fn(bad['resource_caps'])
            return provenance(bad)
        cases = {
            'single+1': lambda x: x['registry_evidence'].update(single_bytes=65537),
            'single float': lambda x: x['registry_evidence'].update(single_bytes=65536.0),
            'total-1': lambda x: x['registry_evidence'].update(total_bytes=2097151),
            'files bool-ish': lambda x: x['registry_evidence'].update(files=True),
            'items': lambda x: x['registry_evidence'].update(items=513),
            'network 0': lambda x: x['registry_evidence'].update(additional_network_for_evidence=0),
            'network true': lambda x: x['registry_evidence'].update(additional_network_for_evidence=True),
            'mode': lambda x: x['registry_evidence'].update(mode='CACHE_OR_NETWORK'),
            'unknown nested': lambda x: x['registry_evidence'].update(extra=1),
            'missing nested': lambda x: x['registry_evidence'].pop('items'),
            'upload': lambda x: x.update(upload_bytes=33554433),
            'upload counting': lambda x: x.update(upload_counting='INNER_ZIP_ONLY'),
            'mutable action': lambda x: x.update(upload_action='actions/upload-artifact@v4'),
            'compression': lambda x: x.update(compression_level=6),
            'message cap': lambda x: x['existing_preserved'].update(MAX_MESSAGE_BYTES=65536),
            'disk as output': lambda x: x['existing_preserved'].update(initial_disk_bytes=1),
            'unknown top': lambda x: x.update(old_caps='UNBOUND'),
        }
        for label, fn in cases.items():
            with self.subTest(case=label), self.assertRaises(v.Refused):
                v.validate_contract('True', mutated(fn))
        for key, value in (('schema', 'issue1393-lock-preserve/1'), ('resource_caps', {k: 'UNBOUND' for k in TOY_CAPS}),
                           ('registry_evidence', 'EXCLUSIVE_SPARSE_CACHE_ONLY_MISSING_IS_PROOF_GAP'),
                           ('jobs', True), ('total_seconds', 900.0)):
            with self.subTest(field=key), self.assertRaises(v.Refused):
                v.validate_contract('True', provenance(dict(copy.deepcopy(c), **{key: value})))
        old = copy.deepcopy(c)
        old['approval_text'] = json.dumps({'schema': 'issue1393-lock-preserve-approval/1',
                                           'scope_sha256': m.contract_scope(old), 'text': '旧 schema 批准'})
        old['approval_sha256'] = m.digest(old['approval_text'].encode())
        with self.assertRaises(v.Refused):
            v.validate_contract('True', old)
        extra = dict(copy.deepcopy(c), unknown_field='x')
        with self.assertRaises(v.Refused):
            v.validate_contract('True', provenance(extra))

    def test_runner_has_no_network_or_http_fetch_path(self):
        import ast
        tree = ast.parse((m.ROOT / m.RUNNER).read_text())
        imported = {a.name.split('.')[0] for n in ast.walk(tree) if isinstance(n, (ast.Import, ast.ImportFrom))
                    for a in getattr(n, 'names', [])} | {n.module.split('.')[0] for n in ast.walk(tree)
                                                          if isinstance(n, ast.ImportFrom) and n.module}
        self.assertFalse(imported & {'socket', 'urllib', 'http', 'requests', 'ssl', 'ftplib', 'subprocess'})
        scan = next(n for n in tree.body if isinstance(n, ast.FunctionDef) and n.name == 'scan_sparse_cache')
        names = {n.attr for n in ast.walk(scan) if isinstance(n, ast.Attribute)} | {
            n.id for n in ast.walk(scan) if isinstance(n, ast.Name)}
        self.assertFalse(names & {'telemetry_command', 'Popen', 'urlopen', 'read_bytes', 'read_text', 'loads'})


class NewCapsRegistryTests(RegistryTests):
    def write_raw_cache(self, name, entries, header=b'etag: "toy"'):
        p = self.work / m.SPARSE_CACHE / '.cache' / m.sparse_path(name)
        p.write_bytes(b'\x03\x02\0\0\0' + header + b'\0' + b''.join(
            ver.encode() + b'\0' + raw + b'\0' for ver, raw in entries))
        return p

    def padded(self, record, size):
        raw = json.dumps(record).encode()
        record = dict(record, pad='')
        base = len(json.dumps(record).encode())
        self.assertLessEqual(base, size)
        return json.dumps(dict(record, pad='p' * (size - base))).encode()

    def child_entries(self, size):
        return [('1.0.0', self.padded(cache_record('child', '1.0.0'), size)),
                ('2.0.0', json.dumps(cache_record('child', '2.0.0')).encode())]

    def test_single_record_equal_cap_complete_and_over_cap_proof_gap_untruncated(self):
        for size, complete in ((700, True), (701, False)):
            with self.subTest(size=size):
                for p in self.out.iterdir():
                    if p.name not in ('Cargo.toml', 'Cargo.lock.old'):
                        p.unlink()
                self.write_raw_cache('child', self.child_entries(size))
                with patch.dict(m.RESOURCE_CAPS, single_bytes=700):
                    if complete:
                        self.preserve()
                    else:
                        with self.assertRaises(v.Refused):
                            self.preserve()
                proof = json.loads((self.out / 'registry.json').read_text())
                self.assertIs(proof['complete'], complete)
                extract = b''.join(p.read_bytes() for p in self.out.glob('registry-cache-*.raw'))
                if not complete:
                    self.assertTrue(any(g.get('identity') == ['child', '1.0.0', m.REGISTRY]
                                        and 'single_bytes' in g['reason'] for g in proof['proof_gap']))
                    self.assertNotIn(b'"pad": "ppp', extract)

    def test_unwanted_large_versions_skipped_and_not_counted(self):
        huge = json.dumps(dict(cache_record('child', '0.9.0'), pad='q' * 15000)).encode()
        entries = [('0.9.0', huge)] + [(ver, json.dumps(cache_record('child', ver)).encode()) for ver in ('1.0.0', '2.0.0')]
        cache = self.write_raw_cache('child', entries)
        with patch.dict(m.RESOURCE_CAPS, single_bytes=1024, total_bytes=12000), patch.object(m, 'SCAN_CHUNK_BYTES', 7):
            self.preserve()
        proof = json.loads((self.out / 'registry.json').read_text())
        self.assertTrue(proof['complete'])
        child = next(f for f in proof['files'] if f['name'] == 'child')
        self.assertEqual(child['bytes'], cache.stat().st_size)
        self.assertGreater(child['bytes'], 12000)
        self.assertEqual(child['entries_scanned'], 3)
        self.assertNotIn(b'qqqq', (self.out / child['evidence_artifact']).read_bytes())
        self.assertIn('NOT_ARCHIVED', child['scope'])
        registry_bytes = sum(p.stat().st_size for p in self.out.iterdir() if m.is_registry_evidence(p.name))
        self.assertLessEqual(registry_bytes, 12000)

    def test_registry_total_files_items_equal_and_over(self):
        def clean():
            for p in self.out.iterdir():
                if p.name not in ('Cargo.toml', 'Cargo.lock.old'):
                    p.unlink()
        def measure(total):
            clean()
            with patch.dict(m.RESOURCE_CAPS, total_bytes=total, files=9, items=9):
                self.preserve()
            return sum(p.stat().st_size for p in self.out.iterdir() if m.is_registry_evidence(p.name))
        # registry.json 内含帽数字；用与被测上限同位数的帽值重新测量，避免位数差影响计数。
        first = measure(99999)
        need_bytes = measure(10 ** len(str(first)) - 1)
        need_files = sum(1 for p in self.out.iterdir() if m.is_registry_evidence(p.name))
        need_items = len(json.loads((self.out / 'registry.json').read_text())['records'])
        self.assertEqual(len(str(need_bytes)), len(str(need_bytes - 1)))
        for key, limit, ok in (('total_bytes', need_bytes, True), ('total_bytes', need_bytes - 1, False),
                               ('files', need_files, True), ('files', need_files - 1, False),
                               ('items', need_items, True), ('items', need_items - 1, False)):
            with self.subTest(key=key, limit=limit):
                clean()
                caps = dict(total_bytes=10 ** len(str(first)) - 1, files=9, items=9)
                caps[key] = limit
                with patch.dict(m.RESOURCE_CAPS, caps):
                    if ok:
                        self.preserve()
                        self.assertTrue(json.loads((self.out / 'registry.json').read_text())['complete'])
                    else:
                        with self.assertRaises(v.Refused):
                            self.preserve()
                        if (self.out / 'registry.json').exists():
                            self.assertFalse(json.loads((self.out / 'registry.json').read_text())['complete'])

    def test_duplicate_records_accumulate_across_files(self):
        with patch.dict(m.RESOURCE_CAPS, items=3):
            with self.assertRaises(v.Refused):
                self.preserve()

    def test_truncated_mid_segment_duplicate_unwanted_and_bad_validator(self):
        good = json.dumps(cache_record('child', '1.0.0')).encode()
        two = json.dumps(cache_record('child', '2.0.0')).encode()
        variants = {
            'truncated': lambda p: p.write_bytes(p.read_bytes()[:-3]),
            'dup unwanted': lambda p: self.write_raw_cache('child', [('0.1.0', b'{}'), ('0.1.0', b'{}'),
                                                                     ('1.0.0', good), ('2.0.0', two)]),
            'no validator': lambda p: self.write_raw_cache('child', [('1.0.0', good), ('2.0.0', two)], header=b'x'),
            'empty file': lambda p: p.write_bytes(b''),
            'bad magic': lambda p: p.write_bytes(b'\x04' + p.read_bytes()[1:]),
        }
        for label, fn in variants.items():
            with self.subTest(case=label):
                for p in self.out.iterdir():
                    if p.name not in ('Cargo.toml', 'Cargo.lock.old'):
                        p.unlink()
                path = self.write_raw_cache('child', [('1.0.0', good), ('2.0.0', two)])
                fn(path)
                with self.assertRaises(v.Refused):
                    self.preserve()
                proof = json.loads((self.out / 'registry.json').read_text())
                self.assertFalse(proof['complete'])

    def test_scan_deadline_and_signal_mid_stream(self):
        for fault in ('deadline', 'signal'):
            with self.subTest(fault=fault):
                for p in self.out.iterdir():
                    if p.name not in ('Cargo.toml', 'Cargo.lock.old'):
                        p.unlink()
                b = budget()
                supervisor = SimpleNamespace(interrupted=None)
                guard = m.EvidenceIO(self.out, b, supervisor)
                real_open = Path.open
                class Reader(io.BytesIO):
                    calls = 0  # 每个 cache 文件新建 Reader 时不清零：只计首个被扫文件之前的累计
                    def read(inner, size=-1):
                        Reader.calls += 1
                        if Reader.calls == 2:
                            if fault == 'deadline':
                                b.mono -= 1000
                            else:
                                supervisor.interrupted = 15
                        return io.BytesIO.read(inner, size)
                def opener(path, mode='r', *a, **k):
                    if '.cache' in str(path) and mode == 'rb':
                        with real_open(path, mode) as f:
                            return Reader(f.read())
                    return real_open(path, mode, *a, **k)
                with patch.object(m, 'SCAN_CHUNK_BYTES', 8), patch.object(Path, 'open', opener), \
                        self.assertRaises(v.Refused):
                    m.snapshot_registry(self.work, self.out, b, m.lock_changes(self.old, self.new), guard)
                # 第 2 次读后即故障：必须在下一次 read 之前拒绝，不能读完整个文件后才兜底。
                self.assertEqual(Reader.calls, 2)
                self.assertFalse(json.loads((self.out / 'registry-failure.json').read_text())['complete'])

    def test_truncated_unwanted_tail_after_complete_wanted_rejected(self):
        good = json.dumps(cache_record('child', '1.0.0')).encode()
        two = json.dumps(cache_record('child', '2.0.0')).encode()
        path = self.write_raw_cache('child', [('1.0.0', good), ('2.0.0', two)])
        for tail in (b'9.9.9\0{"name": "child"', b'9.9.9', b'9.9.9\0'):
            with self.subTest(tail=tail):
                for p in self.out.iterdir():
                    if p.name not in ('Cargo.toml', 'Cargo.lock.old'):
                        p.unlink()
                path.write_bytes(self.write_raw_cache('child', [('1.0.0', good), ('2.0.0', two)]).read_bytes() + tail)
                with self.assertRaises(v.Refused):
                    self.preserve()
                proof = json.loads((self.out / 'registry.json').read_text())
                self.assertFalse(proof['complete'])

    def test_failure_path_never_exceeds_registry_files_or_total(self):
        for key in ('files', 'total_bytes'):
            with self.subTest(key=key):
                for p in self.out.iterdir():
                    if p.name not in ('Cargo.toml', 'Cargo.lock.old'):
                        p.unlink()
                caps = {'files': 2} if key == 'files' else {'total_bytes': 600}
                with patch.dict(m.RESOURCE_CAPS, caps), self.assertRaises(v.Refused):
                    self.preserve()
                rows = [{'name': p.name, 'bytes': p.stat().st_size} for p in self.out.iterdir()]
                files, size = m.registry_usage(rows)
                with patch.dict(m.RESOURCE_CAPS, caps):
                    self.assertLessEqual(files, m.RESOURCE_CAPS['files'])
                    self.assertLessEqual(size, m.RESOURCE_CAPS['total_bytes'])
                if (self.out / 'registry.json').exists():
                    self.assertFalse(json.loads((self.out / 'registry.json').read_text())['complete'])

    def test_complete_registry_json_downgraded_even_when_registry_caps_full(self):
        self.preserve()
        files, size = m.registry_usage([{'name': p.name, 'bytes': p.stat().st_size} for p in self.out.iterdir()])
        with patch.dict(m.RESOURCE_CAPS, files=files, total_bytes=size):
            self.assertTrue(m.bounded_registry_save(self.out, 'registry.json', {'status': 'FAILED', 'complete': False}))
            self.assertFalse(m.bounded_registry_save(self.out, 'registry-failure.json', {'complete': False}))
        self.assertFalse(json.loads((self.out / 'registry.json').read_text())['complete'])
        self.assertFalse((self.out / 'registry-failure.json').exists())

    def test_bounded_save_falls_back_to_minimal_record_when_full_value_does_not_fit(self):
        minimal = v.canonical(m.MINIMAL_REGISTRY_FAILURE)
        big = {'status': 'FAILED', 'complete': False, 'proof_gap': [{'reason': 'x' * 400}]}
        with patch.dict(m.RESOURCE_CAPS, files=1, total_bytes=len(minimal)):
            self.assertTrue(m.bounded_registry_save(self.out, 'registry.json', big))
        self.assertEqual((self.out / 'registry.json').read_bytes(), minimal)
        with patch.dict(m.RESOURCE_CAPS, files=1, total_bytes=len(minimal) - 1):
            self.assertFalse(m.bounded_registry_save(self.out, 'registry.json', big))

    def test_constraint_parent_range_and_text_unchanged_target_changed_recorded(self):
        self.preserve()
        proof = json.loads((self.out / 'registry.json').read_text())
        parents = [e for e in proof['constraint_parents'] if e['target'][0] == 'child']
        self.assertTrue(parents and all(e['constraints'][0]['req'] == '>=1, <3' for e in parents))
        self.assertEqual(proof['counts']['records'], len(proof['records']))


class UploadBoundTests(ToyCapsTest):
    def test_bound_covers_python_store_zip_and_formula_exact(self):
        names = {'a.json': 0, 'Cargo.lock': 55961, '中文名.txt': 17, 'x' * 200: 3}
        self.assertEqual(m.container_bound(names.items()),
                         98 + sum(size + 128 + 2 * len(('/' + n).encode()) for n, size in names.items()))
        with tempfile.TemporaryDirectory() as d:
            root = Path(d)
            for n, size in names.items():
                (root / n).write_bytes(b'z' * size)
            with v.zipfile.ZipFile(root / 'x.zip', 'w', compression=v.zipfile.ZIP_STORED) as z:
                for n in names:
                    z.write(root / n, n)
            self.assertLessEqual((root / 'x.zip').stat().st_size, m.container_bound(names.items()))

    def test_bound_rejects_forbidden_names_and_zip64_regimes(self):
        for name in ('a:b', 'a|b', 'a\nb', 'a*b', 'a?b', 'a"b', 'a<b', 'a>b', 'a\rb', '', 'a/b', '\udcff'):
            with self.subTest(name=repr(name)), self.assertRaises(v.Refused):
                m.container_bound([(name, 1)])
        with self.assertRaises(v.Refused):
            m.container_bound([('a', 0xFFFFFFFF)])
        with self.assertRaises(v.Refused):
            m.container_bound([('f%d' % i, 0) for i in range(65535)])

    def test_receipt_reserve_is_finite_and_precomputed(self):
        self.assertEqual(m.RECEIPT_RESERVE, m.container_bound(m.RECEIPT_LIMITS.items()))
        self.assertEqual(set(m.RECEIPT_LIMITS), {m.RECEIPT_REQUEST, m.RECEIPT_FROZEN, m.RECEIPT_ACTUAL})

    def test_output_rejects_directories_and_symlinks(self):
        with tempfile.TemporaryDirectory() as d:
            out = Path(d)
            (out / 'sub').mkdir()
            with self.assertRaises(v.Refused):
                m.EvidenceIO(out, budget())
            (out / 'sub').rmdir()
            (out / 'link').symlink_to('/etc/hosts')
            with self.assertRaises(v.Refused):
                m.EvidenceIO(out, budget())

    def test_write_refused_when_containers_would_exceed_upload_cap(self):
        with tempfile.TemporaryDirectory() as d:
            out = Path(d)
            guard = m.EvidenceIO(out, budget())
            room = guard.headroom('big.bin')
            cap = m.RESOURCE_CAPS['upload_bytes']
            with patch.dict(m.RESOURCE_CAPS, upload_bytes=cap - room + 100):
                with patch.object(Path, 'open') as opened, self.assertRaises(v.Refused):
                    guard.write('big.bin', b'b' * 101)
                opened.assert_not_called()
                guard.write('big.bin', b'b' * 100)
                self.assertEqual(guard.check()['main_bound'] + m.RECEIPT_RESERVE, m.RESOURCE_CAPS['upload_bytes'])

    def test_check_refuses_existing_output_over_combined_cap_equal_passes(self):
        with tempfile.TemporaryDirectory() as d:
            out = Path(d)
            (out / 'a.log').write_bytes(b'a' * 1234)
            (out / '中文.json').write_bytes(b'{}')
            exact = m.container_bound((p.name, p.stat().st_size) for p in out.iterdir()) + m.RECEIPT_RESERVE
            with patch.dict(m.RESOURCE_CAPS, upload_bytes=exact):
                self.assertEqual(m.EvidenceIO(out, budget()).check()['main_bound'] + m.RECEIPT_RESERVE, exact)
            with patch.dict(m.RESOURCE_CAPS, upload_bytes=exact - 1), self.assertRaises(v.Refused):
                m.EvidenceIO(out, budget())
            guard = m.EvidenceIO(out, budget())
            with patch.dict(m.RESOURCE_CAPS, upload_bytes=exact - 1):
                for call in (guard.check, lambda: guard.take_items(0), lambda: guard.run(len, b'')):
                    with self.assertRaises(v.Refused):
                        call()
            self.assertEqual((out / 'a.log').read_bytes(), b'a' * 1234)

    def test_package_projection_blocks_before_any_tail_write(self):
        with tempfile.TemporaryDirectory() as d:
            out = Path(d)
            m.save(out / 'result.json', {'status': 'FAILED'})
            m.save(out / 'steps.json', [])
            (out / 'original.log').write_bytes(b'o' * 1000)
            state = m.EvidenceIO(out, budget()).check()
            with patch.dict(m.RESOURCE_CAPS, upload_bytes=state['main_bound'] + m.RECEIPT_RESERVE + 2000), \
                    self.assertRaises(v.Refused):
                m.package(out, budget())
            self.assertFalse((out / 'evidence.zip').exists())
            self.assertFalse((out / 'FILES.json').exists())
            self.assertEqual((out / 'original.log').read_bytes(), b'o' * 1000)
            m.package(out, budget())
            zip_size = (out / 'evidence.zip').stat().st_size
            self.assertLessEqual(zip_size, m.container_bound(
                [(p.name, p.stat().st_size) for p in out.iterdir() if p.name not in m.PACKAGE_TAIL] +
                [('FILES.json', (out / 'FILES.json').stat().st_size)]))


class GateTests(ToyCapsTest):
    def setUp(self):
        super().setUp()
        self.temp = Path(self.enterContext(tempfile.TemporaryDirectory())).resolve()
        self.out = self.temp / 'issue1393-lock-evidence'
        self.out.mkdir()
        (self.temp / 'issue1393-lock-start.json').write_text(json.dumps(
            {'monotonic': time.monotonic() - 100, 'wall': time.time() - 100, 'total_seconds': 900}))
        (self.out / 'result.json').write_bytes(b'{}\n')
        (self.out / 'evidence.zip').write_bytes(b'e' * 3000)
        self.gh = self.temp / 'gh-output'
        self.env = {'ISSUE1393_LOCK_EXECUTE': 'True', 'RUNNER_TEMP': str(self.temp), 'GITHUB_OUTPUT': str(self.gh)}
        self.spawn = self.enterContext(patch.object(subprocess, 'Popen', side_effect=AssertionError('零真实子进程')))

    def run_gate(self, mode, **env):
        if self.gh.exists():
            self.gh.unlink()
        with patch.dict(os.environ, dict(self.env, **env), clear=True), contextlib.redirect_stdout(io.StringIO()) as out:
            self.assertEqual(m.gate(mode), 0)
        outputs = dict(line.split('=', 1) for line in self.gh.read_text().splitlines())
        return outputs['allowed'] == 'true', json.loads(out.getvalue().splitlines()[-1])

    def main_bound(self):
        return m.container_bound((p.name, p.stat().st_size) for p in self.out.iterdir())

    def test_upload_gate_equal_and_over_cap(self):
        exact = self.main_bound() + m.RECEIPT_RESERVE
        with patch.dict(m.RESOURCE_CAPS, upload_bytes=exact - 1):
            allowed, status = self.run_gate('upload-gate')
        self.assertFalse(allowed)
        self.assertFalse((self.temp / m.RECEIPT_FROZEN).exists())
        self.assertEqual(json.loads((self.temp / m.RECEIPT_REQUEST).read_text())['status'], 'REFUSED')
        self.assertEqual((self.out / 'evidence.zip').read_bytes(), b'e' * 3000)
        (self.temp / m.RECEIPT_REQUEST).unlink()
        with patch.dict(m.RESOURCE_CAPS, upload_bytes=exact):
            allowed, status = self.run_gate('upload-gate')
        self.assertTrue(allowed)
        frozen = json.loads((self.temp / m.RECEIPT_FROZEN).read_text())
        self.assertEqual(frozen['main_bound'], self.main_bound())
        self.assertEqual({e['name'] for e in frozen['entries']}, {'result.json', 'evidence.zip'})
        self.assertFalse(any(p.name.startswith('issue1393-lock-UPLOAD') for p in self.out.iterdir()))

    def test_receipt_gate_combined_cap_change_and_actual_limit(self):
        self.assertTrue(self.run_gate('upload-gate')[0])
        actual = self.temp / m.RECEIPT_ACTUAL
        actual.write_bytes(b'{"conclusion": "success"}\n')
        allowed, status = self.run_gate('receipt-gate', PAYLOAD_OUTCOME='success')
        self.assertTrue(allowed)
        sizes = [(n, (self.temp / n).stat().st_size) for n in m.RECEIPT_LIMITS]
        exact = self.main_bound() + m.container_bound(sizes)
        self.assertEqual(status['main_bound'] + status['receipt_bound'], exact)
        with patch.dict(m.RESOURCE_CAPS, upload_bytes=exact):
            self.assertTrue(self.run_gate('receipt-gate', PAYLOAD_OUTCOME='success')[0])
        with patch.dict(m.RESOURCE_CAPS, upload_bytes=exact - 1):
            self.assertFalse(self.run_gate('receipt-gate', PAYLOAD_OUTCOME='success')[0])
        actual.write_bytes(b'a' * 4097)
        self.assertFalse(self.run_gate('receipt-gate', PAYLOAD_OUTCOME='success')[0])
        actual.write_bytes(b'{}\n')
        (self.out / 'late.txt').write_bytes(b'x')
        self.assertFalse(self.run_gate('receipt-gate', PAYLOAD_OUTCOME='success')[0])
        (self.out / 'late.txt').unlink()
        (self.out / 'result.json').write_bytes(b'{"x": 1}\n')
        self.assertFalse(self.run_gate('receipt-gate', PAYLOAD_OUTCOME='success')[0])
        (self.out / 'result.json').write_bytes(b'{}\n')
        self.assertTrue(self.run_gate('receipt-gate', PAYLOAD_OUTCOME='success')[0])
        (self.out / 'result.json').write_bytes(b'[]\n')  # 同长度、内容不同
        self.assertFalse(self.run_gate('receipt-gate', PAYLOAD_OUTCOME='success')[0])

    def test_gates_refuse_registry_evidence_over_caps(self):
        (self.out / 'registry-a.raw').write_bytes(b'r' * 100)
        (self.out / 'registry.json').write_bytes(b'{}')
        with patch.dict(m.RESOURCE_CAPS, files=1):
            self.assertFalse(self.run_gate('upload-gate')[0])
        with patch.dict(m.RESOURCE_CAPS, total_bytes=101):
            self.assertFalse(self.run_gate('upload-gate')[0])
        self.assertFalse((self.temp / m.RECEIPT_FROZEN).exists())
        self.assertEqual((self.out / 'registry-a.raw').read_bytes(), b'r' * 100)
        (self.temp / m.RECEIPT_REQUEST).unlink()
        with patch.dict(m.RESOURCE_CAPS, total_bytes=102, files=2):
            self.assertTrue(self.run_gate('upload-gate')[0])
        (self.temp / m.RECEIPT_ACTUAL).write_bytes(b'{}\n')
        with patch.dict(m.RESOURCE_CAPS, total_bytes=101):
            self.assertFalse(self.run_gate('receipt-gate', PAYLOAD_OUTCOME='success')[0])

    def test_receipt_without_frozen_only_when_main_not_uploaded(self):
        (self.temp / m.RECEIPT_ACTUAL).write_bytes(b'{}\n')
        self.assertFalse(self.run_gate('receipt-gate', PAYLOAD_OUTCOME='success')[0])
        self.assertTrue(self.run_gate('receipt-gate', PAYLOAD_OUTCOME='skipped')[0])

    def test_false_gate_deadline_and_unknown_mode_refuse(self):
        self.assertFalse(self.run_gate('upload-gate', ISSUE1393_LOCK_EXECUTE='False')[0])
        (self.temp / 'issue1393-lock-start.json').write_text(json.dumps(
            {'monotonic': time.monotonic() - 899, 'wall': time.time() - 899, 'total_seconds': 900}))
        self.assertFalse(self.run_gate('upload-gate')[0])
        self.assertFalse(self.run_gate('other-gate')[0])

    def test_workflow_pins_action_compression_and_gates(self):
        import yaml
        text = (m.ROOT / m.WORKFLOW).read_text()
        flow = yaml.safe_load(text)
        steps = flow['jobs']['lock-prepare']['steps']
        uploads = [s for s in steps if str(s.get('uses', '')).startswith('actions/upload-artifact')]
        self.assertEqual(len(uploads), 2)
        for s in uploads:
            self.assertEqual(s['uses'], m.UPLOAD_ACTION)
            self.assertEqual(s['with']['compression-level'], 0)
            self.assertIn("allowed == 'true'", s['if'])
        receipt_paths = uploads[1]['with']['path'].splitlines()
        # 多文件 path 的 rootDirectory 是最近公共祖先；三件都必须直接位于 runner.temp，
        # 才使 destinationPath = '/'+名，与 container_bound/receipt-gate 计数的名字一致。
        self.assertEqual(receipt_paths, ['${{ runner.temp }}/' + n for n in m.RECEIPT_LIMITS])
        self.assertTrue(all(p.count('/') == 1 for p in receipt_paths))
        self.assertEqual(uploads[0]['with']['path'], '${{ runner.temp }}/issue1393-lock-evidence/')
        archive = next(s for s in steps if s.get('id') == 'archive')['run']
        self.assertIn("Path(os.environ['RUNNER_TEMP']) / 'issue1393-lock-evidence'", archive)
        self.assertEqual(uploads[0]['if'], "always() && env.ISSUE1393_LOCK_EXECUTE == 'True' && steps.archive.outputs.allowed == 'true'")
        self.assertEqual(uploads[1]['if'], "always() && env.ISSUE1393_LOCK_EXECUTE == 'True' && steps.receipt.outputs.allowed == 'true'")
        self.assertEqual(uploads[0]['timeout-minutes'], '${{ fromJSON(steps.archive.outputs.minutes) }}')
        self.assertEqual(uploads[1]['timeout-minutes'], '${{ fromJSON(steps.receipt.outputs.minutes) }}')
        order = [s.get('id') or s.get('name') for s in steps]
        self.assertLess(order.index('archive'), order.index('payload'))
        self.assertLess(order.index('payload'), order.index('receipt'))
        self.assertLess(order.index('receipt'), steps.index(uploads[1]))
        runs = {s.get('id'): s.get('run', '') for s in steps}
        archive_run = runs['archive']
        gate_at = archive_run.index('issue1393_lock_prepare.py upload-gate')
        for placeholder in ("save('result.json'", "save('steps.json'", "save('PACKAGE.json'"):
            self.assertLess(archive_run.index(placeholder), gate_at)
        self.assertNotIn('write', archive_run[gate_at:])
        receipt_run = runs['receipt']
        self.assertIn('if len(raw) > 4096:', receipt_run)
        self.assertEqual(m.RECEIPT_LIMITS[m.RECEIPT_ACTUAL], 4096)
        self.assertLess(receipt_run.index("'issue1393-lock-UPLOAD-ACTUAL.json', 'xb'"),
                        receipt_run.index('issue1393_lock_prepare.py receipt-gate'))
        self.assertIn('issue1393_lock_prepare.py upload-gate', runs['archive'])
        self.assertIn('issue1393_lock_prepare.py receipt-gate', runs['receipt'])
        self.assertNotIn('RESOURCE_CAPS', text)
        self.assertNotIn('UPLOAD-REQUEST.json', runs['archive'].split('upload-gate')[0])
        self.assertNotIn('@v4\n', ''.join(str(s.get('uses', '')) + '\n' for s in uploads))
        self.assertIn("ISSUE1393_LOCK_EXECUTE: 'False'", text)


if __name__ == '__main__':
    unittest.main()
