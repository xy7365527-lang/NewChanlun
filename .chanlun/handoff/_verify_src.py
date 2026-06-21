"""验证 btc_1m_full.json 是否=引擎序列(_btc_closes.npy),并探测格式。"""
import json, numpy as np
OUT='/Users/silencehan/Projects/NewChanlun/.chanlun/handoff/_srccheck.txt'
O=[]
def p(s): O.append(str(s))
ref=np.load('/Users/silencehan/Projects/NewChanlun/analysis/data_cache/_btc_closes.npy').astype(float)
p("ref n=%d first=%.2f last=%.2f"%(len(ref),ref[0],ref[-1]))
d=json.load(open('analysis/data_cache/btc_1m_full.json'))
p("type=%s"%type(d).__name__)
if isinstance(d,dict):
    p("keys=%s"%list(d.keys()))
    cl=d.get('closes') or d.get('close')
    if cl:
        p("closes n=%d first=%.2f last=%.2f"%(len(cl),cl[0],cl[-1]))
        p("match_len=%s match_first=%s match_last=%s"%(len(cl)==len(ref),abs(cl[0]-ref[0])<0.01,abs(cl[-1]-ref[-1])<0.01))
        mid=len(cl)//2
        p("mid match=%s"%(abs(cl[mid]-ref[mid])<0.01))
elif isinstance(d,list):
    p("list_len=%d elem0=%s"%(len(d),str(d[0])[:100]))
open(OUT,'w').write("\n".join(O)); print("OK")
