#!/usr/bin/env python3
"""#1419 S9 第三轮：由 PREFLIGHT/DRIVE/SLOT-REAP/背景采样/ARMS/ARM-TIMINGS/COMPARISON-RESULT 汇总 RESULT.json。"""
import json
from datetime import datetime
from pathlib import Path

D = Path('/private/tmp/goal1323-native-20261012/tb1419-impl/s9-formal-r3')
IMPL = D.parent
RUN = Path('/Volumes/AgentStorage/issue1419-s9-formal-r3-20261014.noindex')
J = lambda p: json.loads(Path(p).read_text())
pre, drive, reap, timings, arms, big = (J(D / n) for n in ('PREFLIGHT.json', 'DRIVE.json', 'SLOT-REAP.json', 'ARM-TIMINGS.json', 'ARMS.json', 'BIG-ARTIFACTS.json'))
comp = J(RUN / 'COMPARISON-RESULT.json')
plan = J(RUN / 'RUN-PLAN.json')
samples = [json.loads(l) for l in (D / 'background-samples.jsonl').read_text().splitlines() if l.strip()]
ORDER = ('min12_up-a', 'min12_up-b', 'min12_down-a', 'min12_down-b')

t0 = datetime.strptime(drive['harness_started_at'], '%Y-%m-%dT%H:%M:%S%z').timestamp()
ends = {a: (RUN / a / 'EXIT.json').stat().st_mtime for a in ORDER}
t_comp = (RUN / 'COMPARISON-RESULT.json').stat().st_mtime
spans, prev = {}, t0
for a in ORDER:
    spans[a] = round(ends[a] - prev, 1); prev = ends[a]
spans['compare_completed'] = round(t_comp - prev, 1)

def rust_in(s):
    return any('rustc' in t['command'] or 'cargo-targets' in t['command'] for t in s['top'])

def window(lo, hi):
    W = [s for s in samples if lo <= datetime.strptime(s['at'], '%Y-%m-%dT%H:%M:%S%z').timestamp() < hi]
    return {'samples': len(W), 'load1_min_max': [round(min(s['loadavg'][0] for s in W), 2), round(max(s['loadavg'][0] for s in W), 2)] if W else None,
            'mds_stores_pcpu_max': max((s['spotlight_pcpu']['mds_stores'] for s in W), default=None),
            'mdworker_pcpu_max': max((s['spotlight_pcpu']['mdworker_shared'] for s in W), default=None),
            'samples_with_foreign_rustc_or_test_binary': sum(1 for s in W if rust_in(s))}

bounds = [t0] + [ends[a] for a in ORDER] + [t_comp]
arm_out, rec_out = {}, {}
for i, a in enumerate(ORDER):
    x, t = arms[a], timings[a]
    ctl = dict((c[0], c[1]) for c in t['controls'][1:4])
    arm_out[a] = {
        'verdict': 'PASS（浏览器 DEFERRED→#1497）' if x['exit'] == {'exit': 0} and x['browser']['status'] == 'DEFERRED' else 'CHECK',
        'exit': x['exit'], 'capture_status': x['capture_status'], 'input_count': x['input_count'], 'core_records': x['core_records'],
        'cleanup_errors': x['cleanup_errors'], 'public_queries': t['event_kinds'].get('bounded_query'),
        'live_known_revisit': '43×live、43×AsKnown、43×回访全部 captured_validated_commit，回访与原 AsKnown 逐值相同',
        'frozen_oracle': x['oracle'], 'normal_recovery': x['recovery'], 'browser': x['browser'] | {'launched': t['browser'].get('browser_launched')},
        'wall_s': spans[a], 'load_during': window(bounds[i], bounds[i + 1]),
        'semantic_core_sha256': big['arms'][a]['evidence/semantic-core.jsonl']['sha256'],
        'database_backup_sha256': big['arms'][a]['evidence/database-backup.sqlite']['sha256'],
    }
    rec_out[a] = {'cuts': x['recovery']['cuts'], 'equal': x['recovery']['equal'],
                  'stop_s_s': ctl.get('stop-s'), 'recover_s': ctl.get('recover'), 'start_s_s': ctl.get('start-s'),
                  'before_query_max_s': t['query_s_by_group']['before']['max'], 'after_query_max_s': t['query_s_by_group']['after']['max']}

r1 = J(IMPL / 's9-formal/BIG-ARTIFACTS.json')['arms']['min12_up-a']
r2 = J(IMPL / 's9-formal-r2/BIG-ARTIFACTS.json')['arms']['min12_up-a']
cross = {'min12_up semantic-core.jsonl sha256': {
            'r1 min12_up-a': r1['evidence/semantic-core.jsonl']['sha256'], 'r2 min12_up-a': r2['evidence/semantic-core.jsonl']['sha256'],
            'r3 min12_up-a': arm_out['min12_up-a']['semantic_core_sha256'], 'r3 min12_up-b': arm_out['min12_up-b']['semantic_core_sha256']},
         'min12_up database-backup.sqlite sha256': {
            'r1 min12_up-a': r1['evidence/database-backup.sqlite']['sha256'], 'r2 min12_up-a': r2['evidence/database-backup.sqlite']['sha256'],
            'r3 min12_up-a': arm_out['min12_up-a']['database_backup_sha256'], 'r3 min12_up-b': arm_out['min12_up-b']['database_backup_sha256']}}
cross['all_equal'] = all(len(set(v.values())) == 1 for v in cross.values() if isinstance(v, dict))

L = [s['loadavg'][0] for s in samples]
rust = [s for s in samples if rust_in(s)]
result = {
    'schema': 'tb1419-s9-formal-result/1', 'round': 3, 'authority': '根批 07479fd501（--browser-deferred-to 1497；根裁 ROOT-RULING-S9-RERUN 第 3 条，浏览器臂移交 #1497）',
    'ticket': '#1419 (TB-02-E)', 'branch': 'codex/1419-center-seed-20261013', 'head': pre['git_head'], 'origin_head_at_preflight': pre['origin_tracking_head'],
    'worktree_clean': pre['git_clean'],
    'status': comp['status'],
    'status_note': 'PASS_EXCEPT_BROWSER_DEFERRED：四臂其余全部门通过、a/b 对拍通过，浏览器臂未执行（DEFERRED→#1497）；不是 PASS',
    'comparison': comp,
    'arms': arm_out,
    'ab_comparison': {c['case']: {k: c[k] for k in ('status', 'expected_records', 'compared_records', 'compared_bytes', 'method', 'sha256')} for c in comp['comparisons']},
    'recovery': rec_out,
    'cross_round_identity': cross,
    'browser': {'status': 'DEFERRED', 'deferred_to': '#1497', 'launched_in_any_arm': any(v['browser']['launched'] for v in arm_out.values()),
                'receipt_planned_cuts': arms['min12_up-a']['browser']['planned_cuts'],
                'note': '各臂 browser/RESULT.json 为 runtime 写出的 DEFERRED 收据（含两轮失败原件路径与 SHA）；30 s 帽与两次点击本轮不适用'},
    'timings': {'harness_elapsed_s': drive['harness_elapsed_s'], 'arm_wall_s': spans,
                'ingest_max_s': {a: timings[a]['ingest_reply_s']['max'] for a in ORDER}, 'ingest_cap_s': 10,
                'query_max_s': {a: [timings[a]['query_max'][0], round(timings[a]['query_max'][1], 3)] for a in ORDER}, 'query_cap_s': 30,
                'query_s_by_group': {a: timings[a]['query_s_by_group'] for a in ORDER}},
    'binary': {'path': pre['binary'], 'sha256': pre['binary_sha256'], 'plan_binary_sha256': plan['binary_sha256']},
    'environment': {'python': pre['python'] + ' (/tmp/tb1414-runtime-venv)', 'node': pre['node'], 'playwright': pre['playwright'],
                    'harness_command_tail': drive['harness_command'][-3:]},
    'background_load': {
        'preflight': {'loadavg': pre['background']['loadavg'], 'spotlight_pcpu': pre['spotlight_pcpu'], 'forbidden_processes': pre['forbidden_processes'],
                      'peer_build': pre['peer_build_busy'], 'hdiutil': '无'},
        'ncpu': pre['background']['ncpu'], 'during_run_load1_min_max': [round(min(L), 2), round(max(L), 2)],
        'chrome_total_pcpu_min_max': [min(s['chrome_total_pcpu'] for s in samples), max(s['chrome_total_pcpu'] for s in samples)],
        'spotlight_max': {'mds_stores': max(s['spotlight_pcpu']['mds_stores'] for s in samples), 'mdworker_shared': max(s['spotlight_pcpu']['mdworker_shared'] for s in samples)},
        'foreign_rust_activity': {'samples': len(rust), 'first': rust[0]['at'] if rust else None, 'last': rust[-1]['at'] if rust else None,
                                  'identified': '别的工位的 cargo/rustc 构建与测试二进制（/Volumes/AgentStorage/cargo-targets.noindex/1424/...），开跑门检时不存在，运行中途出现'},
        'note': ('门检时无 cargo/rustc/lake/S/Q/探针/浏览器验收/hdiutil 进程，cargo 锁全 IDLE，mds_stores 3%。运行中途 12:30 起 Spotlight 回升（mds_stores 最高 508%、mdworker 最高 493%），'
                 '同时段 /private/tmp 下别的工位 worktree（wt-1406、wt-1424）新建大文件；12:42 起有别的工位的 rustc 构建与 cargo-targets.noindex/1424 测试二进制。'
                 '这些不属本图、未动；本轮四臂计时与前两轮同量级'),
        'samples_file': 'background-samples.jsonl（每 10 s）',
    },
    'disk': {'data_free_gib_min': min(s['data_free_gib'] for s in samples), 'stopline_gib': 15, 'stopline_hit': 'disk_stopline' in drive,
             'agentstorage_free_gib_min': min(s['agentstorage_free_gib'] for s in samples),
             'note': '数据卷 12:31 由 24.3 降到 21.6 GiB（别的工位 worktree），最低 20.2 GiB，未触停手线；本轮未向数据卷写大件'},
    'slot_reap': {'path_deliver': str(D / 'SLOT-REAP.json'), 'path_run': str(RUN / 'SLOT-REAP.json'), 'clean': reap['clean'],
                  'found_processes': len(reap['first']['processes']), 'orphan_chrome': len(reap['first']['orphan_chrome']),
                  'listeners': len(reap['first']['listeners']), 'sockets_unlinked': [a['socket'] for a in reap['actions'] if 'socket' in a],
                  'postcheck': 'ps/lsof 复查：无 S/Q/http 捕获/浏览器残留，18860–18863 无监听，/private/tmp/tb1419e-* 无残留'},
    'run_dir': str(RUN), 'deliver_dir': str(D),
    'harness': {'rc': drive['harness_rc'], 'started_at': drive['harness_started_at'], 'ended_at': drive['harness_ended_at'], 'elapsed_s': drive['harness_elapsed_s']},
    'push': {'commit': '07479fd501a72fe202dbc0b41a8250b6a5047841', 'remote': 'origin codex/1419-center-seed-20261013', 'range': 'daac6d8661..07479fd501（快进）'},
}
(D / 'RESULT.json').write_text(json.dumps(result, ensure_ascii=False, indent=1) + '\n')
print(json.dumps({'status': result['status'], 'spans': spans, 'cross': cross['all_equal'],
                  'win': {a: arm_out[a]['load_during'] for a in ORDER}, 'rec': rec_out}, ensure_ascii=False, indent=1))
