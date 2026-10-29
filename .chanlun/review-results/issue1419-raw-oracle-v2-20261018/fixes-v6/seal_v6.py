"""本票v6工作草稿封包；仅只读Git查询与apply --check，不提交或暂存。"""
import ast
import hashlib
import json
from pathlib import Path
import subprocess
import sys
import jsonschema

HERE=Path(__file__).resolve().parent
ROOT=HERE.parent
REPO=ROOT.parents[2]
REL=str(ROOT.relative_to(REPO))
BASE='1cb46982baa0f6ca06cd0884d602bcaaa8395f2e'
MODIFIED={'driver/fixed_cut_driver.py','driver/driver_v2.py','driver/test_v2.py'}
EXCLUDED={'MANIFEST.json','SHA256SUMS','fixes-v6/changes.patch'}


def sha(raw):return hashlib.sha256(raw).hexdigest()
def git(*args):return subprocess.check_output(['git',*args],cwd=REPO)
def write(path,obj):path.write_text(json.dumps(obj,ensure_ascii=False,indent=2)+'\n')


def source_revisions():
    records=[]
    for tag in ('repaired','final','accepted','verified','delivered'):
        report=json.loads((HERE/(tag+'-observations.json')).read_text())
        resolved=[]
        for name,identity in report['source_identity'].items():
            candidates=[ROOT/name]+sorted(HERE.glob(Path(name).name+'.*.reference'))
            found=next((p for p in candidates if sha(p.read_bytes())==identity['sha256']),None)
            if found is not None:source={'path':str(found.relative_to(ROOT))}
            else:
                raw=git('show',BASE+':'+REL+'/'+name)
                assert sha(raw)==identity['sha256'],name
                source={'commit':BASE,'path':REL+'/'+name}
            resolved.append(dict(original_path=name,**identity,resolved_source=source))
        stdout=HERE/('tests.'+tag+'.stdout')
        records.append(dict(run=tag,tests=report['tests'],failures=report['failures'],errors=report['errors'],
                            observations=len(report['observations']),stdout=stdout.name,stdout_sha256=sha(stdout.read_bytes()),sources=resolved))
    initial_sources=[]
    for name in ('driver/fixed_cut_driver.py','driver/driver_v2.py','fixes-v6/test_v6.py'):
        p=HERE/(Path(name).name+'.initial.reference');raw=p.read_bytes()
        initial_sources.append(dict(original_path=name,bytes=len(raw),sha256=sha(raw),resolved_source={'path':str(p.relative_to(ROOT))}))
    records.insert(0,dict(run='initial',tests=10,failures=1,errors=0,stdout='tests.initial.stdout',
                         stdout_sha256=sha((HERE/'tests.initial.stdout').read_bytes()),sources=initial_sources,
                         command='python3 -B -m unittest discover -s '+REL+'/fixes-v6 -p test_v6.py -v',
                         unchanged_dependencies='driver/test_v2.py、content_closure及v4/v5测试均为固定1cb原件；没有保存该轮OBS'))
    write(HERE/'TEST-SOURCE-REVISIONS.json',dict(runs=records,qualification='初跑与中间失败原件保留；delivered为最终受测面；accepted后补排序预收费，verified后将work回报更新到提交及观测复制之后'))
    return records


def check():
    assert git('rev-parse','HEAD').decode().strip()==BASE
    assert git('rev-parse','HEAD^{tree}').decode().strip()=='71c474dcbd39e2cddd649d3f7b39caf5f35e67e3'
    assert git('branch','--show-current').decode().strip()=='codex/1419-raw-fix-v6-20261018'
    states=[x for x in git('status','--porcelain','--untracked-files=all','-z').decode().split('\0') if x]
    for item in states:
        assert item[3:].startswith(REL+'/'),item
        name=item[3:][len(REL)+1:]
        assert name in MODIFIED|EXCLUDED or name.startswith('fixes-v6/'),item
    previous=json.loads((HERE/'V5-MANIFEST.json.reference').read_text())
    assert (HERE/'V5-MANIFEST.json.reference').read_bytes()==git('show',BASE+':'+REL+'/MANIFEST.json')
    assert (HERE/'V5-SHA256SUMS.reference').read_bytes()==git('show',BASE+':'+REL+'/SHA256SUMS')
    preserved=[]
    for entry in previous['files']:
        if entry['path'] in MODIFIED:continue
        raw=(ROOT/entry['path']).read_bytes()
        assert len(raw)==entry['bytes'] and sha(raw)==entry['sha256'],entry['path']
        preserved.append(entry)
    tested=json.loads((HERE/'delivered-observations.json').read_text())
    assert tested['tests']==22 and tested['failures']==tested['errors']==0 and len(tested['observations'])==173
    for name,entry in tested['source_identity'].items():
        raw=(ROOT/name).read_bytes();assert len(raw)==entry['bytes'] and sha(raw)==entry['sha256'],name
    for source in json.loads((HERE/'STATIC-WIRE-SOURCES.json').read_text())['sources']:
        raw=git('show',source['commit']+':'+source['path'])
        assert len(raw)==source['bytes'] and sha(raw)==source['sha256']
        assert raw==git('show','46a11d1cbdeecab3444efbfd324be29a73ecb293:'+source['path'])
        for piece in source['excerpts']:
            fragment=b''.join(raw.splitlines(keepends=True)[piece['first_line']-1:piece['last_line']])
            assert fragment.decode()==piece['text'] and sha(fragment)==piece['sha256']
    for read in json.loads((HERE/'READ-SOURCES.json').read_text())['reads']:
        path=Path(read['source']);path=path if path.is_absolute() else ROOT/path
        raw=path.read_bytes();assert len(raw)==read['bytes'] and sha(raw)==read['sha256']
        if 'copy' in read:assert (HERE/read['copy']).read_bytes()==raw
    proposal=json.loads((HERE/'EXACT-RUN-PROPOSAL.json').read_text())
    prior=json.loads((ROOT/'fixes-v5/EXACT-RUN-PROPOSAL.json').read_text())
    unchanged=('existing_caps_reference','proposed_aggregate_caps_not_authorized','hard_limits','commands',
               'input_artifacts','independent_expected','cuts','replay','unverified','volume_contract',
               'profile','signed_catalog','proposed_nonsecret_environment','environment_rule')
    for key in unchanged:assert proposal[key]==prior[key],key
    for key in ('authorization','execution_authorized'):assert proposal[key] is False
    for key in ('source_commit','source_tree','source_manifest_sha256','binary_sha256','frozen_run_rule'):assert proposal[key]=='PENDING_SOURCE'
    schema=json.loads((HERE/'DRIVER-CONTRACT.schema.json').read_text())
    jsonschema.Draft202012Validator.check_schema(schema)
    validator=jsonschema.Draft202012Validator(schema);validator.validate(proposal)
    for key in ('authorization','execution_authorized','source_commit','source_tree','binary_sha256','product_run'):
        bad=dict(proposal,**{key:True if key in ('authorization','execution_authorized') else '伪造'})
        assert list(validator.iter_errors(bad)),key
    sys.path.insert(0,str(ROOT/'driver'))
    from fixed_cut_driver import FIXED,WATCH_IDENTITY
    contract=proposal['watch_contract']
    assert set(contract['stable_fields'])==set(WATCH_IDENTITY)
    assert sum(len(contract[k]) for k in ('stable_fields','per_generation_fields','read_view_fields'))==len(FIXED)
    assert set(contract['stable_fields']+contract['per_generation_fields']+contract['read_view_fields'])==FIXED
    # 原F1和F5复用不能只看标题：实际核完整文件/函数体。
    for entry in json.loads((HERE/'REUSED-EVIDENCE.json').read_text())['source']:
        raw=(ROOT/entry['path']).read_bytes();old=git('show',BASE+':'+REL+'/'+entry['path'])
        if 'symbol' in entry:
            def segment(raw):
                text=raw.decode();node=next(n for n in ast.parse(text).body if isinstance(n,ast.FunctionDef) and n.name==entry['symbol'])
                return ast.get_source_segment(text,node).encode()
            raw,old=segment(raw),segment(old)
        assert raw==old and sha(raw)==entry['sha256']
    py_count=json_count=0
    for p in ROOT.rglob('*'):
        if not p.is_file():continue
        assert p.suffix!='.pyc' and '__pycache__' not in p.parts,p
        if p.suffix=='.py' or '.py.' in p.name and p.suffix=='.reference':ast.parse(p.read_text(),filename=str(p));py_count+=1
        if p.suffix=='.json' and p.name not in ('STATIC-CHECK.json','SCOPE.json','MANIFEST.json'):
            json.loads(p.read_text());json_count+=1
    subprocess.run(['git','diff','--check'],cwd=REPO,check=True)
    return dict(status='PASS_SCOPED_STATIC',baseline=BASE,preserved_count=len(preserved),preserved=preserved,
                tests=22,observations=173,source_identity='全部最终受测源哈希相符',python_ast=py_count,json_parsed=json_count,
                unchanged_contract_fields=list(unchanged),schema='草案正控及6项授权/来源负控通过',
                source_files=3,source_excerpts=sum(len(s['excerpts']) for s in json.loads((HERE/'STATIC-WIRE-SOURCES.json').read_text())['sources']),
                authorization=False,product_run='NOT_RUN',independent_review='PENDING')


def main():
    revisions=source_revisions()
    checks=check();write(HERE/'STATIC-CHECK.json',checks)
    base_files={x for x in git('ls-tree','-r','--name-only','-z',BASE,'--',REL).decode().split('\0') if x}
    anticipated={str(p.relative_to(ROOT)) for p in ROOT.rglob('*') if p.is_file()}|{'fixes-v6/changes.patch','fixes-v6/SCOPE.json'}
    additions=sorted(name for name in anticipated if REL+'/'+name not in base_files)
    assert all(name.startswith('fixes-v6/') for name in additions)
    write(HERE/'SCOPE.json',dict(baseline=BASE,modified=sorted(MODIFIED|{'MANIFEST.json','SHA256SUMS'}),added=additions,
         patch_exclusions=sorted(EXCLUDED),preserved_old_manifest_entries=checks['preserved_count'],
         method='固定Git ls-tree+产物完整rglob+git status越界核查+旧manifest逐字节核验',production_files_modified=[],
         git_mutations=[],authorization=False,product_run='NOT_RUN',independent_review='PENDING_INDEPENDENT_REVIEW_OF_V6_DIFF'))
    files=sorted(p for p in ROOT.rglob('*') if p.is_file() and str(p.relative_to(ROOT)) not in EXCLUDED)
    existing=[str(p.relative_to(REPO)) for p in files if str(p.relative_to(REPO)) in base_files]
    chunks=[git('diff','--binary',BASE,'--',*existing)]
    for p in files:
        rel=str(p.relative_to(REPO))
        if rel in base_files:continue
        result=subprocess.run(['git','diff','--no-index','--binary','--','/dev/null',rel],cwd=REPO,capture_output=True)
        assert result.returncode==1,(rel,result.stderr.decode());chunks.append(result.stdout)
    patch=HERE/'changes.patch';patch.write_bytes(b''.join(chunks))
    subprocess.run(['git','apply','--check','--reverse',str(patch)],cwd=REPO,check=True)
    paths={line.split(' b/',1)[1] for line in patch.read_text().splitlines() if line.startswith('diff --git ')}
    assert paths=={REL+'/'+n for n in MODIFIED}|{REL+'/'+n for n in additions if n!='fixes-v6/changes.patch'}
    entries=[]
    for p in sorted(ROOT.rglob('*')):
        if not p.is_file() or str(p.relative_to(ROOT)) in {'MANIFEST.json','SHA256SUMS'}:continue
        raw=p.read_bytes();entries.append(dict(path=str(p.relative_to(ROOT)),bytes=len(raw),sha256=sha(raw)))
    write(ROOT/'MANIFEST.json',dict(schema='issue1419-raw-oracle-v6-manifest/1',ticket=1419,classification='开放票工作草稿',baseline=BASE,
         branch='codex/1419-raw-fix-v6-20261018',parent_task='01a12e01-45a1-7173-a3e8-d338d108fb93',manager='/root/manage_1419_sandcastle',
         worker_identity='Sandcastle外部Codex；不声明原生子代理注册',session_id='由运行时事件登记',authorization=False,execution_authorized=False,
         source_commit='PENDING_SOURCE',source_tree='PENDING_SOURCE',binary_sha256='PENDING_SOURCE',product_run='NOT_RUN',
         review_status='PENDING_INDEPENDENT_REVIEW_OF_V6_DIFF',prior_manifest='fixes-v6/V5-MANIFEST.json.reference',files=entries,
         exclusions=['MANIFEST.json不自引','SHA256SUMS不自引；另覆盖MANIFEST.json'],
         patch=dict(path='fixes-v6/changes.patch',sha256=sha(patch.read_bytes()),scope='相对1cb全部新增/修改，排除补丁自身和可再生根清单',
                    regenerate='python3 -B '+REL+'/fixes-v6/seal_v6.py',reverse_check='PASS')))
    sums=[(e['path'],e['sha256']) for e in entries]+[('MANIFEST.json',sha((ROOT/'MANIFEST.json').read_bytes()))]
    (ROOT/'SHA256SUMS').write_text(''.join(f'{h}  {name}\n' for name,h in sorted(sums)))
    actual={str(p.relative_to(ROOT)) for p in ROOT.rglob('*') if p.is_file()}
    assert actual=={e['path'] for e in entries}|{'MANIFEST.json','SHA256SUMS'}
    for name,h in sums:assert sha((ROOT/name).read_bytes())==h
    print(json.dumps(dict(status='SEALED_LOCAL_CANDIDATE',manifest_entries=len(entries),files=len(actual),
          preserved=checks['preserved_count'],patch_files=len(paths),patch_bytes=patch.stat().st_size,
          patch_sha256=sha(patch.read_bytes()),manifest_sha256=sha((ROOT/'MANIFEST.json').read_bytes()),sums_sha256=sha((ROOT/'SHA256SUMS').read_bytes()),
          tests=22,observations=173,authorization=False,independent_review='PENDING',product_run='NOT_RUN'),ensure_ascii=False,indent=2))


if __name__=='__main__':main()
