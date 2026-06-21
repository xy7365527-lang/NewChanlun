"""§1 理论上限：BTC 462万 closes ZigZag Σ|涨跌幅|（多阈值，含只做多对照）。"""
import numpy as np
c=np.load('/Users/silencehan/Projects/NewChanlun/analysis/data_cache/_btc_closes.npy').astype(float)
OUT='/Users/silencehan/Projects/NewChanlun/.chanlun/handoff/_theomax.txt'
O=[]
def p(s): O.append(str(s))
n=len(c)
p("n=%d first=%.2f last=%.2f BH=%.1f%%"%(n,c[0],c[-1],(c[-1]/c[0]-1)*100))

def zigzag(c, thr):
    """返回 (Σ|全段涨跌|%, Σ只涨段%, Σ只跌段%, 段数)。"""
    tot=up=dn=0.0; nseg=0
    piv=c[0]; ext=c[0]; direction=0
    for x in c:
        if x>ext: ext=x
        if x<ext and direction==0: ext=ext  # init
        if direction>=0:
            if x<=ext*(1-thr):
                r=ext/piv-1; tot+=abs(r);
                if r>0: up+=r
                else: dn+=-r
                nseg+=1; piv=ext; ext=x; direction=-1; continue
        if direction<=0:
            if x>=ext*(1+thr):
                r=ext/piv-1; tot+=abs(r)
                if r>0: up+=r
                else: dn+=-r
                nseg+=1; piv=ext; ext=x; direction=1; continue
        if direction<=0 and x<ext: ext=x
    r=ext/piv-1; tot+=abs(r)
    if r>0: up+=r
    else: dn+=-r
    nseg+=1
    return tot*100,up*100,dn*100,nseg

p("")
p("### S1 理论上限 Σ|涨跌幅|算术和(完美捕捉每段)")
p("thr | Σ|全段| | 只涨段Σ | 只跌段Σ | 段数 | 均段%")
for thr in [0.01,0.03,0.05,0.10,0.20]:
    t,u,dd,ns=zigzag(c,thr)
    p("%3.0f%% | %8.0f%% | %7.0f%% | %7.0f%% | %5d | %.1f%%"%(thr*100,t,u,dd,ns,t/ns))
p("对照 BH=%.0f%%  HEAD策略=+29.6%%  v3只做多=+901.7%%"%((c[-1]/c[0]-1)*100))
open(OUT,'w').write("\n".join(O))
print("OK",len(O))
