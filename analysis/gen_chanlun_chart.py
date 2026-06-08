"""生成缠论递归可视化 HTML（lightweight-charts）。

读取 run_databento_recursive.py 输出的 JSON，生成独立交互式 HTML：
- K 线图
- 笔（蓝色折线）
- 线段（橙色粗线）
- 中枢框（半透明矩形）
- 买卖点标注（marker）
- 每个递归层级一个 tab

用法：
  PYTHONPATH=src python analysis/gen_chanlun_chart.py --symbol QQQ
  PYTHONPATH=src python analysis/gen_chanlun_chart.py  # 全部
"""

from __future__ import annotations

import argparse
import json
from pathlib import Path

DATA_DIR = Path(__file__).resolve().parent / "data_cache"
OUT_DIR = Path(__file__).resolve().parent / "charts"

SYMBOLS = ["QQQ", "OKLO", "HK700"]


def _ts_to_epoch(ts_str: str) -> int:
    from datetime import datetime, timezone
    for fmt in (
        "%Y-%m-%dT%H:%M:%S%z",
        "%Y-%m-%dT%H:%M:%S",
        "%Y-%m-%d %H:%M:%S%z",
        "%Y-%m-%d %H:%M:%S",
    ):
        try:
            dt = datetime.strptime(ts_str, fmt)
            return int(dt.timestamp())
        except ValueError:
            continue
    return 0


def _downsample_ohlc(ohlc: list[dict], max_bars: int = 80000) -> list[dict]:
    if len(ohlc) <= max_bars:
        return ohlc
    step = len(ohlc) // max_bars
    result = []
    for i in range(0, len(ohlc), step):
        chunk = ohlc[i:i + step]
        result.append({
            "t": chunk[0]["t"],
            "o": chunk[0]["o"],
            "h": max(c["h"] for c in chunk),
            "l": min(c["l"] for c in chunk),
            "c": chunk[-1]["c"],
        })
    return result


def build_html(data: dict, symbol: str) -> str:
    ohlc_raw = data["ohlc"]
    l1 = data["L1"]
    recursive = data.get("recursive", {})

    ohlc = _downsample_ohlc(ohlc_raw)
    ohlc_js = json.dumps([
        {"time": _ts_to_epoch(b["t"]), "open": b["o"], "high": b["h"], "low": b["l"], "close": b["c"]}
        for b in ohlc
    ])

    strokes = l1["strokes"]
    stroke_lines_js = json.dumps([
        {"time": _ts_to_epoch(s["t0"]), "value": s["p0"]}
        for s in strokes
    ] + ([{"time": _ts_to_epoch(strokes[-1]["t1"]), "value": strokes[-1]["p1"]}] if strokes else []))

    stroke_points = []
    for s in strokes:
        stroke_points.append({"time": _ts_to_epoch(s["t0"]), "value": s["p0"]})
        stroke_points.append({"time": _ts_to_epoch(s["t1"]), "value": s["p1"]})
    seen = set()
    deduped_strokes = []
    for pt in stroke_points:
        key = (pt["time"], pt["value"])
        if key not in seen:
            seen.add(key)
            deduped_strokes.append(pt)
    deduped_strokes.sort(key=lambda x: x["time"])
    stroke_lines_js = json.dumps(deduped_strokes)

    segments = l1["segments"]
    seg_points = []
    for sg in segments:
        seg_points.append({"time": _ts_to_epoch(sg["t0"]), "value": sg["p0"]})
        seg_points.append({"time": _ts_to_epoch(sg["t1"]), "value": sg["p1"]})
    seen2 = set()
    deduped_segs = []
    for pt in seg_points:
        key = (pt["time"], pt["value"])
        if key not in seen2:
            seen2.add(key)
            deduped_segs.append(pt)
    deduped_segs.sort(key=lambda x: x["time"])
    seg_lines_js = json.dumps(deduped_segs)

    zhongshus = l1["zhongshus"]
    zs_boxes_js = json.dumps([
        {
            "t0": _ts_to_epoch(z["t0"]),
            "t1": _ts_to_epoch(z["t1"]),
            "zg": z["zg"], "zd": z["zd"],
            "settled": z["settled"],
        }
        for z in zhongshus
    ])

    bsps = l1.get("bsp", [])
    markers_js = json.dumps([
        {
            "time": _ts_to_epoch(b["ts"]),
            "position": "belowBar" if b["side"] == "buy" else "aboveBar",
            "color": "#e91e63" if b["side"] == "buy" else "#2196F3",
            "shape": "arrowUp" if b["side"] == "buy" else "arrowDown",
            "text": f"{b['kind']}{'*' if b['confirmed'] else ''}",
        }
        for b in bsps
    ])

    rec_levels = sorted(recursive.keys())
    rec_tabs_html = ""
    rec_tabs_js = ""
    for lv_key in rec_levels:
        lv = recursive[lv_key]
        lv_zs = lv["zhongshus"]
        lv_mv = lv["moves"]
        rec_tabs_html += f'<button class="tab-btn" onclick="showTab(\'{lv_key}\')">{lv_key} ({len(lv_zs)}zs, {len(lv_mv)}mv)</button>\n'
        rec_tabs_js += f"""
        recData['{lv_key}'] = {{
            zhongshus: {json.dumps(lv_zs)},
            moves: {json.dumps(lv_mv)},
        }};
        """

    n_bars = len(ohlc_raw)
    n_strokes = len(strokes)
    n_segs = len(segments)
    n_zs = len(zhongshus)
    n_moves = len(l1["moves"])
    n_bsp = len(bsps)
    max_lv = rec_levels[-1] if rec_levels else "L1"

    return f"""<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="UTF-8">
<meta name="viewport" content="width=device-width, initial-scale=1.0">
<title>{symbol} Chanlun Recursive Analysis</title>
<script src="https://unpkg.com/lightweight-charts@4.2.0/dist/lightweight-charts.standalone.production.js"></script>
<style>
* {{ margin: 0; padding: 0; box-sizing: border-box; }}
body {{ font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif; background: #131722; color: #d1d4dc; }}
.header {{ padding: 12px 20px; background: #1e222d; border-bottom: 1px solid #2a2e39; display: flex; align-items: center; gap: 20px; }}
.header h1 {{ font-size: 18px; font-weight: 600; }}
.stats {{ display: flex; gap: 16px; font-size: 13px; color: #787b86; }}
.stats span {{ color: #d1d4dc; }}
.tabs {{ padding: 8px 20px; background: #1e222d; display: flex; gap: 8px; border-bottom: 1px solid #2a2e39; }}
.tab-btn {{ padding: 6px 14px; border: 1px solid #2a2e39; background: #131722; color: #787b86; border-radius: 4px; cursor: pointer; font-size: 12px; }}
.tab-btn:hover {{ background: #2a2e39; }}
.tab-btn.active {{ background: #2962ff; color: #fff; border-color: #2962ff; }}
.controls {{ padding: 8px 20px; background: #1e222d; display: flex; gap: 12px; align-items: center; font-size: 12px; }}
.controls label {{ display: flex; align-items: center; gap: 4px; cursor: pointer; color: #787b86; }}
.controls input[type=checkbox] {{ accent-color: #2962ff; }}
#chart-container {{ width: 100%; height: calc(100vh - 130px); }}
#rec-info {{ padding: 12px 20px; font-size: 13px; display: none; background: #1e222d; border-bottom: 1px solid #2a2e39; }}
#rec-info table {{ border-collapse: collapse; width: 100%; }}
#rec-info th, #rec-info td {{ padding: 4px 10px; text-align: left; border-bottom: 1px solid #2a2e39; font-size: 12px; }}
#rec-info th {{ color: #787b86; }}
</style>
</head>
<body>
<div class="header">
    <h1>{symbol} Chanlun</h1>
    <div class="stats">
        Bars: <span>{n_bars:,}</span> |
        Strokes: <span>{n_strokes:,}</span> |
        Segments: <span>{n_segs:,}</span> |
        Zhongshus: <span>{n_zs:,}</span> |
        Moves: <span>{n_moves}</span> |
        BSPs: <span>{n_bsp}</span> |
        Depth: <span>L1-{max_lv}</span>
    </div>
</div>
<div class="tabs">
    <button class="tab-btn active" onclick="showTab('L1')">L1 (Main)</button>
    {rec_tabs_html}
</div>
<div class="controls">
    <label><input type="checkbox" id="cb-strokes" checked onchange="toggleStrokes()"> Strokes</label>
    <label><input type="checkbox" id="cb-segments" checked onchange="toggleSegments()"> Segments</label>
    <label><input type="checkbox" id="cb-zhongshus" checked onchange="toggleZhongshus()"> Zhongshus</label>
    <label><input type="checkbox" id="cb-bsp" checked onchange="toggleBsp()"> BSPs</label>
</div>
<div id="rec-info"></div>
<div id="chart-container"></div>

<script>
const ohlcData = {ohlc_js};
const strokeData = {stroke_lines_js};
const segData = {seg_lines_js};
const zsBoxes = {zs_boxes_js};
const bspMarkers = {markers_js};
const recData = {{}};
{rec_tabs_js}

const container = document.getElementById('chart-container');
const chart = LightweightCharts.createChart(container, {{
    layout: {{ background: {{ color: '#131722' }}, textColor: '#d1d4dc' }},
    grid: {{ vertLines: {{ color: '#1e222d' }}, horzLines: {{ color: '#1e222d' }} }},
    crosshair: {{ mode: LightweightCharts.CrosshairMode.Normal }},
    rightPriceScale: {{ borderColor: '#2a2e39' }},
    timeScale: {{ borderColor: '#2a2e39', timeVisible: true, secondsVisible: false }},
}});

const candleSeries = chart.addCandlestickSeries({{
    upColor: '#26a69a', downColor: '#ef5350',
    borderUpColor: '#26a69a', borderDownColor: '#ef5350',
    wickUpColor: '#26a69a', wickDownColor: '#ef5350',
}});
candleSeries.setData(ohlcData);

const strokeSeries = chart.addLineSeries({{
    color: '#42a5f5', lineWidth: 1, lineStyle: 0,
    crosshairMarkerVisible: false, lastValueVisible: false, priceLineVisible: false,
}});
strokeSeries.setData(strokeData);

const segSeries = chart.addLineSeries({{
    color: '#ff9800', lineWidth: 2, lineStyle: 0,
    crosshairMarkerVisible: false, lastValueVisible: false, priceLineVisible: false,
}});
segSeries.setData(segData);

candleSeries.setMarkers(bspMarkers);

let zsRectangles = [];
function drawZhongshus() {{
    clearZhongshus();
    for (const zs of zsBoxes) {{
        const color = zs.settled ? 'rgba(33, 150, 243, 0.12)' : 'rgba(255, 152, 0, 0.15)';
        const borderColor = zs.settled ? 'rgba(33, 150, 243, 0.4)' : 'rgba(255, 152, 0, 0.5)';
        const rect = {{
            p1: {{ time: zs.t0, price: zs.zd }},
            p2: {{ time: zs.t1, price: zs.zg }},
            background: color,
            borderColor: borderColor,
        }};
        try {{
            const r = LightweightCharts.createSeriesRectangle
                ? candleSeries.createPriceLine({{ price: zs.zg, color: borderColor, lineWidth: 1, lineStyle: 2 }})
                : null;
        }} catch(e) {{}}
        zsRectangles.push(zs);
    }}
}}

// Zhongshu via box series (plugin not available in v4.2, use price lines as fallback)
let zsLines = [];
function drawZsLines() {{
    clearZsLines();
    if (!document.getElementById('cb-zhongshus').checked) return;
    for (const zs of zsBoxes) {{
        const lineTop = candleSeries.createPriceLine({{
            price: zs.zg,
            color: zs.settled ? 'rgba(33,150,243,0.3)' : 'rgba(255,152,0,0.3)',
            lineWidth: 1, lineStyle: 2, axisLabelVisible: false,
        }});
        const lineBot = candleSeries.createPriceLine({{
            price: zs.zd,
            color: zs.settled ? 'rgba(33,150,243,0.3)' : 'rgba(255,152,0,0.3)',
            lineWidth: 1, lineStyle: 2, axisLabelVisible: false,
        }});
        zsLines.push(lineTop, lineBot);
    }}
}}
function clearZsLines() {{
    for (const l of zsLines) {{ try {{ candleSeries.removePriceLine(l); }} catch(e) {{}} }}
    zsLines = [];
}}

// Draw zhongshu boxes via canvas overlay
class ZhongshuRenderer {{
    constructor(chart, series, boxes) {{
        this.chart = chart;
        this.series = series;
        this.boxes = boxes;
        this.visible = true;
        this._canvas = document.createElement('canvas');
        this._canvas.style.position = 'absolute';
        this._canvas.style.top = '0';
        this._canvas.style.left = '0';
        this._canvas.style.pointerEvents = 'none';
        this._canvas.style.zIndex = '1';
        container.style.position = 'relative';
        container.appendChild(this._canvas);
        this._resize();
        this._draw();

        const ts = chart.timeScale();
        ts.subscribeVisibleTimeRangeChange(() => this._draw());
        ts.subscribeSizeChange(() => {{ this._resize(); this._draw(); }});
        new ResizeObserver(() => {{ this._resize(); this._draw(); }}).observe(container);
    }}

    _resize() {{
        const r = container.getBoundingClientRect();
        const dpr = window.devicePixelRatio || 1;
        this._canvas.width = r.width * dpr;
        this._canvas.height = r.height * dpr;
        this._canvas.style.width = r.width + 'px';
        this._canvas.style.height = r.height + 'px';
        this._ctx = this._canvas.getContext('2d');
        this._ctx.scale(dpr, dpr);
    }}

    _draw() {{
        const ctx = this._ctx;
        if (!ctx) return;
        const r = container.getBoundingClientRect();
        ctx.clearRect(0, 0, r.width, r.height);
        if (!this.visible) return;

        const ts = this.chart.timeScale();
        for (const box of this.boxes) {{
            const x0 = ts.timeToCoordinate(box.t0);
            const x1 = ts.timeToCoordinate(box.t1);
            const y0 = this.series.priceToCoordinate(box.zg);
            const y1 = this.series.priceToCoordinate(box.zd);
            if (x0 === null || x1 === null || y0 === null || y1 === null) continue;

            const xLeft = Math.min(x0, x1);
            const xRight = Math.max(x0, x1);
            const yTop = Math.min(y0, y1);
            const yBottom = Math.max(y0, y1);
            const w = Math.max(xRight - xLeft, 2);
            const h = Math.max(yBottom - yTop, 2);

            if (box.settled) {{
                ctx.fillStyle = 'rgba(33, 150, 243, 0.08)';
                ctx.strokeStyle = 'rgba(33, 150, 243, 0.35)';
            }} else {{
                ctx.fillStyle = 'rgba(255, 152, 0, 0.12)';
                ctx.strokeStyle = 'rgba(255, 152, 0, 0.45)';
            }}
            ctx.fillRect(xLeft, yTop, w, h);
            ctx.lineWidth = 1;
            ctx.strokeRect(xLeft, yTop, w, h);
        }}
    }}

    setVisible(v) {{
        this.visible = v;
        this._draw();
    }}
}}

const zsRenderer = new ZhongshuRenderer(chart, candleSeries, zsBoxes);

let showStrokes = true, showSegments = true, showZs = true, showBsp = true;
function toggleStrokes() {{
    showStrokes = document.getElementById('cb-strokes').checked;
    strokeSeries.applyOptions({{ visible: showStrokes }});
}}
function toggleSegments() {{
    showSegments = document.getElementById('cb-segments').checked;
    segSeries.applyOptions({{ visible: showSegments }});
}}
function toggleZhongshus() {{
    showZs = document.getElementById('cb-zhongshus').checked;
    zsRenderer.setVisible(showZs);
}}
function toggleBsp() {{
    showBsp = document.getElementById('cb-bsp').checked;
    candleSeries.setMarkers(showBsp ? bspMarkers : []);
}}

function showTab(key) {{
    document.querySelectorAll('.tab-btn').forEach(b => b.classList.remove('active'));
    event.target.classList.add('active');
    const info = document.getElementById('rec-info');
    if (key === 'L1') {{
        info.style.display = 'none';
        return;
    }}
    const rd = recData[key];
    if (!rd) {{ info.style.display = 'none'; return; }}
    let html = '<table><tr><th colspan="6">' + key + ' Zhongshus (' + rd.zhongshus.length + ')</th></tr>';
    html += '<tr><th>#</th><th>ZG</th><th>ZD</th><th>Comp</th><th>Count</th><th>Settled</th></tr>';
    rd.zhongshus.forEach((z, i) => {{
        html += '<tr><td>' + i + '</td><td>' + z.zg.toFixed(2) + '</td><td>' + z.zd.toFixed(2) +
            '</td><td>' + z.comp_s + '..' + z.comp_e + '</td><td>' + z.cnt + '</td><td>' +
            (z.settled ? 'Y' : 'N') + '</td></tr>';
    }});
    html += '</table><br><table><tr><th colspan="6">' + key + ' Moves (' + rd.moves.length + ')</th></tr>';
    html += '<tr><th>#</th><th>Kind</th><th>Dir</th><th>ZS</th><th>Range</th><th>Settled</th></tr>';
    rd.moves.forEach((m, i) => {{
        html += '<tr><td>' + i + '</td><td>' + m.kind + '</td><td>' + m.dir +
            '</td><td>' + m.zs_cnt + '</td><td>' + m.l.toFixed(2) + '~' + m.h.toFixed(2) +
            '</td><td>' + (m.settled ? 'Y' : 'N') + '</td></tr>';
    }});
    html += '</table>';
    info.innerHTML = html;
    info.style.display = 'block';
}}

chart.timeScale().fitContent();
</script>
</body>
</html>"""


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--symbol", choices=SYMBOLS + ["ALL"], default="ALL")
    args = parser.parse_args()

    OUT_DIR.mkdir(parents=True, exist_ok=True)
    symbols = SYMBOLS if args.symbol == "ALL" else [args.symbol]

    for sym in symbols:
        json_path = DATA_DIR / f"{sym.lower()}_recursive_result.json"
        if not json_path.exists():
            print(f"SKIP {sym}: {json_path} not found (run engine first)")
            continue

        print(f"Generating chart for {sym}...")
        with open(json_path) as f:
            data = json.load(f)

        html = build_html(data, sym)
        out_path = OUT_DIR / f"{sym.lower()}_chanlun.html"
        with open(out_path, "w") as f:
            f.write(html)
        print(f"  Saved: {out_path} ({len(html)//1024}KB)")


if __name__ == "__main__":
    main()
