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


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument('command', choices=['hold', 'freeze', 'resume', 'status'])
    parser.add_argument('--root', type=Path, default=DEFAULT_ROOT)
    parser.add_argument('--generation', type=int)
    args = parser.parse_args()
    root = args.root
    root.mkdir(parents=True, exist_ok=True)
    with (root / 'admission.lock').open('a') as lock:
        fcntl.flock(lock, fcntl.LOCK_EX)
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
            if state['mode'] != 'open':
                print('frozen', flush=True)
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
