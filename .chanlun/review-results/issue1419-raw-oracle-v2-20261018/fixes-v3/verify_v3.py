"""#1419 当前修复包静态/原件验证；不启动产品，不写Git。"""
import ast
import hashlib
import json
from pathlib import Path
import subprocess
import sys

ROOT=Path(__file__).resolve().parents[1]
REPO=ROOT.parents[2]
BASE='7cc205c8a83a683fd0ce7c84dfe753a198c5dc92'


def check():
    git=lambda *args:subprocess.check_output(['git',*args],cwd=REPO)
    assert git('rev-parse','HEAD').decode().strip()==BASE
    assert git('branch','--show-current').decode().strip()=='codex/1419-raw-fix-v3-20261018'
    changed=[s for s in git('status','--porcelain','--untracked-files=all','-z').decode().split('\0') if s]
    assert all(s[3:].startswith(str(ROOT.relative_to(REPO))+'/') for s in changed)
    original=json.loads((ROOT/'fixes-v3/V2-MANIFEST.json.reference').read_text())
    preserve=[]
    for entry in original['files']:
        name=entry['path']
        if name.startswith(('raw/','expected/','sources/','validation/')) or name in (
            'RAW-INPUT.json','RAW-INVENTORY.json','INDEPENDENT-WITNESS-LEDGER.json','DEFINITION-REFERENCES.json',
            'S-RESOURCES.reference.json','Q-RESOURCES.reference.json','EXACT-RUN-PROPOSAL.json','EXACT-RUN-PROPOSAL.md'):
            actual=(ROOT/name).read_bytes()
            assert hashlib.sha256(actual).hexdigest()==entry['sha256'],name
            preserve.append(name)
    old=json.loads((ROOT/'EXACT-RUN-PROPOSAL.json').read_text())
    new=json.loads((ROOT/'fixes-v3/EXACT-RUN-PROPOSAL.json').read_text())
    for k in ('authorization','execution_authorized'):assert new[k] is False
    for k in ('source_commit','source_tree','source_manifest_sha256','binary_sha256','frozen_run_rule'):assert new[k]=='PENDING_SOURCE'
    for k in ('existing_caps_reference','proposed_aggregate_caps_not_authorized','hard_limits','commands','input_artifacts','independent_expected','cuts','replay'):
        assert old[k]==new[k],k
    original_matrix=json.loads(git('show',BASE+':'+str((ROOT/'AC-DOMAIN-MATRIX.json').relative_to(REPO))))
    matrix=json.loads((ROOT/'AC-DOMAIN-MATRIX.json').read_text())
    assert {k:matrix[k] for k in original_matrix}==original_matrix
    assert matrix['entire_ticket_complete'] is False
    py=list(ROOT.rglob('*.py'))
    for p in py:ast.parse(p.read_text(),filename=str(p))
    # 入口拒绝必须在真正创建任何产品命令之前发生；这里仅调用Python准备入口。
    result=subprocess.run([sys.executable,'-B',str(ROOT/'driver/fixed_cut_driver.py'),'--contract',str(ROOT/'fixes-v3/EXACT-RUN-PROPOSAL.json'),'--live'],capture_output=True,text=True)
    assert result.returncode!=0 and '禁止执行：PENDING_SOURCE' in result.stderr
    sys.path.insert(0,str(ROOT/'driver'))
    from driver_v2 import reject_live
    try:reject_live()
    except RuntimeError as e:assert 'authorization=false' in str(e)
    else:raise AssertionError('live未拒绝')
    subprocess.run(['git','diff','--check'],cwd=REPO,check=True)
    return dict(status='PASS',head=BASE,git_mutations='NONE',outside_task_directory=[],preserved_original_files=preserve,
                python_ast_files=len(py),signed_ac_quantifiers_unchanged=True,original_caps_unchanged=True,
                live_exit=result.returncode,live_stderr=result.stderr.strip(),product_run='NOT_RUN',authorization=False)


if __name__=='__main__':print(json.dumps(check(),ensure_ascii=False,indent=2))
