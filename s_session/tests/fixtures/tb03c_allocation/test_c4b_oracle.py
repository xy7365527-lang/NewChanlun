"""C4b RAW→独立 Fraction 手算锁；不模拟 Rust 的事务、恢复或存续来源栈。"""
import copy
import hashlib
import json
import re
from pathlib import Path
from typing import Any
import unittest
from test_oracle import quota, fee_split

HERE = Path(__file__).resolve().parent
HAND = json.loads((HERE / 'C4B-HAND-ORACLE.json').read_text())
POLICY = 'frozen-manifest-proportional-cumulative/2'
REVISION = 'we7-frozen-1'  # 此独立夹具的显式冻结版本，与测试 E 配置同一声明。
IDENTITY = ('source_namespace', 'source_epoch', 'external_fact_id')
Json = dict[str, Any]


def quantity(value: Any) -> int:
    """原件的精确非负十进制整数；退款不以负费用入账。"""
    if not isinstance(value, str) or not re.fullmatch(r'0|[1-9][0-9]*', value):
        raise ValueError('nonnegative integer text required')
    return int(value)


def identity(source: Json) -> tuple[str, str, str]:
    values = tuple(source[key] for key in IDENTITY)
    if any(not isinstance(value, str) or not value for value in values):
        raise ValueError('source identity')
    return values


def derive_from_raw(raw: Json, manifest: Json) -> Json:
    """从原件与独立冻结政策推导本固定路径的全部数值，不读 HAND。"""
    manifest_body = {key: value for key, value in manifest.items() if key != 'manifest_hash'}
    digest = hashlib.sha256(json.dumps(manifest_body, ensure_ascii=False, sort_keys=True,
                                      separators=(',', ':')).encode()).hexdigest()
    if manifest['manifest_hash'] != digest or manifest['projection_kind'] != 'netted':
        raise ValueError('frozen manifest')
    if raw['units']['numeric_encoding'] != 'exact_decimal_integer_text':
        raise ValueError('numeric encoding')
    for kind in ('money', 'quantity'):
        if raw['units'][kind]['quantum'] != '1':
            raise ValueError('fixture quantum')
    participants = manifest['participants']
    ids = [participant['chong_id'] for participant in participants]
    weights = [quantity(participant['requested_q']) for participant in participants]
    ranks = [manifest['tie_break_order'].index(chong) for chong in ids]
    n = len(ids)
    q, capital, late_fees = [0] * n, [0] * n, [0] * n
    originals: dict[tuple[str, str, str], Json] = {}
    allocations: dict[tuple[str, str, str], tuple[list[int], list[int]]] = {}
    frontier = 0
    result: Json = {}
    opening_seen: set[str] = set()

    def snapshot() -> Json:
        return {'q': q[:], 'K': capital[:]}

    def bind(event: Json) -> tuple[Json, tuple[list[int], list[int]]]:
        fill = event['netted_fill']
        ref = fill['fill_ref']
        if identity(ref) != identity(event['depends_on']):
            raise ValueError('dependency identity')
        original = originals.get(identity(ref))
        if original is None or identity(ref) not in allocations:
            raise ValueError('named original nonzero fill missing')
        if ref['allocation_id'] != original['allocation_id']:
            raise ValueError('original allocation')
        expected_policy = {'fill_allocation_policy_id': POLICY,
                           'policy_revision': REVISION, 'manifest_hash': digest}
        if any(ref[key] != value for key, value in expected_policy.items()):
            raise ValueError('policy triple')
        for key in ('manifest_hash', 'symbol'):
            if fill[key] != original['netted_fill'][key]:
                raise ValueError('original manifest or symbol')
        return original['netted_fill'], allocations[identity(ref)]

    for index, event in enumerate(raw['events']):
        key = identity(event['source'])
        if key in originals:
            raise ValueError('duplicate original identity')
        originals[key] = event
        if 'observed_existing_book' in event:
            chong = event['book_identity']['chong_id']
            if chong in opening_seen:
                raise ValueError('duplicate opening')
            opening_seen.add(chong)
            i = ids.index(chong)
            q[i] = quantity(event['observed_existing_book']['units'])
            for component in event['known_cost_components']:
                if component['kind'] not in ('acquisition_cash', 'commission'):
                    raise ValueError('unsupported opening cost')
                capital[i] += quantity(component['canonical_amount'])
            continue
        fill = event['netted_fill']
        if fill['manifest_hash'] != digest or fill['symbol'] != manifest['symbol']:
            raise ValueError('report manifest or symbol')
        if fill['income_changes'] != {'Pi': '0', 'A': '0', 'W': '0'}:
            raise ValueError('undeclared income')
        for field in ('netted_fill', 'stable_point') + (('depends_on',) if 'fill_ref' in fill else ()):
            fragment = raw['evidence_index']['fragments'][event[field]['source_ref']]
            if fragment != [{'json_pointer': f'/events/{index}/{field}'}]:
                raise ValueError('source fragment pointer')
        kind = fill['report_kind']
        if kind in ('incremental', 'cumulative'):
            start, end = quantity(fill['from']), quantity(fill['to'])
            if start != frontier or start >= end or fill['cash_kind'] != 'acquisition_cash':
                raise ValueError('fill interval')
            reported = end-start if kind == 'incremental' else end
            if quantity(fill['reported_quantity']) != reported:
                raise ValueError('reported quantity')
            delta = [a-b for a, b in zip(quota(end, weights, ranks), quota(start, weights, ranks))]
            fees = fee_split(quantity(fill['commission']), delta, ranks)
            price = quantity(fill['unit_price'])
            q = [old+change for old, change in zip(q, delta)]
            capital = [old+change*price+fee for old, change, fee in zip(capital, delta, fees)]
            allocations[key] = (delta, fees)
            frontier = end
            result['after_fills'] = snapshot()
        elif kind == 'fee_only':
            original, (delta, _) = bind(event)
            if fill['cause'] != 'fill' or not fill['fee_type']:
                raise ValueError('fee cause')
            fees = fee_split(quantity(fill['amount']), delta, ranks)
            late_fees = [a+b for a, b in zip(late_fees, fees)]
            capital = [a+b for a, b in zip(capital, fees)]
            result['late_fee_N1'] = {'units': [0]*n, 'fees': fees, **snapshot()}
        elif kind == 'bust':
            original, _ = bind(event)
            start, end = quantity(fill['from']), quantity(fill['to'])
            if (start != frontier or end >= start or quantity(fill['reported_quantity']) != start-end
                    or end < quantity(original['from']) or start > quantity(original['to'])
                    or fill['component'] != 'acquisition_cash'):
                raise ValueError('single original bust interval')
            reverse = [a-b for a, b in zip(quota(start, weights, ranks), quota(end, weights, ranks))]
            cash = [count*quantity(original['unit_price']) for count in reverse]
            q = [a-b for a, b in zip(q, reverse)]
            capital = [a-b for a, b in zip(capital, cash)]
            frontier = end
            result['bust_N3'] = {'units': [-x for x in reverse], 'cash_reversed': cash,
                                 'commission_reversed': [0]*n,
                                 'K_without_late_fee': [a-b for a, b in zip(capital, late_fees)],
                                 **snapshot()}
        elif kind == 'reverse_component':
            _, (_, fees) = bind(event)
            if fill['component'] != 'commission':
                raise ValueError('refund component')
            capital = [a-b for a, b in zip(capital, fees)]
            result['refund_N3_commission'] = {'units': [0]*n, 'commission_reversed': fees,
                                             **snapshot()}
        else:
            raise ValueError('unsupported fixture kind')
    return result


def assert_hand(raw: Json, manifest: Json) -> None:
    """数值字典完整比较，使 RAW 变化不能仅通过引用自洽就漏过。"""
    expected = {key: value for key, value in HAND.items() if isinstance(value, dict)}
    if derive_from_raw(raw, manifest) != expected:
        raise ValueError('RAW does not derive frozen HAND numeric fields')


class C4bOracleTests(unittest.TestCase):
    def test_all_hand_numeric_fields_derive_from_raw(self) -> None:
        raw = json.loads((HERE / 'C4B-RAW.json').read_text())
        manifest = json.loads((HERE / 'FROZEN-MANIFEST.json').read_text())
        assert_hand(raw, manifest)

    def test_five_review_mutations_cannot_survive_raw_hand_and_identity_binding(self) -> None:
        source = json.loads((HERE / 'C4B-RAW.json').read_text())
        manifest = json.loads((HERE / 'FROZEN-MANIFEST.json').read_text())
        for mutation in ('fee_amount', 'bust_interval', 'policy_revision', 'manifest_hash', 'source_epoch'):
            raw = copy.deepcopy(source)
            if mutation == 'fee_amount':
                raw['events'][6]['netted_fill']['amount'] = '6'
            elif mutation == 'bust_interval':
                raw['events'][7]['netted_fill'].update(to='6', reported_quantity='1')
            elif mutation == 'policy_revision':
                raw['events'][6]['netted_fill']['fill_ref']['policy_revision'] = 'stale'
            elif mutation == 'manifest_hash':
                raw['events'][6]['netted_fill']['manifest_hash'] = 'wrong'
                raw['events'][6]['netted_fill']['fill_ref']['manifest_hash'] = 'wrong'
            else:
                raw['events'][6]['netted_fill']['fill_ref']['source_epoch'] = 'old'
                raw['events'][6]['depends_on']['source_epoch'] = 'old'
            with self.subTest(mutation=mutation), self.assertRaises(ValueError):
                assert_hand(raw, manifest)

    def test_each_source_and_policy_identity_field_is_bound(self) -> None:
        source = json.loads((HERE / 'C4B-RAW.json').read_text())
        manifest = json.loads((HERE / 'FROZEN-MANIFEST.json').read_text())
        for field in IDENTITY + ('allocation_id', 'fill_allocation_policy_id', 'policy_revision', 'manifest_hash'):
            raw = copy.deepcopy(source)
            raw['events'][6]['netted_fill']['fill_ref'][field] = 'wrong'
            if field in IDENTITY:
                raw['events'][6]['depends_on'][field] = 'wrong'
            with self.subTest(field=field), self.assertRaises(ValueError):
                assert_hand(raw, manifest)

    def test_late_fee_uses_original_fill_not_latest_manifest_or_book(self) -> None:
        original = [0, 2, 1]
        actual = fee_split(5, original, [0, 1, 2])
        self.assertEqual(actual, HAND['late_fee_N1']['fees'])
        self.assertNotEqual(actual, fee_split(5, [1, 3, 3], [0, 1, 2]))
        self.assertNotEqual(actual, fee_split(5, [1, 1, 1], [0, 1, 2]))

    def test_integer_residue_zero_shares_and_inverse_telescoping(self) -> None:
        for amount in range(30):
            fees = fee_split(amount, [0, 2, 1], [0, 1, 2])
            self.assertEqual(fees[0], 0)
            self.assertEqual(sum(fees), amount)
        for start in range(8):
            for middle in range(start+1):
                for end in range(middle+1):
                    a, b, c = [quota(f, [1, 3, 3], [0, 1, 2]) for f in (start, middle, end)]
                    self.assertEqual([x-z for x, z in zip(a, c)],
                                     [(x-y)+(y-z) for x, y, z in zip(a, b, c)])

    def test_raw_retains_original_objects_and_current_fixture_passes(self) -> None:
        original = json.loads((HERE / 'RAW.json').read_text())
        raw = json.loads((HERE / 'C4B-RAW.json').read_text())
        self.assertEqual(raw['events'][:6], original['events'])
        assert_hand(raw, json.loads((HERE / 'FROZEN-MANIFEST.json').read_text()))


if __name__ == '__main__':
    unittest.main()
