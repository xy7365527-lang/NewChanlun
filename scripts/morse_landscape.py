"""morse_landscape.py — Morse 地形（穿越基础设施组件三）

时序 Kruskal 算法构建生成森林，标记每条 reference 边为 tree 或 critical。
critical 边 = 参与不可缩环路的边 = 离散 Morse 函数的临界 1-单纯形。

无状态、全量重算、可缓存。
"""

from __future__ import annotations

import json
import os
from dataclasses import dataclass
from pathlib import Path
from typing import Any, Literal


DEFAULT_RELATIONS_PATH = Path(".chanlun/block-topology/relations.jsonl")
DEFAULT_CACHE_PATH = Path(".chanlun/morse_landscape.json")


@dataclass(frozen=True)
class MorseLandscape:
    """Morse 地形：每条 reference 边的 tree/critical 标记 + 统计。"""
    edge_marks: dict[str, Literal["tree", "critical"]]  # key = f"{from_id}:{to_id}"
    stats: dict[str, Any]  # {nodes, edges, critical, components, tree}

    def to_dict(self) -> dict:
        return {
            "edge_marks": self.edge_marks,
            "stats": self.stats,
        }

    @classmethod
    def from_dict(cls, data: dict) -> "MorseLandscape":
        return cls(
            edge_marks=data["edge_marks"],
            stats=data["stats"],
        )


class UnionFind:
    """Union-Find with path compression and union by rank."""

    def __init__(self) -> None:
        self._parent: dict[str, str] = {}
        self._rank: dict[str, int] = {}

    def find(self, x: str) -> str:
        if x not in self._parent:
            self._parent[x] = x
            self._rank[x] = 0
        root = x
        while self._parent[root] != root:
            root = self._parent[root]
        # path compression
        while self._parent[x] != root:
            next_x = self._parent[x]
            self._parent[x] = root
            x = next_x
        return root

    def union(self, x: str, y: str) -> bool:
        """合并 x 和 y 的集合。如果已在同一集合返回 False，否则返回 True。"""
        rx, ry = self.find(x), self.find(y)
        if rx == ry:
            return False
        if self._rank[rx] < self._rank[ry]:
            rx, ry = ry, rx
        self._parent[ry] = rx
        if self._rank[rx] == self._rank[ry]:
            self._rank[rx] += 1
        return True

    def component_count(self, nodes: set[str]) -> int:
        """统计给定节点集的连通分量数。"""
        roots = set()
        for n in nodes:
            roots.add(self.find(n))
        return len(roots)


def build_morse_landscape(
    relations_path: Path = DEFAULT_RELATIONS_PATH,
    cache_path: Path | None = DEFAULT_CACHE_PATH,
) -> MorseLandscape:
    """时序 Kruskal 构建 Morse 地形。

    - 输入：relations.jsonl 中 relation="references" 的边
    - 边排序：timestamp ASC → (from_block_id, to_block_id) 字典序 tie-breaking
    - 无向化：有向边直接视为无向边（不合并双向引用）
    - Union-Find：tree 边连接两个分量，critical 边两端已在同一分量
    - 缓存：cache_path 存在且 mtime >= relations.jsonl 则反序列化
    """
    # 缓存检查
    if cache_path is not None and cache_path.exists() and relations_path.exists():
        cache_mtime = os.path.getmtime(cache_path)
        relations_mtime = os.path.getmtime(relations_path)
        if cache_mtime >= relations_mtime:
            data = json.loads(cache_path.read_text(encoding="utf-8"))
            return MorseLandscape.from_dict(data)

    # 读取 references 边
    edges: list[dict] = []
    if relations_path.exists():
        with open(relations_path, encoding="utf-8") as f:
            for line in f:
                line = line.strip()
                if not line:
                    continue
                rel = json.loads(line)
                if rel.get("relation") == "references":
                    edges.append(rel)

    # 排序：timestamp ASC → from ASC → to ASC
    edges.sort(key=lambda e: (e.get("timestamp", ""), e["from"], e["to"]))

    # 收集所有节点
    nodes: set[str] = set()
    for e in edges:
        nodes.add(e["from"])
        nodes.add(e["to"])

    # 时序 Kruskal
    uf = UnionFind()
    # 先注册所有节点
    for n in nodes:
        uf.find(n)

    edge_marks: dict[str, Literal["tree", "critical"]] = {}
    tree_count = 0

    for e in edges:
        from_id = e["from"]
        to_id = e["to"]
        edge_key = f"{from_id}:{to_id}"

        # 无向化：Union-Find 中 union(from, to) 不区分方向
        if uf.union(from_id, to_id):
            edge_marks[edge_key] = "tree"
            tree_count += 1
        else:
            edge_marks[edge_key] = "critical"

    n_nodes = len(nodes)
    n_edges = len(edges)
    n_components = uf.component_count(nodes) if nodes else 0
    n_critical = n_edges - tree_count

    stats = {
        "nodes": n_nodes,
        "edges": n_edges,
        "critical": n_critical,
        "components": n_components,
        "tree": tree_count,
    }

    landscape = MorseLandscape(edge_marks=edge_marks, stats=stats)

    # 写缓存
    if cache_path is not None:
        cache_path.parent.mkdir(parents=True, exist_ok=True)
        cache_path.write_text(
            json.dumps(landscape.to_dict(), ensure_ascii=False),
            encoding="utf-8",
        )

    return landscape
