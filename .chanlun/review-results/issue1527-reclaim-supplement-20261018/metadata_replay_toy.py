"""验证 COPYFILE_METADATA 不复制正文；只在本目录合成小样执行。"""
import ctypes
import os
from common import *


def main():
    root = OUT / 'metadata-replay-toy'
    root.mkdir()
    src, dst = root / 'metadata-stage', root / 'content-target'
    src.mkdir(); dst.mkdir()
    (src / 'file').write_bytes(b'')
    (dst / 'file').write_bytes(b'must-survive-metadata-replay\n')
    for directory in (src, dst):
        os.symlink('file', directory / 'link')
    os.chmod(src / 'file', 0o751)
    run(['/usr/bin/xattr', '-w', 'user.issue1527', 'synthetic-metadata', str(src / 'file')])
    # 与真实候选同样 ACL为空、fflags=0；不冒充验证未出现的扩展属性语义。
    for rel in ('file', 'link', '.'):
        os.utime(src / rel, ns=(1700000000123456789, 1700000000123456789), follow_symlinks=False)
    lib = archive_library()
    expected = {rel: attrs(lib, src / rel) for rel in ('file', 'link', '.')}
    before = sha(dst / 'file')
    libc = ctypes.CDLL(None, use_errno=True)
    libc.copyfile.argtypes = [ctypes.c_char_p, ctypes.c_char_p, ctypes.c_void_p, ctypes.c_uint32]
    libc.copyfile.restype = ctypes.c_int
    flags = (1 << 0) | (1 << 1) | (1 << 2) | (1 << 18) | (1 << 19)
    for rel in ('file', 'link', '.'):
        rc = libc.copyfile(os.fsencode(src / rel), os.fsencode(dst / rel), None, flags)
        assert rc == 0, (rc, ctypes.get_errno())
        assert attrs(lib, dst / rel) == expected[rel]
    assert sha(dst / 'file') == before and os.readlink(dst / 'link') == 'file'
    dump(OUT / 'METADATA-REPLAY-TOY.json', {
        'passed': True, 'api': 'copyfile', 'flags': flags,
        'flag_names': ['COPYFILE_ACL', 'COPYFILE_STAT', 'COPYFILE_XATTR', 'COPYFILE_NOFOLLOW_SRC', 'COPYFILE_NOFOLLOW_DST'],
        'COPYFILE_DATA': False, 'regular_content_sha256_before': before,
        'regular_content_sha256_after': sha(dst / 'file'),
        'regular_directory_symlink_attrs_match': True,
        'scope': '三类小样，与候选同样ACL空/fflags零；完整base未运行', 'deleteAuthorized': False})
    print('小样证明仅回放metadata可保正文；仍禁止metadata-only包直解完整base。')


if __name__ == '__main__':
    main()
