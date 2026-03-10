"""ACP Bridge — Agent Client Protocol 事件源。

409号推论5：ACP bridge 通过 stdin/stdout 与外部 IDE/工具通信。
粗粒度任务级 RPC（不是 token 级流式），注入 daemon 事件循环。

ACP = Agent Client Protocol（stdio NDJSON）。
每行一个 JSON 对象，以换行符分隔（Newline-Delimited JSON）。

架构位置：逢亮内部模块（与 channel_adapter 并列的事件源）。
不是独立进程（409号原则6）。

通信协议：
    → 客户端发送请求（一行 JSON）
    ← 服务端回复响应（一行 JSON）

请求格式：
    {"id": "req-1", "method": "task", "params": {"text": "..."}}
    {"id": "req-2", "method": "status"}
    {"id": "req-3", "method": "feed", "params": {"text": "..."}}

响应格式：
    {"id": "req-1", "result": {...}}
    {"id": "req-2", "error": {"code": -1, "message": "..."}}

纯 Python，零外部依赖。
"""

from __future__ import annotations

import io
import json
import logging
import sys
import threading
from collections import deque
from dataclasses import dataclass
from datetime import datetime, timezone
from typing import TextIO

from channel_adapter import ChannelAdapter, ChannelEvent

log = logging.getLogger("acp_bridge")


# ---------------------------------------------------------------------------
# ACP 消息结构
# ---------------------------------------------------------------------------

@dataclass(frozen=True)
class ACPRequest:
    """ACP 请求（客户端 → 逢亮）。"""
    id: str
    method: str
    params: dict

    @classmethod
    def from_line(cls, line: str) -> ACPRequest | None:
        """从 NDJSON 行解析请求。格式不合法返回 None。"""
        try:
            data = json.loads(line)
        except json.JSONDecodeError:
            return None

        req_id = data.get("id", "")
        method = data.get("method", "")
        if not req_id or not method:
            return None

        return cls(
            id=str(req_id),
            method=method,
            params=data.get("params", {}),
        )


@dataclass(frozen=True)
class ACPResponse:
    """ACP 响应（逢亮 → 客户端）。"""
    id: str
    result: dict | None = None
    error: dict | None = None

    def to_line(self) -> str:
        """序列化为 NDJSON 行。"""
        data: dict = {"id": self.id}
        if self.error is not None:
            data["error"] = self.error
        else:
            data["result"] = self.result or {}
        return json.dumps(data, ensure_ascii=False)


# ---------------------------------------------------------------------------
# ACP 方法路由
# ---------------------------------------------------------------------------

# 支持的方法及其描述
_ACP_METHODS = {
    "task": "提交任务（粗粒度 RPC）",
    "status": "查询 daemon 状态",
    "feed": "注入文本到 S_net",
    "ping": "连通性检测",
}


def _handle_ping(req: ACPRequest) -> ACPResponse:
    """ping — 连通性检测。"""
    return ACPResponse(id=req.id, result={"pong": True})


def _handle_unknown(req: ACPRequest) -> ACPResponse:
    """未知方法。"""
    return ACPResponse(
        id=req.id,
        error={
            "code": -32601,
            "message": f"unknown method: {req.method}",
            "supported": list(_ACP_METHODS.keys()),
        },
    )


# ---------------------------------------------------------------------------
# ACPBridge — 作为 ChannelAdapter 接入事件循环
# ---------------------------------------------------------------------------

class ACPBridge(ChannelAdapter):
    """ACP Bridge — 通过 stdin/stdout NDJSON 与外部 IDE/工具通信。

    task 和 feed 方法产生 ChannelEvent，注入 daemon 事件队列。
    status 和 ping 直接回复，不进入事件队列。

    用法（作为 daemon 内部模块）：
        queue = deque()
        bridge = ACPBridge(queue)
        bridge.start()  # 非阻塞，在后台线程读 stdin

    用法（外部 IDE 通过 pipe 连接）：
        echo '{"id":"1","method":"ping","params":{}}' | python -c "
        from acp_bridge import ACPBridge
        ..."
    """

    def __init__(
        self,
        event_queue: deque[ChannelEvent],
        input_stream: TextIO | None = None,
        output_stream: TextIO | None = None,
    ) -> None:
        super().__init__(event_queue)
        self._input = input_stream or sys.stdin
        self._output = output_stream or sys.stdout
        self._thread: threading.Thread | None = None
        self._output_lock = threading.Lock()

    def start(self) -> None:
        if self._running:
            return

        self._running = True
        self._thread = threading.Thread(
            target=self._read_loop,
            daemon=True,
            name="acp-bridge",
        )
        self._thread.start()
        log.info("ACPBridge started (reading from %s)", self._input.name if hasattr(self._input, 'name') else 'stream')

    def stop(self) -> None:
        self._running = False
        log.info("ACPBridge stopped")

    def _read_loop(self) -> None:
        """持续读取 stdin，解析 NDJSON 请求。"""
        while self._running:
            try:
                line = self._input.readline()
            except (IOError, ValueError):
                # stdin 关闭或不可读
                log.info("ACP input stream closed")
                self._running = False
                break

            if not line:
                # EOF
                log.info("ACP input stream EOF")
                self._running = False
                break

            line = line.strip()
            if not line:
                continue

            req = ACPRequest.from_line(line)
            if req is None:
                self._send_response(ACPResponse(
                    id="?",
                    error={"code": -32700, "message": "parse error"},
                ))
                continue

            self._dispatch(req)

    def _dispatch(self, req: ACPRequest) -> None:
        """路由请求到对应处理器。"""
        if req.method == "ping":
            self._send_response(_handle_ping(req))

        elif req.method == "status":
            # status 直接回复当前队列状态（不进入事件队列）
            self._send_response(ACPResponse(
                id=req.id,
                result={
                    "adapter": "acp",
                    "running": self._running,
                    "queue_size": len(self._queue),
                },
            ))

        elif req.method in ("task", "feed"):
            # task 和 feed 产生 ChannelEvent，注入事件队列
            event = ChannelEvent(
                source="acp",
                channel_id=req.method,
                payload=req.params,
                raw=json.dumps({"id": req.id, "method": req.method, "params": req.params},
                               ensure_ascii=False),
            )
            self._emit(event)

            # 回复确认（事件已入队，实际处理由 daemon 主循环完成）
            self._send_response(ACPResponse(
                id=req.id,
                result={
                    "accepted": True,
                    "method": req.method,
                    "queued": True,
                },
            ))

        else:
            self._send_response(_handle_unknown(req))

    def _send_response(self, resp: ACPResponse) -> None:
        """向 output stream 写入一行 NDJSON 响应。线程安全。"""
        with self._output_lock:
            try:
                self._output.write(resp.to_line() + "\n")
                self._output.flush()
            except (IOError, ValueError):
                log.warning("ACP output stream write failed")
