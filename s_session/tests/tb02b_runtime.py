#!/usr/bin/env python3
"""#1392：正式 launcher 的新笔运行，原消息 SIGKILL/恢复与公共观察原件。

独立采集扩展（本票）：`--initial-as-known` 在 S/Q 起来、尚无输入时用独立 client 取正式
AsKnown0，并在后续输入/修订之后重访比较；`--prefix-map`/`--provenance` 把 prefix→实际 cut
映射与输入来源原样带进证据；`--browser-lifecycle-asof` 只在计划显式选择时才要求浏览器断言
生命周期（不再用 generation>=12 隐式推定）。本驱动只采集，不宣布业务验收。
"""
import argparse
from concurrent.futures import ThreadPoolExecutor
import hashlib
import json
import os
import signal
from pathlib import Path
import subprocess
import time

from tb02a_runtime import Tb02aRun, canonical, helper_exchange, write_json
from tb01c_runtime import exchange, require_kill_receipt, validate_reply, wait_for_marker
from tb02b_verify import verify
from s_service_control import process_identity


def write_json_over(path, value):
    """本驱动的 RESULT.json 在采集收尾后补写：保留原件目录内同名文件。"""
    Path(path).write_text(json.dumps(value, ensure_ascii=False, indent=2) + "\n")


class Tb02bRun(Tb02aRun):
    def __init__(self, playwright_module, oracle_case=None, initial_as_known=False, prefix_map=None,
                 provenance=None, browser_lifecycle_asof=None, fault_index=None, fault_stage="after_batch", **args):
        super().__init__(fault_index=fault_index, fault_stage=fault_stage, **args)
        self.playwright_module = playwright_module
        self.oracle_case = oracle_case
        self.initial_as_known = bool(initial_as_known)
        self.browser_lifecycle_asof = None if browser_lifecycle_asof is None else str(browser_lifecycle_asof)
        self.prefix_map_path = Path(prefix_map).resolve() if prefix_map else None
        self.provenance_path = Path(provenance).resolve() if provenance else None
        self.prefix_map = json.loads(self.prefix_map_path.read_text()) if self.prefix_map_path else None
        self.initial_candidate = None
        self.initial_failure = None
        if self.prefix_map_path is not None:
            (self.output / 'prefix-map.json').write_bytes(self.prefix_map_path.read_bytes())
            self.source_hashes[str(self.prefix_map_path)] = hashlib.sha256((self.output / 'prefix-map.json').read_bytes()).hexdigest()
        if self.provenance_path is not None:
            (self.output / 'provenance.json').write_bytes(self.provenance_path.read_bytes())
            self.source_hashes[str(self.provenance_path)] = hashlib.sha256((self.output / 'provenance.json').read_bytes()).hexdigest()

    def after_start(self):
        """S/Q 已起、尚无输入：独立 client 取正式 AsKnown0；不碰 live/watch client。"""
        if not self.initial_as_known:
            return
        try:
            candidate = self.query({'id': 'initial-as-known-0', 'client': 'initial', 'op': 'load',
                                    'mode': 'AsKnown', 'generation': '0'})
        except Exception as exc:
            self.initial_failure = {'error': type(exc).__name__ + ': ' + str(exc)}
            write_json_over(self.output / 'initial-as-known.json',
                            {'prefix': 0, 'cut': 0, 'mode': 'AsKnown', 'status': 'query_failed',
                             **self.initial_failure})
            self.emit('initial_state_failure', detail=self.initial_failure['error'])
            return
        self.initial_candidate = candidate
        write_json_over(self.output / 'initial-as-known.json',
                        {'prefix': 0, 'cut': 0, 'mode': 'AsKnown', 'status': 'captured',
                         'cursor': candidate.get('cursor'), 'pages': candidate.get('pages'),
                         'candidate_sha256': hashlib.sha256(canonical(candidate)).hexdigest()})

    def revisit_initial_state(self):
        if not self.initial_as_known:
            return
        record = {'prefix': 0, 'cut': 0, 'mode': 'AsKnown'}
        if self.initial_candidate is None:
            record.update({'status': 'skipped_initial_unavailable', 'failure': self.initial_failure})
            write_json_over(self.output / 'initial-as-known-revisit.json', record)
            return
        actual = self.query({'id': 'initial-as-known-0-revisit', 'client': 'initial', 'op': 'load',
                             'mode': 'AsKnown', 'generation': '0'})
        equal = canonical(actual) == canonical(self.initial_candidate)
        record.update({'status': 'compared', 'equal': equal,
                       'candidate_sha256': hashlib.sha256(canonical(actual)).hexdigest()})
        write_json_over(self.output / 'initial-as-known-revisit.json', record)
        if not equal:
            raise ValueError('后续输入/修订改变了 prefix0/AsKnown0 的完整公共候选')

    def execute(self):
        result = super().execute()
        result['initial_as_known'] = {'requested': self.initial_as_known, 'prefix': 0,
                                      'captured': self.initial_candidate is not None,
                                      'failure': self.initial_failure}
        if self.prefix_map is not None:
            result['prefix_map'] = self.prefix_map
        if self.provenance_path is not None:
            result['provenance_sha256'] = hashlib.sha256((self.output / 'provenance.json').read_bytes()).hexdigest()
        if self.initial_failure is not None:
            # 产品不支持 prefix0 初态时保留失败：其余采集照旧完整，交父级处理，不伪造初态。
            result['status'] = 'FAIL'
            result['error'] = 'InitialAsKnown0未取得（保留失败交父处理）：' + self.initial_failure['error']
        write_json_over(self.output / 'RESULT.json', result)
        return result

    def files_at_fault(self, label):
        entries = []
        for suffix in ('', '-wal', '-shm'):
            path = Path(self.config['db'] + suffix)
            if path.exists():
                raw = path.read_bytes()
                target = self.output / (label + '-db' + suffix)
                target.write_bytes(raw)
                entries.append({'original_path': str(path), 'copy': str(target), 'bytes': len(raw),
                                'sha256': hashlib.sha256(raw).hexdigest()})
        write_json(self.output / (label + '-files.json'), entries)
        return entries

    def inject_fault(self, index):
        self.control('stop-s')
        marker = self.output / (self.fault_stage + '.marker')
        environment = dict(os.environ)
        environment.update(S_SESSION_PAUSE=self.fault_stage, S_SESSION_PAUSE_MARKER=str(marker), S_SESSION_PAUSE_DB=self.config['db'])
        environment.pop('S_SESSION_PAUSE_RELEASE', None)
        self.control('start-s', '--writer-epoch', '1', environment=environment)
        peer, writer = self.identity('q'), self.identity('s')
        with ThreadPoolExecutor(max_workers=1) as pool:
            pending = pool.submit(self.send, index, time.monotonic_ns())
            wait_for_marker(marker, {'pid': str(writer['pid']), 'stage': self.fault_stage,
                                    'db': self.config['db'], 'exe': self.config['binary']}, time.monotonic() + 20)
            published = index + int(self.fault_stage == 'after_commit')
            before = self.observed_frontier()
            if before != {'generation': str(published), 'accepted': str(index + 1), 'committed': str(published)}:
                raise ValueError('实际故障前沿不在具名事务边界')
            self.files_at_fault('before-kill')
            stop = self.control('stop-s', '--signal', 'KILL')
            require_kill_receipt(stop, 's', writer)
            self.files_at_fault('after-kill')
            if pending.result(timeout=15)['transport'] != 'DeliveryUnknown':
                raise ValueError('没有原消息的真实丢回执轨迹')
        if self.identity('q') != peer:
            raise ValueError('S 故障改变了独立 Q 的进程身份')
        self.control('recover', '--new-epoch', '2', '--clock-event-id', 'recover-001')
        self.control('start-s', '--writer-epoch', '2')
        after = self.observed_frontier()
        if after != {'generation': str(index + 1), 'accepted': str(index + 1), 'committed': str(index + 1)}:
            raise ValueError('原接纳消息未在 Ready 前恢复')
        self.files_at_fault('after-recover')
        self.lifecycle = {'stage': self.fault_stage, 'signal': 'SIGKILL', 'before': before, 'after': after,
                          'writer_epoch': '2', 'q_survived': self.identity('q') == peer}
        self.emit('fault', old_s=writer, new_s=self.identity('s'), q=peer, stop_receipt=stop)

    def raw_watch(self, label, cursor):
        response = helper_exchange(self.helper, {'id': label, 'client': 'negative', 'op': 'rawWatch',
                                                'cursor': cursor, 'client_id': 'tb02b-' + label, 'max_batches': '4'})
        receipt = json.loads(Path(response['path']).read_text())
        if receipt.get('result') == 'captured_envelope_validated':
            return json.loads(Path(receipt['envelope']['path']).read_text())['payload']
        if receipt.get('committed_changed'):
            raise ValueError('失败游标改变了客户端已安装 cut')
        return {'rejected': True, 'error': receipt.get('error', '')}

    def collect_core(self):
        # 后续输入/修订之后再访一次 prefix0/AsKnown0；无论结果如何都不改 live/watch 既有语义。
        self.revisit_initial_state()
        # 原消息重投，不改消息/输入身份；保全独立回执与接纳前沿。
        original = self.messages[-1]
        before = self.observed_frontier()
        duplicate = exchange(self.config['socket'], original, timeout_ms=10000,
                             max_frame_bytes=int(self.config['s_resources']['max_frame_bytes']))
        current_epoch = '2' if self.fault_index is not None else '1'
        fenced = original['payload']['writer_epoch'] != current_epoch
        write_json(self.output / 'duplicate-receipt.json', {'original': original, 'result': duplicate,
                   'current_writer_epoch': current_epoch,
                   'expected': 'StaleWriter_then_original_identity_receipt' if fenced else 'Committed_duplicate'})
        if duplicate['transport'] != 'Received':
            raise ValueError('重复原消息未得到权威回执')
        result = validate_reply(original, duplicate['response'], producer='S', producer_epoch=current_epoch)
        if self.observed_frontier() != before:
            raise ValueError('原消息重投改变了已恢复的提交前沿')
        if fenced:
            # #1374 已有写入隔离：旧 writer 请求先被拒绝；不得改写原消息来冒用新 epoch。
            # super().collect_core 随后用原消息四元身份查询正式 receipt，逐条要求 Committed。
            if result.get('ok') is not False or not result.get('error', '').startswith('StaleWriter'):
                raise ValueError('恢复后旧 writer 原消息未被准确隔离')
        elif result.get('kind') != 'Committed':
            raise ValueError('同 writer 原消息未幂等命中原提交')
        # 游标检查必须仍取自 live 记录：initial/AsKnown0 与重访记录不能改写这里的语义。
        live = [item for item in self.public if item['command'].get('client') == 'live']
        if not live:
            raise ValueError('缺少 live 正式首 cut 公共记录，拒绝用其他 client 代取游标')
        first = live[0]['candidate']['cursor']
        stale = self.raw_watch('old-cursor', first)
        if len(self.messages) > 5 and stale.get('gap', {}).get('reason') != 'delivery_retention_gap':
            raise ValueError('旧 cursor 未显式返回保留窗口缺口')
        bad = dict(live[-1]['candidate']['cursor'], order_version='s-record-order/999')
        rejected = self.raw_watch('wrong-version', bad)
        if not rejected.get('rejected'):
            raise ValueError('版本不匹配 cursor 被接受')
        write_json(self.output / 'cursor-checks.json', {'old_cursor': stale, 'wrong_version': rejected})
        write_json(self.output / 'query-oracle.json', verify(Path(self.config['db']), self.oracle_case, Path(self.node)))
        command = [self.node, str(self.root / 'tests/tb02b_browser.cjs'), '--base-url', 'http://127.0.0.1:' + self.config['port'],
                   '--output', str(self.output / 'browser'), '--playwright-module', str(self.playwright_module),
                   '--generation', str(len(self.messages))]
        if self.browser_lifecycle_asof is not None:
            # 只在计划显式选择时断言生命周期；不再用 generation>=12 隐式推定。
            command += ['--lifecycle-asof', self.browser_lifecycle_asof]
        write_json(self.output / 'browser-command.json', command)
        with (self.output / 'browser.stdout').open('xb') as out, (self.output / 'browser.stderr').open('xb') as err:
            child = subprocess.Popen(command, stdout=out, stderr=err, start_new_session=True)
            try:
                write_json(self.output / 'browser-process.json', {'pid': child.pid, 'identity': process_identity(child.pid), 'command': command})
                child.wait(timeout=120)
            except BaseException:
                os.killpg(child.pid, signal.SIGTERM)
                try:
                    child.wait(timeout=5)
                except subprocess.TimeoutExpired:
                    os.killpg(child.pid, signal.SIGKILL)
                    child.wait(timeout=5)
                raise
        if child.returncode:
            raise ValueError('真实浏览器验收失败，原件见 browser.stderr')
        return super().collect_core()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    for name in ('config', 'messages', 'output', 'node', 'playwright-module'):
        parser.add_argument('--' + name, type=Path, required=True)
    parser.add_argument('--oracle-case')
    parser.add_argument('--fault-index', type=int)
    parser.add_argument('--fault-stage', choices=('after_begin', 'after_batch', 'before_commit', 'after_commit'), default='after_batch')
    parser.add_argument('--initial-as-known', action='store_true',
                        help='独立 client 取正式 prefix0/AsKnown0 并在后续输入后重访比较')
    parser.add_argument('--prefix-map', type=Path, help='prefix→实际 cut 映射原件，原样带入证据')
    parser.add_argument('--provenance', type=Path, help='输入来源/原件 SHA/版本原件，原样带入证据')
    parser.add_argument('--browser-lifecycle-asof', help='显式浏览器生命周期断言的 as-of 代际（可选）')
    def cancel(_signum, _frame):
        raise RuntimeError('采集收到终止请求；回收本次 browser/helper/S/Q 后保留失败原件')

    signal.signal(signal.SIGTERM, cancel)
    result = Tb02bRun(**vars(parser.parse_args())).execute()
    print(json.dumps(result, ensure_ascii=False))
    return 0 if result['status'] == 'CAPTURED_REQUIRES_DOUBLE_RUN_ORACLE_AND_BROWSER' else 1


if __name__ == '__main__':
    raise SystemExit(main())
