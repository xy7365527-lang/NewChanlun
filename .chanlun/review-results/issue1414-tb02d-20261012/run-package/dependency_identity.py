"""#1414新具名Chrome两观测点契约；单次hash与其它依赖严格门不变。"""
import hashlib
import os
from pathlib import Path
import stat

CHROME_CONTRACT='named-chrome-object-content-endpoints/1'
CHROME_EQUAL_FIELDS=('path','kind','contract','dev','ino','size','mtime_ns','file_type',
                     'sha256','mode','uid','gid','nlink')
STAMP_FIELDS=('dev','ino','size','mtime_ns','ctime_ns','file_type')
CHROME_RECEIPT_FIELDS=set(CHROME_EQUAL_FIELDS)|{'ctime_ns','within_observation_identity_stable'}


def require(condition, detail):
    if not condition:
        raise ValueError(detail)


def identity(value):
    return [value.st_dev, value.st_ino, value.st_size, value.st_mtime_ns,
            value.st_ctime_ns, stat.S_IFMT(value.st_mode)]


def regular_stamp(path):
    value=Path(path).lstat()
    require(stat.S_ISREG(value.st_mode), '依赖须为非symlink常规文件：'+str(path))
    return identity(value)


def file_digest(path, expected_stamp, observations=None):
    h=hashlib.sha256()
    def record(label, value):
        actual=identity(value)
        if observations is not None:
            observations.update(stage=label)
            observations[label+'_stamp']=actual
            observations['changed_fields']=[field for field,old,new in zip(STAMP_FIELDS,expected_stamp,actual) if old!=new]
        return actual
    if observations is not None:observations['stage']='open_identity'
    with os.fdopen(os.open(path,os.O_RDONLY|os.O_NOFOLLOW),'rb') as stream:
        require(record('open_identity',os.fstat(stream.fileno())) == expected_stamp, '真实目标open/fstat身份漂移')
        if observations is not None:observations['stage']='hash'
        for raw in iter(lambda:stream.read(1048576),b''):
            h.update(raw)
        require(record('end_fstat',os.fstat(stream.fileno())) == expected_stamp, '真实目标读取中身份漂移')
    if observations is not None:observations['stage']='end_lstat'
    final=Path(path).lstat()
    require(record('end_lstat',final) == expected_stamp and stat.S_ISREG(final.st_mode), '真实目标读取后身份漂移')
    return h.hexdigest()


def declared_python_identity(path, sha256, declaration, observations=None):
    """所有预期项必须已由新计划冻结；本函数不从当前观测自填预期。"""
    required={'path','link_stamp','readlink','resolved_path','target_stamp','sha256'}
    require(type(declaration) is dict and set(declaration) == required, '具名Python链接声明字段缺失/多余')
    require(str(path) == declaration['path'] and sha256 == declaration['sha256'], 'Python特例未绑定计划依赖路径/SHA')
    require(Path(path).is_absolute() and Path(declaration['resolved_path']).is_absolute(), '依赖路径须绝对')
    for name in ('link_stamp','target_stamp'):
        require(type(declaration[name]) is list and len(declaration[name]) == 6
                and all(type(v) is int for v in declaration[name]), '依赖预期身份未冻结：'+name)
    require(declaration['link_stamp'][5] == stat.S_IFLNK
            and declaration['target_stamp'][5] == stat.S_IFREG, '链接/真实目标类型声明不符')
    link_before=identity(Path(path).lstat())
    if observations is not None:observations.update(stage='read_identity',link_stamp=link_before)
    require(link_before == declaration['link_stamp'], '声明的venv链接自身身份改变')
    link_text=os.readlink(path)
    if observations is not None:observations['readlink']=link_text
    require(link_text == declaration['readlink'], '声明的venv readlink改变')
    resolved=str(Path(path).resolve(strict=True))
    if observations is not None:observations['resolved_path']=resolved
    require(resolved == declaration['resolved_path'], '声明的venv真实目标路径改变')
    target_before=regular_stamp(resolved)
    if observations is not None:observations['target_stamp']=target_before
    require(target_before == declaration['target_stamp'], '声明的venv真实目标身份改变')
    observed_sha=file_digest(resolved,target_before,observations)
    if observations is not None:observations.update(stage='hash',hash_observed_sha256=observed_sha)
    require(observed_sha == sha256, '声明的venv真实目标SHA不符')
    require(identity(Path(path).lstat()) == link_before and os.readlink(path) == link_text
            and str(Path(path).resolve(strict=True)) == resolved
            and regular_stamp(resolved) == target_before, '读取后venv链接/路径/真实目标改变')
    return {'path':str(path),'kind':'declared_venv_python_symlink',
            'link_stamp':link_before,'readlink':link_text,'resolved_path':resolved,
            'target_stamp':target_before,'sha256':observed_sha,'pre_post_equal':True}


def chrome_metadata(value):
    return {'dev':value.st_dev,'ino':value.st_ino,'size':value.st_size,
            'mtime_ns':value.st_mtime_ns,'ctime_ns':value.st_ctime_ns,
            'file_type':stat.S_IFMT(value.st_mode),'mode':value.st_mode,
            'uid':value.st_uid,'gid':value.st_gid,'nlink':value.st_nlink}


def declared_chrome_identity(path, sha256, declaration, observations):
    """单次读全部六字段+权限owner/link稳定；不允许hash内ctime漂移。"""
    observations['stage']='read_identity'
    before_value=Path(path).lstat()
    before=identity(before_value);before_meta=chrome_metadata(before_value)
    observations.update(lstat_before=before_meta)
    require(stat.S_ISREG(before_value.st_mode),'声明Chrome必须非symlink regular')
    h=hashlib.sha256()
    observations['stage']='open_identity'
    with os.fdopen(os.open(path,os.O_RDONLY|os.O_NOFOLLOW),'rb') as stream:
        opened=os.fstat(stream.fileno());observations['fstat_open']=chrome_metadata(opened)
        observations['changed_fields']=changed_fields(before_meta,observations['fstat_open'])
        require(identity(opened)==before and chrome_metadata(opened)==before_meta,'Chrome open/fstat身份漂移')
        observations['stage']='hash'
        for raw in iter(lambda:stream.read(1048576),b''):h.update(raw)
        observations['stage']='end_fstat'
        finished=os.fstat(stream.fileno());observations['fstat_after']=chrome_metadata(finished)
        observations['changed_fields']=changed_fields(before_meta,observations['fstat_after'])
        require(identity(finished)==before and chrome_metadata(finished)==before_meta,'Chrome hash内完整身份漂移')
    observations['stage']='end_lstat'
    after=Path(path).lstat();observations['lstat_after']=chrome_metadata(after)
    observations['changed_fields']=changed_fields(before_meta,observations['lstat_after'])
    require(identity(after)==before and chrome_metadata(after)==before_meta,'Chrome末lstat完整身份漂移')
    observed_sha=h.hexdigest()
    observations.update(stage='hash',hash_observed_sha256=observed_sha,hash_read_completed_stable=True)
    if observed_sha!=sha256:observations['changed_fields']=['sha256']
    return {'path':path,'kind':'declared_chrome','contract':declaration['contract'],
            **before_meta,'sha256':observed_sha,'within_observation_identity_stable':True}


def changed_fields(before,after,prefix=''):
    """失败也列实得字段，不伪造缺失pre/post数据。"""
    if type(before) is not dict or type(after) is not dict:
        return [prefix or 'receipt'] if before!=after else []
    result=[]
    for key in sorted(set(before)|set(after)):
        name=prefix+'.'+key if prefix else key
        if key not in before or key not in after:result.append(name)
        elif before[key]!=after[key]:result.append(name)
    return result


def chrome_comparison(before,after):
    valid=(type(before) is dict and set(before)==CHROME_RECEIPT_FIELDS
           and before.get('kind')=='declared_chrome' and before.get('contract')==CHROME_CONTRACT
           and type(before.get('path')) is str and Path(before['path']).is_absolute()
           and type(before.get('sha256')) is str and len(before['sha256'])==64
           and all(c in '0123456789abcdef' for c in before['sha256'])
           and all(type(before.get(field)) is int for field in ('dev','ino','size','mtime_ns','ctime_ns','file_type','mode','uid','gid','nlink'))
           and before.get('file_type')==stat.S_IFREG and stat.S_IFMT(before.get('mode',0))==stat.S_IFREG
           and before.get('within_observation_identity_stable') is True)
    if not valid:
        return {'contract':CHROME_CONTRACT,'pass':False,'failure_stage':'cross_identity',
                'changed_fields':changed_fields(before,after),'error':'缺失或不同契约的Chrome pre receipt',
                'endpoint_object_content_equal':False,'ctime_before':before.get('ctime_ns') if type(before) is dict else None,
                'ctime_after':after['ctime_ns']}
    changed=changed_fields(before,after)
    changed_required=[field for field in CHROME_EQUAL_FIELDS if before[field]!=after[field]]
    return {'contract':CHROME_CONTRACT,'pass':not changed_required,'failure_stage':None if not changed_required else 'cross_identity',
            'equal_fields':[field for field in CHROME_EQUAL_FIELDS if before[field]==after[field]],
            'changed_fields':changed,'changed_required_fields':changed_required,
            'ctime_before':before['ctime_ns'],'ctime_after':after['ctime_ns'],
            'metadata_change_observed':before['ctime_ns']!=after['ctime_ns'],
            'endpoint_object_content_equal':not changed_required,
            'full_receipt_equal':before==after,
            'continuous_interval_metadata_unchanged_claim':False,'whole_chrome_bundle_verified_claim':False}


def verify_dependencies(bindings, python_path, python_declaration, before_receipt=None,
                        *, browser_path=None, chrome_declaration=None):
    """同一函数用于pre/post；未声明依赖和任何source仍保持常规文件门。"""
    require(python_path in bindings, '计划依赖全集缺Python')
    require(type(python_declaration) is dict and python_declaration.get('path') == python_path,
            'Python例外路径未唯一绑定')
    # 声明在观测前决定分派，不从原严格比较失败后切换。
    selected=browser_path is not None or chrome_declaration is not None
    declaration_error=None
    if selected:
        try:
            require(type(browser_path) is str and Path(browser_path).is_absolute()
                    and browser_path in bindings and browser_path not in (python_path,python_declaration.get('resolved_path')),
                    'Chrome必须唯一绑定计划browser且不与Python alias/target路径重叠')
            require(type(chrome_declaration) is dict and set(chrome_declaration)=={'path','sha256','contract'}
                    and chrome_declaration['path']==browser_path and chrome_declaration['sha256']==bindings[browser_path]
                    and chrome_declaration['contract']==CHROME_CONTRACT,'Chrome明确事前声明不符')
        except (ValueError,TypeError,KeyError) as exc:declaration_error=str(exc)
    receipts, mismatches, errors, comparisons, observations = {}, [], [], {}, {}
    for path,sha256 in bindings.items():
        observed={'path':path,'stage':'read_identity'}
        observations[path]=observed
        try:
            if selected and declaration_error is not None:
                observed['stage']='declaration'
                raise ValueError(declaration_error)
            if path == python_path:
                receipt=declared_python_identity(path,sha256,python_declaration,observed)
            elif selected and path==browser_path:
                receipt=declared_chrome_identity(path,sha256,chrome_declaration,observed)
            else:
                before=regular_stamp(path)
                observed['stamp']=before
                observed_sha=file_digest(path,before,observed)
                observed.update(stage='hash',hash_observed_sha256=observed_sha)
                if observed_sha!=sha256:observed['changed_fields']=['sha256']
                require(observed_sha == sha256,'常规依赖SHA不符')
                receipt={'path':path,'kind':'regular','stamp':before,'sha256':observed_sha,'pre_post_equal':True}
            # 实得完整观测先存；跨期拒绝不会再丢失post值。
            receipts[path]=receipt
            if selected and path==browser_path:
                require(receipt['sha256']==sha256,'声明Chrome SHA不符')
            if before_receipt is not None:
                observed['stage']='cross_identity'
                prior=before_receipt.get(path)
                if selected and path==browser_path:
                    comparison=chrome_comparison(prior,receipt)
                else:
                    comparison={'contract':'original-full-receipt-equality','pass':path in before_receipt and receipt==prior,
                                'changed_fields':changed_fields(prior,receipt),
                                'failure_stage':None if path in before_receipt and receipt==prior else 'cross_identity'}
                comparisons[path]=comparison
                if not comparison['pass']:
                    if 'pre_post_equal' in receipt:receipt['pre_post_equal']=False
                    raise ValueError('依赖跨期契约不相同')
            else:
                comparisons[path]={'contract':CHROME_CONTRACT if selected and path==browser_path else 'original-full-receipt-equality',
                                   'pass':None,'stage':'pre_observation_only','changed_fields':[]}
            observed['stage']='complete'
        except (OSError,ValueError,RuntimeError) as exc:
            mismatches.append(path)
            errors.append({'path':path,'error':type(exc).__name__+': '+str(exc),'failure_stage':observed['stage']})
            if path not in comparisons:
                comparisons[path]={'pass':False,'failure_stage':observed['stage'],
                    'changed_fields':observed.get('changed_fields',[]),'full_observed_receipt_available':path in receipts}
    if before_receipt is not None:
        for path in sorted(set(before_receipt)-set(bindings)):
            mismatches.append(path)
            errors.append({'path':path,'error':'原pre依赖不在post全集','failure_stage':'bindings_set'})
            comparisons[path]={'pass':False,'failure_stage':'bindings_set','changed_fields':['path_removed'],
                               'full_observed_receipt_available':False}
        for path in sorted(set(bindings)-set(before_receipt)):
            if path not in mismatches:mismatches.append(path)
            errors.append({'path':path,'error':'post新增未绑定的pre路径','failure_stage':'bindings_set'})
            comparisons[path]={'pass':False,'failure_stage':'bindings_set','changed_fields':['path_added'],
                               'full_observed_receipt_available':path in receipts}
    return {'status':'PASS' if not mismatches else 'FAIL','receipts':receipts,
            'mismatches':mismatches,'errors':errors,'comparisons':comparisons,'observations':observations,
            'python_exception_path':python_path,'chrome_contract_path':browser_path if selected else None,
            'all_other_paths_non_symlink_regular':True}
