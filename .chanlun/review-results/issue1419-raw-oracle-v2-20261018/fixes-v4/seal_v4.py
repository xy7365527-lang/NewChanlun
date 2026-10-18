"""#1419 工作草稿封包：只写允许产物；Git调用均只读，补丁仅check不apply。"""
import ast
import hashlib
import json
from pathlib import Path
import subprocess
import sys

HERE=Path(__file__).resolve().parent
ROOT=HERE.parent
REPO=ROOT.parents[2]
BASE='4161ebfe6d1a11b93cccf1de1a8f134c6c7d2c6a'
REL=str(ROOT.relative_to(REPO))
MODIFIED={'driver/content_closure.py','driver/driver_v2.py','driver/fixed_cut_driver.py','driver/test_v2.py'}
EXCLUDE={'MANIFEST.json','SHA256SUMS','fixes-v4/changes.patch'}


def digest(b):return hashlib.sha256(b).hexdigest()
def git(*args):return subprocess.check_output(['git',*args],cwd=REPO)
def write(p,v):p.write_text(json.dumps(v,ensure_ascii=False,indent=2)+'\n')


def check():
    assert git('rev-parse','HEAD').decode().strip()==BASE
    branch=git('branch','--show-current').decode().strip()
    assert branch.startswith('codex/')
    status=[x for x in git('status','--porcelain','--untracked-files=all','-z').decode().split('\0') if x]
    assert all(x[3:].startswith(REL+'/') for x in status),status
    old=json.loads((HERE/'V3-MANIFEST.json.reference').read_text())
    preserved=[]
    for e in old['files']:
        if e['path'] in MODIFIED:continue
        b=(ROOT/e['path']).read_bytes()
        assert len(b)==e['bytes'] and digest(b)==e['sha256'],e['path']
        preserved.append(e)
    fixed=json.loads((HERE/'STATIC-WIRE-SOURCES.json').read_text())
    for s in fixed['sources']:
        b=git('show',s['commit']+':'+s['path'])
        assert len(b)==s['bytes'] and digest(b)==s['sha256']
        for e in s['excerpts']:
            piece=b[e['byte_start_inclusive']:e['byte_end_exclusive']]
            assert piece.decode()==e['text'] and digest(piece)==e['sha256']
            assert piece==b''.join(b.splitlines(keepends=True)[e['first_line']-1:e['last_line']])
    evidence=json.loads((HERE/'REUSED-EVIDENCE.json').read_text())
    for e in evidence['files']:
        b=(HERE/e['copy']).read_bytes();assert len(b)==e['bytes'] and digest(b)==e['sha256']
    checks=json.loads((HERE/'CHECK-OUTPUT.json').read_text())
    assert checks['tests']==22 and checks['failures']==checks['errors']==0
    for name,entry in checks['source_identity'].items():
        b=(ROOT/name).read_bytes();assert digest(b)==entry['sha256'] and len(b)==entry['bytes'],name
    new=json.loads((HERE/'EXACT-RUN-PROPOSAL.json').read_text())
    prior=json.loads((ROOT/'fixes-v3/EXACT-RUN-PROPOSAL.json').read_text())
    for k in ('authorization','execution_authorized'):assert new[k] is False
    for k in ('source_commit','source_tree','source_manifest_sha256','binary_sha256','frozen_run_rule'):assert new[k]=='PENDING_SOURCE'
    for k in ('existing_caps_reference','proposed_aggregate_caps_not_authorized','hard_limits','commands','input_artifacts','independent_expected','cuts','replay','unverified'):
        assert prior[k]==new[k],k
    schema=json.loads((HERE/'DRIVER-CONTRACT.schema.json').read_text())
    assert set(schema['required'])<=new.keys()
    for k,r in schema['properties'].items():assert new[k]==r['const']
    sys.path[:0]=[str(ROOT/'driver'),str(HERE)]
    from fixed_cut_driver import FIXED
    from test_v4 import public_objects
    assert set(schema['$defs']['accepted_binding']['properties']['accepted_cut']['required'])==FIXED
    wire_source=fixed['sources'][0]['excerpts'][0]['text']
    import re
    formal_fields=set(re.findall(r'"([a-z_]+)"',wire_source.split('TYPED_WIRE_FIELDS:')[1]))
    for obj in public_objects():
        assert set(obj)==formal_fields
        assert 'payload_json' not in obj and isinstance(obj['payload']['data'],dict)
    files=[p for p in ROOT.rglob('*') if p.is_file()]
    assert not any(p.suffix=='.pyc' or '__pycache__' in p.parts for p in files)
    for p in files:
        if p.suffix=='.py':ast.parse(p.read_text(),filename=str(p))
        if p.suffix=='.json':json.loads(p.read_text())
    subprocess.run(['git','diff','--check','--',*(REL+'/'+n for n in MODIFIED)],cwd=REPO,check=True)
    return dict(status='PASS_SCOPED_STATIC',head=BASE,branch=branch,preserved=preserved,
                tested_source_identity='CHECK-OUTPUT.json',signed_quantifiers_unchanged=True,
                caps_unchanged=True,public_fixture_fields=sorted(formal_fields),
                authorization=False,product_run='NOT_RUN',independent_review='PENDING')


def main():
    result=check()
    write(HERE/'STATIC-CHECK.json',result)
    baseline_files={p for p in git('ls-tree','-r','--name-only','-z',BASE,'--',REL).decode().split('\0') if p}
    anticipated={str(p.relative_to(ROOT)) for p in ROOT.rglob('*') if p.is_file()}|{'fixes-v4/changes.patch','fixes-v4/SCOPE.json'}
    additions=sorted(n for n in anticipated if REL+'/'+n not in baseline_files)
    assert all(n.startswith('fixes-v4/') for n in additions)
    write(HERE/'SCOPE.json',dict(baseline=BASE,modified=sorted(MODIFIED|{'MANIFEST.json','SHA256SUMS'}),
        added=additions,preserved_from_v3_manifest=len(result['preserved']),
        method='git ls-tree 固定基线 + 当前产物目录 rglob 文件全集 + git status 越界核查',
        patch_exclusions=sorted(EXCLUDE),production_files_modified=[],git_mutations=[],
        product_run='NOT_RUN',review='PENDING_INDEPENDENT_REVIEW_OF_V4_DIFF'))
    files=sorted(p for p in ROOT.rglob('*') if p.is_file() and str(p.relative_to(ROOT)) not in EXCLUDE)
    existing=[str(p.relative_to(REPO)) for p in files if str(p.relative_to(REPO)) in baseline_files]
    chunks=[git('diff','--binary',BASE,'--',*existing)]
    for p in files:
        rel=str(p.relative_to(REPO))
        if rel in baseline_files:continue
        diff=subprocess.run(['git','diff','--no-index','--binary','--','/dev/null',rel],cwd=REPO,capture_output=True)
        assert diff.returncode==1,(rel,diff.stderr.decode());chunks.append(diff.stdout)
    patch=HERE/'changes.patch';patch.write_bytes(b''.join(chunks))
    subprocess.run(['git','apply','--check','--reverse',str(patch)],cwd=REPO,check=True)
    paths={line.split(' b/',1)[1] for line in patch.read_text().splitlines() if line.startswith('diff --git ')}
    assert paths=={REL+'/'+x for x in MODIFIED}|{REL+'/'+x for x in additions if x!='fixes-v4/changes.patch'}
    entries=[]
    for p in sorted(ROOT.rglob('*')):
        if not p.is_file() or p.name in ('MANIFEST.json','SHA256SUMS'):continue
        b=p.read_bytes();entries.append(dict(path=str(p.relative_to(ROOT)),bytes=len(b),sha256=digest(b)))
    write(ROOT/'MANIFEST.json',dict(schema='issue1419-raw-oracle-v4-manifest/1',ticket=1419,
        classification='开放票工作草稿',baseline=BASE,branch=result['branch'],
        parent_task='01a12e01-45a1-7173-a3e8-d338d108fb93',manager='/root/manage_1419_sandcastle',
        worker_identity='Sandcastle外部Codex；不声明原生子代理注册',session_id='由运行时事件登记',
        authorization=False,execution_authorized=False,product_run='NOT_RUN',source_tree='PENDING_SOURCE',
        source_commit='PENDING_SOURCE',binary_sha256='PENDING_SOURCE',review_status='PENDING_INDEPENDENT_REVIEW_OF_V4_DIFF',
        prior_manifest='fixes-v4/V3-MANIFEST.json.reference',files=entries,
        exclusions=['MANIFEST.json不自引','SHA256SUMS不自引；覆盖MANIFEST.json'],
        patch=dict(path='fixes-v4/changes.patch',sha256=digest(patch.read_bytes()),
                   scope='相对固定基线全部新增/修改；排除补丁自身与可再生根MANIFEST.json、SHA256SUMS',
                   regenerate='python3 -B '+REL+'/fixes-v4/seal_v4.py',reverse_check='PASS')))
    sums=[(x['path'],x['sha256']) for x in entries]+[('MANIFEST.json',digest((ROOT/'MANIFEST.json').read_bytes()))]
    (ROOT/'SHA256SUMS').write_text(''.join(f'{h}  {name}\n' for name,h in sorted(sums)))
    actual={str(p.relative_to(ROOT)) for p in ROOT.rglob('*') if p.is_file()}
    assert actual=={x['path'] for x in entries}|{'MANIFEST.json','SHA256SUMS'}
    for name,h in sums:assert digest((ROOT/name).read_bytes())==h
    print(json.dumps(dict(status='SEALED_LOCAL_CANDIDATE',manifest_entries=len(entries),files=len(actual),
        preserved=len(result['preserved']),patch_files=len(paths),patch_bytes=patch.stat().st_size,
        patch_sha256=digest(patch.read_bytes()),manifest_sha256=digest((ROOT/'MANIFEST.json').read_bytes()),
        independent_review='PENDING',product_run='NOT_RUN',authorization=False),ensure_ascii=False,indent=2))


if __name__=='__main__':main()
