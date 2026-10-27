"""#1527 I-1：小文件/纯账本模型；不导入或执行 Rust/SQLite，不能签 Rust 验收。"""
import fcntl
import json
import os
from pathlib import Path
import tempfile
import unittest

class Ledger:
    def __init__(self, t0, wire_cap, deadline, existing):
        self.t0, self.cap, self.deadline = t0, wire_cap, deadline
        self.wire, self.existing, self.stopped, self.pending = 0, existing, None, {}
    def actual(self, count, now):
        if self.stopped:
            raise ValueError(self.stopped)
        self.wire += count
        if self.wire > self.cap:
            self.stopped = 'wire'
        elif now >= self.deadline:
            self.stopped = 'deadline'
        if self.stopped:
            raise ValueError(self.stopped)
    def finish(self, now):
        if self.stopped or self.pending or now >= self.deadline:
            raise ValueError('unfinished')

class Checks(unittest.TestCase):
    def test_actual_wire_still_precedes_deadline(self):
        account = Ledger(11, 9, 20, 4096)
        with self.assertRaisesRegex(ValueError, 'wire'):
            account.actual(10, 20)
        self.assertEqual((account.wire, account.existing, account.t0), (10, 4096, 11))
    def test_pre_reservation_account_and_finish_not_swapped(self):
        account = Ledger(11, 99, 20, 4096)
        view = account
        account.actual(12, 12)
        account.pending[1] = 8
        view = Ledger(19, 9999, 99, 0)
        view.finish(20)
        with self.assertRaises(ValueError):
            account.finish(13)
        del account.pending[1]
        account.finish(13)
        self.assertEqual((account.t0, account.wire, account.existing), (11, 12, 4096))
    def test_nonzero_physical_baseline_and_sidecar_included(self):
        with tempfile.TemporaryDirectory(prefix='i1527-physical-') as directory:
            root = Path(directory)
            main, wal = root/'main', root/'main-wal'
            main.write_bytes(b'x' * 8192)
            wal.write_bytes(b'y' * 4096)
            measured = sum(p.stat().st_blocks * 512 for p in [root, main, wal])
            self.assertGreater(measured, 0)
            self.assertGreaterEqual(measured, main.stat().st_blocks * 512 + wal.stat().st_blocks * 512)
            sparse = root/'sparse'
            with sparse.open('wb') as out:
                out.seek(1024*1024)
                out.write(b'x')
            # 本机 seek 不保证稀疏：模型只核物理量不能拿逻辑长度冒充。
            physical = sparse.stat().st_blocks * 512
            self.assertEqual(physical % 512, 0)
            self.assertNotEqual(physical, sparse.stat().st_size)
    def test_dual_lock_failfast_and_release(self):
        with tempfile.TemporaryDirectory(prefix='i1527-lock-') as directory:
            for name in ['main.writer.lock', 'main.maintenance.lock']:
                path=Path(directory)/name
                with path.open('a+b') as first, path.open('a+b') as second:
                    fcntl.flock(first, fcntl.LOCK_EX|fcntl.LOCK_NB)
                    with self.assertRaises(BlockingIOError):
                        fcntl.flock(second, fcntl.LOCK_EX|fcntl.LOCK_NB)
                    fcntl.flock(first, fcntl.LOCK_UN)
                    fcntl.flock(second, fcntl.LOCK_EX|fcntl.LOCK_NB)
    def test_same_bytes_replacement_keeps_original_fd_and_fails_identity(self):
        with tempfile.TemporaryDirectory(prefix='i1527-fd-') as directory:
            path=Path(directory)/'main'
            path.write_bytes(b'original')
            with path.open('rb') as file:
                original=os.fstat(file.fileno())
                replacement=path.with_suffix('.next');replacement.write_bytes(b'original');replacement.replace(path)
                self.assertNotEqual((original.st_dev,original.st_ino),(path.stat().st_dev,path.stat().st_ino))
                self.assertEqual(file.read(),b'original')
    def test_known_media_do_not_fallback(self):
        def choose(wal, shm, journal):
            if shm or (wal and journal): return 'UnsupportedCombination'
            if journal: return 'D2ProductionCapacityMissing'
            return 'D1Wal' if wal else 'D1Main'
        self.assertEqual([choose(*bits) for bits in [(0,0,0),(1,0,0),(0,0,1),(1,0,1),(1,1,0)]],
                         ['D1Main','D1Wal','D2ProductionCapacityMissing','UnsupportedCombination','UnsupportedCombination'])

if __name__ == '__main__':
    unittest.main(verbosity=2)
