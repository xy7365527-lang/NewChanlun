"""只验本轮增量，不重读两份 LFS 或完整源。"""
import ast
import os
import shutil
from common import *
from restore_members import restore


def main():
    parsed = []
    for path in sorted(OUT.glob('*.py')):
        ast.parse(path.read_text(), filename=str(path))
        parsed.append(path.name)
    lib = archive_library()
    members = read_json(OUT / 'MEMBERS.json')['members']
    verified = []
    for row in members:
        restored = OUT / 'member-restore' / row['candidate'] / RULE
        assert sha(restored) == RULE_SHA and attrs(lib, restored) == row['attrs']
        assert sha(OUT / row['archive']) == row['archive_sha256']
        verified.append(row['candidate'])
    guards = []
    for target in (OUT / 'member-restore', PARENT / 'worktree'):
        try:
            restore(OUT, target)
        except ValueError as exc:
            guards.append({'target': str(target), 'rejected': True, 'reason': str(exc)})
        else:
            raise AssertionError('已有目标未被阻断')
    guard = OUT / 'hash-guard'
    guard.mkdir()
    (guard / 'members').mkdir()
    altered = read_json(OUT / 'MEMBERS.json')
    for row in altered['members']:
        shutil.copyfile(OUT / row['archive'], guard / row['archive'])
    altered['members'][0]['archive_sha256'] = '0' * 64
    dump(guard / 'MEMBERS.json', altered)
    target = OUT / 'guard-must-not-exist'
    try:
        restore(guard, target)
    except ValueError as exc:
        assert not target.exists()
        guards.append({'target': str(target), 'rejected': True, 'reason': str(exc)})
    else:
        raise AssertionError('hash不符未被阻断')
    toy = read_json(OUT / 'UNICODE-TOY.json')
    raw = bytes.fromhex(toy['original_relative_hex'])
    file = os.path.join(os.fsencode(OUT / 'unicode-toy-v3/restored'), raw)
    assert os.path.basename(raw) in os.listdir(os.path.dirname(file))
    assert sha(file) == toy['content_sha256'] and attrs(lib, os.fsdecode(file)) == toy['metadata']
    dump(OUT / 'INCREMENT-VERIFY.json', {'syntax_checked': parsed, 'verified_members': verified,
        'guards': guards, 'unicode_independent_bytes_content_attrs_check': True,
        'metadata_replay_toy': read_json(OUT / 'METADATA-REPLAY-TOY.json')['passed'],
        'lfs_rehashed_again': False, 'full_source_read': False,
        'passed': True, 'deleteAuthorized': False})
    print('增量独立复核、已有目标拒绝及hash破坏拒绝通过；未重复LFS大读。')


if __name__ == '__main__':
    main()
