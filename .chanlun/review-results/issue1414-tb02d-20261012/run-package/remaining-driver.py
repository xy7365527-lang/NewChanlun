"""#1414 PR #1477 P2：新字节四臂 fresh（UP/a→UP/b→DOWN/a→DOWN/b），比较 UP/a↔UP/b、DOWN/a↔DOWN/b。

旧首 composite 锚（core c0dbe0db…）只属 787f5cf 字节的历史证据（ROOT-RULING 600207e3…）：
本包不读取、不比较、不绑定它，也不设首 composite 门。根单次授权、写者二进制重建前门与独立包审为前门。
"""
from pathlib import Path
import errno
import hashlib
import importlib.util
import json
import os
import shutil
import signal
import socket
import stat
import subprocess
import sys
import time

FREEZE = Path(__file__).resolve().parent
WANTED = [('second_open_up','a'),('second_open_up','b'),('second_open_down','a'),('second_open_down','b')]
PAIRS = [('second_open_up',0,1),('second_open_down',2,3)]
PACKAGE = ('remaining-driver.py','remaining-harness.py','CURRENT-SOURCES.json','dependency_identity.py')
AUTHORIZATION = 'ROOT-P2-RERUN-AUTHORIZATION.json'
AUTHORIZATION_STATUS = 'ROOT_APPROVED_P2_RERUN_FOUR_FRESH_ONCE'
LIMITS = {'arm':900,'collect_core':120,'frozen_oracle':180,'oracle_node':120,'query':30,'browser':30,'browser_cleanup':5}
FINAL_COMPARE = {'comparator':'tb02c_compare.compare_exact_traces','layout':'remaining-harness.expected_layout(40)',
                 'method':'exact_bytes_and_strict_jsonl','excluded_fields':[],'identity_renaming':[],
                 'pairs':[['second_open_up','a','b'],['second_open_down','a','b']]}
FIRST_FAILURE = 'stop remaining arms; preserve all results; no retry'
LARGE_ARTIFACTS = ('evidence/browser/network.har','session.sqlite','session.sqlite-wal','evidence/database-backup.sqlite')
REBUILD_PASS = 'PASS_REBUILT_WRITER_MATCHES_EXPECTED'

def require(ok, message):
    if not ok:
        raise ValueError(message)

def read(path):
    return json.loads(Path(path).read_text())

def digest(path):
    h = hashlib.sha256()
    with Path(path).open('rb') as f:
        for block in iter(lambda:f.read(1048576),b''):
            h.update(block)
    return h.hexdigest()

def dump(path, value):
    with Path(path).open('x') as f:
        json.dump(value, f, ensure_ascii=False, indent=2)
        f.write('\n')

def binding_mismatches(bindings):
    changed = []
    for path, hashed in bindings.items():
        try:
            if digest(path) != hashed:
                changed.append(path)
        except OSError:
            changed.append(path)
    return changed

def gate_evidence(evidence, case, capture_status):
    capture = read(evidence/'RESULT.json')
    oracle = read(evidence/'query-oracle.json')
    browser = read(evidence/'browser/RESULT.json')
    recovery = read(evidence/'normal-recovery.json')
    require(capture['status'] == capture_status and capture['input_count'] == 40
            and capture['core_records'] == 221 and not capture['cleanup']['errors'], '采集未完整')
    require(oracle['status'] == 'passed' and oracle['case'] == case and oracle['cuts'] == 40
            and set(oracle['oracle_cuts']) == {'15','26','27','31','38','39','40'}, '完整DB/proof七cut oracle未过')
    require(browser['status'] == 'passed' and browser['cuts'] == ['27','40'] and not browser['page_errors'], '真实browser未过')
    require(recovery['equal'] is True and recovery['cuts'] == [27,40], '正常恢复未过')

def verify_dependency_plan(plan):
    # 原 explicit_composite.verify_dependency_plan 的同一判据，迁入本 driver；不再携带首 composite helper。
    from dependency_identity import CHROME_CONTRACT
    dep=plan['dependencies'];chrome=plan.get('declared_chrome_dependency')
    require(type(chrome) is dict and chrome=={'path':dep['browser'],
        'sha256':dep['browser_sha256'],'contract':CHROME_CONTRACT}
        and plan['environment_overrides'].get('TB02D_CHROMIUM_EXECUTABLE')==dep['browser']
        and dep['browser'] not in (dep['python'],plan['declared_python_dependency']['resolved_path'],
                                  dep['node'],str(Path(dep['playwright_module'])/'package.json')),
        '新remaining依赖契约未事前绑定browser/launch且隔离其它角色')

def planned_socket(directory):
    return '/private/tmp/tb1414-'+hashlib.sha256(str(directory).encode()).hexdigest()[:16]+'.sock'

def verify_rerun_plan(plan):
    """事前静态门：四臂顺序、两对比较、原窗、首失败停、无旧首composite绑定、写者二进制期望、确定性资源。"""
    require(plan.get('schema') == 'tb1414-p2-rerun-four-fresh/1', '计划schema不符')
    require(not {'first_composite','first_composite_current_fact'} & set(plan), '旧首composite绑定不得出现在再验收计划')
    require(plan['limits_seconds'] == LIMITS, '窗口改变')
    require([(x['case'],x['arm']) for x in plan['runs']] == WANTED, '四fresh范围/顺序错误')
    require(plan['final_compare'] == FINAL_COMPARE, '比较对/比较方法改变')
    require(plan['first_failure'] == FIRST_FAILURE, '首失败停改变')
    require(type(plan['port_base']) is int and [x['port'] for x in plan['runs']] == [plan['port_base']+i for i in range(len(WANTED))],
            '端口不是port_base起连续四个')
    for run in plan['runs']:
        directory = plan['output']+'/'+run['case']+'-'+run['arm']
        require(run['directory'] == directory and run['socket'] == planned_socket(directory), '运行目录/确定性socket不符')
    writer = plan['writer_binary']
    require(writer['path'] == str(Path(plan['repo'])/'rust/target/release/s_structure_session')
            and type(writer['expected_sha256']) is str and len(writer['expected_sha256']) == 64
            and type(writer['expected_bytes']) is int, '写者二进制期望未事前冻结')
    verify_dependency_plan(plan)

def verify_binary_pregate(plan, auth, plan_sha256):
    """执行前门：正式 worktree 已按 PLAN 记录的命令重建 release 写者二进制，且 SHA 等于期望；本 driver 不构建。"""
    writer = plan['writer_binary']
    binding = auth.get('binary_rebuild_receipt')
    require(type(binding) is dict and set(binding) == {'path','sha256'} and digest(binding['path']) == binding['sha256'],
            '缺写者二进制重建前门收据')
    receipt = read(binding['path'])
    require(receipt['status'] == REBUILD_PASS and receipt['plan_sha256'] == plan_sha256
            and receipt['script_sha256'] == writer['pregate_script']['sha256']
            and receipt['rebuild'] == writer['rebuild'] and receipt['exit_code'] == 0
            and receipt['binary_after'] == {'path':writer['path'],'sha256':writer['expected_sha256'],'bytes':writer['expected_bytes']},
            '重建前门收据不符')
    require(Path(writer['path']).stat().st_size == writer['expected_bytes'] and digest(writer['path']) == writer['expected_sha256'],
            '写者二进制不是P2新字节')
    return binding

def large_artifact_receipt(run):
    """TRIAGE §8.5：S/Q 停止后为 HAR/DB/WAL/backup 留 SHA 收据；只加收据，不参与业务门，异常记入收据不外抛。"""
    directory = Path(run['directory'])
    items, errors = [], []
    for relative in LARGE_ARTIFACTS:
        path = directory/relative
        item = {'relative':relative,'path':str(path)}
        try:
            before = path.lstat()
            require(stat.S_ISREG(before.st_mode), '非常规文件')
            h = hashlib.sha256()
            with os.fdopen(os.open(path,os.O_RDONLY|os.O_NOFOLLOW),'rb') as f:
                opened = os.fstat(f.fileno())
                for block in iter(lambda:f.read(1048576),b''):
                    h.update(block)
                ended = os.fstat(f.fileno())
            after = path.lstat()
            stamps = [[s.st_dev,s.st_ino,s.st_size,s.st_mtime_ns,s.st_ctime_ns] for s in (before,opened,ended,after)]
            item.update(bytes=before.st_size, sha256=h.hexdigest(), stamp=stamps[0],
                        identity_stable=all(x == stamps[0] for x in stamps))
        except Exception as exc:
            item['error'] = repr(exc)
            errors.append(relative)
        items.append(item)
    receipt = {'schema':'tb1414-p2-large-artifact-sha/1','case':run['case'],'arm':run['arm'],
               'taken':'after cleanup_run; owned S/Q stop attempted','items':items,'errors':errors,'business_gate':False}
    summary = {'directory':str(directory),'receipt':str(directory/'LARGE-ARTIFACT-SHA256.json'),'errors':errors}
    try:
        dump(directory/'LARGE-ARTIFACT-SHA256.json', receipt)
        summary['sha256'] = digest(directory/'LARGE-ARTIFACT-SHA256.json')
    except Exception as exc:
        summary['dump_error'] = repr(exc)
    return summary

def cleanup_run(plan, run):
    directory = Path(run['directory'])
    config = read(directory/'config.json')
    e = directory/'evidence'
    actions = []
    # 单次runtime的原execute已回收；wrapper仍独立核所属S/Q，任一错误不跳另一项。
    if e.exists():
        for service in ('q','s'):
            try:
                stopped = subprocess.run([plan['dependencies']['python'],str(Path(plan['repo'])/'s_session/s_service_control.py'),
                    'stop-'+service,'--config',str(directory/'config.json'),'--state-dir',str(e/'control')],
                    capture_output=True, timeout=30)
                (directory/('wrapper-stop-'+service+'.stdout')).write_bytes(stopped.stdout)
                (directory/('wrapper-stop-'+service+'.stderr')).write_bytes(stopped.stderr)
                actions.append({'service':service,'exit':stopped.returncode})
            except BaseException as exc:
                actions.append({'service':service,'error':repr(exc)})
    services = []
    for service in ('s','q'):
        handle = e/'control'/(service+'.process.json')
        if handle.exists():
            rec = read(handle)
            ps = subprocess.run(['ps','-p',str(rec['pid']),'-o','pid=,ppid=,stat=,comm='],capture_output=True,text=True)
            services.append({'service':service,'pid':rec['pid'],'ps_exit':ps.returncode,'rows':ps.stdout.strip(),'error':ps.stderr.strip()})
    absent = all(x['ps_exit'] == 1 and not x['rows'] and not x['error'] for x in services)
    with socket.socket() as s:
        s.settimeout(.5)
        port_errno = s.connect_ex(('127.0.0.1',int(config['port'])))
    path = Path(config['socket'])
    if path.exists():
        expected = read(e/'control/s.process.json')['socket_identity']
        def identity():
            st = path.lstat()
            require(stat.S_ISSOCK(st.st_mode) and st.st_uid == os.getuid(), 'socket身份/用户不符')
            return {'path':str(path),'device':st.st_dev,'inode':st.st_ino,'uid':st.st_uid}
        require(absent and identity() == expected, '所属服务未退/socket身份不符')
        with socket.socket(socket.AF_UNIX) as s:
            s.settimeout(.5)
            try:
                s.connect(str(path))
                raise ValueError('所属socket仍服务')
            except OSError as exc:
                require(exc.errno == errno.ECONNREFUSED, 'socket未证明拒连')
        require(identity() == expected, 'socket身份漂移')
        path.unlink()
    result = {'services':services,'actions':actions,'port_connect_errno':port_errno,'socket_absent':not path.exists()}
    result['passed'] = absent and port_errno == errno.ECONNREFUSED and not path.exists() and all(x.get('exit') == 0 for x in actions)
    dump(directory/'WRAPPER-CLEANUP.json', result)
    require(result['passed'], '所属资源清理未过')

def main():
    require(sys.argv[1:] == ['--execute'], '只能由根单次授权执行')
    plan, auth = read(FREEZE/'PLAN.json'), read(FREEZE/AUTHORIZATION)
    plan_sha256 = digest(FREEZE/'PLAN.json')
    require(auth['status'] == AUTHORIZATION_STATUS and auth['output'] == plan['output'], '缺根单次授权')
    require(auth['plan_sha256'] == plan_sha256, '计划指纹改变')
    require(set(auth['scripts']) == set(PACKAGE), '执行物集合改变')
    for name in PACKAGE:
        require(auth['scripts'][name] == digest(FREEZE/name), '执行物指纹改变')
    require(auth['runs'] == [[x['case'],x['arm'],x['port'],x['socket']] for x in plan['runs']], '授权资源与计划不符')
    require(auth['independent_review'], '缺独立准备审')
    for path, sha in auth['independent_review'].items():
        require(digest(path) == sha, '独立准备审改变')
    verify_rerun_plan(plan)
    rebuild_binding = verify_binary_pregate(plan, auth, plan_sha256)
    sources = read(FREEZE/'CURRENT-SOURCES.json')
    binary = Path(plan['writer_binary']['path'])
    require(sources.get(str(binary)) == plan['writer_binary']['expected_sha256'], 'CURRENT未绑定P2新写者二进制')
    for path, sha in {**sources,**plan['frozen_evidence']}.items():
        require(digest(path) == sha, '来源改变：'+path)
    dep = plan['dependencies']
    dependency_bindings = {dep['python']:dep['python_sha256'],dep['node']:dep['node_sha256'],
                           dep['browser']:dep['browser_sha256'],str(Path(dep['playwright_module'])/'package.json'):dep['playwright_package_sha256']}
    for path, sha in dependency_bindings.items():
        require(digest(path) == sha, '依赖二进制改变')
    from dependency_identity import verify_dependencies
    dependency_pre=verify_dependencies(dependency_bindings,dep['python'],plan['declared_python_dependency'],
                    browser_path=dep['browser'],chrome_declaration=plan['declared_chrome_dependency'])
    require(dependency_pre['status']=='PASS','remaining严格依赖初核失败')
    out = Path(plan['output'])
    require(not out.exists(), '输出必须新且独占')
    for run in plan['runs']:
        with socket.socket() as s:
            s.bind(('127.0.0.1',run['port']))
    sys.path[:0] = [str(Path(plan['repo'])/'s_session/tests'),str(Path(plan['repo'])/'s_session')]
    spec = importlib.util.spec_from_file_location('remaining_harness',FREEZE/'remaining-harness.py')
    harness = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(harness)
    runplan = harness.prepare(out,binary,Path(dep['python']),Path(dep['node']),Path(dep['playwright_module']),plan['port_base'])
    require([(x['case'],x['arm']) for x in runplan['runs']] == WANTED, '真实准备范围不符')
    require(runplan['run_deadline_seconds'] == 900 and runplan['operation_deadline_seconds'] == 30
            and runplan['offline_deadlines_seconds'] == {'collect_core':120,'frozen_oracle':180,'oracle_node':120}, '实际helper门不符')
    require(runplan['binary_sha256'] == plan['writer_binary']['expected_sha256'], 'harness实际写者二进制不是P2新字节')
    runtime_root = Path(plan['repo'])/'s_session'
    expected_runtime = {path:sha for path,sha in sources.items()
                        if (Path(path).is_relative_to(runtime_root) and Path(path).suffix in {'.py','.js','.cjs','.html','.sh','.json','.jsonl'})
                        or Path(path) == binary}
    require(runplan['source_sha256'] == expected_runtime, 'fresh实际原runtime源集合不等于已采纳正式冻结源')
    require(digest(plan['matrix_reference']['path']) == plan['matrix_reference']['sha256'], '矩阵依据改变')
    old_matrix = read(plan['matrix_reference']['path'])
    require([(x['case'],x['arm']) for x in old_matrix['arms']] == WANTED, '矩阵臂集合/顺序不符')
    for run, planned in zip(runplan['runs'], plan['runs']):
        directory = Path(run['directory'])
        expected = next(x for x in old_matrix['arms'] if (x['case'],x['arm']) == (run['case'],run['arm']))
        for name, item in expected['historical_generated_inputs'].items():
            require(digest(directory/name) == item['sha256'], '自然冻结输入改变')
        config = read(directory/'config.json')
        require(str(directory) == planned['directory'] and config['socket'] == planned['socket']
                and config['port'] == str(planned['port']), '实际运行目录/socket/端口与计划不符')
        require(not Path(config['socket']).exists(), 'socket已有')
    freeze_copy = out/'freeze'
    freeze_copy.mkdir()
    frozen_names = ('PLAN.json',AUTHORIZATION)+PACKAGE
    for name in frozen_names:
        shutil.copyfile(FREEZE/name, freeze_copy/name)
    shutil.copyfile(rebuild_binding['path'], freeze_copy/'REBUILD-RECEIPT.json')
    os.environ.update(plan['environment_overrides'])
    package_hashes = {str(FREEZE/name):digest(FREEZE/name) for name in frozen_names}
    package_hashes[rebuild_binding['path']] = rebuild_binding['sha256']
    package_hashes.update(auth['independent_review'])
    dump(out/'PACKAGE-BINDING.json', package_hashes)
    dump(out/'DEPENDENCY-PRE.json',dependency_pre)
    def cancel(_signum, _frame):
        raise RuntimeError('根中断；原harness回收本arm，wrapper核资源后保留失败')
    signal.signal(signal.SIGTERM, cancel)
    signal.signal(signal.SIGINT, cancel)
    started = time.time()
    completed = []
    large_receipts = []
    cause = None
    try:
        for run in runplan['runs']:
            try:
                harness.execute({**runplan,'runs':[run]},out)
            finally:
                try:
                    cleanup_run(plan,run)
                finally:
                    large_receipts.append(large_artifact_receipt(run))
            harness.validate_source_binding(runplan,run)
            gate_evidence(Path(run['directory'])/'evidence',run['case'],'CAPTURED_REQUIRES_DOUBLE_RUN_ORACLE_AND_BROWSER')
            require(read(Path(run['directory'])/'EXIT.json') == {'exit':0}, 'fresh EXIT非0')
            e = Path(run['directory'])/'evidence'
            queries = [json.loads(line) for line in (e/'events.jsonl').read_text().splitlines() if json.loads(line)['kind']=='bounded_query']
            commands = next(x for x in old_matrix['arms'] if (x['case'],x['arm']) == (run['case'],run['arm']))
            require([x['command'] for x in queries] == commands['public_commands'] + commands['normal_recovery_commands']
                    and len(queries) == 124 and all(x['deadline_seconds']==30 for x in queries), '124真实Q或单调用30门不符')
            for query in queries:
                receipt = read(e/'http/commands'/query['command']['id']/'RESULT.json')
                require(receipt['result']=='captured_validated_commit' and receipt['full_state_installed'] is True
                        and receipt['sources_unchanged'] is True, '真实Q安装收据未过')
            completed.append(run)
        # 四臂全部通过后才比较；两对都只在新字节 fresh 之间，旧首 composite core 不参与。
        from tb02c_compare import compare_exact_traces
        layout = harness.expected_layout(40)
        comparisons = []
        for case, left_index, right_index in PAIRS:
            left, right = completed[left_index], completed[right_index]
            require((left['case'],left['arm'],right['case'],right['arm']) == (case,'a',case,'b'), '比较对不符')
            compared = compare_exact_traces(Path(left['directory'])/'evidence/semantic-core.jsonl',
                                            Path(right['directory'])/'evidence/semantic-core.jsonl', layout)
            comparisons.append({'case':case,'left_arm':left['arm'],'right_arm':right['arm'],**compared})
        require(all(x['method'] == FINAL_COMPARE['method'] and x['excluded_fields'] == [] and x['identity_renaming'] == []
                    for x in comparisons), '比较方法/排除字段/身份改名改变')
        result = {'status':'PASS' if all(x['status']=='PASS' for x in comparisons) else 'NOT_VERIFIED','comparisons':comparisons,
                  'fresh_runs':WANTED,'pairs':FINAL_COMPARE['pairs'],
                  'historical_first_composite':'787f5cf 字节历史证据；本次未读取、未比较、未绑定（ROOT-RULING 600207e3…）',
                  'scope':'TB-02-D P2 新字节四臂，非整图完成'}
        dump(out/'PAIR-COMPARISON-RESULT.json',result)
        require(result['status']=='PASS','原完整221布局/全部字段/字节未通过')
    except BaseException as exc:
        cause = repr(exc)
        raise
    finally:
        mismatches = binding_mismatches({**sources,**package_hashes})
        evidence_mismatches = binding_mismatches(plan['frozen_evidence'])
        dependency_mismatches = binding_mismatches(dependency_bindings)
        try:
            dependency_post=verify_dependencies(dependency_bindings,dep['python'],plan['declared_python_dependency'],
                dependency_pre['receipts'],browser_path=dep['browser'],chrome_declaration=plan['declared_chrome_dependency'])
        except Exception as exc:
            dependency_post={'status':'FAIL','receipts':{},'mismatches':list(dependency_bindings),
                             'errors':[{'failure_stage':'postcheck','error':repr(exc)}]}
        dump(out/'DEPENDENCY-POST.json',dependency_post)
        dependency_mismatches=sorted(set(dependency_mismatches)|set(dependency_post['mismatches']))
        matrix_mismatches = binding_mismatches({plan['matrix_reference']['path']:plan['matrix_reference']['sha256']})
        dump(out/'SOURCE-POSTCHECK.json',{'count':len(sources),'mismatches':mismatches,
             'evidence_count':len(plan['frozen_evidence']),'evidence_mismatches':evidence_mismatches,
             'dependency_mismatches':dependency_mismatches,'matrix_mismatches':matrix_mismatches})
        dump(out/'EXECUTION.json',{'cause':cause,'completed_fresh':[x['directory'] for x in completed],
             'large_artifact_receipts':large_receipts,'elapsed_seconds':time.time()-started})
        require(not mismatches and not evidence_mismatches and not dependency_mismatches
                and not matrix_mismatches,'source/package/证据/依赖/矩阵后核失败')

if __name__ == '__main__':
    main()
