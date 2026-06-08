#!/usr/bin/env python3
"""
CDP 直连 TV Desktop，切换 QQQ 图表到不同周期后提取缠论标注。

周期切换策略（方法2优先）：
  1. 通过 TradingViewApi 内部 API 调用 setResolution()
  2. 等待 5-10 秒让指标重新计算
  3. 提取 labels/lines/boxes

用法:
    .venv/bin/python scripts/cdp_extract_multiframe.py
    .venv/bin/python scripts/cdp_extract_multiframe.py --timeframes 30,5,15
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
CALL_TIMEOUT = 25
CHART_LOAD_TIMEOUT = 90
INDICATOR_TIMEOUT = 60
RESOLUTION_WAIT = 8  # 切换周期后等待指标计算秒数

OUTPUT_DIR = Path("analysis/data_cache")

TIMEFRAMES = ["30", "5"]
for arg in sys.argv[1:]:
    if arg.startswith("--timeframes"):
        parts = arg.split("=", 1)
        val = parts[1] if len(parts) == 2 else (sys.argv[sys.argv.index(arg) + 1] if sys.argv.index(arg) + 1 < len(sys.argv) else "")
        if val:
            TIMEFRAMES = [t.strip() for t in val.split(",")]

TIMEFRAME_NAMES = {
    "1": "1min", "3": "3min", "5": "5min", "15": "15min",
    "30": "30min", "60": "60min", "240": "4h",
    "D": "daily", "W": "weekly", "M": "monthly",
}


# ── Raw WebSocket ─────────────────────────────────────────────────────────────

def connect_ws(host, port, path, timeout=15) -> socket.socket:
    sock = socket.create_connection((host, port), timeout=timeout)
    sock.settimeout(timeout)
    key = base64.b64encode(os.urandom(16)).decode()
    hs = (
        f"GET {path} HTTP/1.1\r\n"
        f"Host: {host}:{port}\r\n"
        f"Upgrade: websocket\r\n"
        f"Connection: Upgrade\r\n"
        f"Sec-WebSocket-Key: {key}\r\n"
        f"Sec-WebSocket-Version: 13\r\n"
        f"\r\n"
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


def send_frame(sock, payload: bytes, opcode: int = 0x1):
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


def recv_frame(sock) -> tuple:
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
    def __init__(self, sock: socket.socket):
        self._sock = sock
        self._msg_id = 0
        self._pending = {}
        self._events = []

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

    def evaluate(self, expr: str, session_id: str, await_promise=False) -> dict:
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

def http_get(path) -> bytes:
    with urllib.request.urlopen(f"http://{CDP_HOST}:{CDP_PORT}{path}", timeout=5) as r:
        return r.read()


def get_pages() -> list:
    return json.loads(http_get("/json"))


def get_browser_path() -> str:
    ver = json.loads(http_get("/json/version"))
    return ver["webSocketDebuggerUrl"].replace(f"ws://{CDP_HOST}:{CDP_PORT}", "")


def is_chart_target(target: dict) -> bool:
    url = target.get("url", "").lower()
    return "tradingview.com" in url and target.get("type") == "page"


# ── 周期切换 JS ───────────────────────────────────────────────────────────────

def make_set_resolution_js(resolution: str) -> str:
    return f"""
(function() {{
    const res = "{resolution}";
    const tried = [];
    let ok = false;

    // 方法1: TradingViewApi._activeChartWidgetWV
    try {{
        const wv = window.TradingViewApi._activeChartWidgetWV.value();
        if (wv && wv.setResolution) {{
            wv.setResolution(res);
            tried.push("wv.setResolution OK");
            ok = true;
        }}
    }} catch(e) {{ tried.push("wv.setResolution: " + e.message); }}

    // 方法2: tvWidget.activeChart().setResolution()
    if (!ok) {{
        try {{
            const w = window.tvWidget;
            if (w && w.activeChart) {{
                w.activeChart().setResolution(res);
                tried.push("tvWidget.activeChart().setResolution OK");
                ok = true;
            }}
        }} catch(e) {{ tried.push("tvWidget: " + e.message); }}
    }}

    // 方法3: _chartWidget via TradingViewApi
    if (!ok) {{
        try {{
            const wv = window.TradingViewApi._activeChartWidgetWV.value();
            const cw = wv._chartWidget;
            if (cw && cw.model) {{
                cw.model().model().setResolution(res);
                tried.push("chartWidget.model setResolution OK");
                ok = true;
            }}
        }} catch(e) {{ tried.push("chartWidget: " + e.message); }}
    }}

    return JSON.stringify({{ ok, tried, resolution: res }});
}})()
"""


GET_CURRENT_RESOLUTION_JS = r"""
(function() {
    try {
        const wv = window.TradingViewApi._activeChartWidgetWV.value();
        if (wv && wv.getResolution) {
            return JSON.stringify({ resolution: wv.getResolution(), method: "wv.getResolution" });
        }
        if (wv && wv._chartWidget) {
            const model = wv._chartWidget.model().model();
            if (model && model.mainSeries) {
                const res = model.mainSeries().resolution();
                return JSON.stringify({ resolution: res, method: "mainSeries.resolution" });
            }
        }
    } catch(e) {}
    return JSON.stringify({ resolution: null, error: "unknown" });
})()
"""


# ── Pine Labels 提取（复用自 cdp_extract_labels.py）─────────────────────────

EXTRACT_PINE_JS = r"""
(function() {
    const out = {
        labels: [], lines: [], boxes: [], tables: [],
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

            const studyInfo = {idx: si, name, labels: 0, lines: 0, boxes: 0, has_graphics: false};
            const g = s._graphics;
            if (!g || !g._primitivesCollection) {
                out.studies.push(studyInfo);
                continue;
            }
            studyInfo.has_graphics = true;
            const pc = g._primitivesCollection;

            try {
                const outer = pc.dwglabels;
                if (outer) {
                    const inner = outer.get('labels');
                    if (inner) {
                        const coll = inner.get(false);
                        if (coll && coll._primitivesDataById && coll._primitivesDataById.size > 0) {
                            coll._primitivesDataById.forEach(function(v, id) {
                                out.labels.push({
                                    id: id, study: name, si: si,
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
                    }
                }
            } catch(e) { out.errors.push(`s${si}_labels: ${e.message}`); }

            try {
                const outer = pc.dwglines;
                if (outer) {
                    const inner = outer.get('lines');
                    if (inner) {
                        const coll = inner.get(false);
                        if (coll && coll._primitivesDataById && coll._primitivesDataById.size > 0) {
                            coll._primitivesDataById.forEach(function(v, id) {
                                if (v.y1 == null && v.y2 == null) return;
                                out.lines.push({
                                    id: id, study: name, si: si,
                                    price1: v.y1 != null ? Math.round(v.y1 * 100) / 100 : null,
                                    bar1: v.x1,
                                    price2: v.y2 != null ? Math.round(v.y2 * 100) / 100 : null,
                                    bar2: v.x2,
                                    color: v.ci, width: v.w, style: v.st,
                                });
                                studyInfo.lines++;
                            });
                        }
                    }
                }
            } catch(e) { out.errors.push(`s${si}_lines: ${e.message}`); }

            try {
                const outer = pc.dwgboxes;
                if (outer) {
                    const inner = outer.get('boxes');
                    if (inner) {
                        const coll = inner.get(false);
                        if (coll && coll._primitivesDataById && coll._primitivesDataById.size > 0) {
                            coll._primitivesDataById.forEach(function(v, id) {
                                out.boxes.push({
                                    id: id, study: name, si: si,
                                    price1: v.price1 != null ? Math.round(v.price1 * 100) / 100 : null,
                                    price2: v.price2 != null ? Math.round(v.price2 * 100) / 100 : null,
                                    time1: v.time1, time2: v.time2,
                                });
                                studyInfo.boxes++;
                            });
                        }
                    }
                }
            } catch(e) { out.errors.push(`s${si}_boxes: ${e.message}`); }

            try {
                const outer = pc.horizlines;
                if (outer) {
                    const inner = outer.get('horizlines');
                    if (inner) {
                        const coll = inner.get(false);
                        if (coll && coll._primitivesDataById && coll._primitivesDataById.size > 0) {
                            coll._primitivesDataById.forEach(function(v, id) {
                                out.lines.push({
                                    id: id, study: name, si: si, type: 'hline',
                                    price1: v.price != null ? Math.round(v.price * 100) / 100 : null,
                                    price2: null, time1: null, time2: null,
                                });
                            });
                        }
                    }
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


WAIT_INDICATOR_JS = r"""
(function() {
    const checks = [
        typeof window.TradingViewApi !== 'undefined',
        typeof window.ChartApiInstance !== 'undefined',
        typeof window.tvWidget !== 'undefined',
        document.querySelectorAll('canvas').length > 0,
    ];
    return JSON.stringify({
        ready: checks.filter(Boolean).length,
        total: checks.length,
        tv_api: typeof window.TradingViewApi,
        canvas: document.querySelectorAll('canvas').length,
    });
})()
"""


def wait_for_indicators(cdp: CDPClient, session_id: str, timeout=INDICATOR_TIMEOUT) -> dict:
    deadline = time.time() + timeout
    last_state = None
    while time.time() < deadline:
        try:
            result = cdp.evaluate(WAIT_INDICATOR_JS, session_id, await_promise=False)
            obj = result.get("result", {})
            if obj.get("type") == "string":
                state = json.loads(obj["value"])
                if state != last_state:
                    print(f"\n  状态: ready={state.get('ready')}/{state.get('total')} "
                          f"| tv_api={state.get('tv_api')} "
                          f"| canvas={state.get('canvas')}")
                    last_state = state
                if state.get("tv_api") != "undefined":
                    print(f"\n  [✓] TradingView API 已就绪")
                    return state
        except TimeoutError:
            print(f"\n  [!] evaluate 超时...")
        except Exception as e:
            print(f"\n  [!] {e}")
        remaining = int(deadline - time.time())
        print(f"\r  等待指标初始化... ({remaining}s)  ", end="", flush=True)
        time.sleep(3)
    print()
    return last_state or {}


def extract_and_save(cdp: CDPClient, session_id: str, resolution: str, symbol: str):
    """切换周期，等待，提取，保存。"""
    tf_name = TIMEFRAME_NAMES.get(resolution, f"{resolution}min")
    symbol_safe = symbol.replace(":", "_").lower()
    output_path = OUTPUT_DIR / f"{symbol_safe}_{tf_name}_chanlun_labels.json"

    print(f"\n{'='*60}")
    print(f"[*] 切换到周期: {resolution} ({tf_name})")

    # 查当前周期
    try:
        cur = run_js(cdp, GET_CURRENT_RESOLUTION_JS, session_id)
        print(f"    当前周期: {cur.get('resolution')} (via {cur.get('method')})")
        if str(cur.get("resolution")) == str(resolution):
            print(f"    [!] 已是目标周期，直接提取")
    except Exception as e:
        print(f"    [!] 无法获取当前周期: {e}")

    # 切换周期
    print(f"[*] 执行 setResolution({resolution})...")
    try:
        switch_result = run_js(cdp, make_set_resolution_js(resolution), session_id)
        print(f"    ok={switch_result.get('ok')} tried={switch_result.get('tried')}")
        if not switch_result.get("ok"):
            print(f"    [!] 切换失败，仍尝试提取当前数据")
    except Exception as e:
        print(f"    [!] setResolution 异常: {e}")

    # 等待指标重新计算
    print(f"[*] 等待 {RESOLUTION_WAIT}s 让指标计算完成...")
    for i in range(RESOLUTION_WAIT, 0, -1):
        print(f"\r    {i}s...  ", end="", flush=True)
        time.sleep(1)
    print()

    # 验证周期已切换
    try:
        cur = run_js(cdp, GET_CURRENT_RESOLUTION_JS, session_id)
        actual = str(cur.get("resolution", "?"))
        print(f"    确认当前周期: {actual}")
        if actual != str(resolution):
            print(f"    [!] 警告：期望 {resolution}，实际 {actual}，数据可能是旧周期")
    except Exception as e:
        print(f"    [!] 验证周期失败: {e}")

    # 提取 Pine labels
    print(f"[*] 提取 Pine labels...")
    pine = run_js(cdp, EXTRACT_PINE_JS, session_id)

    print(f"\n── {tf_name} 提取结果 ──")
    print(f"  总数据源: {pine.get('total_sources', 0)}")
    print(f"  labels: {len(pine.get('labels', []))}")
    print(f"  lines:  {len(pine.get('lines', []))}")
    print(f"  boxes:  {len(pine.get('boxes', []))}")

    print(f"\n  研究列表:")
    for st in pine.get("studies", []):
        has = "✓" if (st.get("labels", 0) + st.get("lines", 0) + st.get("boxes", 0)) > 0 else " "
        print(f"    {has}[{st.get('idx')}] {st.get('name','?')[:40]} "
              f"labels={st.get('labels',0)} lines={st.get('lines',0)} boxes={st.get('boxes',0)}")

    if pine.get("labels"):
        print(f"\n  缠论 labels（前20条）:")
        for lbl in pine["labels"][:20]:
            print(f"    [{lbl.get('study','?')[:20]}] text={lbl.get('text')!r} "
                  f"price={lbl.get('price')} t={lbl.get('time')}")

    if pine.get("errors"):
        print(f"\n  JS errors: {pine['errors'][:5]}")

    output = {
        "timestamp": time.strftime("%Y-%m-%dT%H:%M:%S"),
        "symbol": symbol,
        "timeframe": resolution,
        "timeframe_name": tf_name,
        "pine": pine,
        "summary": {
            "label_count": len(pine.get("labels", [])),
            "line_count": len(pine.get("lines", [])),
            "box_count": len(pine.get("boxes", [])),
            "studies": pine.get("studies", []),
        },
    }

    OUTPUT_DIR.mkdir(parents=True, exist_ok=True)
    output_path.write_text(json.dumps(output, ensure_ascii=False, indent=2))
    print(f"\n[✓] 保存到 {output_path}")
    return output["summary"]


# ── Main ──────────────────────────────────────────────────────────────────────

def main():
    symbol = "NASDAQ:QQQ"
    print(f"[*] 多周期提取: {symbol}, 周期={TIMEFRAMES}")
    print(f"[*] 连接 browser-level CDP...")
    browser_path = get_browser_path()
    sock = connect_ws(CDP_HOST, CDP_PORT, browser_path)
    cdp = CDPClient(sock)
    print(f"[✓] Browser connected")

    cdp.call("Target.setDiscoverTargets", {"discover": True})

    try:
        pages = get_pages()
        chart_target = next((p for p in pages if is_chart_target(p)), None)
        if not chart_target:
            print("[ERROR] 未找到 TradingView 图表 target，请先在 TV Desktop 打开图表")
            sys.exit(1)

        print(f"\n[✓] 图表 target: {chart_target.get('url','')[:80]}")
        tid = chart_target.get("targetId") or chart_target.get("id")
        session_id = cdp.attach(tid)
        print(f"    sessionId: {session_id}")

        cdp.call("Runtime.enable", session_id=session_id)

        print(f"\n[*] 等待图表 API 就绪...")
        wait_for_indicators(cdp, session_id, timeout=30)

        summaries = {}
        for resolution in TIMEFRAMES:
            try:
                summary = extract_and_save(cdp, session_id, resolution, symbol)
                summaries[resolution] = summary
            except Exception as e:
                print(f"\n[ERROR] 周期 {resolution} 提取失败: {e}")
                summaries[resolution] = {"error": str(e)}

        cdp.detach(session_id)

        print(f"\n{'='*60}")
        print(f"[✓] 所有周期提取完成:")
        for tf, s in summaries.items():
            name = TIMEFRAME_NAMES.get(tf, tf)
            if "error" in s:
                print(f"  {name}: ERROR - {s['error']}")
            else:
                print(f"  {name}: labels={s.get('label_count',0)} "
                      f"lines={s.get('line_count',0)} boxes={s.get('box_count',0)}")

    finally:
        cdp.close()
        print("[*] 连接关闭")


if __name__ == "__main__":
    main()
