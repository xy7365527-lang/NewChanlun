"""snet_lazy.py -- SNetLazy: SQLite-backed S_net（按需加载，不全量加载到内存）。

解决 OOM 问题：788K 边全量加载占 ~8GB 内存。
SNetLazy 将边存储在 SQLite 中，通过 LRU 缓存按需查询。

设计：
  - SNetLazy 是 SNet 的子类，覆盖所有查询方法
  - signifiers 和 morphemes 保留内存（数量级 ~数万，内存可忽略）
  - edges 不加载到内存，通过 SQLite 查询
  - LRU 缓存热顶点的邻居列表（缓存大小可配置）
  - 写入（add_edge 等）直接写 SQLite + 失效缓存
  - edges 属性返回空列表警告，提供 iter_edges() 流式读取
  - edge_count 属性返回 COUNT(*)

内存预算：
  - signifiers: ~数万条 → <50MB
  - LRU 缓存: 默认 10000 个顶点的邻居 → ~50-100MB
  - 总计: <200MB（vs 原来 ~8GB）

认识论等级: L0（存储引擎替换，查询语义不变）

谱系引用: v239-swarm/snet-lazy-load
"""

from __future__ import annotations

import sys
import warnings
from collections import OrderedDict
from typing import Optional

from signifier_net import (
    SNet,
    Signifier,
    SignifierEdge,
    AxisType,
    MorphemeStructure,
)


class _LRUCache:
    """简单的 LRU 缓存（不依赖 functools.lru_cache，支持失效）。"""

    def __init__(self, maxsize: int = 10000) -> None:
        self._maxsize = maxsize
        self._cache: OrderedDict[str, object] = OrderedDict()

    def get(self, key: str) -> object | None:
        if key in self._cache:
            self._cache.move_to_end(key)
            return self._cache[key]
        return None

    def put(self, key: str, value: object) -> None:
        if key in self._cache:
            self._cache.move_to_end(key)
        else:
            if len(self._cache) >= self._maxsize:
                self._cache.popitem(last=False)
        self._cache[key] = value

    def invalidate(self, key: str) -> None:
        self._cache.pop(key, None)

    def clear(self) -> None:
        self._cache.clear()

    def __len__(self) -> int:
        return len(self._cache)


class SNetLazy(SNet):
    """SQLite-backed S_net，按需加载边。

    替代全量内存 SNet，将运行时内存从 ~8GB 降到 <200MB。
    查询接口与 SNet 完全一致——穿越算法无需改动。

    构造方式：
      SNetLazy.from_persistence(persistence, signifiers, morphemes)

    不支持不可变操作（add_edge 返回新 SNet 等），因为 lazy 模式下
    不可变语义没有意义（边在 SQLite 中）。写入操作直接修改 SQLite。
    """

    def __init__(
        self,
        signifiers: dict[str, Signifier] | None = None,
        morphemes: dict[str, MorphemeStructure] | None = None,
        persistence=None,
        cache_size: int = 10000,
    ) -> None:
        # 绕过 SNet.__init__ 的边索引构建（不传 edges）
        # 直接设置内部状态
        self._signifiers: dict[str, Signifier] = dict(signifiers) if signifiers else {}
        self._edges: list[SignifierEdge] = []  # 空，不用
        self._morphemes: dict[str, MorphemeStructure] = dict(morphemes) if morphemes else {}
        self._hyperedges: list = []  # 超边（SNetLazy 暂不持久化超边到 SQLite）
        self._vertex_to_hyperedges: dict[str, list[int]] = {}
        self._syn_out: dict[str, list[SignifierEdge]] = {}
        self._par_out: dict[str, list[SignifierEdge]] = {}
        self._morpheme_out: dict[str, list[SignifierEdge]] = {}
        self._syn_degree: dict[str, int] = {}

        # SQLite 后端
        self._persistence = persistence
        # LRU 缓存: key = f"{axis}:{source}" -> list[SignifierEdge]
        self._edge_cache = _LRUCache(maxsize=cache_size)
        # degree 缓存: key = sid -> int
        self._degree_cache = _LRUCache(maxsize=cache_size)
        # cooccurrence 缓存: key = f"{a}:{b}" -> float
        self._cooccurrence_cache = _LRUCache(maxsize=cache_size * 2)
        # 边总数缓存
        self._edge_count_cache: int | None = None
        # 运行时新增边缓冲区（用于 block topology 增量写入）
        self._runtime_new_edges: list[SignifierEdge] = []

    @classmethod
    def from_persistence(
        cls,
        persistence,
        signifiers: dict[str, Signifier] | None = None,
        morphemes: dict[str, MorphemeStructure] | None = None,
        cache_size: int = 10000,
    ) -> "SNetLazy":
        """从 SNetPersistence 实例创建 SNetLazy。

        如果 signifiers/morphemes 未提供，从 SQLite 加载（一次性，内存可忽略）。
        """
        if signifiers is None:
            signifiers = persistence.load_signifiers()
        if morphemes is None:
            morphemes = persistence.load_morphemes()
        return cls(
            signifiers=signifiers,
            morphemes=morphemes,
            persistence=persistence,
            cache_size=cache_size,
        )

    # ------------------------------------------------------------------
    # 只读查询（覆盖 SNet 的内存查询，改为 SQLite + LRU）
    # ------------------------------------------------------------------

    def syntagmatic_neighbors(self, sid: str) -> list[SignifierEdge]:
        """返回能指 sid 的所有组合轴邻居（按 weight 降序）。"""
        cache_key = f"syntagmatic:{sid}"
        cached = self._edge_cache.get(cache_key)
        if cached is not None:
            return list(cached)

        if self._persistence is None:
            return []

        edges = self._persistence.query_edges_by_source_and_axis(
            sid, "syntagmatic",
        )
        edges.sort(key=lambda e: -e.weight)
        self._edge_cache.put(cache_key, edges)
        return list(edges)

    def paradigmatic_alternatives(self, sid: str) -> list[SignifierEdge]:
        """返回能指 sid 的所有聚合轴替换项（按 weight 降序）。"""
        cache_key = f"paradigmatic:{sid}"
        cached = self._edge_cache.get(cache_key)
        if cached is not None:
            return list(cached)

        if self._persistence is None:
            return []

        edges = self._persistence.query_edges_by_source_and_axis(
            sid, "paradigmatic",
        )
        edges.sort(key=lambda e: -e.weight)
        self._edge_cache.put(cache_key, edges)
        return list(edges)

    def morpheme_links(self, sid: str) -> list[SignifierEdge]:
        """返回能指 sid 的所有语素轴连接（按 weight 降序）。"""
        cache_key = f"morpheme:{sid}"
        cached = self._edge_cache.get(cache_key)
        if cached is not None:
            return list(cached)

        if self._persistence is None:
            return []

        edges = self._persistence.query_edges_by_source_and_axis(
            sid, "morpheme",
        )
        edges.sort(key=lambda e: -e.weight)
        self._edge_cache.put(cache_key, edges)
        return list(edges)

    def cooccurrence_weight(self, sid_a: str, sid_b: str) -> float:
        """返回两个能指之间的组合轴共现权重（无边则 0.0）。"""
        cache_key = f"{sid_a}:{sid_b}"
        cached = self._cooccurrence_cache.get(cache_key)
        if cached is not None:
            return cached

        if self._persistence is None:
            return 0.0

        weight = self._persistence.query_cooccurrence_weight(sid_a, sid_b)
        self._cooccurrence_cache.put(cache_key, weight)
        return weight

    def syn_degree(self, sid: str) -> int:
        """返回能指 sid 的组合轴度数（无向）。"""
        cached = self._degree_cache.get(sid)
        if cached is not None:
            return cached

        if self._persistence is None:
            return 0

        degree = self._persistence.query_syn_degree(sid)
        self._degree_cache.put(sid, degree)
        return degree

    def degree_normalized_neighbors(
        self,
        sid: str,
        n: int = 5,
        alpha: float = 0.5,
    ) -> list[SignifierEdge]:
        """返回 sid 的 degree-normalized 组合轴邻居（按归一化分数降序）。"""
        edges = self.syntagmatic_neighbors(sid)
        if not edges:
            return []

        def score(e: SignifierEdge) -> float:
            tgt_degree = self.syn_degree(e.target)
            if tgt_degree <= 0:
                tgt_degree = 1
            return e.weight / (tgt_degree ** alpha)

        return sorted(edges, key=lambda e: -score(e))[:n]

    # ------------------------------------------------------------------
    # edges 属性（冷路径）
    # ------------------------------------------------------------------

    @property
    def edges(self) -> list[SignifierEdge]:
        """返回所有边。

        警告：这会从 SQLite 全量读取所有边到内存。
        仅在 bootstrap/报告等冷路径中使用。
        穿越热路径应使用 syntagmatic_neighbors() 等按需查询。
        """
        if self._persistence is None:
            return []
        print(
            "SNetLazy.edges: loading all edges from SQLite (cold path)",
            file=sys.stderr,
        )
        return list(self._persistence.iter_all_edges())

    @property
    def edge_count(self) -> int:
        """返回边总数（不加载到内存）。"""
        if self._edge_count_cache is not None:
            return self._edge_count_cache
        if self._persistence is None:
            return 0
        count = self._persistence.edge_count()
        self._edge_count_cache = count
        return count

    # ------------------------------------------------------------------
    # 写入操作（可变模式——直接写 SQLite）
    # ------------------------------------------------------------------

    def add_signifier(self, sig: Signifier) -> "SNetLazy":
        """添加能指（可变操作——直接修改内部状态 + SQLite）。"""
        self._signifiers[sig.id] = sig
        if self._persistence is not None:
            self._persistence.save_incremental(new_signifiers=[sig])
        return self

    def add_edge(self, edge: SignifierEdge) -> "SNetLazy":
        """添加边（可变操作——直接写 SQLite + 失效缓存 + 追踪新增）。"""
        if self._persistence is not None:
            self._persistence.save_incremental(new_edges=[edge])
        self._runtime_new_edges.append(edge)
        self._invalidate_edge_caches(edge)
        return self

    def add_edges(self, edges: list[SignifierEdge]) -> "SNetLazy":
        """批量添加边。"""
        if not edges:
            return self
        if self._persistence is not None:
            self._persistence.save_incremental(new_edges=edges)
        self._runtime_new_edges.extend(edges)
        for edge in edges:
            self._invalidate_edge_caches(edge)
        return self

    def add_signifiers(self, sigs: list[Signifier]) -> "SNetLazy":
        """批量添加能指。"""
        if not sigs:
            return self
        for sig in sigs:
            self._signifiers[sig.id] = sig
        if self._persistence is not None:
            self._persistence.save_incremental(new_signifiers=sigs)
        return self

    def add_morpheme_structure(self, ms: MorphemeStructure) -> "SNetLazy":
        """添加语素分解结构。"""
        self._morphemes[ms.signifier_id] = ms
        if self._persistence is not None:
            self._persistence.save_incremental(new_morphemes=[ms])
        return self

    def merge_edge_weights(self) -> "SNetLazy":
        """SNetLazy 不支持 merge_edge_weights（SQLite 中边已通过 UNIQUE INDEX 去重）。

        返回 self（no-op）。
        如果需要合并，应该在全量 SNet 中完成后再保存到 SQLite。
        """
        return self

    def drain_runtime_new_edges(self) -> list[SignifierEdge]:
        """取出并清空运行时新增边缓冲区。

        用于 block topology 增量写入：daemon 每步调用此方法获取
        本步新增的边，写入 block topology 后缓冲区清空。
        """
        edges = self._runtime_new_edges
        self._runtime_new_edges = []
        return edges

    def _invalidate_edge_caches(self, edge: SignifierEdge) -> None:
        """失效与边相关的缓存条目。"""
        axis_name = edge.axis.value
        self._edge_cache.invalidate(f"{axis_name}:{edge.source}")
        self._edge_cache.invalidate(f"{axis_name}:{edge.target}")
        if edge.axis == AxisType.SYNTAGMATIC:
            self._degree_cache.invalidate(edge.source)
            self._degree_cache.invalidate(edge.target)
            self._cooccurrence_cache.invalidate(f"{edge.source}:{edge.target}")
            self._cooccurrence_cache.invalidate(f"{edge.target}:{edge.source}")
        self._edge_count_cache = None

    # ------------------------------------------------------------------
    # 序列化
    # ------------------------------------------------------------------

    def to_dict(self) -> dict:
        """序列化为 dict（会触发全量边加载，仅用于迁移）。"""
        return {
            "signifiers": {
                sid: {
                    "id": sig.id,
                    "surface_forms": list(sig.surface_forms),
                    "source": sig.source,
                    "lang": sig.lang,
                    "domain": sig.domain,
                }
                for sid, sig in self._signifiers.items()
            },
            "edges": [
                {
                    "source": e.source,
                    "target": e.target,
                    "axis": e.axis.value,
                    "weight": e.weight,
                    "evidence": e.evidence,
                    "relation": e.relation,
                    "differential": e.differential,
                }
                for e in (self._persistence.iter_all_edges() if self._persistence else [])
            ],
            "morphemes": {
                sid: {
                    "signifier_id": ms.signifier_id,
                    "morphemes": [
                        {
                            "form": m.form,
                            "meaning": m.meaning,
                            "lang": m.lang,
                            "shared_with": list(m.shared_with),
                        }
                        for m in ms.morphemes
                    ],
                    "etymology": ms.etymology,
                }
                for sid, ms in self._morphemes.items()
            },
        }

    def __repr__(self) -> str:
        n_edges = self.edge_count
        return (
            f"SNetLazy(signifiers={len(self._signifiers)}, "
            f"edges={n_edges}, "
            f"morphemes={len(self._morphemes)})"
        )
