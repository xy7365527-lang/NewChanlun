#!/usr/bin/env python3
"""#1419 S9：由 PREFLIGHT/DRIVE/SLOT-REAP/背景采样/ARM-TIMINGS 与外盘原件汇总 RESULT.json，并修正 ARMS.json 的「已运行」口径。"""
import json
from pathlib import Path

D = Path('/private/tmp/goal1323-native-20261012/tb1419-impl/s9-formal')
RUN = Path('/Volumes/AgentStorage/issue1419-s9-formal-20261014')
J = lambda p: json.loads(Path(p).read_text())
pre, drive, reap, timings, big = J(D / 'PREFLIGHT.json'), J(D / 'DRIVE.json'), J(D / 'SLOT-REAP.json'), J(D / 'ARM-TIMINGS.json'), J(D / 'BIG-ARTIFACTS.json')
samples = [json.loads(l) for l in (D / 'background-samples.jsonl').read_text().splitlines() if l.strip()]
arms = J(D / 'ARMS.json')
for name, a in arms.items():
    a['ran'] = (RUN / name / 'evidence').exists()
    if not a['ran']:
        arms[name] = {'ran': False, 'reason': 'harness execute() 在首个失败臂 min12_up-a 抛出后终止，本臂只做了 prepare（config/clock/messages），未启动'}
(D / 'ARMS.json').write_text(json.dumps(arms, ensure_ascii=False, indent=1) + '\n')

ua = arms['min12_up-a']; tu = timings['min12_up-a']; bt = tu['browser']
mx = lambda key: max(s[key] for s in samples)
mn = lambda key: min(s[key] for s in samples)
load1 = [s['loadavg'][0] for s in samples]
result = {
    'schema': 'tb1419-s9-formal-result/1',
    'ticket': '#1419 (TB-02-E)', 'branch': 'codex/1419-center-seed-20261013', 'head': pre['git_head'], 'worktree_clean': pre['git_clean'],
    'status': 'FAIL',
    'first_failure': {
        'arm': 'min12_up-a', 'stage': 'browser（tb02e_browser.cjs:63 等最终 cut 43 提交，waitForFunction 10 s）',
        'error': 'page.waitForFunction: Timeout 10000ms exceeded（browser.stderr）→ runtime「真实浏览器验证失败」→ harness「正式验证失败」',
        'cause': ('页面取最终 cut 的第 2 个 /api/v2/snapshot（snapshot_token 分页）请求在 5.174 s 后得到 HTTP 503 '
                  'QueryBudgetExceeded「查询阶段达到事前时间上限」（Q verify 期限 verify_deadline_ms=5000，'
                  'tests/fixtures/tb01c/query-resources.json:8；抛出点 s_query_integrity.py:250）；页面未提交 cut 43，10 s 等待超时'),
        'clicks_performed': 0,
        'retried': False,
    },
    'binary': {'path': pre['binary'], 'sha256': pre['binary_sha256'], 'built_from': '48e2386463（48e2386463..daac6d8661 无 rust/ 与 formal/ 改动）'},
    'arms': {
        'min12_up-a': {
            'verdict': 'FAIL（浏览器）',
            'inputs': ua['input_count'], 'capture_core_records': ua['core_records'],
            'public_queries': tu['event_kinds'].get('bounded_query'),
            'live_known_revisit': '43×live、43×AsKnown、43×回访全部 captured_validated_commit，回访与原 AsKnown 逐值相同（execute 未抛错并进入 collect_core）',
            'frozen_oracle': ua['oracle'],
            'normal_recovery': ua['recovery'],
            'browser': ua['browser'] | {'status': 'FAIL'},
            'exit': ua['exit'], 'cleanup_errors': ua['cleanup_errors'],
        },
        'min12_up-b': {'verdict': 'NOT_RUN'}, 'min12_down-a': {'verdict': 'NOT_RUN'}, 'min12_down-b': {'verdict': 'NOT_RUN'},
    },
    'ab_comparison': {'status': 'NOT_RUN', 'reason': '首臂失败，harness 未进入 compare_completed，无 COMPARISON-RESULT.json'},
    'recovery': {'min12_up-a': {'cuts': ua['recovery']['cuts'], 'equal': ua['recovery']['equal'],
                                'controls_s': {c[0]: c[1] for c in tu['controls'] if c[0] in ('recover', 'start-s')} | {'stop-s(恢复前)': tu['controls'][1][1]},
                                'before_queries_s': tu['query_s_by_group'].get('before'), 'after_queries_s': tu['query_s_by_group'].get('after')},
                 'other_arms': 'NOT_RUN'},
    'browser_arm': {
        'min12_up-a': {
            'cap_s': 30,
            'inner_window_s': bt['inner_window_s(READINESS elapsed)'], 'inner_margin_s': bt['inner_margin_s'],
            'outer_window_upper_bound_s': bt['outer_window_upper_bound_s(browser.stdout 创建→cleanup 首个 stop-q 开始)'],
            'outer_margin_lower_bound_s': bt['outer_margin_lower_bound_s'],
            'cap_hit': False,
            'click1_final_cut_CC022_s': None, 'click1_note': '未到达：失败发生在第一次点击之前（等最终 cut 43 提交）',
            'click2_asof27_CC022_s': None, 'click2_note': '未到达',
            'phases_s': {'Chrome 启动到 navigation-start': bt['diag_marks_ms'].get('navigation-start', 0) / 1000,
                         'goto 完成（navigation-loaded）': bt['diag_marks_ms'].get('navigation-loaded', 0) / 1000,
                         'snapshot#1（新查询，200）': 0.867, 'snapshot#2（token 分页，503 QueryBudgetExceeded）': 5.174,
                         '其后等待提交至 10 s 超时、关闭': 'navigation-loaded 后 10 s 超时；elapsed 15.427 s'},
            'chrome': {'path': pre['chrome_path'], 'version': pre['chrome_version'], 'sha256': pre['chrome_sha256'],
                       'note': 'browser/RESULT.json 未写出（失败），版本取自门检时 Info.plist；TB-02-D 冻结时 154.0.8037.99，F1a 时 155.0.8059.40'},
        }
    },
    'timings_min12_up-a': {'ingest_reply_s': tu['ingest_reply_s'], 'ingest_last5': tu['ingest_last5'], 'ingest_cap_s': 10,
                           'query_s_by_group': tu['query_s_by_group'], 'query_max': tu['query_max'], 'query_cap_s': 30},
    'environment': {'python': pre['python'] + ' (/tmp/tb1414-runtime-venv)', 'node': pre['node'], 'playwright': pre['playwright'],
                    'harness_env_added': drive['harness_env_added']},
    'background_load': {
        'preflight_loadavg': pre['background']['loadavg'], 'ncpu': pre['background']['ncpu'],
        'during_run_load1_min_max': [round(min(load1), 2), round(max(load1), 2)],
        'chrome_total_pcpu_min_max': [mn('chrome_total_pcpu'), mx('chrome_total_pcpu')],
        'note': ('门检时无 cargo/rustc/lake/S/Q/探针/浏览器验收进程，peer cargo 锁全 IDLE。背景：用户自用 Chrome 154（GPU 进程 110–160%、若干 renderer）、'
                 'agent-browser 无头 Chrome（--remote-debugging-port=0，约 100%）、CLIProxy（至 166%）、WindowServer（约 60%）；'
                 '运行期间 mds_stores（Spotlight）100–288%，推断与外盘写入约 7 GiB/2.6 万个 HTTP 收据文件有关（未用 fs_usage 证实）'),
        'samples_file': 'background-samples.jsonl（每 30 s）',
    },
    'disk': {'data_free_gib_min': mn('data_free_gib'), 'stopline_gib': 15, 'stopline_hit': 'disk_stopline' in drive,
             'agentstorage_free_gib_min': mn('agentstorage_free_gib')},
    'slot_reap': {'path_deliver': str(D / 'SLOT-REAP.json'), 'path_run': str(RUN / 'SLOT-REAP.json'), 'clean': reap['clean'],
                  'found_processes': len(reap['first']['processes']), 'orphan_chrome': len(reap['first']['orphan_chrome']),
                  'listeners': len(reap['first']['listeners']), 'sockets_unlinked': [a['socket'] for a in reap['actions'] if 'socket' in a],
                  'postcheck': 'ps/lsof 复查：无 S/Q/http 捕获/playwright Chrome 残留，18860–18863 无监听，/private/tmp/tb1419e-* 无残留'},
    'run_dir': str(RUN), 'deliver_dir': str(D),
    'harness': {'rc': drive['harness_rc'], 'started_at': drive['harness_started_at'], 'ended_at': drive['harness_ended_at'], 'elapsed_s': drive['harness_elapsed_s']},
    'deviation_from_brief': ['重输出（每臂 HTTP 收据约 6.9 GB、轨迹 1.86 GB、库）写外盘 RUN；本目录只放小件副本与大件 SHA（harness 文首要求外盘目录；数据卷 24 GiB、停手线 15 GiB）'],
}
(D / 'RESULT.json').write_text(json.dumps(result, ensure_ascii=False, indent=1) + '\n')
print('ok')
