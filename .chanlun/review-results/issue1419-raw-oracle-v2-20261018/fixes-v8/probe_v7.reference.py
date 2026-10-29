"""raw-v7 独立审查反例探针（只读源，离线纯 Python）。用法：python3 probe_v7.py <包根>"""
import sys, copy, json
from pathlib import Path
ROOT=Path(sys.argv[1]).resolve()
sys.path[:0]=[str(ROOT/'fixes-v6'),str(ROOT/'driver'),str(ROOT/'fixes-v5'),str(ROOT/'fixes-v4')]
import test_v6 as t
from unittest.mock import patch
patch.object(Path,'write_bytes',return_value=0).start()
R={}
def run(name, fn):
    try: out=fn(); R[name]=dict(result='ACCEPTED', detail=out)
    except Exception as e: R[name]=dict(result='REJECTED', error=f'{type(e).__name__}: {e}')

def tamper_to(newto, gen_idx=0, also_cut=True):
    def f():
        c,ex,cursor,pages,b=t.ready()
        p=copy.deepcopy(pages[gen_idx]); p['deltas'][0]['seq_range']['to']=newto
        b2=copy.deepcopy(b)
        if also_cut: b2[p['deltas'][0]['generation']]['accepted_cut']['seq_range']['to']=newto
        p['observed_head']['seq_range']['to']=newto
        ex.pages=[p]; out=c.watch(cursor,140,'client',accepted_commits=b2)
        return dict(last_seq=out['next_cursor']['last_seq'], seq_range=out['deltas'][0]['seq_range'], input_frontier=out['deltas'][0]['input_frontier'])
    return f
# P1: seq_range.to != input_frontier（q3 validateDelta:1566 与 fixed:1969 拒）
run('P1_seq_range_to_inflated_999', tamper_to('999'))
run('P1b_seq_range_to_minus1', tamper_to('-1'))
# P2: 刚 bind_watch 后（after=139）追平空页，观测头同代但 input_frontier/ structure_cut 变脸
def empty_after_bind(mut):
    def f():
        c,ex,cursor,pages,b=t.ready()
        e=copy.deepcopy(pages[0]); e['deltas']=[]; e['next_cursor']=copy.deepcopy(cursor)
        fixed=t.watch_material()[0][0]['fixed_cut']
        e['observed_head']=dict(copy.deepcopy(fixed),history_mode='RecomputedWithRevision',as_of_generation=None)
        e['delivery_frontier']['head_generation']=e['observed_head']['cut_generation']
        mut(e['observed_head'])
        ex.pages=[e]; st=copy.deepcopy(c.session_state['watch'])
        out=c.watch(cursor,142,'client',max_batches=3,accepted_commits=b)
        return dict(deltas=len(out['deltas']), unchanged=c.session_state['watch']==st)
    return f
run('P2a_caught_up_empty_after_bind_honest', empty_after_bind(lambda h: None))
run('P2b_caught_up_empty_after_bind_structure_drift', empty_after_bind(lambda h: h.update(structure_cut='alien')))
run('P2c_caught_up_empty_after_bind_frontier_drift', empty_after_bind(lambda h: h.update(input_frontier='150',seq_range={'from':'138','to':'150'})))
# P3: 追平空页但 next_cursor 被推进（Gap 外不准动游标）
def empty_cursor_moved():
    c,ex,cursor,pages,b=t.ready(); ex.pages=[copy.deepcopy(pages[0])]
    cur=c.watch(cursor,140,'client',accepted_commits=b)['next_cursor']
    e=copy.deepcopy(pages[0]); e['deltas']=[]; nc=copy.deepcopy(cur); nc['after_generation']='141'; e['next_cursor']=nc
    ex.pages=[e]; c.watch(cur,142,'client',accepted_commits=b)
run('P3_empty_page_cursor_advanced', empty_cursor_moved)
# P4: seq_range.from 偏移（回归 R6-1，多批第二代）
def from_off():
    c,ex,cursor,pages,b=t.ready(); ex.pages=[copy.deepcopy(pages[0])]
    cur=c.watch(cursor,140,'client',accepted_commits=b)['next_cursor']
    p=copy.deepcopy(pages[1]); p['deltas'][0]['seq_range']['from']='141'
    b2=copy.deepcopy(b); b2['141']['accepted_cut']['seq_range']['from']='141'; p['observed_head']['seq_range']['from']='141'
    ex.pages=[p]; c.watch(cur,141,'client',accepted_commits=b2)
run('P4_from_off_by_one_gen141', from_off)
# P5: 多批调用中先非空后追平空页 → 止于已到代
def nonempty_then_empty():
    c,ex,cursor,pages,b=t.ready()
    e=copy.deepcopy(pages[0]); e['deltas']=[]
    ex.pages=[copy.deepcopy(pages[0]), e]
    out=c.watch(cursor,145,'client',max_batches=1,accepted_commits=b)
    return dict(n=len(out['deltas']), after=out['next_cursor']['after_generation'], calls=ex.calls)
run('P5_nonempty_then_caught_up_empty', nonempty_then_empty)
print(json.dumps(R,ensure_ascii=False,indent=1))
