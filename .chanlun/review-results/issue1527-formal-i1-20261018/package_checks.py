"""精确封包与固定基线重建：只用 git 只读操作和临时文件镜像，无 checkout/commit。"""
from pathlib import Path
import hashlib
import json
import os
import subprocess
import tempfile

ROOT=Path(__file__).resolve().parents[3]
OUT=Path(__file__).resolve().parent
BASE='5cb7253d46d973669e7d10cd3331e12c9ef313b1'
def digest(path):return hashlib.sha256(path.read_bytes()).hexdigest()
def main():
    checks=json.loads((OUT/'CHECKS.json').read_text())
    def run(argv,inputs,cwd=ROOT,expected=0):
        fingerprints={str(p):digest(p) for p in inputs}
        result=subprocess.run(argv,cwd=cwd,capture_output=True)
        checks['checks'].append({'argv':list(map(str,argv)),'cwd':str(cwd),'rc':result.returncode,
            'input_sha256':fingerprints,'stdout_sha256':hashlib.sha256(result.stdout).hexdigest(),
            'stderr':result.stderr.decode(errors='replace'),'kind':'封包检查；不是Rust执行'})
        (OUT/'CHECKS.json').write_text(json.dumps(checks,ensure_ascii=False,indent=2)+'\n')
        if result.returncode!=expected:raise RuntimeError((argv,result.returncode,result.stderr.decode()))
        return result.stdout
    tracked=subprocess.check_output(['git','diff','--name-only'],cwd=ROOT,text=True).splitlines()
    untracked=subprocess.check_output(['git','ls-files','--others','--exclude-standard'],cwd=ROOT,text=True).splitlines()
    source=sorted(set(tracked+[p for p in untracked if p.startswith(('rust/','s_session/tests/issue1527_recovery/'))]))
    inputs=[ROOT/p for p in source]
    rust=[str(p) for p in inputs if p.suffix=='.rs']
    run(['rustfmt','--edition','2021','--config','skip_children=true',*rust],inputs)
    run(['rustfmt','--edition','2021','--config','skip_children=true','--check',*rust],inputs)
    run(['python3','-B',str(OUT/'static_checks.py')],inputs+[OUT/'static_checks.py',ROOT.parent/'python-r4-worktree/s_session/recovery_admission.py'])
    run(['git','diff','--check'],inputs)
    diff=run(['git','diff','--binary',BASE,'--',*tracked],inputs)
    for p in source:
        if p not in tracked:
            diff+=run(['git','diff','--no-index','--binary','--','/dev/null',p],[ROOT/p],expected=1)
    patch=OUT/'FINAL.patch';patch.write_bytes(diff)
    with tempfile.TemporaryDirectory(prefix='issue1527-exact-patch-') as tmp:
        mirror=Path(tmp)
        for p in tracked:
            target=mirror/p;target.parent.mkdir(parents=True,exist_ok=True)
            target.write_bytes(subprocess.check_output(['git','show',BASE+':'+p],cwd=ROOT))
        before={p:digest(mirror/p) for p in tracked}
        run(['git','apply','--check',str(patch)],[patch]+[mirror/p for p in tracked],cwd=mirror)
        run(['git','apply',str(patch)],[patch]+[mirror/p for p in tracked],cwd=mirror)
        result={p:{'expected':digest(ROOT/p),'reconstructed':digest(mirror/p)} for p in source}
        assert all(v['expected']==v['reconstructed'] for v in result.values())
        checks=json.loads((OUT/'CHECKS.json').read_text())
        checks['patch_reconstruction']={'base':BASE,'base_file_sha256':before,'files':result,'result':'全部逐字一致','sqlite_or_rust_execution':False}
        checks['latest_status']='受限静态/模型/精确patch检查通过；完整任务BLOCKED'
        (OUT/'CHECKS.json').write_text(json.dumps(checks,ensure_ascii=False,indent=2)+'\n')
    patterns={
        'rust/src/bin/s_structure_session.rs':['fn open_db','fn load_profile','fn cmd_accept','fn cmd_advance','fn cmd_recover','fn run()','fn main()','_admission_open_guard'],
        'rust/src/bin/s_session_v2/mod.rs':['fn serve(','pub fn dispatch','pub fn legacy_write_lock','scope.enter()'],
        'rust/src/bin/s_session_v3/driver.rs':['pub(super) fn open','pub(super) fn recover','AuditConnection::open','scope.enter()'],
        'rust/src/bin/s_session_v3/writer_cli.rs':['pub(super) fn run','fn preflight_input','fn serve(','fn advance(','fn recover(','scope.require_admitted()'],
        'rust/src/bin/s_session_v3/audit_owner.rs':['pub(super) fn startup','load_profile(profile)'],
        'rust/src/bin/s_session_v3/state.rs':['pub(super) fn audit_with_access','fn audit_with_budget'],
        'rust/src/s_content/access.rs':['fn owner_job'],
        'rust/src/bin/s_session_v3/admission.rs':['pub(crate) fn admit','fn startup_gate','pub(crate) fn enter','pub(crate) fn require_current','pub(crate) fn read_startup'],
        'rust/src/bin/s_session_v3/admission_provider.rs':['pub(super) fn measure','pub(super) fn verify_before_open','pub(super) fn verify_runtime'],
        'rust/src/bin/s_session_v3/admission_private/mod.rs':['pub(super) fn validate','D2ProductionCapacityMissing','PRAGMA quick_check'],
    }
    inventory=[]
    for p,queries in patterns.items():
        text=(ROOT/p).read_text();cut=text.find('#[cfg(test)]\nmod tests');text=text if cut<0 else text[:cut]
        for line,value in enumerate(text.splitlines(),1):
            if any(q in value for q in queries):inventory.append({'file':p,'line':line,'text':value.strip(),'file_sha256':digest(ROOT/p)})
    (OUT/'CALL-SURFACE.json').write_text(json.dumps(inventory,ensure_ascii=False,indent=2)+'\n')
    # 自摘要不循环：MANIFEST 不含自身原始字节摘要；其余交付文件都列真实 SHA。
    report_files=[p for p in OUT.iterdir() if p.is_file() and p.name!='MANIFEST.json']
    files={str(p.relative_to(ROOT)):{'sha256':digest(p),'bytes':p.stat().st_size,
        'role':'继承资料，不代表本轮执行' if p.name.startswith('INHERITED-') else '本轮交付'} for p in sorted(set(inputs+report_files))}
    manifest={'issue':1527,'base':BASE,'manager':'/root/manage_1527_sandcastle','parent_thread':'01a12e01-45a1-7173-a3e8-d338d108fb93',
        'status':'blocked','production':'BLOCK_NOT_MERGEABLE','patch':str(patch.relative_to(ROOT)),'patch_sha256':digest(patch),
        'manifest_self_hash':'不含自身以避免递归；其他文件均为原始字节SHA256','files':files,
        'not_run':['cargo','rustc','Rust测试','实际SQLite','正式S/Q','旧binary','历史重型','网络'],
        'open':['D2生产RAM额度来源','写集外重复profile/legacy输入输出/失败stderr计费','私有完整业务审计受限门面','I-2','I-3','I-4','I-5','9011','64GiB/store完整验收','AgentStorage148GiB双跑','金融/隔离/真实恢复']}
    (OUT/'MANIFEST.json').write_text(json.dumps(manifest,ensure_ascii=False,indent=2)+'\n')
    print(json.dumps({'source_files':len(source),'delivery_files':len(files),'patch_sha256':digest(patch),'manifest_sha256':digest(OUT/'MANIFEST.json'),'status':'blocked'},ensure_ascii=False))
if __name__=='__main__':main()
