"""仅恢复五处获准的精确 DevSkim 成员；不能用于完整 base 恢复。"""
import argparse
import hashlib
import os
from pathlib import Path
import tarfile
from common import RULE, BLOB, RULE_SHA, read_json, sha, run, dump, archive_library, attrs


def restore(package, destination):
    package = package.resolve(strict=True)
    destination = destination.absolute()
    if os.path.lexists(destination) or not destination.parent.is_dir() or destination.parent.is_symlink():
        raise ValueError('目标须为已有父目录下的新空路径')
    if destination.parent.resolve() != destination.parent:
        raise ValueError('目标祖先不得经 symlink')
    manifest = read_json(package / 'MEMBERS.json')
    if manifest['exact_exception_path'] != RULE or manifest['exact_exception_blob'] != BLOB:
        raise ValueError('精确路径/blob 豁免不匹配')
    rows = manifest['members']
    expected_names = {'worktree', 'r4-candidate-worktree', 'review-worktree-01',
                      'r4-repair-worktree', 'review-tail-worktree'}
    if len(rows) != 5 or {r['candidate'] for r in rows} != expected_names:
        raise ValueError('只接受原五候选')
    for row in rows:
        relative = 'members/' + row['candidate'] + '.pax.tar.gz'
        archive = package / relative
        if row['archive'] != relative or archive.is_symlink() or sha(archive) != row['archive_sha256']:
            raise ValueError('归档 hash 不符')
        if row['sha256'] != RULE_SHA or row['git_blob'] != BLOB or row['bytes'] != 2888:
            raise ValueError('成员固定身份不符')
        allowed = {RULE, str(Path(RULE).with_name('._secrets.json'))}
        with tarfile.open(archive, 'r:gz') as t:
            members = t.getmembers()
            if len(members) not in (1, 2) or len({m.name for m in members}) != len(members):
                raise ValueError('重复或额外成员')
            if any(m.name not in allowed or not m.isfile() or m.size > 65536 for m in members):
                raise ValueError('越界成员或类型')
            raw = t.extractfile(t.getmember(RULE)).read()
            if len(raw) != 2888 or hashlib.sha256(raw).hexdigest() != RULE_SHA:
                raise ValueError('恢复前内容 hash 不符')
    destination.mkdir(mode=0o700)
    lib, receipts = archive_library(), []
    for row in rows:
        target = destination / row['candidate']
        target.mkdir()
        argv = ['/usr/bin/bsdtar', '--acls', '--xattrs', '--fflags', '--numeric-owner',
                '--same-owner', '-xpf', str(package / row['archive']), '-C', str(target)]
        p = run(argv)
        file = target / RULE
        actual = sha(file)
        actual_attrs = attrs(lib, file)
        assert actual == RULE_SHA and actual_attrs == row['attrs']
        # 原路径用 bytes API 核每级目录名；不以 Unicode 规范化替代。
        cursor = os.fsencode(target)
        for part in os.fsencode(RULE).split(b'/'):
            assert part in os.listdir(cursor)
            cursor = os.path.join(cursor, part)
        receipts.append({'candidate': row['candidate'], 'sha256': actual,
                         'bytes': file.stat().st_size, 'attrs_match': True,
                         'raw_path_bytes_match': True, 'argv': argv, 'exit_code': p.returncode})
    result = {'members': receipts, 'passed': len(receipts) == 5,
              'complete_base_restore': False, 'deleteAuthorized': False}
    dump(destination / 'RESTORE-RECEIPT.json', result)
    return result


if __name__ == '__main__':
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument('--package', type=Path, required=True)
    p.add_argument('--destination', type=Path, required=True)
    args = p.parse_args()
    print('五处精确成员恢复验证通过' if restore(args.package, args.destination)['passed'] else '恢复失败')
