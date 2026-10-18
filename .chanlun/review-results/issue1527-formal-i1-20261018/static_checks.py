"""#1527 作者限定静态核验；不是 Rust 类型、借用、ABI 或 SQLite 行为证明。"""
import ast
import hashlib
import json
from pathlib import Path
import re
import subprocess

BASE='5cb7253d46d973669e7d10cd3331e12c9ef313b1'
ROOT=Path(__file__).resolve().parents[3]
R=ROOT/'rust/src/bin/s_session_v3'
def read(path): return path.read_text()
def sha(path): return hashlib.sha256(path.read_bytes()).hexdigest()

def main():
    assert subprocess.check_output(['git','rev-parse','HEAD'],cwd=ROOT,text=True).strip()==BASE
    job=ROOT/'rust/src/s_content/job.rs'
    original=subprocess.check_output(['git','show',BASE+':rust/src/s_content/job.rs'],cwd=ROOT,text=True)
    s=read(job);a=s.index('    /// #1527');b=s.index('    /// 首次读取开始',a)
    assert s[:a]+s[b:]==original, 'Job 除 new_started 外有变化'
    anchor=read(R/'admission_anchor.rs')
    assert 'job_mut' not in anchor
    assert not re.search(r'pub[^\n]*fn[^\n]*->\s*&(?:\x27\w+\s+)?mut\s+(?:\w+::)*Job\b',anchor)
    assert anchor.count('Job::new_started(')==1
    assert 'job.charge_wire_bytes(self.prelude.wire_bytes)' in anchor
    scope=read(R/'admission.rs')
    assert scope.index('MeasuredDomain::measure') < scope.index('consumed\n            .into_job')
    assert scope.index('admission_private::validate') < scope.index('started.seal()') < scope.index('source.verify_before_open()') < scope.index('inner: Rc::new(Inner')
    assert '.writer.lock' in scope and '.maintenance.lock' in scope and 'file.try_lock()' in scope
    assert 'actual' not in scope or 'charge_wire_bytes(n as u64)' in scope
    provider=read(R/'admission_provider.rs')
    assert '.blocks.checked_mul(512)' in re.sub(r'\s+','',provider)
    assert 'store_bytes: store' in provider and 'staged_bytes: staged' in provider
    assert 'UnknownDatabaseSidecar' in provider and 'SourceMembershipChanged' in provider
    assert 'std::fs::remove_' not in provider
    vfs=read(R/'admission_private/fd_wal.rs')
    assert 'fn pin(file: File)' in vfs and 'main_file: File' in vfs
    assert 'Connection::open_with_flags_and_vfs' in vfs
    assert 'SQLITE_OPEN_READ_ONLY' in vfs and 'sqlite3_vfs_register(&mut *vfs, 0)' in vfs
    assert 'ctx.before_io()' in vfs and 'ctx.after_read(n as u64)' in vfs
    assert 'TRACE_CAPACITY' not in vfs and 'interrupted < 3' not in vfs
    assert 'sqlite3_sourceid()' in vfs
    driver=read(R/'driver.rs').split('#[cfg(test)]\nmod tests')[0]
    for opening in ['super::audit_owner::AuditConnection::open(db)?']:
        for m in re.finditer(re.escape(opening),driver):
            assert 'scope.require_admitted()?;' in driver[m.start()-140:m.start()]
    cli=read(R/'writer_cli.rs').split('#[cfg(test)]\nmod tests')[0]
    assert cli.count('"--admission-job-limits"')==3
    assert 'preflight_input(&options, &clock, envelope, &scope, &frozen_profile)' in cli
    assert 'scope.require_admitted()?;' in cli[:cli.index('Connection::open_with_flags')][-160:]
    assert 'out.emit()?;' in read(R/'mod.rs')
    root=read(ROOT/'rust/src/bin/s_structure_session.rs')
    assert 'fn open_db(db: &Path) -> Result<Connection, String> {\n    v3::admission::require_current(db)?;' in root
    assert '_admission_open_guard' in root
    s2=read(ROOT/'rust/src/bin/s_session_v2/mod.rs')
    assert s2.count('scope.enter()?')==2 and 'pub(crate) fn enter(&self)' in s2
    # Pythonlane 只读 AST 合同交叉核，不导入/执行当前在制品。
    py=ROOT.parent/'python-r4-worktree/s_session/recovery_admission.py'
    tree=ast.parse(read(py))
    constants={}
    for node in tree.body:
        if isinstance(node,ast.Assign) and len(node.targets)==1 and isinstance(node.targets[0],ast.Name):
            v=node.value
            if isinstance(v,ast.Call) and isinstance(v.func,ast.Name) and v.func.id=='frozenset':v=v.args[0]
            try:constants[node.targets[0].id]=ast.literal_eval(v)
            except (ValueError,TypeError):pass
    assert constants['ENV_SHA']=='S_CONTROL_ADMISSION_SHA256'
    assert constants['ENV_IDENTITY']=='S_CONTROL_ADMISSION_IDENTITY'
    assert constants['PUBLISHED']=='s-admission' and constants['PENDING']=='.s-admission-pending'
    for field in constants['BINDING_FIELDS']:assert '"'+field+'"' in anchor
    assert len(constants['LIMIT_FIELDS'])==8 and len(constants['BINDING_FIELDS'])==6 and len(constants['IDENTITY_FIELDS'])==4
    for path in (ROOT/'s_session/tests/issue1527_recovery').glob('*.py'):ast.parse(read(path))
    changed=subprocess.check_output(['git','diff','--name-only'],cwd=ROOT,text=True).splitlines()
    untracked=subprocess.check_output(['git','ls-files','--others','--exclude-standard'],cwd=ROOT,text=True).splitlines()
    allow={'rust/src/bin/s_session_v3/'+x for x in ['admission_anchor.rs','admission.rs','admission_provider.rs','mod.rs','writer_cli.rs','driver.rs']}
    allow.update(['rust/src/s_content/job.rs','rust/src/bin/s_session_v2/mod.rs','rust/src/bin/s_structure_session.rs'])
    prefixes=('rust/src/bin/s_session_v3/admission_private/','s_session/tests/issue1527_recovery/','.chanlun/review-results/issue1527-formal-i1-20261018/')
    assert all(p in allow or p.startswith(prefixes) for p in changed+untracked)
    print(json.dumps({'限定静态检查':'通过','Python合同输入SHA256':sha(py),'Rust类型与执行':'NOT_RUN','SQLite执行':'NOT_RUN','规则完整性与恢复验收':'OPEN'},ensure_ascii=False))
if __name__=='__main__':main()
