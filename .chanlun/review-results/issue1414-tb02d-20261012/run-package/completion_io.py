"""#1414 browser-only：新core与停库全组custody。未获新授权不得调用。"""
import contextlib
import hashlib
import json
import os
from pathlib import Path
import stat
import time


def require(ok, detail):
    if not ok:
        raise ValueError(detail)


def stamp(path):
    value = Path(path).lstat()
    require(stat.S_ISREG(value.st_mode), '来源必须是非symlink常规文件：' + str(path))
    return [value.st_dev, value.st_ino, value.st_size, value.st_mtime_ns, value.st_ctime_ns]


def fd_stamp(stream):
    value = os.fstat(stream.fileno())
    return [value.st_dev, value.st_ino, value.st_size, value.st_mtime_ns, value.st_ctime_ns]


def deadline_check(deadline):
    if time.monotonic() >= deadline:
        raise TimeoutError('原绝对期限耗尽；不得重试或扩窗')


def open_source(path):
    # O_NOFOLLOW把lstat/open之间的symlink替换也挡住。
    return os.fdopen(os.open(path, os.O_RDONLY | os.O_NOFOLLOW), 'rb')


def dump(path, value):
    with Path(path).open('x') as stream:
        json.dump(value, stream, ensure_ascii=False, indent=2, allow_nan=False)
        stream.write('\n')
        stream.flush()
        os.fsync(stream.fileno())


class CopyLine:
    """一份原source流，每块同步写副本/hash；原strict parser读取本适配器。"""
    def __init__(self, source, copy, whole_hash, deadline, chunk_bytes, authority=None, fingerprint_parser=None):
        self.source, self.copy, self.whole_hash = source, copy, whole_hash
        self.deadline, self.chunk_bytes, self.authority = deadline, chunk_bytes, authority
        self.finished = False
        self.bytes = self.newlines = 0
        self.line_hash = hashlib.sha256()
        self.last = b''
        self.fingerprint_parser=fingerprint_parser

    def read(self, size):
        if size == 0 or self.finished:
            return b''
        deadline_check(self.deadline)
        size = self.chunk_bytes if size < 0 else min(size, self.chunk_bytes)
        raw = self.source.readline(size)
        self.copy.write(raw)
        if self.authority is not None:
            self.authority.write(raw)
        self.whole_hash.update(raw)
        self.line_hash.update(raw)
        if self.fingerprint_parser is not None and raw:
            self.fingerprint_parser.send(raw)
        self.bytes += len(raw)
        self.newlines += raw.count(b'\n')
        self.last = raw[-1:] or self.last
        self.finished = not raw or raw.endswith(b'\n')
        return raw


def seal_core(source, destination, authority_directory, expected_stamp, deadline, expected_prefix219):
    """单遍原221的复制+严格布局；不调用旧219构造或输入/Q重放。"""
    from tb02c_compare import _record_identity, CHUNK_BYTES
    from tb02d_harness import expected_layout
    import ijson
    from record_fingerprint import RecordFingerprint
    source, destination, authority_directory = map(Path, (source, destination, authority_directory))
    layout = expected_layout(40)
    require(len(layout) == 221, '原事前expected_layout40不是221')
    require(expected_prefix219['lines'] == 219, '须有具名继承219完整SHA/bytes绑定')
    before = stamp(source)
    require(before == expected_stamp, '原221与已有stat证据不符')
    authority_directory.mkdir()
    digest = hashlib.sha256()
    records, total_bytes, total_newlines = [], 0, 0
    known, recovery_bindings = {}, []
    with open_source(source) as incoming, destination.open('xb') as outgoing:
        require(fd_stamp(incoming) == before, 'core打开时身份漂移')
        for index, expected in enumerate(layout):
            deadline_check(deadline)
            filename = None
            if expected['kind'] == 'authoritative_table':
                filename = expected['table'] + '.jsonl'
            elif expected['kind'] == 'authoritative_schema':
                filename = 'schema.jsonl'
            path = authority_directory / filename if filename else None
            selected = None
            if expected.get('command.id') in ('known-26','known-39'):
                selected = {('candidate',)}
            elif expected['kind'] == 'normal_recovery_cut':
                selected = {('before',),('after',)}
            observer = RecordFingerprint(selected) if selected else None
            target = observer.target() if observer else None
            if target is not None:next(target)
            parser = ijson.basic_parse_coro(target) if target is not None else None
            with path.open('xb') if path else contextlib.nullcontext(None) as authority:
                line = CopyLine(incoming, outgoing, digest, deadline, CHUNK_BYTES, authority,parser)
                try:
                    actual = _record_identity(line)
                finally:
                    if parser is not None:parser.close()
                    if target is not None:target.close()
                require(actual == expected, f'第{index+1}行身份/次序不符')
                require(line.newlines == 1 and line.last == b'\n', '每个完整JSON对象必须恰有末尾换行')
                if authority is not None:
                    authority.flush()
                    os.fsync(authority.fileno())
                total_bytes += line.bytes
                total_newlines += line.newlines
                if index == 218:
                    require(total_bytes == expected_prefix219['bytes']
                            and digest.copy().hexdigest() == expected_prefix219['sha256'],
                            '新core前219不等于原已验219完整SHA/bytes')
                if observer is not None:
                    fingerprints=observer.complete()
                    if expected['kind']=='public_candidate':
                        generation='27' if expected['command.id']=='known-26' else '40'
                        known[generation]=fingerprints[('candidate',)]
                    else:
                        generation=expected['generation']
                        require(generation in known and fingerprints[('before',)] == fingerprints[('after',)] == known[generation],
                                '恢复before/after全字段不等于已验prefix219原known-'+generation)
                        recovery_bindings.append({'generation':generation,'known_candidate_sha256':known[generation],
                                                  'before_sha256':fingerprints[('before',)],'after_sha256':fingerprints[('after',)]})
                records.append({'record':index+1, 'identity':actual, 'bytes':line.bytes,
                                'sha256':line.line_hash.hexdigest(),
                                'authority_copy':str(path) if path else None})
        require(not incoming.read(1), '221布局末尾有额外字节')
        require(fd_stamp(incoming) == before and stamp(source) == before, 'core读取期间身份漂移')
        outgoing.flush()
        os.fsync(outgoing.fileno())
        require(fd_stamp(outgoing)[2] == total_bytes == before[2], 'core副本bytes不符')
    deadline_check(deadline)
    require(len(recovery_bindings)==2 and set(known)=={'27','40'}, '恢复两行全内容绑定未完整成立')
    return {'status':'SINGLE_SOURCE_PASS_STRICT221_COPIED', 'source':str(source),
            'copy':str(destination), 'source_stamp':before, 'sha256':digest.hexdigest(),
            'copy_stamp':stamp(destination),
            'bytes':total_bytes, 'lines':total_newlines, 'layout':records,
            'source_passes':1, 'independent_copy_reread':False,
            'verified_prefix219':expected_prefix219,
            'recovery_full_structure_bindings':recovery_bindings,
            'structure_fingerprint':'complete_typed_json_maps_key_sorted_arrays_ordered_no_field_exclusions',
            'original_compare_exact_traces_required_for_later_pair':True}


def copy_file(source, destination, deadline, expected=None):
    source, destination = Path(source), Path(destination)
    before = stamp(source)
    digest, count = hashlib.sha256(), 0
    with open_source(source) as incoming, destination.open('xb') as outgoing:
        require(fd_stamp(incoming) == before, 'copy打开时来源身份漂移')
        while True:
            deadline_check(deadline)
            raw = incoming.read(1048576)
            if not raw:
                break
            outgoing.write(raw)
            digest.update(raw)
            count += len(raw)
        require(fd_stamp(incoming) == before and stamp(source) == before, 'copy来源身份漂移')
        outgoing.flush()
        os.fsync(outgoing.fileno())
        require(fd_stamp(outgoing)[2] == count == before[2], 'copy长度不符')
    if expected is not None:
        require(count == expected['bytes'] and digest.hexdigest() == expected['sha256'], 'sealed→working完整bytes/SHA不符')
    return {'source':str(source), 'copy':str(destination), 'bytes':count,
            'sha256':digest.hexdigest(), 'source_stamp':before}


def group_stamps(database):
    # 不把WAL/SHM的缺失容错为“无需复制”；本案事前全组存在。
    return {str(Path(str(database)+suffix)):stamp(Path(str(database)+suffix))
            for suffix in ('', '-wal', '-shm')}


def seal_database_group(database, sealed_directory, working_directory, expected_stamps,
                        deadline, assert_stopped, history_bindings=None, history_directory=None):
    """原件从未SQLite打开；sealed从未SQLite打开；仅新working可操作。"""
    database = Path(database)
    sealed_directory, working_directory = Path(sealed_directory), Path(working_directory)
    stopped_before = assert_stopped()
    before = group_stamps(database)
    history_bindings=history_bindings or {}
    history_before={path:stamp(path) for path in history_bindings}
    require(before == expected_stamps, '停库DB/WAL/SHM与具名stat集合不符')
    sealed_directory.mkdir()
    working_directory.mkdir()
    sealed = []
    for suffix in ('', '-wal', '-shm'):
        source = Path(str(database)+suffix)
        sealed.append(copy_file(source, sealed_directory / source.name, deadline))
    history=[]
    if history_bindings:
        Path(history_directory).mkdir()
        for index,(path,sha) in enumerate(history_bindings.items()):
            history.append(copy_file(path,Path(history_directory)/(f'{index:02}-'+Path(path).name),deadline,
                                     {'bytes':history_before[path][2],'sha256':sha}))
    require(group_stamps(database) == before, '全组复制期间来源集合/身份改变')
    require(all(stamp(path)==before_stamp for path,before_stamp in history_before.items()),
            '历史controller身份材料在全组复制中漂移')
    stopped_after = assert_stopped()
    working = []
    for entry in sealed:
        working.append(copy_file(entry['copy'], working_directory / Path(entry['copy']).name,
                                 deadline, entry))
    require(group_stamps(database) == before, '创建working期间原停库集合改变')
    require(all(stamp(path)==before_stamp for path,before_stamp in history_before.items()),
            '创建working期间历史controller材料漂移')
    return {'status':'STOPPED_THREE_FILE_CUSTODY_CREATED_NOT_STATE_IDENTITY_PROOF',
            'source_group':before, 'sealed':sealed, 'working':working,
            'history_stamps':history_before,'history_copies':history,
            'historical_owner_locks_process_records_not_installed_into_working':True,
            'stopped_before':stopped_before, 'stopped_after':stopped_after,
            'original_sqlite_opened':False, 'sealed_sqlite_opened':False,
            'state_identity_requires_exact_protocol_transition_and_all_tables_schema':True}
