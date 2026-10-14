#!/usr/bin/env python3
"""#1419 S9 第三轮正式运行驱动（根批 07479fd501：harness 带 --browser-deferred-to 1497，四臂全跑；只此一次，失败不重试；帽/期限不改，二进制同首轮）。

开跑前门 → probe_slot 快照 → tb02e_harness.py --execute（四臂 + a/b 比较）→ 回收与 SLOT-REAP。
重输出（每臂 HTTP 收据、轨迹、库）落外盘 RUN；本目录 DELIVER 只放门收据、背景负载采样、harness 输出与回收收据。
运行期间每 30 s 采样负载与磁盘；数据卷可用 < 15 GiB 时向 harness 发 SIGINT（harness 自行回收本臂 S/Q/浏览器）。
"""
import glob, hashlib, json, os, re, shutil, signal, subprocess, sys, threading, time
from pathlib import Path

DELIVER = Path('/private/tmp/goal1323-native-20261012/tb1419-impl/s9-formal-r3')
RUN = Path('/Volumes/AgentStorage/issue1419-s9-formal-r3-20261014.noindex')
WT = Path('/Users/silencehan/.codex/worktrees/goal1323-tb1419/NewChanlun')
HEAD = '07479fd501a72fe202dbc0b41a8250b6a5047841'
SS = WT / 's_session'
BIN = Path('/private/tmp/goal1323-native-20261012/tb1419-impl/artifacts/s_structure_session-48e2386463')
BIN_SHA = '38c61eeed8ba79a7c5ec1d74575ca46c9cf32614ded9c5d9f04294f18642245b'
PY = Path('/tmp/tb1414-runtime-venv/bin/python')
NODE = Path('/Users/silencehan/.cache/codex-runtimes/codex-primary-runtime/dependencies/node/bin/node')
PW = Path('/Users/silencehan/.cache/codex-runtimes/codex-primary-runtime/dependencies/node/node_modules/playwright')
CHROME = '/Applications/Google Chrome.app/Contents/MacOS/Google Chrome'
SLOT = Path('/private/tmp/goal1323-native-20261012/tb1419-impl/f1-s0b/probe_slot.py')
BUSY = Path('/private/tmp/goal1323-native-20261012/tb1394-integration-build/tools/peer_build_busy.py')
PORTS, SOCK = '18860-18863', '/private/tmp/tb1419e-'
STOP_GIB = 15
FORBIDDEN = re.compile(r'(^|/)(cargo|rustc)( |$)|s_structure_session|s_query\.py|s_readonly_server|tb02[a-e]_(harness|runtime|browser)'
                       r'|f1a\.py|s0b_probe\.py|probe_slot\.py|min12_parser_run|lake (build|env)|tb1481|(^|/)hdiutil( |$)')


def now():
    return time.strftime('%Y-%m-%dT%H:%M:%S%z')


def sh(cmd, **kw):
    return subprocess.run(cmd, capture_output=True, text=True, **kw)


def sha(path):
    h = hashlib.sha256()
    with open(path, 'rb') as f:
        for block in iter(lambda: f.read(1 << 20), b''):
            h.update(block)
    return h.hexdigest()


def ps_rows():
    out = sh(['ps', '-axo', 'pid=,ppid=,pcpu=,rss=,command=']).stdout
    rows = []
    for line in out.splitlines():
        p = line.split(None, 4)
        if len(p) == 5:
            rows.append({'pid': int(p[0]), 'ppid': int(p[1]), 'pcpu': float(p[2]), 'rss_kb': int(p[3]), 'command': p[4]})
    return rows


def top(rows, k=12):
    return [{**r, 'command': r['command'][:200]} for r in sorted(rows, key=lambda r: -r['pcpu'])[:k]]


def free_gib(path):
    return round(shutil.disk_usage(path).free / 2**30, 2)


def background(label):
    rows = ps_rows()
    chrome = [r for r in rows if 'Google Chrome' in r['command']]
    mds = {k: round(sum(r['pcpu'] for r in rows if r['command'].endswith('/' + k) or ('/' + k + ' ') in r['command']), 1)
           for k in ('mds_stores', 'mds', 'mdworker_shared')}
    return {'at': now(), 'label': label, 'loadavg': os.getloadavg(), 'ncpu': os.cpu_count(), 'spotlight_pcpu': mds,
            'data_free_gib': free_gib('/private/tmp'), 'agentstorage_free_gib': free_gib('/Volumes/AgentStorage'),
            'chrome_total_pcpu': round(sum(r['pcpu'] for r in chrome), 1), 'chrome_processes': len(chrome),
            'top': top(rows)}


def preflight():
    me = os.getpid()
    rows = ps_rows()
    forbidden = [{**r, 'command': r['command'][:300]} for r in rows if r['pid'] != me and FORBIDDEN.search(r['command'])]
    busy = sh([sys.executable, str(BUSY), *sorted(glob.glob('/private/tmp/goal1323-native-20261012/*target*'))])
    lo, hi = PORTS.split('-')
    listen = [l for l in sh(['lsof', '-nP', f'-iTCP:{lo}-{hi}', '-sTCP:LISTEN']).stdout.splitlines()[1:] if l.strip()]
    git_head = sh(['git', '-C', str(WT), 'rev-parse', 'HEAD']).stdout.strip()
    git_dirty = sh(['git', '-C', str(WT), 'status', '--porcelain']).stdout
    remote_tracking = sh(['git', '-C', str(WT), 'rev-parse', 'origin/codex/1419-center-seed-20261013']).stdout.strip()
    gate = {
        'git_head': git_head, 'git_head_ok': git_head == HEAD, 'git_clean': git_dirty == '', 'origin_tracking_head': remote_tracking,
        'binary': str(BIN), 'binary_sha256': sha(BIN), 'binary_sha_ok': sha(BIN) == BIN_SHA,
        'peer_build_busy': busy.stdout.strip(), 'peer_build_idle': busy.returncode == 0,
        'forbidden_processes': forbidden, 'ports': PORTS, 'port_listeners': listen,
        'stale_sockets': sorted(glob.glob(SOCK + '*.sock')), 'run_dir_absent': not RUN.exists(),
        'data_free_gib': free_gib('/private/tmp'), 'agentstorage_free_gib': free_gib('/Volumes/AgentStorage'),
        'python': sh([str(PY), '-c', 'import sys;print(sys.version.split()[0])']).stdout.strip(),
        'node': sh([str(NODE), '--version']).stdout.strip(),
        'playwright': json.loads((PW / 'package.json').read_text())['version'],
        'chrome_path': CHROME, 'chrome_version': sh(['plutil', '-extract', 'CFBundleShortVersionString', 'raw',
                                                    '/Applications/Google Chrome.app/Contents/Info.plist']).stdout.strip(),
        'chrome_sha256': sha(CHROME),
    }
    gate['ok'] = (gate['git_head_ok'] and gate['git_clean'] and gate['binary_sha_ok'] and gate['peer_build_idle']
                  and not forbidden and not listen and not gate['stale_sockets'] and gate['run_dir_absent']
                  and gate['data_free_gib'] >= STOP_GIB)
    mds = background('mds-check')['spotlight_pcpu']
    gate['spotlight_pcpu'] = mds
    gate['mds_stores_settled'] = mds['mds_stores'] < 20
    gate['ok'] = gate['ok'] and gate['mds_stores_settled']
    return gate


def main():
    drive = {'schema': 'tb1419-s9-drive/1', 'started_at': now(), 'run_dir': str(RUN), 'deliver_dir': str(DELIVER)}
    gate = preflight()
    gate['background'] = background('preflight')
    (DELIVER / 'PREFLIGHT.json').write_text(json.dumps(gate, ensure_ascii=False, indent=1) + '\n')
    if not gate['ok']:
        drive.update(status='PREFLIGHT_FAIL', ended_at=now())
        (DELIVER / 'DRIVE.json').write_text(json.dumps(drive, ensure_ascii=False, indent=1) + '\n')
        print(json.dumps({'preflight': 'FAIL'}))
        return 2
    snap = sh([sys.executable, str(SLOT), 'snapshot', str(DELIVER)])
    drive['snapshot'] = {'rc': snap.returncode, 'stdout': snap.stdout.strip(), 'stderr': snap.stderr.strip()}

    env = dict(os.environ, TB02E_CHROMIUM_EXECUTABLE=CHROME, PYTHONDONTWRITEBYTECODE='1')
    cmd = [str(PY), str(SS / 'tests/tb02e_harness.py'), '--output', str(RUN), '--binary', str(BIN), '--python', str(PY),
           '--node', str(NODE), '--playwright-module', str(PW), '--execute', '--browser-deferred-to', '1497']
    drive['harness_command'] = cmd
    drive['harness_env_added'] = {'TB02E_CHROMIUM_EXECUTABLE': CHROME, 'PYTHONDONTWRITEBYTECODE': '1'}
    stop = threading.Event()
    samples = (DELIVER / 'background-samples.jsonl').open('x')
    stopline = {}

    with (DELIVER / 'harness.stdout').open('x') as out, (DELIVER / 'harness.stderr').open('x') as err:
        t0, w0 = time.monotonic(), now()
        child = subprocess.Popen(cmd, stdout=out, stderr=err, cwd=str(SS / 'tests'), env=env, start_new_session=True)

        def in_browser():
            return [p.parent.parent.name for p in RUN.glob('*/evidence/browser.stdout')
                    if not (p.parent / 'browser/READINESS-DIAGNOSTICS.json').exists()]

        def sampler():
            last = 0.0
            while not stop.wait(1):
                el = time.monotonic() - t0
                arms = in_browser()
                if el - last < (2 if arms else 10):
                    continue
                last = el
                s = background('browser-window' if arms else 'during')
                s['browser_arms'] = arms
                s['elapsed_s'] = round(el, 1)
                samples.write(json.dumps(s, ensure_ascii=False) + '\n'); samples.flush()
                if s['data_free_gib'] < STOP_GIB and not stopline:
                    stopline.update(at=now(), data_free_gib=s['data_free_gib'])
                    try:
                        os.killpg(child.pid, signal.SIGINT)
                    except ProcessLookupError:
                        pass
        th = threading.Thread(target=sampler, daemon=True); th.start()
        try:
            rc = child.wait(timeout=7200)
        except subprocess.TimeoutExpired:
            os.killpg(child.pid, signal.SIGINT)
            rc = child.wait(timeout=120)
            drive['driver_timeout'] = True
        stop.set(); th.join(5); samples.close()
        drive.update(harness_rc=rc, harness_started_at=w0, harness_ended_at=now(), harness_elapsed_s=round(time.monotonic() - t0, 1))
    if stopline:
        drive['disk_stopline'] = stopline

    # 回收：本票进程 = 命令行或祖先含 RUN 路径或 socket 前缀；快照复制进 RUN 供 probe_slot 读取。
    reap_out = RUN if RUN.exists() else DELIVER
    if reap_out != DELIVER:
        shutil.copy2(DELIVER / 'SLOT-SNAPSHOT.json', RUN / 'SLOT-SNAPSHOT.json')
    reap = sh([sys.executable, str(SLOT), 'reap', str(reap_out), '--ports', PORTS, '--socket-prefix', SOCK, '--kill'])
    drive['reap'] = {'out': str(reap_out), 'rc': reap.returncode, 'stdout': reap.stdout.strip(), 'stderr': reap.stderr.strip()}
    if reap_out != DELIVER and (RUN / 'SLOT-REAP.json').exists():
        shutil.copy2(RUN / 'SLOT-REAP.json', DELIVER / 'SLOT-REAP.json')
    drive['background_after'] = background('after')
    drive['ended_at'] = now()
    drive['status'] = 'HARNESS_EXIT_0' if drive['harness_rc'] == 0 else 'HARNESS_EXIT_NONZERO'
    (DELIVER / 'DRIVE.json').write_text(json.dumps(drive, ensure_ascii=False, indent=1) + '\n')
    print(json.dumps({'harness_rc': drive['harness_rc'], 'elapsed_s': drive['harness_elapsed_s'], 'reap_rc': reap.returncode}))
    return 0


if __name__ == '__main__':
    sys.exit(main())
