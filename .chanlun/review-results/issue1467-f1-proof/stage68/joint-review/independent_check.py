#!/usr/bin/env python3
"""独立检查冻结 JointCut；只读原观察、已审笔段和作者输出，不导入作者函数。"""
from __future__ import annotations

import collections
import datetime as dt
import hashlib
import itertools
import json
import platform
import resource
import time
from fractions import Fraction
from pathlib import Path

E = Path('/Users/silencehan/Documents/Codex/research-evidence/issue1467')
A = E / 'stage68/joint-author'
OUT = Path(__file__).resolve().parent
LIMIT = 96 * 1024 * 1024


def load(path: Path):
    """读取具有固定路径的 JSON 数据。"""
    return json.loads(path.read_text())


def save(name: str, value: object) -> None:
    """只写本次独立评审目录。"""
    (OUT / name).write_text(json.dumps(value, ensure_ascii=False, indent=2) + '\n')


def digest(path: Path) -> str:
    """返回输入内容指纹。"""
    return hashlib.sha256(path.read_bytes()).hexdigest()


def intersect(intervals: list[list[int]]) -> list[int]:
    """完整材料范围的交，不事先筛掉无交窗口。"""
    return [max(r[0] for r in intervals), min(r[1] for r in intervals)]


def window(s: int, e: int, segments: list[dict], prices: list[int]) -> dict:
    """从真实价格与原始段重新组装一个首末均包含的窗口。"""
    chosen = segments[s:e + 1]
    assert len(chosen) % 4 == 1 and len(chosen) >= 5
    clusters = []
    for pos in range(s + 1, e, 4):
        block = segments[pos:pos + 3]
        interval = intersect([x['range'] for x in block])
        clusters.append({
            'id': f'IC68:{s}:{len(clusters)}',
            'members': [x['id'] for x in block], 'core': interval,
            'Outer0': interval, 'Outer0_original_DD_GG': None,
            'own': [block[0]['start'] + 1, block[-1]['end']],
            'strict': interval[0] < interval[1],
            'direction_alternates': all(x['up'] != y['up'] for x, y in zip(block, block[1:])),
            'happened': block[-1]['end'],
            'raw_known': max(x['known_at'] for x in block),
        })
    core_bounds = [x['core'] for x in clusters]
    movement = None
    if len(clusters) == 1:
        movement = 'P'
    elif all(x[1] < y[0] for x, y in zip(core_bounds, core_bounds[1:])):
        movement = 'U'
    elif all(y[1] < x[0] for x, y in zip(core_bounds, core_bounds[1:])):
        movement = 'D'
    ref, end = segments[e - 4], segments[e]
    lo, hi = core_bounds[-1]
    rp, rq = prices[ref['start']], prices[ref['end']]
    cp, cq = prices[end['start']], prices[end['end']]
    prior = prices[chosen[0]['start']:end['start'] + 1]
    prior_extreme = max(prior) if end['up'] else min(prior)
    checks = {
        'strict_cores': all(x['strict'] for x in clusters),
        'member_direction_alternates': all(x['direction_alternates'] for x in clusters),
        'coherent_type': movement is not None and (movement == 'P' or (movement == 'U') == end['up']),
        'same_direction_comparison_arms': ref['up'] == end['up'],
        'entry_cross': rp < lo <= rq if end['up'] else rp > hi >= rq,
        'exit_cross': cp <= hi < cq if end['up'] else cp >= lo > cq,
        'whole_extreme': cq > prior_extreme if end['up'] else cq < prior_extreme,
        'signed_force_weakening': end['force']['L'] < ref['force']['L'],
    }
    structural = all(checks[x] for x in ('strict_cores', 'member_direction_alternates', 'coherent_type'))
    role = all(checks[x] for x in ('same_direction_comparison_arms', 'entry_cross', 'exit_cross'))
    owned = list(itertools.chain.from_iterable(range(x['start'] + 1, x['end'] + 1) for x in chosen))
    assert owned == list(range(chosen[0]['start'] + 1, end['end'] + 1))
    actual_prices = prices[chosen[0]['start']:end['end'] + 1]
    return {
        'id': f'W68:S{s}-S{e}', 'raw_start': s, 'raw_stop': e + 1,
        'k': len(clusters), 'grade': 0, 'kind': movement,
        'conceptual_direction': None if movement == 'P' else movement,
        'technical_up': end['up'], 'raws': [x['id'] for x in chosen], 'cores': clusters,
        'start': chosen[0]['start'], 'end': end['end'], 'own': [owned[0], owned[-1]],
        'whole': [min(actual_prices), max(actual_prices)],
        'b_ref': ref['id'], 'c': end['id'], 'b_force': ref['force'], 'c_force': end['force'],
        'prior_whole_extreme': prior_extreme, 'end_price': cq,
        'candidate_happened': end['end'], 'candidate_known': max(x['known_at'] for x in chosen),
        'structural_pass': structural, 'role_pass': role,
        'CycleDiv68': all(checks.values()), 'RawEnd68': True, 'JEnd68': all(checks.values()),
        'OriginalGeneralDiv': None, 'OriginalMovementCompleted': None,
        'original_happened': None, 'original_known': None, 'original_published': None,
        'checks': checks, 'rejections': [name for name, passed in checks.items() if not passed],
        'owns_each_event_once': len(owned) == len(set(owned)),
    }


def all_paths(edges: list[dict]) -> list[list[dict]]:
    """直接枚举所有根路径，包括每个短前缀；不从作者 maximal 结果求闭包。"""
    found = [[]]
    for path in found:
        cursor = path[-1]['raw_stop'] if path else 1
        found.extend(path + [edge] for edge in edges if edge['raw_start'] == cursor)
    return sorted(found, key=lambda p: (len(p), [x['id'] for x in p]))


def path_row(path: list[dict], edges: list[dict], t: int) -> dict:
    """逐事件核真实因子、尾、是否可继续及 NoPP。"""
    cursor = path[-1]['raw_stop'] if path else 1
    end = path[-1]['end'] if path else 21
    extendable = any(w['raw_start'] == cursor for w in edges)
    pp = [[a['id'], b['id']] for a, b in zip(path, path[1:]) if a['kind'] == b['kind'] == 'P']
    owned = [e for w in path for e in range(w['start'] + 1, w['end'] + 1)]
    prelude = list(range(1, min(t, 21) + 1))
    tail = list(range(end + 1, t + 1))
    assert prelude + owned + tail == list(range(1, t + 1))
    return {
        'delta': [w['id'] for w in path], 'kinds': [w['kind'] for w in path],
        'is_maximal': not extendable, 'no_pp': not pp, 'pp_violations': pp,
        'has_three': len(path) >= 3, 'candidate_F2_pass': False, 'original_F2_pass': False,
        'factor_owns': [w['own'] for w in path],
        'active_tail': [end + 1, t] if end < t else [], 'original_completed': [None] * len(path),
        'extendable_now': extendable,
        'candidate_windows_inside_tail': [w['id'] for w in edges if w['start'] >= end],
        'complete_original_decomposition': None,
    }


def check_segments(source: dict, pens: list[dict], certs: list[dict]) -> list[dict]:
    """信任已审 raw 证书身份，独立复核端点、全范围、力度及前缀笔证。"""
    observations = source['observations']
    raw_prefixes = load(E / 'stage66/nested-review/independent-prefixes.json')
    result = []
    for i, cert in enumerate(sorted(certs, key=lambda x: x['start'])):
        assert cert['id'] == f'S{i}'
        a, b = cert['members']
        actual_pens = pens[a:b]
        assert actual_pens[0]['start'] == cert['start'] and actual_pens[-1]['end'] == cert['end']
        speeds = []
        for pen in actual_pens:
            start, end = observations[pen['start']], observations[pen['end']]
            assert (start['price'], end['price']) == (pen['start_price'], pen['end_price'])
            assert pen['up'] == (end['price'] > start['price'])
            speeds.append(Fraction(end['price'] - start['price'], end['time'] - start['time']))
        assert all(p['end'] == q['start'] for p, q in zip(actual_pens, actual_pens[1:]))
        own_prices = [x['price'] for x in observations[cert['start']:cert['end'] + 1]]
        assert cert['range'] == [min(own_prices), max(own_prices)]
        assert cert['up'] == (observations[cert['end']]['price'] > observations[cert['start']]['price'])
        for t in range(cert['known_at'], len(observations)):
            assert raw_prefixes[t]['cut'] == t and raw_prefixes[t]['stable'][a:b] == actual_pens
        assert cert['end'] <= cert['known_at']
        assert not result or result[-1]['end'] == cert['start']
        assert all(x.denominator == 1 for x in speeds)
        result.append({**cert, 'range': [min(own_prices), max(own_prices)],
                       'force': {'first_pen': a, 'last_pen': b - 1, 'first_v': int(speeds[0]),
                                 'last_v': int(speeds[-1]), 'L': int(speeds[-1] - speeds[0])}})
    return result


def main() -> None:
    """运行一次独立有限证书检查，所有对拍都在独立计算之后。"""
    started = time.monotonic()
    input_paths = [E / f'stage66/{name}' for name in (
        'nested-author/run/source.json', 'nested-review/independent-pens.json',
        'nested-review/independent-narrow-segments.json', 'nested-review/independent-S15.json',
        'nested-review/independent-prefixes.json')]
    input_paths += [A / name for name in ('FINAL.json', 'manifest.json', 'Report.md', 'WorkCard-v2.md',
                   'all-windows.json', 'raw-roots.json', 'prefix-results.json', 'all-delta-paths.json',
                   'path-classes.json', 'model.json', 'finite-obstruction.json')]
    input_paths.append(Path(__file__).resolve())
    before = {str(p): digest(p) for p in input_paths}
    save('inputhash-before.json', before)
    source, pens = load(input_paths[0]), load(input_paths[1])
    obs = source['observations']
    assert len(obs) == 443 and all(x['index'] == x['time'] == i for i, x in enumerate(obs))
    certs = load(input_paths[2])['certificates'] + [load(input_paths[3])]
    segments = check_segments(source, pens, certs)
    assert len(segments) == 21
    prices = [x['price'] for x in obs]
    slots = [(s, e) for s in range(1, 21) for e in range(s + 4, 21, 4)]
    rows = [window(s, e, segments, prices) for s, e in slots]
    assert len(rows) == 40
    save('independent-windows.json', rows)
    save('independent-raw-force.json', segments)
    edges = [w for w in rows if w['JEnd68']]
    prefixes = []
    for t in range(443):
        available = [w for w in rows if w['candidate_known'] <= t]
        for w in available:
            assert window(w['raw_start'], w['raw_stop'] - 1, segments, prices[:t + 1]) == w
        visible_edges = [w for w in available if w['JEnd68']]
        paths = all_paths(visible_edges)
        assert all(len(path) < 3 for path in paths)
        detail = [path_row(path, visible_edges, t) for path in paths]
        prefixes.append({'t': t, 'raw_known': [s['id'] for s in segments if s['known_at'] <= t],
                         'edges': [w['id'] for w in visible_edges], 'all_paths': detail,
                         'counts': {'all': len(paths), 'nonempty': len(paths) - 1,
                                    'no_pp_nonempty': sum(bool(p) and r['no_pp'] for p, r in zip(paths, detail)),
                                    'three_children': 0, 'NoPP_and_three': 0},
                         'available_count': len(available)})
    save('independent-prefixes.json', prefixes)
    # Enumerate the complete finite length language before applying numeric predicates.
    frontier = [()]
    for seq in frontier:
        frontier.extend(seq + (k,) for k in range(1, 5) if sum(4 * x + 1 for x in seq) + 4 * k + 1 <= 20)
    no_pp_long = [p for p in frontier if len(p) >= 3 and all(a != 1 or b != 1 for a, b in zip(p, p[1:]))]
    assert no_pp_long == [(1, 2, 1)]
    bounds = []
    for n in range(1, 11):
        min_trends = min(sum(bits) for bits in itertools.product((0, 1), repeat=n)
                         if all(x or y for x, y in zip(bits, bits[1:])))
        assert min_trends == n // 2
        bounds.append({'n': n, 'minimum_nonP': min_trends, 'length_lower_bound': 5 * n + 4 * min_trends})
    middle = next(w for w in rows if w['id'] == 'W68:S6-S14')
    assert middle['b_force']['L'] == -1200 and middle['c_force']['L'] == -300
    assert middle['end_price'] == 60800 and middle['prior_whole_extreme'] == 60000
    assert middle['structural_pass'] and middle['role_pass'] and not middle['JEnd68']
    save('independent-obstruction.json', {'all_length_sequences': [list(x) for x in frontier],
         'NoPP_at_least_three': [list(x) for x in no_pp_long], 'bounds': bounds,
         'middle': middle, 'proof': 'NoPP gives at most ceil(n/2) P, so at least floor(n/2) nonP. Each P needs5 and each nonP needs>=9. Thus >=5n+4floor(n/2). At n>=4 >=28; n=3 under20 forces (k1,k2,k3)=(1,2,1). The fixed start fixes actual cuts. The middle fails two necessary conjuncts.'})
    # Author data serves solely as a comparison target, never as construction input.
    author_windows = load(A / 'all-windows.json')
    assert rows == author_windows
    author_classes = load(A / 'path-classes.json')
    author_all = load(A / 'all-delta-paths.json')
    author_main = load(A / 'prefix-results.json')
    for own, classes, allrow, mainrow in zip(prefixes, author_classes, author_all, author_main, strict=True):
        assert own['t'] == classes['t'] == allrow['t'] == mainrow['t']
        assert own['counts'] == classes['counts']
        assert own['raw_known'] == mainrow['raw_known']
        assert own['available_count'] == mainrow['counts']['available_grammar_windows']
        assert own['counts']['all'] == allrow['all_path_count']
        for p, c, q in zip(own['all_paths'], classes['all_edge_paths'], allrow['all_paths'], strict=True):
            assert all(c[k] == v for k, v in p.items()), (own['t'], p, c)
            assert all(p[k] == v for k, v in q.items()), (own['t'], p, q)
        maximal = [p['delta'] for p in own['all_paths'] if p['is_maximal']]
        assert maximal == [p['delta'] for p in mainrow['deltas']]
    counts = {'grammar': len(rows), 'structure': sum(x['structural_pass'] for x in rows),
              'roles_after_structure': sum(x['structural_pass'] and x['role_pass'] for x in rows),
              'extreme_after_roles': sum(x['structural_pass'] and x['role_pass'] and x['checks']['whole_extreme'] for x in rows),
              'JEnd68': len(edges)}
    after = {str(p): digest(p) for p in input_paths}
    save('inputhash-after.json', after)
    assert before == after
    rss = resource.getrusage(resource.RUSAGE_SELF).ru_maxrss * (1 if platform.system() == 'Darwin' else 1024)
    assert rss <= LIMIT
    result = {'counts': counts, 'edges': [{'id': x['id'], 'edge': [x['raw_start'], x['raw_stop']],
        'own': x['own'], 'happened': x['candidate_happened'], 'known': x['candidate_known']} for x in edges],
        'all_40_window_fields_match': True, 'all_443_path_classes_match': True,
        'all_443_price_support_prefix_recomputations_match': True,
        'selected_prefixes': [prefixes[t] for t in (138, 238, 338, 438, 442)],
        'NoPP_removed_max_children': max(len(p['delta']) for r in prefixes for p in r['all_paths']),
        'source_parser_reruns': 0, 'new_histories': 0, 'original_completion_claim': False}
    save('independent-results.json', result)
    save('receipt.json', {'command': ['python3', str(Path(__file__).resolve())], 'cwd': str(Path.cwd()),
        'completed_utc': dt.datetime.now(dt.timezone.utc).isoformat(), 'elapsed_seconds': time.monotonic() - started,
        'max_rss_bytes': rss, 'limit_bytes': LIMIT, 'inputs_unchanged': True,
        'author_scripts_imported_or_executed': False, 'parser_executed': False, 'exit_code': 0})
    print(json.dumps({'counts': counts, 'rss_bytes': rss, 'windows_equal': True, '443_prefixes_equal': True}))


if __name__ == '__main__':
    main()
