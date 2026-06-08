#!/usr/bin/env python3
"""
CDP 直连 TradingView Desktop，导航到图表页后提取缠论 Pine label 标注。

策略：
  1. 先尝试 Page.navigate 把 tabbed-window 渲染器导航到 tradingview.com/chart
  2. 若无响应，改用 Target.createTarget 创建新 target
  3. 等待 tradingview.com/chart target 出现 + 页面加载完成
  4. 再等指标初始化（poll window.TradingViewApi）
  5. 提取 Pine labels，保存 JSON

用法:
    /Library/Developer/CommandLineTools/usr/bin/python3 scripts/cdp_extract_labels.py
    /Library/Developer/CommandLineTools/usr/bin/python3 scripts/cdp_extract_labels.py --symbol AAPL
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
OUTPUT_PATH = Path("analysis/data_cache/qqq_chanlun_labels.json")
CALL_TIMEOUT = 25
CHART_LOAD_TIMEOUT = 90   # 等待 chart 页面出现
INDICATOR_TIMEOUT = 60    # 等待指标初始化

SYMBOL = "NASDAQ:QQQ"
for arg in sys.argv[1:]:
    if arg.startswith("--symbol"):
        parts = arg.split("=", 1)
        SYMBOL = parts[1] if len(parts) == 2 else (sys.argv[sys.argv.index(arg) + 1] if sys.argv.index(arg) + 1 < len(sys.argv) else SYMBOL)

CHART_URL = f"https://www.tradingview.com/chart/?symbol={SYMBOL}"


# ── Raw WebSocket（无 Origin 头） ─────────────────────────────────────────────

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
    """Browser-level CDP，支持 flatten sessionId。"""

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
        """取出并清空事件队列（不阻塞）。"""
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

    def call_nowait(self, method: str, params=None, session_id=None):
        """发出命令，不等响应（fire-and-forget）。"""
        self._msg_id += 1
        mid = self._msg_id
        payload = {"id": mid, "method": method, "params": params or {}}
        if session_id:
            payload["sessionId"] = session_id
        send_frame(self._sock, json.dumps(payload).encode())
        return mid

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


# ── HTTP 辅助 ─────────────────────────────────────────────────────────────────

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


# ── 导航策略 ─────────────────────────────────────────────────────────────────

def navigate_tabbed_window(cdp: CDPClient) -> bool:
    """
    找 tabbed-window target，attach，发 Page.navigate。
    不等响应——渲染器可能冻结，但浏览器进程会执行导航。
    返回是否发出了命令。
    """
    pages = get_pages()
    tabbed = next((p for p in pages if "tabbed-window" in p.get("url", "")), None)
    if not tabbed:
        tabbed = next((p for p in pages if p.get("type") == "page" and p.get("webSocketDebuggerUrl")), None)
    if not tabbed:
        return False

    tid = tabbed.get("id") or tabbed.get("targetId")
    print(f"  attach 到 tabbed-window (id={tid[:8]}...)...")
    try:
        # attach（此步可能有响应，因为是 browser-level）
        sid = cdp.attach(tid)
        print(f"  sessionId: {sid}")

        # Page.navigate：fire-and-forget，不等响应
        print(f"  发送 Page.navigate -> {CHART_URL}")
        cdp.call_nowait("Page.navigate", {"url": CHART_URL}, session_id=sid)

        # 也试一次 Target.activateTarget 唤醒渲染器
        cdp.call_nowait("Target.activateTarget", {"targetId": tid})
        return True
    except Exception as e:
        print(f"  tabbed-window navigate error: {e}")
        return False


def create_target(cdp: CDPClient) -> str:
    """用 Target.createTarget 创建新 tab，返回 targetId。"""
    print(f"  Target.createTarget -> {CHART_URL}")
    r = cdp.call("Target.createTarget", {"url": CHART_URL})
    tid = r.get("targetId", "")
    print(f"  新 targetId: {tid}")
    return tid


def wait_for_chart_target(cdp: CDPClient, timeout: int = CHART_LOAD_TIMEOUT) -> dict:
    """
    轮询直到发现 tradingview.com target，返回该 target 信息。
    同时收集 CDP 事件（监听 Target.targetCreated / targetInfoChanged）。
    """
    deadline = time.time() + timeout
    attempt = 0
    while time.time() < deadline:
        # 收事件
        evts = cdp.drain_events()
        for e in evts:
            m = e.get("method", "")
            if m in ("Target.targetCreated", "Target.targetInfoChanged"):
                info = e["params"].get("targetInfo", {})
                if is_chart_target(info):
                    print(f"\n  [事件] 发现图表 target: {info.get('url','')[:80]}")
                    return info

        # 也轮询 /json
        try:
            pages = get_pages()
            for p in pages:
                if is_chart_target(p):
                    return p
        except Exception:
            pass

        remaining = int(deadline - time.time())
        print(f"\r  等待图表加载... ({remaining}s)  ", end="", flush=True)
        time.sleep(2)
        attempt += 1

    print()
    return None


# ── 等待指标初始化 ────────────────────────────────────────────────────────────

WAIT_INDICATOR_JS = r"""
(function() {
    // 检查 TradingView 内部 API 是否已初始化
    const checks = [
        typeof window.TradingViewApi !== 'undefined',
        typeof window.ChartApiInstance !== 'undefined',
        typeof window.tvWidget !== 'undefined',
        document.querySelectorAll('canvas').length > 0,
    ];
    const ready = checks.filter(Boolean).length;
    return JSON.stringify({
        ready: ready,
        total: checks.length,
        tv_api: typeof window.TradingViewApi,
        chart_api: typeof window.ChartApiInstance,
        canvas: document.querySelectorAll('canvas').length,
        url: window.location.href,
        title: document.title,
    });
})()
"""

def wait_for_indicators(cdp: CDPClient, session_id: str, timeout=INDICATOR_TIMEOUT) -> dict:
    """轮询直到 TradingViewApi 或 ChartApiInstance 可用。"""
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
                          f"| canvas={state.get('canvas')} "
                          f"| title={state.get('title','')[:30]}")
                    last_state = state
                if state.get("tv_api") != "undefined" or state.get("chart_api") != "undefined":
                    print(f"\n  [✓] TradingView API 已初始化")
                    return state
                if state.get("canvas", 0) > 0:
                    # canvas 出现说明图表在渲染，继续等一会
                    pass
        except TimeoutError:
            print(f"\n  [!] evaluate 超时，稍等...")
        except Exception as e:
            print(f"\n  [!] {e}")

        remaining = int(deadline - time.time())
        print(f"\r  等待指标初始化... ({remaining}s)  ", end="", flush=True)
        time.sleep(3)

    print()
    return last_state or {}


# ── Pine Labels 提取 ──────────────────────────────────────────────────────────

EXTRACT_PINE_JS = r"""
(function() {
    // 基于 tradingview-mcp buildGraphicsJS 的完整正确路径：
    // chart.model().model().dataSources() + s.metaInfo() + pc.dwglabels.get('labels').get(false)._primitivesDataById

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

            // ── dwglabels (label.new) ──────────────────────────────
            try {
                const outer = pc.dwglabels;
                if (outer) {
                    const inner = outer.get('labels');
                    if (inner) {
                        const coll = inner.get(false);
                        if (coll && coll._primitivesDataById && coll._primitivesDataById.size > 0) {
                            coll._primitivesDataById.forEach(function(v, id) {
                                const text = v.t || v.text || '';
                                const price = v.y != null ? Math.round(v.y * 100) / 100 : null;
                                const time = v.x != null ? v.x : null;
                                out.labels.push({
                                    id: id, study: name, si: si,
                                    text: text, price: price, time: time,
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

            // ── dwglines (line.new) —— 格式: v.x1,y1,x2,y2 ─────────────
            try {
                const outer = pc.dwglines;
                if (outer) {
                    const inner = outer.get('lines');
                    if (inner) {
                        const coll = inner.get(false);
                        if (coll && coll._primitivesDataById && coll._primitivesDataById.size > 0) {
                            coll._primitivesDataById.forEach(function(v, id) {
                                if (v.y1 == null && v.y2 == null) return; // 跳过空行
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

            // ── dwgboxes (box.new) ────────────────────────────────
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

            // ── horizlines (hline) ────────────────────────────────
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


# ── Main ──────────────────────────────────────────────────────────────────────

def main():
    print(f"[*] 目标: {CHART_URL}")
    print(f"[*] 连接 browser-level CDP...")
    browser_path = get_browser_path()
    sock = connect_ws(CDP_HOST, CDP_PORT, browser_path)
    cdp = CDPClient(sock)
    print(f"[✓] Browser connected")

    # 启用 target discovery（接收 targetCreated 事件）
    cdp.call("Target.setDiscoverTargets", {"discover": True})

    try:
        # ── 阶段1：检查是否已有图表 target ────────────────────────────────────
        pages = get_pages()
        chart_target = next((p for p in pages if is_chart_target(p)), None)

        if chart_target:
            print(f"\n[✓] 已有图表 target: {chart_target.get('url','')[:80]}")
        else:
            print(f"\n[*] 没有图表 target，开始导航...")

            # ── 阶段2a：尝试 Page.navigate 到 tabbed-window ────────────────────
            print(f"\n[策略A] Page.navigate on tabbed-window:")
            nav_sent = navigate_tabbed_window(cdp)
            if nav_sent:
                print(f"  导航命令已发出，等待页面加载...")
            else:
                print(f"  没有找到合适的 target 来导航")

            # ── 阶段2b：等待 tradingview.com target 出现（轮询 5s）────────────
            chart_target = wait_for_chart_target(cdp, timeout=10)

            # ── 阶段2c：如果还没有，用 Target.createTarget ────────────────────
            if not chart_target:
                print(f"\n[策略B] Target.createTarget（新建 tab）:")
                new_tid = create_target(cdp)

                if new_tid:
                    print(f"  等待新 target 加载...")
                    chart_target = wait_for_chart_target(cdp, timeout=CHART_LOAD_TIMEOUT)

            if not chart_target:
                print(f"\n[ERROR] {CHART_LOAD_TIMEOUT}s 内未发现图表 target")
                print(f"  当前 targets:")
                for p in get_pages():
                    print(f"    [{p.get('type')}] {p.get('url','')[:80]}")
                sys.exit(1)

        print(f"\n[✓] 使用图表 target:")
        print(f"    URL: {chart_target.get('url','')[:100]}")

        # ── 阶段3：attach 到图表 renderer ────────────────────────────────────
        tid = chart_target.get("targetId") or chart_target.get("id")
        print(f"\n[*] Attach 到图表渲染器...")
        session_id = cdp.attach(tid)
        print(f"    sessionId: {session_id}")

        # Enable Runtime
        print(f"[*] Runtime.enable...")
        cdp.call("Runtime.enable", session_id=session_id)
        print(f"[✓] Runtime.enable OK")

        # ── 阶段4：等待指标初始化 ─────────────────────────────────────────────
        print(f"\n[*] 等待图表和缠论指标初始化...")
        indicator_state = wait_for_indicators(cdp, session_id, timeout=INDICATOR_TIMEOUT)

        if not indicator_state.get("canvas", 0):
            print(f"\n[!] canvas 未出现，图表可能未完全加载")
            print(f"    额外等待 10s...")
            time.sleep(10)

        # ── 阶段5：提取 Pine labels ───────────────────────────────────────────
        print(f"\n[*] 提取 Pine labels...")
        pine = run_js(cdp, EXTRACT_PINE_JS, session_id)

        print(f"\n── 提取结果 ──")
        print(f"  method: {pine.get('method')}")
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
            print(f"\n  缠论 labels（前50条）:")
            for lbl in pine["labels"][:50]:
                print(f"    [{lbl.get('study','?')[:20]}] text={lbl.get('text')!r} "
                      f"price={lbl.get('price')} t={lbl.get('time')}")

        if pine.get("lines"):
            print(f"\n  lines（前20条）:")
            for ln in pine["lines"][:20]:
                print(f"    [{ln.get('study','?')[:20]}] "
                      f"p1={ln.get('price1')} t1={ln.get('time1')} "
                      f"→ p2={ln.get('price2')} t2={ln.get('time2')}")

        if pine.get("errors"):
            print(f"\n  JS errors: {pine['errors'][:5]}")

        # ── 阶段6：保存结果 ───────────────────────────────────────────────────
        output = {
            "timestamp": time.strftime("%Y-%m-%dT%H:%M:%S"),
            "symbol": SYMBOL,
            "chart_url": chart_target.get("url", ""),
            "indicator_state": indicator_state,
            "pine": pine,
            "summary": {
                "label_count": len(pine.get("labels", [])),
                "line_count": len(pine.get("lines", [])),
                "box_count": len(pine.get("boxes", [])),
                "method": pine.get("method"),
                "studies": pine.get("studies", []),
            },
        }

        cdp.detach(session_id)

        OUTPUT_PATH.parent.mkdir(parents=True, exist_ok=True)
        OUTPUT_PATH.write_text(json.dumps(output, ensure_ascii=False, indent=2))
        print(f"\n[✓] 结果保存到 {OUTPUT_PATH}")
        print(f"    labels={output['summary']['label_count']}, "
              f"lines={output['summary']['line_count']}, "
              f"boxes={output['summary']['box_count']}")

    finally:
        cdp.close()
        print("[*] 连接关闭")


if __name__ == "__main__":
    main()
