"""一次性小范围取证：旧封包、两份 LFS、五处唯一豁免成员。"""
import hashlib
import os
import stat
import tarfile
from common import *


def verify_old_freeze():
    freeze = read_json(FROZEN / 'FREEZE.json')
    records = read_json(FROZEN / 'FILES.json')
    archive = Path(freeze['archive'])
    assert archive == FROZEN / 'reclaim-seal-original-output.pax.tar.gz'
    assert sha(archive) == freeze['sha256'] and archive.stat().st_size == freeze['bytes']
    expected = {OLD.name + '/' + r['path']: r for r in records}
    seen, problems, companions = set(), [], []
    with tarfile.open(archive, 'r:gz') as t:
        for member in t:
            name = member.name.rstrip('/')
            if name == OLD.name:
                continue
            if name not in expected:
                p = Path(name)
                paired = str(p.with_name(p.name[2:])) if p.name.startswith('._') else None
                if member.isfile() and paired in set(expected) | {OLD.name}:
                    raw = t.extractfile(member).read()
                    assert raw[:4] == b'\x00\x05\x16\x07'
                    companions.append({'member': name, 'bytes': len(raw),
                                       'sha256': hashlib.sha256(raw).hexdigest()})
                else:
                    problems.append({'member': name, 'reason': 'unexpected'})
                continue
            rec = expected[name]
            seen.add(name)
            if member.isfile() or member.islnk():
                actual = hashlib.sha256(t.extractfile(member).read()).hexdigest()
                if actual != rec.get('sha256'):
                    problems.append({'member': name, 'reason': 'content'})
            elif member.issym() and member.linkname != rec.get('link'):
                problems.append({'member': name, 'reason': 'symlink'})
    live_mismatches = []
    for rec in records:
        path = OLD / rec['path']
        st = path.lstat()
        if st.st_mode != rec['mode'] or st.st_size != rec['size']:
            live_mismatches.append({'path': rec['path'], 'reason': 'stat'})
        if 'sha256' in rec and sha(path) != rec['sha256']:
            live_mismatches.append({'path': rec['path'], 'reason': 'sha256'})
        if 'link' in rec and os.readlink(path) != rec['link']:
            live_mismatches.append({'path': rec['path'], 'reason': 'symlink'})
    dump(OUT / 'ORIGINAL-FREEZE-VERIFY.json', {
        'freeze': freeze, 'files_manifest_sha256': sha(FROZEN / 'FILES.json'),
        'records': len(records), 'archive_members_matched': len(seen),
        'archive_problems': problems, 'missing': sorted(set(expected) - seen),
        'appledouble_companions': companions,
        'live_mismatches': live_mismatches,
        'payload_verified': not problems and len(seen) == len(expected) and not live_mismatches,
        'complete_base_restore': False,
        'scope': '旧输出仓外包逐成员内容/链接核对；不是原五工位完整恢复，也未实恢复全部Unix属性',
        'deleteAuthorized': False})
    assert not problems and len(seen) == len(expected) and not live_mismatches


def verify_lfs():
    resolutions = {name: read_json(OLD / name / 'CONTENT-RESOLUTION.json') for name in NAMES}
    deps = resolutions['worktree']['lfs_dependencies']
    assert sum(d['size'] for d in deps) == 304471992 and len(deps) == 2
    receipt, total = [], 0
    for dep in deps:
        pointer = git('cat-file', 'blob', dep['git_pointer_oid'])
        assert pointer == (f'version https://git-lfs.github.com/spec/v1\noid sha256:{dep["lfs_sha256"]}\nsize {dep["size"]}\n').encode()
        for name in NAMES:
            other = next(d for d in resolutions[name]['lfs_dependencies'] if d['path'] == dep['path'])
            assert (other['git_pointer_oid'], other['size'], other['lfs_sha256']) == (dep['git_pointer_oid'], dep['size'], dep['lfs_sha256'])
        obj = Path(dep['object_path'])
        fd = os.open(obj, os.O_RDONLY | os.O_NOFOLLOW)
        with os.fdopen(fd, 'rb') as stream:
            before = os.fstat(stream.fileno())
            assert stat.S_ISREG(before.st_mode) and before.st_size == dep['size']
            h, count = hashlib.sha256(), 0
            while chunk := stream.read(min(1024 * 1024, dep['size'] - count)):
                h.update(chunk)
                count += len(chunk)
            after = os.fstat(stream.fileno())
        total += count
        stable = signature(before) == signature(after) == signature(obj.lstat())
        assert stable and count == dep['size'] and h.hexdigest() == dep['lfs_sha256']
        receipt.append(dict(dep, pointer_bytes=len(pointer), pointer_sha256=hashlib.sha256(pointer).hexdigest(),
                            actual_sha256=h.hexdigest(), bytes_read=count, stable=stable,
                            signature=signature(after), stored_lfs_object_bytes_hashed=True,
                            all_five_original_pointer_records_match=True))
    dump(OUT / 'LFS-VERIFY.json', {'objects': receipt, 'bytes_read': total, 'read_limit': 304471992,
         'passes': 2, 'source_worktree_lfs_reread': False, 'copied': False,
         'first_worktree_source_bytes_still_unverified': True, 'deleteAuthorized': False})


def seal_rules():
    lib = archive_library()
    blob = git('cat-file', 'blob', BLOB)
    assert len(blob) == 2888 and hashlib.sha256(blob).hexdigest() == RULE_SHA
    assert hashlib.sha1(b'blob 2888\0' + blob).hexdigest() == BLOB
    (OUT / 'members').mkdir()
    result = []
    for name in NAMES:
        record = next(e for e in read_json(OLD / name / 'INVENTORY.json')['entries'] if e['path'] == RULE)
        assert record['git_expected']['oid'] == BLOB and record['size'] == 2888
        source = PARENT / name / RULE
        fd = os.open(source, os.O_RDONLY | os.O_NOFOLLOW)
        with os.fdopen(fd, 'rb') as f:
            before = os.fstat(f.fileno())
            assert stat.S_ISREG(before.st_mode) and before.st_size == 2888
            data = f.read(2889)
            after = os.fstat(f.fileno())
        assert data == blob and signature(before) == signature(after)
        assert attrs(lib, source) == record['attrs']
        archive = OUT / 'members' / (name + '.pax.tar.gz')
        argv = ['/usr/bin/bsdtar', '--format=pax', '--acls', '--xattrs', '--fflags',
                '--no-recursion', '-czf', str(archive), '-C', str(PARENT / name), RULE]
        p = run(argv)
        with tarfile.open(archive, 'r:gz') as t:
            members = t.getmembers()
            allowed = {RULE, str(Path(RULE).with_name('._secrets.json'))}
            assert {m.name for m in members} <= allowed and all(m.isfile() for m in members)
            member = t.getmember(RULE)
            assert t.extractfile(member).read() == blob
        assert signature(source.lstat()) == signature(after)
        result.append({'candidate': name, 'source': str(source), 'raw_path_hex': os.fsencode(RULE).hex(),
                       'git_blob': BLOB, 'bytes': 2888, 'sha256': RULE_SHA,
                       'archive': str(archive.relative_to(OUT)), 'archive_bytes': archive.stat().st_size,
                       'archive_sha256': sha(archive), 'attrs': record['attrs'],
                       'source_signature': signature(after), 'argv': argv, 'exit_code': p.returncode})
    dump(OUT / 'MEMBERS.json', {'exact_exception_path': RULE, 'exact_exception_blob': BLOB,
         'source_bytes_read': 14440, 'archive_source_bytes_read': 14440,
         'no_other_name_exception': True, 'members': result, 'deleteAuthorized': False})


if __name__ == '__main__':
    verify_old_freeze()
    verify_lfs()
    seal_rules()
    print('旧冻结包、两份LFS、五处精确成员核验完成；不代表完整恢复。')
