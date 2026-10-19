"""原路径字节往返小样；所有样本均在补核目录。"""
import hashlib
import os
from pathlib import Path
import tarfile
from common import *


def replay_exact_name(parent, actual, expected):
    """二次 rename 仅适用于新建、受控恢复目录；不碰原源。"""
    parent = os.fsencode(parent)
    names = os.listdir(parent)
    if expected in names:
        return False
    if actual not in names:
        raise ValueError('未找到清单指定的 PAX 路径')
    temp = b'.issue1527-raw-path-stage'
    if temp in names:
        raise ValueError('暂存名已存在')
    os.rename(os.path.join(parent, actual), os.path.join(parent, temp))
    os.rename(os.path.join(parent, temp), os.path.join(parent, expected))
    if expected not in os.listdir(parent):
        raise ValueError('文件系统未保留要求的原路径字节')
    return True


def main():
    toy = OUT / 'unicode-toy-v3'
    toy.mkdir()
    source, restored = toy / 'source', toy / 'restored'
    source.mkdir(); restored.mkdir()
    dependency = read_json(OLD / 'LFS-AND-PATH-DEPENDENCIES.json')['path_encoding'][0]
    raw_relative = bytes.fromhex(dependency['source_path_utf8_hex'])
    raw_name = raw_relative.split(b'/')[-1]
    pax_name = bytes.fromhex(dependency['pax_paths_utf8_hex'][0]).split(b'/')[-1]
    # 实例的整个原始相对路径，而非仅比较显示出来的 basename。
    file = os.path.join(os.fsencode(source), raw_relative)
    os.makedirs(os.path.dirname(file))
    data = b'issue1527 synthetic unicode bytes only\n'
    with open(file, 'xb') as f:
        f.write(data)
    os.chmod(file, 0o750)
    run(['/usr/bin/xattr', '-w', 'user.issue1527', 'synthetic-value', os.fsdecode(file)])
    os.utime(file, ns=(1700000000123456789, 1700000000123456789))
    assert raw_name in os.listdir(os.path.dirname(file))
    lib = archive_library()
    original_attrs = attrs(lib, os.fsdecode(file))
    archive = toy / 'unicode.pax.tar.gz'
    run(['/usr/bin/bsdtar', '--format=pax', '--acls', '--xattrs', '--fflags',
         '-czf', str(archive), '-C', str(source), os.fsdecode(raw_relative)])
    with tarfile.open(archive, 'r:gz') as t:
        member = next(m for m in t if m.isfile() and not Path(m.name).name.startswith('._'))
        archived_raw = os.fsencode(member.name)
    run(['/usr/bin/bsdtar', '--acls', '--xattrs', '--fflags', '--same-owner',
         '--numeric-owner', '-xpf', str(archive), '-C', str(restored)])
    directory = os.path.join(os.fsencode(restored), os.path.dirname(raw_relative))
    before_names = os.listdir(directory)
    repaired = replay_exact_name(directory, pax_name, raw_name)
    target = os.path.join(os.fsencode(restored), raw_relative)
    assert raw_name in os.listdir(directory)
    assert sha(target) == hashlib.sha256(data).hexdigest()
    assert attrs(lib, os.fsdecode(target)) == original_attrs
    # 反例：只能在合成样本上试 metadata-only 解压截断，不接触 base。
    meta = toy / 'metadata-only.pax.tar.gz'
    writer = lib.writer(meta)
    entry = lib.entry(Path(os.fsdecode(file)))
    lib.lib.archive_entry_set_pathname(entry, b'payload')
    lib.lib.archive_entry_set_size(entry, 0)
    lib.check(lib.lib.archive_write_header(writer, entry), writer)
    lib.lib.archive_entry_free(entry)
    lib.close_writer(writer)
    bad = toy / 'truncation-proof'
    bad.mkdir()
    (bad / 'payload').write_bytes(data)
    run(['/usr/bin/bsdtar', '-xpf', str(meta), '-C', str(bad)])
    truncated = (bad / 'payload').stat().st_size == 0
    assert truncated
    dump(OUT / 'UNICODE-TOY.json', {
        'original_relative_hex': raw_relative.hex(), 'archive_relative_hex': archived_raw.hex(),
        'source_basename_hex': raw_name.hex(), 'known_original_pax_basename_hex': pax_name.hex(),
        'restored_basename_before_hex': [n.hex() for n in before_names],
        'restored_basename_after_hex': [n.hex() for n in os.listdir(directory)],
        'two_step_rename_required': repaired, 'exact_bytes_equal': True,
        'content_sha256': sha(target), 'metadata_match': True, 'metadata': original_attrs,
        'archive_sha256': sha(archive), 'metadata_only_would_truncate': truncated,
        'metadata_only_extract_to_base_forbidden': True,
        'scope': '合成正文和属性的原路径字节验证；未恢复真实完整base',
        'complete_base_restore': False, 'deleteAuthorized': False})
    print('Unicode 原路径字节及小样属性恢复通过；metadata-only 截断反例成立。')


if __name__ == '__main__':
    main()
