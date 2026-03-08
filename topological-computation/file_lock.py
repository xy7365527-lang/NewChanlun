"""Cross-platform file locking for multi-instance shared file writes.

Uses fcntl.flock on Unix and a .lock file with atomic creation on Windows.
Pure Python, no external dependencies.
"""

from __future__ import annotations

import os
import sys
import time
from contextlib import contextmanager
from pathlib import Path
from typing import IO, Generator


if sys.platform == "win32":
    import msvcrt

    @contextmanager
    def locked_append(path: str | Path, encoding: str = "utf-8") -> Generator[IO, None, None]:
        """Open file for append with exclusive lock (Windows).

        Uses a separate .lock file with msvcrt.locking for mutual exclusion.
        The data file is opened for append; the lock file guards the write.
        """
        p = Path(path)
        p.parent.mkdir(parents=True, exist_ok=True)
        lock_path = p.with_suffix(p.suffix + ".lock")

        # Open or create lock file; keep it open for the lock duration
        lock_fh = open(lock_path, "a+b")
        try:
            # msvcrt.locking needs at least 1 byte to lock
            lock_fh.seek(0)
            lock_fh.write(b"\x00")
            lock_fh.flush()
            lock_fh.seek(0)
            # Blocking lock on byte 0
            msvcrt.locking(lock_fh.fileno(), msvcrt.LK_LOCK, 1)
            try:
                fh = open(p, "a", encoding=encoding)
                try:
                    yield fh
                    fh.flush()
                finally:
                    fh.close()
            finally:
                lock_fh.seek(0)
                msvcrt.locking(lock_fh.fileno(), msvcrt.LK_UNLCK, 1)
        finally:
            lock_fh.close()

else:
    import fcntl

    @contextmanager
    def locked_append(path: str | Path, encoding: str = "utf-8") -> Generator[IO, None, None]:
        """Open file for append with exclusive lock (Unix).

        Uses fcntl.flock on the data file directly.
        """
        p = Path(path)
        p.parent.mkdir(parents=True, exist_ok=True)
        fh = open(p, "a", encoding=encoding)
        try:
            fcntl.flock(fh.fileno(), fcntl.LOCK_EX)
            yield fh
            fh.flush()
        finally:
            fcntl.flock(fh.fileno(), fcntl.LOCK_UN)
            fh.close()


def get_instance_id() -> str:
    """Auto-detect instance identity from hostname + PID."""
    import socket
    return f"{socket.gethostname()}-{os.getpid()}"
