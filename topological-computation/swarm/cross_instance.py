"""Cross-instance operation injection with interpretation.

逻辑分两层：
1. 注入（原有）：把外来块的顶点/边加入本地图（盲目合并）
2. 解读（新增）：用自己的 f 值判断外来 fold 是否合理，并将回应写入共享层

解读层不阻断注入——外来块始终被注入，但会附带一份"我的立场"写入共享 relations。
这使得蜂群的分歧可见：共享层不只是数据同步媒介，也是多个实例之间立场交换的场所。
"""

from __future__ import annotations

import sys
import os

sys.path.insert(0, os.path.join(os.path.dirname(os.path.abspath(__file__)), ".."))

from engine import Graph, Vertex, Edge, EdgeType, VertexStatus
from swarm.shared_layer import SharedLayer


class CrossInstanceSync:
    """Sync operations from other instances via shared block store.

    每次 sync() 包含两个阶段：
    1. 注入：把外来块的顶点/边合并到本地图
    2. 解读：用本地 f 值判断外来 fold，将回应写入共享 relations

    解读结果类型：
    - agree：本地认为外来 fold 合理（f 小，拓扑近似）
    - negate：本地否定外来 fold（f 大，拓扑距离远）
    - defer：本地暂不判断（灰色区或操作类型不支持）
    """

    def __init__(
        self,
        shared_layer: SharedLayer,
        instance_id: str,
        daemon,  # SwarmDaemon — forward reference to avoid circular import
    ):
        self.shared = shared_layer
        self.instance_id = instance_id
        self.daemon = daemon
        # Pre-fill known_blocks with all existing CIDs to skip historical blocks.
        # Without this, first sync() downloads ALL historical blocks (3000+),
        # which takes minutes and hangs the traversal worker.
        self.known_blocks: set[str] = shared_layer.all_block_hashes()
        self.injected_count: int = 0

        # 解读统计
        self.agreed_count: int = 0    # 同意对方 fold 的次数
        self.negated_count: int = 0   # 否定对方 fold 的次数
        self.deferred_count: int = 0  # defer（灰色区）的次数

        # Peer state tracking (from new block types)
        self.peer_snet_states: dict[str, dict] = {}  # instance_id -> last snet_update
        self.peer_settlements: dict[str, list[dict]] = {}  # instance_id -> settlement events

    # Max blocks to download per sync call — prevents unbounded IPFS I/O
    _MAX_BLOCKS_PER_SYNC = 100

    def sync(self) -> int:
        """Scan for new blocks, inject those from other instances.

        对每个外来块按 type 分发：
        - traversal_position → 更新 peer_positions（不注入图）
        - graph_delta / feed_event → 注入顶点/边 + 解读（graph_delta only）
        - settlement_event → 记录 peer settlement（不注入图）
        - snet_update → 记录 peer S_net 激活态（不注入图）
        - interpretation → 跳过（是对方的解读回应，不注入）

        Returns the number of blocks injected.
        """
        all_cids = self.shared.all_block_hashes()
        new_cids = all_cids - self.known_blocks

        # If too many new blocks, mark excess as known (skip) to avoid
        # downloading thousands of historical blocks on first sync.
        if len(new_cids) > self._MAX_BLOCKS_PER_SYNC:
            # Process only the most recent MAX blocks; mark the rest as known
            cid_list = list(new_cids)
            skip = cid_list[self._MAX_BLOCKS_PER_SYNC:]
            self.known_blocks.update(skip)
            new_cids = set(cid_list[:self._MAX_BLOCKS_PER_SYNC])

        new_blocks: list[dict] = []
        for cid in new_cids:
            self.known_blocks.add(cid)
            content = self.shared.read_block(cid)
            if content is not None:
                content["hash"] = cid
                new_blocks.append(content)

        injected = 0
        for block in new_blocks:
            if block.get("instance") == self.instance_id:
                continue

            block_type = block.get("type", "")

            if block_type == "traversal_position":
                self._update_peer_position(block)

            elif block_type == "settlement_event":
                self._update_peer_settlement(block)

            elif block_type == "snet_update":
                self._update_peer_snet(block)

            elif block_type == "interpretation":
                pass  # 对方的解读回应，不需要注入

            elif block_type in ("graph_delta", "feed_event"):
                self._inject_external_operation(block)
                injected += 1
                if block_type == "graph_delta":
                    self._interpret_and_respond(block, block.get("hash", ""))

            else:
                self._inject_external_operation(block)
                injected += 1
                self._interpret_and_respond(block, block.get("hash", ""))

        self.injected_count += injected
        return injected

    def _update_peer_position(self, block: dict) -> None:
        """Update peer_positions from a traversal_position block.

        穿越位置是区块事件（方案C决策），通过 SharedLayer 传递而非前端直连。
        """
        instance_id = block.get("instance")
        if not instance_id or instance_id == self.instance_id:
            return
        self.daemon.peer_positions[instance_id] = {
            "position_label": block.get("position_label", ""),
            "step": block.get("step", 0),
            "timestamp": block.get("timestamp", 0),
        }

    def _update_peer_settlement(self, block: dict) -> None:
        """Record peer settlement event (informational, no graph injection).

        Settlement events from other instances are tracked for visibility
        but not injected into the local graph — each instance settles
        its own cycles based on its own traversal.
        """
        instance_id = block.get("instance")
        if not instance_id or instance_id == self.instance_id:
            return
        self.peer_settlements.setdefault(instance_id, []).append({
            "settled_at_step": block.get("settled_at_step", 0),
            "cycle_edges": block.get("cycle_edges", []),
            "step": block.get("step", 0),
            "timestamp": block.get("timestamp", 0),
        })

    def _update_peer_snet(self, block: dict) -> None:
        """Record peer S_net activation state (informational).

        Peer S_net states are tracked for multi-instance visualization
        and potential cross-instance resonance detection.
        """
        instance_id = block.get("instance")
        if not instance_id or instance_id == self.instance_id:
            return
        self.peer_snet_states[instance_id] = {
            "currently_active": block.get("currently_active", []),
            "dialogue_focus_set": block.get("dialogue_focus_set", []),
            "edge_suggestions_count": len(block.get("edge_suggestions", [])),
            "step": block.get("step", 0),
            "timestamp": block.get("timestamp", 0),
        }

    def _merge_block_into_graph(self, graph: Graph, block: dict) -> Graph:
        """Merge block vertices/edges into one graph view."""
        # Extract vertices from block
        for vd in block.get("vertices", []):
            vid = vd["id"]
            if graph.vertex(vid) is None:
                v = Vertex(
                    id=vid,
                    status=VertexStatus(vd.get("status", "active")),
                    content=vd.get("content"),
                    created_at=vd.get("created_at", 0),
                )
                graph = graph.add_vertex(v)

        # Extract edges from block
        existing_edges = {
            (e.source, e.target, e.edge_type.value) for e in graph.edges
        }
        for ed in block.get("edges", []):
            key = (ed["source"], ed["target"], ed["edge_type"])
            if key not in existing_edges:
                src_v = graph.vertex(ed["source"])
                tgt_v = graph.vertex(ed["target"])
                if src_v is not None and tgt_v is not None:
                    e = Edge(
                        source=ed["source"],
                        target=ed["target"],
                        edge_type=EdgeType(ed["edge_type"]),
                        created_at=ed.get("created_at", 0),
                    )
                    graph = graph.add_edge(e)
                    existing_edges.add(key)

        return graph

    def _inject_external_operation(self, block: dict) -> None:
        """Inject an external block's vertices and edges into the local daemon."""
        active_graph = self._merge_block_into_graph(self.daemon.k_active, block)
        full_graph = self._merge_block_into_graph(self.daemon.k_full, block)

        # Update daemon state
        self.daemon.k_active = active_graph
        self.daemon.k_full = full_graph
        # Hot-update engine graph reference instead of full re-initialization
        if self.daemon.engine is not None:
            self.daemon.engine.k_active = active_graph
            self.daemon.engine.k_full = full_graph
        else:
            self.daemon._initialize_engine()

    def _interpret_and_respond(self, block: dict, block_hash: str) -> None:
        """用自己的拓扑解读外来操作，将回应写入共享层 relations。

        解读在注入之后执行——此时外来顶点已在本地图中，
        因此 f 值计算基于融合后的图（而非注入前的图）。
        这是有意的：我用自己对这个世界的整体理解来判断对方的操作。
        """
        from swarm.identity import InstanceSnapshot, process_other_operation

        # 构建本地快照（注入后的图 + 本地 settlement）
        my_snapshot = InstanceSnapshot(
            instance_id=self.instance_id,
            graph=self.daemon.k_active,
            settlement=self.daemon.settlement,
            step=self.daemon.total_steps,
        )

        # 解读外来操作
        response = process_other_operation(my_snapshot, block)

        # 更新统计
        if response.response == "agree":
            self.agreed_count += 1
        elif response.response == "negate":
            self.negated_count += 1
        else:
            self.deferred_count += 1

        # 将回应写入共享层 relations（仅 agree 和 negate 有记录价值）
        if response.response in ("agree", "negate"):
            # 先把回应写成一个 block，再用 relation 连接
            response_block = {
                "instance": self.instance_id,
                "step": self.daemon.total_steps,
                "type": "interpretation",
                "target_block": block_hash,
                "target_instance": block.get("instance"),
                "operation": response.operation,
                "response": response.response,
                "my_f": response.my_f,
                "other_f": response.other_f,
                "target_v": response.target_v,
                "target_w": response.target_w,
                "reason": response.reason,
            }
            response_hash = self.shared.write_block(response_block)
            self.known_blocks.add(response_hash)

            # relation: block_hash --[agree|negate]--> response_hash
            self.shared.write_relation(
                from_hash=block_hash,
                to_hash=response_hash,
                relation=response.response,
                instance_id=self.instance_id,
            )

    def interpretation_summary(self) -> dict:
        """返回解读统计摘要（含 peer state tracking）。"""
        total = self.agreed_count + self.negated_count + self.deferred_count
        return {
            "instance_id": self.instance_id,
            "injected_total": self.injected_count,
            "interpreted_total": total,
            "agreed": self.agreed_count,
            "negated": self.negated_count,
            "deferred": self.deferred_count,
            "agreement_rate": (
                round(self.agreed_count / total, 4) if total > 0 else 0.0
            ),
            "negation_rate": (
                round(self.negated_count / total, 4) if total > 0 else 0.0
            ),
            "peer_snet_tracked": len(self.peer_snet_states),
            "peer_settlements_tracked": sum(
                len(v) for v in self.peer_settlements.values()
            ),
        }
