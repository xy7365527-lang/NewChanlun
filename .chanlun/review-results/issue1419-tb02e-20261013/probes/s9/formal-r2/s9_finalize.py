#!/usr/bin/env python3
"""#1419 S9 第二轮：由 PREFLIGHT/DRIVE/SLOT-REAP/背景采样/ARM-TIMINGS 与外盘原件汇总 RESULT.json（含冷 verify 点读数与首轮对照）。"""
import json
from datetime import datetime, timedelta
from pathlib import Path

D = Path('/private/tmp/goal1323-native-20261012/tb1419-impl/s9-formal-r2')
R1 = Path('/private/tmp/goal1323-native-20261012/tb1419-impl/s9-formal')
RUN = Path('/Volumes/AgentStorage/issue1419-s9-formal-r2-20261014.noindex')
J = lambda p: json.loads(Path(p).read_text())
pre, drive, reap, timings = J(D / 'PREFLIGHT.json'), J(D / 'DRIVE.json'), J(D / 'SLOT-REAP.json'), J(D / 'ARM-TIMINGS.json')
r1 = J(R1 / 'RESULT.json')
samples = [json.loads(l) for l in (D / 'background-samples.jsonl').read_text().splitlines() if l.strip()]
arms = J(D / 'ARMS.json')
for name in ('min12_up-b', 'min12_down-a', 'min12_down-b'):
    arms[name] = {'ran': False, 'reason': 'harness execute() 在首个失败臂 min12_up-a 抛出后终止，本臂只做了 prepare（config/clock/messages），未启动'}
(D / 'ARMS.json').write_text(json.dumps(arms, ensure_ascii=False, indent=1) + '\n')

ua = arms['min12_up-a']; tu = timings['min12_up-a']; bt = tu['browser']
# 冷 verify 点：HAR 中 cut 43 的第 2 个 snapshot 请求（503）
har = J(RUN / 'min12_up-a/evidence/browser/network.har')
snaps = [e for e in har['log']['entries'] if e['request']['url'].endswith('/api/v2/snapshot')]
bad = [e for e in snaps if e['response']['status'] == 503]
assert len(bad) == 1
e503 = bad[0]
t_start = datetime.fromisoformat(e503['startedDateTime'].replace('Z', '+00:00')).astimezone()
t_end = t_start + timedelta(milliseconds=e503['time'])
body = json.loads(e503['response']['content']['text'])['payload']
window = []
for s in samples:
    at = datetime.strptime(s['at'], '%Y-%m-%dT%H:%M:%S%z')
    if t_start - timedelta(seconds=2) <= at <= t_end + timedelta(seconds=2):
        q = [{'pid': t['pid'], 'pcpu': t['pcpu'], 'rss_mb': t['rss_kb'] // 1024} for t in s['top'] if t['command'].endswith('s_session/s_r')]
        window.append({'at': s['at'], 'loadavg': [round(x, 2) for x in s['loadavg']], 'chrome_total_pcpu': s['chrome_total_pcpu'],
                       'spotlight_pcpu': s['spotlight_pcpu'], 'q_s_readonly_server_top12': q})
cold = {
    'request': 'cut 43 页面第 2 个 /api/v2/snapshot（snapshot_token 分页，message_id query-2）',
    'started_at': t_start.isoformat(timespec='milliseconds'), 'ended_at': t_end.isoformat(timespec='milliseconds'),
    'request_total_s': round(e503['time'] / 1000, 3), 'har_wait_s': round(e503['timings']['wait'] / 1000, 3),
    'http_status': 503, 'error': body['error'], 'detail': body['detail'],
    'verify_s': ('≥5.000（耗尽 verify_deadline_ms=5000 未完成，真实完成时间不可知）；'
                 '请求总 5.156 s − 5.000 s 期限 ≈ 0.156 s 为 capture 与往返'),
    'deadline_source': 's_session/tests/fixtures/tb01c/query-resources.json:8 verify_deadline_ms=5000；Q 工作者 max_query_workers=2；抛出 s_query_integrity.py:250（_deadline）',
    'load_during': window,
    'reading': ('请求窗内 1 分钟负载 8.36–8.49（28 逻辑核），Spotlight ≈0，Q 进程 s_readonly_server pid 5480 单核 95–99%、RSS 675→817 MB；'
                '即 Q 在单线程满负荷做 verify 期间耗尽 5 s，未见资源争用迹象'),
    'first_snapshot_s': round(snaps[0]['time'] / 1000, 3),
}
r1c = r1['first_failure']
load1 = [s['loadavg'][0] for s in samples]
result = {
    'schema': 'tb1419-s9-formal-result/1', 'round': 2, 'ruling': 'ROOT-RULING-S9-RERUN.md（2026-10-14 11:55）',
    'ticket': '#1419 (TB-02-E)', 'branch': 'codex/1419-center-seed-20261013', 'head': pre['git_head'], 'worktree_clean': pre['git_clean'],
    'status': 'FAIL',
    'same_failure_point_as_round1': True,
    'ruling_clause_3_applies': '第二轮仍在同一冷 verify 点失败（裁定第 3 条的触发条件成立，判定由根作出）',
    'first_failure': {
        'arm': 'min12_up-a', 'stage': 'browser（tb02e_browser.cjs:63 等最终 cut 43 提交，waitForFunction 10 s）',
        'error': 'page.waitForFunction: Timeout 10000ms exceeded（browser.stderr）→ runtime「真实浏览器验证失败」→ harness「正式验证失败」',
        'cause': '页面取 cut 43 的第 2 个 /api/v2/snapshot（token 分页）请求 5.156 s 后得 HTTP 503 QueryBudgetExceeded「查询阶段达到事前时间上限」；页面未提交 cut 43',
        'clicks_performed': 0, 'retried': False,
    },
    'cold_verify_point': cold,
    'round1_vs_round2': {
        'request_total_s': [5.174, cold['request_total_s']],
        'first_snapshot_s': [0.867, cold['first_snapshot_s']],
        'load1_during_run_min_max': [r1['background_load']['during_run_load1_min_max'], [round(min(load1), 2), round(max(load1), 2)]],
        'spotlight_mds_stores_pcpu_max': [287.6, max(s['spotlight_pcpu']['mds_stores'] for s in samples)],
        'browser_inner_window_s': [r1['browser_arm']['min12_up-a']['inner_window_s'], bt['inner_window_s(READINESS elapsed)']],
        'chrome_launch_to_navigation_s': [5.134, bt['diag_marks_ms'].get('navigation-start', 0) / 1000],
        'ingest_max_s': [4.777, tu['ingest_reply_s']['max']], 'query_max_s': [8.123, round(tu['query_max'][1], 3)],
        'harness_elapsed_s': [429.7, drive['harness_elapsed_s']],
        'reading': '1 分钟负载由 9.6–18.1 降到 7.3–9.1、mds_stores 峰值由 288% 降到 15%（浏览器窗内 ≤1.5%），503 请求耗时只差 0.018 s；失败点不随这两项背景负载变化',
    },
    'binary': {'path': pre['binary'], 'sha256': pre['binary_sha256'], 'same_as_round1': pre['binary_sha256'] == r1['binary']['sha256']},
    'arms': {
        'min12_up-a': {
            'verdict': 'FAIL（浏览器）', 'inputs': ua['input_count'], 'capture_core_records': ua['core_records'],
            'public_queries': tu['event_kinds'].get('bounded_query'),
            'live_known_revisit': '43×live、43×AsKnown、43×回访全部 captured_validated_commit，回访与原 AsKnown 逐值相同',
            'frozen_oracle': ua['oracle'], 'normal_recovery': ua['recovery'],
            'browser': ua['browser'] | {'status': 'FAIL'}, 'exit': ua['exit'], 'cleanup_errors': ua['cleanup_errors'],
        },
        'min12_up-b': {'verdict': 'NOT_RUN'}, 'min12_down-a': {'verdict': 'NOT_RUN'}, 'min12_down-b': {'verdict': 'NOT_RUN'},
    },
    'ab_comparison': {'status': 'NOT_RUN', 'reason': '首臂失败，harness 未进入 compare_completed，无 COMPARISON-RESULT.json'},
    'recovery': {'min12_up-a': {'cuts': ua['recovery']['cuts'], 'equal': ua['recovery']['equal'],
                                'controls_s': {c[0]: c[1] for c in tu['controls'] if c[0] in ('recover', 'start-s')} | {'stop-s(恢复前)': tu['controls'][1][1]},
                                'before_queries_s': tu['query_s_by_group'].get('before'), 'after_queries_s': tu['query_s_by_group'].get('after')},
                 'other_arms': 'NOT_RUN'},
    'browser_arm': {'min12_up-a': {
        'cap_s': 30, 'inner_window_s': bt['inner_window_s(READINESS elapsed)'], 'inner_margin_s': bt['inner_margin_s'],
        'outer_window_upper_bound_s': bt['outer_window_upper_bound_s(browser.stdout 创建→cleanup 首个 stop-q 开始)'],
        'outer_margin_lower_bound_s': bt['outer_margin_lower_bound_s'], 'cap_hit': False,
        'click1_final_cut_CC022_s': None, 'click1_note': '未到达：失败发生在第一次点击之前（等最终 cut 43 提交）',
        'click2_asof27_CC022_s': None, 'click2_note': '未到达',
        'phases_s': {'Chrome 启动到 navigation-start': bt['diag_marks_ms'].get('navigation-start', 0) / 1000,
                     'goto 完成（navigation-loaded）': bt['diag_marks_ms'].get('navigation-loaded', 0) / 1000,
                     'snapshot#1（新查询，200）': cold['first_snapshot_s'], 'snapshot#2（token 分页，503）': cold['request_total_s'],
                     '其后': 'navigation-loaded 后 10 s 超时；elapsed ' + str(bt['inner_window_s(READINESS elapsed)']) + ' s'},
        'chrome': {'path': pre['chrome_path'], 'version': pre['chrome_version'], 'sha256': pre['chrome_sha256']},
    }},
    'timings_min12_up-a': {'ingest_reply_s': tu['ingest_reply_s'], 'ingest_last5': tu['ingest_last5'], 'ingest_cap_s': 10,
                           'query_s_by_group': tu['query_s_by_group'], 'query_max': tu['query_max'], 'query_cap_s': 30},
    'environment': {'python': pre['python'] + ' (/tmp/tb1414-runtime-venv)', 'node': pre['node'], 'playwright': pre['playwright'],
                    'harness_env_added': drive['harness_env_added']},
    'background_load': {
        'preflight_loadavg': pre['background']['loadavg'], 'preflight_spotlight_pcpu': pre['spotlight_pcpu'], 'ncpu': pre['background']['ncpu'],
        'during_run_load1_min_max': [round(min(load1), 2), round(max(load1), 2)],
        'chrome_total_pcpu_min_max': [min(s['chrome_total_pcpu'] for s in samples), max(s['chrome_total_pcpu'] for s in samples)],
        'spotlight_mds_stores_pcpu_max': max(s['spotlight_pcpu']['mds_stores'] for s in samples),
        'note': ('门检时 mds_stores 0%，无 cargo/rustc/lake/S/Q/探针/浏览器验收进程，peer cargo 锁全 IDLE。背景：用户自用 Chrome 154（GPU 进程 136–166%）、'
                 'agent-browser 无头 Chrome（约 100%）、WindowServer（约 60–76%）、ChatGPT/Codex 渲染进程；大件写 .noindex 目录，Spotlight 除 12:02:10 一次 mds_stores 15.4%/mdworker 16.8%、12:03:51 一次 mdworker 12.4% 外均 <2%（浏览器窗内 ≤1.5%）'),
        'samples_file': 'background-samples.jsonl（平时每 10 s，浏览器窗内每 2 s）',
    },
    'disk': {'data_free_gib_min': min(s['data_free_gib'] for s in samples), 'stopline_gib': 15, 'stopline_hit': 'disk_stopline' in drive,
             'agentstorage_free_gib_min': min(s['agentstorage_free_gib'] for s in samples)},
    'slot_reap': {'path_deliver': str(D / 'SLOT-REAP.json'), 'path_run': str(RUN / 'SLOT-REAP.json'), 'clean': reap['clean'],
                  'found_processes': len(reap['first']['processes']), 'orphan_chrome': len(reap['first']['orphan_chrome']),
                  'listeners': len(reap['first']['listeners']), 'sockets_unlinked': [a['socket'] for a in reap['actions'] if 'socket' in a],
                  'postcheck': 'ps/lsof 复查：无 S/Q/http 捕获/playwright Chrome 残留，18860–18863 无监听，/private/tmp/tb1419e-* 无残留'},
    'run_dir': str(RUN), 'deliver_dir': str(D),
    'harness': {'rc': drive['harness_rc'], 'started_at': drive['harness_started_at'], 'ended_at': drive['harness_ended_at'], 'elapsed_s': drive['harness_elapsed_s']},
    'driver_changes_vs_round1': ['DELIVER/RUN 路径换为 r2 与 .noindex', '门检加 mds_stores < 20% 条件', '背景采样加 Spotlight 字段，平时 10 s、浏览器窗内 2 s', 'harness/runtime/页面脚本/帽/二进制不变'],
}
(D / 'RESULT.json').write_text(json.dumps(result, ensure_ascii=False, indent=1) + '\n')
print(json.dumps(result['cold_verify_point'], ensure_ascii=False, indent=1)[:2500])
print(result['round1_vs_round2'])
print(result['background_load']['during_run_load1_min_max'], result['background_load']['chrome_total_pcpu_min_max'], result['disk'])
