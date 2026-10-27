"""#1670 per_leg RAW → 独立手算账簿；不导入生产 reducer 或数据库。"""
import copy
import hashlib
import json
import re
import subprocess
import sys
import unittest
from pathlib import Path
from test_oracle import quota

HERE = Path(__file__).resolve().parent
HAND = json.loads((HERE / 'PER-LEG-HAND-ORACLE.json').read_text())
FIELDS = {'units', 'acquisition_cash', 'commission'}


def number(value):
    if not isinstance(value, str) or not re.fullmatch(r'0|[1-9][0-9]*', value):
        raise ValueError('nonnegative canonical integer required')
    return int(value)


def per_leg_manifest():
    manifest = json.loads((HERE / 'FROZEN-MANIFEST.json').read_text())
    manifest['projection_kind'] = 'per_leg'
    body = {key: value for key, value in manifest.items() if key != 'manifest_hash'}
    manifest['manifest_hash'] = hashlib.sha256(json.dumps(
        body, ensure_ascii=False, sort_keys=True, separators=(',', ':')).encode()).hexdigest()
    return manifest


def derive(raw, manifest):
    event = raw['events'][3]
    report = event['per_leg_report']
    if report['manifest_hash'] != manifest['manifest_hash'] or report['symbol'] != manifest['symbol']:
        raise ValueError('per_leg report does not match frozen manifest')
    for field in ('per_leg_report', 'stable_point'):
        fragment = raw['evidence_index']['fragments'][event[field]['source_ref']]
        if fragment != [{'json_pointer': f'/events/3/{field}'}]:
            raise ValueError('source fragment pointer')
    participants = manifest['participants']
    ids = [participant['chong_id'] for participant in participants]
    attribution = report['leg_attribution']
    if set(attribution) != set(ids):
        raise ValueError('attribution must exactly cover frozen participants')
    quantity = number(report['reported_quantity'])
    price = number(report['unit_price'])
    routes = []
    for chong in ids:
        leg = attribution[chong]
        if set(leg) != FIELDS:
            raise ValueError('exact leg fields required')
        units = number(leg['units'])
        cash = number(leg['acquisition_cash'])
        commission = number(leg['commission'])
        if cash != units * price or (units == 0 and commission != 0):
            raise ValueError('leg amount does not close')
        routes.append({'chong_id': chong, 'units': str(units),
                       'acquisition_cash': str(cash), 'commission': str(commission)})
    if (sum(number(route['units']) for route in routes) != quantity
            or sum(number(route['acquisition_cash']) for route in routes) != quantity * price
            or sum(number(route['commission']) for route in routes) != number(report['commission'])):
        raise ValueError('attribution totals do not close')
    q, capital = [], []
    for event in raw['events'][:3]:
        opening = event['observed_existing_book']
        q.append(number(opening['units']))
        capital.append(sum(number(component['canonical_amount'])
                           for component in event['known_cost_components']
                           if component['kind'] in ('acquisition_cash', 'commission')))
    for index, route in enumerate(routes):
        q[index] += number(route['units'])
        capital[index] += number(route['acquisition_cash']) + number(route['commission'])
    return {'policy': 'per-leg-source-attribution/1', 'routes': routes,
            'after_fill': {'q': q, 'K': capital}}


def assert_hand(raw, manifest):
    actual = derive(raw, manifest)
    expected = {key: HAND[key] for key in ('policy', 'routes', 'after_fill')}
    if actual != expected:
        raise ValueError(f'RAW does not derive HAND: {actual!r} != {expected!r}')


class PerLegOracleTests(unittest.TestCase):
    def setUp(self):
        self.raw = json.loads((HERE / 'PER-LEG-RAW.json').read_text())
        self.manifest = per_leg_manifest()

    def test_generator_reproduces_raw(self):
        result = subprocess.run(
            [sys.executable, '-B', str(HERE / 'generate_per_leg_fixtures.py'), '--check'],
            capture_output=True, text=True)
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)

    def test_hand_matches_independent_oracle(self):
        assert_hand(self.raw, self.manifest)

    def test_source_attribution_is_not_proportional_allocation(self):
        actual = [int(route['units']) for route in HAND['routes']]
        weights = [int(participant['requested_q']) for participant in self.manifest['participants']]
        ranks = list(range(len(weights)))
        self.assertNotEqual(actual, quota(3, weights, ranks))

    def test_closed_negative_controls(self):
        for mutation in ('missing', 'unknown', 'quantity', 'cash', 'commission', 'zero_with_fee', 'pointer'):
            raw = copy.deepcopy(self.raw)
            attribution = raw['events'][3]['per_leg_report']['leg_attribution']
            if mutation == 'missing':
                del attribution['chong-N2']
            elif mutation == 'unknown':
                attribution['chong-unknown'] = attribution.pop('chong-N3')
            elif mutation == 'quantity':
                attribution['chong-N1'].update(units='3', acquisition_cash='300')
            elif mutation == 'cash':
                attribution['chong-N1']['acquisition_cash'] = '201'
            elif mutation == 'commission':
                attribution['chong-N1']['commission'] = '3'
            elif mutation == 'zero_with_fee':
                attribution['chong-N3']['commission'] = '1'
            else:
                raw['evidence_index']['fragments'][
                    raw['events'][3]['per_leg_report']['source_ref']] = [
                        {'json_pointer': '/events/3/netted_fill'}]
            with self.subTest(mutation=mutation), self.assertRaises(ValueError):
                assert_hand(raw, self.manifest)


if __name__ == '__main__':
    unittest.main()
