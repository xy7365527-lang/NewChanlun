"""Channel Adapter — 外部消息源作为 daemon 事件循环的事件源。

409号推论4：Channel adapter 是 codec 插件，不是独立决策层。
枚举性复杂度不构成主体——WhatsApp/Telegram/Webhook 的格式差异
用 codec 穷举，不产生否定运动。

架构位置：逢亮内部模块，注入 daemon_loop 的 pending_events 队列。
不是独立进程（409号原则6：进程边界是工程投机，模块边界是充分回应）。

纯 Python，零外部依赖。
"""

from __future__ import annotations

import json
import logging
import threading
from abc import ABC, abstractmethod
from collections import deque
from dataclasses import dataclass, field
from datetime import datetime, timezone
from http.server import HTTPServer, BaseHTTPRequestHandler
from typing import Callable

log = logging.getLogger("channel_adapter")


# ---------------------------------------------------------------------------
# 事件结构
# ---------------------------------------------------------------------------

@dataclass(frozen=True)
class ChannelEvent:
    """从外部 channel 进入 daemon 事件循环的规范化事件。

    所有 adapter 产出统一的 ChannelEvent，daemon 不关心来源的格式差异。
    """
    source: str          # adapter 类型标识（"webhook", "telegram", "whatsapp" 等）
    channel_id: str      # 来源 channel 标识（URL path / chat ID 等）
    payload: dict        # 规范化后的消息内容
    raw: str             # 原始消息（用于审计/调试）
    timestamp: str = ""  # ISO 时间戳

    def to_dict(self) -> dict:
        return {
            "type": "channel_event",
            "source": self.source,
            "channel_id": self.channel_id,
            "payload": self.payload,
            "raw": self.raw,
            "timestamp": self.timestamp or datetime.now(timezone.utc).isoformat(),
        }


# ---------------------------------------------------------------------------
# 基类：ChannelAdapter
# ---------------------------------------------------------------------------

class ChannelAdapter(ABC):
    """Channel adapter 抽象基类。

    每个 adapter 实现两件事：
    1. start() — 启动监听（非阻塞，在后台线程/协程中运行）
    2. stop() — 停止监听

    事件通过 _emit(event) 注入队列，daemon 主循环消费队列。
    """

    def __init__(self, event_queue: deque[ChannelEvent]) -> None:
        self._queue = event_queue
        self._running = False

    @abstractmethod
    def start(self) -> None:
        """启动 adapter（非阻塞）。"""

    @abstractmethod
    def stop(self) -> None:
        """停止 adapter。"""

    @property
    def running(self) -> bool:
        return self._running

    def _emit(self, event: ChannelEvent) -> None:
        """将事件注入队列。线程安全（deque.append 是原子的）。"""
        self._queue.append(event)
        log.info("channel event from %s/%s queued (queue size: %d)",
                 event.source, event.channel_id, len(self._queue))


# ---------------------------------------------------------------------------
# WebhookAdapter — HTTP webhook 接收外部消息
# ---------------------------------------------------------------------------

class _WebhookHandler(BaseHTTPRequestHandler):
    """Webhook HTTP handler — 接收 POST 请求并注入事件队列。"""

    adapter: WebhookAdapter | None = None  # 由 WebhookAdapter.start() 设置

    def do_POST(self) -> None:
        content_length = int(self.headers.get("Content-Length", 0))
        body = self.rfile.read(content_length) if content_length > 0 else b""

        # 解析 JSON body
        try:
            data = json.loads(body) if body else {}
        except json.JSONDecodeError:
            self._respond(400, {"error": "invalid JSON"})
            return

        # 从 URL path 提取 channel_id（如 /webhook/telegram → "telegram"）
        path = self.path.strip("/")
        parts = path.split("/", 1)
        channel_id = parts[1] if len(parts) > 1 else "default"

        # 规范化：payload 中提取 text 字段（如果存在）
        payload = _normalize_webhook_payload(data)

        event = ChannelEvent(
            source="webhook",
            channel_id=channel_id,
            payload=payload,
            raw=body.decode("utf-8", errors="replace"),
        )

        self.adapter._emit(event)
        self._respond(200, {"accepted": True, "channel_id": channel_id})

    def do_GET(self) -> None:
        """Health check endpoint."""
        self._respond(200, {
            "status": "ok",
            "adapter": "webhook",
            "queue_size": len(self.adapter._queue) if self.adapter else 0,
        })

    def _respond(self, status: int, data: dict) -> None:
        body = json.dumps(data, ensure_ascii=False).encode("utf-8")
        self.send_response(status)
        self.send_header("Content-Type", "application/json; charset=utf-8")
        self.send_header("Content-Length", str(len(body)))
        self.end_headers()
        self.wfile.write(body)

    def log_message(self, format, *args) -> None:
        """Suppress default HTTP logging — 用 channel_adapter logger。"""
        pass


def _normalize_webhook_payload(data: dict) -> dict:
    """规范化 webhook payload — 提取通用字段。

    不同 webhook 源的格式差异在这里穷举（409号：枚举性复杂度）。
    """
    result: dict = {}

    # 通用 text 字段提取
    for key in ("text", "message", "content", "body"):
        if key in data and isinstance(data[key], str):
            result["text"] = data[key]
            break

    # 嵌套 message 对象（Telegram 风格）
    if "text" not in result and "message" in data and isinstance(data["message"], dict):
        msg = data["message"]
        for key in ("text", "content"):
            if key in msg and isinstance(msg[key], str):
                result["text"] = msg[key]
                break

    # sender 信息
    for key in ("sender", "from", "user", "author"):
        if key in data:
            result["sender"] = str(data[key])
            break

    # 保留原始 payload 的其余字段
    result["_original_keys"] = list(data.keys())

    return result


class WebhookAdapter(ChannelAdapter):
    """HTTP webhook adapter — 在指定端口监听 POST 请求。

    POST /webhook/<channel_id> — 接收消息并注入事件队列
    GET /webhook/ — 健康检查

    用法：
        queue = deque()
        adapter = WebhookAdapter(queue, port=9800)
        adapter.start()  # 非阻塞
        # ... daemon 循环中消费 queue ...
        adapter.stop()
    """

    def __init__(self, event_queue: deque[ChannelEvent], port: int = 9800) -> None:
        super().__init__(event_queue)
        self._port = port
        self._server: HTTPServer | None = None
        self._thread: threading.Thread | None = None

    def start(self) -> None:
        if self._running:
            return

        _WebhookHandler.adapter = self
        self._server = HTTPServer(("0.0.0.0", self._port), _WebhookHandler)
        self._thread = threading.Thread(
            target=self._server.serve_forever,
            daemon=True,
            name="webhook-adapter",
        )
        self._thread.start()
        self._running = True
        log.info("WebhookAdapter started on port %d", self._port)

    def stop(self) -> None:
        if not self._running:
            return

        if self._server:
            self._server.shutdown()
            self._server = None
        self._running = False
        log.info("WebhookAdapter stopped")


# ---------------------------------------------------------------------------
# EventSourceManager — 管理多个 adapter 实例
# ---------------------------------------------------------------------------

class EventSourceManager:
    """管理所有 channel adapter 实例 + 提供统一事件队列。

    daemon_loop 通过此管理器：
    1. 注册 adapter 实例
    2. 启动/停止所有 adapter
    3. drain() 从队列取出所有待处理事件
    """

    def __init__(self) -> None:
        self._queue: deque[ChannelEvent] = deque()
        self._adapters: list[ChannelAdapter] = []

    @property
    def queue(self) -> deque[ChannelEvent]:
        return self._queue

    def register(self, adapter: ChannelAdapter) -> None:
        """注册一个 adapter（adapter 的 queue 必须是本管理器的 queue）。"""
        if adapter._queue is not self._queue:
            raise ValueError("adapter must share the same event queue")
        self._adapters.append(adapter)

    def create_webhook(self, port: int = 9800) -> WebhookAdapter:
        """创建并注册一个 WebhookAdapter。"""
        adapter = WebhookAdapter(self._queue, port=port)
        self._adapters.append(adapter)
        return adapter

    def start_all(self) -> None:
        """启动所有已注册的 adapter。"""
        for adapter in self._adapters:
            if not adapter.running:
                adapter.start()

    def stop_all(self) -> None:
        """停止所有已注册的 adapter。"""
        for adapter in self._adapters:
            if adapter.running:
                adapter.stop()

    def drain(self) -> list[dict]:
        """取出队列中所有待处理事件，返回 dict 列表（可直接注入 handoff pending_events）。"""
        events: list[dict] = []
        while self._queue:
            event = self._queue.popleft()
            events.append(event.to_dict())
        return events
