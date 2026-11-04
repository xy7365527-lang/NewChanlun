import json
d=json.load(open('/home/hanju/nc-remote/data/btc_1m_full.json'))
H=[round(x/1e-8) if x is not None else None for x in d['highs']]
L=[round(x/1e-8) if x is not None else None for x in d['lows']]
print('nulls', sum(1 for x in H if x is None), sum(1 for x in L if x is None))
def cont(a,b): return (a[0]>=b[0] and a[1]<=b[1]) or (b[0]>=a[0] and b[1]<=a[1])
def sd(p,c):
  if c[0]>p[0] and c[1]>p[1]: return 1
  if c[0]<p[0] and c[1]<p[1]: return -1
  return 0
def fold(bars):
  # baohan.md: first adjacent raw strict non-inclusive pair -> dir; fold from 0
  d0=None
  for i in range(len(bars)-1):
    if not cont(bars[i],bars[i+1]):
      d0=sd(bars[i],bars[i+1]); break
  if d0 is None: return None
  # 组锚 = 组内首根 raw 序号（baohan.md「组锚与序号关系」）
  out=[]; acc=bars[0]; anc=0; dr=d0
  for k,b in enumerate(bars[1:],1):
    if cont(acc,b):
      acc=(max(acc[0],b[0]),max(acc[1],b[1])) if dr==1 else (min(acc[0],b[0]),min(acc[1],b[1]))
    else:
      dr=sd(acc,b); out.append((anc,)+acc); acc=b; anc=k
  out.append((anc,)+acc); return out
n=len(H)
for s in [0,5,104]:
  bars=list(zip(H[s:],L[s:]))
  m=fold(bars); print('start',s,'merged',len(m))
fx=list(zip(H[5:141],L[5:141]))
m=[(a+5,h,l) for a,h,l in fold(fx)]; print('fixture136 merged',len(m)); print('FIX',fx); print('EXP',m)
