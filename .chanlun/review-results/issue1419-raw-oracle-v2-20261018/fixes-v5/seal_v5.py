"""#1419 v5封包；只写本票工作草稿，Git全为只读（apply仅--check）。"""
import ast
import hashlib
import json
from pathlib import Path
import subprocess
import sys

import jsonschema

HERE = Path(__file__).resolve().parent
ROOT = HERE.parent
REPO = ROOT.parents[2]
REL = str(ROOT.relative_to(REPO))
BASE = '46a11d1cbdeecab3444efbfd324be29a73ecb293'
MODIFIED = {'driver/content_closure.py', 'driver/driver_v2.py',
            'driver/fixed_cut_driver.py', 'driver/test_v2.py'}
EXCLUDED = {'MANIFEST.json', 'SHA256SUMS', 'fixes-v5/changes.patch'}


def sha(raw):
    return hashlib.sha256(raw).hexdigest()


def git(*args):
    return subprocess.check_output(['git', *args], cwd=REPO)


def write(path, value):
    path.write_text(json.dumps(value, ensure_ascii=False, indent=2)+'\n')


def check():
    assert git('rev-parse', 'HEAD').decode().strip() == BASE
    branch = git('branch', '--show-current').decode().strip()
    assert branch == 'codex/1419-raw-fix-v5-20261018'
    status = [x for x in git('status','--porcelain','--untracked-files=all','-z').decode().split('\0') if x]
    assert all(x[3:].startswith(REL+'/') for x in status), status
    for line in status:
        name = line[3:][len(REL)+1:]
        assert name in MODIFIED | EXCLUDED or name.startswith('fixes-v5/'), name
    # 两份旧清单必须与固定Git原件相等；所有未授权旧实体保持字节不变。
    for name, ref in [('MANIFEST.json','V4-MANIFEST.json.reference'),
                      ('SHA256SUMS','V4-SHA256SUMS.reference')]:
        assert (HERE/ref).read_bytes() == git('show', BASE+':'+REL+'/'+name)
    previous = json.loads((HERE/'V4-MANIFEST.json.reference').read_text())
    preserved = []
    for entry in previous['files']:
        if entry['path'] in MODIFIED:
            continue
        raw = (ROOT/entry['path']).read_bytes()
        assert len(raw) == entry['bytes'] and sha(raw) == entry['sha256'], entry['path']
        preserved.append(entry)
    tested = json.loads((HERE/'verified-observations.json').read_text())
    assert tested['tests'] == 26 and tested['failures'] == tested['errors'] == 0
    assert len(tested['observations']) == 193
    for name, entry in tested['source_identity'].items():
        raw = (ROOT/name).read_bytes()
        assert len(raw) == entry['bytes'] and sha(raw) == entry['sha256'], name
    revisions = json.loads((HERE/'TEST-SOURCE-REVISIONS.json').read_text())
    for run in revisions['runs']:
        for item in run['sources']:
            source = item['resolved_source']
            raw = git('show', source['commit']+':'+source['path']) if 'commit' in source else (ROOT/source['path']).read_bytes()
            assert len(raw) == item['bytes'] and sha(raw) == item['sha256']
    for entry in json.loads((HERE/'REUSED-EVIDENCE.json').read_text())['files']:
        raw = (HERE/entry['copy']).read_bytes()
        assert len(raw) == entry['bytes'] and sha(raw) == entry['sha256']
        assert Path(entry['source']).read_bytes() == raw if Path(entry['source']).is_absolute() else True
    for source in json.loads((HERE/'STATIC-WIRE-SOURCES.json').read_text())['sources']:
        raw = git('show', source['commit']+':'+source['path'])
        assert len(raw) == source['bytes'] and sha(raw) == source['sha256']
        assert git('rev-parse',source['commit']+':'+source['path']).decode().strip() == source['blob']
        for excerpt in source['excerpts']:
            piece = raw[excerpt['byte_start_inclusive']:excerpt['byte_end_exclusive']]
            assert piece.decode() == excerpt['text'] and sha(piece) == excerpt['sha256']
            assert piece == b''.join(raw.splitlines(keepends=True)[excerpt['first_line']-1:excerpt['last_line']])
    for entry in json.loads((HERE/'XERJ-AND-PRIOR-READS.json').read_text())['files']:
        raw = (ROOT/entry['path']).read_bytes()
        assert len(raw) == entry['bytes'] and sha(raw) == entry['sha256']
    proposal = json.loads((HERE/'EXACT-RUN-PROPOSAL.json').read_text())
    prior = json.loads((ROOT/'fixes-v4/EXACT-RUN-PROPOSAL.json').read_text())
    for key in ('authorization','execution_authorized'):
        assert proposal[key] is False
    for key in ('source_commit','source_tree','source_manifest_sha256','binary_sha256','frozen_run_rule'):
        assert proposal[key] == 'PENDING_SOURCE'
    unchanged = ('existing_caps_reference','proposed_aggregate_caps_not_authorized','hard_limits','commands',
                 'input_artifacts','independent_expected','cuts','replay','unverified','volume_contract',
                 'profile','signed_catalog','proposed_nonsecret_environment','environment_rule')
    for key in unchanged:
        assert proposal[key] == prior[key], key
    schema = json.loads((HERE/'DRIVER-CONTRACT.schema.json').read_text())
    jsonschema.Draft202012Validator.check_schema(schema)
    jsonschema.Draft202012Validator(schema).validate(proposal)
    rejected = []
    for key in ('authorization','execution_authorized','source_commit','source_tree','binary_sha256','product_run'):
        bad = dict(proposal, **{key:True if key.endswith('authorized') or key=='authorization' else '伪造'})
        assert list(jsonschema.Draft202012Validator(schema).iter_errors(bad)), key
        rejected.append(key)
    sys.path[:0] = [str(ROOT/'driver'), str(HERE)]
    from fixed_cut_driver import FIXED, WATCH_IDENTITY, CURSOR
    contract = proposal['watch_contract']
    assert set(contract['stable_fields']) == set(WATCH_IDENTITY)
    assert set(contract['stable_fields']).isdisjoint(contract['per_generation_fields'])
    assert set(contract['stable_fields']) | set(contract['per_generation_fields']) == FIXED
    assert set(schema['$defs']['accepted_binding']['properties']['accepted_cut']['required']) == FIXED
    assert set(schema['$defs']['watch_cursor']['required']) == CURSOR
    # 复用F5证据的实际依据：头、Graph构造/解码等旧代码原字节保留。
    path = REL+'/driver/content_closure.py'
    old, new = git('show',BASE+':'+path).decode(), (ROOT/'driver/content_closure.py').read_text()
    def extract(text, name):
        node = next(n for n in ast.parse(text).body if isinstance(n,(ast.FunctionDef,ast.ClassDef)) and n.name==name)
        return ast.get_source_segment(text,node)
    assert all(extract(old,name) == extract(new,name) for name in ('Graph','header_from_raw'))
    files = [p for p in ROOT.rglob('*') if p.is_file()]
    assert not any(p.suffix == '.pyc' or '__pycache__' in p.parts for p in files)
    python_count = json_count = 0
    for p in files:
        if p.suffix == '.py' or '.py.' in p.name and p.suffix == '.reference':
            ast.parse(p.read_text(), filename=str(p)); python_count += 1
        # 本脚本将重新生成这两件；计数只涵盖实际解析的输入，重封保持确定性。
        if p.suffix == '.json' and p not in (HERE/'STATIC-CHECK.json', HERE/'SCOPE.json'):
            json.loads(p.read_text()); json_count += 1
    subprocess.run(['git','diff','--check','--',*(REL+'/'+p for p in MODIFIED)],cwd=REPO,check=True)
    return dict(status='PASS_SCOPED_STATIC', baseline=BASE, branch=branch, preserved=preserved,
                verified_tests=26, observations=193, historical_runs=len(revisions['runs']),
                python_ast_files=python_count, json_parse_files=json_count,
                schema='Draft202012Validator：合同正控及6个授权/来源负控', schema_negatives=rejected,
                unchanged_contract_fields=list(unchanged), caps_unchanged=True,
                reused_graph_header_source_unchanged=True, authorization=False, product_run='NOT_RUN',
                independent_review='PENDING')


def main():
    checks = check()
    write(HERE/'STATIC-CHECK.json', checks)
    base_files = {n for n in git('ls-tree','-r','--name-only','-z',BASE,'--',REL).decode().split('\0') if n}
    anticipated = {str(p.relative_to(ROOT)) for p in ROOT.rglob('*') if p.is_file()} | {'fixes-v5/changes.patch','fixes-v5/SCOPE.json'}
    additions = sorted(n for n in anticipated if REL+'/'+n not in base_files)
    assert all(n.startswith('fixes-v5/') for n in additions)
    write(HERE/'SCOPE.json',dict(baseline=BASE,modified=sorted(MODIFIED | {'MANIFEST.json','SHA256SUMS'}),
          added=additions,patch_exclusions=sorted(EXCLUDED),preserved_old_manifest_entries=len(checks['preserved']),
          method='固定Git ls-tree + 产物目录完整rglob + git status越界核查 + 旧manifest逐字节hash',
          production_files_modified=[],git_mutations=[],authorization=False,
          product_run='NOT_RUN',independent_review='PENDING_INDEPENDENT_REVIEW_OF_V5_DIFF'))
    files = sorted(p for p in ROOT.rglob('*') if p.is_file() and str(p.relative_to(ROOT)) not in EXCLUDED)
    existing = [str(p.relative_to(REPO)) for p in files if str(p.relative_to(REPO)) in base_files]
    chunks = [git('diff','--binary',BASE,'--',*existing)]
    for p in files:
        rel = str(p.relative_to(REPO))
        if rel in base_files:
            continue
        result = subprocess.run(['git','diff','--no-index','--binary','--','/dev/null',rel],cwd=REPO,capture_output=True)
        assert result.returncode == 1, (rel,result.stderr.decode())
        chunks.append(result.stdout)
    patch = HERE/'changes.patch'
    patch.write_bytes(b''.join(chunks))
    subprocess.run(['git','apply','--check','--reverse',str(patch)],cwd=REPO,check=True)
    paths = {line.split(' b/',1)[1] for line in patch.read_text().splitlines() if line.startswith('diff --git ')}
    assert paths == {REL+'/'+p for p in MODIFIED} | {REL+'/'+p for p in additions if p!='fixes-v5/changes.patch'}
    entries = []
    for p in sorted(ROOT.rglob('*')):
        if not p.is_file() or str(p.relative_to(ROOT)) in {'MANIFEST.json','SHA256SUMS'}:
            continue
        raw = p.read_bytes()
        entries.append(dict(path=str(p.relative_to(ROOT)),bytes=len(raw),sha256=sha(raw)))
    write(ROOT/'MANIFEST.json',dict(schema='issue1419-raw-oracle-v5-manifest/1',ticket=1419,
          classification='开放票工作草稿',baseline=BASE,branch=checks['branch'],
          parent_task='01a12e01-45a1-7173-a3e8-d338d108fb93',manager='/root/manage_1419_sandcastle',
          worker_identity='Sandcastle外部Codex；不声明原生子代理注册',session_id='由运行时事件登记',
          authorization=False,execution_authorized=False,source_commit='PENDING_SOURCE',source_tree='PENDING_SOURCE',
          binary_sha256='PENDING_SOURCE',product_run='NOT_RUN',review_status='PENDING_INDEPENDENT_REVIEW_OF_V5_DIFF',
          prior_manifest='fixes-v5/V4-MANIFEST.json.reference',files=entries,
          exclusions=['MANIFEST.json不自引','SHA256SUMS不自引；另覆盖MANIFEST.json'],
          patch=dict(path='fixes-v5/changes.patch',sha256=sha(patch.read_bytes()),
                     scope='相对固定46a全部新增/修改；仅排除补丁自身及可再生根清单',
                     regenerate='python3 -B '+REL+'/fixes-v5/seal_v5.py',reverse_check='PASS')))
    sums = [(entry['path'],entry['sha256']) for entry in entries]+[('MANIFEST.json',sha((ROOT/'MANIFEST.json').read_bytes()))]
    (ROOT/'SHA256SUMS').write_text(''.join(f'{h}  {name}\n' for name,h in sorted(sums)))
    actual = {str(p.relative_to(ROOT)) for p in ROOT.rglob('*') if p.is_file()}
    assert actual == {entry['path'] for entry in entries} | {'MANIFEST.json','SHA256SUMS'}
    for name,h in sums:
        assert sha((ROOT/name).read_bytes()) == h
    print(json.dumps(dict(status='SEALED_LOCAL_CANDIDATE',manifest_entries=len(entries),files=len(actual),
          preserved=len(checks['preserved']),patch_files=len(paths),patch_bytes=patch.stat().st_size,
          patch_sha256=sha(patch.read_bytes()),manifest_sha256=sha((ROOT/'MANIFEST.json').read_bytes()),
          sums_sha256=sha((ROOT/'SHA256SUMS').read_bytes()),authorization=False,
          independent_review='PENDING',product_run='NOT_RUN'),ensure_ascii=False,indent=2))


if __name__ == '__main__':
    main()
