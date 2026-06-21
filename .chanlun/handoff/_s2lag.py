"""§2 确认滞后:每笔建仓entry_bar vs 最近理想ZigZag转折点 的bar滞后+价格滑移。
口径: long理想=谷底(down->up转折), short理想=顶部(up->down转折)。
基准用5%阈值ZigZag(move级别尺度)。L3:依赖阈值选择。"""
import json, numpy as np
c=np.load('/Users/silencehan/Projects/NewChanlun/analysis/data_cache/_btc_closes.npy').astype(float)
t=json.load(open('analysis/data_cache/t_engine_BTC_structural_trades.json'))
tr=t['trades']
OUT='/Users/silencehan/Projects/NewChanlun/.chanlun/handoff/_s2.txt'
O=[]; p=lambda s:O.append(str(s))

def pivots(c, thr):
    """返回转折点 [(bar, price, kind)], kind='trough'(谷,下->上) / 'peak'(顶,上->下)。"""
    piv=[]; ext=c[0]; exti=0; d=1
    for i,x in enumerate(c):
        if d>0:
            if x>ext: ext=x; exti=i
            elif x<=ext*(1-thr): piv.append((exti,ext,'peak')); ext=x; exti=i; d=-1
        else:
            if x<ext: ext=x; exti=i
            elif x>=ext*(1+thr): piv.append((exti,ext,'trough')); ext=x; exti=i; d=1
    return piv

for THR in [0.05,0.03]:
    pv=pivots(c,THR)
    troughs=[(b,pr) for b,pr,k in pv if k=='trough']
    peaks=[(b,pr) for b,pr,k in pv if k=='peak']
    tb=np.array([b for b,_ in troughs]); tp=np.array([pr for _,pr in troughs])
    pb=np.array([b for b,_ in peaks]); pp=np.array([pr for _,pr in peaks])
    p("=== 阈值%.0f%% 理想转折: 谷%d 顶%d ==="%(THR*100,len(troughs),len(peaks)))
    lags_l=[]; slip_l=[]; lags_s=[]; slip_s=[]
    for x in tr:
        eb=x[1]; ep=x[2]; pol=x[10]
        if pol=='long' and len(tb):
            j=np.searchsorted(tb,eb)-1   # 最近的前一个谷底
            if j>=0:
                lags_l.append(eb-tb[j]); slip_l.append((ep-tp[j])/tp[j]*100)
        elif pol=='short' and len(pb):
            j=np.searchsorted(pb,eb)-1   # 最近的前一个顶
            if j>=0:
                lags_s.append(eb-pb[j]); slip_s.append((pp[j]-ep)/ep*100)
    import statistics as st
    def desc(a): return "n=%d 中位lag=%.0fbar 均lag=%.0f 中位滑移=%.1f%% 均滑移=%.1f%%"%(
        len(a[0]),st.median(a[0]) if a[0] else 0,sum(a[0])/len(a[0]) if a[0] else 0,
        st.median(a[1]) if a[1] else 0,sum(a[1])/len(a[1]) if a[1] else 0)
    p("  long建仓 vs 谷底: "+desc((lags_l,slip_l)))
    p("  short建仓vs 顶部: "+desc((lags_s,slip_s)))
    p("")
open(OUT,'w').write("\n".join(O)); print("OK",len(O))
