#!/usr/bin/env python3
"""一次性探查：attach 到当前 TV chart，introspect mainSeries bars 结构。"""
import base64, json, os, socket, struct, time, urllib.request

CDP_HOST, CDP_PORT = "localhost", 9222


def connect_ws(host, port, path, timeout=15):
    sock = socket.create_connection((host, port), timeout=timeout)
    sock.settimeout(timeout)
    key = base64.b64encode(os.urandom(16)).decode()
    hs = (f"GET {path} HTTP/1.1\r\nHost: {host}:{port}\r\nUpgrade: websocket\r\n"
          f"Connection: Upgrade\r\nSec-WebSocket-Key: {key}\r\nSec-WebSocket-Version: 13\r\n\r\n")
    sock.sendall(hs.encode())
    resp = b""
    while b"\r\n\r\n" not in resp:
        resp += sock.recv(4096)
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


class CDP:
    def __init__(self, sock):
        self.sock = sock
        self.mid = 0
        self.pending = {}

    def call(self, method, params=None, session_id=None, timeout=30):
        self.mid += 1
        mid = self.mid
        p = {"id": mid, "method": method, "params": params or {}}
        if session_id:
            p["sessionId"] = session_id
        send_frame(self.sock, json.dumps(p).encode())
        deadline = time.time() + timeout
        while time.time() < deadline:
            if mid in self.pending:
                msg = self.pending.pop(mid)
                if "error" in msg:
                    raise RuntimeError(msg["error"])
                return msg.get("result", {})
            self.sock.settimeout(min(1.0, deadline - time.time()))
            try:
                op, raw = recv_frame(self.sock)
                if op == 0x8:
                    break
                m = json.loads(raw)
                if "id" in m:
                    self.pending[m["id"]] = m
            except socket.timeout:
                pass
        raise TimeoutError(method)

    def evaluate(self, expr, session_id):
        r = self.call("Runtime.evaluate", {"expression": expr, "returnByValue": True}, session_id=session_id)
        obj = r.get("result", {})
        if r.get("exceptionDetails"):
            return {"_exc": r["exceptionDetails"].get("exception", {}).get("description", "")[:300]}
        if obj.get("type") == "string":
            try:
                return json.loads(obj["value"])
            except Exception:
                return {"_raw": obj["value"][:500]}
        return {"_obj": str(obj)[:300]}


PROBE_JS = r"""
(function() {
    const out = {steps: []};
    try {
        const wv = window.TradingViewApi._activeChartWidgetWV.value();
        const cw = wv._chartWidget;
        const series = cw.model().model().mainSeries();
        out.steps.push("got mainSeries");
        out.series_methods = Object.getOwnPropertyNames(Object.getPrototypeOf(series)).filter(n => typeof series[n]==='function').slice(0,80);

        // 尝试 bars()
        let bars = null;
        try { bars = series.bars(); out.steps.push("series.bars() ok"); } catch(e){ out.steps.push("bars() err:"+e.message); }
        if (bars) {
            out.bars_methods = Object.getOwnPropertyNames(Object.getPrototypeOf(bars)).filter(n => typeof bars[n]==='function').slice(0,60);
            try { out.bars_size = bars.size(); } catch(e){ out.bars_size_err = e.message; }
            try { const r = bars.range(); out.bars_range = r ? {first: r.firstBar ? r.firstBar() : null, last: r.lastBar ? r.lastBar() : null} : null; } catch(e){ out.range_err = e.message; }
            // 尝试 first/last index
            try { out.first_index = bars.first(); } catch(e){}
            try { out.last_index = bars.last(); } catch(e){}
            // valueAt
            try {
                const fi = bars.first ? bars.first() : 0;
                const sample = bars.valueAt(fi);
                out.sample_first = sample;
                out.sample_first_keys = sample ? Object.keys(sample) : null;
            } catch(e){ out.valueAt_err = e.message; }
            // 尝试 each / 取最后一根
            try {
                const li = bars.last ? bars.last() : null;
                if (li != null) out.sample_last = bars.valueAt(li);
            } catch(e){}
        }
    } catch(e) {
        out.fatal = e.message;
    }
    return JSON.stringify(out);
})()
"""


def main():
    ver = json.load(urllib.request.urlopen(f"http://{CDP_HOST}:{CDP_PORT}/json/version", timeout=5))
    bpath = ver["webSocketDebuggerUrl"].replace(f"ws://{CDP_HOST}:{CDP_PORT}", "")
    sock = connect_ws(CDP_HOST, CDP_PORT, bpath)
    cdp = CDP(sock)
    cdp.call("Target.setDiscoverTargets", {"discover": True})
    pages = json.load(urllib.request.urlopen(f"http://{CDP_HOST}:{CDP_PORT}/json", timeout=5))
    chart = next((p for p in pages if "tradingview.com" in p.get("url", "").lower() and p.get("type") == "page"), None)
    tid = chart.get("targetId") or chart.get("id")
    r = cdp.call("Target.attachToTarget", {"targetId": tid, "flatten": True})
    sid = r["sessionId"]
    cdp.call("Runtime.enable", session_id=sid)
    res = cdp.evaluate(PROBE_JS, sid)
    print(json.dumps(res, ensure_ascii=False, indent=2)[:4000])


if __name__ == "__main__":
    main()
