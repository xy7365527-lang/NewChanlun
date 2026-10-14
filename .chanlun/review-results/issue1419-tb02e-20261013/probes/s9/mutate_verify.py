"""tb02e_verify.check 的变异核对：改冻结期待的任一字段，核对必须拒。"""
import copy, json, sys
from contextlib import closing
from pathlib import Path
T = Path('/private/tmp/goal1323-native-20261012/tb1419-wip-wt/s_session/tests'); sys.path.insert(0, str(T))
import tb02e_verify as V
case, db = sys.argv[1], Path(sys.argv[2])
oracle = json.loads((V.FIXTURE / 'hand-oracle.json').read_text())['cases'][case]
audit = V.reader._query_integrity()
with closing(V.reader.open_readonly(db)) as conn, conn:
    conn.execute('BEGIN'); proof = audit.verify(audit.capture(conn), vars(V.reader))
snap = {n: audit.project_state(proof, int(n), vars(V.reader))['snapshot'] for n in oracle['cuts']}
def m_lo(e): e['assignments'][0]['lo'] = str(int(e['assignments'][0]['lo']) - 1)
def m_geo(e): e['assignments'][0]['lo'] = str(int(e['assignments'][0]['lo']) + 100)
def m_reason(e): e['assignments'][0]['reason'] = 'SEED_NOT_YET_EVALUABLE'
def m_offset(e): e['scan_cursor']['awaiting_offset'] = '1' if e['scan_cursor']['awaiting_offset'] else None
def m_await(e): e['scan_cursor']['awaiting_unit_id'] = None; e['scan_cursor']['awaiting_offset'] = '1'
def m_done(e): e['completed_units'] = e['completed_units'][:-1]
def m_absent(e): e['center_kinds_absent'] = e['center_kinds_absent'] + ['CC-022.unit_assignment']
def m_wait(e): e['waiting_reasons'] = ['UNIT_NOT_COMPLETED']
out = {}
for name, f in [('lo-1', m_lo), ('lo+100', m_geo), ('reason', m_reason), ('offset', m_offset), ('awaiting', m_await), ('completed', m_done), ('absent', m_absent), ('waiting', m_wait)]:
    caught = []
    for n, exp in oracle['cuts'].items():
        e = copy.deepcopy(exp)
        try:
            f(e)
        except (IndexError, TypeError, KeyError):
            continue
        if e == exp:
            continue
        try:
            V.check(snap[n], e, case); caught.append((n, False))
        except ValueError:
            caught.append((n, True))
    out[name] = {'cuts_mutated': len(caught), 'all_caught': all(c for _, c in caught) and bool(caught)}
print(json.dumps(out, ensure_ascii=False))
print('ALL_MUTANTS_CAUGHT', all(v['all_caught'] for v in out.values()))
