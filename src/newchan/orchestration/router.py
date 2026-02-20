"""orchestration/router.py — 声明式事件路由匹配器

读取 dispatch-spec.yaml 中的 orchestration_protocol.routes，
提供 match_event() 纯函数进行路由匹配。不执行动作——执行由调用方负责。
"""

from __future__ import annotations

from dataclasses import dataclass
from fnmatch import fnmatch
from pathlib import Path
from typing import Any

import yaml


# --- 优先级排序权重 ---
_PRIORITY_ORDER: dict[str, int] = {
    "critical": 0,
    "high": 1,
    "medium": 2,
    "low_async": 3,
}


@dataclass(frozen=True, slots=True)
class Route:
    """一条不可变路由规则。"""

    event: str
    pattern: str
    target: str
    action: str
    priority: str
    condition: str

    @property
    def priority_rank(self) -> int:
        return _PRIORITY_ORDER.get(self.priority, 99)


def load_routes(spec_path: Path | str) -> tuple[Route, ...]:
    """从 dispatch-spec.yaml 加载 orchestration_protocol.routes。

    返回按 priority 排序的不可变元组。
    """
    path = Path(spec_path)
    if not path.exists():
        msg = f"dispatch-spec.yaml not found: {path}"
        raise FileNotFoundError(msg)

    with path.open(encoding="utf-8") as f:
        data = yaml.safe_load(f)

    protocol = data.get("orchestration_protocol")
    if protocol is None:
        msg = "orchestration_protocol section not found in dispatch-spec.yaml"
        raise ValueError(msg)

    raw_routes = protocol.get("routes", [])
    routes = tuple(
        Route(
            event=r["event"],
            pattern=r.get("pattern", ""),
            target=r["target"],
            action=r["action"],
            priority=r.get("priority", "medium"),
            condition=r.get("condition", ""),
        )
        for r in raw_routes
    )
    return tuple(sorted(routes, key=lambda rt: rt.priority_rank))


def match_event(
    routes: tuple[Route, ...],
    event_type: str,
    context: dict[str, Any] | None = None,
) -> list[Route]:
    """匹配事件类型和上下文，返回命中的路由列表（按优先级排序）。

    匹配规则：
    1. event_type 必须完全匹配 route.event
    2. 如果 route 有 pattern 且 context 中有 "path"，用 fnmatch 匹配
    3. 如果 route 有 pattern 且 context 中有 "annotation"，用字符串包含匹配
    4. 如果 route 有 condition，仅做存在性检查（context 中有对应 key 且为真值）
    5. 无 pattern 且无 condition 的路由，只要 event 匹配即命中
    """
    ctx = context or {}
    matched: list[Route] = []

    for route in routes:
        if route.event != event_type:
            continue

        if route.pattern:
            if not _match_pattern(route, ctx):
                continue

        if route.condition:
            if not _match_condition(route.condition, ctx):
                continue

        matched.append(route)

    return matched


def _match_pattern(route: Route, ctx: dict[str, Any]) -> bool:
    """根据事件类型和上下文匹配 pattern。"""
    if route.event == "annotation":
        annotation = ctx.get("annotation", "")
        return route.pattern in annotation

    # file_change / file_create / tool_error: 用 fnmatch 匹配 path
    path = ctx.get("path", "")
    if path:
        return fnmatch(path, route.pattern)

    # tool_error 也可能匹配 tool_name
    tool_name = ctx.get("tool_name", "")
    if tool_name:
        return fnmatch(tool_name, route.pattern)

    return False


def _match_condition(condition: str, ctx: dict[str, Any]) -> bool:
    """简单条件匹配：检查 context 中是否存在满足条件的 key。

    支持 "key >= N" 格式的简单数值比较。
    """
    # 解析 "key >= N" 格式
    for op in (">=", "<=", ">", "<", "=="):
        if op in condition:
            parts = condition.split(op, 1)
            key = parts[0].strip()
            try:
                threshold = int(parts[1].strip())
            except (ValueError, IndexError):
                return False
            value = ctx.get(key)
            if value is None:
                return False
            try:
                num = int(value)
            except (ValueError, TypeError):
                return False
            if op == ">=":
                return num >= threshold
            if op == "<=":
                return num <= threshold
            if op == ">":
                return num > threshold
            if op == "<":
                return num < threshold
            if op == "==":
                return num == threshold
    return False
