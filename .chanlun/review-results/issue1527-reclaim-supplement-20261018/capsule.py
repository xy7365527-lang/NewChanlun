"""本轮报告小包的封存/新目录恢复。封包留在允许写集；仓外复制由经理执行。"""
import argparse
import hashlib
import os
from pathlib import Path
import stat
import tarfile
from common import *

EXCLUDED = {'REPORT-CAPSULE.pax.tar.gz', 'REPORT-CAPSULE.json', 'REPORT-CAPSULE-MEMBERS0',
            'REPORT-CAPSULE-INDEX.json', 'CAPSULE-RESTORE-VERIFY.json', 'capsule-restore'}


def allowed_artifacts():
    rows = []
    lib = archive_library()
    for directory, dirs, files in os.walk(OUT, followlinks=False):
        if Path(directory) == OUT:
            dirs[:] = [d for d in dirs if d not in EXCLUDED]
            files = [f for f in files if f not in EXCLUDED]
        for name in sorted(dirs + files):
            path = Path(directory) / name
            rel = path.relative_to(OUT).as_posix()
            st = path.lstat()
            row = {'path': rel, 'raw_path_hex': os.fsencode(rel).hex(), 'mode': st.st_mode,
                   'bytes': st.st_size, 'attrs': attrs(lib, path)}
            if stat.S_ISREG(st.st_mode):
                row['sha256'] = sha(path)
            elif stat.S_ISLNK(st.st_mode):
                row['link_hex'] = os.fsencode(os.readlink(path)).hex()
            elif not stat.S_ISDIR(st.st_mode):
                raise ValueError('特殊文件不得进入本轮报告包')
            rows.append(row)
    return sorted(rows, key=lambda r: os.fsencode(r['path']))


def pack():
    archive = OUT / 'REPORT-CAPSULE.pax.tar.gz'
    if archive.exists():
        raise ValueError('小包已存在，拒绝覆盖')
    rows = allowed_artifacts()
    dump(OUT / 'REPORT-CAPSULE-INDEX.json', rows)
    listed = [r['path'] for r in rows] + ['REPORT-CAPSULE-INDEX.json']
    names = OUT / 'REPORT-CAPSULE-MEMBERS0'
    names.write_bytes(b'\0'.join(os.fsencode(n) for n in listed) + b'\0')
    argv = ['/usr/bin/bsdtar', '--format=pax', '--acls', '--xattrs', '--fflags', '--no-recursion',
            '-czf', str(archive), '-C', str(OUT), '--null', '-T', str(names)]
    run(argv)
    dump(OUT / 'REPORT-CAPSULE.json', {'archive': archive.name, 'sha256': sha(archive),
         'bytes': archive.stat().st_size, 'indexed_artifacts': len(rows),
         'index_sha256': sha(OUT / 'REPORT-CAPSULE-INDEX.json'),
         'manifest_sha256': sha(OUT / 'MANIFEST.json'), 'argv': argv,
         'outside_checkout_preserved': False,
         'manager_copy_target': str(PARENT / 'reclaim-frozen/supplement-20261018/REPORT-CAPSULE.pax.tar.gz'),
         'root_directory_metadata_excluded': True,
         'scope': '本轮新报告、证物和小样；不包含原完整base、共享Git/LFS',
         'deleteAuthorized': False})
    print('报告小包已封存在允许写集内；尚非仓外保全。')


def restore(archive, expected_sha256, index_sha256, destination):
    archive = archive.resolve(strict=True)
    destination = destination.absolute()
    if os.path.lexists(destination) or destination.parent.resolve() != destination.parent or not destination.parent.is_dir():
        raise ValueError('必须使用无symlink祖先的新目标')
    if sha(archive) != expected_sha256:
        raise ValueError('报告包hash不符')
    with tarfile.open(archive, 'r:gz') as t:
        index = t.extractfile(t.getmember('REPORT-CAPSULE-INDEX.json')).read()
        if hashlib.sha256(index).hexdigest() != index_sha256:
            raise ValueError('报告清单hash不符')
        rows = json.loads(index)
        allowed = {r['path'] for r in rows} | {'REPORT-CAPSULE-INDEX.json'}
        for member in t:
            path = Path(member.name)
            pair = str(path.with_name(path.name[2:])) if path.name.startswith('._') else None
            if path.is_absolute() or '..' in path.parts or (member.name not in allowed and pair not in allowed):
                raise ValueError('越界或非清单成员')
            if member.isdev() or member.isfifo() or member.islnk():
                raise ValueError('非本轮预期成员类型')
            if member.issym():
                record = next(r for r in rows if r['path'] == member.name)
                if os.fsencode(member.linkname).hex() != record.get('link_hex'):
                    raise ValueError('symlink目标不符')
                # 本轮toy只允许同目录 file；不接受链向包外。
                if member.linkname != 'file':
                    raise ValueError('不允许向包外的symlink')
    destination.mkdir(mode=0o700)
    run(['/usr/bin/bsdtar', '--acls', '--xattrs', '--fflags', '--same-owner', '--numeric-owner',
         '-xpf', str(archive), '-C', str(destination)])
    # bytes 清单是唯一匹配判据；规范化仅用于寻找 libarchive 改名后的精确映射。
    # 本报告包的非ASCII名称限已核toy，使用固定已知 NFC/NFD 两种字节映射。
    import unicodedata
    repaired = []
    for row in sorted(rows, key=lambda r: len(Path(r['path']).parts)):
        raw = bytes.fromhex(row['raw_path_hex'])
        parent, name = os.path.split(raw)
        folder = os.path.join(os.fsencode(destination), parent)
        if name not in os.listdir(folder):
            alternate = unicodedata.normalize('NFD', os.fsdecode(name)).encode()
            from unicode_toy import replay_exact_name
            replay_exact_name(folder, alternate, name)
            repaired.append(row['path'])
    # 子文件写入/改名后回放目录mtime；不动新目标根自身的mtime。
    for row in sorted(rows, key=lambda r: len(Path(r['path']).parts), reverse=True):
        if stat.S_ISDIR(row['mode']):
            path = destination / row['path']
            os.utime(path, ns=(path.lstat().st_atime_ns, row['attrs']['mtime_ns']), follow_symlinks=False)
    lib, errors = archive_library(), []
    for row in rows:
        path = destination / row['path']
        if attrs(lib, path) != row['attrs']:
            errors.append({'path': row['path'], 'reason': 'attrs'})
        if 'sha256' in row and sha(path) != row['sha256']:
            errors.append({'path': row['path'], 'reason': 'sha256'})
        if 'link_hex' in row and os.fsencode(os.readlink(path)).hex() != row['link_hex']:
            errors.append({'path': row['path'], 'reason': 'symlink'})
        cursor = os.fsencode(destination)
        for component in bytes.fromhex(row['raw_path_hex']).split(b'/'):
            if component not in os.listdir(cursor):
                errors.append({'path': row['path'], 'reason': 'raw-path'})
                break
            cursor = os.path.join(cursor, component)
    receipt = {'archive_sha256': expected_sha256, 'index_sha256': index_sha256,
               'verified_artifacts': len(rows), 'path_repairs': repaired, 'errors': errors,
               'passed': not errors, 'complete_base_restore': False, 'deleteAuthorized': False}
    dump(destination / 'CAPSULE-RESTORE-RECEIPT.json', receipt)
    if errors:
        raise ValueError('报告恢复验证失败，见目标收据')
    return receipt


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('action', choices=['pack', 'restore'])
    parser.add_argument('--archive', type=Path)
    parser.add_argument('--sha256')
    parser.add_argument('--index-sha256')
    parser.add_argument('--destination', type=Path)
    args = parser.parse_args()
    if args.action == 'pack':
        pack()
    else:
        if not all((args.archive, args.sha256, args.index_sha256, args.destination)):
            parser.error('restore须提供archive、两项hash和destination')
        result = restore(args.archive, args.sha256, args.index_sha256, args.destination)
        print('本轮报告小包内容/属性/原路径字节恢复通过' if result['passed'] else '失败')
