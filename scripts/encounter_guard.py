"""encounter_guard.py — 偶遇记录 + DAG 守护（穿越基础设施组件二）

偶遇检测：ceremony 写入新 reference 边后，全量重算 Morse 地形，
检测新边是否为 critical。critical = 新偶遇 = 拓扑事件。

DAG 守护：depends_on 子图的无环检测。硬拒绝（raise），不是警告。

无状态、无锁、可重现。
"""

from __future__ import annotations

import json
import sys
from datetime import datetime, timezone
from pathlib import Path

sys.path.insert(0, str(Path(__file__).parent))
from morse_landscape import build_morse_landscape, DEFAULT_RELATIONS_PATH


TOPO_EVENTS_PATH = Path(".chanlun/topo_events.jsonl")


class CircularDependencyError(Exception):
    """depends_on 子图中检测到环。"""

    def __init__(self, from_id: str, to_id: str, cycle_path: list[str]):
        self.from_id = from_id
        self.to_id = to_id
        self.cycle_path = cycle_path
        path_str = " → ".join(cycle_path)
        super().__init__(
            f"Circular dependency detected: adding {from_id} → {to_id} "
            f"would create cycle: {path_str}"
        )


def check_encounter(
    from_id: str,
    to_id: str,
    timestamp: str | None = None,
    relations_path: Path = DEFAULT_RELATIONS_PATH,
    topo_events_path: Path = TOPO_EVENTS_PATH,
) -> bool:
    """检查新 reference 边是否为 critical（偶遇事件）。

    全量重算 Morse 地形，检查指定边的标记。
    如果是 critical -> 写入 topo_event -> 返回 True
    如果是 tree -> 返回 False

    注意：调用此函数前，新边应已写入 relations.jsonl。
    """
    landscape = build_morse_landscape(
        relations_path=relations_path,
        cache_path=None,  # 不用缓存，确保包含新边
    )
    edge_key = f"{from_id}:{to_id}"
    mark = landscape.edge_marks.get(edge_key)

    if mark == "critical":
        _record_topo_event(
            from_id,
            to_id,
            timestamp or datetime.now(timezone.utc).isoformat(),
            "new_critical",
            path=topo_events_path,
        )
        return True
    return False


def _record_topo_event(
    from_id: str,
    to_id: str,
    timestamp: str,
    event_type: str,
    path: Path = TOPO_EVENTS_PATH,
) -> None:
    """追加拓扑事件到 topo_events.jsonl。"""
    event = {
        "from": from_id,
        "to": to_id,
        "timestamp": timestamp,
        "event_type": event_type,
        "detected_at": datetime.now(timezone.utc).isoformat(),
    }
    path.parent.mkdir(parents=True, exist_ok=True)
    with open(path, "a", encoding="utf-8") as f:
        f.write(json.dumps(event, ensure_ascii=False, separators=(",", ":")) + "\n")


def check_dag_integrity(
    from_id: str,
    to_id: str,
    relations_path: Path = DEFAULT_RELATIONS_PATH,
) -> None:
    """检查新增 depends_on 边是否会创建环。

    如果会创建环 -> raise CircularDependencyError
    如果不会 -> 静默返回

    算法：在 depends_on 子图上，从 to_id 出发 DFS 搜索 from_id。
    如果能到达 -> 添加 from_id -> to_id 会形成环。
    """
    # 自环检测
    if from_id == to_id:
        raise CircularDependencyError(from_id, to_id, [from_id, to_id])

    # 构建 depends_on 邻接表
    adj: dict[str, list[str]] = {}
    if relations_path.exists():
        with open(relations_path, encoding="utf-8") as f:
            for line in f:
                line = line.strip()
                if not line:
                    continue
                rel = json.loads(line)
                if rel.get("relation") == "depends_on":
                    src = rel["from"]
                    dst = rel["to"]
                    adj.setdefault(src, []).append(dst)

    # DFS 从 to_id 出发，搜索是否能到达 from_id
    visited: set[str] = set()
    path_stack: list[str] = []

    def _dfs(node: str) -> bool:
        """返回 True 如果从 node 能到达 from_id。"""
        if node == from_id:
            path_stack.append(node)
            return True
        if node in visited:
            return False
        visited.add(node)
        path_stack.append(node)
        for neighbor in adj.get(node, []):
            if _dfs(neighbor):
                return True
        path_stack.pop()
        return False

    if _dfs(to_id):
        # 构建环路径：from_id -> to_id -> ... -> from_id
        cycle_path = [from_id] + path_stack
        raise CircularDependencyError(from_id, to_id, cycle_path)
