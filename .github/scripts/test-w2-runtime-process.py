import importlib.util
import json
from pathlib import Path
import signal
import subprocess
import sys
import tempfile
import time
import unittest
from types import SimpleNamespace
from unittest.mock import patch

SCRIPT = Path(__file__).with_name('w2-runtime-process.py')
spec = importlib.util.spec_from_file_location('runtime_process', SCRIPT)
runtime = importlib.util.module_from_spec(spec)
spec.loader.exec_module(runtime)


class Preservation(unittest.TestCase):
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
    def test_state_write_failure_still_stops_child(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            args = SimpleNamespace(log=root / 'export.log', state=root / 'process.json',
                                   command=[sys.executable, '-c', 'import time; time.sleep(60)'])
            started = []
            popen = runtime.subprocess.Popen
            def record(*args, **kwargs):
                child = popen(*args, **kwargs)
                started.append(child)
                return child
            with patch.object(runtime, 'save', side_effect=OSError('disk full')), patch.object(runtime.subprocess, 'Popen', side_effect=record):
                with self.assertRaisesRegex(OSError, 'disk full'):
                    runtime.run(args)
            self.assertIsNotNone(started[0].poll())
            self.assertEqual(runtime.live_group(started[0].pid), [])

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
