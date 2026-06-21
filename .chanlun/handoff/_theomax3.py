"""§1 理论上限：正确 ZigZag（单向极值追踪）Σ|涨跌幅|。"""
import numpy as np
c=np.load('/Users/silencehan/Projects/NewChanlun/analysis/data_cache/_btc_closes.npy').astype(float)
OUT='/Users/silencehan/Projects/NewChanlun/.chanlun/handoff/_theomax.txt'
O=[]
def p(s): O.append(str(s))
n=len(c)
p("n=%d first=%.2f last=%.2f BH=%.1f%%"%(n,c[0],c[-1],(c[-1]/c[0]-1)*100))

def zigzag(c, thr):
    tot=up=dn=0.0; nseg=0
    piv=c[0]; ext=c[0]; d=1  # 起始假设向上追踪高点
    for x in c:
        if d>0:
            if x>ext: ext=x
            elif x<=ext*(1-thr):
                r=ext/piv-1.0; tot+=abs(r); up+=r; nseg+=1
                piv=ext; ext=x; d=-1
        else:
            if x<ext: ext=x
            elif x>=ext*(1+thr):
                r=ext/piv-1.0; tot+=abs(r); dn+=-r; nseg+=1
                piv=ext; ext=x; d=1
    r=ext/piv-1.0; tot+=abs(r)
    if r>0: up+=r
    else: dn+=-r
    nseg+=1
    return tot*100,up*100,dn*100,nseg

p("")
p("### S1 理论上限 Σ|涨跌幅|算术和(完美捕捉每段)")
p("thr | Sigma|all| | up_only | dn_only | nseg | avg%")
for thr in [0.01,0.03,0.05,0.10,0.20]:
    t,u,dd,ns=zigzag(c,thr)
    p("%3.0f%% | %9.0f%% | %8.0f%% | %7.0f%% | %6d | %.1f%%"%(thr*100,t,u,dd,ns,t/ns))
p("BH=%.0f%%  HEAD=+29.6%%  v3longonly=+901.7%%"%((c[-1]/c[0]-1)*100))
open(OUT,'w').write("\n".join(O))
print("OK")
