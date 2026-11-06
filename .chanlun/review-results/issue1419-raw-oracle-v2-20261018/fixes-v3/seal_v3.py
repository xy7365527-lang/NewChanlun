"""只写本票产物清单与完整可审补丁；不写Git、不提交、不改变源/产品授权。"""
import hashlib
import json
from pathlib import Path
import subprocess

from verify_v3 import ROOT,REPO,BASE,check


def digest(b):return hashlib.sha256(b).hexdigest()


def write(path,value):path.write_text(json.dumps(value,ensure_ascii=False,indent=2)+'\n')


def main():
    static=check()
    write(ROOT/'fixes-v3/static-check.stdout',static)
    relative=str(ROOT.relative_to(REPO))
    excluded={'MANIFEST.json','SHA256SUMS','fixes-v3/changes.patch'}
    files=sorted(p for p in ROOT.rglob('*') if p.is_file() and str(p.relative_to(ROOT)) not in excluded)
    assert not any(p.suffix=='.pyc' or '__pycache__' in p.parts for p in files)
    tracked=set(subprocess.check_output(['git','ls-tree','-r','--name-only','-z',BASE,'--',relative],cwd=REPO).decode().split('\0'))
    existing=[str(p.relative_to(REPO)) for p in files if str(p.relative_to(REPO)) in tracked]
    chunks=[subprocess.check_output(['git','diff','--binary',BASE,'--',*existing],cwd=REPO)]
    for p in files:
        rel=str(p.relative_to(REPO))
        if rel in tracked:continue
        result=subprocess.run(['git','diff','--no-index','--binary','--','/dev/null',rel],cwd=REPO,capture_output=True)
        assert result.returncode==1,(rel,result.stderr.decode())
        chunks.append(result.stdout)
    patch=ROOT/'fixes-v3/changes.patch';patch.write_bytes(b''.join(chunks))
    # 逆向只读检查当前候选确实是补丁的after；不触碰index或文件。
    subprocess.run(['git','apply','--check','--reverse',str(patch)],cwd=REPO,check=True)
    entries=[]
    for p in sorted(ROOT.rglob('*')):
        if not p.is_file() or str(p.relative_to(ROOT)) in ('MANIFEST.json','SHA256SUMS'):continue
        b=p.read_bytes();entries.append(dict(path=str(p.relative_to(ROOT)),bytes=len(b),sha256=digest(b)))
    write(ROOT/'MANIFEST.json',dict(schema='issue1419-raw-oracle-v3-manifest/1',ticket=1419,
        parent_task='01a12e01-45a1-7173-a3e8-d338d108fb93',manager='/root/manage_1419_sandcastle',
        worker_identity='Sandcastle外部Codex；不声明原生子代理注册',session_id='由运行时事件登记',
        classification='开放票工作草稿',baseline=BASE,branch='codex/1419-raw-fix-v3-20261018',
        authorization=False,execution_authorized=False,product_run='NOT_RUN',source_tree='PENDING_SOURCE',
        source_commit='PENDING_SOURCE',binary_sha256='PENDING_SOURCE',review_status='PENDING_INDEPENDENT_REVIEW_OF_V3_DIFF',
        prior_manifest='fixes-v3/V2-MANIFEST.json.reference',files=entries,
        exclusions=['MANIFEST.json不自引','SHA256SUMS不自引；覆盖MANIFEST.json'],
        patch={'path':'fixes-v3/changes.patch','sha256':digest(patch.read_bytes()),'scope':'全部新增实体及已有文件修改；不含自身或两个可再生根清单，避免哈希循环','regenerate':'python3 -B fixes-v3/seal_v3.py','reverse_check':'PASS'}))
    checks=[(x['path'],x['sha256']) for x in entries]+[('MANIFEST.json',digest((ROOT/'MANIFEST.json').read_bytes()))]
    (ROOT/'SHA256SUMS').write_text(''.join(f'{h}  {name}\n' for name,h in sorted(checks)))
    # 最终文件集合、字节和SHA256SUMS闭合；不把工具结束以后尚未做的检查写成通过。
    actual={str(p.relative_to(ROOT)) for p in ROOT.rglob('*') if p.is_file()}
    assert actual=={x['path'] for x in entries}|{'MANIFEST.json','SHA256SUMS'}
    for name,h in checks:assert digest((ROOT/name).read_bytes())==h,name
    subprocess.run(['git','diff','--check'],cwd=REPO,check=True)
    print(json.dumps(dict(status='SEALED_LOCAL_REVIEW_CANDIDATE',artifact_files=len(actual),manifest_entries=len(entries),
        sha256_entries=len(checks),manifest_sha256=digest((ROOT/'MANIFEST.json').read_bytes()),
        sha256sums_sha256=digest((ROOT/'SHA256SUMS').read_bytes()),patch_sha256=digest(patch.read_bytes()),
        patch_bytes=patch.stat().st_size,reverse_patch_check='PASS',preserved_original_files=len(static['preserved_original_files']),
        authorization=False,product_run='NOT_RUN',independent_review='PENDING'),ensure_ascii=False,indent=2))


if __name__=='__main__':main()
