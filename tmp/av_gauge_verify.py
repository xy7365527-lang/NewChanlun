"""Alpha Vantage 1min gauge equivalence verification for 20 US stocks."""
import sys, time, json, traceback
sys.path.insert(0, 'src')

import requests
import pandas as pd
from newchan.a_inclusion import merge_inclusion
from newchan.a_fractal import fractals_from_merged
from newchan.a_stroke import strokes_from_fractals
from newchan.a_segment_v1 import segments_from_strokes_v1
from newchan.a_recursive_engine import build_recursive_levels
from newchan.a_topology import gauge_equivalence_report

API_KEY = 'WP4GIQ6VALD179P3'
SYMBOLS = [
    'AAPL', 'MSFT', 'GOOGL', 'AMZN', 'META',
    'NVDA', 'AMD', 'TSLA', 'MARA', 'COIN',
    'GME', 'AMC', 'PLTR', 'SOFI', 'RIVN',
    'QQQ', 'IWM', 'XLF', 'GLD', 'SPY',
]

results = []

for i, sym in enumerate(SYMBOLS):
    print(f"\n[{i+1}/{len(SYMBOLS)}] Fetching {sym} ...")
    url = (
        f'https://www.alphavantage.co/query?function=TIME_SERIES_INTRADAY'
        f'&symbol={sym}&interval=1min&outputsize=full'
        f'&apikey={API_KEY}&datatype=json'
    )
    try:
        resp = requests.get(url, timeout=30)
        data = resp.json()

        # Rate limit check
        if 'Note' in data or 'Information' in data:
            msg = data.get('Note', data.get('Information', ''))
            print(f"  Rate limited: {msg[:80]}... waiting 5s")
            time.sleep(5)
            resp = requests.get(url, timeout=30)
            data = resp.json()
            if 'Note' in data or 'Information' in data:
                print(f"  Still rate limited, skipping {sym}")
                results.append({'symbol': sym, 'error': 'rate_limited'})
                continue

        ts = data.get('Time Series (1min)', {})
        if not ts:
            print(f"  No data for {sym}, keys: {list(data.keys())[:5]}")
            results.append({'symbol': sym, 'error': 'no_data'})
            continue

        # Build DataFrame
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
        n_bars = len(klines)
        print(f"  Got {n_bars} bars")

        # Pipeline
        merged, m2r = merge_inclusion(klines)
        fractals = fractals_from_merged(merged)
        strokes = strokes_from_fractals(merged, fractals, merged_to_raw=m2r)
        segments = segments_from_strokes_v1(strokes)
        levels = build_recursive_levels(segments)

        # Gauge report
        report = gauge_equivalence_report(klines)

        # Extract metrics
        overall_weighted = report.get('tier_weighted_rate', {}).get('overall_weighted', None)

        t8_raw = report.get('t8_divergence_topology', {})
        t8_summary = t8_raw.get('summary', {}) if isinstance(t8_raw, dict) else {}
        t8_total = t8_summary.get('total', 0)
        t8_w1_passed = t8_summary.get('w1_passed', 0)
        t8_macd_consensus = t8_summary.get('macd_w1_consensus', 0)

        strong_inv = report.get('strong_invariant_summary', {})
        n_centers_rate = strong_inv.get('n_centers', {}).get('rate', None)
        trend_kinds_rate = strong_inv.get('trend_kinds', {}).get('rate', None)
        beta1_tau_rate = strong_inv.get('beta1_tau', {}).get('rate', None)

        # Recursive depth
        rec_depth = len(levels)
        level_details = []
        for lv_idx, lv in enumerate(levels):
            segs = lv.get('segments', lv) if isinstance(lv, dict) else lv
            seg_count = len(segs) if hasattr(segs, '__len__') else 0
            level_details.append(f"L{lv_idx}: {seg_count} segs")

        rec_info = f"depth={rec_depth}"
        if level_details:
            rec_info += f" ({', '.join(level_details[:4])})"

        entry = {
            'symbol': sym,
            'n_bars': n_bars,
            'n_strokes': len(strokes),
            'n_segments': len(segments),
            'overall_weighted': overall_weighted,
            't8_total': t8_total,
            't8_w1_passed': t8_w1_passed,
            't8_macd_consensus': t8_macd_consensus,
            'n_centers_rate': n_centers_rate,
            'trend_kinds_rate': trend_kinds_rate,
            'beta1_tau_rate': beta1_tau_rate,
            'rec_depth': rec_depth,
            'rec_info': rec_info,
            'error': None,
        }
        results.append(entry)
        print(f"  overall_weighted={overall_weighted}, t8={t8_w1_passed}/{t8_total}, depth={rec_depth}")

    except Exception as e:
        tb = traceback.format_exc()
        print(f"  ERROR: {e}\n{tb}")
        results.append({'symbol': sym, 'error': str(e)})

    # Rate limit: 600/min tier → ~0.1s per call, use 0.5s buffer
    if i < len(SYMBOLS) - 1:
        time.sleep(0.5)

# Dump results as JSON for downstream processing
output_path = 'tmp/av_gauge_results.json'
with open(output_path, 'w', encoding='utf-8') as f:
    json.dump(results, f, ensure_ascii=False, indent=2, default=str)
print(f"\nResults saved to {output_path}")
print(f"Total: {len(results)} symbols, {sum(1 for r in results if not r.get('error'))} success, {sum(1 for r in results if r.get('error'))} errors")
