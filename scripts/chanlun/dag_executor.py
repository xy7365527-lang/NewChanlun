"""DAG 拓扑排序执行引擎原型。

⚠ 已过时：本脚本适配 dispatch-dag.yaml v1.x 的 nodes.structural / edges.structural_edges 格式。
当前 dispatch-dag.yaml 已升级为 v3.1（event_skill_map + system_nodes + platform_layer + skill_flow_edges），
本脚本的 extract_graph() 无法解析新格式，运行会产生空图。
如需验证当前 DAG，请使用 ceremony_sequence 中的 cold_start/warm_start 子 DAG（本脚本的 ceremony_topo_sort 仍可工作）。
"""

from __future__ import annotations

import sys
from collections import defaultdict, deque
from pathlib import Path

import yaml


def load_dag(path: Path) -> dict:
    with open(path, encoding="utf-8") as f:
        return yaml.safe_load(f)


def extract_graph(dag: dict) -> tuple[set[str], dict[str, list[str]], dict[str, str]]:
    """从 dispatch-dag.yaml 提取节点集、邻接表、节点类型映射。"""
    nodes: set[str] = set()
    node_type: dict[str, str] = {}
    adj: dict[str, list[str]] = defaultdict(list)

    # ceremony 是隐式源节点
    nodes.add("ceremony")
    node_type["ceremony"] = "source"

    # 结构节点
    for n in dag.get("nodes", {}).get("structural", []):
        nid = n["id"]
        nodes.add(nid)
        node_type[nid] = n["type"]

    # 按需结构节点
    for n in dag.get("nodes", {}).get("optional_structural", []):
        nid = n["id"]
        nodes.add(nid)
        node_type[nid] = n.get("type", "conditional")

    # 结构边
    for edge in dag.get("edges", {}).get("structural_edges", []):
        src = edge["from"]
        targets = edge["to"] if isinstance(edge["to"], list) else [edge["to"]]
        for t in targets:
            nodes.add(src)
            nodes.add(t)
            adj[src].append(t)

    return nodes, dict(adj), node_type


def detect_cycle(nodes: set[str], adj: dict[str, list[str]], self_loop_exceptions: set[str]) -> list[str] | None:
    """Kahn's algorithm。返回拓扑序列，有环则返回 None。"""
    # 过滤自环例外
    filtered_adj: dict[str, list[str]] = defaultdict(list)
    in_degree: dict[str, int] = {n: 0 for n in nodes}

    for src, targets in adj.items():
        for t in targets:
            if src == t and src in self_loop_exceptions:
                continue  # 合法自环，跳过
            filtered_adj[src].append(t)
            in_degree[t] = in_degree.get(t, 0) + 1

    queue = deque(n for n in nodes if in_degree.get(n, 0) == 0)
    order: list[str] = []

    while queue:
        node = queue.popleft()
        order.append(node)
        for neighbor in filtered_adj.get(node, []):
            in_degree[neighbor] -= 1
            if in_degree[neighbor] == 0:
                queue.append(neighbor)

    if len(order) != len(nodes):
        cycle_nodes = [n for n in nodes if n not in set(order)]
        return cycle_nodes  # 返回环中的节点
    return None


def topological_sort(nodes: set[str], adj: dict[str, list[str]], self_loop_exceptions: set[str]) -> list[str]:
    """返回拓扑排序结果。"""
    filtered_adj: dict[str, list[str]] = defaultdict(list)
    in_degree: dict[str, int] = {n: 0 for n in nodes}

    for src, targets in adj.items():
        for t in targets:
            if src == t and src in self_loop_exceptions:
                continue
            filtered_adj[src].append(t)
            in_degree[t] = in_degree.get(t, 0) + 1

    queue = deque(n for n in nodes if in_degree.get(n, 0) == 0)
    order: list[str] = []

    while queue:
        node = queue.popleft()
        order.append(node)
        for neighbor in filtered_adj.get(node, []):
            in_degree[neighbor] -= 1
            if in_degree[neighbor] == 0:
                queue.append(neighbor)

    return order


def validate_single_terminal_sink(node_type: dict[str, str]) -> tuple[bool, list[str]]:
    sinks = [n for n, t in node_type.items() if t == "terminal_sink"]
    return len(sinks) == 1, sinks


def validate_dominator_reachability(
    nodes: set[str],
    adj: dict[str, list[str]],
    node_type: dict[str, str],
    terminal_sink: str,
) -> tuple[bool, list[str]]:
    """验证：所有非 dominator 节点到 terminal_sink 的路径必须经过至少一个 dominator。

    方法：从 terminal_sink 反向 BFS，移除所有 dominator 节点后，
    如果任何非 dominator 节点仍能到达 terminal_sink，则违反约束。
    """
    dominators = {n for n, t in node_type.items() if t == "dominator"}

    # 构建反向邻接表
    rev_adj: dict[str, list[str]] = defaultdict(list)
    for src, targets in adj.items():
        for t in targets:
            if src != t:  # 忽略自环
                rev_adj[t].append(src)

    # 从 terminal_sink 反向 BFS，但不穿过 dominator 节点
    reachable_without_dominator: set[str] = set()
    queue = deque([terminal_sink])
    reachable_without_dominator.add(terminal_sink)

    while queue:
        node = queue.popleft()
        for pred in rev_adj.get(node, []):
            if pred not in reachable_without_dominator and pred not in dominators:
                reachable_without_dominator.add(pred)
                queue.append(pred)

    # 非 dominator、非 source、非 terminal_sink 的节点不应在此集合中
    violators = [
        n for n in reachable_without_dominator
        if n != terminal_sink and node_type.get(n) not in ("dominator", "source", "terminal_sink")
    ]
    return len(violators) == 0, violators


def validate_connectivity(nodes: set[str], adj: dict[str, list[str]], source: str) -> tuple[bool, list[str]]:
    """从 source 出发 BFS，检查所有节点是否可达。"""
    visited: set[str] = set()
    queue = deque([source])
    visited.add(source)

    while queue:
        node = queue.popleft()
        for neighbor in adj.get(node, []):
            if neighbor not in visited:
                visited.add(neighbor)
                queue.append(neighbor)

    unreachable = [n for n in nodes if n not in visited]
    return len(unreachable) == 0, unreachable


def ceremony_topo_sort(dag: dict, start_type: str) -> list[str] | None:
    """对 ceremony_sequence 的 cold_start 或 warm_start 子 DAG 做拓扑排序。"""
    seq = dag.get("ceremony_sequence", {}).get(start_type, {})
    ceremony_nodes = seq.get("nodes", [])
    if not ceremony_nodes:
        return None

    ids = {n["id"] for n in ceremony_nodes}
    adj: dict[str, list[str]] = defaultdict(list)
    in_degree = {n["id"]: 0 for n in ceremony_nodes}

    for n in ceremony_nodes:
        for dep in n.get("depends_on", []):
            adj[dep].append(n["id"])
            in_degree[n["id"]] += 1

    queue = deque(nid for nid, deg in in_degree.items() if deg == 0)
    order: list[str] = []

    while queue:
        node = queue.popleft()
        order.append(node)
        for neighbor in adj.get(node, []):
            in_degree[neighbor] -= 1
            if in_degree[neighbor] == 0:
                queue.append(neighbor)

    return order if len(order) == len(ids) else None


def main() -> None:
    dag_path = Path(__file__).resolve().parent.parent.parent / ".chanlun" / "dispatch-dag.yaml"
    if not dag_path.exists():
        print(f"ERROR: {dag_path} not found")
        sys.exit(1)

    dag = load_dag(dag_path)
    nodes, adj, node_type = extract_graph(dag)

    print("=" * 60)
    print("dispatch-dag.yaml DAG 执行引擎验证")
    print("=" * 60)

    # 1. 节点清单
    print(f"\n[节点] 共 {len(nodes)} 个:")
    for n in sorted(nodes):
        print(f"  {n:25s} type={node_type.get(n, 'unknown')}")

    # 2. 环检测
    self_loop_exceptions = {"meta-observer"}
    cycle_nodes = detect_cycle(nodes, adj, self_loop_exceptions)
    if cycle_nodes is not None:
        print(f"\n[FAIL] 检测到环路，涉及节点: {cycle_nodes}")
        print("拒绝实例化。")
        sys.exit(1)
    print("\n[PASS] 无环（meta-observer 自环已豁免）")

    # 3. 拓扑排序
    order = topological_sort(nodes, adj, self_loop_exceptions)
    print(f"\n[执行序列] 拓扑排序:")
    for i, n in enumerate(order):
        print(f"  {i + 1}. {n}")

    # 4. 唯一汇点验证
    ok, sinks = validate_single_terminal_sink(node_type)
    if ok:
        print(f"\n[PASS] 唯一 terminal_sink: {sinks[0]}")
    else:
        print(f"\n[FAIL] terminal_sink 数量 != 1: {sinks}")
        sys.exit(1)

    # 5. Dominator 可达性验证
    ok, violators = validate_dominator_reachability(nodes, adj, node_type, sinks[0])
    if ok:
        print("[PASS] 所有非 dominator 节点到汇点的路径经过至少一个 dominator")
    else:
        print(f"[FAIL] 以下节点可绕过 dominator 到达汇点: {violators}")
        sys.exit(1)

    # 6. 连通性验证
    ok, unreachable = validate_connectivity(nodes, adj, "ceremony")
    if ok:
        print("[PASS] 所有节点从 ceremony 可达")
    else:
        print(f"[WARN] 以下节点从 ceremony 不可达: {unreachable}")
        print("       （按需节点可能需要事件触发激活，非阻塞性警告）")

    # 7. Ceremony 子 DAG 验证
    for start_type in ("cold_start", "warm_start"):
        result = ceremony_topo_sort(dag, start_type)
        if result is None:
            print(f"\n[WARN] ceremony {start_type} 子 DAG 为空或有环")
        else:
            print(f"\n[PASS] ceremony {start_type} 拓扑序列:")
            for i, n in enumerate(result):
                print(f"  {i + 1}. {n}")

    print("\n" + "=" * 60)
    print("验证完成。所有 DAG 不变量通过。")
    print("=" * 60)


if __name__ == "__main__":
    main()
