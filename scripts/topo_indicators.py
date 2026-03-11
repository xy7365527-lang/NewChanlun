#!/usr/bin/env python
"""可计算拓扑判据——独立于 agent 智能的 structural forcing 指标。

三个判据：
1. residue_density: 某区域 residue_of 关系的增长率（热区检测）
2. broken_dependency_chains: depends_on 链指向已被 negates 的 block
3. unstable_settled: settled 谱系前提中 >50% 被否定

数据来源：.chanlun/block-topology/relations.jsonl + meta.json + settled/

谱系依据：Gemini 质询→编排者指令（ceremony 不动点后跑拓扑指标）
"""
import json
import os
import re
from datetime import datetime, timezone


def _load_relations(root):
    """加载 relations.jsonl，返回关系列表。"""
    path = os.path.join(root, ".chanlun/block-topology/relations.jsonl")
    if not os.path.isfile(path):
        return []
    relations = []
    with open(path, encoding="utf-8") as f:
        for line in f:
            line = line.strip()
            if line:
                try:
                    relations.append(json.loads(line))
                except (json.JSONDecodeError, ValueError):
                    continue
    return relations


def _load_id_mapping(root):
    """加载 meta.json 的 id_mapping (sha -> 旧编号) 和 (旧编号 -> sha)。"""
    meta_path = os.path.join(root, ".chanlun/block-topology/meta.json")
    if not os.path.isfile(meta_path):
        return {}, {}
    try:
        with open(meta_path, encoding="utf-8") as f:
            meta = json.load(f)
        id_to_sha = meta.get("id_mapping", {})
        sha_to_id = {v: k for k, v in id_to_sha.items()}
        return id_to_sha, sha_to_id
    except Exception:
        return {}, {}


def _sha_to_label(sha, sha_to_id):
    """将 sha 转为可读标签：有旧编号用旧编号，否则取 sha 前 12 位。"""
    label = sha_to_id.get(sha)
    if label:
        return label
    return sha[:12]


def compute_residue_density(relations, sha_to_id, recent_n_days=30):
    """判据1：residue_of 密度变化。

    统计每个 residue_of target 区域的 residue 数量，
    按 timestamp 区分最近 N 天 vs 更早，计算增长率。
    增长快的区域是热区。

    返回：按增长率降序排列的区域列表。
    """
    now = datetime.now(timezone.utc)
    residue_by_target = {}  # target_sha -> {"recent": count, "older": count}

    for rel in relations:
        if rel.get("relation") != "residue_of":
            continue
        target = rel["to"]
        if target not in residue_by_target:
            residue_by_target[target] = {"recent": 0, "older": 0, "total": 0}

        ts_str = rel.get("timestamp", "")
        is_recent = False
        if ts_str:
            try:
                ts = datetime.fromisoformat(ts_str)
                if ts.tzinfo is None:
                    ts = ts.replace(tzinfo=timezone.utc)
                delta = (now - ts).days
                is_recent = delta <= recent_n_days
            except (ValueError, TypeError):
                pass

        if is_recent:
            residue_by_target[target]["recent"] += 1
        else:
            residue_by_target[target]["older"] += 1
        residue_by_target[target]["total"] += 1

    hotzones = []
    for target_sha, counts in residue_by_target.items():
        # 增长率 = recent / max(older, 1)
        # 全部是 recent 时 rate = recent count
        rate = counts["recent"] / max(counts["older"], 1)
        hotzones.append({
            "block": _sha_to_label(target_sha, sha_to_id),
            "block_sha": target_sha,
            "recent_count": counts["recent"],
            "older_count": counts["older"],
            "total": counts["total"],
            "growth_rate": round(rate, 2),
        })

    hotzones.sort(key=lambda x: -x["growth_rate"])
    return hotzones


def compute_broken_dependency_chains(relations, sha_to_id):
    """判据2：depends_on 链完整性。

    如果 A depends_on B，且 C negates B，则 A 的依赖基础已被动摇。
    额外条件：A 本身未被否定或更新（没有 negates 边指向 A，也没有
    更新的 supersedes/refines 边 from A）。

    过滤：自否定依赖（A depends_on B 且 A negates B）不算断链——
    A 依赖 B 正是为了否定 B 的某些方面（否定源型，见 v222 诊断报告）。

    返回：依赖被动摇但自身未更新的 block 列表。
    """
    # 收集 negated targets 和 negator->target 映射
    negated_blocks = set()
    blocks_that_negate = {}  # negated_target -> [negator_sha, ...]
    negator_targets = {}  # negator_sha -> set(negated_target_sha)
    for rel in relations:
        if rel.get("relation") == "negates":
            negated_blocks.add(rel["to"])
            blocks_that_negate.setdefault(rel["to"], []).append(rel["from"])
            negator_targets.setdefault(rel["from"], set()).add(rel["to"])

    if not negated_blocks:
        return []

    # 收集 depends_on 关系
    deps = []  # (from_sha, to_sha)
    for rel in relations:
        if rel.get("relation") == "depends_on":
            deps.append((rel["from"], rel["to"]))

    # 收集 "已更新" 的 block（自身被 negates/supersedes/refines 为 from）
    updated_blocks = set()
    for rel in relations:
        if rel.get("relation") in ("negates", "supersedes", "refines"):
            updated_blocks.add(rel["to"])

    broken = []
    seen = set()
    for from_sha, to_sha in deps:
        if to_sha in negated_blocks and from_sha not in updated_blocks:
            # 自否定依赖过滤：如果 A 自身就是否定 B 的来源，跳过
            if to_sha in negator_targets.get(from_sha, set()):
                continue
            if from_sha not in seen:
                seen.add(from_sha)
                negators = blocks_that_negate.get(to_sha, [])
                broken.append({
                    "block": _sha_to_label(from_sha, sha_to_id),
                    "block_sha": from_sha,
                    "depends_on": _sha_to_label(to_sha, sha_to_id),
                    "depends_on_sha": to_sha,
                    "negated_by": [
                        _sha_to_label(n, sha_to_id) for n in negators
                    ],
                })

    return broken


def compute_unstable_settled(relations, sha_to_id, root):
    """判据3：settled 前提偏差。

    对每个 settled 谱系（通过 id_mapping 在 block-topology 中有对应 block）：
    1. 收集该 block 的所有 depends_on targets
    2. 统计其中被 negates 的比例（排除自否定依赖）
    3. 比例 > 50% 的标记为 unstable

    过滤：自否定依赖（X depends_on Y 且 X negates Y）不计入
    negation_ratio——X 依赖 Y 正是为了否定 Y（否定源型/元记录型，
    见 v222 unstable-settled 诊断报告）。

    返回：按偏差比例降序排列的 unstable settled 谱系列表。
    """
    id_to_sha, _ = _load_id_mapping(root)
    if not id_to_sha:
        return []

    # 构建 negated 集合
    negated_blocks = set()
    for rel in relations:
        if rel.get("relation") == "negates":
            negated_blocks.add(rel["to"])

    if not negated_blocks:
        return []

    # 构建 negator->targets 映射（用于自否定过滤）
    negator_targets = {}  # negator_sha -> set(negated_target_sha)
    for rel in relations:
        if rel.get("relation") == "negates":
            negator_targets.setdefault(rel["from"], set()).add(rel["to"])

    # 构建 depends_on 映射：block_sha -> [target_sha, ...]
    deps_map = {}
    for rel in relations:
        if rel.get("relation") == "depends_on":
            deps_map.setdefault(rel["from"], []).append(rel["to"])

    # 检查 settled 目录下谱系文件
    settled_dir = os.path.join(root, ".chanlun/genealogy/settled")
    if not os.path.isdir(settled_dir):
        return []

    unstable = []
    for genealogy_id, sha in id_to_sha.items():
        targets = deps_map.get(sha, [])
        if not targets:
            continue
        # 过滤自否定依赖：X depends_on Y 且 X negates Y → 不计入
        self_negated = negator_targets.get(sha, set())
        filtered_targets = [t for t in targets if t not in self_negated]
        if not filtered_targets:
            continue
        negated_count = sum(1 for t in filtered_targets if t in negated_blocks)
        ratio = negated_count / len(filtered_targets)
        if ratio > 0.5:
            unstable.append({
                "genealogy_id": genealogy_id,
                "block_sha": sha,
                "total_deps": len(targets),
                "self_negated_deps": len(targets) - len(filtered_targets),
                "effective_deps": len(filtered_targets),
                "negated_deps": negated_count,
                "negation_ratio": round(ratio, 2),
            })

    unstable.sort(key=lambda x: -x["negation_ratio"])
    return unstable


def compute_all(root):
    """计算全部三个拓扑判据，返回 topo_indicators dict。

    每个判据独立计算，任一失败不阻塞其他判据。
    """
    relations = _load_relations(root)
    _, sha_to_id = _load_id_mapping(root)

    result = {}

    try:
        result["hotzone_residue_density"] = compute_residue_density(
            relations, sha_to_id
        )
    except Exception as exc:
        result["hotzone_residue_density_error"] = f"{type(exc).__name__}: {exc}"

    try:
        result["broken_dependency_chains"] = compute_broken_dependency_chains(
            relations, sha_to_id
        )
    except Exception as exc:
        result["broken_dependency_chains_error"] = f"{type(exc).__name__}: {exc}"

    try:
        result["unstable_settled"] = compute_unstable_settled(
            relations, sha_to_id, root
        )
    except Exception as exc:
        result["unstable_settled_error"] = f"{type(exc).__name__}: {exc}"

    return result


def has_anomalies(indicators):
    """判断 topo_indicators 中是否有异常项需要生成工位。"""
    if indicators.get("hotzone_residue_density"):
        # 有 residue 增长率 > 1.0 的热区
        for hz in indicators["hotzone_residue_density"]:
            if hz.get("growth_rate", 0) > 1.0:
                return True
    if indicators.get("broken_dependency_chains"):
        return True
    if indicators.get("unstable_settled"):
        return True
    return False


def generate_workstations(indicators):
    """从异常指标生成工位列表。"""
    workstations = []

    # 热区工位
    hotzones = indicators.get("hotzone_residue_density", [])
    hot_blocks = [hz for hz in hotzones if hz.get("growth_rate", 0) > 1.0]
    if hot_blocks:
        block_labels = ", ".join(hz["block"] for hz in hot_blocks[:5])
        workstations.append({
            "priority": "P2",
            "name": f"拓扑热区诊断：{len(hot_blocks)}个 residue 热区",
            "status": f"hotzone_blocks: {block_labels}",
            "source": "topo_indicators",
        })

    # 断链工位
    broken = indicators.get("broken_dependency_chains", [])
    if broken:
        block_labels = ", ".join(b["block"] for b in broken[:5])
        workstations.append({
            "priority": "P1",
            "name": f"依赖链断裂：{len(broken)}条 depends_on 指向已否定 block",
            "status": f"broken_deps: {block_labels}",
            "source": "topo_indicators",
        })

    # unstable settled 工位
    unstable = indicators.get("unstable_settled", [])
    if unstable:
        ids = ", ".join(u["genealogy_id"] for u in unstable[:5])
        workstations.append({
            "priority": "P1",
            "name": f"前提偏差：{len(unstable)}个 settled 谱系前提 >50% 被否定",
            "status": f"unstable_ids: {ids}",
            "source": "topo_indicators",
        })

    return workstations


if __name__ == "__main__":
    import sys
    root = os.getcwd()
    if len(sys.argv) > 1:
        root = sys.argv[1]
    indicators = compute_all(root)
    print(json.dumps(indicators, ensure_ascii=False, indent=2))
    if has_anomalies(indicators):
        print("\n=== 异常工位 ===")
        for ws in generate_workstations(indicators):
            print(f"  [{ws['priority']}] {ws['name']}")
    else:
        print("\n无拓扑异常。")
