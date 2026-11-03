from __future__ import annotations

import argparse
import fcntl
import json
import os
from pathlib import Path
import sys
import tempfile

DEFAULT_ROOT = Path('/Volumes/AgentStorage/newchanlun-legacy-drain')


def read_state(root: Path) -> dict:
    try:
        state = json.loads((root / 'state.json').read_text())
    except FileNotFoundError:
        return {'v': 1, 'mode': 'open', 'generation': 0}
    if (not isinstance(state, dict) or state.get('v') != 1
            or state.get('mode') not in ('open', 'draining')
            or type(state.get('generation')) is not int or state['generation'] < 0):
        raise ValueError('unknown drain state; admission denied')
    return state


def write_state(root: Path, state: dict) -> None:
    fd, name = tempfile.mkstemp(dir=root, prefix='.state-')
    try:
        with os.fdopen(fd, 'w') as stream:
            json.dump(state, stream)
            stream.write('\n')
            stream.flush()
            os.fsync(stream.fileno())
        os.replace(name, root / 'state.json')
    finally:
        if os.path.exists(name):
            os.unlink(name)


def open_lock(path: Path):
    if path is None:
        raise ValueError('--path is required')
    fd = os.open(path, os.O_RDWR | os.O_CREAT | os.O_NOFOLLOW, 0o600)
    return os.fdopen(fd, 'r+')


def hold_file(path: Path, nonblock: bool) -> int:
    with open_lock(path) as lock:
        try:
            fcntl.flock(lock, fcntl.LOCK_EX | (fcntl.LOCK_NB if nonblock else 0))
        except BlockingIOError:
            print('busy', flush=True)
            return 75
        print('admitted', flush=True)
        sys.stdin.buffer.read()
    return 0


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument('command', choices=['hold', 'freeze', 'resume', 'status', 'lock'])
    parser.add_argument('--root', type=Path, default=DEFAULT_ROOT)
    parser.add_argument('--generation', type=int)
    # hold：冻结专用准入只认 draining 且代次一致；控制锁先于全局锁取得（唯一锁序）。
    parser.add_argument('--frozen-generation', type=int)
    parser.add_argument('--control', type=Path)
    parser.add_argument('--control-mode', choices=['sh', 'ex'], default='sh')
    # lock：与 drain 状态无关的单一 flock（运行锁、仓库初始化锁、控制命令的排他控制锁）。
    parser.add_argument('--path', type=Path)
    parser.add_argument('--nonblock', action='store_true')
    args = parser.parse_args()
    if args.command == 'lock':
        return hold_file(args.path, args.nonblock)
    root = args.root
    root.mkdir(parents=True, exist_ok=True)
    control = None
    if args.control is not None:
        if args.command != 'hold':
            raise ValueError('--control only applies to hold')
        control = open_lock(args.control)
        fcntl.flock(control, fcntl.LOCK_EX if args.control_mode == 'ex' else fcntl.LOCK_SH)
    with (root / 'admission.lock').open('a') as lock:
        fcntl.flock(lock, fcntl.LOCK_SH if args.command == 'hold' else fcntl.LOCK_EX)
        state = read_state(root)
        if args.command == 'freeze':
            if state['mode'] == 'open':
                state = {'v': 1, 'mode': 'draining', 'generation': state['generation'] + 1}
                write_state(root, state)
        elif args.command == 'resume':
            if args.generation != state['generation']:
                raise ValueError('generation mismatch; not reopening')
            if state['mode'] == 'draining':
                state = {**state, 'mode': 'open'}
                write_state(root, state)
        elif args.command == 'hold':
            if args.frozen_generation is None:
                admitted = state['mode'] == 'open'
            else:
                admitted = state['mode'] == 'draining' and state['generation'] == args.frozen_generation
            if not admitted:
                print('frozen', flush=True)
                print(f"旧入口已冻结或代次不符（drain {state['mode']}/gen{state['generation']}），未新增动作", file=sys.stderr, flush=True)
                return 75
            print('admitted', flush=True)
            sys.stdin.buffer.read()
            return 0
        print(json.dumps(state), flush=True)
    return 0


if __name__ == '__main__':
    try:
        raise SystemExit(main())
    except (OSError, ValueError, json.JSONDecodeError) as error:
        print(f'drain control: {error}', file=sys.stderr)
        raise SystemExit(78)
