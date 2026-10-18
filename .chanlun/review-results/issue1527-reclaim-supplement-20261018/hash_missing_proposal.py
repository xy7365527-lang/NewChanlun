"""待根批准后才可运行；只读首工位缺 hash 的 8251 个普通文件。当前未执行。"""
import argparse
import hashlib
import os
import stat
from common import *


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--execute-approved-first-hash', action='store_true')
    args = parser.parse_args()
    if not args.execute_approved_first_hash:
        raise SystemExit('尚未执行；需要根对精确读集/读量另准，参数本身不构成批准。')
    budget = read_json(OUT / 'SOURCE-INTEGRITY-AND-BUDGET.json')['sources'][0]
    assert budget['candidate'] == 'worktree'
    records = budget['missing_hash_files']
    assert len(records) == 8251 and sum(r['bytes'] for r in records) == 1358527370
    target = OUT / 'FIRST-MISSING-HASH-RESULTS.jsonl'
    # 固定输入清单与源HEAD；不扩集、不抓取Git对象、不重读已有hash的源正文。
    assert git('rev-parse', 'HEAD', root=PARENT / 'worktree').decode().strip() == budget['head_expected']
    original = {r['path']: r for r in read_json(OLD / 'worktree/INVENTORY.json')['entries']}
    lfs = {r['path']: r for r in read_json(OLD / 'worktree/CONTENT-RESOLUTION.json')['lfs_dependencies']}
    total = 0
    with target.open('x') as output:
        for record in records:
            rel = Path(record['path'])
            assert not rel.is_absolute() and '..' not in rel.parts and record['path'] != RULE
            # 不处理任何凭据类名；原清单只有一处精确豁免且已从缺hash读集移除。
            if any(p.lower().startswith('.env') or p.lower() in ('credentials', 'auth.json', 'secrets.json', '.ssh', '.aws') for p in rel.parts):
                raise RuntimeError('新可疑路径：停止，禁止扩大豁免')
            path = PARENT / 'worktree' / rel
            fd = os.open(path, os.O_RDONLY | os.O_NOFOLLOW)
            with os.fdopen(fd, 'rb') as stream:
                before = os.fstat(stream.fileno())
                old = original[record['path']]
                assert stat.S_ISREG(before.st_mode)
                assert [before.st_ino, before.st_size, before.st_mtime_ns] == [old['ino'], old['size'], old['attrs']['mtime_ns']]
                digest = hashlib.sha256()
                git_digest = hashlib.sha1(b'blob ' + str(before.st_size).encode() + b'\0')
                count = 0
                while data := stream.read(min(1024 * 1024, before.st_size - count)):
                    digest.update(data); git_digest.update(data); count += len(data)
                after = os.fstat(stream.fileno())
            assert count == before.st_size and signature(before) == signature(after) == signature(path.lstat())
            total += count
            assert total <= 1358527370
            expected = lfs.get(record['path'])
            matches = ((digest.hexdigest(), count) == (expected['lfs_sha256'], expected['size'])
                       if expected else git_digest.hexdigest() == record['git_expected']['oid'])
            output.write(json.dumps({'path': record['path'], 'raw_path_hex': os.fsencode(record['path']).hex(),
                                    'sha256': digest.hexdigest(), 'git_blob_sha1': git_digest.hexdigest(),
                                    'bytes_read': count, 'matches_fixed_source_or_lfs': matches}, ensure_ascii=False) + '\n')
            output.flush()
            if not matches:
                raise RuntimeError('固定source不符，停止；保留增量失败证据，不换判据')
    dump(OUT / 'FIRST-MISSING-HASH-RECEIPT.json', {'count': len(records), 'bytes_read': total,
        'result_sha256': sha(target), 'complete_base_restore': False, 'deleteAuthorized': False})


if __name__ == '__main__':
    main()
