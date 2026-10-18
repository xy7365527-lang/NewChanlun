"""#1527 仅本轮独占报告封包。不会执行 Rust/SQLite/旧模型或修改 Git。"""
import hashlib
import importlib.util
import json
from pathlib import Path
import subprocess
import sys

OUT = Path(__file__).resolve().parent
ROOT = OUT.parents[2]
spec = importlib.util.spec_from_file_location('b2b3_validation', OUT/'validate.py')
v = importlib.util.module_from_spec(spec)
spec.loader.exec_module(v)


def sha(raw):
    return hashlib.sha256(raw).hexdigest()


def canonical(node):
    return json.dumps(node, ensure_ascii=False, sort_keys=True, separators=(',', ':')).encode()


def run(argv, **kwargs):
    result = subprocess.run(argv, cwd=ROOT, capture_output=True, **kwargs)
    return result, {'argv': argv, 'rc': result.returncode,
                    'stdout': result.stdout.decode(), 'stderr': result.stderr.decode()}


def main():
    checks = json.loads((OUT/'CHECKS.json').read_text())
    checks['checks'].append({'argv':['git','apply',str((OUT/'INITIAL-SHARED.patch').relative_to(ROOT))],
        'rc':0,'kind':'先生成前后文窄patch，再由工具实际应用candidate；记录来自该工具返回'})
    checks['checks'].append({'argv':['python3','-B',str((OUT/'validate.py').relative_to(ROOT))],
        'rc':0,'kind':'先前已实际完成的静态/小管道/固定blob补丁检查',
        'result_file':str((OUT/'STATIC-RESULT.json').relative_to(ROOT)),
        'result_sha256':sha((OUT/'STATIC-RESULT.json').read_bytes())})
    for name in v.PRODUCTS:
        path = ROOT/name
        raw = path.read_bytes()
        argv = ['rustfmt','--edition','2021','--emit','stdout','--config','skip_children=true']
        result, item = run(argv,input=raw)
        item.pop('stdout')
        item.update({'stdin':name,'stdin_sha256':sha(raw),'kind':'最终Rust stdin语法解析（非类型检查）'})
        checks['checks'].append(item)
        if result.returncode:
            (OUT/'CHECKS.json').write_text(json.dumps(checks,ensure_ascii=False,indent=2)+'\n')
            raise SystemExit(result.returncode)
        if result.stdout != raw:
            path.write_bytes(result.stdout)
            item['format_updated']=True
    for argv in [
        ['git','diff','--check'],
        ['python3','-B',str((OUT/'validate.py').relative_to(ROOT)),'--final-static-patch'],
    ]:
        result, item = run(argv)
        checks['checks'].append(item)
        if result.returncode:
            (OUT/'CHECKS.json').write_text(json.dumps(checks,ensure_ascii=False,indent=2)+'\n')
            raise SystemExit(result.returncode)
    historical = Path('.chanlun/review-results/issue1527-formal-i1-20261018')
    inherited=[]
    for name in ['IMPLEMENTATION.md','CHECKS.json','MANIFEST.json']:
        path=historical/name
        raw=(ROOT/path).read_bytes()
        assert raw==v.base(str(path))
        inherited.append({'path':str(path),'sha256':sha(raw),'本轮执行':'未重跑，保留原run04 blocked事实'})
    checks['inherited']=inherited
    checks['scope']={'Rust类型/借用/ABI/测试':'NOT_RUN','实际SQLite':'NOT_RUN',
        '历史28件及六模型':'未重跑','独立评审':'本轮未读取活报告，不认定通过',
        '正式9011/S/Q/历史重型/网络':'NOT_RUN'}
    checks['final_source_hashes']={name:sha((ROOT/name).read_bytes()) for name in v.PRODUCTS}
    checks['frozen_inputs']=[]
    input_paths=[
        ROOT.parent/'ROOT-I1-IMPLEMENTATION-SCOPE.md',
        ROOT.parent/'I1-RESPONSIBILITY-AND-REUSE.md',
        ROOT.parent/'PRIOR-R4-V3-DESIGN-TRANSCRIPT.md',
        ROOT.parent/'xerj-reference-sqlite-query.json',
        ROOT.parent/'review-repair-worktree/.chanlun/review-results/issue1527-r4-repair-review-20261018/REVIEW.md',
        v.FROZEN/'API-HANDOFF.md', v.FROZEN/'FREEZE.json', v.FROZEN/'FILES.json',v.FROZEN/'budget-handoff.patch',
        Path('/Users/silencehan/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/libsqlite3-sys-0.38.2/sqlite3/sqlite3.c'),
        Path('/Users/silencehan/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/rusqlite-0.40.2/src/transaction.rs'),
    ]
    for path in input_paths:
        checks['frozen_inputs'].append({'path':str(path),'sha256':sha(path.read_bytes()),'用途':'只读接口/规范证据，不认作运行通过'})
    (OUT/'CHECKS.json').write_text(json.dumps(checks,ensure_ascii=False,indent=2)+'\n')
    names = sorted(set(v.PRODUCTS + [str(p.relative_to(ROOT)) for p in OUT.iterdir() if p.is_file() and p.name!='MANIFEST.json']))
    files=[]
    for name in names:
        raw=(ROOT/name).read_bytes()
        files.append({'path':name,'bytes':len(raw),'sha256':sha(raw),
            'role':'本轮产品/测试增量' if name in v.PRODUCTS else '本轮独占工作草稿'})
    manifest_path=str((OUT/'MANIFEST.json').relative_to(ROOT))
    manifest={'source':v.SOURCE,'tree':'f7941291da81370bd31955eecff49887262c43be',
        'status':'blocked','qualification':'BLOCK_NOT_MERGEABLE',
        'parent_task':'01a12e01-45a1-7173-a3e8-d338d108fb93','manager':'/root/manage_1527_sandcastle',
        'session_identity':'外部Sandcastle工蜂；真实session_id由运行时登记，未宣称原生子代理',
        'handoff':{'source':v.HANDOFF,'tree':'be64c9dd538e2fef0c59658cc80390a6c2d2048b',
            'method':'本地无813e对象；固定5cb加冻结patch在临时树重建并核FILES SHA，仅两共享文件',
            'qualification':'静态接口核验，不当作独评通过'},
        'write_set':sorted(names+[manifest_path]),'files':files,'inherited':inherited,
        'patches':{'FINAL.patch':'固定3f79以来十个产品/测试文件的完整增量；不递归打包报告',
            'NARROW-SHARED.patch':'根文件五函数、S2委托、state可见性、audit_owner startup；须与FINAL依赖一起合流',
            'INITIAL-SHARED.patch':'修改前先生成并应用的两处共享接口patch'},
        'blockers':['B1：D2生产RAM额度合同未提供；不造schema/默认数值',
            'B3旧域：旧证据图摘要及Snapshot/Delta缺与原Job匹配的计费入口；当前具名拒绝准入',
            'Rust类型/借用/FFI与SQLite实际行为尚未验证；本轮禁止执行'],
        'preserved_obligations':['I1最终验收','完整9011','49GB含WAL布局估算','64GiB/store','AgentStorage148GiB双跑前置',
            'I2全资源及metadata/shared extent峰值','I3来源连续','I4硬IO','I5真实EINTR/取消','金融','隔离','独立双跑','全部六目标'],
        'self_hash_rule':'self_sha256=去除此字段后的UTF8规范JSON(sort_keys,separators=逗号冒号,ensure_ascii=False)；自身不递归进入files'}
    manifest['self_sha256']=sha(canonical(manifest))
    (OUT/'MANIFEST.json').write_text(json.dumps(manifest,ensure_ascii=False,indent=2)+'\n')
    # 写后核验当前全部文件与规范自摘要；成功才返回0。
    loaded=json.loads((OUT/'MANIFEST.json').read_text()); expected=loaded.pop('self_sha256')
    assert expected==sha(canonical(loaded))
    assert all(sha((ROOT/f['path']).read_bytes())==f['sha256'] for f in files)
    assert set(subprocess.check_output(['git','diff','--name-only'],cwd=ROOT,text=True).splitlines()) <= set(v.PRODUCTS)
    print(json.dumps({'封包':'通过','实际产品/测试文件':len(v.PRODUCTS),'完整写集文件':len(manifest['write_set']),
        'MANIFEST实际文件SHA256':sha((OUT/'MANIFEST.json').read_bytes()),
        'FINAL.patch SHA256':sha((OUT/'FINAL.patch').read_bytes()),
        'NARROW-SHARED.patch SHA256':sha((OUT/'NARROW-SHARED.patch').read_bytes()),
        '结论':'BLOCKED，原失败记录保留；未运行Rust/SQLite'},ensure_ascii=False))


if __name__=='__main__':
    main()
