"""Stage66 independent finite audit; reads one frozen history, emits no new history.

R_W-v1 is reimplemented from its frozen definition, not imported from author code.
All arithmetic results are calculated before comparing author outputs.
"""
from pathlib import Path
from fractions import Fraction
from collections import Counter
import hashlib
import json
import math
import re
import time

OUT = Path(__file__).resolve().parent
AUTHOR = OUT.parent / 'nested-author'
ROOT = Path('/Users/silencehan/.codex/worktrees/order-event-base-research/NewChanlun')

def read(p):
    return json.loads(p.read_text())

def put(name, data):
    (OUT / name).write_text(json.dumps(data, ensure_ascii=False, indent=2) + '\n')

def sha(p):
    return hashlib.sha256(p.read_bytes()).hexdigest()

manifest = read(AUTHOR / 'manifest.json')
inputs = [AUTHOR / x['path'] for x in manifest['files']]
inputs += [AUTHOR / 'manifest.json', AUTHOR / 'FINAL']
inputs += [Path(x['path']) for x in read(AUTHOR / 'sourcehash.json')]
inputs += [ROOT / '.chanlun/review-results/issue1467-f1-proof/strict_point_reference.rs',
           ROOT / '.agents/skills/rigorous-open-math-research/SKILL.md']
inputs = sorted(set(inputs))

def inventory():
    return [{'path': str(p), 'bytes': p.stat().st_size, 'sha256': sha(p)} for p in inputs]

before = inventory()
put('inputhash-before.json', before)
freeze_errors = []
expected = {'Report.md': 'b345bff754af332b75b4582753a1f820e2d4d7dd17c35cbef07423a40c9bb97a',
            'manifest.json': '6ecad7097243ef76892769621542efc724a3ee3e02049febe59160592ead9099'}
for p, h in expected.items():
    if sha(AUTHOR / p) != h:
        freeze_errors.append(p)
for x in manifest['files']:
    p = AUTHOR / x['path']
    if sha(p) != x['sha256'] or p.stat().st_size != x['bytes']:
        freeze_errors.append(x['path'])
for x in read(AUTHOR / 'sourcehash.json'):
    if sha(Path(x['path'])) != x['sha256']:
        freeze_errors.append(x['path'])
assert not freeze_errors, freeze_errors

started = time.monotonic()
source = read(AUTHOR / 'run/source.json')
bid, ask = source['bid'], source['ask']
aq = int(source['initial']['ask_quantity'])
bq = int(source['initial']['bid_quantity'])
assert aq > 0 and bq > 0
assert aq == math.lcm(*range(1, 961)) == int(source['ask_quantity'])
prices = []
order_audit = []
for i in range(len(source['events']) + 1):
    if i:
        e = source['events'][i - 1]
        amount = int(e['amount'])
        assert e['id'] == e['time'] == i
        assert e['side'] == 'bid' and e['price'] == bid
        assert int(e['quantity_before']) == bq and amount > 0
        if e['action'] == 'cancel':
            assert amount < bq
            bq -= amount
        else:
            assert e['action'] == 'add'
            bq += amount
        assert int(e['quantity_after']) == bq
    value = Fraction(ask * bq + bid * aq, bq + aq)
    assert value.denominator == 1 and bid < value < ask
    p = int(value)
    prices.append(p)
    obs = source['observations'][i]
    assert obs['index'] == obs['time'] == i
    assert obs['price'] == p and obs['trade_volume'] == 0 and obs['untradable']
    assert int(obs['bid_quantity']) == bq and int(obs['ask_quantity']) == aq
    assert p % 50 == 0
    order_audit.append({'index': i, 'price': p, 'bid_quantity': str(bq),
                        'quantity_positive': True, 'exact_weighted_price': str(value)})
assert len(prices) == 443 and len(source['events']) == 442
card = read(AUTHOR / 'card.json')
expected_prices = [card['left_support'], card['initial_anchor']]
for a, b in zip(card['all_endpoints'], card['all_endpoints'][1:]):
    expected_prices.extend(Fraction(a) + Fraction(b - a) * j / 4 for j in range(1, 5))
expected_prices.append(card['right_support'])
assert prices == expected_prices
put('order-replay.json', order_audit)

def rw(values):
    groups = []
    for i, value in enumerate(values):
        if groups and groups[-1]['price'] == value:
            groups[-1]['last'] = i
        else:
            groups.append({'first': i, 'last': i, 'price': value})
    anchors = []
    for j in range(1, len(groups) - 1):
        a, b, c = groups[j - 1:j + 2]
        top = b['price'] > a['price'] and b['price'] > c['price']
        low = b['price'] < a['price'] and b['price'] < c['price']
        if not (top or low):
            continue
        f = {'top': top, 'raw': b['first'], 'merged': j,
             'price': b['price'], 'known_at': c['first']}
        if not anchors:
            anchors.append(f)
        elif anchors[-1]['top'] == top:
            prev = anchors[-1]
            extends = f['price'] > prev['price'] if top else f['price'] < prev['price']
            if extends:
                anchors[-1] = f
        else:
            prev = anchors[-1]
            strict_order = prev['price'] > f['price'] if prev['top'] else f['price'] > prev['price']
            if j >= prev['merged'] + 3 and f['raw'] > prev['raw'] + 3 and strict_order:
                anchors.append(f)
    strokes = [{'up': not a['top'], 'start': a['raw'], 'end': b['raw'],
                'start_price': a['price'], 'end_price': b['price']}
               for a, b in zip(anchors, anchors[1:])]
    return {'groups': len(groups), 'anchors': anchors, 'strokes': strokes}

prefixes = []
for cut in range(len(prices)):
    r = rw(prices[:cut + 1])
    prefixes.append({'cut': cut, 'stable_count': max(len(r['strokes']) - 1, 0),
                     'stable': r['strokes'][:-1],
                     'active': r['strokes'][-1] if r['strokes'] else None,
                     'anchors': [{k: v for k, v in a.items() if k != 'merged'} for a in r['anchors']]})
whole = rw(prices)
pens = whole['strokes']
assert len(pens) == 110 and len(prefixes[-1]['stable']) == 109
assert [pens[0]['start_price']] + [p['end_price'] for p in pens] == card['all_endpoints']
assert all(x['stable'] == pens[:x['stable_count']] for x in prefixes)
put('independent-prefixes.json', prefixes)
put('independent-pens.json', pens)

def interval(p):
    return sorted([p['start_price'], p['end_price']])

def inter(ranges):
    return [max(x[0] for x in ranges), min(x[1] for x in ranges)]

def whole_range(start, end):
    return [min(prices[start:end + 1]), max(prices[start:end + 1])]

def inclusive(a, b):
    return a[0] <= b[0] and b[1] <= a[1]

def narrow_witness(stable, segment):
    begin = segment * 5
    members = stable[begin:begin + 5]
    if len(members) != 5:
        return None, 'insufficient stable members'
    up = members[0]['up']
    features = [(n, interval(p)) for n, p in enumerate(stable) if n >= begin and p['up'] != up]
    chosen = None
    for j in range(1, len(features)):
        ar, br = features[j - 1][1], features[j][1]
        if inclusive(ar, br) or inclusive(br, ar):
            return None, {'containment': [features[j - 1][0], features[j][0]]}
        if j < 2:
            continue
        left, mid, right = [v[1] for v in features[j - 2:j + 1]]
        dominant = mid[1] > max(left[1], right[1]) if up else mid[0] < min(left[0], right[0])
        if dominant:
            chosen = features[j - 2:j + 1]
            break
    if chosen is None:
        return None, 'no selected fractal'
    left, mid, right = [x[1] for x in chosen]
    second_axis = mid[0] > max(left[0], right[0]) if up else mid[1] < min(left[1], right[1])
    no_gap = inter([left, mid])[0] < inter([left, mid])[1]
    initial = inter([interval(p) for p in members[:3]])
    start, end = members[0]['start'], members[-1]['end']
    ends_are_extremes = whole_range(start, end) == sorted([prices[start], prices[end]])
    terminal_matches = prices[end] == (mid[1] if up else mid[0])
    if not (second_axis and no_gap and initial[0] < initial[1] and ends_are_extremes and terminal_matches):
        return None, 'geometric precondition false'
    return {'id': f'S{segment}', 'up': up, 'start': start, 'end': end,
            'own': [start + 1, end], 'members': [begin, begin + 5],
            'range': whole_range(start, end), 'first_selected': [x[0] for x in chosen],
            'selected_ranges': [x[1] for x in chosen], 'gap': False,
            'initial_three_core': initial}, None

segments = []
pending = []
for s in range(22):
    witnesses = [narrow_witness(p['stable'], s)[0] for p in prefixes]
    first = next((i for i, v in enumerate(witnesses) if v), None)
    if first is None:
        pending.append({'segment': s, 'eof_reason': narrow_witness(prefixes[-1]['stable'], s)[1]})
    else:
        assert all(v == witnesses[first] for v in witnesses[first:])
        segments.append({**witnesses[first], 'known_at': first, 'persists_all_later_prefixes': True})
assert len(segments) == 20 and [p['segment'] for p in pending] == [15, 21]
put('independent-narrow-segments.json', {'certificates': segments, 'pending': pending})

# Direct source certificate, distinct from the restrictive no-containment helper.
old = pens[75:80]
new = pens[80:83]
assert [p['up'] for p in old] == [True, False, True, False, True]
assert [p['up'] for p in new] == [False, True, False]
d = [old[0]['start_price'], old[1]['end_price'], old[3]['end_price']]
g = [old[0]['end_price'], old[2]['end_price'], old[4]['end_price']]
old_breaks = [(i + 1, j + 1) for i in range(len(g)) for j in range(len(d))
              if j >= i + 2 and d[j] <= g[i]]
assert old_breaks == []
new_d = d + [new[0]['end_price']]
new_breaks = [(i + 1, j + 1) for i in range(len(g)) for j in range(len(new_d))
              if j >= i + 2 and new_d[j] <= g[i]]
assert new_breaks == [(2, 4)]
initial_old = inter([interval(p) for p in old[:3]])
initial_new = inter([interval(p) for p in new])
assert initial_old[0] < initial_old[1] and initial_new[0] < initial_new[1]
assert new[-1]['end_price'] < new[0]['end_price']
assert new[1]['end_price'] < new[0]['start_price']
cert_cuts = [p['cut'] for p in prefixes if p['stable'][75:83] == pens[75:83]]
assert cert_cuts == list(range(338, 443))
prior = next(s for s in segments if s['id'] == 'S14')
assert prior['end'] == old[0]['start'] and prior['known_at'] <= 338
s15 = {'id': 'S15', 'up': True, 'start': 301, 'end': 321, 'own': [302, 321],
       'members': [75, 80], 'range': whole_range(301, 321), 'known_at': 338,
       'prior_S14_known_at': prior['known_at'], 'old_first_three_overlap': initial_old,
       'old_breaks': old_breaks, 'new_first_breaks': new_breaks,
       'new_first_three_overlap': initial_new, 'd': d, 'g': g,
       'new_third_breaks_first_end': True, 'new_second_does_not_rebreak_turn': True,
       'all_later_finite_prefixes': True, 'scope': 'selected source071 boundary, no full normalizer'}
put('independent-S15.json', s15)
seg_by_id = {s['id']: s for s in segments + [s15]}

def force(first, stop):
    a, b = pens[first], pens[stop - 1]
    va = Fraction(a['end_price'] - a['start_price'], a['end'] - a['start'])
    vb = Fraction(b['end_price'] - b['start_price'], b['end'] - b['start'])
    assert va.denominator == vb.denominator == 1
    return {'first_pen': first, 'last_pen': stop - 1, 'first_v': int(va),
            'last_v': int(vb), 'L': int(vb - va)}

objects = []
for index, name in enumerate(['P0', 'P1', 'P2', 'c0']):
    first_s = 1 + index * 5
    ss = [seg_by_id[f'S{s}'] for s in range(first_s, first_s + 5)]
    a, z = ss[0]['start'], ss[-1]['end']
    bf, cf = force(first_s * 5, first_s * 5 + 5), force((first_s + 4) * 5, (first_s + 5) * 5)
    up = pens[first_s * 5]['up']
    core_members = ss[1:4]
    core = inter([s['range'] for s in core_members])
    assert core[0] < core[1]
    assert all(x['up'] != y['up'] for x, y in zip(ss, ss[1:]))
    prior_extreme = (max if up else min)(prices[a:ss[-1]['start'] + 1])
    extreme = prices[z] > prior_extreme if up else prices[z] < prior_extreme
    assert cf['L'] < bf['L'] and extreme
    objects.append({'id': name, 'direction': 'Up' if up else 'Down', 'rank': 0,
                    'start': a, 'end': z, 'own': [a + 1, z], 'whole': whole_range(a, z),
                    'segments': [s['id'] for s in ss], 'core': core,
                    'core_directions': ['Up' if s['up'] else 'Down' for s in core_members],
                    'core_happened': core_members[-1]['end'],
                    'core_known': max(s['known_at'] for s in core_members),
                    'b_force': bf, 'c_force': cf, 'prior_extreme': prior_extreme,
                    'end_price': prices[z], 'local_predicate': True,
                    'happened_at': z, 'local_known_at': max(s['known_at'] for s in ss),
                    'original_completed': None})
put('independent-objects.json', objects)
b = {'id': 'b_low', 'start': 1, 'end': 21, 'own': [2, 21],
     'whole': whole_range(1, 21), 'force': force(0, 5), 'raw_known_at': seg_by_id['S0']['known_at']}
kernel = inter([x['whole'] for x in objects[:3]])
parent_force = force(80, 105)
parent = {'whole': whole_range(1, 421), 'own': [2, 421], 'core': kernel,
          'member_directions': [x['direction'] for x in objects[:3]],
          'b': b, 'c': objects[3], 'b_force': b['force'], 'c_force': parent_force,
          'prior_min': min(prices[1:322]), 'end_price': prices[421],
          'local_happened_at': 421, 'local_known_at': 438, 'original_known_at': None,
          'original_published_at': None, 'original_completed': None}
assert parent_force['L'] == -3500 < b['force']['L'] == -1000
assert prices[421] == 37800 < min(prices[1:322]) == 40000
assert kernel == [62000, 72000]
put('independent-parent.json', parent)

partition = {'support': [1, 1], 'b': [2, 21], 'P0': [22, 121], 'P1': [122, 221],
             'P2': [222, 321], 'c0': [322, 421], 'tail': [422, 442]}
counts = Counter(i for a, z in partition.values() for i in range(a, z + 1))
assert counts == Counter(range(1, 443))
dup = sorted(set(range(22, 422)) & set(range(322, 422)))
promotion = inter([b['whole']] + [x['whole'] for x in objects[:2]])
relations = {'partition': partition, 'duplicates': 0, 'missing': 0,
             'D1_outer_adj': [['b_low', 'K1'], ['K1', 'c0']],
             'MemberNext': [['P0', 'P1'], ['P1', 'P2']], 'canonical_adj': None,
             'D0_conditional_pair': ['P0', 'P1'], 'D0_local_joint_known': 238,
             'promotion': {'core': promotion, 'directions': ['Down', 'Up', 'Down'],
                           'time_contiguous': True, 'candidate_joint_known': 238,
                           'source_complete_members': 'unestablished',
                           'first_among_selected_carrier_triples_only': True},
             'full_extension': {'K1_seed_direction': 'Up', 'Zn_among_selected': ['P0', 'P2'],
                                'c0_direction': 'Down', 'c0_is_Zn': False,
                                'c0_core_intersection': inter([objects[3]['whole'], kernel]),
                                'seed_Zn_outer': {'GG': 72800, 'DD': 40000},
                                'extension_original_membership': 'unestablished',
                                'if_full_center_own_is_22_421_duplicate_events': len(dup),
                                'conditional_duplicates': [dup[0], dup[-1]],
                                'whole_hull_not_proved_Zn_outer': [37800, 72800]}}
assert len(dup) == 100 and promotion == [62000, 72000]
put('independent-relations.json', relations)

# Only now consult author scanner outputs as comparison data, not as an oracle.
author_ref = read(AUTHOR / 'run/reference.stdout')
comparison = {'pens_equal': pens == author_ref['strokes'],
              'prefixes_equal': prefixes == author_ref['prefixes'],
              'narrow_certificates_equal': segments == read(AUTHOR / 'run/segments.json')}
assert all(comparison.values()), comparison
authors = read(AUTHOR / 'source-supplement/objects-v2.json')
for ours, theirs in zip(objects, authors):
    assert ours['whole'] == theirs['whole'] and ours['own'] == theirs['own']
    assert ours['b_force'] == theirs['general_div']['b_force']
    assert ours['c_force'] == theirs['general_div']['c_force']
    assert ours['local_known_at'] == theirs['completion']['known_at']
put('comparison-after-independent-computation.json', comparison)

source_checks = []
ev = read(AUTHOR / 'source-evidence.json')
for x in ev['blog']:
    p = ROOT / x['path']
    lines = p.read_text().splitlines()
    raw = lines[x['line'] - 1]
    boundary = next((i + 1 for i, s in enumerate(lines) if '↑正文' in s), None)
    head_note = bool(re.match(r'^\s*[（(]?\s*(?:娇注|娇|注)\s*[：:]', raw))
    annotations = re.findall(r'[（(]\s*(?:娇注|娇|注)\s*[：:][^）)]*[）)]', raw)
    row = {'path': x['path'], 'line': x['line'], 'raw': raw, 'body_boundary': boundary,
           'line_exact': raw == x['raw'], 'hash_exact': sha(p) == x['file_sha256'],
           'before_boundary': boundary is not None and x['line'] < boundary,
           'head_note': head_note, 'inline_annotations': annotations,
           'author_accepts': x['author_body']}
    assert row['line_exact'] and row['hash_exact']
    if x['author_body']:
        assert row['before_boundary'] and not head_note
        # Existing annotations must not appear inside the author's accepted text.
        assert all(a not in x['accepted_text'] for a in annotations)
    source_checks.append(row)
put('source-line-audit.json', source_checks)
doctrine_checks = []
for x in ev['doctrine']:
    p = ROOT / x['path']
    lines = p.read_text().splitlines()
    actual = '\n'.join(lines[x['start'] - 1:x['end']])
    item = {'path': x['path'], 'start': x['start'], 'end': x['end'],
            'hash_exact': sha(p) == x['sha256'], 'text_exact': actual == x['text']}
    assert item['hash_exact'] and item['text_exact']
    doctrine_checks.append(item)
put('doctrine-extract-audit.json', doctrine_checks)

# Evidence guard is not the truth value of the semantic proposition.
carrier_b = read(AUTHOR / 'source-supplement/actual-relations-v2.json')['raw_initial_carrier'][0]
hb = {'object': 'b_low', 'actual_tag': carrier_b['semantic_kind'],
      'required_tag_in_author_interface': 'completed-original-motion',
      'semantic_GeneralDiv': carrier_b['original_general_div'],
      'semantic_Completed': carrier_b['original_completed'],
      'evidence_tag_matches': carrier_b['semantic_kind'] == 'completed-original-motion',
      'evidence_Completed_present_true': carrier_b['original_completed'] is True,
      'source_requires_this_literal_tag': False,
      'source_084_initial_layer_admission_theorem': None,
      'null_implies_semantic_false': False,
      'all_legal_b_absent': None}
hb['reviewer_evaluated_author_evidence_guard'] = hb['evidence_tag_matches'] and hb['evidence_Completed_present_true']
hb['author_executable_guard_found_in_two_scripts'] = any(
    'H_b' in (AUTHOR / name).read_text() or 'typed_gate_value' in (AUTHOR / name).read_text()
    for name in ['build_check.mjs', 'source-supplement.mjs'])
assert hb['semantic_GeneralDiv'] is None and hb['semantic_Completed'] is None
assert not hb['reviewer_evaluated_author_evidence_guard']
put('H_b-evidence-versus-semantics.json', hb)

after = inventory()
put('inputhash-after.json', after)
assert before == after
summary = {'freeze_pass': True, 'manifest_entries': len(manifest['files']),
           'raw_history_count': 1, 'observations': len(prices), 'events': len(source['events']),
           'prefixes': len(prefixes), 'pens': len(pens), 'eof_stable': 109,
           'narrow_certificates': 20, 'selected_071_certificate': 1,
           'pending_tail_slot': 21, 'S15_known': 338,
           'objects_local_known': {o['id']: o['local_known_at'] for o in objects},
           'parent_force': [b['force']['L'], parent_force['L']], 'parent_local_known': 438,
           'original_completed_certified': 0, 'source_lines_checked': len(source_checks),
           'comparison': comparison, 'source_extension_claim_accepted': False,
           'inputhash_equal': True, 'elapsed_seconds': round(time.monotonic() - started, 3)}
put('recompute-result.json', summary)
print(json.dumps(summary, ensure_ascii=False, indent=2))
