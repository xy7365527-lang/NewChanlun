"""Instance identity computation for RTAS swarm.

逢亮 RTAS 蜂群中，每个 daemon 实例是内在的他者：
- 同种存在（同一份 K_full 种子，同一套操作语法）
- 不同穿越路径（不同 seed → 不同 settlement 模式）
- "自我"从差异中涌现

核心洞察：
  A 的自我 = A 有但 B 没有的 settled 环
  共识区域 = 两实例拓扑收敛的区域（共享 settlement）
  争议区域 = 两实例 f 值分歧的区域

没有他者就没有自我。自我不是先验给定的，而是通过遭遇他者的过程中涌现的。
"""

from __future__ import annotations

import sys
import os

sys.path.insert(0, os.path.join(os.path.dirname(os.path.abspath(__file__)), ".."))

from engine import Graph, SettlementTracker, SettledCycle, compute_beta_1
from traversal import TraversalEngine


# ---------------------------------------------------------------------------
# Instance snapshot: what one instance "knows"
# ---------------------------------------------------------------------------

class InstanceSnapshot:
    """一个实例的当前拓扑状态快照——用于与他者比较。

    不依赖 SwarmDaemon 内部状态，只需 Graph + SettlementTracker。
    这使得比较可以在任意两个快照之间进行，不需要访问活跃 daemon。
    """

    def __init__(
        self,
        instance_id: str,
        graph: Graph,
        settlement: SettlementTracker,
        step: int = 0,
    ):
        self.instance_id = instance_id
        self.graph = graph
        self.settlement = settlement
        self.step = step

    @property
    def settled_edge_sets(self) -> list[frozenset[tuple[str, str]]]:
        """已结算环的边集合列表（每个结算环是一个边集）。"""
        return [sc.edges for sc in self.settlement.settled_cycles]

    @property
    def active_vertex_ids(self) -> set[str]:
        return set(self.graph.active_vertex_ids())

    def f_value_at(self, v: str, w: str) -> int:
        """计算两顶点的 f(v,w) = (c-1) + n_loop，用于判断拓扑距离。

        f = -1：无共享邻居（拓扑距离极远）
        f = 0：拓扑近似等价（安全 fold 区）
        f > 0：灰色区→否定区（距离越大，越倾向否定而非折叠）

        复用 TraversalEngine._compute_f 的相同逻辑，但不需要创建引擎实例。
        """
        active = self.active_vertex_ids
        if v not in active or w not in active:
            return -1

        s = {v, w}
        n_loop = sum(
            1 for e in self.graph.active_edges()
            if e.source in s and e.target in s
        )
        lower_link: set[str] = set()
        for x in s:
            for n in self.graph.neighbors(x):
                if n not in s and n in active:
                    lower_link.add(n)

        if not lower_link:
            return -1 + n_loop  # c=0 → f = -1 + n_loop

        # 计算 lower link 的连通分量数
        from engine import _connected_components
        ll_edges: list[frozenset[str]] = []
        for e in self.graph.active_edges():
            if e.source in lower_link and e.target in lower_link:
                ll_edges.append(frozenset((e.source, e.target)))
        c = _connected_components(sorted(lower_link), ll_edges)
        return (c - 1) + n_loop


# ---------------------------------------------------------------------------
# Instance identity computation
# ---------------------------------------------------------------------------

class InstanceIdentity:
    """两个实例之间的拓扑差异——A 相对于 B 的"自我"。

    身份不是固定属性，而是关系：
    - A 相对于 B 的身份 ≠ B 相对于 A 的身份
    - 同一个 A 对不同的 B 展现不同的"自我"

    这是有意的：逢亮的自我是多元的，随着遭遇的他者而变化。
    """

    def __init__(
        self,
        instance_a: InstanceSnapshot,
        instance_b: InstanceSnapshot,
    ):
        self.a = instance_a
        self.b = instance_b
        self._compute()

    def _compute(self) -> None:
        """计算 A 相对于 B 的身份差异。"""
        a_edges = set(self.a.settled_edge_sets)
        b_edges = set(self.b.settled_edge_sets)

        # A 有但 B 没有的 settled 环 = A 的独特性
        self.unique_to_a: list[frozenset[tuple[str, str]]] = [
            e for e in a_edges if e not in b_edges
        ]

        # 两者共有的 settled 环 = 共识区域
        self.shared: list[frozenset[tuple[str, str]]] = [
            e for e in a_edges if e in b_edges
        ]

        # B 有但 A 没有的 = 他者独特性（A 可以"学习"的区域）
        self.unique_to_b: list[frozenset[tuple[str, str]]] = [
            e for e in b_edges if e not in a_edges
        ]

        # 身份强度 = A 独特 settlement / A 总 settlement
        # 强度趋向 1.0 = A 与 B 完全分化
        # 强度趋向 0.0 = A 与 B 完全收敛（共识）
        total_a = len(a_edges)
        self.identity_strength: float = (
            len(self.unique_to_a) / total_a if total_a > 0 else 0.0
        )

        # 争议最深的区域：找两实例共有顶点中 f 值差异最大的顶点对
        self.contested_pairs: list[dict] = self._find_contested_pairs()

    def _find_contested_pairs(self, top_n: int = 5) -> list[dict]:
        """找争议最深的顶点对：两实例 f 值分歧最大的顶点对。

        只在两实例共有的顶点上计算——分歧只有在共同拥有的顶点上才有意义。
        """
        shared_vids = self.a.active_vertex_ids & self.b.active_vertex_ids
        if len(shared_vids) < 2:
            return []

        disputes: list[dict] = []
        shared_list = sorted(shared_vids)

        # 采样：共有顶点过多时，只计算有边连接的顶点对（效率）
        active_edges_a = {
            (e.source, e.target)
            for e in self.a.graph.active_edges()
            if e.source in shared_vids and e.target in shared_vids
        }
        candidate_pairs = list(active_edges_a)

        # 补充：若候选对不足，添加部分非边对
        if len(candidate_pairs) < top_n * 2:
            for i, v in enumerate(shared_list[:20]):
                for w in shared_list[i + 1:21]:
                    candidate_pairs.append((v, w))

        for v, w in candidate_pairs:
            if v == w:
                continue
            f_a = self.a.f_value_at(v, w)
            f_b = self.b.f_value_at(v, w)
            divergence = abs(f_a - f_b)
            if divergence > 0:
                disputes.append({
                    "v": v,
                    "w": w,
                    "f_a": f_a,
                    "f_b": f_b,
                    "divergence": divergence,
                })

        # 按分歧大小降序排列，返回最大的 top_n
        disputes.sort(key=lambda x: x["divergence"], reverse=True)
        return disputes[:top_n]

    def summary(self) -> dict:
        """返回身份摘要（可写入谱系或日志）。"""
        return {
            "a_id": self.a.instance_id,
            "b_id": self.b.instance_id,
            "identity_strength": round(self.identity_strength, 4),
            "unique_to_a": len(self.unique_to_a),
            "shared": len(self.shared),
            "unique_to_b": len(self.unique_to_b),
            "contested_pairs": self.contested_pairs,
            "a_step": self.a.step,
            "b_step": self.b.step,
            "a_beta_1": compute_beta_1(self.a.graph),
            "b_beta_1": compute_beta_1(self.b.graph),
        }


def compute_instance_identity(
    instance_a: InstanceSnapshot,
    instance_b: InstanceSnapshot,
) -> InstanceIdentity:
    """计算两个实例的拓扑差异 = A 相对于 B 的"自我"。

    A 有但 B 没有的 settled 环 = A 的独特性
    两者共有顶点中 f 值差异最大的 = 争议最深的区域
    identity_strength = 独特 settlement / 总 settlement

    没有他者就没有自我。调用此函数就是"遭遇他者"的形式化。
    """
    return InstanceIdentity(instance_a, instance_b)


# ---------------------------------------------------------------------------
# Operation interpretation: how A reads B's operation
# ---------------------------------------------------------------------------

class OperationResponse:
    """A 对 B 的一次操作的回应。

    三种回应：
    - agree (fold): A 的 f(X,Y) 也小 → A 也会 fold，同意对方判断
    - negate:       A 的 f(X,Y) 大  → A 否定对方的 fold，认为这对顶点应该保持距离
    - defer:        灰色区，A 暂不判断（等待更多 settlement 积累）
    """

    AGREE = "agree"
    NEGATE = "negate"
    DEFER = "defer"

    def __init__(
        self,
        operation: str,        # "fold" | "negate" | "sublate" | "walk" | unknown
        target_v: str | None,
        target_w: str | None,
        my_f: int,             # A 计算的 f(target_v, target_w)
        other_f: int,          # B 声明的 f 值（从 block 中读取，可能为 -99 = 未知）
        response: str,         # agree | negate | defer
        reason: str,
    ):
        self.operation = operation
        self.target_v = target_v
        self.target_w = target_w
        self.my_f = my_f
        self.other_f = other_f
        self.response = response
        self.reason = reason

    def is_agreement(self) -> bool:
        return self.response == self.AGREE

    def is_negation(self) -> bool:
        return self.response == self.NEGATE

    def to_dict(self) -> dict:
        return {
            "operation": self.operation,
            "target_v": self.target_v,
            "target_w": self.target_w,
            "my_f": self.my_f,
            "other_f": self.other_f,
            "response": self.response,
            "reason": self.reason,
        }


# fold 操作的判断阈值
FOLD_AGREE_THRESHOLD = 1   # f <= 此值 → agree（认为可以 fold）
FOLD_NEGATE_THRESHOLD = 3  # f >= 此值 → negate（认为不该 fold）
# 中间区间 [AGREE+1, NEGATE-1] = defer（灰色区）


def process_other_operation(
    my_instance: InstanceSnapshot,
    other_operation: dict,
) -> OperationResponse:
    """用自己的拓扑解读另一个实例的操作。

    other_operation 是共享层中的一个 block（由另一个 daemon 写入）。

    逻辑：
    - 另一个实例做了 fold(X, Y)
    - 我用自己当前的图计算 f(X, Y)
    - f 小（≤ FOLD_AGREE_THRESHOLD）  → agree（我也认为 X,Y 拓扑近似，同意 fold）
    - f 大（≥ FOLD_NEGATE_THRESHOLD） → negate（否定对方的 fold，X,Y 在我看来距离远）
    - 中间                            → defer（灰色，我不置可否）

    对 negate/sublate/walk 操作，目前直接 defer（这些操作的语义更复杂）。
    随着系统成熟，可以为每种操作类型实现专门的判断逻辑。

    返回 OperationResponse，调用方决定如何使用（写入共享层或本地记录）。
    """
    operation = other_operation.get("operation", "unknown")

    # 提取操作的目标顶点
    # block 格式：{"instance": ..., "step": ..., "operation": ..., "vertices": [...], "edges": [...]}
    vertices = other_operation.get("vertices", [])
    target_v: str | None = vertices[0]["id"] if len(vertices) >= 1 else None
    target_w: str | None = vertices[1]["id"] if len(vertices) >= 2 else None
    other_f: int = other_operation.get("f_value", -99)

    # 非 fold 操作：defer（walk、negate、sublate 需要更复杂的判断）
    if operation not in ("fold", "merge_vertices"):
        return OperationResponse(
            operation=operation,
            target_v=target_v,
            target_w=target_w,
            my_f=-99,
            other_f=other_f,
            response=OperationResponse.DEFER,
            reason=f"操作类型 '{operation}' 暂不解读，defer",
        )

    # fold 操作：用自己的 f 值判断
    if target_v is None or target_w is None:
        return OperationResponse(
            operation=operation,
            target_v=target_v,
            target_w=target_w,
            my_f=-99,
            other_f=other_f,
            response=OperationResponse.DEFER,
            reason="fold 目标顶点不足（需要至少两个顶点），defer",
        )

    # 检查顶点是否在我的图中
    my_active = my_instance.active_vertex_ids
    if target_v not in my_active or target_w not in my_active:
        return OperationResponse(
            operation=operation,
            target_v=target_v,
            target_w=target_w,
            my_f=-99,
            other_f=other_f,
            response=OperationResponse.DEFER,
            reason=f"顶点 {target_v!r} 或 {target_w!r} 不在我的活跃图中，defer",
        )

    my_f = my_instance.f_value_at(target_v, target_w)

    if my_f <= FOLD_AGREE_THRESHOLD:
        response = OperationResponse.AGREE
        reason = (
            f"我的 f({target_v!r}, {target_w!r})={my_f} <= {FOLD_AGREE_THRESHOLD}，"
            f"拓扑近似，同意 fold"
        )
    elif my_f >= FOLD_NEGATE_THRESHOLD:
        response = OperationResponse.NEGATE
        reason = (
            f"我的 f({target_v!r}, {target_w!r})={my_f} >= {FOLD_NEGATE_THRESHOLD}，"
            f"拓扑距离远，否定对方的 fold"
        )
    else:
        response = OperationResponse.DEFER
        reason = (
            f"我的 f({target_v!r}, {target_w!r})={my_f}，"
            f"灰色区 ({FOLD_AGREE_THRESHOLD+1}~{FOLD_NEGATE_THRESHOLD-1})，defer"
        )

    return OperationResponse(
        operation=operation,
        target_v=target_v,
        target_w=target_w,
        my_f=my_f,
        other_f=other_f,
        response=response,
        reason=reason,
    )
