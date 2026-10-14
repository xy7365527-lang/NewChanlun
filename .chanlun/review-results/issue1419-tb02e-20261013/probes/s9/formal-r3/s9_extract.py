#!/usr/bin/env python3
"""#1419 S9 正式运行收据抽取（只读 RUN，不起服务、不改原件）。

产出（DELIVER）：raw/（小件原件副本）、BIG-ARTIFACTS.json（外盘大件 SHA）、ARM-TIMINGS.json、RESULT.json、SHA256SUMS。
浏览器点击耗时：tb02e_browser.cjs 的 READINESS 事件上限 128 条，会被请求事件占满，点击标记丢失；
故按 HAR 请求时刻（startedDateTime+time）与浏览器各产物的落盘时刻（APFS mtime）分段推导，口径写进 RESULT。
"""
import hashlib, json, os, shutil, sys
from datetime import datetime
from pathlib import Path

DELIVER = Path('/private/tmp/goal1323-native-20261012/tb1419-impl/s9-formal-r3')
RUN = Path('/Volumes/AgentStorage/issue1419-s9-formal-r3-20261014.noindex')
ARMS = ('min12_up-a', 'min12_up-b', 'min12_down-a', 'min12_down-b')
CAP_S = 30.0
SMALL = ['EXIT.json', 'INTERRUPTED.json', 'stdout', 'stderr', 'config.json', 'clock.json', 'messages.jsonl',
         'evidence/RESULT.json', 'evidence/events.jsonl', 'evidence/query-oracle.json', 'evidence/normal-recovery.json',
         'evidence/browser-command.json', 'evidence/browser.stdout', 'evidence/browser.stderr', 'evidence/http-helper.stderr',
         'evidence/source-before-run/MANIFEST.json']
BIG = ['evidence/semantic-core.jsonl', 'evidence/database-backup.sqlite', 'session.sqlite', 'session.sqlite-wal']


def sha(path):
    h = hashlib.sha256()
    with open(path, 'rb') as f:
        for block in iter(lambda: f.read(1 << 22), b''):
            h.update(block)
    return h.hexdigest()


def load(path):
    try:
        return json.loads(Path(path).read_text())
    except Exception as exc:  # 缺件照实记
        return {'_missing_or_invalid': repr(exc)[:200]}


def iso(ts):
    return datetime.fromtimestamp(ts).astimezone().isoformat(timespec='milliseconds')


def har_time(s):
    return datetime.fromisoformat(s.replace('Z', '+00:00')).timestamp()


def stats(values):
    if not values:
        return None
    v = sorted(values)
    return {'n': len(v), 'min': round(v[0], 3), 'median': round(v[len(v) // 2], 3), 'max': round(v[-1], 3), 'sum': round(sum(v), 3)}


def arm_timings(arm):
    ev = RUN / arm / 'evidence'
    events = [json.loads(l) for l in (ev / 'events.jsonl').read_text().splitlines() if l.strip()]
    ingest = [(int(e['operation']), (int(e['received_ns']) - int(e['sent_ns'])) / 1e9) for e in events if e['kind'] == 'ingest']
    queries = [(e['command']['id'], int(e['duration_ns']) / 1e9) for e in events if e['kind'] == 'bounded_query']
    controls = [(e['action'], int(e['duration_ns']) / 1e9, int(e['observed_monotonic_ns'])) for e in events if e['kind'] == 'control']
    groups = {}
    for qid, t in queries:
        groups.setdefault(qid.split('-')[0], []).append(t)
    out = {'ingest_reply_s': stats([t for _, t in ingest]), 'ingest_last5': [[i + 1, round(t, 3)] for i, t in ingest[-5:]],
           'ingest_max': max(ingest, key=lambda x: x[1]) if ingest else None,
           'query_s_by_group': {k: stats(v) for k, v in groups.items()},
           'query_max': max(queries, key=lambda x: x[1]) if queries else None,
           'controls': [[a, round(d, 3)] for a, d, _ in controls],
           'event_kinds': {k: sum(1 for e in events if e['kind'] == k) for k in sorted({e['kind'] for e in events})}}
    # 单调钟→墙钟：normal-recovery.json 紧随 after-normal-<末cut> 查询事件写盘，二者差为毫秒级。
    anchor = [e for e in events if e['kind'] == 'bounded_query' and e['command']['id'].startswith('after-normal-')]
    rec = ev / 'normal-recovery.json'
    offset = (rec.stat().st_mtime - int(anchor[-1]['observed_monotonic_ns']) / 1e9) if anchor and rec.exists() else None
    out['browser'] = browser_timings(ev, controls, offset)
    return out


def browser_timings(ev, controls, offset):
    b = ev / 'browser'
    if not b.exists():
        return {'present': False}
    receipt = load(b / 'RESULT.json')
    if receipt.get('status') == 'DEFERRED':
        return {'present': True, 'deferred': True, 'result': receipt, 'browser_launched': (ev / 'browser.stdout').exists(),
                'note': '--browser-deferred-to：浏览器未执行，无 HAR/READINESS 计时'}
    m = lambda name: (b / name).stat().st_mtime if (b / name).exists() else None
    t0 = b.stat().st_birthtime  # fs.mkdirSync(output) 后紧接 began=performance.now()
    diag = load(b / 'READINESS-DIAGNOSTICS.json')
    marks = {e['kind']: e['ms'] for e in diag.get('events', []) if e.get('kind') not in ('request', 'response', 'finished', 'failed')}
    seqs = []
    har = load(b / 'network.har')
    for e in har.get('log', {}).get('entries', []):
        text = (e['request'].get('postData') or {}).get('text', '')
        if '/api/v2/snapshot' not in e['request']['url'] or not text:
            continue
        start = har_time(e['startedDateTime']); end = start + e['time'] / 1000
        payload = json.loads(text).get('payload', {})
        if 'history_mode' in payload:
            seqs.append({'mode': payload['history_mode'], 'as_of': payload.get('as_of_generation'), 'start': start, 'end': end, 'requests': 1,
                         'status': [e['response']['status']]})
        elif seqs:
            s = seqs[-1]; s['end'] = max(s['end'], end); s['requests'] += 1; s['status'].append(e['response']['status'])
    t_txt1, t_png1, t_txt2, t_png2 = m('CC-022.txt'), m('CC-022.png'), m('CC-022-asof.txt'), m('CC-022-asof.png')
    t_res, t_diag = m('RESULT.json'), m('READINESS-DIAGNOSTICS.json')
    final = [s for s in seqs if s['mode'] != 'AsKnown' and t_txt1 and s['end'] <= t_txt1]
    asof = [s for s in seqs if s['mode'] == 'AsKnown']
    r = lambda a, z: round(z - a, 3) if a is not None and z is not None else None
    phases = {
        'launch_to_first_request': r(t0, min((s['start'] for s in seqs), default=None)),
        'wait_final_cut_network(t0→最终cut末页响应)': r(t0, final[-1]['end'] if final else None),
        'click1_CC022_final(末页响应→CC-022.txt落盘；含页面提交尾、点击、等视图、innerText)': r(final[-1]['end'] if final else None, t_txt1),
        'screenshot1': r(t_txt1, t_png1),
        'asof_fill_and_submit(截图1→AsKnown首请求)': r(t_png1, asof[0]['start'] if asof else None),
        'wait_asof_network(AsKnown首请求→末页响应)': r(asof[0]['start'] if asof else None, asof[-1]['end'] if asof else None),
        'click2_CC022_asof27(AsKnown末页响应→CC-022-asof.txt落盘；含页面提交尾、点击、innerText)': r(asof[-1]['end'] if asof else None, t_txt2),
        'screenshot2': r(t_txt2, t_png2),
        'close_and_har(截图2→READINESS落盘)': r(t_png2, t_diag),
    }
    inner = diag.get('elapsed_ms', 0) / 1000 if isinstance(diag.get('elapsed_ms'), (int, float)) else None
    outer_start = (ev / 'browser.stdout').stat().st_birthtime if (ev / 'browser.stdout').exists() else None
    stop_q = [c for c in controls if c[0] == 'stop-q']
    outer_end_ub = (stop_q[0][2] / 1e9 - stop_q[0][1] + offset) if stop_q and offset is not None else None
    outer = r(outer_start, outer_end_ub)
    return {'present': True, 'result': load(b / 'RESULT.json'), 't0': iso(t0),
            'inner_window_s(READINESS elapsed)': inner, 'inner_margin_s': round(CAP_S - inner, 3) if inner is not None else None,
            'inner_check(t0+elapsed vs READINESS mtime, s)': r(t0 + inner, t_diag) if inner is not None and t_diag else None,
            'outer_window_upper_bound_s(browser.stdout 创建→cleanup 首个 stop-q 开始)': outer,
            'outer_margin_lower_bound_s': round(CAP_S - outer, 3) if outer is not None else None,
            'phases_s': phases, 'snapshot_sequences': [{**{k: v for k, v in s.items() if k not in ('start', 'end')},
                                                        'start': iso(s['start']), 'span_s': round(s['end'] - s['start'], 3)} for s in seqs],
            'diag_counts': diag.get('counts'), 'diag_dropped': diag.get('dropped'), 'diag_marks_ms': marks,
            'note': 'READINESS 事件缓冲 128 条，点击标记若被挤掉则仅剩 HAR+mtime 推导；推导值含页面 JS 提交尾，是点击段的上界'}


def main():
    raw = DELIVER / 'raw'
    raw.mkdir(exist_ok=False)
    for name in ('RUN-PLAN.json', 'COMPARISON-RESULT.json', 'SLOT-SNAPSHOT.json', 'SLOT-REAP.json'):
        if (RUN / name).exists():
            shutil.copy2(RUN / name, raw / name)
    big, timings, arms = {}, {}, {}
    for arm in ARMS:
        d = RUN / arm
        if not (d / 'evidence').exists():
            arms[arm] = {'ran': False}
            continue
        dst = raw / arm
        for rel in SMALL:
            if (d / rel).exists():
                (dst / rel).parent.mkdir(parents=True, exist_ok=True)
                shutil.copy2(d / rel, dst / rel)
        for sub in ('evidence/browser', 'evidence/control'):
            if (d / sub).exists():
                shutil.copytree(d / sub, dst / sub)
        for p in sorted(d.glob('interrupted-stop-*')):
            shutil.copy2(p, dst / p.name)
        entry = {rel: {'bytes': (d / rel).stat().st_size, 'sha256': sha(d / rel)} for rel in BIG if (d / rel).exists()}
        http = d / 'evidence/http'
        if http.exists():
            lines = sorted(f'{sha(p)}  {p.relative_to(http)}' for p in http.rglob('*') if p.is_file())
            manifest = ('\n'.join(lines) + '\n').encode()
            (dst / 'evidence').mkdir(parents=True, exist_ok=True)
            (dst / 'evidence/http-MANIFEST.sha256').write_bytes(manifest)
            entry['evidence/http/'] = {'files': len(lines), 'bytes': sum(p.stat().st_size for p in http.rglob('*') if p.is_file()),
                                       'manifest_sha256': hashlib.sha256(manifest).hexdigest()}
        big[arm] = entry
        try:
            timings[arm] = arm_timings(arm)
        except Exception as exc:
            timings[arm] = {'error': repr(exc)[:300]}
        ev = d / 'evidence'
        cap, orc, rec, br = load(ev / 'RESULT.json'), load(ev / 'query-oracle.json'), load(ev / 'normal-recovery.json'), load(ev / 'browser/RESULT.json')
        arms[arm] = {'ran': True, 'exit': load(d / 'EXIT.json'), 'interrupted': load(d / 'INTERRUPTED.json') if (d / 'INTERRUPTED.json').exists() else None,
                     'capture_status': cap.get('status'), 'capture_error': cap.get('error'), 'input_count': cap.get('input_count'),
                     'core_records': cap.get('core_records'), 'cleanup_errors': (cap.get('cleanup') or {}).get('errors'),
                     'oracle': {'status': orc.get('status'), 'cuts': orc.get('cuts'), 'oracle_cuts': orc.get('oracle_cuts')},
                     'recovery': {'equal': rec.get('equal'), 'cuts': rec.get('cuts')},
                     'browser': {'status': br.get('status'), 'cuts': br.get('cuts'), 'page_errors': br.get('page_errors'), 'version': br.get('browser_version'),
                                 'deferred_to': br.get('deferred_to'), 'browser_executed': br.get('browser_executed'), 'planned_cuts': br.get('planned_cuts')}}
    (DELIVER / 'BIG-ARTIFACTS.json').write_text(json.dumps({'run_dir': str(RUN), 'arms': big}, ensure_ascii=False, indent=1) + '\n')
    (DELIVER / 'ARM-TIMINGS.json').write_text(json.dumps(timings, ensure_ascii=False, indent=1) + '\n')
    (DELIVER / 'ARMS.json').write_text(json.dumps(arms, ensure_ascii=False, indent=1) + '\n')
    print(json.dumps({a: (v.get('capture_status'), v.get('exit')) for a, v in arms.items() if v.get('ran')}, ensure_ascii=False))


if __name__ == '__main__':
    main()
