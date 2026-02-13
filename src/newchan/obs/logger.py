"""结构化 JSON 日志

轻量封装 Python logging，输出 JSON 格式日志，
便于后续 ELK/Loki 等日志系统接入。
"""

from __future__ import annotations

import json
import logging
from typing import Any


class _JsonFormatter(logging.Formatter):
    """将日志记录格式化为单行 JSON。"""

    def format(self, record: logging.LogRecord) -> str:
        log_entry: dict[str, Any] = {
            "ts": self.formatTime(record),
            "level": record.levelname,
            "logger": record.name,
            "msg": record.getMessage(),
        }
        # 附加结构化字段
        extra = getattr(record, "_structured_extra", None)
        if extra:
            log_entry.update(extra)
        return json.dumps(log_entry, default=str, ensure_ascii=False)


class StructuredLogger:
    """结构化 JSON 日志封装。

    Usage::

        log = StructuredLogger("newchan.engine")
        log.event("bar processed", bar_idx=42, tf="5m", event_count=3)
    """

    def __init__(self, name: str) -> None:
        self._logger = logging.getLogger(name)
        if not self._logger.handlers:
            handler = logging.StreamHandler()
            handler.setFormatter(_JsonFormatter())
            self._logger.addHandler(handler)
            self._logger.setLevel(logging.DEBUG)

    def event(
        self,
        msg: str,
        *,
        bar_idx: int = -1,
        tf: str = "",
        event_count: int = 0,
        **extra: Any,
    ) -> None:
        """记录一条结构化事件日志。"""
        structured = {"bar_idx": bar_idx, "tf": tf, "event_count": event_count}
        structured.update(extra)
        record = self._logger.makeRecord(
            self._logger.name, logging.INFO, "(structured)", 0,
            msg, args=(), exc_info=None,
        )
        record._structured_extra = structured  # type: ignore[attr-defined]
        self._logger.handle(record)

    def warning(self, msg: str, **extra: Any) -> None:
        """记录警告日志。"""
        record = self._logger.makeRecord(
            self._logger.name, logging.WARNING, "(structured)", 0,
            msg, args=(), exc_info=None,
        )
        record._structured_extra = extra  # type: ignore[attr-defined]
        self._logger.handle(record)

    def error(self, msg: str, **extra: Any) -> None:
        """记录错误日志。"""
        record = self._logger.makeRecord(
            self._logger.name, logging.ERROR, "(structured)", 0,
            msg, args=(), exc_info=None,
        )
        record._structured_extra = extra  # type: ignore[attr-defined]
        self._logger.handle(record)
