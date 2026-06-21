"""§4按多空四象限 + §5最大单笔（自洽HEAD structural trades）。"""
import json
from collections import defaultdict
t=json.load(open('analysis/data_cache/t_engine_BTC_structural_trades.json'))
tr=t['trades']
OUT='/Users/silencehan/Projects/NewChanlun/.chanlun/handoff/_s45.txt'
O=[]; p=lambda s:O.append(str(s))

def pnl(x):
    lad,eb,ep,xb,xp,sh,w,dfr,part,reason,pol=x[:11]
    return sh*(xp-ep) if pol=='long' else sh*(ep-xp)

tot=sum(pnl(x) for x in tr)
p("### 对账: Σ逐笔pnl=%.0f vs (final_nav-10万)=%.0f 差=%.0f"%(tot,t['final_nav']-1e5,tot-(t['final_nav']-1e5)))

# §4 按多空 × 盈亏符号
q=defaultdict(lambda:[0,0.0])  # (pol, sign)
byp=defaultdict(lambda:[0,0.0])
byreason=defaultdict(lambda:[0,0.0])
for x in tr:
    pol=x[10]; pn=pnl(x); reason=x[9]
    byp[pol][0]+=1; byp[pol][1]+=pn
    sign='赚' if pn>0 else '亏'
    q[(pol,sign)][0]+=1; q[(pol,sign)][1]+=pn
    byreason[(pol,reason)][0]+=1; byreason[(pol,reason)][1]+=pn
p("")
p("### S4 多空总账")
for pol,(n,c) in sorted(byp.items()): p("  %-5s n=%-4d cash=%+.0f"%(pol,n,c))
p("### S4 四象限(多空×盈亏)")
for (pol,sg),(n,c) in sorted(q.items()): p("  %-5s/%s n=%-4d cash=%+.0f"%(pol,sg,n,c))
p("### S4 按(多空,reason)")
for (pol,r),(n,c) in sorted(byreason.items(),key=lambda x:x[1][1]): p("  %-5s/%-14s n=%-4d cash=%+.0f"%(pol,r,n,c))

# §5 最大10笔亏损
p("")
p("### S5 最大10笔亏损")
for x in sorted(tr,key=pnl)[:10]:
    lad,eb,ep,xb,xp,sh,w,dfr,part,reason,pol=x[:11]
    org=x[11] if len(x)>11 else '?'
    p("  L%d %-5s %-13s b%d($%.0f)->b%d($%.0f) sh=%.3f pnl=%+.0f org=%s"%(lad-3,pol,reason,eb,ep,xb,xp,sh,pnl(x),org))
# §5 最大10笔盈利(对照)
p("### S5b 最大10笔盈利")
for x in sorted(tr,key=pnl,reverse=True)[:10]:
    lad,eb,ep,xb,xp,sh,w,dfr,part,reason,pol=x[:11]
    p("  L%d %-5s %-13s b%d->b%d pnl=%+.0f"%(lad-3,pol,reason,eb,xb,pnl(x)))
open(OUT,'w').write("\n".join(O)); print("OK",len(O))
