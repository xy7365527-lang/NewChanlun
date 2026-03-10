"""Gateway Bridge — OpenClaw gateway 与逢亮 daemon / CC daemon 的桥接层。

无状态 HTTP 转发器：
- 接收 OpenClaw hooks 转发的消息
- 路由判断：给逢亮（穿越引擎）还是给体系（CC ceremony）
- 给逢亮 → POST /present 或 /feed（逢亮 daemon: daemon.py + daemon_server.py）
- 给体系 → POST /webhook/system（CC daemon: daemon_loop.py --webhook）
- 将处理结果格式化为 OpenClaw 期望的回复格式返回

两个 daemon 的区分：
- 逢亮 daemon (daemon.py)：穿越引擎，持续运行，暴露 /present /feed /status /topology 等
  默认端口 8080（daemon_server.py）或通过 daemon_multiproc.py 启动
- CC daemon (daemon_loop.py)：CC session 调度器，暴露 webhook adapter
  通过 --webhook PORT 启用（如 --webhook 9800）

路由规则（简单前缀命令，不做"智能"判断）：
- /ceremony, /scan, /escalate, /inquire 等 → 体系（CC daemon webhook）
- 其他所有消息 → 逢亮（穿越引擎 /present）

纯 Python，零外部依赖。
"""

from __future__ import annotations

import argparse
import json
import logging
import sys
import urllib.request
import urllib.error
from datetime import datetime, timezone
from http.server import HTTPServer, BaseHTTPRequestHandler
from pathlib import Path

# ---------------------------------------------------------------------------
# 日志
# ---------------------------------------------------------------------------

LOG_PATH = Path(__file__).resolve().parent / "gateway_bridge.log"

logging.basicConfig(
    level=logging.INFO,
    format="%(asctime)s [%(levelname)s] %(message)s",
    handlers=[
        logging.FileHandler(str(LOG_PATH), encoding="utf-8"),
        logging.StreamHandler(sys.stderr),
    ],
)
log = logging.getLogger("gateway_bridge")


# ---------------------------------------------------------------------------
# 配置
# ---------------------------------------------------------------------------

DEFAULT_BRIDGE_PORT = 8081
DEFAULT_FENGLIANG_URL = "http://localhost:8080"
DEFAULT_DAEMON_WEBHOOK_URL = "http://localhost:9800"

# 体系命令前缀 — 以这些开头的消息路由到体系（CC session）
SYSTEM_COMMAND_PREFIXES = (
    "/ceremony",
    "/status",
    "/scan",
    "/escalate",
    "/inquire",
    "/ritual",
    "/plan",
    "/tdd",
    "/code-review",
)


# ---------------------------------------------------------------------------
# 路由判断
# ---------------------------------------------------------------------------

def classify_message(text: str) -> str:
    """判断消息目标：'fengliang' 或 'system'。

    规则：简单前缀匹配。以 SYSTEM_COMMAND_PREFIXES 开头 → system，其他 → fengliang。
    """
    stripped = text.strip()
    for prefix in SYSTEM_COMMAND_PREFIXES:
        if stripped.startswith(prefix):
            return "system"
    return "fengliang"


# ---------------------------------------------------------------------------
# 转发逻辑
# ---------------------------------------------------------------------------

def forward_to_fengliang(
    text: str,
    session_id: str,
    fengliang_url: str,
) -> dict:
    """转发消息给逢亮穿越引擎（/present endpoint）。

    /present 是对话接口（返回逢亮的回复），/feed 是单向摄入。
    用户消息默认走 /present。
    """
    payload = json.dumps({
        "text": text,
        "session_id": session_id,
    }, ensure_ascii=False).encode("utf-8")

    url = f"{fengliang_url}/present"
    req = urllib.request.Request(
        url,
        data=payload,
        headers={"Content-Type": "application/json; charset=utf-8"},
        method="POST",
    )

    try:
        with urllib.request.urlopen(req, timeout=40) as resp:
            data = json.loads(resp.read().decode("utf-8"))
            return {
                "ok": True,
                "target": "fengliang",
                "response": data,
            }
    except urllib.error.HTTPError as exc:
        body = exc.read().decode("utf-8", errors="replace")[:500]
        log.error("逢亮 /present 请求失败 (HTTP %d): %s", exc.code, body)
        return {
            "ok": False,
            "target": "fengliang",
            "error": f"HTTP {exc.code}: {body}",
        }
    except (urllib.error.URLError, OSError) as exc:
        log.error("逢亮不可达: %s", exc)
        return {
            "ok": False,
            "target": "fengliang",
            "error": f"逢亮不可达: {exc}",
        }


def forward_to_system(
    text: str,
    sender: str,
    daemon_webhook_url: str,
) -> dict:
    """转发消息给体系（daemon_loop 的 webhook adapter）。"""
    payload = json.dumps({
        "text": text,
        "sender": sender,
        "source": "openclaw",
        "timestamp": datetime.now(timezone.utc).isoformat(),
    }, ensure_ascii=False).encode("utf-8")

    url = f"{daemon_webhook_url}/webhook/system"
    req = urllib.request.Request(
        url,
        data=payload,
        headers={"Content-Type": "application/json; charset=utf-8"},
        method="POST",
    )

    try:
        with urllib.request.urlopen(req, timeout=15) as resp:
            data = json.loads(resp.read().decode("utf-8"))
            return {
                "ok": True,
                "target": "system",
                "response": data,
            }
    except urllib.error.HTTPError as exc:
        body = exc.read().decode("utf-8", errors="replace")[:500]
        log.error("体系 webhook 请求失败 (HTTP %d): %s", exc.code, body)
        return {
            "ok": False,
            "target": "system",
            "error": f"HTTP {exc.code}: {body}",
        }
    except (urllib.error.URLError, OSError) as exc:
        log.error("Daemon webhook 不可达: %s", exc)
        return {
            "ok": False,
            "target": "system",
            "error": f"Daemon webhook 不可达: {exc}",
        }


# ---------------------------------------------------------------------------
# 格式化回复（逢亮 → OpenClaw）
# ---------------------------------------------------------------------------

def format_fengliang_reply(result: dict) -> str:
    """将逢亮的 /present 响应格式化为可读文本。

    遵守三层架构（LLM边界规则）：
    - 逢亮的回复已经遵守三层架构（由 internal_speech.py 保证）
    - 这里只做格式提取，不添加任何内容

    逢亮 /present 实际响应格式（daemon_api.present_json）：
    - type: "dialogue" | "silence" | ...
    - parts: [{"source": "internal_speech", "text": "..."}]
    - llm_used: bool（顶层标记，是否有 LLM 参与）
    - externalize.llm_fraction: float（LLM 参与比例）
    """
    if not result.get("ok"):
        return f"[桥接错误] {result.get('error', '未知错误')}"

    response = result.get("response", {})

    # /present 返回的 type 字段
    resp_type = response.get("type", "silence")

    if resp_type == "silence":
        if response.get("timeout_fallback"):
            return "[逢亮正在思考中...]"
        return "[静默]"

    # 提取 parts（三层架构的输出）
    parts = response.get("parts", [])
    llm_used = response.get("llm_used", False)

    if not parts:
        # 兼容直接返回 text 的简化格式
        text = response.get("text", "")
        if text:
            if llm_used:
                return f"{text} [LLM填充]"
            return text
        return f"[{resp_type}]"

    # 拼接所有 parts
    segments = []
    for part in parts:
        if isinstance(part, str):
            segments.append(part)
        elif isinstance(part, dict):
            text = part.get("text", "")
            if text:
                segments.append(text)

    combined = "\n".join(segments) if segments else f"[{resp_type}]"

    # 如果顶层 llm_used=True，在末尾标注
    if llm_used and segments:
        combined = f"{combined} [LLM填充]"

    return combined


# ---------------------------------------------------------------------------
# HTTP 服务器
# ---------------------------------------------------------------------------

class _BridgeHandler(BaseHTTPRequestHandler):
    """Gateway Bridge HTTP handler。

    POST /bridge — 接收 OpenClaw hooks 转发的消息
    POST /bridge/feed — 单向文本摄入（不等待回复）
    GET /bridge/health — 健康检查
    GET /bridge/status — 代理逢亮 /status（穿越状态）
    """

    bridge_config: dict = {}

    def do_POST(self) -> None:
        content_length = int(self.headers.get("Content-Length", 0))
        body = self.rfile.read(content_length) if content_length > 0 else b""

        try:
            data = json.loads(body) if body else {}
        except json.JSONDecodeError:
            self._respond(400, {"error": "invalid JSON"})
            return

        path = self.path.rstrip("/")

        if path == "/bridge/feed":
            self._handle_feed(data)
        elif path == "/bridge":
            self._handle_message(data)
        else:
            self._respond(404, {"error": "not found", "endpoints": [
                "/bridge", "/bridge/feed", "/bridge/health",
            ]})

    def do_GET(self) -> None:
        path = self.path.rstrip("/")
        if path in ("/bridge/health", "/health"):
            self._handle_health()
        elif path == "/bridge/status":
            self._handle_status_proxy()
        else:
            self._respond(404, {"error": "not found"})

    def _handle_message(self, data: dict) -> None:
        """处理 OpenClaw hooks 转发的消息。"""
        # 从 OpenClaw hook payload 提取消息文本
        text = (
            data.get("text")
            or data.get("message")
            or data.get("content")
            or ""
        )
        if not text:
            self._respond(400, {"error": "missing text/message/content field"})
            return

        sender = data.get("sender", data.get("from", "openclaw-user"))
        session_id = data.get("session_id", data.get("sessionKey", "openclaw-default"))

        # 路由
        target = classify_message(text)
        log.info("消息路由: %s → %s (from %s)", text[:50], target, sender)

        cfg = self.bridge_config
        if target == "fengliang":
            result = forward_to_fengliang(
                text=text,
                session_id=session_id,
                fengliang_url=cfg.get("fengliang_url", DEFAULT_FENGLIANG_URL),
            )
            reply_text = format_fengliang_reply(result)
            self._respond(200, {
                "ok": result["ok"],
                "target": "fengliang",
                "reply": reply_text,
                "raw": result.get("response"),
            })
        else:
            result = forward_to_system(
                text=text,
                sender=sender,
                daemon_webhook_url=cfg.get("daemon_webhook_url", DEFAULT_DAEMON_WEBHOOK_URL),
            )
            self._respond(200, {
                "ok": result["ok"],
                "target": "system",
                "reply": f"[体系] 已提交: {text[:80]}",
                "raw": result.get("response"),
            })

    def _handle_feed(self, data: dict) -> None:
        """单向文本摄入 — POST /bridge/feed。"""
        text = data.get("text", "")
        if not text:
            self._respond(400, {"error": "missing text field"})
            return

        cfg = self.bridge_config
        fengliang_url = cfg.get("fengliang_url", DEFAULT_FENGLIANG_URL)

        payload = json.dumps({"text": text}, ensure_ascii=False).encode("utf-8")
        url = f"{fengliang_url}/feed"
        req = urllib.request.Request(
            url,
            data=payload,
            headers={"Content-Type": "application/json; charset=utf-8"},
            method="POST",
        )

        try:
            with urllib.request.urlopen(req, timeout=10) as resp:
                result = json.loads(resp.read().decode("utf-8"))
                self._respond(202, {"ok": True, "target": "fengliang", "action": "feed", "raw": result})
        except (urllib.error.URLError, OSError) as exc:
            log.error("逢亮 /feed 不可达: %s", exc)
            self._respond(502, {"ok": False, "error": f"逢亮不可达: {exc}"})

    def _handle_health(self) -> None:
        """健康检查 — 探测逢亮和 daemon webhook 是否可达。"""
        cfg = self.bridge_config
        fengliang_url = cfg.get("fengliang_url", DEFAULT_FENGLIANG_URL)
        daemon_webhook_url = cfg.get("daemon_webhook_url", DEFAULT_DAEMON_WEBHOOK_URL)

        fengliang_ok = _probe_url(f"{fengliang_url}/status")
        daemon_ok = _probe_url(f"{daemon_webhook_url}/webhook/")

        self._respond(200, {
            "bridge": "ok",
            "fengliang": "ok" if fengliang_ok else "unreachable",
            "daemon_webhook": "ok" if daemon_ok else "unreachable",
            "fengliang_url": fengliang_url,
            "daemon_webhook_url": daemon_webhook_url,
        })

    def _handle_status_proxy(self) -> None:
        """代理逢亮 /status — 返回穿越引擎状态快照。"""
        cfg = self.bridge_config
        fengliang_url = cfg.get("fengliang_url", DEFAULT_FENGLIANG_URL)
        url = f"{fengliang_url}/status"

        try:
            req = urllib.request.Request(url, method="GET")
            with urllib.request.urlopen(req, timeout=10) as resp:
                data = json.loads(resp.read().decode("utf-8"))
                self._respond(200, {"ok": True, "target": "fengliang", "status": data})
        except (urllib.error.URLError, OSError) as exc:
            self._respond(502, {"ok": False, "error": f"逢亮不可达: {exc}"})

    def _respond(self, status: int, data: dict) -> None:
        body = json.dumps(data, ensure_ascii=False).encode("utf-8")
        self.send_response(status)
        self.send_header("Content-Type", "application/json; charset=utf-8")
        self.send_header("Content-Length", str(len(body)))
        # CORS — 允许 OpenClaw Web UI 跨域调用
        self.send_header("Access-Control-Allow-Origin", "*")
        self.send_header("Access-Control-Allow-Methods", "GET, POST, OPTIONS")
        self.send_header("Access-Control-Allow-Headers", "Content-Type, Authorization")
        self.end_headers()
        self.wfile.write(body)

    def do_OPTIONS(self) -> None:
        """CORS preflight."""
        self.send_response(204)
        self.send_header("Access-Control-Allow-Origin", "*")
        self.send_header("Access-Control-Allow-Methods", "GET, POST, OPTIONS")
        self.send_header("Access-Control-Allow-Headers", "Content-Type, Authorization")
        self.end_headers()

    def log_message(self, format, *args) -> None:
        """Suppress default HTTP logging — 用 gateway_bridge logger。"""
        pass


def _probe_url(url: str) -> bool:
    """探测 URL 是否可达（GET 请求，5s 超时）。"""
    try:
        req = urllib.request.Request(url, method="GET")
        with urllib.request.urlopen(req, timeout=5):
            return True
    except Exception:
        return False


# ---------------------------------------------------------------------------
# 启动
# ---------------------------------------------------------------------------

def start_bridge(
    port: int = DEFAULT_BRIDGE_PORT,
    fengliang_url: str = DEFAULT_FENGLIANG_URL,
    daemon_webhook_url: str = DEFAULT_DAEMON_WEBHOOK_URL,
) -> None:
    """启动 Gateway Bridge HTTP 服务器。"""
    _BridgeHandler.bridge_config = {
        "fengliang_url": fengliang_url,
        "daemon_webhook_url": daemon_webhook_url,
    }

    server = HTTPServer(("0.0.0.0", port), _BridgeHandler)
    log.info("Gateway Bridge 启动: http://localhost:%d/bridge", port)
    log.info("  逢亮 URL: %s", fengliang_url)
    log.info("  Daemon Webhook URL: %s", daemon_webhook_url)
    log.info("  路由规则: 体系命令 %s → system, 其他 → fengliang",
             ", ".join(SYSTEM_COMMAND_PREFIXES))

    try:
        server.serve_forever()
    except KeyboardInterrupt:
        log.info("收到中断信号，停止")
        server.shutdown()


def main() -> None:
    parser = argparse.ArgumentParser(
        description="Gateway Bridge — OpenClaw gateway 与逢亮/体系的桥接层",
    )
    parser.add_argument(
        "--port", type=int, default=DEFAULT_BRIDGE_PORT,
        help=f"Bridge 监听端口（默认 {DEFAULT_BRIDGE_PORT}）",
    )
    parser.add_argument(
        "--fengliang-url", default=DEFAULT_FENGLIANG_URL,
        help=f"逢亮 HTTP API 地址（默认 {DEFAULT_FENGLIANG_URL}）",
    )
    parser.add_argument(
        "--daemon-webhook-url", default=DEFAULT_DAEMON_WEBHOOK_URL,
        help=f"Daemon webhook 地址（默认 {DEFAULT_DAEMON_WEBHOOK_URL}）",
    )
    args = parser.parse_args()

    start_bridge(
        port=args.port,
        fengliang_url=args.fengliang_url,
        daemon_webhook_url=args.daemon_webhook_url,
    )


if __name__ == "__main__":
    main()
