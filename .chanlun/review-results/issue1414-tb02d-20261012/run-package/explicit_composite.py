"""#1414首显式composite新schema草案；仅根签完整review/采纳/source映射后准入。

独立aggregate审和根release尚未到达；有限内存fixture不产生实际释放。
"""
from pathlib import Path
import stat
import hashlib
import json

HISTORICAL_RECEIPT_IDENTITIES={
    'SOURCE-LINEAGE.json':{
        'path':'/Volumes/AgentStorage/issue1414-r4-key-reuse-supplement-r2-20261013/second_open_up-a-supplement/evidence/SOURCE-LINEAGE.json',
        'bytes':265608,'sha256':'2c315a3d849528f86587c2f6148e30c6864ae116a92bde5e6e6c553c10eb04f8'},
    'ORIGINAL-STAMP-POSTCHECK.json':{
        'path':'/Volumes/AgentStorage/issue1414-r4-key-reuse-supplement-r2-20261013/second_open_up-a-supplement/evidence/ORIGINAL-STAMP-POSTCHECK.json',
        'bytes':391759,'sha256':'5260cdb8cde8f803fe6aa2e07479422d5c19445d76741388c570e3943d080170'}}


def historical_receipt(path, expected_sha256, declaration):
    """两件已具名历史小JSON；事前精确长度帽，不按通用receipt失败换门。"""
    from completion_io import require, stamp, fd_stamp, open_source
    from reuse_state import pairs
    require(str(path)==declaration['path'] and expected_sha256==declaration['sha256'],
            '历史具名收据路径/SHA偏离事前声明')
    before=stamp(path)
    require(before[2]==declaration['bytes'],'历史具名收据固定长度不符')
    with open_source(path) as stream:
        require(fd_stamp(stream)==before,'历史具名收据open身份漂移')
        raw=stream.read(declaration['bytes']+1)
        require(len(raw)==declaration['bytes'] and fd_stamp(stream)==before,
                '历史具名收据读取中长度/身份漂移')
    require(stamp(path)==before and hashlib.sha256(raw).hexdigest()==expected_sha256,
            '历史具名收据完整SHA/身份不符')
    return json.loads(raw,object_pairs_hook=pairs)


def verify_dependency_plan(plan):
    from completion_io import require
    from dependency_identity import CHROME_CONTRACT
    dep=plan['dependencies'];chrome=plan.get('declared_chrome_dependency')
    require(type(chrome) is dict and chrome=={'path':dep['browser'],
        'sha256':dep['browser_sha256'],'contract':CHROME_CONTRACT}
        and plan['environment_overrides'].get('TB02D_CHROMIUM_EXECUTABLE')==dep['browser']
        and dep['browser'] not in (dep['python'],plan['declared_python_dependency']['resolved_path'],
                                  dep['node'],str(Path(dep['playwright_module'])/'package.json')),
        '新remaining依赖契约未事前绑定browser/launch且隔离其它角色')


def verify_comparison_binding(comparison, first):
    from completion_io import require
    require(comparison['status']=='PASS' and comparison['sha256'][0]==first['core_sha256']
            and comparison['compared_bytes']==first['core_bytes'],
            '首arm完整core不同于实际独审绑定')


def verify_explicit_first(plan, sources):
    from completion_io import require, stamp
    from reuse_state import receipt, verify_reuse_grant, reference_verified_core

    def load(binding):
        require(type(binding) is dict and set(binding)=={'path','sha256'},'缺精确小收据绑定')
        return receipt(binding['path'],binding['sha256'])

    first=plan.get('first_composite')
    require(type(first) is dict and first.get('contract')=='explicit-four-layer-observed-composite/1',
            '缺新具名显式composite；禁止旧gate失败后fallback')
    composite=load(first['binding'])
    require(composite['status']=='COMPLETE_EXPLICIT_COMPOSITE_PAYLOAD_PENDING_DETACHED_RELEASE'
            and composite['pending']==[],'首composite仍未释放')
    release=load(first['root_release']);review=load(first['composite_actual_review'])
    require(release['status']=='ROOT_ACCEPTED_EXPLICIT_FIRST_COMPOSITE'
            and release['composite_binding']==first['binding']
            and release['review']==first['composite_actual_review']
            and review['verdict']=='VERIFIED_COMPLETE_COMPOSITE'
            and review['composite_binding']==first['binding']
            and review['layers']==composite['layers']
            and review['source_mapping_provenance']==composite['source_mapping_provenance']
            and review['core']==composite['core'],'根/实际独审未绑定完整复合门')
    require(first['core_sha256']==composite['core']['sha256']
            and first['core_bytes']==composite['core']['bytes'],'最终原比较器core binding漂移')

    mapping=load(first['source_mapping']);mapped_review=load(first['source_mapping_review'])
    require(mapping['status']=='AUTHOR_ENUMERATED_THREE_SOURCE_SETS_PENDING_INDEPENDENT_MAPPING_REVIEW'
            and mapped_review['status']=='PASS_EXACT_THREE_SOURCE_MAPPING'
            and mapped_review['mapping']==first['source_mapping']
            and mapped_review['composite_binding']==first['binding'],'三来源映射未独审')
    require(composite['source_mapping_provenance']==first['source_mapping'],
            'payload未绑定同一三来源映射')
    historical=load(mapping['historical_current']);actual=load(mapping['actual_current'])
    fresh=load(mapping['formal_fresh_current'])
    require(fresh==sources,'fresh source集合/每SHA不等于新正式已冻全集')
    enumeration=load(mapping['formal_fresh_enumeration'])
    require(enumeration['status']=='AUTHOR_ENUMERATED_EXACT_DOMAINS_PENDING_INDEPENDENT_REVIEW'
            and enumeration['current']==mapping['formal_fresh_current']
            and enumeration['count']==len(fresh)
            and enumeration['mismatches']==[] and enumeration['errors']==[],
            'fresh事前完整实际枚举未闭合')
    for name,manifest in (('historical',historical),('actual',actual),('formal_fresh',fresh)):
        roles=load(mapping['source_roles'][name])
        members=[p for paths in roles['domains'].values() for p in paths]
        require(len(members)==len(set(members)) and set(members)==set(manifest)
                and mapping['counts'][name]==len(manifest),'三来源角色域漏成员/重复成员')
    formal_roles=load(mapping['source_roles']['formal_fresh'])
    require(formal_roles['domains']==enumeration['domains'],
            'fresh实际枚举域不同于角色映射')
    require(set(formal_roles['domains']['package_helpers'])=={
        str(Path(__file__).resolve().parent/name) for name in plan['proposed_package_helpers']},
        'fresh缺新运行包helper或混入其它包来源')
    require(all(Path(p).name not in ('s_tb1414_observation.py',) for p in fresh),
            'formal fresh混入已剥离观测helper')
    verify_dependency_plan(plan)
    adoption=load(composite['formal_adoption']);adoption_review=load(composite['formal_adoption_review'])
    require(mapping['adopted_source_receipt']==composite['formal_adoption']
            and mapping['adoption_independent_review']==composite['formal_adoption_review']
            and plan['formal_adoption']==plan['formal_adoption_actual_evidence']==composite['formal_adoption'],
            '采纳声明未逐字绑定实际ACTUAL-BINDING/独审引用')
    require(adoption['status']=='PASS_EXACT_TWO_FILE_ADOPTION_AND_POSTCHECK_PENDING_INDEPENDENT_MAPPING_REVIEW'
            and adoption_review['status']=='PASS_VERIFIED_EXACT_TWO_FILE_ACTUAL_ADOPTION_MAPPING_NOT_RUNTIME_ACCEPTANCE'
            and {k:adoption_review['actual_binding'][k] for k in ('path','sha256')}==composite['formal_adoption']
            and adoption_review['formal_after_verified_current']==adoption['formal_after']
            and adoption_review['pure_actual_runtime_pass'] is False
            and adoption['pure_actual_runtime_pass'] is False and adoption['failure'] is None,
            'pure两件实际采纳/独审未闭合；旧作者pending字样不追溯改写')
    checks=adoption_review['checks']
    require(all(checks[k] is True for k in ('all_ref_bytes_sha_read_identity_match',
        'actual_after_byte_for_byte_equal_candidates','before_backups_exact_plan_bytes',
        'write_receipts_match_backup_and_postcheck','same_device_inode_mode_uid_gid_nlink',
        'three_protected_full_identity_matches_backup','one_authorization_plan_script_diff_targets_and_reviews_match',
        'index_current_full_stat_matches_before_after'))
        and checks['observation_helper_lexists'] is False,'实际采纳保护/授权/备份门未闭合')
    post_bindings=[b for b in adoption['files'] if Path(b['path']).name=='POSTCHECK.json']
    require(len(post_bindings)==1,'缺唯一采纳后核收据')
    post_binding={k:post_bindings[0][k] for k in ('path','sha256')}
    require(mapping['adopted_source_postcheck']==composite['formal_adoption_postcheck_evidence']
            ==plan['formal_adoption_postcheck_evidence']==post_binding,
            '采纳后核声明未逐字绑定实际POSTCHECK路径/SHA')
    adoption_post=load(post_binding)
    require(adoption_post['status']=='PASS_EXACT_TWO_FILE_MAPPING_NOT_RUNTIME_ACCEPTANCE'
            and adoption_post['targets']==adoption['formal_after']
            and adoption_post['protected']==adoption_review['protected_verified_current']
            and adoption_post['observation_helper_absent'] is True,'实际采纳后核身份不符')
    root=mapping['formal_root']+'/'
    baseline={p:s for p,s in actual.items() if p.startswith(root)}
    expected=dict(baseline)
    require(set(mapping['formal_two_file_delta'])=={'s_session/browser/tb01c-client.js','s_session/tests/tb02d_browser.cjs'},
            '正式两件之外的产品改面未获根采纳')
    for relative,change in mapping['formal_two_file_delta'].items():
        path=root+relative
        require(baseline[path]==change['before_sha256'],'正式before不等于实际隔离包保护基线')
        expected[path]=change['after_sha256']
    require(len(adoption['formal_after'])==2
            and {r['path']:r['sha256'] for r in adoption['formal_after']}=={
                root+p:change['after_sha256'] for p,change in mapping['formal_two_file_delta'].items()},
            'actual adoption after不等于root选定pure两件')
    require({p:s for p,s in sources.items() if p.startswith(root)}==expected,
            '正式产品全集不是原基线仅pure两件变化')
    require(mapping['actual_observed_identity']!=mapping['formal_pure_identity']
            and mapping['pure_actual_browser_pass_claim'] is False,'不得把observed实际PASS改名pure已跑')
    require(mapping['formal_pure_identity']=={
        'client_sha256':'083f424d9ad9d2e4e6402f44a2e1b9cfe5ae9bb377053df91fc0ae2ffe132822',
        'runner_sha256':'a3d32f0be360851a54a8e9e978445dcafcda6805f3e110b572c69a338277803d'},
        '正式pure两件偏离根选择')
    for path,sha in mapping['immutable_common_sources'].items():
        require(historical.get(path)==actual.get(path)==fresh.get(path)==sha,'未变规则/input/profile/binary三源不一致')

    r2=composite['layers']['r2'];check=load(r2['actual_check'])
    r2_plan=load(r2['plan'])
    require(plan['named_historical_receipts']==HISTORICAL_RECEIPT_IDENTITIES,
            '两件历史收据事前精确读帽声明不符')
    require(check['plan_sha256']==r2['plan']['sha256']
            and r2_plan['current_sources_manifest_sha256']==mapping['historical_current']['sha256']
            and r2_plan['current_sources_count']==len(historical)
            and check['source_manifest_count']==len(historical)
            and check['manifest_mapping_matches_current'] is True
            and check['tracked_stamp_receipts_all_unchanged'] is True,'r2源快照/原件/计划层未闭合')
    require(check['status']=='FAIL_VERIFIED_FINITE_PROGRESS_BROWSER_NAVIGATION_TIMEOUT'
            and check['inherited_queries']==120 and check['new_queries_receipt_count']==4
            and check['historical_reconstructed_records']==139 and check['new_reconstructed_records']==0
            and check['input_replayed']==0 and check['history_artifact_bindings']==373
            and check['normal_recovery_epoch1_to2_equal'] is True
            and check['oracle_leaf_status']=='passed' and check['cleanup_passed'] is True
            and check['source_evidence_dependencies_mismatches']==[],'r2有限4Q/恢复/7cut/历史谱系门失败')
    r2receipts={name:(historical_receipt(binding['path'],binding['sha256'],
                    HISTORICAL_RECEIPT_IDENTITIES[name]) if name in HISTORICAL_RECEIPT_IDENTITIES
                    else load(binding)) for name,binding in r2['receipts'].items()}
    lineage=r2receipts['SOURCE-LINEAGE.json']
    require(lineage['old_status']==lineage['previous_supplement_status']=='FAIL'
            and lineage['input_replayed']==0 and lineage['public_queries_reused']==120
            and lineage['original_receipt_records_inherited']==80 and lineage['records_before_recovery']==219
            and lineage['inherited_reconstructed_records']==139 and lineage['reconstructed_records']==0
            and len(lineage['inherited_source_bindings'])==373,'旧FAIL/219/历史139不准改写')
    require(lineage['formal_adoption_mapping']==r2_plan['formal_adoption'],'历史五件正式采纳层改变')
    comparisons=r2receipts['PUBLIC-RAW-BYTE-COMPARISONS.json']
    ids=['before-normal-27','before-normal-40','after-normal-27','after-normal-40']
    require([r['command']['id'] for r in comparisons]==ids
            and all(r['all_raw_bytes_equal'] is True for r in comparisons),'原4Q完整raw比较失败')
    for cid in ids:
        r=r2receipts['http/'+cid+'/RESULT.json']
        require(r['result']=='captured_validated_commit' and r['full_state_installed'] is True
                and r['sources_unchanged'] is True,'原4Q安装不完整')
    recovery=r2receipts['normal-recovery.json'];oracle=r2receipts['query-oracle.json']
    require(recovery['equal'] is True and recovery['cuts']==[27,40]
            and recovery['recover']['reply']['previous_epoch']=='1'
            and recovery['recover']['reply']['new_epoch']=='2'
            and oracle['status']=='passed' and oracle['case']=='second_open_up' and oracle['cuts']==40
            and set(oracle['oracle_cuts'])=={'15','26','27','31','38','39','40'},'原恢复/oracle叶子未过')
    original=r2receipts['ORIGINAL-STAMP-POSTCHECK.json']
    require(not original['errors'] and original['records'] and all(r['unchanged'] for r in original['records']),
            'r2原件后核未过')

    observed=composite['layers']['observed'];observed_plan=load(observed['plan'])
    observed_review=load(observed['actual_check'])
    require(observed_plan['output']==observed['output']
            and observed_plan['current_sources_sha256']==mapping['actual_current']['sha256']
            and observed_plan['current_sources_count']==len(actual)
            and observed_review['status']=='PASS_VERIFIED_ACTUAL_OBSERVED_CODEPOINT_BROWSER_COMPLETION_NOT_COMPOSITE_RELEASE'
            and observed_review['actual_output']==observed['output']
            and observed_review['plan_sha256']==observed['plan']['sha256']
            and observed_review['current_sources_sha256']==mapping['actual_current']['sha256']
            and observed_review['source_snapshot_mapping_and_stamps_equal_current_and_before'] is True
            and observed_review['source_all_post_mismatches_errors']==[]
            and observed_review['all_four_axes_and_pending27_full_assertions_verified'] is True
            and observed_review['cleanup_pass'] is True,'新observed实际scope/源/全业务/清理未通过')
    require(mapping['actual_observed_identity']=={
        'client_sha256':observed_review['actual_observed_client_sha256'],
        'runner_sha256':observed_review['actual_observed_runner_sha256']},'observed/pure身份映射错层')
    receipts={name:load(binding) for name,binding in observed['receipts'].items()}
    require(receipts['EXIT.json']['exit']==receipts['WRAPPER-EXECUTION.json']['exit']==0
            and receipts['WRAPPER-EXECUTION.json']['cause'] is None,'本次actual success门未过')
    browser=receipts['browser/RESULT.json'];cleanup=receipts['CLEANUP-CHECK.json'];post=receipts['SOURCE-POSTCHECK.json']
    require(browser['status']=='passed' and browser['cuts']==['27','40'] and browser['page_errors']==[]
            and cleanup['errors']==[] and cleanup['port_refused'] is True and cleanup['socket_absent'] is True
            and post['count']==len(actual)
            and all(post[k]==[] for k in ('source_mismatches','evidence_mismatches','dependency_mismatches','source_errors','evidence_errors','dependency_errors')),
            '本次browser或sources/evidence/dependencies/cleanup后核未过')
    require(receipts['DEPENDENCY-POST.json']['status']=='PASS'
            and receipts['DATABASE-POSTCHECK.json']['status']=='PASS'
            and receipts['DATABASE-POSTCHECK.json']['sealed_group_unchanged'] is True
            and receipts['CORE-REFERENCE-POSTCHECK.json']['status']=='PASS','本次依赖/sealed/core后核未过')
    capture=receipts['RESULT.json']
    require(capture['inherited_queries']==120 and capture['inherited_recovery_queries']==4
            and capture['new_recovery_queries']==capture['new_input_replayed']==0
            and capture['core_records']==221 and capture['s_started'] is False
            and capture['recover_reexecuted'] is False and capture['oracle_reexecuted'] is False,
            '复合新增/继承计数或动作名分错误')
    grant=observed_plan['independent_actual_reuse_grant']
    verify_reuse_grant(grant,observed_plan)
    seal=composite['layers']['strict_seal'];seal_check=load(seal['actual_check'])
    require(seal['acceptance']==grant['acceptance_bindings']['root_reuse_adoption']
            and seal['source_receipts']==grant['source_receipts']
            and seal_check['core_strict_layout_receipt_matches_expected221'] is True
            and seal_check['authority_records']==18 and seal_check['small_sha_mismatches']==[]
            and seal_check['exact_transition_tables']==['meta','s_clock_events','s_protocol_meta','writer_epoch_history'],
            'strict221/17表schema/四recover转移复用层未闭合')
    state=load(seal['source_receipts']['state_identity'])
    tables={'batches','catalog','meta','objects','observations','raw_events','raw_ohlc','relations',
        's_clock_events','s_delivery_policy','s_delivery_refs','s_input_messages','s_protocol_meta',
        'structure_deltas','structure_facts','witnesses','writer_epoch_history'}
    transitions={'meta','s_clock_events','s_protocol_meta','writer_epoch_history'}
    require(state==seal_check['state_identity_receipt']
            and state['status']=='EXACT_ALL_TABLES_SCHEMA_WITH_FOUR_SPECIFIC_RECOVER_TRANSITIONS'
            and state['schema_all_values_equal'] is True and state['excluded_fields']==[]
            and len(state['tables'])==17 and {r['table'] for r in state['tables']}==tables
            and all(r['excluded_fields']==[] and r['transition']==(
                'exact_recover_idle1_to2' if r['table'] in transitions else 'identity') for r in state['tables']),
            '17表/schema/四转移的实际完整状态收据缺失或不符')
    custody=receipts['DATABASE-FORK-CUSTODY.json']
    require(custody['verified_state_identity']==seal_check['state_identity_receipt']
            and custody['old_failed_working_used'] is False
            and custody['sealed_original_sqlite_opened'] is False
            and len(custody['working_copies'])==len(grant['sealed_group'])==3,'新working不来自完整sealed三件状态')
    require([Path(r['path']).name for r in grant['sealed_group']]==[
        'session.sqlite','session.sqlite-wal','session.sqlite-shm'], 'sealed三件缺WAL/SHM或身份错配')
    for copied,original in zip(custody['working_copies'],grant['sealed_group']):
        require(copied['source']==original['path'] and copied['sha256']==original['sha256']
                and copied['bytes']==original['bytes']
                and copied['copy']==str(Path(observed['output'])/'working'/Path(original['path']).name),
                '本次sealed→working逐件copy身份/SHA/bytes映射不符')
    core_ref=reference_verified_core(observed_plan['prior_core_receipt']['path'],
                                    observed_plan['prior_core_receipt']['sha256'],grant)
    require(composite['core']=={k:core_ref[k] for k in ('path','sha256','bytes','stamp')},'首strict221实体不等于已验引用')
    st=Path(core_ref['path']).lstat()
    require(stat.S_ISREG(st.st_mode) and stamp(core_ref['path'])==core_ref['stamp'],'首core路径/身份漂移')
    return Path(core_ref['path']).parent
