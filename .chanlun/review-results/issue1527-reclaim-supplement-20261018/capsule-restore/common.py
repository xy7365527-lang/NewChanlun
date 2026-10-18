"""#1527 补核工具；不修改原封包、产品和 Git 配置。"""
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import subprocess
import sys

sys.dont_write_bytecode = True
OUT = Path(__file__).resolve().parent
ROOT = OUT.parents[2]
PARENT = ROOT.parent
OLD = PARENT / 'review-tail-worktree/.chanlun/review-results/issue1527-reclaim-seal-20261018'
FROZEN = PARENT / 'reclaim-frozen'
NAMES = ['worktree', 'r4-candidate-worktree', 'review-worktree-01',
         'r4-repair-worktree', 'review-tail-worktree']
RULE = '.chanlun/review-results/issue1371-tb01-b/postmerge-devskim/evidence/rules/secrets.json'
BLOB = 'ed44c86593eefebbc7e1061157628e360fba203c'
RULE_SHA = 'efbb24386803bd7a8af3ddcabfd31bfd02097863592fe2696367bd7b4a3ef85e'
ENV = dict(os.environ, GIT_OPTIONAL_LOCKS='0', GIT_NO_LAZY_FETCH='1',
           GIT_TERMINAL_PROMPT='0', GIT_CONFIG_NOSYSTEM='1',
           GIT_CONFIG_GLOBAL='/dev/null', PYTHONDONTWRITEBYTECODE='1')


def read_json(path):
    return json.loads(Path(path).read_text())


def dump(path, value):
    Path(path).write_text(json.dumps(value, ensure_ascii=False, indent=2) + '\n')


def sha(path):
    h = hashlib.sha256()
    with open(path, 'rb') as stream:
        for data in iter(lambda: stream.read(1024 * 1024), b''):
            h.update(data)
    return h.hexdigest()


def run(argv, check=True, data=None):
    p = subprocess.run(argv, input=data, capture_output=True, env=ENV)
    if check and p.returncode:
        raise RuntimeError(f'命令失败 rc={p.returncode} argv={argv!r}; stderr_sha256='
                           + hashlib.sha256(p.stderr).hexdigest())
    return p


def git(*args, root=ROOT):
    return run(['git', '-C', str(root), *args]).stdout


def archive_library():
    # 冻结旧脚本的逐字副本，仅 import Archive；迁移后不依赖旧 tail 目录。
    reader = OUT / 'original_attribute_reader.py'
    expected = 'b1d0481a79d7ed815e762fbf84dc313a3193d0abcdaa824c955feabd2e99e42d'
    if sha(reader) != expected:
        raise RuntimeError('旧属性读取器 hash 已变化')
    spec = importlib.util.spec_from_file_location('original_seal', reader)
    mod = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(mod)
    return mod.Archive()


def attrs(lib, path):
    entry = lib.entry(Path(path))
    try:
        return lib.attrs(entry)
    finally:
        lib.lib.archive_entry_free(entry)


def signature(st):
    return [st.st_dev, st.st_ino, st.st_size, st.st_mtime_ns, st.st_ctime_ns]
