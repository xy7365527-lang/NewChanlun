#!/usr/bin/env python3
"""#1404：正式 S/Q、自然前缀、正常停机恢复及既有页面；无故障注入。"""
import argparse
import json
import os
import signal
from pathlib import Path
import subprocess

from tb02a_runtime import Tb02aRun, write_json
from tb02c_verify import verify


class Tb02cRun(Tb02aRun):
    def __init__(self, case, playwright_module, **kwargs):
        super().__init__(fault_index=None, fault_stage='after_batch', **kwargs)
        self.case = case
        self.playwright_module = playwright_module

    def collect_core(self):
        records = super().collect_core()
        # 保存的是正式固定 cut 的完整公共结果；进程、连接与控制代际单列。
        before = [self.query({'id': f'before-normal-{n}', 'client': 'history', 'op': 'load',
                              'mode': 'AsKnown', 'generation': str(n)}) for n in (16, 28)]
        stopped = self.control('stop-s')
        recovered = self.control('recover', '--new-epoch', '2', '--clock-event-id', 'recover-normal')
        started = self.control('start-s', '--writer-epoch', '2')
        after = [self.query({'id': f'after-normal-{n}', 'client': 'history', 'op': 'load',
                             'mode': 'AsKnown', 'generation': str(n)}) for n in (16, 28)]
        if before != after:
            raise ValueError('正常恢复改写了已发布的段/来源/关系/变化或旧cut')
        write_json(self.output / 'normal-recovery.json', {'stop': stopped, 'recover': recovered, 'start': started,
                                                        'cuts': [16, 28], 'equal': True})
        with (self.output / 'semantic-core.jsonl').open('a') as stream:
            for n, a, b in zip((16, 28), before, after):
                stream.write(json.dumps({'kind': 'normal_recovery_cut', 'generation': str(n), 'before': a, 'after': b}, ensure_ascii=False, sort_keys=True) + '\n')
        write_json(self.output / 'query-oracle.json', verify(Path(self.config['db']), self.case, Path(self.node)))
        command = [self.node, str(self.root / 'tests/tb02c_browser.cjs'), '--base-url', 'http://127.0.0.1:' + self.config['port'],
                   '--output', str(self.output / 'browser'), '--playwright-module', str(self.playwright_module)]
        write_json(self.output / 'browser-command.json', command)
        with (self.output / 'browser.stdout').open('x') as out, (self.output / 'browser.stderr').open('x') as err:
            child = subprocess.Popen(command, stdout=out, stderr=err, start_new_session=True)
            try:
                child.wait(timeout=180)
            except BaseException:
                os.killpg(child.pid, signal.SIGTERM)
                try:
                    child.wait(timeout=5)
                except subprocess.TimeoutExpired:
                    os.killpg(child.pid, signal.SIGKILL)
                    child.wait(timeout=5)
                raise
        if child.returncode:
            raise ValueError('真实浏览器验证失败，见browser.stderr')
        return records + 2


if __name__ == '__main__':
    p = argparse.ArgumentParser(description=__doc__)
    for name in ('config', 'messages', 'output', 'node', 'playwright-module'):
        p.add_argument('--' + name, type=Path, required=True)
    p.add_argument('--case', choices=('first_up', 'first_down'), required=True)
    def cancel(_signum, _frame):
        raise RuntimeError('采集停止；回收本次浏览器与S/Q并保留失败原件')
    signal.signal(signal.SIGTERM, cancel)
    signal.signal(signal.SIGINT, cancel)
    result = Tb02cRun(**vars(p.parse_args())).execute()
    print(json.dumps(result, ensure_ascii=False))
    raise SystemExit(1 if result['status'] == 'FAIL' else 0)
