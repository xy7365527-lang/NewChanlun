#!/usr/bin/env python3
"""#1419 的独立导出进程组与停止后归档。"""
import argparse
import json
import os
from pathlib import Path
import signal
import subprocess
import sys
import time


def save(path, value):
    temporary = path.with_suffix(path.suffix + '.tmp')
    temporary.write_text(json.dumps(value, ensure_ascii=False, indent=2) + '\n')
    temporary.replace(path)


def live_group(group):
    live = []
    for entry in Path('/proc').iterdir():
        if not entry.name.isdigit():
            continue
        try:
            fields = (entry / 'stat').read_text().rsplit(')', 1)[1].split()
            if int(fields[2]) == group and fields[0] != 'Z':
                live.append(int(entry.name))
        except (FileNotFoundError, ProcessLookupError):
            pass
    return live


def signal_group(group, signum):
    try:
        os.killpg(group, signum)
    except ProcessLookupError:
        pass


def run(args):
    received = []
    def interrupted(signum, _frame):
        received.append(signum)
    for signum in (signal.SIGINT, signal.SIGTERM, signal.SIGHUP):
        signal.signal(signum, interrupted)
    with args.log.open('wb') as log:
        child = subprocess.Popen(args.command, stdout=log, stderr=subprocess.STDOUT,
                                 start_new_session=True)
        state = {'pid': child.pid, 'process_group': child.pid, 'stopped': False,
                 'command': args.command, 'returncode': None, 'signal': None}
        try:
            save(args.state, state)
            while child.poll() is None and not received:
                time.sleep(0.05)
        finally:
            if live_group(child.pid):
                signal_group(child.pid, signal.SIGTERM)
                deadline = time.monotonic() + 1
                while live_group(child.pid) and time.monotonic() < deadline:
                    child.poll()
                    time.sleep(0.05)
                if live_group(child.pid):
                    signal_group(child.pid, signal.SIGKILL)
            code = child.wait()
            deadline = time.monotonic() + 1
            while live_group(child.pid) and time.monotonic() < deadline:
                time.sleep(0.05)
        state.update(returncode=code, signal=received[0] if received else None,
                     stopped=not live_group(child.pid))
        save(args.state, state)
        if not state['stopped']:
            return 125
        return 128 + received[0] if received else (code if code >= 0 else 128 - code)


def package(args):
    states = list(args.directory.glob('*/process.json'))
    status = {'writers_stopped': False, 'tar_returncode': None,
              'archive_complete': False}
    save(args.state, status)
    for path in states:
        state = json.loads(path.read_text())
        if not state['stopped'] or live_group(state['process_group']):
            save(args.state, status)
            return 125
    if any(not (db.parent.parent / 'process.json').exists()
           for db in args.directory.glob('*/w2.run/s.sqlite')):
        save(args.state, status)
        return 125
    status['writers_stopped'] = True
    code = subprocess.run(['tar', '-czf', str(args.archive), '-C',
                           str(args.directory.parent), args.directory.name]).returncode
    status.update(tar_returncode=code, archive_complete=code == 0)
    save(args.state, status)
    return code


def main():
    parser = argparse.ArgumentParser()
    commands = parser.add_subparsers(dest='operation', required=True)
    runner = commands.add_parser('run')
    runner.add_argument('--state', type=Path, required=True)
    runner.add_argument('--log', type=Path, required=True)
    runner.add_argument('command', nargs=argparse.REMAINDER)
    packer = commands.add_parser('package')
    packer.add_argument('--state', type=Path, required=True)
    packer.add_argument('--directory', type=Path, required=True)
    packer.add_argument('--archive', type=Path, required=True)
    args = parser.parse_args()
    if args.operation == 'run':
        if args.command[:1] == ['--']:
            args.command.pop(0)
        if not args.command:
            parser.error('run requires a command')
        return run(args)
    return package(args)


if __name__ == '__main__':
    sys.exit(main())
