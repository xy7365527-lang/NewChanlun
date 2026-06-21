#!/usr/bin/env python3
"""逐笔追踪一条衰减链：从 bar910416 lad4 long 15.92 shares 起，按时间往后追,
看 sink/reduce/recover/drain 每步拿走/归还多少 shares + notional 排除 price 干扰。"""
import json, numpy as np
BASE = "/Users/silencehan/Projects/NewChanlun/analysis"
closes = np.load(f"{BASE}/data_cache/_btc_closes.npy")
d=json.load(open(f"{BASE}/data_cache/t_engine_BTC_structural_trades.json"))
IX={n:i for i,n in enumerate(d['trade_schema'])}
g=lambda t,k:t[IX[k]]
tr=d['trades']
out={}

# 1) 该 seed 批次：entry_bar==910416 的所有 leg
seed=[t for t in tr if g(t,'entry_bar')==910416]
out['seed_910416'] = {
    'n_legs': len(seed),
    'entry_price': g(seed[0],'entry_price') if seed else None,
    'total_shares': round(sum(g(t,'shares') for t in seed),6),
    'ladders': sorted(set(g(t,'ladder') for t in seed)),
    'polarities': sorted(set(g(t,'polarity') for t in seed)),
    'legs': [[g(t,'ladder'),g(t,'exit_bar'),round(g(t,'entry_price'),2),round(g(t,'exit_price'),2),
              round(g(t,'shares'),8),g(t,'exit_reason'),g(t,'polarity')] for t in sorted(seed,key=lambda t:g(t,'exit_bar'))]
}

# 2) 时间序追踪：从 bar 910416 起，lad4 相关的所有 trades(entry或exit涉及lad4)按 exit_bar 排序前40笔
#    更通用：追踪“core”——用 lad>=4 的 long 仓位事件，按 exit_bar 排序
events=[t for t in tr if g(t,'exit_bar')>=910416 and g(t,'ladder')>=3]
events.sort(key=lambda t:(g(t,'exit_bar'), g(t,'ladder')))
chain=[]
for t in events[:60]:
    eb=g(t,'entry_bar'); xb=g(t,'exit_bar'); px=g(t,'exit_price'); sh=g(t,'shares')
    chain.append({
        'lad':g(t,'ladder'),'entry_bar':eb,'exit_bar':xb,
        'entry_price':round(g(t,'entry_price'),2),'exit_price':round(px,2),
        'shares':round(sh,8),'notional':round(sh*px,4),
        'reason':g(t,'exit_reason'),'pol':g(t,'polarity')})
out['chain_from_910416'] = chain

# 3) reduce/recover/drain 的 shares 取走量统计：是否=当前core的1/3？
#    无法直接知道“当前core”，但可看每个 reason 的 shares 相对其 batch 总额的比例
from collections import defaultdict
batches=defaultdict(list)
for t in tr:
    batches[(g(t,'ladder'),g(t,'entry_bar'),g(t,'polarity'))].append(t)
ratio_stats=defaultdict(list)
for key,ts in batches.items():
    ts=sorted(ts,key=lambda t:g(t,'exit_bar'))
    tot=sum(g(t,'shares') for t in ts)
    if tot<=0: continue
    cum=0
    remaining=tot
    for t in ts:
        sh=g(t,'shares')
        frac_of_remaining = sh/remaining if remaining>1e-12 else None
        if frac_of_remaining is not None:
            ratio_stats[g(t,'exit_reason')].append(frac_of_remaining)
        remaining-=sh
out['reduce_recover_drain_frac_of_remaining'] = {
    k:{'n':len(v),'median':round(float(np.median(v)),4),
       'mean':round(float(np.mean(v)),4),
       'p25':round(float(np.percentile(v,25)),4),
       'p75':round(float(np.percentile(v,75)),4)} for k,v in ratio_stats.items() if v}

# 4) notional 排除：seed 批次的总 notional vs 后续衰减期 notional（用各期 entry notional）
#    取 lad4 long 各时间窗的 max notional(=该期满仓资金规模)
def winmax_notional(lo,hi):
    vals=[]
    for t in tr:
        if lo<=g(t,'entry_bar')<hi:
            vals.append(g(t,'shares')*g(t,'entry_price'))
    return max(vals) if vals else 0
wins=[(910416,950000),(950000,1000000),(1000000,1100000),(1100000,1300000),
      (1300000,1500000),(1500000,1806004),(1806004,2000000),(2000000,2218375),
      (2218375,2400000),(2400000,2627490)]
out['notional_decay'] = []
for lo,hi in wins:
    mid=(lo+hi)//2
    px=float(closes[mid]) if mid<len(closes) else 0
    mn=winmax_notional(lo,hi)
    # 同期 max shares
    shs=[g(t,'shares') for t in tr if lo<=g(t,'entry_bar')<hi]
    out['notional_decay'].append({
        'win':[lo,hi],'mid_price':round(px,0),
        'max_shares':round(max(shs),8) if shs else 0,
        'max_notional':round(mn,2)})

json.dump(out, open(f"{BASE}/_btc_decay_chain.json",'w'), indent=2, default=str)
print("OK legs=",out['seed_910416']['n_legs']," chain=",len(chain))
