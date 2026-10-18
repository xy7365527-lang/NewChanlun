"""#1527 B2/B3 定向静态/小文件验证；不运行 Rust、SQLite 或旧模型。"""
import ast
import hashlib
import json
import os
from pathlib import Path
import subprocess
import tempfile

ROOT = Path(__file__).resolve().parents[3]
OUT = Path(__file__).resolve().parent
SOURCE = '3f79ce63cfddfacc49c4870bf4aeb436c82fa572'
HANDOFF = '813e1f92b3da7cf3780ab4886365d2fb2c6bf933'
R = Path('rust/src/bin/s_session_v3')
FROZEN = Path('/Volumes/AgentStorage/issue1488-ticket-manager-20261018/budget-handoff-frozen')
HANDOFF_FILES = {}
HANDOFF_CHECKS = []
PRODUCTS = [
    'rust/src/bin/s_session_v2/mod.rs',
    str(R/'admission.rs'), str(R/'admission_anchor.rs'),
    str(R/'admission_private/fd_wal.rs'), str(R/'admission_private/mod.rs'),
    str(R/'audit_owner.rs'), str(R/'state.rs'),
    'rust/src/bin/s_structure_session.rs',
    's_session/tests/issue1527_recovery/provider_tests.rs',
    's_session/tests/issue1527_recovery/b2b3_terminal_tests.rs',
]
SHARED = ['rust/src/bin/s_session_v2/mod.rs', str(R/'audit_owner.rs'),
          str(R/'state.rs'), 'rust/src/bin/s_structure_session.rs']


def git(*args):
    return subprocess.check_output(['git', *args], cwd=ROOT)


def base(path, rev=SOURCE):
    if rev == HANDOFF:
        if not HANDOFF_FILES:
            patch = (FROZEN/'budget-handoff.patch').read_bytes()
            assert sha(patch) == '76bab4f9c68bcf309126a11edbdd5babad3d08333db77827f67aca76cb6d774c'
            frozen_hashes = {f['path']: f['sha256'] for f in json.loads((FROZEN/'FILES.json').read_text())}
            names = [str(R/'state.rs'), str(R/'audit_owner.rs')]
            with tempfile.TemporaryDirectory(prefix='i1527-frozen1488-') as tmp:
                t=Path(tmp)
                for name in names:
                    p=t/name;p.parent.mkdir(parents=True,exist_ok=True)
                    p.write_bytes(git('show','5cb7253d46d973669e7d10cd3331e12c9ef313b1:'+name))
                argv=['git','apply'] + ['--include='+name for name in names] + [str(FROZEN/'budget-handoff.patch')]
                r=subprocess.run(argv,cwd=t,capture_output=True,text=True)
                HANDOFF_CHECKS.append({'argv':argv,'cwd_role':'固定5cb临时文件树','rc':r.returncode,'stderr':r.stderr})
                assert r.returncode==0,r.stderr
                for name in names:
                    data=(t/name).read_bytes()
                    assert sha(data)==frozen_hashes[name], name
                    HANDOFF_FILES[name]=data
        return HANDOFF_FILES[str(path)]
    return git('show', rev+':'+str(path))


def read(path):
    return (ROOT/path).read_bytes()


def sha(data):
    return hashlib.sha256(data).hexdigest()


def static_checks():
    assert git('rev-parse', 'HEAD').decode().strip() == SOURCE
    assert git('rev-parse', 'HEAD^{tree}').decode().strip() == 'f7941291da81370bd31955eecff49887262c43be'
    # Job 完整原文件不变；state 除可见性外整文件不变；本轮不是重写算法。
    assert read('rust/src/s_content/job.rs') == base('rust/src/s_content/job.rs')
    state = read(R/'state.rs')
    assert state.replace(b'pub(super) fn audit_with_budget(', b'fn audit_with_budget(', 1) == base(R/'state.rs')
    def audit_part(data):
        return data[data.index(b'fn audit_with_budget('):data.index(b'fn audit_with_capture(')]
    assert audit_part(state) == audit_part(base(R/'state.rs', HANDOFF))
    owner = read(R/'audit_owner.rs')
    after = b'let (profile_value, profile_id, profile_hash) =\n            super::admission::load_startup_profile(profile)?;'
    before = b'let (profile_value, profile_id, profile_hash) = load_profile(profile)?;'
    assert owner.replace(after, before, 1) == base(R/'audit_owner.rs')
    def startup(data):
        a = data.index(b'    pub(super) fn startup(')
        b = data.index(b'\n    pub(super) fn ', a+1)
        return data[a:b]
    assert startup(base(R/'audit_owner.rs')) == startup(base(R/'audit_owner.rs', HANDOFF))
    anchor = read(R/'admission_anchor.rs').decode()
    assert anchor.count('Job::new_started(') == 1
    assert 'job_mut' not in anchor and 'Deref for Admission' not in anchor
    facade=anchor[anchor.index('pub(crate) struct AdmissionAccounting'):anchor.index('impl AdmissionAccounting')]
    assert 'job: RefMut<' in facade and 'pub(crate) job:' not in facade
    assert 'retain_terminal_job(&job, first);' in anchor
    assert anchor.index('retain_terminal_job(&job, first);') < anchor.index('.charge_wire_bytes(self.prelude.wire_bytes)')
    report = anchor[anchor.index('    pub(crate) fn report('):anchor.index('impl Drop for CommandAccounting')]
    assert report.index('sink.write(') < report.index('job.charge_wire_bytes(n as u64)') < report.index('job.finish_check()')
    assert 'pre_io_check' not in report and 'Job::new' not in report
    root = read('rust/src/bin/s_structure_session.rs').decode()
    assert 'CommandAccounting::begin()' in root and 'accounting.report(&e, &mut std::io::stderr().lock())' in root
    # 准确枚举根写集，任何其它根业务函数的变化均拒绝。
    original = base('rust/src/bin/s_structure_session.rs').decode()
    diff = git('diff', '--unified=0', '--', 'rust/src/bin/s_structure_session.rs').decode()
    allowed_functions = ('load_profile(', 'cmd_accept(', 'cmd_advance(', 'cmd_recover(', 'main(')
    for line in diff.splitlines():
        if line.startswith('@@'):
            assert any(name in line for name in allowed_functions), line
    assert root[root.index('fn verify_reachable_root('):root.index('fn ensure_writable(')] == original[original.index('fn verify_reachable_root('):original.index('fn ensure_writable(')]
    private = read(R/'admission_private/mod.rs').decode()
    production = private.split('#[cfg(test)]')[0]
    assert 'state::audit_with_budget(&tx, Some(deadline), None, Some(account))' in production
    assert 'Access for' not in production and 'owner_job(' not in production
    assert 'RefCell<&\'s mut AdmissionAccounting' in production
    assert 'LegacyAdmissionBudgetAdapterMissing' in production
    assert 'crate::v2::audit_private_startup(&tx)?;' in production
    assert 'D2ProductionCapacityMissing' in production
    vfs = read(R/'admission_private/fd_wal.rs').decode()
    policy = vfs[vfs.index('fn audit_sql_allowed('):vfs.index('unsafe extern "C" fn authorizer(')]
    for denied in ('SQLITE_ATTACH', 'SQLITE_UPDATE', 'SQLITE_INSERT', 'SQLITE_DELETE', 'SQLITE_CREATE_TEMP_TABLE'):
        assert denied not in policy
    assert '_ => false' in policy and 'Some(b"main")' in policy
    assert 'sqlite3_progress_handler(' in vfs and 'ctx.before_io()' in vfs
    assert 'OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX' in vfs
    assert 'ffi::SQLITE_OPEN_URI' in vfs
    # B3 的实际阻点按两份原算法逐字核对，不能把旧摘要装作 CheckedNodeUsage。
    evidence = read('rust/src/bin/s_session_v2/evidence_store.rs')
    assert b'h.update([kind as u8]);\n    h.update(payload);' in evidence
    assert evidence == base('rust/src/bin/s_session_v2/evidence_store.rs')
    assert read(R/'admission_provider.rs') == base(R/'admission_provider.rs')
    ast.parse(Path(__file__).read_text())
    allowed = set(PRODUCTS)
    changed = set(git('diff', '--name-only').decode().splitlines())
    untracked = set(git('ls-files', '--others', '--exclude-standard').decode().splitlines())
    assert changed <= allowed
    assert all(p in allowed or p.startswith(str(OUT.relative_to(ROOT))+'/') for p in untracked)
    return {'原Job与原审计正文': '逐字未变', '1488冻结函数': '逐字兼容',
            '外围实际write接线': '源码检查通过，未运行Rust', '写集': sorted(changed | (untracked & allowed)),
            'S2旧费用缺口': '具名阻断，未签完整准入'}


def short_pipe_probe():
    # OS 小管道观察，只证明 Write 返回量不能以尝试载荷长替代；不是 Rust/SQLite 证据。
    rd, wr = os.pipe()
    try:
        os.set_blocking(wr, False)
        payload = b'x' * (1024 * 1024)
        n = os.write(wr, payload)
        assert 0 < n < len(payload)
        data = bytearray()
        while len(data) < n:
            data.extend(os.read(rd, n-len(data)))
        assert len(data) == n
        return {'尝试字节': len(payload), '实际write返回': n, '实际read收齐': len(data),
                '范围': '一次小管道部分写；不证明Rust实现、OS EINTR或完整账'}
    finally:
        os.close(wr)
        os.close(rd)


def patch_checks():
    # 临时文件树只由固定 Git blob 构建；不用 worktree/index/分支操作。
    import difflib
    pieces = []
    narrow = []
    initial_paths = set(git('ls-tree', '-r', '--name-only', SOURCE).decode().splitlines())
    for name in PRODUCTS:
        old = base(name) if name in initial_paths else b''
        new = read(name)
        patch = ''.join(difflib.unified_diff(old.decode().splitlines(True), new.decode().splitlines(True),
            fromfile='a/'+name if name in initial_paths else '/dev/null', tofile='b/'+name, n=5))
        pieces.append(patch)
        if name in SHARED:
            narrow.append(patch)
    (OUT/'FINAL.patch').write_text(''.join(pieces))
    (OUT/'NARROW-SHARED.patch').write_text(''.join(narrow))
    results = []
    with tempfile.TemporaryDirectory(prefix='i1527-b2b3-patch-') as tmp:
        t = Path(tmp)
        for name in PRODUCTS:
            if name in initial_paths:
                p = t/name
                p.parent.mkdir(parents=True, exist_ok=True)
                p.write_bytes(base(name))
        for mode in ('--check', None):
            argv = ['git', 'apply'] + ([mode] if mode else []) + [str(OUT/'FINAL.patch')]
            r = subprocess.run(argv, cwd=t, capture_output=True, text=True)
            results.append({'argv': argv, 'cwd_role': '固定blob组成的临时文件树', 'rc': r.returncode, 'stderr': r.stderr})
            assert r.returncode == 0, r.stderr
        assert all((t/p).read_bytes() == read(p) for p in PRODUCTS)
    # 1488 两个共享文件只应用准确窄patch，其余整个文件须逐字保留。
    shared_names = [str(R/'state.rs'), str(R/'audit_owner.rs')]
    with tempfile.TemporaryDirectory(prefix='i1527-1488-narrow-') as tmp:
        t = Path(tmp)
        parts=[]
        for name in shared_names:
            p=t/name;p.parent.mkdir(parents=True,exist_ok=True);p.write_bytes(base(name,HANDOFF))
            parts.extend(difflib.unified_diff(base(name).decode().splitlines(True),read(name).decode().splitlines(True),
                fromfile='a/'+name,tofile='b/'+name,n=5))
        argv = ['git','apply','-']
        r = subprocess.run(argv,input=''.join(parts).encode(),cwd=t,capture_output=True)
        results.append({'argv':argv,'stdin_role':'仅state可见性及startup同账窄patch','cwd_role':'1488固定813e临时文件树','rc':r.returncode,'stderr':r.stderr.decode()})
        assert r.returncode==0,r.stderr
        state_name=shared_names[0]
        assert (t/state_name).read_bytes().replace(b'pub(super) fn audit_with_budget(',b'fn audit_with_budget(',1)==base(state_name,HANDOFF)
        owner_name=shared_names[1]
        assert (t/owner_name).read_bytes().replace(
            b'let (profile_value, profile_id, profile_hash) =\n            super::admission::load_startup_profile(profile)?;',
            b'let (profile_value, profile_id, profile_hash) = load_profile(profile)?;',1)==base(owner_name,HANDOFF)
    return results


def main():
    import sys
    if sys.argv[1:] == ['--final-static-patch']:
        result={'static':static_checks(), 'patch':patch_checks(), 'frozen_1488':HANDOFF_CHECKS}
        (OUT/'FINAL-STATIC-RESULT.json').write_text(json.dumps(result,ensure_ascii=False,indent=2)+'\n')
        print(json.dumps(result,ensure_ascii=False))
        return
    result = {'static': static_checks(), 'small_pipe': short_pipe_probe(), 'patch': patch_checks(), 'frozen_1488': HANDOFF_CHECKS,
              'Rust类型/借用/FFI/测试': 'NOT_RUN', '实际SQLite/正式S/Q': 'NOT_RUN'}
    (OUT/'STATIC-RESULT.json').write_text(json.dumps(result,ensure_ascii=False,indent=2)+'\n')
    print(json.dumps(result,ensure_ascii=False))


if __name__ == '__main__':
    main()
