#!/usr/bin/env python3
"""#1527 顺序只读封存；只写本报告目录，无 Git 写入与源删除。"""
import base64
import collections
import ctypes as C
import datetime
import hashlib
import json
import os
from pathlib import Path
import re
import shutil
import stat
import subprocess
import sys

OUT = Path(__file__).resolve().parent
ROOT = OUT.parents[2]
PARENT = ROOT.parent
NAMES = ['worktree', 'r4-candidate-worktree', 'review-worktree-01',
         'r4-repair-worktree', 'review-tail-worktree']
EXCLUDED = OUT.relative_to(ROOT).as_posix()
ENV = dict(os.environ, GIT_OPTIONAL_LOCKS='0', GIT_NO_LAZY_FETCH='1',
           GIT_TERMINAL_PROMPT='0', GIT_CONFIG_NOSYSTEM='1')


def dump(path, obj):
    path.write_text(json.dumps(obj, ensure_ascii=False, indent=2) + '\n')


def sha(path):
    h = hashlib.sha256()
    with path.open('rb') as f:
        for chunk in iter(lambda: f.read(1024 * 1024), b''):
            h.update(chunk)
    return h.hexdigest()


def command(argv, cwd=None, check=True, data=None):
    p = subprocess.run(argv, cwd=cwd, env=ENV, input=data, capture_output=True)
    if check and p.returncode:
        raise RuntimeError(f'命令失败 rc={p.returncode} argv={argv!r}; stderr_sha256='
                           + hashlib.sha256(p.stderr).hexdigest())
    return p


def git(root, *args):
    return command(['git', '-C', str(root), *args]).stdout


def suspect(rel):
    for part in Path(rel).parts:
        low = part.lower()
        if (low.startswith('.env') or low in {'.aws', '.ssh', '.gnupg', '.netrc',
                '.npmrc', '.pypirc', 'auth.json', 'credentials', 'credentials.json',
                'id_rsa', 'id_ed25519', 'id_ecdsa', 'secrets.json', 'secrets.yaml',
                'secrets.yml', 'token.json', 'tokens.json'} or
                re.search(r'\.(pem|p12|pfx|key|keychain|keychain-db)$', low)):
            return True
    return False


def walk(root, excluded=None):
    paths = ['.']
    def visit(directory):
        for entry in sorted(os.scandir(directory), key=lambda e: os.fsencode(e.name)):
            p = Path(entry.path)
            rel = p.relative_to(root).as_posix()
            if excluded and (rel == excluded or rel.startswith(excluded + '/')):
                continue
            paths.append(rel)
            if entry.is_dir(follow_symlinks=False):
                visit(p)
    visit(root)
    return paths


class Archive:
    """系统 libarchive 生成无文件内容的 PAX 属性载荷；不跟随 symlink。"""
    def __init__(self):
        self.lib = C.CDLL('/usr/lib/libarchive.dylib')
        self.libc = C.CDLL(None)
        self.libc.free.argtypes = [C.c_void_p]
        signatures = {
            'archive_read_disk_new': (C.c_void_p, []),
            'archive_read_disk_set_symlink_physical': (C.c_int, [C.c_void_p]),
            'archive_read_disk_entry_from_file': (C.c_int, [C.c_void_p, C.c_void_p, C.c_int, C.c_void_p]),
            'archive_entry_new': (C.c_void_p, []),
            'archive_entry_free': (None, [C.c_void_p]),
            'archive_entry_set_pathname': (None, [C.c_void_p, C.c_char_p]),
            'archive_entry_set_size': (None, [C.c_void_p, C.c_longlong]),
            'archive_entry_set_mtime': (None, [C.c_void_p, C.c_longlong, C.c_long]),
            'archive_entry_set_hardlink': (None, [C.c_void_p, C.c_char_p]),
            'archive_entry_pathname': (C.c_char_p, [C.c_void_p]),
            'archive_entry_symlink': (C.c_char_p, [C.c_void_p]),
            'archive_entry_hardlink': (C.c_char_p, [C.c_void_p]),
            'archive_entry_mode': (C.c_uint, [C.c_void_p]),
            'archive_entry_uid': (C.c_longlong, [C.c_void_p]),
            'archive_entry_gid': (C.c_longlong, [C.c_void_p]),
            'archive_entry_mtime': (C.c_longlong, [C.c_void_p]),
            'archive_entry_mtime_nsec': (C.c_long, [C.c_void_p]),
            'archive_entry_fflags': (None, [C.c_void_p, C.POINTER(C.c_ulong), C.POINTER(C.c_ulong)]),
            'archive_entry_acl_to_text': (C.c_void_p, [C.c_void_p, C.POINTER(C.c_ssize_t), C.c_int]),
            'archive_entry_acl_reset': (C.c_int, [C.c_void_p, C.c_int]),
            'archive_entry_acl_next': (C.c_int, [C.c_void_p, C.c_int, C.POINTER(C.c_int), C.POINTER(C.c_int), C.POINTER(C.c_int), C.POINTER(C.c_int), C.POINTER(C.c_char_p)]),
            'archive_entry_xattr_reset': (C.c_int, [C.c_void_p]),
            'archive_entry_xattr_next': (C.c_int, [C.c_void_p, C.POINTER(C.c_char_p), C.POINTER(C.c_void_p), C.POINTER(C.c_size_t)]),
            'archive_write_new': (C.c_void_p, []),
            'archive_write_set_format_pax': (C.c_int, [C.c_void_p]),
            'archive_write_add_filter_gzip': (C.c_int, [C.c_void_p]),
            'archive_write_open_filename': (C.c_int, [C.c_void_p, C.c_char_p]),
            'archive_write_header': (C.c_int, [C.c_void_p, C.c_void_p]),
            'archive_write_close': (C.c_int, [C.c_void_p]),
            'archive_write_free': (C.c_int, [C.c_void_p]),
            'archive_read_new': (C.c_void_p, []),
            'archive_read_support_format_all': (C.c_int, [C.c_void_p]),
            'archive_read_support_filter_all': (C.c_int, [C.c_void_p]),
            'archive_read_open_filename': (C.c_int, [C.c_void_p, C.c_char_p, C.c_size_t]),
            'archive_read_next_header': (C.c_int, [C.c_void_p, C.POINTER(C.c_void_p)]),
            'archive_read_free': (C.c_int, [C.c_void_p]),
            'archive_error_string': (C.c_char_p, [C.c_void_p]),
        }
        for name, (restype, args) in signatures.items():
            fn = getattr(self.lib, name)
            fn.restype, fn.argtypes = restype, args
        self.disk = self.lib.archive_read_disk_new()
        self.check(self.lib.archive_read_disk_set_symlink_physical(self.disk), self.disk)

    def check(self, rc, handle):
        if rc != 0:
            err = self.lib.archive_error_string(handle) or b''
            raise RuntimeError(f'libarchive rc={rc}, error_sha256={hashlib.sha256(err).hexdigest()}')

    def entry(self, path):
        e = self.lib.archive_entry_new()
        self.lib.archive_entry_set_pathname(e, os.fsencode(path))
        self.check(self.lib.archive_read_disk_entry_from_file(self.disk, e, -1, None), self.disk)
        return e

    def attrs(self, e):
        l = self.lib
        # ACL 按有序的数值 type/perm/tag/qualifier 比较；PAX 解码会补数值名称。
        acl_entries = []
        for _ in range(l.archive_entry_acl_reset(e, 0x3f00)):
            t, perm, tag, qualifier = C.c_int(), C.c_int(), C.c_int(), C.c_int()
            name = C.c_char_p()
            self.check(l.archive_entry_acl_next(e, 0x3f00, C.byref(t), C.byref(perm),
                C.byref(tag), C.byref(qualifier), C.byref(name)), self.disk)
            acl_entries.append([t.value, perm.value, tag.value, qualifier.value,
                os.fsdecode(name.value) if qualifier.value < 0 and name.value else None])
        acl = json.dumps(acl_entries, separators=(',', ':')).encode()
        xattrs = {}
        count = l.archive_entry_xattr_reset(e)
        for _ in range(count):
            name, value, size = C.c_char_p(), C.c_void_p(), C.c_size_t()
            self.check(l.archive_entry_xattr_next(e, C.byref(name), C.byref(value), C.byref(size)), self.disk)
            xattrs[os.fsdecode(name.value)] = {'size': size.value,
                'sha256': hashlib.sha256(C.string_at(value, size.value)).hexdigest()}
        fs, fc = C.c_ulong(), C.c_ulong()
        l.archive_entry_fflags(e, C.byref(fs), C.byref(fc))
        return {'mode': l.archive_entry_mode(e), 'uid': l.archive_entry_uid(e),
                'gid': l.archive_entry_gid(e),
                'mtime_ns': l.archive_entry_mtime(e) * 10**9 + l.archive_entry_mtime_nsec(e),
                'fflags_set': fs.value, 'fflags_clear': fc.value,
                'symlink': os.fsdecode(l.archive_entry_symlink(e)) if l.archive_entry_symlink(e) else None,
                'acl_entries': len(acl_entries), 'acl_sha256': hashlib.sha256(acl).hexdigest(),
                'xattrs': xattrs}

    def writer(self, path):
        a = self.lib.archive_write_new()
        self.check(self.lib.archive_write_set_format_pax(a), a)
        self.check(self.lib.archive_write_add_filter_gzip(a), a)
        self.check(self.lib.archive_write_open_filename(a, os.fsencode(path)), a)
        return a

    def close_writer(self, a):
        self.check(self.lib.archive_write_close(a), a)
        self.check(self.lib.archive_write_free(a), a)

    def archive_attrs(self, path):
        a = self.lib.archive_read_new()
        self.check(self.lib.archive_read_support_format_all(a), a)
        self.check(self.lib.archive_read_support_filter_all(a), a)
        self.check(self.lib.archive_read_open_filename(a, os.fsencode(path), 10240), a)
        result = {}
        while True:
            e = C.c_void_p()
            rc = self.lib.archive_read_next_header(a, C.byref(e))
            if rc == 1:
                break
            self.check(rc, a)
            name = os.fsdecode(self.lib.archive_entry_pathname(e)).removeprefix('./').rstrip('/') or '.'
            result[name] = self.attrs(e)
        self.check(self.lib.archive_read_free(a), a)
        return result


def read_hash(path, size):
    h = hashlib.sha256()
    g = hashlib.sha1(b'blob ' + str(size).encode() + b'\0')
    fd = os.open(path, os.O_RDONLY | os.O_NOFOLLOW)
    with os.fdopen(fd, 'rb') as f:
        before = os.fstat(f.fileno())
        for data in iter(lambda: f.read(1024 * 1024), b''):
            h.update(data)
            g.update(data)
        after = os.fstat(f.fileno())
    stable = (before.st_ino, before.st_size, before.st_mtime_ns, before.st_ctime_ns) == (
        after.st_ino, after.st_size, after.st_mtime_ns, after.st_ctime_ns)
    return h.hexdigest(), g.hexdigest(), stable


def tar_pack(root, paths, target):
    for rel in paths:
        if suspect(rel):
            raise RuntimeError('凭据类名阻断封包')
    names = target.with_suffix(target.suffix + '.members0')
    names.write_bytes(b'\0'.join(os.fsencode(p) for p in paths) + b'\0')
    argv = ['/usr/bin/bsdtar', '--format=pax', '--acls', '--xattrs', '--fflags',
            '--no-recursion', '-czf', str(target), '-C', str(root), '--null', '-T', str(names)]
    p = command(argv, check=False)
    return {'argv': argv, 'exit_code': p.returncode,
            'stderr_bytes': len(p.stderr), 'stderr_sha256': hashlib.sha256(p.stderr).hexdigest()}


def git_tree(root, head):
    result = {}
    raw = git(root, 'ls-tree', '-rz', '--full-tree', head)
    for line in raw.split(b'\0'):
        if not line:
            continue
        meta, name = line.split(b'\t', 1)
        mode, kind, oid = meta.decode().split()
        result[os.fsdecode(name)] = {'mode': mode, 'type': kind, 'oid': oid}
    return raw, result


def seal_one(name, lib):
    root, dest = PARENT / name, OUT / name
    dest.mkdir(exist_ok=True)
    if (dest / 'SEAL.json').exists():
        raise RuntimeError('已有封存收据，禁止覆盖或重复全量读')
    head = git(root, 'rev-parse', 'HEAD').decode().strip()
    branch = git(root, 'symbolic-ref', 'HEAD').decode().strip()
    gd = Path(git(root, 'rev-parse', '--absolute-git-dir').decode().strip())
    common = (root / git(root, 'rev-parse', '--git-common-dir').decode().strip()).resolve()
    index_sha = sha(gd / 'index')
    before_status = git(root, 'status', '--porcelain=v1', '-z', '--untracked-files=all', '--ignored=matching')
    (dest / 'status.before0').write_bytes(before_status)
    tree_raw, tree = git_tree(root, head)
    (dest / 'source-tree0').write_bytes(tree_raw)
    (dest / 'index-stage0').write_bytes(git(root, 'ls-files', '--stage', '-z'))
    (dest / 'index-flags0').write_bytes(git(root, 'ls-files', '-v', '-z'))
    ignored = set(os.fsdecode(p) for p in git(root, 'ls-files', '--others', '--ignored', '--exclude-standard', '-z').split(b'\0') if p)
    all_paths = walk(root, EXCLUDED if name == 'review-tail-worktree' else None)
    blocked_paths = [p for p in all_paths if suspect(p)]
    records, delta, anomalies, hardlinks = [], [], [], collections.defaultdict(list)
    if blocked_paths:
        dump(dest / 'CREDENTIAL-BLOCK.json', {'paths': blocked_paths, 'content_read': False})
    a = lib.writer(dest / 'metadata-only.pax.tar.gz')
    payload_bytes = 0
    # 首次 worktree 全量哈希已完成，但小包限额使进程在写清单前退出。
    # 不重复约1.27 GiB读取；该次未落盘的逐文件哈希不得冒充已验证证据。
    unrepeated_first_pass = name == 'worktree' and (dest / 'FIRST-PASS-FAILURE.json').exists()
    for rel in all_paths:
        path = root if rel == '.' else root / rel
        s = path.lstat()
        e = lib.entry(path)
        lib.lib.archive_entry_set_pathname(e, os.fsencode(rel))
        if name == 'review-tail-worktree' and rel == '.chanlun/review-results':
            pre = json.loads((OUT / 'PRE-OUTPUT-PARENT-METADATA.json').read_text())
            lib.lib.archive_entry_set_mtime(e, pre['mtime_ns'] // 10**9, pre['mtime_ns'] % 10**9)
        attrs = lib.attrs(e)
        r = {'path': rel, 'attrs': attrs, 'size': s.st_size, 'dev': s.st_dev,
             'ino': s.st_ino, 'nlink': s.st_nlink, 'ctime_ns': s.st_ctime_ns,
             'birthtime': s.st_birthtime, 'atime_ns': s.st_atime_ns,
             'category': 'directory' if stat.S_ISDIR(s.st_mode) else
                'tracked' if rel in tree else 'git-pointer' if rel == '.git' else
                'ignored' if rel in ignored else 'untracked'}
        if rel in blocked_paths:
            r['content_blocked'] = True
            anomalies.append({'path': rel, 'reason': '凭据类名：未读取原文字节，完整恢复被阻断'})
            lib.lib.archive_entry_set_size(e, 0)
        elif stat.S_ISREG(s.st_mode) and unrepeated_first_pass and rel in tree:
            r['byte_verification'] = '首轮读取完成但哈希清单未落盘；为遵守6.7GiB限额不重复读，保留缺口'
            lib.lib.archive_entry_set_size(e, 0)
        elif stat.S_ISREG(s.st_mode):
            r['sha256'], r['git_blob_oid'], stable = read_hash(path, s.st_size)
            payload_bytes += s.st_size
            if not stable:
                anomalies.append({'path': rel, 'reason': '读取时变化'})
            if s.st_nlink > 1:
                hardlinks[(s.st_dev, s.st_ino)].append(rel)
            lib.lib.archive_entry_set_size(e, 0)
        elif stat.S_ISLNK(s.st_mode):
            raw = os.fsencode(os.readlink(path))
            r['sha256'] = hashlib.sha256(raw).hexdigest()
            r['git_blob_oid'] = hashlib.sha1(b'blob ' + str(len(raw)).encode() + b'\0' + raw).hexdigest()
        elif not stat.S_ISDIR(s.st_mode):
            anomalies.append({'path': rel, 'reason': '不支持的特殊文件'})
        if not stat.S_ISDIR(s.st_mode):
            expected = tree.get(rel)
            mode = '120000' if stat.S_ISLNK(s.st_mode) else '100755' if s.st_mode & stat.S_IXUSR else '100644'
            r['git_expected'] = expected
            r['matches_source_blob_and_mode'] = bool(expected and expected['oid'] == r.get('git_blob_oid') and expected['mode'] == mode)
            if not r['matches_source_blob_and_mode'] and rel not in blocked_paths and 'byte_verification' not in r:
                delta.append(rel)
        lib.check(lib.lib.archive_write_header(a, e), a)
        lib.lib.archive_entry_free(e)
        records.append(r)
    lib.close_writer(a)
    for key, members in hardlinks.items():
        r = next(r for r in records if r['path'] == members[0])
        if len(members) != r['nlink']:
            anomalies.append({'paths': members, 'reason': '外部或未知 hardlink', 'nlink': r['nlink']})
    actual_paths = set(all_paths)
    deleted = sorted(set(tree) - actual_paths)
    attrs = lib.archive_attrs(dest / 'metadata-only.pax.tar.gz')
    mismatches = [r['path'] for r in records if attrs.get(r['path']) != r['attrs']]
    forced = []
    if name == 'review-tail-worktree':
        forced = ['.chanlun/review-results/issue1527-r4-tail-review-20261018/' + f
                  for f in ['CHECKS.json', 'MANIFEST.json', 'REVIEW.md']]
    members = set(delta + forced)
    for rel in list(members):
        for ancestor in Path(rel).parents:
            members.add(ancestor.as_posix())
    # 空目录同样保留；其余目录属性已进入完整 metadata-only 载荷。
    for r in records:
        if r['category'] == 'directory' and not any(p != r['path'] and
                p.startswith(r['path'] + '/') for p in all_paths):
            members.add(r['path'])
    members = sorted(members, key=lambda p: (len(Path(p).parts), os.fsencode(p)))
    unique_bytes = sum((root / p).lstat().st_size for p in members if stat.S_ISREG((root / p).lstat().st_mode))
    oversized = []
    if unique_bytes > 32 * 1024**2:
        oversized = [p for p in delta if p in tree]
        anomalies.append({'reason': '原文字节与Git对象的差异使包超过32MiB；只封独有新增成果，原件保留',
                          'omitted_tracked_count': len(oversized), 'initial_payload_bytes': unique_bytes})
        members = [p for p in members if p not in oversized]
    dump(dest / 'INVENTORY.json', {'entries': records, 'deleted_tracked': deleted,
                                  'delta': delta, 'archive_members': members,
                                  'unpackaged_tracked': oversized})
    tar_receipt = tar_pack(root, members, dest / 'unique.pax.tar.gz')
    private_paths = walk(gd)
    private_tar = tar_pack(gd, private_paths, dest / 'git-private.pax.tar.gz')
    private_manifest = []
    for rel in private_paths:
        path = gd if rel == '.' else gd / rel
        e = lib.entry(path)
        r = {'path': rel, 'attrs': lib.attrs(e), 'size': path.lstat().st_size}
        if path.is_file() and not path.is_symlink():
            r['sha256'] = sha(path)
        private_manifest.append(r)
        lib.lib.archive_entry_free(e)
    # 无 external diff/textconv/filter/hook；补丁仅辅助，原字节由 PAX 保存。
    for file, args in [('head-to-worktree.patch', ['diff', '--binary', '--no-ext-diff', '--no-textconv', head, '--']),
                       ('head-to-index.patch', ['diff', '--cached', '--binary', '--no-ext-diff', '--no-textconv', head, '--'])]:
        (dest / file).write_bytes(git(root, *args, *[':(exclude)' + p for p in blocked_paths]))
    shared_index = git(root, 'rev-parse', '--shared-index-path').decode().strip()
    if shared_index:
        anomalies.append({'reason': 'split index 外部依赖', 'path': shared_index})
    after_status = git(root, 'status', '--porcelain=v1', '-z', '--untracked-files=all', '--ignored=matching')
    (dest / 'status.after0').write_bytes(after_status)
    stable = head == git(root, 'rev-parse', 'HEAD').decode().strip() and index_sha == sha(gd / 'index')
    changed_meta = []
    for r in records:
        s = (root if r['path'] == '.' else root / r['path']).lstat()
        if (s.st_ino, s.st_size, s.st_mtime_ns, s.st_ctime_ns) != (
                r['ino'], r['size'], r['attrs']['mtime_ns'], r['ctime_ns']):
            if not (name == 'review-tail-worktree' and r['path'] == '.chanlun/review-results'):
                changed_meta.append(r['path'])
    counts = collections.Counter(r['category'] for r in records)
    summary = {'source': str(root), 'head': head, 'preserved_ref': branch,
               'common_git_dir': str(common), 'git_private_dir': str(gd),
               'git_pointer': {'path': str(root / '.git'), 'kind': 'regular-pointer'},
               'shared_index': shared_index or None, 'counts': dict(counts),
               'source_regular_bytes_read': payload_bytes, 'entries': len(records),
               'delta_files': len(delta), 'forced_original_reports': forced,
               'deleted_tracked': deleted, 'metadata_roundtrip_mismatches': mismatches,
               'credential_name_blocks': blocked_paths, 'credential_contents_read': False,
               'unique_payload_bytes': unique_bytes,
               'unpackaged_tracked_count': len(oversized),
               'first_pass_hash_receipt_missing': unrepeated_first_pass,
               'metadata_entry_count': len(attrs), 'source_stable': stable,
               'changed_metadata_during_read': changed_meta, 'anomalies': anomalies,
               'status_stable': before_status == after_status,
               'unique_archive': tar_receipt, 'git_private_archive': private_tar,
               'hardlink_classes': list(hardlinks.values()),
               'index_sha256': index_sha, 'space_after': shutil.disk_usage(OUT)._asdict(),
               'excluded_new_output': EXCLUDED if name == 'review-tail-worktree' else None,
               'complete_source_restore_verified': False, 'deleteAuthorized': False}
    dump(dest / 'GIT-PRIVATE-INVENTORY.json', private_manifest)
    dump(dest / 'SEAL.json', summary)
    print(json.dumps({'source': name, 'entries': len(records), 'read_bytes': payload_bytes,
        'unique_files': len(delta), 'metadata_mismatches': len(mismatches),
        'tar_rc': tar_receipt['exit_code'], 'stable': stable,
        'archive_bytes': sum(p.stat().st_size for p in dest.glob('*.tar.gz'))}), flush=True)
    return summary


def main():
    lib = Archive()
    results = []
    for name in NAMES:
        if shutil.disk_usage(OUT).free < 2 * 1024**3:
            raise RuntimeError('剩余空间低于2 GiB，停止新封包')
        results.append(seal_one(name, lib))
    dump(OUT / 'SEAL-SUMMARIES.json', results)


if __name__ == '__main__':
    main()
