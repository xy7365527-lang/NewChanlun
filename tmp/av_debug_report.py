"""Debug: inspect gauge_equivalence_report structure for COIN."""
import sys, json
sys.path.insert(0, 'src')

import requests
import pandas as pd
from newchan.a_topology import gauge_equivalence_report

API_KEY = 'WP4GIQ6VALD179P3'
url = (
    f'https://www.alphavantage.co/query?function=TIME_SERIES_INTRADAY'
    f'&symbol=COIN&interval=1min&outputsize=full'
    f'&apikey={API_KEY}&datatype=json'
)
resp = requests.get(url, timeout=30)
data = resp.json()
ts = data.get('Time Series (1min)', {})
rows = []
for dt_str, vals in sorted(ts.items()):
    rows.append({
        'dt': pd.Timestamp(dt_str),
        'open': float(vals['1. open']),
        'high': float(vals['2. high']),
        'low': float(vals['3. low']),
        'close': float(vals['4. close']),
    })
klines = pd.DataFrame(rows)
print(f"bars: {len(klines)}")

report = gauge_equivalence_report(klines)
print(f"\nreport keys: {list(report.keys())}")
for k, v in report.items():
    print(f"\n--- {k} ---")
    print(f"  type: {type(v)}")
    if isinstance(v, dict):
        for k2, v2 in v.items():
            print(f"  {k2}: type={type(v2).__name__}, val={str(v2)[:200]}")
    elif isinstance(v, list):
        print(f"  len={len(v)}")
        if v:
            print(f"  [0] type={type(v[0])}, val={str(v[0])[:200]}")
    else:
        print(f"  val={str(v)[:200]}")
