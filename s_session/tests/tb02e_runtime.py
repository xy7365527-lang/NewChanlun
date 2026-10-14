#!/usr/bin/env python3
"""#1419 S9（根裁 D-E (B)）：正式 S/Q、自然前缀、正常停机恢复及既有页面；无故障注入。

照 TB-02-D 的 tb02d_runtime：逐条 accept/advance，每条后 live + AsKnown 公共查询，全部回访；
恢复切面取冻结账簿的 recovery_cuts（第一个完成段出现的 27 与末条 43），之后对冻结 oracle 核对并起真实浏览器。
"""
import argparse
import json
import os
import signal
import subprocess
import time
from pathlib import Path

from tb02d_runtime import Tb02dRun, bounded_operation
from tb02a_runtime import write_json
from tb02e_verify import verify


class Tb02eRun(Tb02dRun):
    def __init__(self, case, playwright_module, **kwargs):
        # 跳过 Tb02dRun 读 tb02d 账簿的那一步，其余初始化同 TB-02-D。
        super(Tb02dRun, self).__init__(fault_index=None, fault_stage='after_batch', **kwargs)
        self.case = case
        self.playwright_module = playwright_module
        oracle = json.loads((self.root / 'tests/fixtures/tb02e/hand-oracle.json').read_text())
        first, last = oracle['recovery_cuts']
        if last != len(self.messages):
            raise ValueError('冻结账簿的末恢复切面不是本次输入条数')
        self.recovery_cuts = (first, last)

    def complete_after_core(self, records):
        before = [self.query({'id': f'before-normal-{n}', 'client': 'history', 'op': 'load',
                              'mode': 'AsKnown', 'generation': str(n)}) for n in self.recovery_cuts]
        stopped = self.control('stop-s')
        recovered = self.control('recover', '--new-epoch', '2', '--clock-event-id', 'recover-normal')
        started = self.control('start-s', '--writer-epoch', '2')
        after = [self.query({'id': f'after-normal-{n}', 'client': 'history', 'op': 'load',
                             'mode': 'AsKnown', 'generation': str(n)}) for n in self.recovery_cuts]
        if before != after:
            raise ValueError('正常恢复改写了已发布的中枢/归属/版本对象或旧cut')
        write_json(self.output / 'normal-recovery.json', {'stop': stopped, 'recover': recovered, 'start': started,
                                                        'cuts': list(self.recovery_cuts), 'equal': True})
        with (self.output / 'semantic-core.jsonl').open('a') as stream:
            for n, a, b in zip(self.recovery_cuts, before, after):
                stream.write(json.dumps({'kind': 'normal_recovery_cut', 'generation': str(n), 'before': a, 'after': b},
                                        ensure_ascii=False, sort_keys=True) + '\n')
        with bounded_operation('frozen_oracle', 180):
            write_json(self.output / 'query-oracle.json', verify(Path(self.config['db']), self.case, Path(self.node)))
        command = [self.node, str(self.root / 'tests/tb02e_browser.cjs'), '--base-url', 'http://127.0.0.1:' + self.config['port'],
                   '--output', str(self.output / 'browser'), '--playwright-module', str(self.playwright_module),
                   '--final-cut', str(self.recovery_cuts[1]), '--pending-cut', str(self.recovery_cuts[0])]
        write_json(self.output / 'browser-command.json', command)
        with (self.output / 'browser.stdout').open('x') as out, (self.output / 'browser.stderr').open('x') as err:
            child = subprocess.Popen(command, stdout=out, stderr=err, start_new_session=True)
            try:
                child.wait(timeout=30)
                if child.returncode:
                    raise ValueError('真实浏览器验证失败，见browser.stderr')
            except BaseException:
                cleanup_errors = []
                # leader 退出不代表浏览器子孙已退出；按本次独立进程组回收（同 TB-02-D）。
                reap_deadline = time.monotonic() + 5
                for sig in (signal.SIGTERM, signal.SIGKILL):
                    try:
                        os.killpg(child.pid, sig)
                    except ProcessLookupError:
                        pass
                    except BaseException as exc:
                        cleanup_errors.append(repr(exc))
                    try:
                        remaining = max(0, reap_deadline - time.monotonic())
                        child.wait(timeout=min(1, remaining) if sig == signal.SIGTERM else remaining)
                    except subprocess.TimeoutExpired:
                        continue
                    except BaseException as exc:
                        cleanup_errors.append(repr(exc))
                self.emit('browser_cleanup', errors=cleanup_errors, returncode=child.returncode)
                raise
        return records + 2


if __name__ == '__main__':
    p = argparse.ArgumentParser(description=__doc__)
    for name in ('config', 'messages', 'output', 'node', 'playwright-module'):
        p.add_argument('--' + name, type=Path, required=True)
    p.add_argument('--case', choices=('min12_up', 'min12_down'), required=True)

    def cancel(_signum, _frame):
        raise RuntimeError('采集停止；回收本次浏览器与S/Q并保留失败原件')
    signal.signal(signal.SIGTERM, cancel)
    signal.signal(signal.SIGINT, cancel)
    result = Tb02eRun(**vars(p.parse_args())).execute()
    print(json.dumps(result, ensure_ascii=False))
    raise SystemExit(1 if result['status'] == 'FAIL' else 0)
