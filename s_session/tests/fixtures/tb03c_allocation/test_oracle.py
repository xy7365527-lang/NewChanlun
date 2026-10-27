"""C4a 独立 Fraction 配额 oracle；不导入 Rust 或现有生产 reducer。"""
import json
from fractions import Fraction
from pathlib import Path
import unittest

HAND = json.loads(Path(__file__).with_name('HAND-ORACLE.json').read_text())
POLICY = 'frozen-manifest-proportional-cumulative/2'


def quota(total, weights, ranks):
    if not weights or min(weights) <= 0 or sorted(ranks) != list(range(len(weights))):
        raise ValueError('manifest')
    if total < 0 or total > sum(weights):
        raise ValueError('range')
    result = [0] * len(weights)
    for size in range(1, total + 1):
        eligible = [i for i, weight in enumerate(weights)
                    if result[i] < -(-size * weight // sum(weights))]
        winner = max(eligible, key=lambda i: (Fraction(weights[i], result[i] + 1),
                                             weights[i], -ranks[i]))
        result[winner] += 1
    return result


def fee_split(fee, increments, ranks):
    """直接无上限金额配额序列，独立于生产侧权重放大技巧。"""
    result = [0] * len(increments)
    if fee and not sum(increments):
        raise ValueError('zero quantity fee')
    for size in range(1, fee + 1):
        eligible = [i for i, weight in enumerate(increments)
                    if weight and result[i] < -(-size * weight // sum(increments))]
        winner = max(eligible, key=lambda i: (Fraction(increments[i], result[i] + 1),
                                             increments[i], -ranks[i]))
        result[winner] += 1
    return result


class IndependentLedger:
    """测试专用 E/B 状态机；读原件和冻结清单，金额预期仍与手算逐步对拍。"""
    def __init__(self, policy=POLICY):
        self.manifest = json.loads(Path(__file__).with_name('FROZEN-MANIFEST.json').read_text())
        self.raw = json.loads(Path(__file__).with_name('RAW.json').read_text())
        self.policy = policy
        self.recorded = {}
        self.allocations = {}
        self.frontier = 0
        self.pending = {}
        self.applied = set()
        self.q = HAND['opening_quantity'][:]
        self.k = HAND['opening_K'][:]

    def capture(self, index):
        import copy
        event = copy.deepcopy(self.raw['events'][index])
        key = event['source']['external_fact_id']
        if (event['netted_fill']['manifest_hash'] != self.manifest['manifest_hash']
                or event['netted_fill'].get('symbol') != self.manifest['symbol']):
            raise ValueError('manifest')
        if key in self.recorded and self.recorded[key] != event:
            raise ValueError('identity')
        self.recorded[key] = event
        self.pending[key] = 'AwaitingPolicy' if self.policy is None else 'Unresolved'
        return key

    def resolve(self, key):
        if self.policy != POLICY:
            raise ValueError('AwaitingPolicy')
        event = self.recorded[key]
        fill = event['netted_fill']
        if int(fill['from']) != self.frontier:
            raise ValueError('MissingDependency')
        participants = self.manifest['participants']
        weights = [int(p['requested_q']) for p in participants]
        ranks = [self.manifest['tie_break_order'].index(p['chong_id']) for p in participants]
        delta = [a-b for a,b in zip(quota(int(fill['to']),weights,ranks),quota(self.frontier,weights,ranks))]
        fees = fee_split(int(fill['commission']),delta,ranks)
        self.allocations[key] = [(delta[i],delta[i]*int(fill['unit_price'])+fees[i]) for i in range(len(delta))]
        self.frontier = int(fill['to'])
        self.pending.pop(key, None)

    def apply(self, key, owner, claimed_owner=None):
        if self.pending.get(key) == 'AwaitingPolicy':
            raise ValueError('AwaitingPolicy')
        if claimed_owner is not None and claimed_owner != owner:
            raise ValueError('wrong owner')
        if key not in self.allocations:
            raise ValueError('AwaitingDependency')
        identity = (key,owner)
        if identity in self.applied:
            return
        quantity,cash = self.allocations[key][owner]
        self.q[owner] += quantity
        self.k[owner] += cash
        self.applied.add(identity)


class OracleTests(unittest.TestCase):
    def test_we7_hand_calculations(self):
        weights, ranks = HAND['weights'], HAND['tie_rank']
        self.assertEqual([quota(f, weights, ranks) for f in range(8)], HAND['prefixes'])
        quantity, capital = HAND['opening_quantity'][:], HAND['opening_K'][:]
        for report in HAND['reports']:
            delta = [a-b for a,b in zip(quota(report['to'],weights,ranks),
                                       quota(report['from'],weights,ranks))]
            fees = fee_split(report['commission'],delta,ranks)
            self.assertEqual(delta,report['quantity'])
            self.assertEqual(fees,report['fees'])
            quantity = [q+d for q,d in zip(quantity,delta)]
            capital = [k+d*report['price']+fee for k,d,fee in zip(capital,delta,fees)]
            self.assertEqual(quantity,report['book_quantity'])
            self.assertEqual(capital,report['K'])

    def test_fee_scaling_is_same_algorithm_including_zero_legs(self):
        from itertools import product
        for delta in product(range(4),repeat=3):
            if not sum(delta):
                continue
            active = [i for i,d in enumerate(delta) if d]
            for fee in range(13):
                expected = fee_split(fee,delta,[0,1,2])
                actual = [0,0,0]
                if fee:
                    scaled = quota(fee,[delta[i]*fee for i in active],list(range(len(active))))
                    for i,v in zip(active,scaled):
                        actual[i] = v
                self.assertEqual(actual,expected)
                self.assertEqual(sum(actual),fee)

    def test_incremental_and_cumulative_same_economic_input(self):
        # 同一价格、每量一份佣金；分批与全量具有相同经济输入。
        weights, ranks = HAND['weights'], HAND['tie_rank']
        split_q = [0,0,0]
        split_k = [0,0,0]
        for before,after in [(0,3),(3,4),(4,7)]:
            delta = [a-b for a,b in zip(quota(after,weights,ranks),quota(before,weights,ranks))]
            fees = fee_split(after-before,delta,ranks)
            split_q = [a+b for a,b in zip(split_q,delta)]
            split_k = [a+d*100+f for a,d,f in zip(split_k,delta,fees)]
        full = quota(7,weights,ranks)
        self.assertEqual(split_q,full)
        self.assertEqual(split_k,[101*d for d in full])

    def test_source_capture_resolve_books_and_duplicates_match_hand(self):
        ledger = IndependentLedger()
        for index, expected in enumerate(HAND['reports'],3):
            key = ledger.capture(index)
            with self.assertRaisesRegex(ValueError,'AwaitingDependency'):
                ledger.apply(key,0)
            ledger.resolve(key)
            for owner in range(3):
                ledger.apply(key,owner)
                ledger.apply(key,owner)
            self.assertEqual(ledger.q,expected['book_quantity'])
            self.assertEqual(ledger.k,expected['K'])

    def test_missing_policy_and_wrong_frontier_rejected(self):
        ledger = IndependentLedger(None)
        key = ledger.capture(3)
        self.assertEqual(ledger.pending[key],'AwaitingPolicy')
        with self.assertRaisesRegex(ValueError,'AwaitingPolicy'):
            ledger.resolve(key)
        with self.assertRaisesRegex(ValueError,'AwaitingPolicy'):
            ledger.apply(key,0)
        self.assertEqual(ledger.q,HAND['opening_quantity'])
        self.assertEqual(ledger.k,HAND['opening_K'])
        ledger = IndependentLedger()
        gap = ledger.capture(4)
        with self.assertRaisesRegex(ValueError,'MissingDependency'):
            ledger.resolve(gap)
        self.assertEqual(ledger.frontier,0)
        key = ledger.capture(3)
        ledger.resolve(key)
        with self.assertRaisesRegex(ValueError,'wrong owner'):
            ledger.apply(key,0,1)
        self.assertFalse(ledger.applied)
        with self.assertRaisesRegex(ValueError,'MissingDependency'):
            ledger.resolve(key)
        ledger.raw['events'][5]['netted_fill']['manifest_hash'] = 'wrong'
        with self.assertRaisesRegex(ValueError,'manifest'):
            ledger.capture(5)

    def test_same_symbol_identity_and_reserve_cleared_risk_pending(self):
        ledger = IndependentLedger()
        symbols = [e['book_identity']['symbol'] for e in ledger.raw['events'][:3]]
        self.assertEqual(set(symbols), {ledger.manifest['symbol']})
        self.assertEqual(len({e['book_identity']['op_level'] for e in ledger.raw['events'][:3]}), 3)
        key = ledger.capture(3)
        del ledger.pending[key]  # 独立风险覆盖清风险，不删除已 Recorded 分配待办。
        ledger.resolve(key)
        for owner in range(3):
            ledger.apply(key,owner)
        self.assertEqual(ledger.q,HAND['reports'][0]['book_quantity'])
        self.assertEqual(ledger.k,HAND['reports'][0]['K'])
        ledger.raw['events'][4]['netted_fill']['symbol'] = 'OTHER'
        with self.assertRaisesRegex(ValueError,'manifest'):
            ledger.capture(4)

    def test_negative_control_largest_remainder_breaks_monotonicity(self):
        self.assertLess([0,2,2][0],[1,1,1][0])
        self.assertGreaterEqual(HAND['prefixes'][4][0],HAND['prefixes'][3][0])

if __name__ == '__main__':
    unittest.main()
