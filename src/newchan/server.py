"""NewChan 本地 HTTP API 服务（bottle）"""

from __future__ import annotations

import json
import threading
import traceback
from pathlib import Path
import pandas as pd
from bottle import Bottle, request, response, static_file

from newchan.b_timeframe import SUPPORTED_TF, resample_ohlc
from newchan.cache import list_cached, load_df, save_df
from newchan.indicators import INDICATOR_REGISTRY, compute_indicator

app = Bottle()

# ------------------------------------------------------------------
# 工具
# ------------------------------------------------------------------

def _json_resp(data, status=200):
    response.content_type = "application/json"
    response.status = status
    return json.dumps(data, ensure_ascii=False)



def _df_to_records(df: pd.DataFrame) -> list[dict]:
    """DataFrame -> list of {time, open, high, low, close, volume, ...}。

    统一转 UTC 后去掉时区标记，确保和实时数据时间戳一致。
    """
    out = df.copy()
    if hasattr(out.index, "tz") and out.index.tz is not None:
        out.index = out.index.tz_convert("UTC").tz_localize(None)
    records = []
    for ts, row in out.iterrows():
        rec = {"time": ts.strftime("%Y-%m-%d %H:%M:%S") if hasattr(ts, "strftime") else str(ts)}
        for col in row.index:
            v = row[col]
            if pd.notna(v):
                rec[col] = round(float(v), 6)
        records.append(rec)
    return records


# ------------------------------------------------------------------
# 静态资源
# ------------------------------------------------------------------

_LWC_JS_DIR: Path | None = None
try:
    import lightweight_charts as _lwc
    _LWC_JS_DIR = Path(_lwc.__file__).parent / "js"
except ImportError:
    pass


@app.route("/static/<filepath:path>")
def serve_static(filepath):
    if _LWC_JS_DIR and (_LWC_JS_DIR / filepath).exists():
        return static_file(filepath, root=str(_LWC_JS_DIR))
    response.status = 404
    return "Not found"


# ------------------------------------------------------------------
# 主页（React 前端 or 旧前端 fallback）
# ------------------------------------------------------------------

_FRONTEND_DIST = Path(__file__).resolve().parents[2] / "frontend" / "dist"


@app.route("/")
def index():
    # 优先 serve React 前端 build 产物
    index_html = _FRONTEND_DIST / "index.html"
    if index_html.exists():
        return static_file("index.html", root=str(_FRONTEND_DIST))
    # 回退到旧的 Python 嵌入 HTML
    from newchan.b_chart import build_app_html
    response.content_type = "text/html; charset=utf-8"
    return build_app_html()


@app.route("/assets/<filepath:path>")
def serve_frontend_assets(filepath):
    """Serve React 前端编译产物（Vite 输出到 assets/）。"""
    return static_file(filepath, root=str(_FRONTEND_DIST / "assets"))


# ------------------------------------------------------------------
# API: 品种列表
# ------------------------------------------------------------------

@app.route("/api/symbols")
def api_symbols():
    return _json_resp(list_cached())


# ------------------------------------------------------------------
# API: 可用指标
# ------------------------------------------------------------------

@app.route("/api/indicators")
def api_indicators():
    result = []
    for name, reg in INDICATOR_REGISTRY.items():
        result.append({
            "name": name,
            "display": reg["display"],
            "params": reg["params"],
            "series": [{"key": s["key"], "color": s["color"], "type": s["type"]} for s in reg["series"]],
        })
    return _json_resp(result)


# ------------------------------------------------------------------
# API: 支持的周期
# ------------------------------------------------------------------

@app.route("/api/timeframes")
def api_timeframes():
    return _json_resp(SUPPORTED_TF)


# ------------------------------------------------------------------
# API: OHLCV 数据
# ------------------------------------------------------------------

@app.route("/api/ohlcv")
def api_ohlcv():
    symbol = request.query.get("symbol", "").upper()
    interval = request.query.get("interval", "1min")
    tf = request.query.get("tf", "1m")

    if not symbol:
        return _json_resp({"error": "missing symbol"}, 400)

    # 纯缓存只读：历史数据由 Databento CLI (fetch-db) 填充，
    # 实时数据由 Databento Live feeder 增量追加到同一缓存
    cache_name = f"{symbol}_{interval}_raw"
    df = load_df(cache_name)

    if df is None:
        return _json_resp({"error": f"{symbol} 无缓存数据，请先运行: python -m newchan.cli fetch-db --symbol {symbol}"}, 404)

    try:
        resampled = resample_ohlc(df, tf)
    except ValueError as e:
        return _json_resp({"error": str(e)}, 400)

    # 分页参数（前端按需加载用）
    to_ts = request.query.get("to", "")
    after_ts = request.query.get("after", "")
    count_back = request.query.get("countBack", "")

    filtered = resampled
    if to_ts:
        to_dt = pd.Timestamp(int(to_ts), unit="s")
        filtered = filtered[filtered.index <= to_dt]
    if after_ts:
        after_dt = pd.Timestamp(int(after_ts), unit="s")
        filtered = filtered[filtered.index > after_dt]
    if count_back:
        n = int(count_back)
        filtered = filtered.iloc[-n:]

    return _json_resp({"data": _df_to_records(filtered), "count": len(filtered)})


# ------------------------------------------------------------------
# API: 指标数据
# ------------------------------------------------------------------

@app.route("/api/indicator")
def api_indicator():
    symbol = request.query.get("symbol", "").upper()
    interval = request.query.get("interval", "1min")
    tf = request.query.get("tf", "1m")
    name = request.query.get("name", "")
    params_str = request.query.get("params", "")

    if not symbol or not name:
        return _json_resp({"error": "missing symbol or name"}, 400)

    cache_name = f"{symbol}_{interval}_raw"
    df = load_df(cache_name)
    if df is None:
        return _json_resp({"error": f"缓存 {cache_name} 不存在"}, 404)

    try:
        resampled = resample_ohlc(df, tf)
        params = {}
        if params_str:
            for pair in params_str.split(","):
                k, v = pair.split("=")
                params[k.strip()] = v.strip()
        result = compute_indicator(name, resampled, params)
    except (ValueError, KeyError) as e:
        return _json_resp({"error": str(e)}, 400)

    return _json_resp({"data": _df_to_records(result)})


# ------------------------------------------------------------------
# API: 拉取数据（异步）
# ------------------------------------------------------------------

_fetch_status: dict[str, dict] = {}


@app.route("/api/fetch", method="POST")
def api_fetch():
    """通过 Databento 异步拉取历史数据到缓存。"""
    body = request.json or {}
    symbol = body.get("symbol", "").upper()
    interval = body.get("interval", "1min")
    start = body.get("start", "2020-01-01")

    if not symbol:
        return _json_resp({"error": "missing symbol"}, 400)

    task_id = f"{symbol}_{interval}"
    if task_id in _fetch_status and _fetch_status[task_id].get("status") == "running":
        return _json_resp({"status": "running", "task_id": task_id})

    _fetch_status[task_id] = {"status": "running", "symbol": symbol, "interval": interval}

    def _do_fetch():
        try:
            from newchan.data_databento import fetch_and_cache
            cache_name, count = fetch_and_cache(symbol, interval, start=start)
            _fetch_status[task_id] = {"status": "done", "count": count}
        except Exception as e:
            _fetch_status[task_id] = {"status": "error", "error": str(e)}
            traceback.print_exc()

    threading.Thread(target=_do_fetch, daemon=True).start()
    return _json_resp({"status": "running", "task_id": task_id})


@app.route("/api/fetch/status")
def api_fetch_status():
    task_id = request.query.get("task_id", "")
    info = _fetch_status.get(task_id, {"status": "unknown"})
    return _json_resp(info)


# ------------------------------------------------------------------
# API: 合成标的
# ------------------------------------------------------------------

@app.route("/api/synthetic", method="POST")
def api_synthetic():
    body = request.json or {}
    sym_a = body.get("a", "").upper()
    sym_b = body.get("b", "").upper()
    op = body.get("op", "spread")
    interval = body.get("interval", "1min")

    if not sym_a or not sym_b:
        return _json_resp({"error": "missing a or b"}, 400)

    df_a = load_df(f"{sym_a}_{interval}_raw")
    df_b = load_df(f"{sym_b}_{interval}_raw")
    if df_a is None:
        return _json_resp({"error": f"{sym_a} 数据不存在"}, 404)
    if df_b is None:
        return _json_resp({"error": f"{sym_b} 数据不存在"}, 404)

    from newchan.synthetic import make_ratio, make_spread

    if op == "spread":
        df_synth = make_spread(df_a, df_b)
    elif op == "ratio":
        df_synth = make_ratio(df_a, df_b)
    else:
        return _json_resp({"error": f"未知 op: {op}"}, 400)

    synth_name = f"{sym_a}_{sym_b}_{op}"
    cache_name = f"{synth_name}_{interval}_raw"
    save_df(cache_name, df_synth)
    return _json_resp({"name": synth_name, "cache": cache_name, "count": len(df_synth)})


# ------------------------------------------------------------------
# API: 连接状态（Databento Live）
# ------------------------------------------------------------------

@app.route("/api/connection")
def api_connection():
    feeder = _get_live_feeder()
    return _json_resp({
        "connected": feeder.is_running,
        "source": "databento_live",
        "bar_count": feeder.bar_count,
    })


@app.route("/api/connection/connect", method="POST")
def api_connect():
    feeder = _get_live_feeder()
    if feeder.is_running:
        return _json_resp({"connected": True, "msg": "Databento Live 已在运行"})
    try:
        feeder.start()
        return _json_resp({"connected": True, "msg": "Databento Live 已启动"})
    except Exception as e:
        return _json_resp({"connected": False, "error": str(e)}, 500)


# ------------------------------------------------------------------
# API: 品种搜索（纯缓存 + Databento 已知品种）
# ------------------------------------------------------------------

@app.route("/api/search")
def api_search():
    q = request.query.get("q", "").strip()
    if not q:
        return _json_resp([])

    results = []
    seen = set()

    # 1. 已缓存品种（优先显示）
    cached = list_cached()
    q_upper = q.upper()
    for item in cached:
        if q_upper in item["symbol"].upper():
            sym = item["symbol"]
            if sym not in seen:
                results.append({
                    "symbol": sym,
                    "secType": "CACHED",
                    "exchange": "",
                    "currency": "USD",
                    "description": f"已缓存 ({item['interval']})",
                    "source": "cache",
                })
                seen.add(sym)

    # 2. Databento 品种目录搜索（支持中英文、交易所名模糊匹配）
    from newchan.data_databento import search_symbols
    db_results = search_symbols(q)
    for item in db_results:
        sym = item["symbol"]
        if sym not in seen:
            results.append({
                "symbol": sym,
                "secType": item["type"],
                "exchange": item["exchange"],
                "currency": "USD",
                "description": f"{item['name']} ({item['cn']})",
                "source": "databento",
            })
            seen.add(sym)

    return _json_resp(results)


# ------------------------------------------------------------------
# API: 实时数据（Databento Live 后台自动追加，这些接口仅为前端兼容）
# ------------------------------------------------------------------

@app.route("/api/subscribe", method="POST")
def api_subscribe():
    body = request.json or {}
    symbol = body.get("symbol", "").upper()
    if not symbol:
        return _json_resp({"error": "missing symbol"}, 400)
    # Databento Live feeder 在后台自动运行，无需单独订阅
    feeder = _get_live_feeder()
    return _json_resp({"subscribed": feeder.is_running, "symbol": symbol})


@app.route("/api/unsubscribe", method="POST")
def api_unsubscribe():
    body = request.json or {}
    symbol = body.get("symbol", "").upper()
    # Databento Live 不支持单品种退订，返回 OK 即可
    return _json_resp({"unsubscribed": True, "symbol": symbol})


@app.route("/api/realtime")
def api_realtime():
    # Databento Live bar 直接写入缓存，前端通过 60s 定时 loadData 自动获取
    # 这个接口保留为空操作，避免前端轮询报错
    return _json_resp({"bars": [], "next_since": 0})


# ------------------------------------------------------------------
# API: 新缠论 overlay（A→B 桥接）
# ------------------------------------------------------------------

@app.route("/api/newchan/overlay")
def api_newchan_overlay():
    symbol = request.query.get("symbol", "").upper()
    interval = request.query.get("interval", "1min")
    tf = request.query.get("tf", "1m")
    detail = request.query.get("detail", "full")
    segment_algo = request.query.get("segment_algo", "v1")
    stroke_mode = request.query.get("stroke_mode", "wide")
    min_strict_sep = int(request.query.get("min_strict_sep", "5"))
    center_sustain_m = int(request.query.get("center_sustain_m", "2"))
    max_post_exit = int(request.query.get("max_post_exit_segments", "6"))
    limit = request.query.get("limit", "")

    if not symbol:
        return _json_resp({"error": "missing symbol"}, 400)

    cache_name = f"{symbol}_{interval}_raw"
    df = load_df(cache_name)
    if df is None:
        return _json_resp({"error": f"缓存 {cache_name} 不存在"}, 404)

    try:
        resampled = resample_ohlc(df, tf)
    except ValueError as e:
        return _json_resp({"error": str(e)}, 400)

    if limit:
        n = int(limit)
        if n > 0:
            resampled = resampled.iloc[-n:]

    try:
        from newchan.ab_bridge_newchan import build_overlay_newchan
        result = build_overlay_newchan(
            resampled,
            symbol=symbol,
            tf=tf,
            detail=detail,
            segment_algo=segment_algo,
            stroke_mode=stroke_mode,
            min_strict_sep=min_strict_sep,
            center_sustain_m=center_sustain_m,
            max_post_exit_segments=max_post_exit,
        )
    except Exception as e:
        import traceback
        traceback.print_exc()
        return _json_resp({"error": str(e)}, 500)

    return _json_resp(result)


# ------------------------------------------------------------------
# 启动服务
# ------------------------------------------------------------------

# ------------------------------------------------------------------
# Databento Live 实时数据
# ------------------------------------------------------------------

_live_feeder = None


def _get_live_feeder():
    global _live_feeder
    if _live_feeder is None:
        from newchan.data_databento_live import DatabentoLiveFeeder
        _live_feeder = DatabentoLiveFeeder()
    return _live_feeder


@app.route("/api/live/status")
def api_live_status():
    feeder = _get_live_feeder()
    return _json_resp(feeder.status())


@app.route("/api/live/start", method="POST")
def api_live_start():
    feeder = _get_live_feeder()
    feeder.start()
    return _json_resp(feeder.status())


@app.route("/api/live/stop", method="POST")
def api_live_stop():
    feeder = _get_live_feeder()
    feeder.stop()
    return _json_resp(feeder.status())


# ------------------------------------------------------------------
# 启动服务
# ------------------------------------------------------------------

def run_server(port: int = 8765, open_browser: bool = True) -> None:
    """启动 bottle 服务。"""
    from socketserver import ThreadingMixIn
    from wsgiref.simple_server import WSGIServer, make_server

    class _ThreadedWSGI(ThreadingMixIn, WSGIServer):
        daemon_threads = True
        allow_reuse_address = True

    print(f"NewChan 图表服务启动: http://localhost:{port}", flush=True)
    print("按 Ctrl+C 停止。", flush=True)

    # 启动 Databento Live 实时数据（非阻塞，后台线程）
    feeder = _get_live_feeder()
    try:
        feeder.start()
        print(f"Databento Live 已启动: {feeder._symbols}", flush=True)
    except Exception as e:
        print(f"Databento Live 启动失败: {e}（可稍后通过 /api/live/start 重试）", flush=True)

    srv = make_server("127.0.0.1", port, app, server_class=_ThreadedWSGI)

    if open_browser:
        import webbrowser
        webbrowser.open(f"http://localhost:{port}")

    try:
        srv.serve_forever()
    except KeyboardInterrupt:
        feeder.stop()
        print("\n服务已停止。")
    finally:
        srv.server_close()
