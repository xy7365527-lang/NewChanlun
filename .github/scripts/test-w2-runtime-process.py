import importlib.util
import json
from pathlib import Path
import signal
import subprocess
import sys
import tempfile
import tarfile
import time
import unittest
from types import SimpleNamespace
from unittest.mock import patch

SCRIPT = Path(__file__).with_name('w2-runtime-process.py')
spec = importlib.util.spec_from_file_location('runtime_process', SCRIPT)
runtime = importlib.util.module_from_spec(spec)
spec.loader.exec_module(runtime)


class Preservation(unittest.TestCase):
    def test_ci_pipeline_retains_python_failure(self):
        result = subprocess.run(['bash', '-euo', 'pipefail', '-c',
                                 'python3 -c \"raise SystemExit(7)\" 2>&1 | tee /dev/null'])
        self.assertEqual(result.returncode, 7)

    def test_tar_failure_is_not_complete(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            args = SimpleNamespace(directory=root, archive=root / 'out.tgz',
                                   state=root / 'status.json')
            with patch.object(runtime.subprocess, 'run', return_value=SimpleNamespace(returncode=1)):
                self.assertEqual(runtime.package(args), 1)
            status = json.loads(args.state.read_text())
            self.assertFalse(status['archive_complete'])
            self.assertEqual(status['tar_returncode'], 1)

    def test_unstopped_writer_refuses_archive(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            (root / 'a').mkdir()
            (root / 'a/process.json').write_text(json.dumps({'stopped': False}))
            args = SimpleNamespace(directory=root, archive=root / 'out.tgz',
                                   state=root / 'status.json')
            with patch.object(runtime.subprocess, 'run') as tar:
                self.assertEqual(runtime.package(args), 125)
                tar.assert_not_called()
            self.assertFalse(json.loads(args.state.read_text())['archive_complete'])

    def test_missing_writer_state_refuses_archive_even_with_another_safe_side(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            (root / 'a').mkdir()
            (root / 'a/process.json').write_text(json.dumps({'stopped': True, 'process_group': 1}))
            (root / 'b/w2.run').mkdir(parents=True)
            (root / 'b/w2.run/s.sqlite').touch()
            args = SimpleNamespace(directory=root, archive=root / 'out.tgz',
                                   state=root / 'status.json')
            with patch.object(runtime, 'live_group', return_value=[]), patch.object(runtime.subprocess, 'run') as tar:
                self.assertEqual(runtime.package(args), 125)
                tar.assert_not_called()

    @unittest.skipUnless(sys.platform == 'linux', '真实进程组验收使用Linux CI')
    def test_persistent_receipt_failure_preserves_stopped_originals(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            side = root / 'a'
            (side / 'w2.run').mkdir(parents=True)
            (side / 'w2.run/s.sqlite').write_bytes(b'database-original')
            (side / 'w2.run/s.sqlite-wal').write_bytes(b'wal-original')
            args = SimpleNamespace(log=side / 'export.log', state=side / 'process.json',
                                   command=[sys.executable, '-c', 'import time; time.sleep(60)'])
            started = []
            popen = runtime.subprocess.Popen
            def record(*args, **kwargs):
                child = popen(*args, **kwargs)
                started.append(child)
                return child
            with patch.object(runtime, 'save', side_effect=OSError('receipt unavailable')), patch.object(runtime.subprocess, 'Popen', side_effect=record):
                self.assertEqual(runtime.run(args), 125)
            self.assertIsNotNone(started[0].poll())
            self.assertEqual(runtime.live_group(started[0].pid), [])
            with tarfile.open(root / 'a.stopped-originals.tgz') as archive:
                self.assertEqual(archive.extractfile('a/w2.run/s.sqlite').read(), b'database-original')
                self.assertEqual(archive.extractfile('a/w2.run/s.sqlite-wal').read(), b'wal-original')

    @unittest.skipUnless(sys.platform == 'linux', '真实进程组验收使用Linux CI')
    def test_initial_receipt_failure_finalizes_and_allows_package(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            evidence = root / 'evidence'
            side = evidence / 'a'
            (side / 'w2.run').mkdir(parents=True)
            db = side / 'w2.run/s.sqlite'
            args = SimpleNamespace(log=side / 'export.log', state=side / 'process.json',
                                   command=[sys.executable, '-c',
                                            'import pathlib,sys,time; pathlib.Path(sys.argv[1]).write_bytes(b\"original\"); time.sleep(60)', str(db)])
            original_save = runtime.save
            calls = []
            def first_failure(path, value):
                calls.append(path)
                if len(calls) == 1:
                    deadline = time.monotonic() + 5
                    while not db.exists() and time.monotonic() < deadline:
                        time.sleep(.01)
                    self.assertTrue(db.exists())
                    raise OSError('initial receipt failure')
                return original_save(path, value)
            with patch.object(runtime, 'save', side_effect=first_failure):
                self.assertEqual(runtime.run(args), 125)
            result = json.loads(args.state.read_text())
            self.assertTrue(result['stopped'])
            self.assertIn('initial receipt failure', result['supervisor_error'])
            package = SimpleNamespace(directory=evidence, archive=root / 'evidence.tgz',
                                      state=root / 'preservation.json')
            self.assertEqual(runtime.package(package), 0)
            with tarfile.open(package.archive) as archive:
                self.assertEqual(archive.extractfile('evidence/a/w2.run/s.sqlite').read(), b'original')

    @unittest.skipUnless(sys.platform == 'linux', '真实进程组验收使用Linux CI')
    def test_natural_failure_retains_exit_code(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            state = root / 'process.json'
            process = subprocess.run([sys.executable, str(SCRIPT), 'run', '--state',
                                      str(state), '--log', str(root / 'export.log'), '--',
                                      sys.executable, '-c', 'raise SystemExit(7)'])
            self.assertEqual(process.returncode, 7)
            result = json.loads(state.read_text())
            self.assertTrue(result['stopped'])
            self.assertEqual(result['returncode'], 7)

    @unittest.skipUnless(sys.platform == 'linux', '真实进程组验收使用Linux CI')
    def test_cancel_stops_child_and_grandchild_before_package(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            evidence = root / 'evidence'
            side = evidence / 'a'
            side.mkdir(parents=True)
            writer = root / 'writer.py'
            writer.write_text("import os,signal,time\nsignal.signal(signal.SIGTERM,signal.SIG_IGN)\nos.fork()\nwhile True:\n print('write',flush=True)\n time.sleep(.01)\n")
            state = side / 'process.json'
            log = side / 'export.log'
            process = subprocess.Popen([sys.executable, str(SCRIPT), 'run', '--state',
                                        str(state), '--log', str(log), '--', sys.executable, str(writer)])
            try:
                deadline = time.monotonic() + 5
                while (not state.exists() or log.stat().st_size == 0) and time.monotonic() < deadline:
                    time.sleep(.02)
                self.assertTrue(state.exists())
                self.assertGreater(log.stat().st_size, 0)
                process.send_signal(signal.SIGTERM)
                self.assertEqual(process.wait(timeout=5), 143)
                result = json.loads(state.read_text())
                self.assertTrue(result['stopped'])
                self.assertEqual(result['returncode'], -signal.SIGKILL)
                self.assertEqual(runtime.live_group(result['process_group']), [])
                before = log.read_bytes()
                args = SimpleNamespace(directory=evidence, archive=root / 'evidence.tgz',
                                       state=root / 'preservation.json')
                self.assertEqual(runtime.package(args), 0)
                self.assertEqual(log.read_bytes(), before)
                self.assertTrue(json.loads(args.state.read_text())['archive_complete'])
            finally:
                if process.poll() is None:
                    process.send_signal(signal.SIGTERM)
                    process.wait(timeout=5)


if __name__ == '__main__':
    unittest.main()
