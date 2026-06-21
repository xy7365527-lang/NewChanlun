#!/usr/bin/env python3
"""BTC 69k->15k 下跌段：核心空头 units 逐笔追踪。用 /usr/bin/python3 (numpy 2.0.2)。"""
import json, numpy as np
from collections import defaultdict, Counter

BASE = "/Users/silencehan/Projects/NewChanlun/analysis"
CLOSES = f"{BASE}/data_cache/_btc_closes.npy"
TRADES = f"{BASE}/data_cache/t_engine_BTC_structural_trades.json"
OUT = f"{BASE}/_btc_dseg_report.json"

rep = {}

# 1) 加载 closes
closes = np.load(CLOSES)
n = len(closes)
rep['closes'] = {'len': int(n), 'min': round(float(closes.min()),2), 'max': round(float(closes.max()),2)}

# 2) 加载 trades
with open(TRADES) as f:
    tdata = json.load(f)
schema = tdata['trade_schema']
trades = tdata['trades']
IX = {nm:i for i,nm in enumerate(schema)}
def g(t,k): return t[IX[k]]
rep['meta'] = {'n_bars': tdata['n_bars'], 'final_nav': tdata['final_nav'],
               'n_trades': tdata['n_trades'], 'schema': schema}

# 3) 交叉验证 npy 与 trades 同源：取若干 trade 的 entry_bar，比对 closes[entry_bar] vs entry_price
checks = []
for t in trades[:8]:
    eb = g(t,'entry_bar'); ep = g(t,'entry_price')
    cb = float(closes[eb]) if eb < n else None
    checks.append({'entry_bar': eb, 'entry_price': round(ep,2),
                   'closes[entry_bar]': round(cb,2) if cb else None,
                   'match': (abs(cb-ep) < max(1.0, ep*0.02)) if cb else None})
rep['npy_trade_crosscheck'] = checks

# 4) 定位 69k 顶 / 15k 底
above = np.where(closes >= 68500)[0]
top_bar = int(above[0])
after = above[above > top_bar + 100000]
next_68500 = int(after[0]) if len(after) else n
bottom_bar = top_bar + int(np.argmin(closes[top_bar:next_68500]))
tlo = max(0, top_bar-50000)
top_max_bar = tlo + int(np.argmax(closes[tlo:top_bar+50000]))
SEG_LO, SEG_HI = top_max_bar, bottom_bar
rep['segment'] = {
    'top_bar': top_max_bar, 'top_close': round(float(closes[top_max_bar]),1),
    'bottom_bar': bottom_bar, 'bottom_close': round(float(closes[bottom_bar]),1),
    'span_bars': bottom_bar - top_max_bar,
    'next_68500_bar': next_68500,
}

# 5) pnl
def pnl(t):
    ep,xp,sh = g(t,'entry_price'),g(t,'exit_price'),g(t,'shares')
    return (xp-ep)*sh if g(t,'polarity')=='long' else (ep-xp)*sh

# 6) 仓位批次：同一次建仓被多次平仓 -> 共享 (ladder,entry_bar,entry_price,polarity)
batches = defaultdict(list)
for t in trades:
    key = (g(t,'ladder'), g(t,'entry_bar'), round(g(t,'entry_price'),4), g(t,'polarity'))
    batches[key].append(t)

def summ(key, ts):
    lad,eb,ep,pol = key
    ts = sorted(ts, key=lambda t: g(t,'exit_bar'))
    legs = [{'exit_bar':g(t,'exit_bar'),'exit_price':round(g(t,'exit_price'),2),
             'shares':round(g(t,'shares'),4),'reason':g(t,'exit_reason'),
             'partial':g(t,'partial'),'pnl':round(pnl(t),2),
             'short_win': (g(t,'exit_price')<ep) if pol=='short' else None} for t in ts]
    return {'ladder':lad,'entry_bar':eb,'entry_price':round(ep,2),'polarity':pol,
            'n_legs':len(ts),'total_shares':round(sum(g(t,'shares') for t in ts),4),
            'last_exit_bar':g(ts[-1],'exit_bar'),'batch_pnl':round(sum(pnl(t) for t in ts),2),
            'legs':legs}

short_b, long_b = [], []
for key,ts in batches.items():
    lad,eb,ep,pol = key
    last_exit = max(g(t,'exit_bar') for t in ts)
    if eb <= SEG_HI and last_exit >= SEG_LO:   # 生命周期与下跌段交集
        (short_b if pol=='short' else long_b).append(summ(key,ts))
short_b.sort(key=lambda b:b['entry_bar'])
long_b.sort(key=lambda b:b['entry_bar'])

# 段内多/空盈亏（entry 在段内）
seg_short = [b for b in short_b if SEG_LO<=b['entry_bar']<=SEG_HI]
seg_long  = [b for b in long_b  if SEG_LO<=b['entry_bar']<=SEG_HI]
rep['exit_reason_global'] = dict(Counter(g(t,'exit_reason') for t in trades))
rep['summary'] = {
    'short_batches_touch_seg': len(short_b),
    'long_batches_touch_seg': len(long_b),
    'seg_short_batches': len(seg_short),
    'seg_long_batches': len(seg_long),
    'seg_short_pnl': round(sum(b['batch_pnl'] for b in seg_short),2),
    'seg_long_pnl': round(sum(b['batch_pnl'] for b in seg_long),2),
    'short_touch_pnl': round(sum(b['batch_pnl'] for b in short_b),2),
    'long_touch_pnl': round(sum(b['batch_pnl'] for b in long_b),2),
}

# 全程持有 69k->15k 的空头：建仓接近顶、平仓接近底
held = [b for b in short_b if b['entry_bar']<=SEG_LO+80000 and b['last_exit_bar']>=SEG_HI-80000]
rep['held_full_short'] = held

# 核心空头候选：与段交集的 short，按 total_shares 排序 top
rep['top_short_by_shares'] = sorted(short_b, key=lambda b:-b['total_shares'])[:12]
# 全部段交集 short（用于逐笔）
rep['all_short_touch'] = short_b

json.dump(rep, open(OUT,'w'), indent=2, default=str)
print("OK short_touch", len(short_b), "long_touch", len(long_b),
      "seg_short", len(seg_short), "seg_long", len(seg_long), "held", len(held))
