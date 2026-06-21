"""§1 理论上限：BTC 1min closes 上完美波段捕捉 Σ|涨跌幅|（多阈值 ZigZag）。
结构自检，输出到项目文件。"""
import json
OUT="/Users/silencehan/Projects/NewChanlun/.chanlun/handoff/_theomax.txt"
O=[]
def p(s): O.append(str(s))

d=json.load(open("analysis/data_cache/databento_btc_1min_continuous.json"))
p("list_len=%d"%len(d))
e0=d[0]
p("elem0_type=%s"%type(e0).__name__)
# 自检 close 提取
closes=None
if isinstance(e0,dict):
    p("elem_keys=%s"%list(e0.keys()))
    for ck in ["close","c","Close","CLOSE"]:
        if ck in e0:
            closes=[float(r[ck]) for r in d]; p("close_field=%s"%ck); break
elif isinstance(e0,list):
    p("elem0=%s"%str(e0)[:120])
    # 猜测 OHLC 顺序，close 常在 idx4(含时间) 或 idx3
    closes=[float(r[-1]) if not isinstance(r[-1],str) else float(r[-2]) for r in d]
    p("close_guess=last_numeric")
if closes is None:
    p("FAILED_close_extract"); open(OUT,"w").write("\n".join(O)); raise SystemExit

n=len(closes)
p("n_closes=%d first=%.2f last=%.2f"%(n,closes[0],closes[-1]))
p("BH=%.1f%%"%((closes[-1]/closes[0]-1)*100))
mn=min(closes); mx=max(closes)
p("min=%.2f max=%.2f"%(mn,mx))

# ZigZag: 多阈值 Σ|段涨跌幅|
def zigzag_sum(c, thr):
    # thr 比例阈值；记录每段（反转 thr）的 |涨跌幅|算术和
    total=0.0; nseg=0
    piv=c[0]; direction=0; ext=c[0]
    for x in c:
        if direction>=0 and x>ext: ext=x
        if direction<=0 and x<ext: ext=x
        if direction>=0:
            if x <= ext*(1-thr):  # 反转向下
                total+=abs(ext/piv-1); nseg+=1; piv=ext; ext=x; direction=-1
        if direction<=0:
            if x >= ext*(1+thr):  # 反转向上
                total+=abs(ext/piv-1); nseg+=1; piv=ext; ext=x; direction=1
    total+=abs(ext/piv-1); nseg+=1
    return total*100, nseg

p("")
p("### S1 理论上限 Σ|涨跌幅|(算术和, 完美捕捉每段)")
for thr in [0.01,0.03,0.05,0.10,0.20]:
    s,ns=zigzag_sum(closes,thr)
    p("  阈值%4.0f%%: Σ|涨跌|=%9.0f%% 段数=%6d 平均每段=%.1f%%"%(thr*100,s,ns,s/ns))
p("  对照: BH=%.0f%%  HEAD策略=+29.6%%"%((closes[-1]/closes[0]-1)*100))

open(OUT,"w").write("\n".join(O))
print("OK",len(O))
