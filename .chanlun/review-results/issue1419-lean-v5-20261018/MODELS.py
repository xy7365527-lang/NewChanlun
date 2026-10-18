"""#1419 v5 工作草稿：仅整数/列表数学模型；不加载 Lean、Rust 或产品。"""
from copy import deepcopy
from itertools import product
import json
from pathlib import Path

OUT = Path(__file__).resolve().parent
OPEN, CONFIRMED, KILLED = 'OPEN', 'CONFIRMED', 'KILLED'


def elem(s):
    return min(s[1:]), max(s[1:])


def projected(direction, b, xs):
    for lo, hi in xs:
        if (hi > b[1]) if direction == 1 else (lo < b[0]):
            return KILLED
        if (lo < b[0]) if direction == 1 else (hi > b[1]):
            return CONFIRMED
    return OPEN


def raw(direction, b, xs):
    for s in xs:
        lo, hi = elem(s)
        if (hi > b[1]) if direction == 1 else (lo < b[0]):
            return KILLED
        if s[0] == -direction:
            result = projected(direction, b, [(lo, hi)])
            if result != OPEN:
                return result
    return OPEN


def first_endpoint_oracle(direction, b, xs):
    """连续单调笔按起点、终点事件顺序检查；只有 b 同向笔可以确认。"""
    for d, start, end in xs:
        for price in (start, end):
            if direction * price > (b[1] if direction == 1 else -b[0]):
                return KILLED
            if d == -direction and direction * price < (b[0] if direction == 1 else -b[1]):
                return CONFIRMED
    return OPEN


def chains(start, direction, values, depth):
    yield ()
    if depth:
        for end in values:
            if direction * end > direction * start:
                for rest in chains(end, -direction, values, depth - 1):
                    yield ((direction, start, end),) + rest


def initial(direction):
    return dict(consumed=0, through=0, raw=[], standard=[], ds=-1 if direction == 1 else None,
                watermark=None, decision=None, matched=None, killed=None)


def feature(s, i):
    lo, hi = elem(s)
    return (lo, hi, i, (i,))


def forward_step(st, direction, p, s):
    """按固定 Rust 源逐句转写的数学投影，Vec 以正序表示。"""
    st = deepcopy(st)
    if st['decision'] is not None:
        return st
    i = st['consumed'] + 1
    st['consumed'] += 1
    st['through'] = i
    if s[0] != direction:
        return st
    lo, hi, _, _ = incoming = feature(s, i)
    reach = hi if direction == 1 else lo
    old = st['watermark']
    st['watermark'] = reach if old is None else (max(old, reach) if direction == 1 else min(old, reach))
    st['raw'].append(incoming)
    es = st['standard']
    if es:
        llo, lhi, _, members = es[-1]
        inclusion = (llo <= lo and hi <= lhi) or (lo <= llo and lhi <= hi)
        if inclusion:
            merge = min if st['ds'] == -1 else max
            es[-1] = (merge(llo, lo), merge(lhi, hi), i, members + (i,))
        else:
            if hi > lhi and lo > llo:
                st['ds'] = 1
            elif hi < lhi and lo < llo:
                st['ds'] = -1
            es.append(incoming)
    else:
        es.append(incoming)
    if len(es) >= 3:
        a, b, c = es[-3:]
        if (b[1] > a[1] and b[1] > c[1]) or (b[0] < a[0] and b[0] < c[0]):
            st['matched'] = tuple(es[-3:])
            st['decision'] = CONFIRMED
            return st
    if direction * reach > direction * p:
        st['decision'] = KILLED
        st['killed'] = (i, reach, p)
    return st


def reverse_step(st, direction, p, s):
    """Lean 新增计算投影的数学转写，列表头对应 Vec.last；不调用 forward_step。"""
    if st['decision'] is not None:
        return st
    n = dict(st, consumed=st['consumed'] + 1, through=st['consumed'] + 1)
    if s[0] != direction:
        return n
    x = feature(s, n['through'])
    lo, hi = x[:2]
    rev = st['standard']
    ds = st['ds']
    if not rev:
        es = [x]
    else:
        last, *rest = rev
        if (last[0] <= lo and hi <= last[1]) or (lo <= last[0] and last[1] <= hi):
            operation = min if ds == -1 else max
            es = [(operation(last[0], lo), operation(last[1], hi), x[2], last[3] + x[3])] + rest
        else:
            es = [x] + rev
            ds = 1 if last[1] < hi and last[0] < lo else (-1 if hi < last[1] and lo < last[0] else ds)
    reach = (lo, hi)[direction == 1]
    w = st['watermark']
    n.update(raw=[x] + st['raw'], standard=es, ds=ds,
             watermark=reach if w is None else (max(w, reach) if direction == 1 else min(w, reach)))
    fractal = False
    if len(es) >= 3:
        c, b, a = es[:3]
        fractal = (a[1] < b[1] and c[1] < b[1]) or (b[0] < a[0] and b[0] < c[0])
    if fractal:
        n.update(decision=CONFIRMED, matched=(a, b, c))
    elif (hi > p if direction == 1 else lo < p):
        n.update(decision=KILLED, killed=(x[2], reach, p))
    return n


def run(step, st, d, p, xs):
    for s in xs:
        st = step(st, d, p, s)
    return st


def forward_view(st):
    return dict(st, raw=list(reversed(st['raw'])), standard=list(reversed(st['standard'])))


def finish(st, available, budget):
    if st['decision'] is not None:
        return st['decision'], 'COMPLETE'
    return (None, 'INCOMPLETE_RESOURCE_BOUND') if budget < available else (OPEN, 'COMPLETE')


def main():
    counts = dict(first_prefixes=0, first_projection_preconditions=0, second_prefixes=0,
                  second_budget_cases=0, second_chunk_splits=0, boundary_observations=0)
    for d in (1, -1):
        b = (2, 10) if d == 1 else (-10, -2)
        values = [d * v for v in (0, 1, 2, 3, 8, 9, 10, 11, 12)]
        for xs in chains(d * 2, d, values, 4):
            result = raw(d, b, xs)
            assert result == first_endpoint_oracle(d, b, xs)
            assert (result == OPEN) == all(b[0] <= elem(s)[0] and elem(s)[1] <= b[1] for s in xs)
            ignored_safe = all((elem(s)[1] <= b[1] if d == 1 else b[0] <= elem(s)[0])
                               for s in xs if s[0] != -d)
            if ignored_safe:
                assert result == projected(d, b, [elem(s) for s in xs if s[0] == -d])
                counts['first_projection_preconditions'] += 1
            for split in range(len(xs) + 1):
                early = raw(d, b, xs[:split])
                assert result == (early if early != OPEN else raw(d, b, xs[split:]))
            counts['first_prefixes'] += 1
        for lo, hi in product((-1, 2, 10, 12), repeat=2):
            if lo > hi:
                continue
            for sd in (1, -1):
                s = (sd, lo, hi) if sd == 1 else (sd, hi, lo)
                ob = raw(d, b, [s])
                crosses = hi > b[1] if d == 1 else lo < b[0]
                assert not crosses or ob == KILLED
                assert sd == -d or ob != CONFIRMED
                counts['boundary_observations'] += 1
        for xs in chains(d, d, [d * v for v in range(5)], 6):
            p = d * 3
            sf = run(forward_step, initial(d), d, p, xs)
            sr = run(reverse_step, initial(d), d, p, xs)
            assert sf == forward_view(sr)
            prices = [e[1] if d == 1 else e[0] for e in sr['raw']]
            assert sr['watermark'] == (None if not prices else (max(prices) if d == 1 else min(prices)))
            for split in range(len(xs) + 1):
                mid = run(reverse_step, initial(d), d, p, xs[:split])
                assert run(reverse_step, mid, d, p, xs[split:]) == sr
                counts['second_chunk_splits'] += 1
            for budget in range(len(xs) + 2):
                state = run(reverse_step, initial(d), d, p, xs[:budget])
                out = finish(state, len(xs), budget)
                assert out == finish(run(forward_step, initial(d), d, p, xs[:budget]), len(xs), budget)
                assert (out[0] == OPEN) == (state['decision'] is None and len(xs) <= budget)
                if state['decision'] is None:
                    assert state['consumed'] == min(budget, len(xs))
                    expected_raw = [feature(s, i + 1) for i, s in enumerate(xs[:budget]) if s[0] == d]
                    assert state['raw'] == list(reversed(expected_raw))
                counts['second_budget_cases'] += 1
            counts['second_prefixes'] += 1
    cases = []
    def record(name, actual, expected):
        assert actual == expected, (name, actual, expected)
        cases.append(dict(name=name, actual=actual, expected=expected))
    record('P2 反向尾笔：投影 OPEN、原始 KILLED',
           [projected(1, (2, 10), []), raw(1, (2, 10), [(1, 2, 12)])], [OPEN, KILLED])
    record('反向尾笔相等 P 不破', raw(1, (2, 10), [(1, 2, 10)]), OPEN)
    record('第一种同笔双越界优先 kill', raw(1, (2, 10), [(-1, 12, 1)]), KILLED)
    record('反向尾笔不能确认（非连续负控）', raw(1, (2, 10), [(1, 1, 9)]), OPEN)
    record('原始先越 P 后候选确认不可复活', raw(1, (2, 10), [(1, 2, 12), (-1, 12, 1)]), KILLED)
    record('第一种确认前缀不被新尾推翻', raw(1, (2, 10), [(1, 2, 10), (-1, 10, 1), (1, 1, 12)]), CONFIRMED)
    priority = [(1, 1, 4), (-1, 4, 0), (1, 0, 3), (-1, 3, 2), (1, 2, 11)]
    record('第二种同笔分型和越界：确认优先', run(reverse_step, initial(1), 1, 10, priority)['decision'], CONFIRMED)
    masked = run(reverse_step, initial(1), 1, 10, [(1, 2, 8), (-1, 8, 1), (1, 1, 12)])
    record('包含掩盖 high 后仍用原始极值 kill',
           [masked['decision'], masked['watermark'], masked['standard'][0][1], list(masked['standard'][0][3])],
           [KILLED, 12, 8, [1, 3]])
    record('第二种相等 P 不破', run(reverse_step, initial(1), 1, 10, [(1, 2, 10)])['decision'], None)
    record('末笔修订不是追加前缀：旧 OPEN 新 KILLED',
           [finish(run(reverse_step, initial(1), 1, 10, [(1, 2, v)]), 1, 1)[0] for v in (9, 12)],
           [OPEN, KILLED])
    xs = [(1, 2, 8), (-1, 8, 3)]
    record('预算不足不等 OPEN', list(finish(run(reverse_step, initial(1), 1, 10, xs[:1]), 2, 1)),
           [None, 'INCOMPLETE_RESOURCE_BOUND'])
    record('反向笔占预算且完全覆盖后 OPEN', list(finish(run(reverse_step, initial(1), 1, 10, xs), 2, 2)),
           [OPEN, 'COMPLETE'])
    result = dict(status='PASS_SCOPED_MATH', counts=counts, cases=cases,
                  limitation='纯 Python 整数/列表模型；不是 Lean 求值、Rust 执行、完整精化或独立 oracle 恢复。')
    (OUT / 'MODEL-RESULTS.json').write_text(json.dumps(result, ensure_ascii=False, indent=2) + '\n')
    print(json.dumps(dict(status=result['status'], counts=counts, named_cases=len(cases)), ensure_ascii=False))


if __name__ == '__main__':
    main()
