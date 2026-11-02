"""#1670 per_leg 原件构造器；手算数值独立保存在 PER-LEG-HAND-ORACLE.json。"""
import argparse
import hashlib
import json
from pathlib import Path

HERE = Path(__file__).resolve().parent


def source_refs(value):
    if isinstance(value, dict):
        for key, item in value.items():
            if key == 'source_ref':
                yield item
            else:
                yield from source_refs(item)
    elif isinstance(value, list):
        for item in value:
            yield from source_refs(item)


def generate():
    raw = json.loads((HERE / 'RAW.json').read_text())
    raw['events'] = raw['events'][:4]
    event = raw['events'][3]
    fill = event.pop('netted_fill')
    manifest = json.loads((HERE / 'FROZEN-MANIFEST.json').read_text())
    manifest['projection_kind'] = 'per_leg'
    manifest_body = {key: value for key, value in manifest.items() if key != 'manifest_hash'}
    manifest_hash = hashlib.sha256(json.dumps(
        manifest_body, ensure_ascii=False, sort_keys=True,
        separators=(',', ':')).encode()).hexdigest()
    event['per_leg_report'] = {
        'manifest_hash': manifest_hash,
        'symbol': fill['symbol'],
        'reported_quantity': '3',
        'unit_price': '100',
        'commission': '3',
        'leg_attribution': {
            'chong-N1': {'units': '2', 'acquisition_cash': '200', 'commission': '2'},
            'chong-N2': {'units': '1', 'acquisition_cash': '100', 'commission': '1'},
            'chong-N3': {'units': '0', 'acquisition_cash': '0', 'commission': '0'},
        },
        'source_ref': fill['source_ref'],
    }
    used = set(source_refs(raw['events']))
    raw['evidence_index']['fragments'] = {
        key: value for key, value in raw['evidence_index']['fragments'].items() if key in used
    }
    raw['evidence_index']['fragments'][fill['source_ref']] = [
        {'json_pointer': '/events/3/per_leg_report'}
    ]
    return raw


if __name__ == '__main__':
    parser = argparse.ArgumentParser()
    parser.add_argument('--check', action='store_true')
    args = parser.parse_args()
    path = HERE / 'PER-LEG-RAW.json'
    generated = json.dumps(generate(), ensure_ascii=False, sort_keys=True, indent=2) + '\n'
    if args.check:
        if not path.exists() or path.read_text() != generated:
            raise SystemExit('PER-LEG-RAW.json is stale; run generate_per_leg_fixtures.py')
    else:
        path.write_text(generated)
