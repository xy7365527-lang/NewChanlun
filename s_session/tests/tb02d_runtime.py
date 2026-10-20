#!/usr/bin/env python3
"""#1414：正式 S/Q、自然前缀、正常停机恢复及既有页面；无故障注入。"""
import argparse
import json
import os
import signal
import contextlib
import time
from pathlib import Path
import subprocess

from tb02a_runtime import Tb02aRun, write_json
from tb02d_verify import verify


@contextlib.contextmanager
def bounded_operation(label, seconds=30):
    """在线动作默认30秒；具名离线批处理显式传窗，不修改S/Q资源政策。"""
    previous = signal.getsignal(signal.SIGALRM)
    def expired(_signum, _frame):
        raise TimeoutError(label + f" exceeded fixed {seconds} second deadline")
    signal.signal(signal.SIGALRM, expired)
    signal.setitimer(signal.ITIMER_REAL, seconds)
    try:
        yield
    finally:
        signal.setitimer(signal.ITIMER_REAL, 0)
        signal.signal(signal.SIGALRM, previous)


class Tb02dRun(Tb02aRun):
    def __init__(self, case, playwright_module, **kwargs):
        super().__init__(fault_index=None, fault_stage='after_batch', **kwargs)
        self.case = case
        self.playwright_module = playwright_module
        oracle = json.loads((self.root/'tests/fixtures/tb02d/hand-oracle.json').read_text())['cases'][case]
        self.recovery_cuts = (oracle['first_first_known_input_count'], len(self.messages))

    def query(self, command):
        start = time.monotonic_ns()
        try:
            with bounded_operation('Q/' + command['id']):
                return super().query(command)
        finally:
            self.emit('bounded_query', command=command, deadline_seconds=30,
                      duration_ns=str(time.monotonic_ns()-start))

    def cleanup(self):
        # 本片helper最多等5s再杀；任何helper错误不跳过S/Q回收。
        errors = []
        if self.helper is not None:
            helper = self.helper
            try:
                helper.stdin.close()
                code = helper.wait(timeout=5)
                if code != 0:
                    errors.append({'action':'bounded-helper-exit','returncode':code})
            except BaseException as exc:
                errors.append({'action':'bounded-helper-close','error':repr(exc)})
                try:
                    helper.kill()
                    helper.wait(timeout=5)
                except BaseException as kill_error:
                    errors.append({'action':'bounded-helper-kill','error':repr(kill_error)})
            finally:
                try:
                    helper.stdout.close()
                except Exception as exc:
                    errors.append({'action':'bounded-helper-output','error':repr(exc)})
                self.helper = None
        result = super().cleanup()
        result['errors'].extend(errors)
        return result

    def collect_core(self):
        with bounded_operation('collect_core', 120):
            records = super().collect_core()
        return self.complete_after_core(records)

    def complete_after_core(self, records):
        # 保存的是正式固定 cut 的完整公共结果；进程、连接与控制代际单列。
        before = [self.query({'id': f'before-normal-{n}', 'client': 'history', 'op': 'load',
                              'mode': 'AsKnown', 'generation': str(n)}) for n in self.recovery_cuts]
        stopped = self.control('stop-s')
        recovered = self.control('recover', '--new-epoch', '2', '--clock-event-id', 'recover-normal')
        started = self.control('start-s', '--writer-epoch', '2')
        after = [self.query({'id': f'after-normal-{n}', 'client': 'history', 'op': 'load',
                             'mode': 'AsKnown', 'generation': str(n)}) for n in self.recovery_cuts]
        if before != after:
            raise ValueError('正常恢复改写了已发布的段/来源/关系/变化或旧cut')
        write_json(self.output / 'normal-recovery.json', {'stop': stopped, 'recover': recovered, 'start': started,
                                                        'cuts': list(self.recovery_cuts), 'equal': True})
        with (self.output / 'semantic-core.jsonl').open('a') as stream:
            for n, a, b in zip(self.recovery_cuts, before, after):
                stream.write(json.dumps({'kind': 'normal_recovery_cut', 'generation': str(n), 'before': a, 'after': b}, ensure_ascii=False, sort_keys=True) + '\n')
        with bounded_operation('frozen_oracle', 180):
            write_json(self.output / 'query-oracle.json', verify(Path(self.config['db']), self.case, Path(self.node)))
        command = [self.node, str(self.root / 'tests/tb02d_browser.cjs'), '--base-url', 'http://127.0.0.1:' + self.config['port'],  # DevSkim: ignore DS162092 有意使用本机回环（本地服务/测试），非调试残留
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
                # leader退出不代表browser子孙已退出；按本次独立进程组回收。
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
    p.add_argument('--case', choices=('second_open_up', 'second_open_down'), required=True)
    def cancel(_signum, _frame):
        raise RuntimeError('采集停止；回收本次浏览器与S/Q并保留失败原件')
    signal.signal(signal.SIGTERM, cancel)
    signal.signal(signal.SIGINT, cancel)
    result = Tb02dRun(**vars(p.parse_args())).execute()
    print(json.dumps(result, ensure_ascii=False))
    raise SystemExit(1 if result['status'] == 'FAIL' else 0)
