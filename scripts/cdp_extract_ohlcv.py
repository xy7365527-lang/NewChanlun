#!/usr/bin/env python3
"""
CDP 直连 TradingView Desktop，对每个标的同时提取「日线 OHLCV + 缠论 Pine 标注」。

核心保证（坐标系一致性）：
  OHLCV 与 labels/lines 在**同一次图表加载**中提取，因此
  ohlcv[i] 严格对应 label.time == i（TV 内部 bar 数组 index）。
  不做事后对齐假设——同源即同坐标系。

输出（analysis/data_cache/）：
  hsi_tv_ohlcv.json, shcomp_tv_ohlcv.json, brn_tv_ohlcv.json

用法:
    .venv/bin/python scripts/cdp_extract_ohlcv.py
    .venv/bin/python scripts/cdp_extract_ohlcv.py --symbols hsi
"""

import base64
import json
import os
import socket
import struct
import sys
import time
import urllib.request
from pathlib import Path

CDP_HOST = "localhost"
CDP_PORT = 9222
CALL_TIMEOUT = 30
SYMBOL_WAIT = 20      # 切换标的后等待（图表+指标重新加载）
RESOLUTION_WAIT = 14  # 切换周期后等待指标计算

OUTPUT_DIR = Path("analysis/data_cache")

ALL_SYMBOLS = [
    {"name": "hsi",    "tv": "TVC:HSI",    "tv_alt": "HKEX:HSI"},
    {"name": "shcomp", "tv": "TVC:SHCOMP", "tv_alt": "SSE:000001"},
    {"name": "brn",    "tv": "TVC:UKOIL",  "tv_alt": "NYMEX:BZ1!"},
]

# CLI 过滤
_sym_filter: set = set()
for _arg in sys.argv[1:]:
    if _arg.startswith("--symbols"):
        parts = _arg.split("=", 1)
        val = parts[1] if len(parts) == 2 else (
            sys.argv[sys.argv.index(_arg) + 1]
            if sys.argv.index(_arg) + 1 < len(sys.argv) else "")
        _sym_filter = {s.strip() for s in val.split(",")}
SYMBOLS = [s for s in ALL_SYMBOLS if not _sym_filter or s["name"] in _sym_filter]


# ── WebSocket / CDP（复用 hk_index_cdp_fetch 框架）──────────────────────────────

def connect_ws(host, port, path, timeout=15):
    sock = socket.create_connection((host, port), timeout=timeout)
    sock.settimeout(timeout)
    key = base64.b64encode(os.urandom(16)).decode()
    hs = (f"GET {path} HTTP/1.1\r\nHost: {host}:{port}\r\nUpgrade: websocket\r\n"
          f"Connection: Upgrade\r\nSec-WebSocket-Key: {key}\r\n"
          f"Sec-WebSocket-Version: 13\r\n\r\n")
    sock.sendall(hs.encode())
    resp = b""
    while b"\r\n\r\n" not in resp:
        chunk = sock.recv(4096)
        if not chunk:
            raise ConnectionError("Handshake closed")
        resp += chunk
    if "101" not in resp.split(b"\r\n")[0].decode():
        raise ConnectionError("WS handshake failed")
    return sock


def send_frame(sock, payload, opcode=0x1):
    mask = os.urandom(4)
    masked = bytes(b ^ mask[i % 4] for i, b in enumerate(payload))
    n = len(payload)
    h = bytes([0x80 | opcode])
    if n < 126:
        h += bytes([0x80 | n])
    elif n < 65536:
        h += struct.pack(">BH", 0x80 | 126, n)
    else:
        h += struct.pack(">BQ", 0x80 | 127, n)
    sock.sendall(h + mask + masked)


def recv_frame(sock):
    def rx(n):
        d = b""
        while len(d) < n:
            c = sock.recv(n - len(d))
            if not c:
                raise ConnectionError("closed")
            d += c
        return d
    h = rx(2)
    opcode = h[0] & 0x0F
    masked = bool(h[1] & 0x80)
    length = h[1] & 0x7F
    if length == 126:
        length = struct.unpack(">H", rx(2))[0]
    elif length == 127:
        length = struct.unpack(">Q", rx(8))[0]
    mk = rx(4) if masked else b""
    pl = rx(length)
    if masked:
        pl = bytes(b ^ mk[i % 4] for i, b in enumerate(pl))
    if opcode == 0x9:
        send_frame(sock, pl, opcode=0xA)
        return recv_frame(sock)
    return opcode, pl


class CDPClient:
    def __init__(self, sock):
        self._sock = sock
        self._msg_id = 0
        self._pending = {}

    def _pump(self, timeout=0.5):
        self._sock.settimeout(timeout)
        while True:
            try:
                opcode, raw = recv_frame(self._sock)
                if opcode == 0x8:
                    break
                msg = json.loads(raw)
                if "id" in msg:
                    self._pending[msg["id"]] = msg
            except socket.timeout:
                break
            except (OSError, ConnectionError):
                raise

    def call(self, method, params=None, session_id=None, timeout=None):
        timeout = timeout or CALL_TIMEOUT
        self._msg_id += 1
        mid = self._msg_id
        payload = {"id": mid, "method": method, "params": params or {}}
        if session_id:
            payload["sessionId"] = session_id
        send_frame(self._sock, json.dumps(payload).encode())
        deadline = time.time() + timeout
        while time.time() < deadline:
            if mid in self._pending:
                msg = self._pending.pop(mid)
                if "error" in msg:
                    raise RuntimeError(f"CDP [{method}]: {msg['error']}")
                return msg.get("result", {})
            self._pump(timeout=min(1.0, deadline - time.time()))
        raise TimeoutError(f"CDP '{method}' timed out after {timeout}s")

    def evaluate(self, expr, session_id, await_promise=False):
        return self.call("Runtime.evaluate", {
            "expression": expr, "returnByValue": True, "awaitPromise": await_promise,
        }, session_id=session_id)

    def attach(self, target_id):
        r = self.call("Target.attachToTarget", {"targetId": target_id, "flatten": True})
        return r.get("sessionId", "")

    def detach(self, session_id):
        try:
            self.call("Target.detachFromTarget", {"sessionId": session_id}, timeout=5)
        except Exception:
            pass

    def close(self):
        try:
            send_frame(self._sock, b"", opcode=0x8)
        except Exception:
            pass
        self._sock.close()


def http_get(path):
    with urllib.request.urlopen(f"http://{CDP_HOST}:{CDP_PORT}{path}", timeout=5) as r:
        return r.read()


def find_chart_target():
    for p in json.loads(http_get("/json")):
        url = p.get("url", "").lower()
        if "tradingview.com" in url and p.get("type") == "page":
            return p
    return None


# ── JS 模板 ───────────────────────────────────────────────────────────────────

def make_set_symbol_js(symbol):
    return f"""
(function() {{
    const sym = "{symbol}";
    const tried = []; let ok = false;
    try {{
        if (window.tvWidget && window.tvWidget.setSymbol) {{
            window.tvWidget.setSymbol(sym, null, function() {{}});
            tried.push("tvWidget.setSymbol OK"); ok = true;
        }}
    }} catch(e) {{ tried.push("tvWidget.setSymbol: " + e.message); }}
    if (!ok) {{
        try {{
            const wv = window.TradingViewApi._activeChartWidgetWV.value();
            if (wv && wv.setSymbol) {{ wv.setSymbol(sym); tried.push("wv.setSymbol OK"); ok = true; }}
        }} catch(e) {{ tried.push("wv.setSymbol: " + e.message); }}
    }}
    return JSON.stringify({{ ok, tried, symbol: sym }});
}})()
"""


def make_set_resolution_js(resolution):
    return f"""
(function() {{
    const res = "{resolution}"; const tried = []; let ok = false;
    try {{
        const wv = window.TradingViewApi._activeChartWidgetWV.value();
        if (wv && wv.setResolution) {{ wv.setResolution(res); tried.push("wv.setResolution OK"); ok = true; }}
    }} catch(e) {{ tried.push("wv.setResolution: " + e.message); }}
    if (!ok) {{
        try {{
            if (window.tvWidget && window.tvWidget.activeChart) {{
                window.tvWidget.activeChart().setResolution(res); tried.push("tvWidget.activeChart().setResolution OK"); ok = true;
            }}
        }} catch(e) {{ tried.push("tvWidget: " + e.message); }}
    }}
    return JSON.stringify({{ ok, tried, resolution: res }});
}})()
"""


GET_CHART_STATE_JS = r"""
(function() {
    const state = { symbol: null, resolution: null };
    try {
        const wv = window.TradingViewApi._activeChartWidgetWV.value();
        if (wv) {
            if (wv.getSymbol) state.symbol = wv.getSymbol();
            if (wv.getResolution) state.resolution = wv.getResolution();
        }
    } catch(e) { state.error = e.message; }
    return JSON.stringify(state);
})()
"""

# OHLCV：遍历 mainSeries.bars()，每根 {i, t(unix), o, h, l, c, v}
EXTRACT_OHLCV_JS = r"""
(function() {
    const out = { ohlcv: [], err: null };
    try {
        const wv = window.TradingViewApi._activeChartWidgetWV.value();
        const series = wv._chartWidget.model().model().mainSeries();
        const bars = series.bars();
        const fi = bars.firstIndex();
        const li = bars.lastIndex();
        out.first_index = fi; out.last_index = li; out.size = bars.size();
        for (let i = fi; i <= li; i++) {
            let b;
            try { b = bars.valueAt(i); } catch(e) { continue; }
            if (!b) continue;
            const v = b.value || b;
            const idx = (b.index != null) ? b.index : i;
            out.ohlcv.push({
                i: idx, t: v[0],
                o: v[1] != null ? Math.round(v[1]*1e4)/1e4 : null,
                h: v[2] != null ? Math.round(v[2]*1e4)/1e4 : null,
                l: v[3] != null ? Math.round(v[3]*1e4)/1e4 : null,
                c: v[4] != null ? Math.round(v[4]*1e4)/1e4 : null,
                v: v[5] != null ? v[5] : null,
            });
        }
    } catch(e) { out.err = e.message; }
    return JSON.stringify(out);
})()
"""

EXTRACT_PINE_JS = r"""
(function() {
    const out = { labels: [], lines: [], boxes: [], studies: [], errors: [] };
    try {
        const chart = window.TradingViewApi._activeChartWidgetWV.value()._chartWidget;
        const sources = chart.model().model().dataSources();
        out.total_sources = sources.length;
        for (let si = 0; si < sources.length; si++) {
            const s = sources[si];
            if (!s || !s.metaInfo) continue;
            let meta, name;
            try { meta = s.metaInfo(); name = meta.description || meta.shortDescription || ''; }
            catch(e) { continue; }
            const studyInfo = {idx: si, name, labels: 0, lines: 0, boxes: 0};
            const g = s._graphics;
            if (!g || !g._primitivesCollection) { out.studies.push(studyInfo); continue; }
            const pc = g._primitivesCollection;
            try {
                const coll = pc.dwglabels?.get('labels')?.get(false);
                if (coll?._primitivesDataById?.size > 0) {
                    coll._primitivesDataById.forEach(function(v, id) {
                        out.labels.push({ id, study: name, si,
                            text: v.t || v.text || '',
                            price: v.y != null ? Math.round(v.y*100)/100 : null,
                            time: v.x != null ? v.x : null,
                            yloc: v.yl, tooltip: v.tt || '' });
                        studyInfo.labels++;
                    });
                }
            } catch(e) { out.errors.push(`s${si}_labels: ${e.message}`); }
            try {
                const coll = pc.dwglines?.get('lines')?.get(false);
                if (coll?._primitivesDataById?.size > 0) {
                    coll._primitivesDataById.forEach(function(v, id) {
                        if (v.y1 == null && v.y2 == null) return;
                        out.lines.push({ id, study: name, si,
                            price1: v.y1 != null ? Math.round(v.y1*100)/100 : null, bar1: v.x1,
                            price2: v.y2 != null ? Math.round(v.y2*100)/100 : null, bar2: v.x2,
                            width: v.w, style: v.st });
                        studyInfo.lines++;
                    });
                }
            } catch(e) { out.errors.push(`s${si}_lines: ${e.message}`); }
            try {
                const coll = pc.dwgboxes?.get('boxes')?.get(false);
                if (coll?._primitivesDataById?.size > 0) {
                    coll._primitivesDataById.forEach(function(v, id) {
                        out.boxes.push({ id, study: name, si,
                            price1: v.price1 != null ? Math.round(v.price1*100)/100 : null,
                            price2: v.price2 != null ? Math.round(v.price2*100)/100 : null,
                            time1: v.time1, time2: v.time2 });
                        studyInfo.boxes++;
                    });
                }
            } catch(e) { out.errors.push(`s${si}_boxes: ${e.message}`); }
            out.studies.push(studyInfo);
        }
    } catch(e) { out.errors.push('main: ' + e.message); }
    return JSON.stringify(out);
})()
"""

WAIT_READY_JS = r"""
(function() {
    return JSON.stringify({
        tv_api: typeof window.TradingViewApi,
        canvas: document.querySelectorAll('canvas').length,
    });
})()
"""


def run_js(cdp, js, session_id):
    result = cdp.evaluate(js, session_id)
    obj = result.get("result", {})
    if result.get("exceptionDetails"):
        raise RuntimeError(result["exceptionDetails"].get("exception", {}).get("description", "JS error")[:200])
    if obj.get("type") == "string":
        try:
            return json.loads(obj["value"])
        except json.JSONDecodeError:
            return {"raw": obj["value"][:300]}
    return {"raw_object": str(obj)[:300]}


def wait_ready(cdp, session_id, timeout=30):
    deadline = time.time() + timeout
    while time.time() < deadline:
        try:
            st = run_js(cdp, WAIT_READY_JS, session_id)
            if st.get("tv_api") != "undefined":
                return True
        except Exception:
            pass
        time.sleep(2)
    return False


def countdown(seconds, label=""):
    for i in range(seconds, 0, -1):
        print(f"\r  {label}({i}s)...  ", end="", flush=True)
        time.sleep(1)
    print()


def verify_alignment(ohlcv, labels):
    """验证 label.time 索引落在 OHLCV 的 [low, high] 内。返回检查报告。"""
    by_idx = {b["i"]: b for b in ohlcv}
    checks = []
    matched = 0
    tested = 0
    for lbl in labels:
        t = lbl.get("time")
        p = lbl.get("price")
        if t is None or p is None or p == 0:
            continue
        bar = by_idx.get(t)
        if bar is None:
            checks.append({"time": t, "price": p, "status": "no_bar"})
            continue
        tested += 1
        lo, hi = bar.get("l"), bar.get("h")
        # 容忍 label 画在 bar 上下方少量偏移（标注通常贴着极值）
        margin = (hi - lo) * 0.05 if (hi is not None and lo is not None) else 0
        inside = lo is not None and hi is not None and (lo - margin) <= p <= (hi + margin)
        if inside:
            matched += 1
        if len(checks) < 12:
            checks.append({"time": t, "price": p, "bar_lh": [lo, hi], "inside": inside})
    return {
        "tested": tested,
        "matched": matched,
        "match_rate": round(matched / tested, 3) if tested else None,
        "samples": checks,
    }


def extract_symbol(cdp, session_id, sym):
    name = sym["name"]
    print(f"\n{'='*65}\n[★] {name.upper()} ({sym['tv']})")

    # 切换 symbol
    def switch(tv):
        r = run_js(cdp, make_set_symbol_js(tv), session_id)
        print(f"    setSymbol({tv}): ok={r.get('ok')} | {r.get('tried')}")
        return bool(r.get("ok"))

    ok = switch(sym["tv"])
    tv_used = sym["tv"]
    if not ok and sym.get("tv_alt"):
        ok = switch(sym["tv_alt"])
        if ok:
            tv_used = sym["tv_alt"]
    countdown(SYMBOL_WAIT, f"等待 {name.upper()} 加载 ")

    # 设日线
    r = run_js(cdp, make_set_resolution_js("D"), session_id)
    print(f"    setResolution(D): ok={r.get('ok')}")
    countdown(RESOLUTION_WAIT, "等待日线指标计算 ")

    state = run_js(cdp, GET_CHART_STATE_JS, session_id)
    print(f"    当前: symbol={state.get('symbol')} res={state.get('resolution')}")

    # 同一次加载内：先 OHLCV 后 pine（顺序不影响坐标系，bars 与 primitives 同源）
    ohlcv_res = run_js(cdp, EXTRACT_OHLCV_JS, session_id)
    ohlcv = ohlcv_res.get("ohlcv", [])
    print(f"    OHLCV: {len(ohlcv)} bars (first_idx={ohlcv_res.get('first_index')} "
          f"last_idx={ohlcv_res.get('last_index')} size={ohlcv_res.get('size')})")
    if ohlcv_res.get("err"):
        print(f"    OHLCV err: {ohlcv_res['err']}")

    pine = run_js(cdp, EXTRACT_PINE_JS, session_id)
    labels = pine.get("labels", [])
    lines = pine.get("lines", [])
    print(f"    Pine: labels={len(labels)} lines={len(lines)} boxes={len(pine.get('boxes', []))}")

    # 对齐验证
    align = verify_alignment(ohlcv, labels)
    print(f"    对齐: tested={align['tested']} matched={align['matched']} "
          f"rate={align['match_rate']}")

    # 时间范围（unix → 可读）
    if ohlcv:
        t0 = time.strftime("%Y-%m-%d", time.gmtime(ohlcv[0]["t"]))
        t1 = time.strftime("%Y-%m-%d", time.gmtime(ohlcv[-1]["t"]))
        print(f"    日期范围: {t0} → {t1}")

    output = {
        "timestamp": time.strftime("%Y-%m-%dT%H:%M:%S"),
        "symbol_name": name,
        "symbol_tv": tv_used,
        "timeframe": "D",
        "bar_count": len(ohlcv),
        "first_index": ohlcv_res.get("first_index"),
        "last_index": ohlcv_res.get("last_index"),
        "ohlcv": ohlcv,
        "pine": {"labels": labels, "lines": lines, "boxes": pine.get("boxes", [])},
        "alignment_check": align,
    }
    out_path = OUTPUT_DIR / f"{name}_tv_ohlcv.json"
    OUTPUT_DIR.mkdir(parents=True, exist_ok=True)
    out_path.write_text(json.dumps(output, ensure_ascii=False, indent=2))
    print(f"    [✓] 保存 {out_path}")
    return output


def main():
    print(f"[*] OHLCV+缠论同源提取：{[s['name'] for s in SYMBOLS]}")
    bpath = json.loads(http_get("/json/version"))["webSocketDebuggerUrl"].replace(
        f"ws://{CDP_HOST}:{CDP_PORT}", "")
    sock = connect_ws(CDP_HOST, CDP_PORT, bpath)
    cdp = CDPClient(sock)
    print("[✓] Browser connected")
    cdp.call("Target.setDiscoverTargets", {"discover": True})

    chart = find_chart_target()
    if not chart:
        print("[ERROR] 未找到 TradingView 图表 target")
        sys.exit(1)
    tid = chart.get("targetId") or chart.get("id")
    session_id = cdp.attach(tid)
    cdp.call("Runtime.enable", session_id=session_id)
    print("[*] 等待 TV API...")
    wait_ready(cdp, session_id, timeout=30)

    results = {}
    try:
        for sym in SYMBOLS:
            try:
                out = extract_symbol(cdp, session_id, sym)
                results[sym["name"]] = {
                    "bars": out["bar_count"],
                    "labels": len(out["pine"]["labels"]),
                    "match_rate": out["alignment_check"]["match_rate"],
                }
            except Exception as e:
                print(f"    [ERROR] {sym['name']}: {e}")
                results[sym["name"]] = {"error": str(e)}
    finally:
        cdp.detach(session_id)
        cdp.close()
        print("\n[*] CDP 连接关闭")

    print(f"\n{'='*65}\n[✓] 汇总:")
    for n, r in results.items():
        if "error" in r:
            print(f"  {n:8s}: ERROR {r['error'][:50]}")
        else:
            print(f"  {n:8s}: bars={r['bars']:4d} labels={r['labels']:4d} match_rate={r['match_rate']}")


if __name__ == "__main__":
    main()
