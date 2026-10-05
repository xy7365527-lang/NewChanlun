#!/usr/bin/env python3
"""独立复核冻结 H2；不调用作者计算函数，不运行 R_W。"""
from __future__ import annotations

import argparse
import ast
from collections import Counter
from datetime import datetime, timezone
from fractions import Fraction
import gc
import hashlib
import importlib.util
import itertools
import json
from pathlib import Path
import platform
import resource
import subprocess
import sys
import textwrap
import time

E = Path('/Users/silencehan/Documents/Codex/research-evidence/issue1467')
R = Path('/Users/silencehan/.codex/worktrees/order-event-base-research/NewChanlun')
A = E / 'stage69/joint-author'
OLD = E / 'stage68/joint-author'
H2 = E / 'stage64/dynamic-author/v2/run'
HERE = Path(__file__).resolve().parent
LIMIT = 96 * 1024 * 1024


def read(path: Path):
    with path.open(encoding='utf-8') as stream:
        return json.load(stream)


def digest(path: Path) -> str:
    with path.open('rb') as stream:
        return hashlib.file_digest(stream, 'sha256').hexdigest()


def write(out: Path, name: str, obj: object) -> None:
    with (out / name).open('x', encoding='utf-8') as stream:
        json.dump(obj, stream, ensure_ascii=False, indent=2)
        stream.write('\n')


def fingerprints(paths: list[Path]) -> list[dict]:
    return [{'path': str(p), 'sha256': digest(p), 'bytes': p.stat().st_size}
            for p in sorted(set(paths))]


def intersect(ranges: list[list[int]]) -> list[int]:
    lower, upper = zip(*ranges)
    return [max(lower), min(upper)]


def force_from_observations(raw: dict, pens: list[dict], obs: list[dict]) -> dict:
    """使用观察的实际时间与价格，分数运算后确认本输入恰为整数。"""
    left, exclusive_right = raw['members']
    velocities = []
    for pen in (pens[left], pens[exclusive_right - 1]):
        start, end = obs[pen['start']], obs[pen['end']]
        assert start['price'] == pen['start_price']
        assert end['price'] == pen['end_price']
        assert end['time'] > start['time']
        velocities.append(Fraction(end['price'] - start['price'], end['time'] - start['time']))
    first, last = velocities
    assert all(v.denominator == 1 for v in (first, last, last - first))
    return {'first_pen': left, 'last_pen': exclusive_right - 1,
            'first_v': int(first), 'last_v': int(last), 'L': int(last - first)}


def raw_inputs(raws: list[dict], pens: list[dict], obs: list[dict]) -> list[dict]:
    rebuilt = []
    for position, raw in enumerate(raws):
        assert raw['id'] == f'S{position}'
        if position:
            assert raws[position - 1]['end'] == raw['start']
        lo, hi = raw['members']
        assert pens[lo]['start'] == raw['start']
        assert pens[hi - 1]['end'] == raw['end']
        prices = [o['price'] for o in obs[raw['start']:raw['end'] + 1]]
        assert raw['range'] == [min(prices), max(prices)]
        assert raw['up'] == (prices[-1] > prices[0])
        row = dict(raw)
        row.update(force=force_from_observations(raw, pens, obs),
                   own=[raw['start'] + 1, raw['end']], start_price=prices[0],
                   end_price=prices[-1], RawClosed=True, ComparisonReady68=True,
                   ComparisonReady_original=None, MovementCompleted_original=None)
        rebuilt.append(row)
    return rebuilt


def build_windows(obs: list[dict], raw: list[dict]) -> list[dict]:
    """先枚举长度，逐窗直接取原始范围、端点、核及实际首末笔力度。"""
    result = []
    for size in range(5, len(raw), 4):
        for first in range(1, len(raw) - size + 1):
            stop = first + size
            ids = list(range(first, stop))
            first_raw, last_raw, arm = raw[first], raw[stop - 1], raw[stop - 5]
            cores = []
            for j, core_first in enumerate(range(first + 1, stop - 1, 4)):
                members = raw[core_first:core_first + 3]
                interval = intersect([m['range'] for m in members])
                alternating = all(members[i]['up'] != members[i + 1]['up'] for i in (0, 1))
                cores.append({'id': f'IC68:{first}:{j}', 'members': [m['id'] for m in members],
                              'core': interval, 'Outer0': interval, 'Outer0_original_DD_GG': None,
                              'own': [members[0]['start'] + 1, members[2]['end']],
                              'strict': interval[0] < interval[1], 'direction_alternates': alternating,
                              'happened': members[2]['end'],
                              'raw_known': max(m['known_at'] for m in members)})
            moves = []
            for old, new in zip(cores, cores[1:]):
                moves.append(1 if new['core'][0] > old['core'][1] else
                             -1 if new['core'][1] < old['core'][0] else 0)
            kind = 'P' if not moves else 'U' if set(moves) == {1} else 'D' if set(moves) == {-1} else None
            core_low, core_high = cores[-1]['core']
            upward = last_raw['up']
            entry = (arm['start_price'] < core_low and arm['end_price'] >= core_low) if upward else (arm['start_price'] > core_high and arm['end_price'] <= core_high)
            leave = (last_raw['start_price'] <= core_high and last_raw['end_price'] > core_high) if upward else (last_raw['start_price'] >= core_low and last_raw['end_price'] < core_low)
            past_prices = [p['price'] for p in obs[first_raw['start']:last_raw['start'] + 1]]
            prior = max(past_prices) if upward else min(past_prices)
            checks = {
                'strict_cores': all(c['strict'] for c in cores),
                'member_direction_alternates': all(c['direction_alternates'] for c in cores),
                'coherent_type': kind is not None and (kind == 'P' or (kind == 'U') == upward),
                'same_direction_comparison_arms': arm['up'] == upward,
                'entry_cross': entry, 'exit_cross': leave,
                'whole_extreme': last_raw['end_price'] > prior if upward else last_raw['end_price'] < prior,
                'signed_force_weakening': last_raw['force']['L'] < arm['force']['L'],
            }
            structure = all(checks[n] for n in ('strict_cores', 'member_direction_alternates', 'coherent_type'))
            roles = all(checks[n] for n in ('same_direction_comparison_arms', 'entry_cross', 'exit_cross'))
            owned = [event for i in ids for event in range(raw[i]['start'] + 1, raw[i]['end'] + 1)]
            assert owned == list(range(first_raw['start'] + 1, last_raw['end'] + 1))
            values = [p['price'] for p in obs[first_raw['start']:last_raw['end'] + 1]]
            accepted = all(checks.values())
            result.append({
                'id': f'W68:S{first}-S{stop - 1}', 'raw_start': first, 'raw_stop': stop,
                'k': len(cores), 'grade': 0, 'kind': kind,
                'conceptual_direction': None if kind == 'P' else kind, 'technical_up': upward,
                'raws': [raw[i]['id'] for i in ids], 'cores': cores,
                'start': first_raw['start'], 'end': last_raw['end'], 'own': [owned[0], owned[-1]],
                'whole': [min(values), max(values)], 'b_ref': arm['id'], 'c': last_raw['id'],
                'b_force': arm['force'], 'c_force': last_raw['force'], 'prior_whole_extreme': prior,
                'end_price': last_raw['end_price'], 'candidate_happened': last_raw['end'],
                'candidate_known': max(raw[i]['known_at'] for i in ids),
                'structural_pass': structure, 'role_pass': roles, 'CycleDiv68': accepted,
                'RawEnd68': True, 'JEnd68': accepted, 'OriginalGeneralDiv': None,
                'OriginalMovementCompleted': None, 'original_happened': None,
                'original_known': None, 'original_published': None, 'checks': checks,
                'rejections': [name for name, passed in checks.items() if not passed],
                'owns_each_event_once': True,
            })
    return sorted(result, key=lambda w: (w['raw_start'], w['k']))


def consumer(path: list[dict], remove_nopp: bool = False) -> dict:
    violations = [[a['id'], b['id']] for a, b in zip(path, path[1:])
                  if a['grade'] == b['grade'] and a['kind'] == b['kind'] == 'P']
    problems = []
    if not path:
        problems.append('no_nonempty_candidate_delta')
    if violations and not remove_nopp:
        problems.append('NoPP')
    members, center = [], None
    if len(path) < 3:
        problems.append('fewer_than_three_completed_candidate_children')
    else:
        tri = path[:3]
        interval = intersect([w['whole'] for w in tri])
        center = {'core': interval, 'strict': interval[0] < interval[1],
                  'continuous': all(tri[i]['end'] == tri[i + 1]['start'] for i in (0, 1)),
                  'directions_alternate': all(tri[i]['technical_up'] != tri[i + 1]['technical_up'] for i in (0, 1)),
                  'same_grade': len({w['grade'] for w in tri}) == 1, 'original_qualified': None}
        for name, field in [('positive_core', 'strict'), ('continuous', 'continuous'),
                            ('direction_alternation', 'directions_alternate'), ('same_grade', 'same_grade')]:
            if not center[field]:
                problems.append(name)
        members = [{'position': i, 'id': w['id'], 'whole': w['whole'], 'own': w['own'],
                    'grade': w['grade'], 'candidate_completed': w['JEnd68'],
                    'original_completed': None, 'MemberNext': tri[i + 1]['id'] if i < 2 else None}
                   for i, w in enumerate(tri)]
    return {'delta': [w['id'] for w in path], 'kinds': [w['kind'] for w in path],
            'no_pp': not violations, 'pp_violations': violations, 'has_three': len(path) >= 3,
            'counterfactual_no_pp_removed': remove_nopp, 'candidate_member_table': members,
            'candidate_center': center, 'candidate_interface_pass': not problems,
            'original_interface_pass': False, 'original_completion_status': 'unknown', 'reasons': problems,
            'original_reasons': ['OriginalCompleted(child) is unknown', 'RootArm68 source compatibility unknown',
                                 'Outer0=core interpretation as original outer unknown',
                                 'JEnd68 => whole MovementCompleted not proved']}


def graph_paths(edges: list[dict], root: int = 1) -> list[list[dict]]:
    """迭代式逐层扩展；每条长度的所有路径保留，含零长度。"""
    all_paths, frontier = [[]], [[]]
    while frontier:
        next_frontier = []
        for prefix in frontier:
            vertex = prefix[-1]['raw_stop'] if prefix else root
            for edge in edges:
                if edge['raw_start'] == vertex:
                    assert edge['raw_stop'] > vertex
                    next_frontier.append(prefix + [edge])
        all_paths.extend(next_frontier)
        frontier = next_frontier
    return sorted(all_paths, key=lambda p: (len(p), [w['id'] for w in p]))


def prefix_records(t: int, windows: list[dict], raw: list[dict]) -> tuple[dict, dict, dict]:
    available = [w for w in windows if w['candidate_known'] <= t]
    edges = [w for w in available if w['JEnd68']]
    allpaths = graph_paths(edges)
    rows, detailed, consumers = [], [], []
    for path in allpaths:
        con = consumer(path)
        endpoint = path[-1]['end'] if path else raw[0]['end']
        stop = path[-1]['raw_stop'] if path else 1
        extension = any(edge['raw_start'] == stop for edge in edges)
        row = {'delta': con['delta'], 'kinds': con['kinds'], 'is_maximal': not extension,
               'no_pp': con['no_pp'], 'pp_violations': con['pp_violations'], 'has_three': con['has_three'],
               'candidate_F2_pass': con['candidate_interface_pass'], 'original_F2_pass': False,
               'factor_owns': [w['own'] for w in path], 'active_tail': [endpoint + 1, t] if t > endpoint else [],
               'original_completed': [None] * len(path)}
        rows.append(row)
        detailed.append(dict(row, extendable_now=extension,
                             candidate_windows_inside_tail=[w['id'] for w in edges if w['start'] >= endpoint],
                             complete_original_decomposition=None,
                             source_partition_only='raw root + factor intervals + unresolved tail; coverage is not source-certified completeness'))
        if t >= raw[0]['end']:
            ledger = [1] + list(range(2, raw[0]['end'] + 1))
            ledger += [event for w in path for event in range(w['start'] + 1, w['end'] + 1)]
            ledger += list(range(endpoint + 1, t + 1))
            assert sorted(ledger) == list(range(1, t + 1))
        consumers.append({'delta': row['delta'], 'fixed_F2_necessary_conditions': con,
                          'NoPP_removed_same_graph': consumer(path, True),
                          'candidate_clock': [{'id': w['id'], 'happened': w['candidate_happened'], 'known': w['candidate_known']} for w in path],
                          'ledger_coverage_once': t >= raw[0]['end'],
                          'original_F2_status': 'unproved, false flag means not admitted, not disproof'})
    allrecord = {'t': t, 'all_path_count': len(rows), 'nonempty_path_count': sum(bool(r['delta']) for r in rows),
                 'all_paths': rows, 'NoPP_paths_with_three': sum(r['has_three'] and r['no_pp'] for r in rows),
                 'NoPP_removed_paths_with_three': sum(r['has_three'] for r in rows)}
    classes = {'t': t, 'all_edge_paths': detailed,
               'no_pp_paths_including_empty': [r['delta'] for r in rows if r['no_pp']],
               'no_pp_nonempty_paths': [r['delta'] for r in rows if r['no_pp'] and r['delta']],
               'three_child_paths': [r['delta'] for r in rows if r['has_three']],
               'no_pp_and_three_child_paths': [r['delta'] for r in rows if r['has_three'] and r['no_pp']],
               'counts': {'all': len(rows), 'nonempty': sum(bool(r['delta']) for r in rows),
                          'no_pp_nonempty': sum(r['no_pp'] and bool(r['delta']) for r in rows),
                          'three_children': sum(r['has_three'] for r in rows),
                          'NoPP_and_three': sum(r['has_three'] and r['no_pp'] for r in rows)}}
    counts = {'available_grammar_windows': len(available),
              'structural_pass': sum(w['structural_pass'] for w in available),
              'roles_pass_after_structure': sum(w['structural_pass'] and w['role_pass'] for w in available),
              'whole_extreme_after_structure_roles': sum(w['structural_pass'] and w['role_pass'] and w['checks']['whole_extreme'] for w in available),
              'candidate_completion_edges': len(edges), 'all_paths': len(rows),
              'maximal_paths': sum(r['is_maximal'] for r in rows),
              'maximal_nonempty_paths': sum(r['is_maximal'] and bool(r['delta']) for r in rows),
              'maximal_no_pp_nonempty_paths': sum(r['is_maximal'] and r['no_pp'] and bool(r['delta']) for r in rows),
              'no_pp_nonempty_paths': classes['counts']['no_pp_nonempty'],
              'three_child_paths': classes['counts']['three_children'],
              'candidate_F2_pass': sum(r['candidate_F2_pass'] for r in rows),
              'NoPP_removed_candidate_F2_pass': sum(c['NoPP_removed_same_graph']['candidate_interface_pass'] for c in consumers),
              'max_factor_count': max(map(len, allpaths)), 'original_F2_admitted': 0}
    return allrecord, classes, {'t': t, 'counts': counts, 'consumer_checks': consumers,
                                'raw_known': [s['id'] for s in raw if s['known_at'] <= t]}


def equal(actual: object, expected: object, context: str) -> None:
    if actual != expected:
        raise AssertionError(context)


def audit_rules() -> dict:
    old, new = (OLD / 'check_joint.py').read_text(), (A / 'check_joint.py').read_text()
    old_ast, new_ast = ast.parse(old), ast.parse(new)
    old_f = {n.name: ast.get_source_segment(old, n) for n in old_ast.body if isinstance(n, ast.FunctionDef)}
    new_f = {n.name: ast.get_source_segment(new, n) for n in new_ast.body if isinstance(n, ast.FunctionDef)}
    functions = {name: old_f[name] == new_f[name] for name in ('force', 'meet', 'f2_contract')}
    assert all(functions.values())
    block = old[old.index('windows=[]\n'):old.index('assert len(windows)==40')]
    modified = block.replace('range(1,21)', 'range(1,len(segments))').replace('while s+4*k<21:', 'while s+4*k<len(segments):')
    extracted = textwrap.dedent(new_f['enumerate_windows'].split('\n', 1)[1]).removesuffix('return windows')
    assert modified == extracted
    assert "if __name__=='__main__':" in new
    compiled = []
    for path in [A / 'check_joint.py', A / 'assemble.py', A / 'seal.py', Path(__file__)]:
        compile(path.read_text(), str(path), 'exec')
        compiled.append(str(path))
    # 普通导入，不调用任何作者函数，不生成 pycache。无 main guard 会因缺 --out 失败。
    code = ('import importlib.util; p=' + repr(str(A / 'check_joint.py')) + '; '
            's=importlib.util.spec_from_file_location("stage69_import_only",p); '
            'm=importlib.util.module_from_spec(s); s.loader.exec_module(m); '
            'print("import_only_ok")')
    imported = subprocess.run([sys.executable, '-B', '-c', code], cwd=R,
                              capture_output=True, text=True, timeout=20, check=True)
    assert imported.stdout == 'import_only_ok\n' and not imported.stderr
    return {'functions_verbatim': functions, 'window_block_only_two_upper_bound_changes': True,
            'ordinary_import_only': imported.stdout.strip(), 'author_functions_executed': 0,
            'compiled_scripts': compiled,
            'static_tools': {name: bool(importlib.util.find_spec(name)) for name in ('ruff', 'mypy', 'pylint', 'black')},
            'out_mkdir_exclusive': 'out.mkdir(parents=False,exist_ok=False)' in new,
            'cwd_recorded': "'cwd':str(Path.cwd())" in new,
            'argv_recorded': "'argv':sys.argv" in new}


def source_capture() -> list[dict]:
    requests = [
        ('docs/chanlun/text/blog/084-第84课.md', [52, 54, 56]),
        ('docs/chanlun/text/blog/037-第37课.md', [16, 18, 20, 22]),
        ('docs/chanlun/text/blog/018-第18课.md', [24, 26, 28]),
        ('docs/chanlun/text/blog/020-第20课.md', [52, 54, 56, 58]),
        ('docs/chanlun/text/blog/043-第43课.md', [14, 34, 38, 48]),
        ('docs/chanlun/text/blog/061-第61课.md', [26, 28]),
        ('docs/chanlun/text/blog/038-第38课.md', [22, 24, 26]),
        ('docs/adr/0011-operation-decomposition-layer.md', list(range(55, 71))),
        ('.chanlun/definitions/beichi.md', [263, 264, 265, 266, 267, 268, 313, 315, 317, 318, 319, 320, 321, 323, 338]),
    ]
    sources = []
    for rel, numbers in requests:
        path = R / rel
        lines = path.read_text().splitlines()
        boundary = next((i for i, line in enumerate(lines, 1) if '↑正文' in line), None)
        sources.append({'path': str(path), 'sha256': digest(path), 'body_boundary': boundary,
                        'lines': [{'line': i, 'text': lines[i - 1],
                                   'within_body': None if boundary is None else i < boundary,
                                   'annotation_present': any(marker in lines[i - 1] for marker in ('娇注', '娇：', '注：', '注:'))}
                                  for i in numbers]})
    return sources


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--out', required=True, type=Path)
    out = parser.parse_args().out.resolve()
    assert out.parent == HERE
    out.mkdir(exist_ok=False)
    started, started_utc = time.monotonic(), datetime.now(timezone.utc).isoformat()
    fixed = {'FINAL.json': 'dec23bf756965dd7ec395853b8d17327516cad49ad484d05d3f1c47e4c5b47a8',
             'manifest.json': '4ceb2a71d4d3fe690a3a5db4bc2ab72ec315b78c7791727067fea7342c03e915',
             'Report.md': '065d72a26e93a55b721e481fb08be1d2811ff64146305ca8aa21e31284a30e98'}
    for name, expected in fixed.items():
        equal(digest(A / name), expected, name)
    author_manifest = read(A / 'manifest.json')
    for entry in author_manifest['entries']:
        equal(digest(A / entry['path']), entry['sha256'], 'author manifest ' + entry['path'])
    sources = source_capture()
    inputs = [A / item['path'] for item in author_manifest['entries']]
    inputs += [A / name for name in fixed]
    inputs += [Path(item['path']) for item in read(A / 'inputhash-final.json')]
    inputs += [Path(s['path']) for s in sources] + [Path(__file__)]
    before = fingerprints(inputs)
    write(out, 'inputhash-before.json', before)
    write(out, 'sources.json', sources)
    rules = audit_rules()
    write(out, 'implementation-audit.json', rules)

    # 旧控制使用独立实现，比较窗口、全部路径、分类的全部字段。
    old_obs = read(E / 'stage66/nested-author/run/source.json')['observations']
    old_pens = read(E / 'stage66/nested-review/independent-pens.json')
    old_raw = read(OLD / 'raw-roots.json')
    old_rebuilt = raw_inputs(old_raw, old_pens, old_obs)
    equal(old_rebuilt, old_raw, 'old raw all fields')
    old_windows = build_windows(old_obs, old_rebuilt)
    equal(old_windows, read(OLD / 'all-windows.json'), '40 old windows all fields')
    old_paths, old_classes = read(OLD / 'all-delta-paths.json'), read(OLD / 'path-classes.json')
    for t in range(443):
        paths, classes, _ = prefix_records(t, old_windows, old_rebuilt)
        equal(paths, old_paths[t], f'old paths all fields {t}')
        equal(classes, old_classes[t], f'old classes all fields {t}')
    write(out, 'stage68-control.json', {'raw_all_fields': 21, 'window_all_fields': 40,
                                      'all_path_all_fields_prefixes': 443, 'classes_all_fields_prefixes': 443,
                                      'method': 'independent reconstruction; no author function call',
                                      'not_claimed': 'all other Stage68 output artifacts or original F1 qualification'})
    del old_paths, old_classes
    gc.collect()

    expected_h2 = {'source.json': 'ce7ba6ab38666b59b978dc942913e6a2b0c38c26409e9c14b70114a69b3260c6',
                   'segments.json': '89cf6da6cd2174c13d5c3786cfd8cb28a059a125836c86abe5d8cf09fd9e0a3e',
                   'reference.stdout': '7491058def0984a3b3ea5eb6b387f8b65b5484beb3cd203860e07c88afd66792'}
    manifest = {x['path']: x['sha256'] for x in read(H2 / 'manifest.json')['artifacts']}
    historical = {x['path']: x['sha256'] for x in read(E / 'stage64/dynamic-review/input-hashes.json')['files']}
    for name, value in expected_h2.items():
        assert digest(H2 / name) == value == manifest[name] == historical['v2/run/' + name]
    source, raw = read(H2 / 'source.json'), read(H2 / 'segments.json')
    obs, events = source['observations'], source['events']
    assert source['name'] == 'T64-H2-dynamic-repair'
    assert len(obs) == 583 and len(events) == 582 and len(raw) == 28
    for t, observation in enumerate(obs):
        assert observation['index'] == observation['time'] == t
    # 数量事件合法性及保存观察逐项交叉检验，不重跑 R_W。
    q, ask = int(source['initial']['bid_quantity']), int(source['initial']['ask_quantity'])
    for t, observation in enumerate(obs):
        if t:
            event = events[t - 1]
            assert event['id'] == event['time'] == t and event['side'] == 'bid'
            assert event['price'] == source['bid'] and int(event['quantity_before']) == q
            amount = int(event['amount'])
            assert amount > 0 and event['action'] in ('add', 'cancel')
            if event['action'] == 'cancel':
                assert amount < q
            q += amount if event['action'] == 'add' else -amount
            assert q == int(event['quantity_after']) and q > 0
        price = Fraction(source['ask'] * q + source['bid'] * ask, q + ask)
        assert price == observation['price']
        assert q == int(observation['bid_quantity']) and ask == int(observation['ask_quantity'])
    ref = read(H2 / 'reference.stdout')
    pens = ref['strokes']
    assert len(pens) == 144 and len(ref['prefixes']) == 583
    for index, pen in enumerate(pens):
        assert pen['start_price'] == obs[pen['start']]['price']
        assert pen['end_price'] == obs[pen['end']]['price']
        assert pen['start'] < pen['end']
        assert pen['up'] == (pen['end_price'] > pen['start_price'])
        if index:
            assert pens[index - 1]['end'] == pen['start']
            assert pens[index - 1]['up'] != pen['up']
    audited = read(E / 'stage64/dynamic-review/H2-prefix-audit.json')
    equal(raw, audited['certificates'], 'H2 audited segment identity')
    stable_checks = 0
    for segment in raw:
        lo, hi = segment['members']
        for t in range(segment['known_at'], 583):
            equal(ref['prefixes'][t]['stable'][lo:hi], pens[lo:hi], 'known-time stable identities')
            stable_checks += 1
    del ref
    gc.collect()
    rebuilt = raw_inputs(raw, pens, obs)
    equal(rebuilt, read(A / 'run/raw-roots.json'), 'H2 raw full-field comparison')
    write(out, 'independent-raws.json', rebuilt)
    # 唯一一次独立 H2 全窗口计算。
    windows = build_windows(obs, rebuilt)
    assert len(windows) == 78
    equal(windows, read(A / 'run/all-windows.json'), '78 H2 windows all fields')
    write(out, 'independent-windows.json', windows)
    layer = [len(windows)]
    pool = windows
    for predicate in [lambda w: w['structural_pass'], lambda w: w['role_pass'],
                      lambda w: w['checks']['whole_extreme'], lambda w: w['checks']['signed_force_weakening']]:
        pool = [w for w in pool if predicate(w)]
        layer.append(len(pool))
    assert layer == [78, 23, 7, 7, 5]
    edges = [w for w in windows if w['JEnd68']]
    equal([[w['raw_start'], w['raw_stop']] for w in edges], [[4, 9], [9, 18], [13, 18], [18, 27], [22, 27]], 'edges')
    author_paths = read(A / 'run/all-delta-paths.json')
    author_classes = read(A / 'run/path-classes.json')
    author_traces = read(A / 'run/prefix-results.json')
    paths_out, classes_out, traces_out = [], [], []
    for t in range(583):
        paths, classes, trace = prefix_records(t, windows, rebuilt)
        equal(paths, author_paths[t], f'H2 all paths all fields {t}')
        equal(classes, author_classes[t], f'H2 classes all fields {t}')
        equal(trace, author_traces[t], f'H2 consumers/counts/raw known all fields {t}')
        equal(trace['raw_known'], audited['certificate_ids_by_prefix'][t], 'audited known times')
        assert paths['all_path_count'] == 1 and paths['nonempty_path_count'] == 0
        assert not paths['all_paths'][0]['delta']
        for w in windows:
            if w['candidate_known'] <= t:
                assert w['end'] <= w['candidate_known'] <= t
                assert all(raw[i]['known_at'] <= t for i in range(w['raw_start'], w['raw_stop']))
        paths_out.append(paths)
        classes_out.append(classes)
        traces_out.append(trace)
    write(out, 'independent-all-paths.json', paths_out)
    write(out, 'independent-path-classes.json', classes_out)
    write(out, 'independent-prefix-results.json', traces_out)

    roots = [w for w in windows if w['raw_start'] == 1]
    assert [w['k'] for w in roots] == [1, 2, 3, 4, 5, 6]
    assert not any(w['JEnd68'] for w in roots)
    coordinate = {(w['raw_start'], w['raw_stop']): w for w in windows}
    types = []
    for count in range(3, 6):
        for ks in itertools.product(range(1, 7), repeat=count):
            if sum(4 * k + 1 for k in ks) > 27 or any(a == b == 1 for a, b in zip(ks, ks[1:])):
                continue
            position, plan = 1, []
            for k in ks:
                next_position = position + 4 * k + 1
                plan.append(coordinate[position, next_position]['id'])
                position = next_position
            types.append({'ks': list(ks), 'lengths': [4 * k + 1 for k in ks], 'windows': plan})
    assert len(types) == 13 and all(len(x['ks']) == 3 for x in types)
    author_budget = read(A / 'run/budget-obstruction.json')['plans_with_at_least_three_and_structural_NoPP']
    equal(types, [{k: plan[k] for k in ('ks', 'lengths', 'windows')} for plan in author_budget], 'all13 NoPP length types')
    write(out, 'root-obstruction.json', {'root': 1, 'prelude': 'S0', 'root_windows': roots,
                                       'all_k_covered': [1, 2, 3, 4, 5, 6], 'nopp_length_types': types,
                                       'four_factors_lower_bound': 28,
                                       'proof': 'Every nonempty rooted path starts with one of these six windows. All fail; every prefix graph is a subgraph. NoPP is unused by this proof.'})
    internal_paths = [p for vertex in range(1, 29) for p in graph_paths(edges, vertex) if len(p) >= 3]
    assert len(internal_paths) == 1
    tri = internal_paths[0]
    contract = consumer(tri)
    existing_internal = read(A / 'internal-chain.json')
    equal(tri, existing_internal['candidate_factors'], 'internal factors all fields')
    equal(contract, existing_internal['fixed_F2_necessary_conditions'], 'internal consumer all fields')
    assert contract['candidate_interface_pass'] and contract['candidate_center']['core'] == [50000, 80000]
    gap = [raw[0]['end'] + 1, tri[0]['start']]
    assert gap == [22, 81] and max(w['candidate_known'] for w in tri) == 558
    write(out, 'independent-internal-chain.json', {'factors': tri, 'consumer': contract,
                                                 'root_gap_events': gap, 'known': 558,
                                                 'all_graph_three_factor_paths': 1, 'rooted_three_factor_paths': 0,
                                                 'original_admitted_triples': 0})
    differences = [i for i, (a, b) in enumerate(zip(old_obs, obs)) if a['price'] != b['price']]
    assert len(differences) == 442 and differences[0] == 0
    write(out, 'history-comparison.json', {'observations_compared': 443, 'different_prices': len(differences),
                                         'first_difference': {'index': 0, 'stage68': old_obs[0]['price'], 'H2': obs[0]['price']},
                                         'prefix_extension': False, 'length_only_causal_attribution': False})
    after = fingerprints(inputs)
    equal(after, before, 'all input before/after')
    write(out, 'inputhash-after.json', after)
    rss = resource.getrusage(resource.RUSAGE_SELF).ru_maxrss * (1 if platform.system() == 'Darwin' else 1024)
    assert rss <= LIMIT
    summary = {'status': 'independent_finite_result_accept', 'h2_hashes': expected_h2,
               'observations': 583, 'events': 582, 'pens': 144, 'audited_raws': 28,
               'quantity_reconstructed_observations': 583, 'stable_identity_checks': stable_checks,
               'windows_all_fields_equal': 78, 'layers': layer,
               'edges': [[w['raw_start'], w['raw_stop']] for w in edges],
               'prefixes_all_fields_equal': 583, 'rooted_nonempty': 0, 'rooted_three': 0,
               'internal_numerical_three': 1, 'original_admitted_three': 0,
               'NoPP_length_types': 13, 'new_histories': 0, 'R_W_runs': 0,
               'independent_H2_window_passes': 1, 'author_candidate_functions_executed': 0,
               'input_hashes_unchanged': True, 'max_rss_bytes': rss, 'original_F1_F2_qualified': False}
    write(out, 'result.json', summary)
    write(out, 'receipt.json', {'cwd': str(Path.cwd()), 'argv': sys.argv, 'python': sys.executable,
                              'started_at': started_utc, 'elapsed_seconds': time.monotonic() - started,
                              'script_sha256': digest(Path(__file__)), 'max_rss_bytes': rss,
                              'rss_limit_bytes': LIMIT, 'exit_code': 0,
                              'artifact_bytes': sum(p.stat().st_size for p in HERE.rglob('*') if p.is_file()),
                              'ordinary_author_import_only': True, 'author_functions_executed': 0,
                              'H2_window_passes': 1, 'stage68_control_window_passes': 1})
    print(json.dumps(summary, ensure_ascii=False, indent=2))


if __name__ == '__main__':
    main()
