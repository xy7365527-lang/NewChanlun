"""#1414静态复用候选；真实独审限定采纳与根新授权后才可调用。"""
import hashlib
import json
import os
from pathlib import Path

# 调用者须先核旧helper的冻结SHA，再将其精确目录加入sys.path。
from completion_io import require, stamp, fd_stamp, open_source, copy_file


def verify_reuse_grant(grant, plan):
    """不以自报bool当证明；先核实际独审/根采纳/来源/postcheck完整SHA。"""
    require(type(grant) is dict and grant.get('status')=='ROOT_ACCEPTED_FINITE_ACTUAL_STATE_REUSE',
            '未提供根具名有限复用采纳')
    required={'actual_independent_review','actual_independent_check','actual_independent_binding',
              'root_reuse_adoption','prior_source_postcheck','prior_database_postcheck','prior_author_binding'}
    require(set(grant['acceptance_bindings'])==required, '独审/根采纳/来源/后核绑定缺失或多余')
    require(grant['acceptance_bindings']==plan['reuse_acceptance_bindings'], '复用grant不是本计划绑定的采纳')
    accepted={}
    for name,binding in grant['acceptance_bindings'].items():
        if name in ('actual_independent_review',):
            before=stamp(binding['path'])
            require(before[2]<=262144,'实际独审文本非小材料')
            with open_source(binding['path']) as stream:
                require(fd_stamp(stream)==before,'独审文本打开时身份改变')
                raw=stream.read(262145)
                require(fd_stamp(stream)==before,'独审文本读取中身份改变')
            require(stamp(binding['path'])==before and len(raw)==before[2]
                    and hashlib.sha256(raw).hexdigest()==binding['sha256'],'实际独审文本SHA/身份不符')
        else:
            accepted[name]=receipt(binding['path'],binding['sha256'])
    adoption=accepted['root_reuse_adoption']
    require(adoption['status']=='ROOT_ACCEPTED_FINITE_ACTUAL_STATE_REUSE'
            and adoption['actual_independent_files']=={name:grant['acceptance_bindings'][name]
                  for name in ('actual_independent_review','actual_independent_check','actual_independent_binding')}
            and adoption['source_receipts']==grant['source_receipts']
            and adoption['core']==grant['core'] and adoption['sealed_group']==grant['sealed_group'],
            '根采纳没有精确绑定独审与这组复用实体')
    source=accepted['prior_source_postcheck'];database=accepted['prior_database_postcheck']
    require(source['source_mismatches']==[] and source['evidence_mismatches']==[]
            and source['dependency_mismatches']==['/tmp/tb1414-runtime-venv/bin/python'],
            '旧实际来源后核不满足已裁定的有限复用边界')
    require(database['sealed_sha_mismatches']==[] and database['sealed_errors']==[]
            and database['original_group_unchanged'] is True and database['history_unchanged'] is True,
            '旧sealed/原件/历史后核不符')
    require(adoption['allowed_dependency_interface_rejection_path']=='/tmp/tb1414-runtime-venv/bin/python'
            and adoption['state_identity_reuse']=='sealed_three_file_exact_copy_only'
            and adoption['old_failure_preserved'] is True,'根采纳范围不符')
    return {'status':'ACCEPTANCE_BINDINGS_SHA_VERIFIED','acceptance_bindings':grant['acceptance_bindings'],
            'source_receipts':grant['source_receipts'],'old_dependency_rejection_preserved':True}


def pairs(items):
    result={}
    for key,value in items:
        require(key not in result,'复用小收据重复JSON键')
        result[key]=value
    return result


def receipt(path, expected_sha256):
    before=stamp(path)
    require(before[2] <= 262144,'复用收据超过事前小材料界')
    with open_source(path) as stream:
        require(fd_stamp(stream)==before,'小收据open身份改变')
        raw=stream.read(262145)
        require(len(raw)==before[2] and fd_stamp(stream)==before,'小收据长度/身份改变')
    require(stamp(path)==before and hashlib.sha256(raw).hexdigest()==expected_sha256,
            '复用小收据完整SHA/身份不符')
    return json.loads(raw,object_pairs_hook=pairs)


def reference_verified_core(core_receipt, core_receipt_sha256, reuse_grant):
    """不打开core正文；不把历史SHA称本轮独立重hash。"""
    require(type(reuse_grant) is dict and reuse_grant.get('status')=='ROOT_ACCEPTED_FINITE_ACTUAL_STATE_REUSE',
            '尚缺旧actual有限独审的明确复用限定')
    require(reuse_grant['source_receipts']['core_custody']==
            {'path':str(core_receipt),'sha256':core_receipt_sha256}, '旧core证明收据未被限定采纳绑定')
    core=receipt(core_receipt,core_receipt_sha256)
    require(core['status']=='SINGLE_SOURCE_PASS_STRICT221_COPIED' and core['lines']==221
            and core['bytes']==1456959869
            and core['sha256']=='c0dbe0db7e6670b53fb1d1f89f722c28c56858c31b93639cbc952e77ffddfb3b',
            '旧已验221实体绑定不符')
    require(core['verified_prefix219']['sha256']=='ea27a082aab949bd6d880edf3a2eb8fd628fc13671686b6edc83bddd3b19ebc0'
            and core['verified_prefix219']['bytes']==1358167511
            and core['verified_prefix219']['lines']==219,'旧prefix219绑定不符')
    require(len(core['layout'])==221 and len(core['recovery_full_structure_bindings'])==2,
            '旧221/恢复内容证明未完整')
    bindings=core['recovery_full_structure_bindings']
    require([entry['generation'] for entry in bindings]==['27','40'] and all(
            entry['known_candidate_sha256']==entry['before_sha256']==entry['after_sha256']
            for entry in bindings),'旧恢复全结构关联不符')
    require(reuse_grant['core']=={'path':core['copy'],'sha256':core['sha256'],'bytes':core['bytes'],
                                'stamp':core['copy_stamp']},'复用采纳未精确绑定core实体')
    require(stamp(core['copy'])==core['copy_stamp'],'旧core实体身份发生改变，不能仅援引旧SHA')
    return {'status':'PRIOR_STRICT221_REFERENCED_WITH_SAME_FILE_IDENTITY_NOT_REREAD',
            'path':core['copy'],'sha256':core['sha256'],'bytes':core['bytes'],
            'stamp':core['copy_stamp'],'core_body_opened_this_phase':False,
            'source_receipt':str(core_receipt),'source_receipt_sha256':core_receipt_sha256,
            'later_compare_exact_traces_still_required':True}


def fork_prior_sealed_group(custody_receipt,custody_sha256,state_receipt,state_sha256,new_working_directory,deadline,
                            reuse_grant,assert_prior_stopped):
    """只从已有sealed三件创建新working；不读原r2库/失败working，不重复全表。"""
    require(type(reuse_grant) is dict and reuse_grant.get('status')=='ROOT_ACCEPTED_FINITE_ACTUAL_STATE_REUSE',
            '尚缺旧actual状态证明的明确复用限定')
    require(reuse_grant['source_receipts']['database_custody']==
            {'path':str(custody_receipt),'sha256':custody_sha256}
            and reuse_grant['source_receipts']['state_identity']==
            {'path':str(state_receipt),'sha256':state_sha256}, 'sealed/state收据未被同一限定采纳绑定')
    core_ref=reuse_grant['source_receipts']['core_custody']
    core=reference_verified_core(core_ref['path'],core_ref['sha256'],reuse_grant)
    custody=receipt(custody_receipt,custody_sha256)
    state=receipt(state_receipt,state_sha256)
    expected_tables={'batches','catalog','meta','objects','observations','raw_events','raw_ohlc','relations',
        's_clock_events','s_delivery_policy','s_delivery_refs','s_input_messages','s_protocol_meta',
        'structure_deltas','structure_facts','witnesses','writer_epoch_history'}
    require(state['status']=='EXACT_ALL_TABLES_SCHEMA_WITH_FOUR_SPECIFIC_RECOVER_TRANSITIONS'
            and state['persistent_writer_epoch']=='2' and state['generation']=='40'
            and state['structure_cut']=='cut-40' and state['schema_all_values_equal'] is True
            and state['excluded_fields']==[] and len(state['tables'])==17
            and {entry['table'] for entry in state['tables']}==expected_tables
            and all(entry['excluded_fields']==[] for entry in state['tables']), '旧全状态证明范围不符')
    require(state['identity']['session_id']=='tb02d-second_open_up'
            and state['identity']['session_generation']=='1', '旧全状态会话/化身不符')
    sealed=custody['sealed']
    require(custody['original_sqlite_opened'] is False and custody['sealed_sqlite_opened'] is False
            and len(sealed)==len(custody['working'])==3,'旧sealed完整custody不符')
    expected_names=['session.sqlite','session.sqlite-wal','session.sqlite-shm']
    require([Path(entry['copy']).name for entry in sealed]==expected_names,'sealed三件同名关系不符')
    expected=[{'path':entry['copy'],'sha256':entry['sha256'],'bytes':entry['bytes']} for entry in sealed]
    require(reuse_grant['sealed_group']==expected,'复用采纳未精确绑定sealed三件')
    stopped_before=assert_prior_stopped()
    require(type(stopped_before) is dict and stopped_before.get('passed') is True,
            'assert_prior_stopped必须在全部实际门成立时返回passed:true，否则抛异常')
    before={entry['copy']:stamp(entry['copy']) for entry in sealed}
    destination=Path(new_working_directory)
    destination.mkdir()
    copied=[]
    for entry in sealed:
        require(before[entry['copy']][2]==entry['bytes'],'sealed事前长度不符')
        copied.append(copy_file(entry['copy'],destination/Path(entry['copy']).name,deadline,entry))
    require(all(stamp(path)==original for path,original in before.items()),'sealed全组复制期间身份漂移')
    stopped_after=assert_prior_stopped()
    require(type(stopped_after) is dict and stopped_after.get('passed') is True,
            '复制后prior stopped门不完整')
    return {'status':'PRIOR_SEALED_THREE_FILE_BYTES_FORKED_TO_NEW_WORKING',
            'source_receipt':str(custody_receipt),'source_receipt_sha256':custody_sha256,
            'state_identity_receipt':str(state_receipt),'state_identity_sha256':state_sha256,
            'verified_state_identity':state,
            'core_reference':core,
            'sealed_group_before':before,'working_copies':copied,
            'stopped_before':stopped_before,'stopped_after':stopped_after,
            'sealed_original_sqlite_opened':False,'old_failed_working_used':False,
            'old_epoch2_state_identity_reused_after_exact_group_bytes':True,
            'all17_table_comparison_reexecuted':False,'oracle_reexecuted':False,
            'old_owner_locks_process_records_installed_into_working':False}
