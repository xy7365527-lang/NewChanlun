#!/usr/bin/env python3
"""Stage69 independent read-only audit, Python stdlib only; exactly two refits.

No author module is imported or invoked as a numeric oracle. Author stdin is
tested separately with invalid requests only (zero successful fits). All writes
are restricted to this review directory. Numeric sums use math.fsum, deliberately
not the author's left-to-right JS accumulation order.
"""
import collections
import copy
import datetime
import hashlib
import json
import math
import pathlib
import re
import subprocess
import time

OUT = pathlib.Path(__file__).resolve().parent
AUTHOR = OUT.parent / 'learning-author'
RUN = AUTHOR / 'run-v1'
E = OUT.parents[1]
CUT = 1609459220000000000
END_MIDDLE = 1609459240000000000
VERSION = 'stage69-softmax-finite-features/1'
CLASSES = ['up', 'down', 'none']
FIELDS = {
    'A': ['last_trade_price', 'seen_trade_count', 'since_last_trade_ns',
          'observed_trade_count_1s', 'observed_trade_quantity_1s'],
}
FIELDS['B'] = FIELDS['A'] + ['bid_price', 'ask_price', 'bid_quantity',
    'ask_quantity', 'spread', 'queue_imbalance', 'published_l1_changes_1s',
    'published_mid2_delta_1s']
SCALES = {'last_trade_price': 10**8, 'seen_trade_count': 1,
    'since_last_trade_ns': 10**9, 'observed_trade_count_1s': 1,
    'observed_trade_quantity_1s': 10**8, 'bid_price': 10**8,
    'ask_price': 10**8, 'bid_quantity': 10**8, 'ask_quantity': 10**8,
    'spread': 10**8, 'published_l1_changes_1s': 1,
    'published_mid2_delta_1s': 2*10**8}


def sha(data):
    return hashlib.sha256(data).hexdigest()


def read(path):
    return json.loads(path.read_text())


def lines(path):
    return [json.loads(line) for line in path.read_text().splitlines()]


def write(name, value):
    (OUT / name).write_text(json.dumps(value, indent=2, ensure_ascii=False,
                                      allow_nan=False) + '\n')


def snapshot(paths):
    return [{'path': str(p), 'bytes': len(p.read_bytes()),
             'sha256': sha(p.read_bytes())} for p in paths]


def compact(value):
    # Exact match to JS JSON.stringify for the string/integer-only fit requests.
    return json.dumps(value, separators=(',', ':'), ensure_ascii=False).encode()


def integer(value):
    assert isinstance(value, str) and re.fullmatch(r'-?(0|[1-9][0-9]*)', value)
    return int(value)


def scalar(value, field):
    if value is None:
        return None
    if field == 'queue_imbalance':
        assert set(value) == {'numerator', 'denominator'}
        num, den = integer(value['numerator']), integer(value['denominator'])
        assert den > 0 and -den <= num <= den
        result = float(num) / float(den)
    else:
        num = integer(value)
        assert field == 'published_mid2_delta_1s' or num >= 0
        result = float(num) / SCALES[field]
    assert math.isfinite(result)
    return result


def qualifies(row):
    return (integer(row['decision_ns']) < CUT and
            row['disposition'] in ('observed_change', 'observed_no_change') and
            row['label'] in CLASSES and row['label_known_ns'] is not None and
            integer(row['label_known_ns']) < CUT)


def train_projection(rows, evaluations, arm):
    return {'op': 'fit', 'version': VERSION, 'arm': arm,
        'fit_cutoff_ns': str(CUT), 'rows': [
            {'product_id': r['product_id'], 'decision_ns': r['decision_ns'],
             'features': {k: r['features'][k] for k in FIELDS[arm]},
             'label': e['label']}
            for r, e in zip(rows, evaluations) if qualifies(e)]}


def preparation(raw, fields):
    result = []
    for j, field in enumerate(fields):
        values = [row[j] for row in raw if row[j] is not None]
        mean = math.fsum(values)/len(values) if values else 0.0
        variance = math.fsum((row[j]-mean)**2 if row[j] is not None else 0.0
                             for row in raw)/len(raw)
        std = math.sqrt(variance)
        result.append({'name': field, 'observed': len(values),
            'missing': len(raw)-len(values), 'mean': mean, 'variance': variance,
            'raw_std': std, 'scale': std if std else 1.0})
    return result


def design(raw, prep):
    return [1.0] + [0.0 if v is None else (v-p['mean'])/p['scale']
                    for v, p in zip(raw, prep)] + [float(v is None) for v in raw]


def probabilities(weights, x):
    logits = [math.fsum(w*v for w, v in zip(row, x)) for row in weights]
    exps = [math.exp(z-max(logits)) for z in logits]
    den = math.fsum(exps)
    return [v/den for v in exps]


def independently_fit(request):
    fields = FIELDS[request['arm']]
    raw = [[scalar(r['features'][k], k) for k in fields] for r in request['rows']]
    prep = preparation(raw, fields)
    xs = [design(r, prep) for r in raw]
    ys = [CLASSES.index(r['label']) for r in request['rows']]
    n, d = len(xs), len(xs[0])
    weights = [[0.0]*d for _ in CLASSES]
    for _ in range(200):
        residuals = [[p-float(k == y) for k, p in enumerate(probabilities(weights, x))]
                     for x, y in zip(xs, ys)]
        # Column summation and one division differs from author's per-row /n.
        gradient = [[math.fsum(res[k]*x[j] for res, x in zip(residuals, xs))/n
                     + (0.001*weights[k][j] if j else 0.0)
                     for j in range(d)] for k in range(3)]
        weights = [[w-0.05*g for w, g in zip(wr, gr)]
                   for wr, gr in zip(weights, gradient)]
    return prep, weights


def max_difference(a, b):
    return max(abs(x-y) for x, y in zip(a, b))


def metric(rows):
    n = len(rows)
    return {'rows': n,
        'accuracy': sum(p.index(max(p)) == CLASSES.index(y) for p, y in rows)/n,
        'mean_logloss': math.fsum(-math.log(max(p[CLASSES.index(y)], 1e-15))
                                 for p, y in rows)/n,
        'multiclass_brier': math.fsum(math.fsum((q-float(k == CLASSES.index(y)))**2
                                              for k, q in enumerate(p))
                                     for p, y in rows)/n}


def main():
    started = datetime.datetime.now(datetime.timezone.utc).isoformat()
    expected_pins = {'manifest.json': '903a7e367e4e55c16ecb6709b17e41316b5edf237308eb2eb2ea3fc237b4ed7c',
                     'FINAL.md': '2b304dcf42f295507b5c26918a520af592d522341ce23b01146ef3172fefe15a'}
    for name, pin in expected_pins.items():
        assert sha((AUTHOR/name).read_bytes()) == pin
    sources = read(AUTHOR/'input-hashes-before.json')['inputs']
    paths = sorted(p for p in AUTHOR.rglob('*') if p.is_file()) + [pathlib.Path(f['path']) for f in sources]
    before = snapshot(paths)
    write('input-hashes-before.json', before)
    for f in sources:
        assert pathlib.Path(f['path']).stat().st_size == f['bytes']
        assert sha(pathlib.Path(f['path']).read_bytes()) == f['sha256']
    for base in (AUTHOR, RUN):
        manifest = read(base/'manifest.json')
        for f in manifest['files']:
            p = base/f['name']
            assert p.stat().st_size == f['bytes'] and sha(p.read_bytes()) == f['sha256']
    for f in read(RUN/'source-freeze.json')['files']:
        assert sha((AUTHOR/f['name']).read_bytes()) == f['sha256']
    assert sha((AUTHOR/'WorkCard.md').read_bytes()) == (AUTHOR/'WorkCard.md.sha256').read_text().strip()
    assert read(RUN/'input-hashes-after.json')['inputs'] == sources

    data = {'A': lines(E/'trade-grid-a-v1/features.jsonl'),
            'B': lines(E/'book-grid-b-v1/features.jsonl')}
    labels = lines(E/'label-grid-v2/labels.jsonl')
    evaluation = read(E/'stage56/consumer-author/run-v4/evaluation-only.json')
    stage56 = read(E/'stage56/consumer-author/run-v4/receipt.json')
    assert stage56['partitions'][0]['end_ns'] == str(CUT)
    assert stage56['partitions'][1]['end_ns'] == str(END_MIDDLE)
    assert next(f['sha256'] for f in stage56['outputs'] if f['name'] == 'evaluation-only.json') == sources[3]['sha256']
    for rows in [*data.values(), labels, evaluation]:
        assert len(rows) == 572
    for i, (a, b, label, ev) in enumerate(zip(data['A'], data['B'], labels, evaluation)):
        for row in (a, b, label, ev):
            assert row['product_id'] == 'BTC-USD' and row['decision_ns'] == a['decision_ns']
        assert a['profile'] == 'trade-history-grid-a/1'
        assert b['profile'] == 'book-history-grid-b/1'
        assert label['profile'] == 'published-mid-first-change/2'
        assert set(a['features']) == set(FIELDS['A']) and set(b['features']) == set(FIELDS['B'])
        assert a['features'] == {k: b['features'][k] for k in FIELDS['A']}
        assert ev['label'] == label['label'] and ev['disposition'] == label['disposition']
        assert ev['label_event_ns'] == label['resolved_at_ns']
        t = integer(a['decision_ns'])
        assert ev['partition'] == ('early' if t < CUT else 'middle' if t < END_MIDDLE else 'late')
        if i:
            assert t-integer(data['A'][i-1]['decision_ns']) == 100000000
        if ev['label_known_ns'] is not None:
            assert integer(ev['label_known_ns']) > integer(ev['label_event_ns']) >= t
        if ev['disposition'] == 'ineligible_at_decision':
            assert ev['label'] is None and ev['label_known_ns'] is None and not ev['loss_eligible']

    eligible = [i for i, row in enumerate(evaluation) if qualifies(row)]
    assert len(eligible) == 169 and eligible == list(range(3, 172))
    assert collections.Counter(evaluation[i]['label'] for i in eligible) == {'up': 127, 'down': 42}
    host_eligibility = lines(RUN/'host-training-eligibility.jsonl')
    for e, h in zip(evaluation, host_eligibility):
        for key in ['product_id', 'decision_ns', 'label', 'disposition', 'label_event_ns', 'label_known_ns']:
            assert h[key] == e[key]
        assert h['used_for_fit'] == qualifies(e)
        assert h['decision_before_cutoff'] == (integer(e['decision_ns']) < CUT)
        assert h['known_strict_before_cutoff'] == (e['label_known_ns'] is not None and integer(e['label_known_ns']) < CUT)
    boundary = copy.deepcopy(evaluation[171])
    boundary['label_known_ns'] = str(CUT-1)
    boundary_results = []
    for value, expected in [(CUT-1, True), (CUT, False), (CUT+1, False)]:
        q = {**boundary, 'label_known_ns': str(value)}
        assert qualifies(q) == expected
        boundary_results.append({'known': str(value), 'eligible': qualifies(q)})
    assert not qualifies({**boundary, 'label': None, 'label_known_ns': None,
                          'disposition': 'ineligible_at_decision'})
    assert not qualifies({**boundary, 'decision_ns': str(CUT)})

    # Reproduce exactly the author's future transformation, then verify its
    # premise explicitly: every eligible training field is unchanged.
    changed = copy.deepcopy(data)
    changed_ev = copy.deepcopy(evaluation)
    for i in range(572):
        if integer(data['A'][i]['decision_ns']) >= CUT:
            for arm in ['A', 'B']:
                changed[arm][i]['features'] = {k: {'numerator': '0', 'denominator': '1'}
                    if k == 'queue_imbalance' else '7' for k in FIELDS[arm]}
            changed_ev[i].update(label='down' if evaluation[i]['label'] == 'up' else 'up',
                label_known_ns=str(CUT+1), partition='transformed-control', loss_eligible=False)
    for i in eligible:
        assert changed_ev[i] == evaluation[i]
        for arm in ['A', 'B']:
            assert changed[arm][i] == data[arm][i]

    receipt = read(RUN/'receipt.json')
    process_receipts = read(RUN/'process-receipts.json')
    diagnostics = read(RUN/'diagnostics.json')
    joined = lines(RUN/'host-evaluation-after-predictions.jsonl')
    assert len(joined) == 800
    assert diagnostics['evaluation_join_at'] >= max(p['finished_at'] for p in process_receipts)
    assert process_receipts[0]['finished_at'] <= process_receipts[1]['started_at']
    result = {'status': 'passed', 'started_at': started,
        'method': 'independent Python stdlib, fsum reductions; no author math imports',
        'refit_count': 2, 'market_decisions': 572, 'train_rows_per_arm': 169,
        'classes': CLASSES, 'class_counts': [127, 42, 0], 'known_cutoff_controls': boundary_results,
        'tolerance': {'weights_abs': 1e-9, 'probabilities_abs': 1e-9,
                      'metrics_abs': 1e-9, 'preprocessing_relative': 1e-11},
        'arms': {}}
    model_hash_jobs = []
    total_rejections = 0
    for arm in ['A', 'B']:
        queries = lines(RUN/f'{arm}-stdin.jsonl')
        answers = lines(RUN/f'{arm}-stdout.jsonl')
        events = lines(RUN/f'{arm}-events.jsonl')
        stderr = lines(RUN/f'{arm}-stderr.jsonl')
        original_in_lines = (RUN/f'{arm}-stdin.jsonl').read_bytes().splitlines()
        original_out_lines = (RUN/f'{arm}-stdout.jsonl').read_bytes().splitlines()
        process = next(p for p in process_receipts if p['arm'] == arm)
        assert len(queries) == len(answers) == len(stderr)
        assert len(events) == 2*len(queries)
        assert process['code'] == 0 and process['signal'] is None
        assert process['command'][1:] == ['--max-old-space-size=64', str(AUTHOR/'learner-stdin.mjs')]
        assert process['input_sha256'] == sha((RUN/f'{arm}-stdin.jsonl').read_bytes())
        assert process['output_sha256'] == sha((RUN/f'{arm}-stdout.jsonl').read_bytes())
        for i, (qin, aout) in enumerate(zip(original_in_lines, original_out_lines)):
            ein, eout = events[2*i:2*i+2]
            assert ein['direction'] == 'in' and eout['direction'] == 'out'
            assert ein['sha256'] == eout['for_input_sha256'] == sha(qin)
            assert eout['sha256'] == sha(aout)
            assert ein['pid'] == eout['pid'] == stderr[i]['pid'] == process['pid']
            assert process['started_at'] <= ein['wall_at'] <= eout['wall_at'] <= process['finished_at']
            if i:
                assert events[2*i-1]['wall_at'] <= ein['wall_at']
            assert stderr[i]['operation'] == answers[i]['status']
        fit_indices = [i for i, a in enumerate(answers) if a['status'] == 'fitted']
        assert len(fit_indices) == 1
        fi = fit_indices[0]
        fit = queries[fi]
        wanted = train_projection(data[arm], evaluation, arm)
        assert fit == wanted  # Source row identity, not only batch self-consistency.
        assert original_in_lines[fi] == compact(wanted)
        assert wanted == train_projection(changed[arm], changed_ev, arm)
        model = read(RUN/f'{arm}-model.json')
        assert model == answers[fi]['model']
        assert model['training_request_sha256'] == sha(original_in_lines[fi])
        assert model['settings'] == {'steps': 200, 'learning_rate': 0.05, 'l2': 0.001}
        assert model['classes'] == CLASSES and model['feature_names'] == FIELDS[arm]
        assert model['train_rows'] == 169 and model['class_counts'] == [127, 42, 0]
        assert len(model['weights']) == 3
        assert sum(map(len, model['weights'])) == (33 if arm == 'A' else 81)
        tic = time.perf_counter()
        prep, weights = independently_fit(wanted)
        runtime = time.perf_counter()-tic
        prep_diff = {}
        for p, saved in zip(prep, model['preprocessing']):
            assert p['name'] == saved['name'] and p['observed'] == saved['observed'] and p['missing'] == saved['missing']
            for key in ['mean', 'variance', 'raw_std', 'scale']:
                d = abs(p[key]-saved[key])
                assert d <= 1e-11*max(1, abs(p[key]), abs(saved[key]))
                prep_diff[f'{p["name"]}.{key}'] = d
        wdiff = max_difference([x for row in weights for x in row],
                               [x for row in model['weights'] for x in row])
        assert wdiff <= 1e-9
        reproduced_model = {**model, 'preprocessing': prep, 'weights': weights}
        write(f'{arm}-independent-model.json', reproduced_model)
        model_hash_jobs.append({'arm': arm, 'original': model, 'independent': reproduced_model})
        saved_predictions = lines(RUN/f'{arm}-predictions.jsonl')
        assert len(saved_predictions) == 572
        success_pairs = [(q, a) for q, a in zip(queries, answers) if a['status'] == 'predicted']
        assert len(success_pairs) == 501
        assert [a for q, a in success_pairs[:400]] == saved_predictions[172:]
        assert [a for q, a in success_pairs[400:500]] == saved_predictions[172:272]
        inferred = []
        pred_diff = 0.0
        saved_weights_prob_diff = 0.0
        prediction_class_disagreements = 0
        for j, (q, answer) in enumerate(success_pairs):
            assert set(q) == {'op', 'version', 'product_id', 'decision_ns', 'features'}
            assert q['op'] == 'predict' and q['version'] == VERSION and q['product_id'] == 'BTC-USD'
            assert set(q['features']) == set(FIELDS[arm])
            assert integer(q['decision_ns']) >= CUT
            if j < 400:
                source = data[arm][172+j]
                assert q['decision_ns'] == source['decision_ns'] and q['features'] == source['features']
            elif j < 500:
                source = data[arm][172+j-400]
                assert q['decision_ns'] == source['decision_ns'] and q['features'] == source['features']
            else:
                assert all(v is None for v in q['features'].values())
            raw = [scalar(q['features'][k], k) for k in FIELDS[arm]]
            p = probabilities(weights, design(raw, prep))
            from_saved = probabilities(model['weights'], design(raw, model['preprocessing']))
            pred_diff = max(pred_diff, max_difference(p, answer['probabilities']))
            saved_weights_prob_diff = max(saved_weights_prob_diff,
                max_difference(from_saved, answer['probabilities']))
            assert all(math.isfinite(v) and 0 <= v <= 1 for v in answer['probabilities'])
            assert abs(math.fsum(answer['probabilities'])-1) <= 1e-12
            expected_class = CLASSES[p.index(max(p))]
            prediction_class_disagreements += expected_class != answer['prediction']
            assert answer['prediction'] == CLASSES[answer['probabilities'].index(max(answer['probabilities']))]
            if j < 400:
                inferred.append({'decision_ns': q['decision_ns'], 'probabilities': p,
                                 'prediction': expected_class})
        assert pred_diff <= 1e-9 and saved_weights_prob_diff <= 1e-12
        assert prediction_class_disagreements == 0
        for i, prediction in enumerate(saved_predictions[:172]):
            assert prediction == {'arm': arm, 'product_id': 'BTC-USD',
                'decision_ns': data[arm][i]['decision_ns'], 'status': 'not-yet-fitted',
                'probabilities': None}
        inspection = read(RUN/f'{arm}-frozen-after-inference.json')
        assert inspection['model'] == model and answers[-1] == inspection
        assert inspection['model_sha256'] == answers[fi]['model_sha256'] == receipt['model_sha256'][arm]
        rejects = [(q, a) for q, a in zip(queries, answers) if a['status'] == 'rejected']
        total_rejections += len(rejects)
        metrics = {}
        for part in ['middle', 'late']:
            rows = [(p['probabilities'], ev['label']) for p, ev in zip(inferred, evaluation[172:]) if ev['partition'] == part]
            assert len(rows) == 200
            assert all(ev['loss_eligible'] for ev in evaluation if ev['partition'] == part)
            m = metric(rows)
            for k, v in m.items():
                assert abs(v-diagnostics['metrics'][arm][part][k]) <= 1e-9
            metrics[part] = m
        for record in [r for r in joined if r['arm'] == arm]:
            i = next(i for i, e in enumerate(evaluation) if e['decision_ns'] == record['decision_ns'])
            assert record['evaluation'] == evaluation[i]
            assert {k: v for k, v in record.items() if k != 'evaluation'} == saved_predictions[i]
        write(f'{arm}-independent-predictions.json', inferred)
        result['arms'][arm] = {'pid': process['pid'], 'fit_log_index_zero_based': fi,
            'requests': len(queries), 'rejections': len(rejects),
            'rejection_errors': dict(collections.Counter(a['error'] for q, a in rejects)),
            'primary_predictions': 400, 'repeat_prefix_predictions': 100,
            'all_missing_predictions': 1, 'fit_compute_ns_logged': stderr[fi]['compute_ns'],
            'independent_refit_wall_seconds': runtime, 'weight_max_abs_diff': wdiff,
            'preprocessing_max_abs_diff': max(prep_diff.values()),
            'preprocessing_diffs': prep_diff, 'refit_probability_max_abs_diff': pred_diff,
            'saved_weight_probability_max_abs_diff': saved_weights_prob_diff,
            'prediction_class_disagreements': prediction_class_disagreements,
            'missing_counts': {p['name']: p['missing'] for p in prep},
            'train_request_sha256': sha(compact(wanted)), 'metrics': metrics,
            'child_sampled_max_rss_bytes': max(s['rss_bytes'] for s in stderr)}

    assert total_rejections == 35
    documented = read(RUN/'rejected-examples.json')
    assert len(documented) == 33
    for r in documented:
        assert any(q == r['request'] and a == r['response'] for q, a in
                   zip(lines(RUN/f'{r["arm"]}-stdin.jsonl'), lines(RUN/f'{r["arm"]}-stdout.jsonl')))

    # JS is used only as a serialization codec here, never fit/predict code.
    hash_command = [process_receipts[0]['command'][0], '-e',
        "const fs=require('node:fs'),c=require('node:crypto'); const h=o=>c.createHash('sha256').update(JSON.stringify(o)).digest('hex'); console.log(JSON.stringify(JSON.parse(fs.readFileSync(0,'utf8')).map(x=>({arm:x.arm,original_content_sha256:h(x.original),independent_content_sha256:h(x.independent)}))))"]
    hashes = json.loads(subprocess.run(hash_command, input=json.dumps(model_hash_jobs),
        text=True, check=True, capture_output=True).stdout)
    for row in hashes:
        assert row['original_content_sha256'] == receipt['model_sha256'][row['arm']]
        row['byte_identical'] = row['original_content_sha256'] == row['independent_content_sha256']
    result['js_serialization_hash_comparison'] = hashes
    result['label_dependence'] = {
        part: {'decisions': sum(e['partition'] == part and e['loss_eligible'] for e in evaluation),
               'unique_nonnull_event_groups': len({e['event_group'] for e in evaluation if e['partition'] == part and e['loss_eligible'] and e['event_group'] is not None}),
               'null_event_group_rows': sum(e['partition'] == part and e['loss_eligible'] and e['event_group'] is None for e in evaluation)}
        for part in ['early', 'middle', 'late']}
    result['unit_controls'] = {
        'last_trade_price_1e8': scalar('100000000', 'last_trade_price'),
        'quantity_1e8': scalar('100000000', 'bid_quantity'),
        'elapsed_1e9': scalar('1000000000', 'since_last_trade_ns'),
        'mid2_delta_2e8': scalar('200000000', 'published_mid2_delta_1s'),
        'imbalance_half': scalar({'numerator': '1', 'denominator': '2'}, 'queue_imbalance')}
    assert list(result['unit_controls'].values()) == [1, 1, 1, 1, 0.5]
    after = snapshot(paths)
    assert before == after
    write('input-hashes-after.json', after)
    result['inputs_unchanged'] = True
    result['finished_at'] = datetime.datetime.now(datetime.timezone.utc).isoformat()
    write('independent-results.json', result)
    print(json.dumps({'status': 'passed', 'refits': 2, 'inputs_unchanged': True,
        'comparison': {a: {k: v for k, v in r.items() if 'diff' in k or k == 'metrics'}
                       for a, r in result['arms'].items()}}, indent=2))


if __name__ == '__main__':
    main()
