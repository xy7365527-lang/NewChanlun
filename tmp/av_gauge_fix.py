"""Re-run COIN and AMC with fixed t8_summary handling."""
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
SYMBOLS = ['COIN', 'AMC']

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

        if 'Note' in data or 'Information' in data:
            msg = data.get('Note', data.get('Information', ''))
            print(f"  Rate limited: {msg[:80]}... waiting 60s")
            time.sleep(60)
            resp = requests.get(url, timeout=30)
            data = resp.json()

        ts = data.get('Time Series (1min)', {})
        if not ts:
            print(f"  No data for {sym}")
            results.append({'symbol': sym, 'error': 'no_data'})
            if i < len(SYMBOLS) - 1:
                time.sleep(13)
            continue

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

        merged, m2r = merge_inclusion(klines)
        fractals = fractals_from_merged(merged)
        strokes = strokes_from_fractals(merged, fractals, merged_to_raw=m2r)
        segments = segments_from_strokes_v1(strokes)
        levels = build_recursive_levels(segments)

        report = gauge_equivalence_report(klines)

        # Debug: print t8 structure
        t8_div = report.get('t8_divergence_topology', {})
        t8_summary = t8_div.get('summary', {})
        print(f"  t8_summary type: {type(t8_summary)}, value: {t8_summary}")

        overall_weighted = report.get('tier_weighted_rate', {}).get('overall_weighted', None)

        # Handle t8_summary being either dict or list
        if isinstance(t8_summary, dict):
            t8_total = t8_summary.get('total', 0)
            t8_w1_passed = t8_summary.get('w1_passed', 0)
            t8_macd_consensus = t8_summary.get('macd_w1_consensus', 0)
        elif isinstance(t8_summary, list):
            # list of per-segment results; aggregate
            t8_total = len(t8_summary)
            t8_w1_passed = sum(1 for x in t8_summary if isinstance(x, dict) and x.get('w1_passed', False))
            t8_macd_consensus = sum(1 for x in t8_summary if isinstance(x, dict) and x.get('macd_w1_consensus', False))
            print(f"  t8 list aggregated: total={t8_total}, w1_passed={t8_w1_passed}")
        else:
            t8_total = t8_w1_passed = t8_macd_consensus = 0

        strong_inv = report.get('strong_invariant_summary', {})
        n_centers_rate = strong_inv.get('n_centers', {}).get('rate', None) if isinstance(strong_inv, dict) else None
        trend_kinds_rate = strong_inv.get('trend_kinds', {}).get('rate', None) if isinstance(strong_inv, dict) else None
        beta1_tau_rate = strong_inv.get('beta1_tau', {}).get('rate', None) if isinstance(strong_inv, dict) else None

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

    if i < len(SYMBOLS) - 1:
        print(f"  Sleeping 13s ...")
        time.sleep(13)

output_path = 'tmp/av_gauge_results_fix.json'
with open(output_path, 'w', encoding='utf-8') as f:
    json.dump(results, f, ensure_ascii=False, indent=2, default=str)
print(f"\nResults saved to {output_path}")
