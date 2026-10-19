"""#1394 未执行的运行路径。只能由具名 Root gate 进入；本轮仅纯离线验证。

导入无启动/数据库副作用。既有 SQLite 永不作为 fresh 域采用；运行后只读自有新库。
"""
import contextlib
import importlib
import os
import signal
import socket
import sqlite3
import stat
import subprocess
import sys
import threading
import time
import traceback
import uuid
import runner as entry
from pathlib import Path

from evidence import (BINARY_SHA, COUNT, SCOPE, canonical, decode, exact, require, sha,
                      cell, validate_final, validate_receipt, validate_snapshot)
from runner import (BINARY, CHECKOUT, file_identity, gate_files, load, native_client,
                    raw_save, received_reply, save, verify_bundle, root_inputs, check_directories, directory_identity, native_controller_paths)


class AbortRun(RuntimeError):
    pass


def error_record(error):
    return {'type': type(error).__name__, 'error': str(error), 'traceback': traceback.format_exc()}


class Deadline:
    """单一 monotonic 绝对限期；每个阻塞调用复用剩余时间，不重置 30s。"""
    def __init__(self, end_ns):
        self.end_ns = end_ns

    def remaining(self):
        value = (self.end_ns - time.monotonic_ns()) / 1_000_000_000
        if value <= 0:
            raise AbortRun('绝对 deadline 已耗尽；不得延长清理/保全预算')
        return value

    def arm(self):
        signal.setitimer(signal.ITIMER_REAL, self.remaining())

    def call(self, function, *args, **kwargs):
        self.arm()
        value = function(*args, **kwargs)
        self.remaining()
        return value


def allocated_bytes(roots):
    """真实 lstat.st_blocks 合算自有目录、DB/WAL/SHM、Controller 与证据；不是采样最大。"""
    seen, total = set(), 0
    def visit(path):
        nonlocal total
        if not os.path.lexists(path):
            return
        info = path.lstat()
        require(not stat.S_ISLNK(info.st_mode) and info.st_uid == os.getuid(), '自有 artifact owner/symlink 不符：' + str(path))
        key = (info.st_dev, info.st_ino)
        if key in seen:
            return
        seen.add(key)
        total += info.st_blocks * 512
        if stat.S_ISDIR(info.st_mode):
            for child in path.iterdir():
                visit(child)
    for root in roots:
        visit(Path(root))
    return total


class OwnerLimits:
    """冻结 caller 私有停机门；不宣称严格峰值或容量证明。"""
    def __init__(self, contract, output):
        self.roots = (output, Path(contract['socket_dir']))
        self.limit = int(contract['caller_observation']['max_owned_artifact_allocated_bytes'])
        self.last_allocated = 0
        self.exceeded = False

    def checkpoint(self, *_):
        self.last_allocated = allocated_bytes(self.roots)
        self.exceeded |= self.last_allocated > self.limit
        require(self.last_allocated <= self.limit, '自有 artifact 实际 st_blocks 超过冻结 4GiB，停止并保全')
        return self.last_allocated

    def guard_write(self, path, size):
        require(any(Path(path).is_relative_to(root) for root in self.roots), '写入不属于精确自有域')
        current = self.checkpoint()
        # 实际已分配量 + 此次写入保守分配估算；写后再次核真实 st_blocks。
        projected = ((size + 4095) // 4096) * 4096
        require(current + projected <= self.limit, '此次证据写入将超 4GiB owner 门；禁止写大快照')

    def before_spawn(self, *_):
        self.checkpoint()

    def after_spawn(self, *_):
        self.checkpoint()


def exchange_once(client, parsed, envelope, raw, ordinal, directory, database, *, active_deadline=None, failure_context=None):
    """异常路径也先留下 exchange 和可核持久状态；不伪造 wire/reply/receipt。"""
    start, transport, failure = time.monotonic_ns(), None, None
    captured = None
    try:
        with Wire(directory, f'input-{ordinal}') as captured:
            transport = client.exchange(parsed['socket'], envelope,
                timeout_ms=int(parsed['s_resources']['response_timeout_ms']),
                max_frame_bytes=int(parsed['s_resources']['max_frame_bytes']))
    except BaseException as error:
        failure = error
        if active_deadline is not None:
            # 失败保全消耗同一个 cleanup30s；不在 finally 另开一个30s。
            failure_context['cleanup_start_ns'] = min(time.monotonic_ns(), active_deadline.end_ns)
            evidence_deadline = Deadline(failure_context['cleanup_start_ns'] + 5_000_000_000)
            database.deadline = evidence_deadline
            evidence_deadline.arm()
    record = dict(result=transport, whole_exchange_elapsed_ns=time.monotonic_ns() - start,
        physical_handle_ns='UNMEASURED', internal_Hash_audit_build_store_SQLCOMMIT_ns='UNMEASURED',
        error=error_record(failure) if failure is not None else None,
        wire_files=[p.name for p in sorted(directory.glob(f'input-{ordinal}-*.*wire'))])
    save(directory / f'input-{ordinal}.exchange.json', record)
    try:
        persistent = database.persistent(envelope, ordinal)
    except BaseException as error:
        persistent = dict(status='UNMEASURED', error=error_record(error), receipt_not_fabricated=True)
        if failure is None:
            failure = error
    save(directory / f'input-{ordinal}.persistent.json', persistent)
    if failure is not None:
        raise failure
    require(transport['transport'] == 'Received', 'DeliveryUnknown/不可达：保存已有持久状态，不重试')
    require(captured.count == 1 and (directory / f'input-{ordinal}-0.request.wire').read_bytes() == raw, '实际原请求 wire 不符')
    reply_raw = (directory / f'input-{ordinal}-0.reply.wire').read_bytes()
    return persistent, reply_raw, received_reply(transport, raw, reply_raw, client)


def load_control():
    # 只在 gate 成功后调用；不执行控制器 CLI，也不启动 Q/HTTP/browser/audit。
    sys.path.insert(0, str(CHECKOUT / 's_session'))
    return importlib.import_module('s_service_control')


class Wire:
    """捕获 socket 实际 sendall 参数和 recv 原字节，不从 decode 结果重建证据。

    sendall 异常时真实送达长度不可知，保留原 send-attempt + UNMEASURED，绝不当成功。
    """
    def __init__(self, directory, label):
        self.directory, self.label, self.count = directory, label, 0

    def __enter__(self):
        observer = self
        self.original = socket.socket

        class ObservedSocket(self.original):
            def __init__(self, *args, **kwargs):
                super().__init__(*args, **kwargs)
                self.observed = self.family == socket.AF_UNIX
                if self.observed:
                    self.index = observer.count
                    observer.count += 1
                    self.sent, self.received, self.events = bytearray(), bytearray(), []
                    self.begin_ns = time.monotonic_ns()
                    self.send_completed, self.recorded = False, False

            def connect(self, address):
                if self.observed:
                    self.events.append(dict(op='connect-attempt', address=str(address), at_ns=time.monotonic_ns()))
                result = super().connect(address)
                if self.observed:
                    self.events.append(dict(op='connect-return', at_ns=time.monotonic_ns()))
                return result

            def sendall(self, raw, *args, **kwargs):
                if self.observed:
                    self.sent.extend(raw)
                    self.events.append(dict(op='sendall-attempt', bytes=len(raw), at_ns=time.monotonic_ns()))
                result = super().sendall(raw, *args, **kwargs)
                if self.observed:
                    self.send_completed = True
                    self.events.append(dict(op='sendall-return', at_ns=time.monotonic_ns()))
                return result

            def recv(self, *args, **kwargs):
                raw = super().recv(*args, **kwargs)
                if self.observed:
                    self.received.extend(raw)
                    self.events.append(dict(op='recv-return', bytes=len(raw), at_ns=time.monotonic_ns()))
                return raw

            def close(self):
                try:
                    if getattr(self, 'observed', False) and not self.recorded:
                        self.recorded = True
                        stem = observer.directory / f'{observer.label}-{self.index}'
                        raw_save(Path(str(stem) + '.request.wire'), bytes(self.sent))
                        raw_save(Path(str(stem) + '.reply.wire'), bytes(self.received))
                        save(Path(str(stem) + '.wire.json'), dict(begin_ns=self.begin_ns,
                             end_ns=time.monotonic_ns(), events=self.events,
                             send_completed=self.send_completed,
                             delivered_byte_extent='COMPLETE_SENDALL_RETURN' if self.send_completed else 'UNMEASURED',
                             request_sha256=sha(self.sent), reply_sha256=sha(self.received),
                             request_bytes=len(self.sent), reply_bytes=len(self.received)))
                finally:
                    return_value = super().close()
                return return_value

            def __exit__(self, *args):
                self.close()

        socket.socket = ObservedSocket
        return self

    def __exit__(self, *args):
        socket.socket = self.original


@contextlib.contextmanager
def observe_processes(directory, handles, lineage, meter):
    original = subprocess.Popen

    class ObservedPopen(original):
        def __init__(self, command, *args, **kwargs):
            is_binary = isinstance(command, (list, tuple)) and command and command[0] == str(BINARY)
            if is_binary:
                require(command[1] in ('init', 'serve'), '禁止运行 recover/audit/其他 binary 命令')
                # 自有 artifact 停机门在 spawn 前核查；严格 RSS/heap peak 仍未测。
                meter.before_spawn(list(command), lineage)
            super().__init__(command, *args, **kwargs)
            self.observed = is_binary
            self.communicate_saved = False
            if is_binary:
                env = kwargs.get('env') or {}
                self.row = dict(pid=self.pid, op=command[1], argv=list(command),
                               parent_pid=os.getpid(), uid=os.getuid(), begin_ns=time.monotonic_ns(),
                               binary_sha256=file_identity(BINARY)['sha256'],
                               resource_sha256=env.get('S_CONTROL_RESOURCE_SHA256'),
                               native_nonce=env.get('S_CONTROL_INSTANCE_ID'), **lineage)
                # 先保留真实 handle，再做可能失败的证据写入，避免丢失已启动子进程。
                handles.append((self, self.row))
                meter.after_spawn(self, self.row)
                require(self.row['binary_sha256'] == BINARY_SHA, 'spawn 后 binary 身份变化')
                if command[1] == 'serve':
                    require(self.row['resource_sha256'] == lineage['frozen_resource_sha256']
                            and self.row['native_nonce'], 'serve 缺实际私有 nonce/resource SHA')
                # native init 没有 control nonce 参数；以 launch_nonce 绑定同域 lineage，不伪造 native nonce。
                save(directory / f'process-{self.pid}.launch.json', self.row)

        def communicate(self, *args, **kwargs):
            result = super().communicate(*args, **kwargs)
            if self.observed and self.row['op'] == 'init' and not self.communicate_saved:
                self.communicate_saved = True
                for name, raw in zip(('stdout', 'stderr'), result):
                    raw_save(directory / f'init.{name}.wire', raw or b'')
            return result

    subprocess.Popen = ObservedPopen
    try:
        yield
    finally:
        subprocess.Popen = original


class OwnedDatabase:
    """只授予本次确实 fresh 的物理域；没有通用打开任意原库的入口。"""
    def __init__(self, directory, database):
        self.directory, self.path = directory, Path(database)
        self.directory_owner = directory_identity(directory)
        self.inode = None
        self.deadline = None
        require(self.path == directory / 's.sqlite' and not os.path.lexists(self.path), '不是 fresh 自有数据库')
        self.init_observed = False

    @contextlib.contextmanager
    def read(self):
        require(self.init_observed and self.path.is_file() and not self.path.is_symlink(), '未观察本次初始化，禁止打开数据库')
        require(exact(directory_identity(self.directory), self.directory_owner), 'DB 目录身份变化')
        info = self.path.lstat()
        require(info.st_uid == os.getuid() and stat.S_ISREG(info.st_mode), '新 DB owner/类型错误')
        current = (info.st_dev, info.st_ino, info.st_uid)
        require(self.inode is None or current == self.inode, '新 DB inode 被替换')
        self.inode = current
        timeout = min(5, self.deadline.remaining()) if self.deadline else 5
        with contextlib.closing(sqlite3.connect(self.path.as_uri() + '?mode=ro', uri=True, timeout=timeout)) as connection:
            if self.deadline:
                connection.set_progress_handler(lambda: int(time.monotonic_ns() >= self.deadline.end_ns), 1000)
            connection.execute('PRAGMA query_only=ON')
            connection.execute('BEGIN')
            yield connection

    @staticmethod
    def query(connection, query, parameters=()):
        cursor = connection.execute(query, parameters)
        names = [item[0] for item in cursor.description]
        rows = [[cell(value) for value in row] for row in cursor]
        rows.sort(key=canonical)
        return dict(names=names, rows=rows, row_count=len(rows), sha256=sha(canonical(rows)))

    def persistent(self, envelope, ordinal):
        with self.read() as connection:
            return dict(
                receipt=self.query(connection, 'SELECT * FROM s_input_messages WHERE source_namespace=? AND source_epoch=? AND message_id=?',
                                   (envelope['source_namespace'], envelope['source_epoch'], envelope['message_id'])),
                publication=self.query(connection, 'SELECT * FROM s_content_publications WHERE generation=?', (ordinal,)),
                delta=self.query(connection, 'SELECT * FROM structure_deltas WHERE generation=?', (ordinal,)),
                clock_events=self.query(connection, 'SELECT * FROM s_clock_events WHERE clock_event_id=?', (envelope['payload']['clock_event_id'],)),
                protocol=self.query(connection, 'SELECT * FROM s_protocol_meta'))

    def snapshot(self):
        with self.read() as connection:
            schema = self.query(connection, 'SELECT type,name,tbl_name,rootpage,sql FROM sqlite_schema ORDER BY type,name')
            tables = {}
            for (name,) in connection.execute("SELECT name FROM sqlite_schema WHERE type='table' ORDER BY name"):
                quoted = '"' + name.replace('"', '""') + '"'
                columns = [list(row) for row in connection.execute('PRAGMA table_xinfo(' + quoted + ')')]
                select = ','.join('"' + col[1].replace('"', '""') + '"' for col in columns)
                value = self.query(connection, 'SELECT ' + select + ' FROM ' + quoted)
                value['columns'] = columns
                tables[name] = value
            return dict(schema=schema, tables=tables,
                        integrity_check=[list(row) for row in connection.execute('PRAGMA integrity_check')],
                        foreign_key_check=[list(row) for row in connection.execute('PRAGMA foreign_key_check')],
                        db_properties={key: connection.execute('PRAGMA ' + key).fetchone()[0]
                                       for key in ('user_version', 'application_id', 'encoding', 'page_size', 'auto_vacuum')})


class Sampler:
    """诊断采样；任何最大样本只报告下界，空样本/缺 PID 显式 UNMEASURED。"""
    def __init__(self, directory, handles, period_ns, limits=None):
        self.directory, self.handles, self.period_ns = directory, handles, period_ns
        self.limits, self.failure = limits, None
        self.end, self.rows = threading.Event(), []
        self.thread = None

    def sample(self):
        if self.limits is not None:
            self.limits.checkpoint()
        row = dict(monotonic_ns=time.monotonic_ns(), files={}, processes=[], maxima_meaning='SAMPLE_LOWER_BOUND')
        for name in ('s.sqlite', 's.sqlite-wal', 's.sqlite-shm'):
            path = self.directory / name
            try:
                current = path.stat()
                row['files'][name] = dict(logical_bytes=current.st_size, allocated_bytes=current.st_blocks * 512)
            except FileNotFoundError:
                row['files'][name] = 'ABSENT_AT_SAMPLE'
        for pid in [os.getpid()] + [process.pid for process, _ in self.handles if process.poll() is None]:
            observed = subprocess.run(['ps', '-p', str(pid), '-o', 'rss='], capture_output=True,
                                      text=True, timeout=1)
            value = observed.stdout.strip()
            row['processes'].append(dict(pid=pid, rss_bytes=int(value) * 1024 if value.isdigit() else 'UNMEASURED'))
        filesystem = os.statvfs(self.directory)
        row['free_disk_sample_bytes'] = filesystem.f_bavail * filesystem.f_frsize
        self.rows.append(row)
        raw = canonical(row) + b'\n'
        if self.limits is not None:
            self.limits.guard_write(self.directory / 'samples.jsonl', len(raw))
        with (self.directory / 'samples.jsonl').open('ab') as target:
            target.write(raw)
            target.flush()
            os.fsync(target.fileno())

    def loop(self):
        while not self.end.wait(self.period_ns / 1_000_000_000):
            try:
                self.sample()
            except Exception as error:
                self.failure = error
                self.rows.append(dict(monotonic_ns=time.monotonic_ns(), measurement='UNMEASURED', error=str(error)))
                break

    def start(self):
        raw_save(self.directory / 'samples.jsonl', b'')
        self.sample()
        self.thread = threading.Thread(target=self.loop, daemon=True)
        self.thread.start()

    def close(self):
        self.end.set()
        if self.thread is not None:
            self.thread.join(timeout=2)
            require(not self.thread.is_alive(), '采样线程未结束')
        samples = [value['rss_bytes'] for row in self.rows for value in row.get('processes', [])
                   if type(value['rss_bytes']) is int]
        gaps = [right['monotonic_ns'] - left['monotonic_ns'] for left, right in zip(self.rows, self.rows[1:])]
        require(self.failure is None, '采样/owner 门失败：' + str(self.failure))
        return dict(sample_count=len(self.rows), target_period_ns=self.period_ns,
                    observed_max_gap_ns=max(gaps) if gaps else 'UNMEASURED',
                    max_rss_sample_bytes=max(samples) if samples else 'UNMEASURED',
                    sample_maxima_are_lower_bounds=True, strict_peak='UNMEASURED', sampler_error=str(self.failure) if self.failure else None,
                    physical_handle_ns='UNMEASURED', internal_Hash_audit_build_store_SQLCOMMIT_ns='UNMEASURED')


def cleanup_case(controller, handles, requested_signal, deadline, directory):
    gates = dict(stop_requested=False, status_stopped=False, stop_confirmed=False,
                 same_handles_reaped=False, socket_removed=False, controller_closed=False)
    errors = []
    if controller is not None:
        try:
            status = deadline.call(controller.status, 's', deadline=deadline.end_ns / 1e9)
            require(status['state'] == 'running', 'STOP 前 status 非 running')
            owned = [row for process, row in handles if row['op'] == 'serve' and process.pid == status['pid']]
            require(len(owned) == 1 and owned[0]['native_nonce'] == status['control_instance_id']
                    and owned[0]['argv'] == status['command'], 'STOP 不属于同一次自有 Popen')
            deadline.call(save, directory / 'STOP-REQUEST.json', dict(action='Controller.stop', status=status,
                          signal=signal.Signals(requested_signal).name))
            gates['stop_requested'] = True
            stopped = deadline.call(controller.stop, 's', requested_signal)
            deadline.call(save, directory / 'STOP-RESULT.json', stopped)
            require(stopped['state'] == 'stopped' and stopped['pid'] == status['pid'], 'stop 未确认停止')
            gates['stop_confirmed'] = True
            after = deadline.call(controller.status, 's', deadline=deadline.end_ns / 1e9)
            deadline.call(save, directory / 'STOPPED-STATUS.json', after)
            require(after['state'] == 'stopped', 'STOP 后 status 未停止')
            gates['status_stopped'] = True
        except BaseException as error:
            errors.append(error_record(error))
    reaped = []
    for process, row in handles:
        try:
            code = deadline.call(process.wait, timeout=deadline.remaining())
            row.update(returncode=code, end_observed_ns=time.monotonic_ns(), same_Popen_reaped=True)
            require(process.poll() is not None and code == (0 if row['op'] == 'init' else -requested_signal), '原 Popen reap/退出码失败')
            reaped.append(row)
        except BaseException as error:
            row.update(same_Popen_reaped=False, reap_error=str(error))
            errors.append(error_record(error))
    gates['same_handles_reaped'] = len(handles) == len(reaped) == 2
    if controller is not None:
        try:
            require(gates['status_stopped'] and gates['same_handles_reaped'], '前置清理门失败，保留 socket')
            deadline.arm()
            with Wire(directory, 'cleanup-socket'):
                result = controller.prepare_socket(deadline.end_ns / 1e9)
            deadline.call(save, directory / 'SOCKET-CLEANUP.json', result)
            require(not os.path.lexists(controller.config['socket']), 'socket 仍存在')
            gates['socket_removed'] = True
        except BaseException as error:
            errors.append(error_record(error))
        try:
            deadline.call(controller.close)
            gates['controller_closed'] = True
        except BaseException as error:
            errors.append(error_record(error))
    try:
        deadline.call(save, directory / 'PROCESSES-REAPED.json', [row for _, row in handles])
    except BaseException as error:
        errors.append(error_record(error))
    return gates, errors


def run_case(label, output, contract, plan, material_lines, clock, meter, checkpoint):
    begin_ns = time.monotonic_ns()
    caller = contract['caller_observation']
    active_deadline = Deadline(begin_ns + int(caller['per_case_observation_deadline_ms']) * 1_000_000)
    old_alarm = signal.getsignal(signal.SIGALRM)
    signal.signal(signal.SIGALRM, lambda *_: (_ for _ in ()).throw(AbortRun('case/cleanup 绝对 deadline 耗尽')))
    active_deadline.arm()
    directory = output / label
    result = dict(label=label, status='PARTIAL', committed_count=0, errors=[], preservation={}, case_begin_ns=begin_ns)
    failure_context = {}
    handles, controller, database, sampler = [], None, None, None
    resource_original, ready_stem = None, None
    requested_signal = signal.SIGTERM if label == 'A' else signal.SIGINT
    lineage = dict(launch_nonce=plan['domains'][label]['launch_nonce'],
                   resource_contract_sha256=plan['contract_sha256'], frozen_resource_sha256=plan['resource_sha256'])
    try:
        domain = next(item for item in contract['domains'] if item['label'] == label)
        directory.mkdir(mode=0o700)
        meter.checkpoint()
        database = OwnedDatabase(directory, domain['db'])
        database.deadline = active_deadline
        client, control = native_client(), load_control()
        config = root_inputs()[1][label]
        # 使用被冻结 Controller 原字节；不改 DB/socket/S7/timeout/Q21。
        raw_save(directory / 'Controller.json', (entry.ROOT_INPUTS / f'Controller-{label}.json').read_bytes())
        parsed = control.read_config(directory / 'Controller.json')
        require(exact(parsed, native_controller_paths(config)), '正式 read_config 偏离原生路径解析/冻结 Controller')
        sampler = Sampler(directory, handles, int(caller['sampling_period_ms']) * 1_000_000, meter)
        sampler.start()
        controller = control.Controller(parsed, Path(domain['state_dir']))
        with observe_processes(directory, handles, lineage, meter):
            with Wire(directory, 'start-ready') as wire:
                ready = controller.start_s('1')
            save(directory / 'START-RESULT.json', ready)
            require(ready['state'] == 'ready' and ready['initialization'] is not None, '正式 fresh init/Ready 未完成')
            database.init_observed = True
            identity = controller.status('s')
            require(identity['state'] == 'running' and controller.socket_identity() == identity['socket_identity'], 'Ready 后 PID/socket 身份失效')
            resource_original = file_identity(identity['resource_identity']['path'])
            require(resource_original['sha256'] == plan['resource_sha256'] and resource_original['mode'] == '0o600', '原私有资源 SHA/权限不符')
            require(identity['control_instance_id'] == ready['reply']['payload']['control_instance_id'], 'Ready 不属于实际 native nonce')
            for index in range(wire.count):
                candidate = directory / f'start-ready-{index}.reply.wire'
                try:
                    if client.decode_frame(candidate.read_bytes()) == ready['reply']:
                        ready_stem = f'start-ready-{index}'
                        break
                except ValueError:
                    pass
            require(ready_stem is not None, '没有对应 Ready 原 wire')
            save(directory / 'RUNNING-IDENTITY.json', identity)
            initial = database.snapshot()
            validate_snapshot(initial)
            save(directory / 'initialized.fullDB.json', initial)
            checkpoint(label + '-ready')
            raw_save(directory / 'PROGRESS.jsonl', b'')
            for ordinal, raw in enumerate(material_lines, 1):
                meter.checkpoint(label, ordinal)
                require(sampler.failure is None, '采样/owner 门失败：' + str(sampler.failure))
                remaining_ns = active_deadline.end_ns - time.monotonic_ns()
                require(remaining_ns >= int(parsed['s_resources']['response_timeout_ms']) * 1_000_000, '剩余域预算不足一次原完整期限，停止而不缩短原 S 响应期限')
                envelope = client.decode_frame(raw)
                persistent, reply_raw, payload = exchange_once(client, parsed, envelope, raw, ordinal, directory, database, active_deadline=active_deadline, failure_context=failure_context)
                meter.checkpoint(label, ordinal)
                validate_receipt(persistent, raw, ordinal, payload, clock)
                progress = canonical(dict(ordinal=ordinal, request_sha256=sha(raw[:-1]),
                         reply_wire_sha256=sha(reply_raw), persistent_sha256=sha(canonical(persistent)), status='Committed',
                         committed_validation_finished_ns=time.monotonic_ns())) + b'\n'
                meter.guard_write(directory / 'PROGRESS.jsonl', len(progress))
                with (directory / 'PROGRESS.jsonl').open('ab') as target:
                    target.write(progress)
                    target.flush()
                    os.fsync(target.fileno())
                result['committed_count'] = ordinal
                if ordinal % int(contract['original_checkpoint_every']) == 0:
                    checkpoint(label + '-committed-' + str(ordinal))
            require(result['committed_count'] == COUNT, 'partial 不得 complete')
    except BaseException as error:
        result['errors'].append(error_record(error))
        failure_context.setdefault('cleanup_start_ns', min(time.monotonic_ns(), active_deadline.end_ns))
    finally:
        # 观察窗到期/失败/正常完成均只获同一个 cleanup30s；其中最后5s留给保全。
        cleanup_start = failure_context.get('cleanup_start_ns', min(time.monotonic_ns(), active_deadline.end_ns))
        preservation_deadline = Deadline(cleanup_start + int(caller['cleanup_timeout_ms']) * 1_000_000)
        cleanup_deadline = Deadline(preservation_deadline.end_ns - 5_000_000_000)
        result['cleanup_start_ns'] = cleanup_start
        result['cleanup_deadline_ns'] = preservation_deadline.end_ns
        operation_deadline = Deadline(preservation_deadline.end_ns - 1_000_000_000)
        try:
            result['cleanup'], errors = cleanup_case(controller, handles, requested_signal, cleanup_deadline, directory)
            result['errors'].extend(errors)
        except BaseException as error:
            result['cleanup'] = {}
            result['errors'].append(error_record(error))
        # 每项独立尝试；sampler/status/STOP 失败不再跳过 final snapshot。
        if database is not None:
            database.init_observed |= any(row['op'] == 'init' and process.poll() == 0 for process, row in handles)
            database.deadline = operation_deadline
        def preserve(name, function):
            try:
                operation_deadline.call(function)
                result['preservation'][name] = 'SAVED'
            except BaseException as error:
                result['preservation'][name] = dict(status='UNMEASURED', error=error_record(error))
                result['errors'].append(error_record(error))
        if sampler is not None:
            preserve('samples', lambda: result.update(samples=sampler.close()))
        preserve('originals', lambda: checkpoint(label + '-after-stop'))
        def snapshot():
            require(database is not None and database.init_observed, '本次初始化未核实，不能打开数据库')
            final = database.snapshot()
            validate_snapshot(final)
            save(directory / 'final.fullDB.json', final)
            # A 在这里逐源核完整 raw/receipt/clock；失败保留快照并禁止启动 B。
            validate_final(final, material_lines, clock)
        preserve('final_snapshot', snapshot)
        preserve('db_files', lambda: save(directory / 'OWNED-DB-FILES-AFTER.json', [file_identity(Path(domain['db'] + suffix))
                 for suffix in ('', '-wal', '-shm') if Path(domain['db'] + suffix).is_file()]))
        preserve('lifecycle', lambda: save(directory / 'LIFECYCLE.json', dict(processes=[row for _, row in handles],
                 ready_wire_stem=ready_stem, resource_original=resource_original, **lineage)))
        preserve('allocation', lambda: meter.checkpoint(label, 'after-preservation'))
        result['case_end_observed_ns'] = time.monotonic_ns()
        result['whole_case_elapsed_ns'] = result['case_end_observed_ns'] - begin_ns
        if result['committed_count'] == COUNT and result['cleanup'] and all(result['cleanup'].values()) and not result['errors']:
            result['status'] = 'VERIFIED'
        try:
            preservation_deadline.call(save, directory / 'CASE-RESULT.json', result)
        finally:
            # 直到清理、snapshot/save 尝试全部结束才恢复外层；不留下无限保全区间。
            signal.setitimer(signal.ITIMER_REAL, 0)
            signal.signal(signal.SIGALRM, old_alarm)
    return result


def run_ab(prepared_dir):
    prepared, contract, output, directories = gate_files(prepared_dir)
    # fresh 与祖先身份再次紧邻创建核查；不接管 Root 以外任意路径。
    check_directories(directories)
    require(not os.path.lexists(output) and not os.path.lexists(contract['socket_dir']), 'fresh 路径在启动前被占用')
    output.mkdir(mode=0o700)
    check_directories(directories)
    socket_dir = Path(contract['socket_dir'])
    socket_dir.mkdir(mode=0o700)
    check_directories(directories)
    owned_dirs = [directory_identity(output), directory_identity(socket_dir)]
    meter = OwnerLimits(contract, output)
    previous_guard = entry.WRITE_GUARD
    entry.WRITE_GUARD = meter.guard_write
    cases, errors, serial = [], [], 0
    final_deadline = Deadline(time.monotonic_ns() + int(contract['caller_observation']['cleanup_timeout_ms']) * 1_000_000)
    old_signals = {sig: signal.getsignal(sig) for sig in (signal.SIGTERM, signal.SIGINT, signal.SIGALRM)}
    for sig in (signal.SIGTERM, signal.SIGINT):
        signal.signal(sig, lambda signum, _: (_ for _ in ()).throw(AbortRun('外部中止：' + signal.Signals(signum).name)))
    signal.signal(signal.SIGALRM, lambda *_: (_ for _ in ()).throw(AbortRun('最终保全绝对 deadline 耗尽')))
    originals = prepared['originals'] + [file_identity(Path(prepared_dir) / 'PREPARED.json')]
    originals = sorted({r['path']:r for r in originals}.values(), key=lambda r:r['path'])
    def checkpoint(label, enforce=True):
        nonlocal serial
        serial += 1
        observed = []
        for row in originals:
            try:
                observed.append(file_identity(row['path']))
            except BaseException as error:
                observed.append(dict(path=row['path'], observation='UNMEASURED', error=str(error)))
        save(output / f'ORIGINALS-CHECKPOINT-{serial:05d}.json', dict(label=label, originals=observed))
        if enforce:
            require(exact(observed, originals), '原件变化/无法核实；失败快照已保存')
        return observed
    result = dict(status='FAIL_OR_PARTIAL', complete9011=False, scope=SCOPE, cases=cases, errors=errors,
        full_resource_capacity=False, strict_RSS_heap_IO_stage_peaks='UNMEASURED',
        crosswalk_complete=False, Client_complete=False, C_complete=False, wholegraph_complete=False)
    try:
        final_deadline.arm()
        raw_save(output / 'RESOURCE.json', (entry.ROOT_INPUTS / 'TestOnly-frozen9011-fullclock-serial-first-attempt.s-resources.json').read_bytes())
        raw_save(output / 'PREPARED.json', (Path(prepared_dir) / 'PREPARED.json').read_bytes())
        save(output / 'CONTRACT.json', contract)
        save(output / 'ORIGINALS-BEFORE.json', originals)
        save(output / 'MEASUREMENT-GAPS.json', dict(strict_RSS_heap_IO_stage_peaks='UNMEASURED',
              sample_maxima_are_lower_bounds=True, full_resource_capacity=False))
        plan = dict(scope=SCOPE, count_per_domain=COUNT, order=['A', 'B'], inputs=prepared['inputs'],
            contract_sha256=sha(canonical(contract)), resource_sha256=contract['resource_sha256'],
            domains={label: dict(launch_nonce=str(uuid.uuid4()), expected_returncode=-int(signal.SIGTERM if label == 'A' else signal.SIGINT)) for label in ('A','B')},
            retries=False, Q_started=False, HTTP_started=False, audit_command_started=False,
            crosswalk_complete=False, Client_complete=False, C_complete=False, wholegraph_complete=False)
        save(output / 'RUN-PLAN.json', plan)
        lines = (Path(prepared['inputs']) / 'envelopes.draft.jsonl').read_bytes().splitlines(keepends=True)
        clock = (Path(prepared['inputs']) / 'clock-plan.full9011.draft.json').read_bytes()
        for label in ('A', 'B'):
            check_directories(owned_dirs)
            # 下一 case 的所有工作都在 case1h 内，run_case 首行立即接管 ITIMER。
            case = run_case(label, output, contract, plan, lines, clock, meter, checkpoint)
            final_deadline = Deadline(case['cleanup_deadline_ns'])
            cases.append(case)
            require(case['status'] == 'VERIFIED', label + ' 失败/partial；禁止推进下一域')
    except BaseException as error:
        errors.append(error_record(error))
    finally:
        # 全局目录清理与首末保全共用最后 case 的 cleanup30s，不另给逐操作30s。
        operations_end = Deadline(final_deadline.end_ns - 500_000_000)
        def final_step(function, *, summary=False):
            try:
                return (final_deadline if summary else operations_end).call(function)
            except BaseException as error:
                errors.append(error_record(error))
        def after():
            observed = checkpoint('final-or-abort', enforce=False)
            save(output / 'ORIGINALS-AFTER.json', observed)
            require(exact(observed, originals), '末次原件变化/UNMEASURED')
        final_step(after)
        def cleanup_directory():
            check_directories(owned_dirs)
            require(not list(socket_dir.iterdir()), 'socket 目录未清空，保留失败原件')
            socket_dir.rmdir()
            save(output / 'SOCKET-DIRECTORY-CLEANUP.json', dict(removed_owned_directory=True))
        final_step(cleanup_directory)
        if not errors:
            verified = final_step(lambda: verify_bundle(output))
            if verified is not None and not errors:
                result.update(verified)
        if errors:
            result.update(status='FAIL_OR_PARTIAL', complete9011=False)
        try:
            final_step(lambda: save(output / 'RUN-RESULT.json', result), summary=True)
            if errors:
                result.update(status='FAIL_OR_PARTIAL', complete9011=False)
        finally:
            signal.setitimer(signal.ITIMER_REAL, 0)
            for sig, handler in old_signals.items():
                signal.signal(sig, handler)
            entry.WRITE_GUARD = previous_guard
    return result
