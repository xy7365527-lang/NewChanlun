import json
d=json.load(open('/home/hanju/nc-remote/data/btc_1m_full.json'))
O,H,L,C,V,T=(d[k] for k in ['opens','highs','lows','closes','volumes','dates'])
n=len(H); print('n',n,'symbol',d['symbol'],'T0',T[0],'T1',T[1])
def cont(i,j): return (H[i]>=H[j] and L[i]<=L[j]) or (H[j]>=H[i] and L[j]<=L[i])
def sd(i,j):
  if H[j]>H[i] and L[j]>L[i]: return 'U'
  if H[j]<H[i] and L[j]<L[i]: return 'D'
  return None
for i in range(0,141):
  print(i,T[i],O[i],H[i],L[i],C[i],V[i],'inc' if i+1<n and cont(i,i+1) else sd(i,i+1))
# first strict pair
fs=next(i for i in range(n-1) if not cont(i,i+1))
print('first non-inclusive adjacent pair at',fs)
for W in [4000005,4400005,4600005,4610005,4613000,4613500,4613590,4613599]:
  s=n-W+5; print('W',W,'start',s,'first pair inclusive',cont(s,s+1))
# dup timestamps / monotonic / anomalies in first 200
import collections
print('dup ts in first 200', [t for t,c in collections.Counter(T[:200]).items() if c>1][:5])
from datetime import datetime as _D
print('nonmono first 200', sum(1 for i in range(199) if T[i+1]<=T[i]), 'non-60s gaps first 200', sum(1 for i in range(199) if (_D.fromisoformat(T[i+1])-_D.fromisoformat(T[i])).total_seconds()!=60))
print('zero vol first 104', sum(1 for i in range(104) if V[i]==0), 'flat h==l first 104', sum(1 for i in range(104) if H[i]==L[i]))
