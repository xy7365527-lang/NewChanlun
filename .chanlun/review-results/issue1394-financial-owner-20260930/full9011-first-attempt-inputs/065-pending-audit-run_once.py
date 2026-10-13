#!/usr/bin/env python3
"""Run the one frozen read-only diagnostic against an owned three-file clone."""
import hashlib
import json
import os
import pathlib
import signal
import stat
import subprocess
import time
from datetime import datetime, timezone

HERE = pathlib.Path(__file__).resolve().parent
PILOT = pathlib.Path('/Users/silencehan/Projects/NewChanlun-1394-reuse-pilot-20261001')
SOURCE = pathlib.Path('/Users/silencehan/Projects/NewChanlun-1394-content-v3/.scratch/issue1394-full9011-formal-runtime-20261001/A')
BINARY = PILOT / '.scratch/issue1394-reuse-pilot-20261001/target/release/deps/s_structure_session-f691a9b42625f576'
BUILD = PILOT / '.scratch/issue1394-reuse-pilot-20261001/LOCAL-output/BUILD.json'
PATCH = PILOT / '.scratch/issue1394-reuse-pilot-20261001/DIAGNOSTIC.patch'
ORIGINAL_NAMES = ('s.sqlite', 's.sqlite-wal', 's.sqlite-shm')
ARGS = (str(BINARY), 'v3::state::readonly_reuse_pilot::readonly_audit_reuse_pilot',
        '--exact', '--ignored', '--nocapture', '--test-threads=1')
MAX_SECONDS = 180


def sha(path):
    digest = hashlib.sha256()
    with path.open('rb') as source:
        for block in iter(lambda: source.read(1024 * 1024), b''):
            digest.update(block)
    return digest.hexdigest()


def save(name, value):
    path = HERE / name
    with path.open('x', encoding='utf-8') as target:
        json.dump(value, target, ensure_ascii=False, sort_keys=True, indent=2)
        target.write('\n')
        target.flush()
        os.fsync(target.fileno())


def identity(path):
    before = path.lstat()
    digest = sha(path)
    after = path.lstat()
    keys = ('st_dev', 'st_ino', 'st_size', 'st_mtime_ns', 'st_ctime_ns')
    if any(getattr(before, key) != getattr(after, key) for key in keys):
        raise RuntimeError(f'file changed while hashing: {path}')
    return dict(path=str(path), sha256=digest, bytes=after.st_size,
                inode=after.st_ino, device=after.st_dev, mtime_ns=after.st_mtime_ns,
                ctime_ns=after.st_ctime_ns, uid=after.st_uid, gid=after.st_gid,
                mode=oct(stat.S_IMODE(after.st_mode)))


def sample(pid):
    at = time.monotonic_ns()
    try:
        checked = subprocess.run(['ps', '-p', str(pid), '-o', 'uid=,rss=,stat='],
                                 capture_output=True, text=True, timeout=3)
    except subprocess.TimeoutExpired:
        return dict(at_ns=at, uid='UNMEASURED', rss_bytes='UNMEASURED',
                    error='ps timeout')
    parts = checked.stdout.split()
    if checked.returncode == 0 and len(parts) >= 3 and parts[0].isdigit() and parts[1].isdigit():
        return dict(at_ns=at, uid=int(parts[0]), rss_bytes=int(parts[1]) * 1024,
                    stat=' '.join(parts[2:]))
    return dict(at_ns=at, uid='UNMEASURED', rss_bytes='UNMEASURED',
                ps_exit=checked.returncode)


def main():
    before = json.loads((HERE / 'ORIGINALS-BEFORE.json').read_bytes())
    owned = json.loads((HERE / 'OWNED-BEFORE.json').read_bytes())
    if [identity(SOURCE / name) for name in ORIGINAL_NAMES] != before:
        raise RuntimeError('original three-file baseline changed before invocation')
    if [identity(HERE / 'owned' / name) for name in ORIGINAL_NAMES] != owned:
        raise RuntimeError('owned clone changed before invocation')
    build = json.loads(BUILD.read_bytes())
    if (build['test_binary']['path'] != str(BINARY)
            or build['test_binary']['sha256'] != sha(BINARY)
            or build['diagnostic_patch_sha256'] != sha(PATCH)):
        raise RuntimeError('frozen diagnostic binary/source binding mismatch')
    if (HERE / 'PROBE.json').exists():
        raise RuntimeError('probe output must be fresh')
    env = os.environ.copy()
    env.update(S3_REUSE_PILOT_DB=str(HERE / 'owned/s.sqlite'),
               S3_REUSE_PILOT_OUTPUT=str(HERE / 'PROBE.json'),
               TMPDIR=str(HERE / 'tmp'))
    started = time.monotonic_ns()
    out = (HERE / 'stdout.raw').open('xb')
    err = (HERE / 'stderr.raw').open('xb')
    process = None
    rows = []
    timeout = False
    try:
        process = subprocess.Popen(ARGS, cwd=HERE, env=env, stdin=subprocess.DEVNULL,
                                   stdout=out, stderr=err, start_new_session=True)
        save('RUNNING.json', dict(pid=process.pid, parent_pid=os.getpid(), uid=os.getuid(),
            argv=list(ARGS), started_utc=datetime.now(timezone.utc).isoformat(),
            started_monotonic_ns=started, binary_sha256=build['test_binary']['sha256'],
            source_binding_sha256=sha(PILOT / '.scratch/issue1394-reuse-pilot-20261001/SOURCE-BINDING.json'),
            db=str(HERE / 'owned/s.sqlite'), output=str(HERE / 'PROBE.json'),
            tmpdir=str(HERE / 'tmp'), timeout_seconds=MAX_SECONDS,
            read_only_test=True, test_invocations=1))
        print(json.dumps(dict(event='LAUNCHED', pid=process.pid, argv=list(ARGS),
                              run_state=str(HERE / 'RUNNING.json'))), flush=True)
        deadline = time.monotonic() + MAX_SECONDS
        while True:
            rows.append(sample(process.pid))
            if process.poll() is not None:
                break
            if time.monotonic() >= deadline:
                timeout = True
                os.killpg(process.pid, signal.SIGTERM)
                try:
                    process.wait(timeout=2)
                except subprocess.TimeoutExpired:
                    os.killpg(process.pid, signal.SIGKILL)
                    process.wait(timeout=5)
                break
            time.sleep(min(0.1, max(0, deadline - time.monotonic())))
        returncode = process.wait()
    finally:
        if process is not None and process.poll() is None:
            os.killpg(process.pid, signal.SIGTERM)
            try:
                process.wait(timeout=2)
            except subprocess.TimeoutExpired:
                os.killpg(process.pid, signal.SIGKILL)
                process.wait(timeout=5)
        for handle in (out, err):
            handle.flush()
            os.fsync(handle.fileno())
            handle.close()
    save('RSS-SAMPLES.json', rows)
    measured = [row['rss_bytes'] for row in rows if type(row['rss_bytes']) is int]
    gaps = [b['at_ns'] - a['at_ns'] for a, b in zip(rows, rows[1:])]
    result = dict(status='TEST_EXIT0' if returncode == 0 and (HERE / 'PROBE.json').is_file()
                  else 'DIAGNOSTIC_FAILED', pid=process.pid, uid=os.getuid(), argv=list(ARGS),
                  returncode=returncode, exit_signal=-returncode if returncode < 0 else None,
                  same_popen_reaped=process.poll() is not None,
                  timeout=timeout, duration_ns=time.monotonic_ns() - started,
                  stdout_sha256=sha(HERE / 'stdout.raw'), stderr_sha256=sha(HERE / 'stderr.raw'),
                  probe_exists=(HERE / 'PROBE.json').is_file(),
                  rss=dict(period_target_ms=100, sample_count=len(rows),
                           max_sample_bytes_lower_bound=max(measured) if measured else 'UNMEASURED',
                           observed_max_gap_ns=max(gaps) if gaps else 'UNMEASURED',
                           true_peak='UNMEASURED'))
    save('RUN-RESULT.json', result)
    print(json.dumps(dict(event='TERMINAL', pid=process.pid, status=result['status'],
                          returncode=returncode, probe_exists=result['probe_exists'])), flush=True)


if __name__ == '__main__':
    main()
