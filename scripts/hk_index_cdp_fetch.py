#!/usr/bin/env python3
"""
港股三大指数多周期缠论标注提取（CDP 直连 TradingView Desktop）

标的：
  - 恒生指数     TVC:HSI
  - 上证指数     SSE:000001
  - 恒生科技指数  HKEX:HSTECH

周期：1分钟 / 5分钟 / 30分钟 / 日线

输出（analysis/data_cache/）：
  hsi_1min_chanlun.json, hsi_5min_chanlun.json, ...
  shcomp_1min_chanlun.json, ...
  hstech_1min_chanlun.json, ...

用法:
    .venv/bin/python scripts/hk_index_cdp_fetch.py
    .venv/bin/python scripts/hk_index_cdp_fetch.py --symbols hsi,hstech
    .venv/bin/python scripts/hk_index_cdp_fetch.py --timeframes daily,30min
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
SYMBOL_WAIT = 18    # 切换标的后等待秒数（等图表+指标重新加载）
RESOLUTION_WAIT = 12  # 切换周期后等待指标计算

OUTPUT_DIR = Path("analysis/data_cache")

ALL_SYMBOLS = [
    {"name": "hsi",    "tv": "TVC:HSI",     "tv_alt": "HKEX:HSI"},
    {"name": "shcomp", "tv": "SSE:000001",   "tv_alt": "TVC:SHCOMP"},
    {"name": "hstech", "tv": "HKEX:HSTECH",  "tv_alt": "HKEX:3032"},
    # BRN: 尝试 TVC:UKOIL（Brent 现货连续），labels=0 时自动回退到 NYMEX:BZ1!
    {"name": "brn",    "tv": "TVC:UKOIL",    "tv_alt": "NYMEX:BZ1!", "retry_alt_on_empty": True},
]

ALL_TIMEFRAMES = [
    {"res": "1",  "name": "1min"},
    {"res": "5",  "name": "5min"},
    {"res": "30", "name": "30min"},
    {"res": "D",  "name": "daily"},
]

# ── CLI 参数解析 ──────────────────────────────────────────────────────────────

_sym_filter: set = set()
_tf_filter: set = set()
for _arg in sys.argv[1:]:
    if _arg.startswith("--symbols"):
        parts = _arg.split("=", 1)
        val = parts[1] if len(parts) == 2 else (
            sys.argv[sys.argv.index(_arg) + 1]
            if sys.argv.index(_arg) + 1 < len(sys.argv) else ""
        )
        _sym_filter = {s.strip() for s in val.split(",")}
    elif _arg.startswith("--timeframes"):
        parts = _arg.split("=", 1)
        val = parts[1] if len(parts) == 2 else (
            sys.argv[sys.argv.index(_arg) + 1]
            if sys.argv.index(_arg) + 1 < len(sys.argv) else ""
        )
        _tf_filter = {t.strip() for t in val.split(",")}

SYMBOLS = [s for s in ALL_SYMBOLS if not _sym_filter or s["name"] in _sym_filter]
TIMEFRAMES = [t for t in ALL_TIMEFRAMES if not _tf_filter or t["name"] in _tf_filter]


# ── Raw WebSocket ─────────────────────────────────────────────────────────────

def connect_ws(host: str, port: int, path: str, timeout: int = 15) -> socket.socket:
    sock = socket.create_connection((host, port), timeout=timeout)
    sock.settimeout(timeout)
    key = base64.b64encode(os.urandom(16)).decode()
    hs = (
        f"GET {path} HTTP/1.1\r\n"
        f"Host: {host}:{port}\r\n"
        f"Upgrade: websocket\r\n"
        f"Connection: Upgrade\r\n"
        f"Sec-WebSocket-Key: {key}\r\n"
        f"Sec-WebSocket-Version: 13\r\n\r\n"
    )
    sock.sendall(hs.encode())
    resp = b""
    while b"\r\n\r\n" not in resp:
        chunk = sock.recv(4096)
        if not chunk:
            raise ConnectionError("Handshake closed")
        resp += chunk
    status = resp.split(b"\r\n")[0].decode()
    if "101" not in status:
        raise ConnectionError(f"WS handshake failed: {status}")
    return sock


def send_frame(sock: socket.socket, payload: bytes, opcode: int = 0x1):
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


def recv_frame(sock: socket.socket) -> tuple:
    def rx(n: int) -> bytes:
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
    def __init__(self, sock: socket.socket):
        self._sock = sock
        self._msg_id = 0
        self._pending = {}
        self._events = []

    def _pump(self, timeout: float = 0.5):
        self._sock.settimeout(timeout)
        while True:
            try:
                opcode, raw = recv_frame(self._sock)
                if opcode == 0x8:
                    break
                msg = json.loads(raw)
                if "id" in msg:
                    self._pending[msg["id"]] = msg
                else:
                    self._events.append(msg)
            except socket.timeout:
                break
            except (OSError, ConnectionError):
                raise

    def drain_events(self) -> list:
        self._pump(timeout=0.3)
        evts = self._events[:]
        self._events.clear()
        return evts

    def call(self, method: str, params=None, session_id=None, timeout=None) -> dict:
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

    def evaluate(self, expr: str, session_id: str, await_promise: bool = False) -> dict:
        return self.call("Runtime.evaluate", {
            "expression": expr,
            "returnByValue": True,
            "awaitPromise": await_promise,
        }, session_id=session_id)

    def attach(self, target_id: str) -> str:
        r = self.call("Target.attachToTarget", {"targetId": target_id, "flatten": True})
        return r.get("sessionId", "")

    def detach(self, session_id: str):
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


# ── HTTP helpers ──────────────────────────────────────────────────────────────

def http_get(path: str) -> bytes:
    with urllib.request.urlopen(
        f"http://{CDP_HOST}:{CDP_PORT}{path}", timeout=5
    ) as r:
        return r.read()


def get_pages() -> list:
    return json.loads(http_get("/json"))


def get_browser_ws_path() -> str:
    ver = json.loads(http_get("/json/version"))
    return ver["webSocketDebuggerUrl"].replace(f"ws://{CDP_HOST}:{CDP_PORT}", "")


def find_chart_target() -> dict | None:
    for p in get_pages():
        url = p.get("url", "").lower()
        if "tradingview.com" in url and p.get("type") == "page":
            return p
    return None


# ── JS 模板 ───────────────────────────────────────────────────────────────────

def make_set_symbol_js(symbol: str) -> str:
    return f"""
(function() {{
    const sym = "{symbol}";
    const tried = [];
    let ok = false;

    // 方法1: tvWidget.setSymbol (公开 API)
    try {{
        if (window.tvWidget && window.tvWidget.setSymbol) {{
            window.tvWidget.setSymbol(sym, null, function() {{}});
            tried.push("tvWidget.setSymbol OK");
            ok = true;
        }}
    }} catch(e) {{ tried.push("tvWidget.setSymbol: " + e.message); }}

    // 方法2: tvWidget.activeChart().setSymbol
    if (!ok) {{
        try {{
            if (window.tvWidget && window.tvWidget.activeChart) {{
                window.tvWidget.activeChart().setSymbol(sym, function() {{}});
                tried.push("activeChart.setSymbol OK");
                ok = true;
            }}
        }} catch(e) {{ tried.push("activeChart.setSymbol: " + e.message); }}
    }}

    // 方法3: TradingViewApi wv
    if (!ok) {{
        try {{
            const wv = window.TradingViewApi._activeChartWidgetWV.value();
            if (wv && wv.setSymbol) {{
                wv.setSymbol(sym);
                tried.push("wv.setSymbol OK");
                ok = true;
            }}
        }} catch(e) {{ tried.push("wv.setSymbol: " + e.message); }}
    }}

    // 方法4: ChartApiInstance
    if (!ok) {{
        try {{
            const api = window.ChartApiInstance;
            if (api && api.chart && api.chart().setSymbol) {{
                api.chart().setSymbol(sym, function() {{}});
                tried.push("ChartApiInstance.chart.setSymbol OK");
                ok = true;
            }}
        }} catch(e) {{ tried.push("ChartApiInstance: " + e.message); }}
    }}

    return JSON.stringify({{ ok, tried, symbol: sym }});
}})()
"""


def make_set_resolution_js(resolution: str) -> str:
    return f"""
(function() {{
    const res = "{resolution}";
    const tried = [];
    let ok = false;

    try {{
        const wv = window.TradingViewApi._activeChartWidgetWV.value();
        if (wv && wv.setResolution) {{
            wv.setResolution(res);
            tried.push("wv.setResolution OK");
            ok = true;
        }}
    }} catch(e) {{ tried.push("wv.setResolution: " + e.message); }}

    if (!ok) {{
        try {{
            if (window.tvWidget && window.tvWidget.activeChart) {{
                window.tvWidget.activeChart().setResolution(res);
                tried.push("tvWidget.activeChart().setResolution OK");
                ok = true;
            }}
        }} catch(e) {{ tried.push("tvWidget: " + e.message); }}
    }}

    if (!ok) {{
        try {{
            const wv = window.TradingViewApi._activeChartWidgetWV.value();
            const cw = wv._chartWidget;
            if (cw && cw.model) {{
                cw.model().model().setResolution(res);
                tried.push("chartWidget.model.setResolution OK");
                ok = true;
            }}
        }} catch(e) {{ tried.push("chartWidget: " + e.message); }}
    }}

    return JSON.stringify({{ ok, tried, resolution: res }});
}})()
"""


GET_CHART_STATE_JS = r"""
(function() {
    const state = { symbol: null, resolution: null, methods: {} };
    try {
        const wv = window.TradingViewApi._activeChartWidgetWV.value();
        if (wv) {
            if (wv.getSymbol) { state.symbol = wv.getSymbol(); state.methods.symbol = "wv.getSymbol"; }
            if (wv.getResolution) { state.resolution = wv.getResolution(); state.methods.resolution = "wv.getResolution"; }
            if (!state.symbol && wv._chartWidget) {
                const ms = wv._chartWidget.model().model().mainSeries();
                if (ms) {
                    if (ms.symbol) { state.symbol = ms.symbol(); state.methods.symbol = "mainSeries.symbol"; }
                    if (ms.resolution) { state.resolution = ms.resolution(); state.methods.resolution = "mainSeries.resolution"; }
                }
            }
        }
    } catch(e) { state.error = e.message; }
    return JSON.stringify(state);
})()
"""


EXTRACT_PINE_JS = r"""
(function() {
    const out = {
        labels: [], lines: [], boxes: [],
        method: 'buildGraphicsJS', studies: [], errors: []
    };

    try {
        const chart = window.TradingViewApi._activeChartWidgetWV.value()._chartWidget;
        const sources = chart.model().model().dataSources();
        out.total_sources = sources.length;

        for (let si = 0; si < sources.length; si++) {
            const s = sources[si];
            if (!s || !s.metaInfo) continue;
            let meta, name;
            try {
                meta = s.metaInfo();
                name = meta.description || meta.shortDescription || '';
            } catch(e) { continue; }

            const studyInfo = {idx: si, name, labels: 0, lines: 0, boxes: 0};
            const g = s._graphics;
            if (!g || !g._primitivesCollection) { out.studies.push(studyInfo); continue; }
            const pc = g._primitivesCollection;

            // labels
            try {
                const coll = pc.dwglabels?.get('labels')?.get(false);
                if (coll?._primitivesDataById?.size > 0) {
                    coll._primitivesDataById.forEach(function(v, id) {
                        out.labels.push({
                            id, study: name, si,
                            text: v.t || v.text || '',
                            price: v.y != null ? Math.round(v.y * 100) / 100 : null,
                            time: v.x != null ? v.x : null,
                            yloc: v.yl, size: v.sz,
                            text_color: v.tci, bg_color: v.ci,
                            tooltip: v.tt || '',
                        });
                        studyInfo.labels++;
                    });
                }
            } catch(e) { out.errors.push(`s${si}_labels: ${e.message}`); }

            // lines
            try {
                const coll = pc.dwglines?.get('lines')?.get(false);
                if (coll?._primitivesDataById?.size > 0) {
                    coll._primitivesDataById.forEach(function(v, id) {
                        if (v.y1 == null && v.y2 == null) return;
                        out.lines.push({
                            id, study: name, si,
                            price1: v.y1 != null ? Math.round(v.y1 * 100) / 100 : null,
                            bar1: v.x1,
                            price2: v.y2 != null ? Math.round(v.y2 * 100) / 100 : null,
                            bar2: v.x2,
                            color: v.ci, width: v.w, style: v.st,
                        });
                        studyInfo.lines++;
                    });
                }
            } catch(e) { out.errors.push(`s${si}_lines: ${e.message}`); }

            // boxes
            try {
                const coll = pc.dwgboxes?.get('boxes')?.get(false);
                if (coll?._primitivesDataById?.size > 0) {
                    coll._primitivesDataById.forEach(function(v, id) {
                        out.boxes.push({
                            id, study: name, si,
                            price1: v.price1 != null ? Math.round(v.price1 * 100) / 100 : null,
                            price2: v.price2 != null ? Math.round(v.price2 * 100) / 100 : null,
                            time1: v.time1, time2: v.time2,
                        });
                        studyInfo.boxes++;
                    });
                }
            } catch(e) { out.errors.push(`s${si}_boxes: ${e.message}`); }

            // hlines
            try {
                const coll = pc.horizlines?.get('horizlines')?.get(false);
                if (coll?._primitivesDataById?.size > 0) {
                    coll._primitivesDataById.forEach(function(v, id) {
                        out.lines.push({
                            id, study: name, si, type: 'hline',
                            price1: v.price != null ? Math.round(v.price * 100) / 100 : null,
                            price2: null, bar1: null, bar2: null,
                        });
                    });
                }
            } catch(e) {}

            out.studies.push(studyInfo);
        }
    } catch(e) {
        out.errors.push('main: ' + e.message);
    }
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


# ── helpers ───────────────────────────────────────────────────────────────────

def run_js(cdp: CDPClient, js: str, session_id: str):
    result = cdp.evaluate(js, session_id)
    obj = result.get("result", {})
    exc = result.get("exceptionDetails")
    if exc:
        raise RuntimeError(exc.get("exception", {}).get("description", "JS error")[:200])
    if obj.get("type") == "string":
        try:
            return json.loads(obj["value"])
        except json.JSONDecodeError:
            return {"raw": obj["value"]}
    return {"raw_object": obj}


def wait_ready(cdp: CDPClient, session_id: str, timeout: int = 30) -> bool:
    deadline = time.time() + timeout
    while time.time() < deadline:
        try:
            st = run_js(cdp, WAIT_READY_JS, session_id)
            if st.get("tv_api") != "undefined":
                return True
        except Exception:
            pass
        remaining = int(deadline - time.time())
        print(f"\r  等待 TV API... ({remaining}s)  ", end="", flush=True)
        time.sleep(2)
    print()
    return False


def countdown(seconds: int, label: str = ""):
    for i in range(seconds, 0, -1):
        print(f"\r  {label}({i}s)...  ", end="", flush=True)
        time.sleep(1)
    print()


# ── 单次提取 ─────────────────────────────────────────────────────────────────

def extract_one(cdp: CDPClient, session_id: str, sym: dict, tf: dict) -> dict:
    """切换到指定周期，等待，提取，返回摘要。"""
    res = tf["res"]
    tf_name = tf["name"]
    sym_name = sym["name"]
    output_path = OUTPUT_DIR / f"{sym_name}_{tf_name}_chanlun.json"

    print(f"\n  [TF→{tf_name}] 切换周期 setResolution({res})")
    try:
        r = run_js(cdp, make_set_resolution_js(res), session_id)
        print(f"    ok={r.get('ok')} | {r.get('tried', [])}")
    except Exception as e:
        print(f"    setResolution 异常: {e}")

    countdown(RESOLUTION_WAIT, f"等待指标计算 {tf_name} ")

    # 验证当前状态
    try:
        state = run_js(cdp, GET_CHART_STATE_JS, session_id)
        print(f"    当前状态: symbol={state.get('symbol')} resolution={state.get('resolution')}")
        actual_res = str(state.get("resolution", ""))
        if actual_res and actual_res != str(res):
            print(f"    [!] 警告: 期望 {res}，实际 {actual_res}")
    except Exception as e:
        print(f"    状态检查失败: {e}")

    # 提取
    print(f"    提取 Pine labels...")
    pine = run_js(cdp, EXTRACT_PINE_JS, session_id)

    label_count = len(pine.get("labels", []))
    line_count = len(pine.get("lines", []))
    box_count = len(pine.get("boxes", []))

    print(f"    labels={label_count} lines={line_count} boxes={box_count}")
    if pine.get("errors"):
        print(f"    JS errors: {pine['errors'][:3]}")

    # 研究列表
    for st in pine.get("studies", []):
        total = st.get("labels", 0) + st.get("lines", 0) + st.get("boxes", 0)
        if total > 0:
            print(f"      [✓] [{st['idx']}] {st['name'][:30]} "
                  f"L={st['labels']} l={st['lines']} b={st['boxes']}")

    output = {
        "timestamp": time.strftime("%Y-%m-%dT%H:%M:%S"),
        "symbol_name": sym_name,
        "symbol_tv": sym["tv"],
        "timeframe": res,
        "timeframe_name": tf_name,
        "pine": pine,
        "summary": {
            "label_count": label_count,
            "line_count": line_count,
            "box_count": box_count,
            "studies": pine.get("studies", []),
        },
    }

    OUTPUT_DIR.mkdir(parents=True, exist_ok=True)
    output_path.write_text(json.dumps(output, ensure_ascii=False, indent=2))
    print(f"    [✓] 保存 {output_path}")
    return output["summary"]


# ── 主流程 ───────────────────────────────────────────────────────────────────

def main():
    total = len(SYMBOLS) * len(TIMEFRAMES)
    print(f"[*] 港股指数多周期提取：{len(SYMBOLS)} 标的 × {len(TIMEFRAMES)} 周期 = {total} 组合")
    print(f"[*] 标的: {[s['name'] for s in SYMBOLS]}")
    print(f"[*] 周期: {[t['name'] for t in TIMEFRAMES]}")
    print(f"[*] 连接 CDP browser...")

    browser_path = get_browser_ws_path()
    sock = connect_ws(CDP_HOST, CDP_PORT, browser_path)
    cdp = CDPClient(sock)
    print(f"[✓] Browser connected")

    cdp.call("Target.setDiscoverTargets", {"discover": True})

    chart_target = find_chart_target()
    if not chart_target:
        print("[ERROR] 未找到 TradingView 图表 target，请先在 TV Desktop 打开图表")
        sys.exit(1)

    print(f"\n[✓] 图表 target: {chart_target.get('url', '')[:80]}")
    tid = chart_target.get("targetId") or chart_target.get("id")
    session_id = cdp.attach(tid)
    print(f"    sessionId: {session_id}")

    cdp.call("Runtime.enable", session_id=session_id)
    print(f"[*] 等待 TV API 就绪...")
    wait_ready(cdp, session_id, timeout=30)

    results = {}
    done = 0

    def switch_to_symbol(tv_sym: str) -> bool:
        """切换到指定 TV symbol，返回 setSymbol 是否成功。"""
        try:
            r = run_js(cdp, make_set_symbol_js(tv_sym), session_id)
            print(f"    setSymbol({tv_sym}): ok={r.get('ok')} | {r.get('tried', [])}")
            return bool(r.get("ok"))
        except Exception as e:
            print(f"    setSymbol({tv_sym}) 异常: {e}")
            return False

    try:
        for sym in SYMBOLS:
            sym_name = sym["name"]
            results[sym_name] = {}
            tv_sym_used = sym["tv"]

            print(f"\n{'='*65}")
            print(f"[★] 标的: {sym_name.upper()} ({sym['tv']})")
            print(f"    切换 symbol...")

            # 第一次切换：尝试主 symbol
            ok = switch_to_symbol(sym["tv"])
            if not ok and sym.get("tv_alt"):
                ok = switch_to_symbol(sym["tv_alt"])
                if ok:
                    tv_sym_used = sym["tv_alt"]
            if not ok:
                print(f"    [!] symbol 切换失败，继续尝试提取当前数据")

            countdown(SYMBOL_WAIT, f"等待 {sym_name.upper()} 加载 ")

            try:
                state = run_js(cdp, GET_CHART_STATE_JS, session_id)
                print(f"    当前标的: {state.get('symbol', '未知')}")
            except Exception as e:
                print(f"    标的验证失败: {e}")

            # 提取所有周期
            all_empty = True
            for tf in TIMEFRAMES:
                done += 1
                print(f"\n  [{done}/{total}] {sym_name}_{tf['name']}")
                try:
                    summary = extract_one(cdp, session_id, sym, tf)
                    results[sym_name][tf["name"]] = summary
                    if summary.get("label_count", 0) > 0:
                        all_empty = False
                except Exception as e:
                    print(f"    [ERROR] {e}")
                    results[sym_name][tf["name"]] = {"error": str(e)}

            # retry_alt_on_empty: 若所有周期都为 0 labels，切换备用 symbol 重试
            if all_empty and sym.get("retry_alt_on_empty") and sym.get("tv_alt"):
                alt = sym["tv_alt"]
                print(f"\n  [!] 所有周期 labels=0，切换备用 symbol: {alt}")
                ok2 = switch_to_symbol(alt)
                if ok2:
                    tv_sym_used = alt
                countdown(SYMBOL_WAIT + 10, f"等待 {alt} 加载 ")  # 多等10s

                for tf in TIMEFRAMES:
                    print(f"\n  [RETRY-ALT] {sym_name}_{tf['name']}")
                    try:
                        # 更新 sym 的 tv 字段以便 extract_one 保存正确 symbol
                        sym_alt = {**sym, "tv": alt}
                        summary = extract_one(cdp, session_id, sym_alt, tf)
                        results[sym_name][tf["name"]] = summary
                    except Exception as e:
                        print(f"    [ERROR] {e}")

    finally:
        cdp.detach(session_id)
        cdp.close()
        print("\n[*] CDP 连接关闭")

    print(f"\n{'='*65}")
    print("[✓] 全部提取完成：")
    for sym_name, tfs in results.items():
        print(f"\n  {sym_name.upper()}:")
        for tf_name, s in tfs.items():
            if "error" in s:
                print(f"    {tf_name:8s}: ERROR - {s['error'][:60]}")
            else:
                print(f"    {tf_name:8s}: "
                      f"labels={s.get('label_count', 0):4d} "
                      f"lines={s.get('line_count', 0):4d} "
                      f"boxes={s.get('box_count', 0):4d}")

    # 保存摘要
    summary_path = OUTPUT_DIR / "hk_index_fetch_summary.json"
    summary_path.write_text(json.dumps({
        "timestamp": time.strftime("%Y-%m-%dT%H:%M:%S"),
        "results": results,
    }, ensure_ascii=False, indent=2))
    print(f"\n[✓] 摘要: {summary_path}")


if __name__ == "__main__":
    main()
