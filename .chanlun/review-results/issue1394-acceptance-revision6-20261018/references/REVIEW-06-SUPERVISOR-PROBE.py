"""REVIEW-06 own probes: boundary sweep (legal queued must pass) + unregistered descendant (must reject)."""
import sys, unittest
from dataclasses import asdict
sys.path.insert(0, sys.argv[1])
import test_review04 as previous, test_review05 as r05, supervisor as sup

class R06(unittest.TestCase):
    def test_sweep_legal(self):
        t5 = r05.Review05Tests(); t5.setUp() if hasattr(t5,'setUp') else None
        bad = []
        for count in (1, 2):
            for i in range(0, 60):
                delay = .12490 + i * .0000025
                try: t5.queued_case(count, delay)
                except AssertionError as e: bad.append((count, round(delay,7), str(e)[:80]))
        print("legal sweep cases=120 bad=", len(bad), bad[:3])
        self.assertFalse(bad)

    def test_unregistered_rejected(self):
        bad = []
        for count, delay in ((1,.124995),(2,.124985),(1,.2),(1,.1245)):
            case = previous.Review04Tests()
            def effect(state):
                state['queue'].append({'kind':'HEARTBEAT','worker':asdict(state['worker'])})
                state['probe'].identities[999] = sup.Identity(999,'999',101,101)
                state['probe'].desc.add(999)   # stray descendant, never registered
                state['queue'].append(state['end'])
                r05.Review05Tests().advancing_clock(state)
            st = case.scenario(count=count, delay=delay, queued=True, effect=effect)
            if st['error'] is None or 999 in st['core'].owners: bad.append((count, delay, st['error']))
        print("stray-descendant cases=4 accepted_wrongly=", len(bad), bad)
        self.assertFalse(bad)

r = unittest.TextTestRunner(verbosity=1).run(unittest.defaultTestLoader.loadTestsFromTestCase(R06))
print("RUN", r.testsRun, "FAIL", len(r.failures), "ERR", len(r.errors))
for t,e in r.failures+r.errors: print("X", t.id(), e.strip().splitlines()[-1][:300])
