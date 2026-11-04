"""本票工作草稿封包；MANIFEST不自引，SHA256SUMS覆盖MANIFEST但不自引。"""
import json
import re
import subprocess
from pathlib import Path
from raw_oracle import digest
from prepare_candidate import ROOT,write


def main():
    repo=ROOT.parents[2]
    # -z保留中文路径原字节，避免porcelain默认C风格引号转义造成假越界。
    status=subprocess.run(['git','status','--porcelain','--untracked-files=all','-z'],cwd=repo,
                          check=True,capture_output=True).stdout.decode('utf-8').split('\0')
    status=[line for line in status if line]
    expected=str(ROOT.relative_to(repo))+'/'
    assert all(line.startswith('?? ') for line in status),'出现非预期已跟踪修改；保留并报告'
    assert all(line[3:].startswith(expected) for line in status),'任务范围外有差异；保留并报告，不覆盖'
    head=subprocess.run(['git','rev-parse','HEAD'],cwd=repo,check=True,capture_output=True,text=True).stdout.strip()
    assert head=='5cb7253d46d973669e7d10cd3331e12c9ef313b1'
    branch=subprocess.run(['git','branch','--show-current'],cwd=repo,check=True,capture_output=True,text=True).stdout.strip()
    assert branch=='codex/1419-raw-oracle-v2-20261018'
    write('validation/checkout-scope.json',{'head':head,'branch':branch,
          'changed_paths':status,'outside_task_directory':[],'git_mutations':'NONE',
          'production_source_changes':'NONE','other_workers_started':False})
    outputs=['tests.stdout','domain-rerun.stdout','supervisor-rerun.stdout']
    passed=set()
    for name in outputs:
        text=(ROOT/'validation'/name).read_text()
        assert '\nOK\n' in text
        passed.update(re.findall(r'^(test_\w+) .* \.\.\. ok$',text,re.M))
    assert len(passed)==13
    write('validation/VALIDATION-SUMMARY.json',{'unique_new_tests_passed':len(passed),'names':sorted(passed),
          'stdout_files':['validation/'+x for x in outputs],
          'first_run':12,'targeted_domain_recheck':2,'targeted_supervisor_log_preservation_recheck':1,
          'prior_23_tests':'复用原件，未重跑','scope':'纯Python定义模型、假传输、固定toy监督及静态原件检查',
          'product_rust_lean_services_real_db_replay_recovery_browser':'NOT_RUN',
          'independent_review_of_this_bundle':'PENDING_MANAGER_REVIEW'})
    files=[]
    for p in sorted(ROOT.rglob('*')):
        if not p.is_file() or p.name in ('MANIFEST.json','SHA256SUMS'):continue
        b=p.read_bytes();files.append({'path':str(p.relative_to(ROOT)),'bytes':len(b),'sha256':digest(b)})
    write('MANIFEST.json',{'schema':'issue1419-raw-oracle-v2-manifest/1','ticket':1419,
          'parent_task':'01a12e01-45a1-7173-a3e8-d338d108fb93','manager':'/root/manage_1419_sandcastle',
          'worker_identity':'Sandcastle外部Codex；不声明原生子代理注册','session_id':'运行时事件登记',
          'classification':'开放票工作草稿','authorization':False,'product_run':'NOT_RUN',
          'source_tree':'PENDING_SOURCE','source_commit':'PENDING_SOURCE','binary_sha256':'PENDING_SOURCE',
          'files':files,'exclusions':['MANIFEST.json不自引','SHA256SUMS不自引；其覆盖MANIFEST.json']})
    manifest=ROOT/'MANIFEST.json'
    checks=[(x['path'],x['sha256']) for x in files]+[('MANIFEST.json',digest(manifest.read_bytes()))]
    (ROOT/'SHA256SUMS').write_text(''.join(f'{h}  {name}\n' for name,h in sorted(checks)))
    print(json.dumps({'artifact_files':len(files)+2,'manifest_entries':len(files),
                      'sha256_entries':len(checks),'manifest_sha256':digest(manifest.read_bytes()),
                      'sha256sums_sha256':digest((ROOT/'SHA256SUMS').read_bytes()),
                      'unique_new_tests':len(passed),'scope_only':True,'product_run':'NOT_RUN'},ensure_ascii=False))


if __name__=='__main__':main()
