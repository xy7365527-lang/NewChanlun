#!/usr/bin/env python3
"""Run only deliberately invalid requests against the frozen real child.

Zero successful fits or market predictions; independent audit already consumed
its two-refit allowance. A response is read before the next request is sent.
"""
import copy
import datetime
import hashlib
import json
import pathlib
import subprocess

OUT = pathlib.Path(__file__).resolve().parent
AUTHOR = OUT.parent/'learning-author'
RUN = AUTHOR/'run-v1'
VERSION = 'stage69-softmax-finite-features/1'
CUT = '1609459220000000000'
node = json.loads((RUN/'process-receipts.json').read_text())[0]['command'][0]
results = []
for arm in ['A', 'B']:
    fields = json.loads((RUN/f'{arm}-model.json').read_text())['feature_names']
    prediction = {'op': 'predict', 'version': VERSION, 'product_id': 'BTC-USD',
                  'decision_ns': CUT, 'features': {k: None for k in fields}}
    fit = {'op': 'fit', 'version': VERSION, 'arm': arm, 'fit_cutoff_ns': CUT,
           'rows': [{'product_id': 'BTC-USD', 'decision_ns': CUT,
                     'features': {k: None for k in fields}, 'label': 'up'}]}
    controls = [('unfitted-prediction', prediction, 'predict:not-fitted'),
                ('fit-decision-equality', fit, 'fit:decision-before-cutoff-and-ordered')]
    after = copy.deepcopy(fit)
    after['rows'][0]['decision_ns'] = str(int(CUT)+1)
    controls.append(('fit-decision-after-one-ns', after, 'fit:decision-before-cutoff-and-ordered'))
    with_known = copy.deepcopy(fit)
    with_known['rows'][0]['decision_ns'] = str(int(CUT)-1)
    with_known['rows'][0]['label_known_ns'] = str(int(CUT)-1)
    controls.append(('fit-forbidden-known', with_known, 'training-row:keys'))
    with_path = copy.deepcopy(fit)
    with_path['label_path'] = '/forbidden/reviewer-control'
    controls.append(('fit-forbidden-path', with_path, 'fit:keys'))
    with_feature = copy.deepcopy(fit)
    with_feature['rows'][0]['decision_ns'] = str(int(CUT)-1)
    with_feature['rows'][0]['features']['bid_price' if arm == 'A' else 'order_seq'] = '1'
    controls.append(('fit-forbidden-feature', with_feature, 'features:keys'))
    malformed = copy.deepcopy(fit)
    malformed['rows'][0]['decision_ns'] = '1609459220000000000.0'
    controls.append(('integer-no-float-string', malformed, 'integer:string'))
    controls.append(('inspect-unfitted', {'op': 'inspect', 'version': VERSION}, 'inspect:state'))
    command = [node, '--max-old-space-size=64', str(AUTHOR/'learner-stdin.mjs')]
    started = datetime.datetime.now(datetime.timezone.utc).isoformat()
    stderr_path = OUT/f'{arm}-boundary-stderr.jsonl'
    entries = []
    with stderr_path.open('w') as err:
        child = subprocess.Popen(command, cwd=OUT, stdin=subprocess.PIPE,
                                 stdout=subprocess.PIPE, stderr=err, text=True,
                                 env={'PATH': str(pathlib.Path(node).parent), 'LANG': 'C'})
        for name, request, expected in controls:
            payload = json.dumps(request, separators=(',', ':'))
            sent = datetime.datetime.now(datetime.timezone.utc).isoformat()
            child.stdin.write(payload+'\n')
            child.stdin.flush()
            answer_line = child.stdout.readline().rstrip('\n')
            received = datetime.datetime.now(datetime.timezone.utc).isoformat()
            answer = json.loads(answer_line)
            assert answer == {'status': 'rejected', 'error': expected}, (name, answer)
            entries.append({'name': name, 'request': request, 'response': answer,
                'sent_at': sent, 'received_at': received,
                'input_sha256': hashlib.sha256(payload.encode()).hexdigest(),
                'output_sha256': hashlib.sha256(answer_line.encode()).hexdigest()})
        child.stdin.close()
        assert child.wait(timeout=10) == 0
        assert child.stdout.read() == ''
    stderr = [json.loads(line) for line in stderr_path.read_text().splitlines()]
    assert len(stderr) == len(entries) and all(s['operation'] == 'rejected' for s in stderr)
    assert all(s['pid'] == child.pid for s in stderr)
    results.append({'arm': arm, 'pid': child.pid, 'command': command,
        'cwd': str(OUT), 'started_at': started,
        'finished_at': datetime.datetime.now(datetime.timezone.utc).isoformat(),
        'exit_code': child.returncode, 'successful_fits': 0, 'predictions': 0,
        'scope': 'negative protocol controls only; no fitted-state controls or sandbox claim',
        'controls': entries})
(OUT/'boundary-controls.json').write_text(json.dumps(results, indent=2)+'\n')
print(json.dumps({'status': 'passed', 'negative_controls': 16,
                  'successful_fits': 0, 'market_predictions': 0}))
