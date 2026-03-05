"""
离散 Cerf 理论片段——分岔点检测（阶段 B-M 第三步）

Cerf 理论研究 Morse 函数族的参数化变化。在离散版本中：
- 按谱系编号 t 递增构造子复形序列 K_1 ⊂ K_2 ⊂ ... ⊂ K_n
- 对每个 K_t 计算（加权）Morse 函数的临界集 C_t
- 分岔点 = C_t 和 C_{t+1} 之间临界集发生"质变"的 t

质变标准：
1. 临界数量变化超过 threshold * 当前临界数
2. 新临界单形的平均权重显著高于旧临界（高权重区块进入临界集）
3. Betti 数变化（拓扑类型改变）

认识论等级：L0（Cerf 定义是代数构造）+ L1（合成验证管线正确性）
有效域：分岔点检测在合成复形上代数正确，与编排者直觉的匹配度待 L2 验证

谱系依据：355号（L2 否定性结果）→ 阶段 B-M → optimal_morse.py → weighted_morse.py → 本模块
"""

from __future__ import annotations

import json
import re
import sys
from dataclasses import dataclass
from pathlib import Path
from typing import FrozenSet

# DM1 模块复用
sys.path.insert(0, str(Path(__file__).resolve().parent.parent / "experiments" / "discrete_morse"))
from simplicial_complex import (
    SimplicialComplex,
    build_simplicial_complex,
    compute_betti_numbers,
)

from weighted_morse import (
    WeightedComplex,
    build_weighted_complex_from_relations,
    weighted_morse_function,
)

from morse_temporal import (
    parse_genealogy_key,
    sorted_genealogy_keys,
    build_subcomplex_relations,
)


# ─────────────────────────────────────────────────────────────────────────────
# 数据结构
# ─────────────────────────────────────────────────────────────────────────────

@dataclass(frozen=True)
class MorseSnapshot:
    """某一时刻的加权 Morse 快照。"""
    t: int                          # 步骤编号（0-based）
    genealogy_key: str              # 当前步添加的谱系编号
    complex_size: int               # 单形总数 (V+E+T)
    num_vertices: int
    num_edges: int
    num_triangles: int
    critical_count: int             # 临界单形总数
    critical_0: int                 # 临界 0-单形数
    critical_1: int                 # 临界 1-单形数
    critical_2: int                 # 临界 2-单形数
    critical_simplices: tuple       # 所有临界单形 (dim, simplex) 的元组
    critical_edge_weights: tuple    # 临界边的权重元组
    betti_0: int
    betti_1: int
    betti_2: int


@dataclass(frozen=True)
class Bifurcation:
    """分岔点记录。"""
    t: int                          # 分岔发生的步骤编号
    genealogy_key: str              # 对应的谱系编号
    type: str                       # 分岔类型：critical_count / weight_shift / betti_change / compound
    magnitude: float                # 分岔幅度（归一化）
    description: str                # 人可读描述
    details: dict                   # 具体数值


# ─────────────────────────────────────────────────────────────────────────────
# 子复形序列 + 加权 Morse
# ─────────────────────────────────────────────────────────────────────────────

def build_temporal_morse_sequence(
    relations_path: str,
    weights: dict | None = None,
    *,
    meta_path: str | None = None,
) -> list[MorseSnapshot]:
    """按谱系编号递增构造子复形序列，对每个计算加权 Morse。

    Parameters
    ----------
    relations_path : str
        relations.jsonl 文件路径。
    weights : dict | None
        关系类型 -> 权重映射。None 时使用默认权重。
    meta_path : str | None
        meta.json 文件路径。None 时自动推断（relations_path 同目录下的 meta.json）。

    Returns
    -------
    list[MorseSnapshot]
        按谱系编号递增排列的 Morse 快照序列。
    """
    relations_p = Path(relations_path)
    if meta_path is None:
        meta_p = relations_p.parent / "meta.json"
    else:
        meta_p = Path(meta_path)

    # 加载数据
    with open(meta_p, "r", encoding="utf-8") as f:
        meta = json.load(f)
    id_mapping: dict[str, str] = meta["id_mapping"]

    all_relations: list[dict] = []
    with open(relations_p, "r", encoding="utf-8") as f:
        for line in f:
            line = line.strip()
            if not line:
                continue
            all_relations.append(json.loads(line))

    # 按谱系编号排序
    sorted_keys = sorted_genealogy_keys(id_mapping)

    # hash -> key 反向映射
    hash_to_key: dict[str, str] = {}
    for key, h in id_mapping.items():
        hash_to_key[h] = key

    # 只保留 from 和 to 都在 id_mapping 中的关系
    mapped_relations: list[dict] = [
        rel for rel in all_relations
        if rel["from"] in hash_to_key and rel["to"] in hash_to_key
    ]

    # 增量构造子复形
    snapshots: list[MorseSnapshot] = []
    included_hashes: set[str] = set()

    for step, key in enumerate(sorted_keys):
        block_hash = id_mapping[key]
        included_hashes.add(block_hash)

        # 过滤当前步的关系
        current_relations = build_subcomplex_relations(mapped_relations, included_hashes)

        # 自环确保所有节点纳入
        node_relations = [{"from": h, "to": h} for h in included_hashes]
        combined_relations = node_relations + current_relations

        # 构造加权复形
        wc = build_weighted_complex_from_relations(combined_relations, weights)
        sc = wc.simplicial_complex

        # 计算加权 Morse
        w_result = weighted_morse_function(wc)
        gf = w_result.gradient_field
        betti = w_result.betti

        # 收集临界单形信息
        critical_simplices = (
            tuple((0, s) for s in gf.unpaired_0)
            + tuple((1, s) for s in gf.unpaired_1)
            + tuple((2, s) for s in gf.unpaired_2)
        )

        snap = MorseSnapshot(
            t=step,
            genealogy_key=key,
            complex_size=sc.num_vertices + sc.num_edges + sc.num_triangles,
            num_vertices=sc.num_vertices,
            num_edges=sc.num_edges,
            num_triangles=sc.num_triangles,
            critical_count=gf.num_critical,
            critical_0=len(gf.unpaired_0),
            critical_1=len(gf.unpaired_1),
            critical_2=len(gf.unpaired_2),
            critical_simplices=critical_simplices,
            critical_edge_weights=tuple(w_result.critical_edge_weights),
            betti_0=betti.beta_0,
            betti_1=betti.beta_1,
            betti_2=betti.beta_2,
        )
        snapshots.append(snap)

    return snapshots


# ─────────────────────────────────────────────────────────────────────────────
# 分岔点检测
# ─────────────────────────────────────────────────────────────────────────────

def detect_bifurcations(
    sequence: list[MorseSnapshot],
    threshold: float = 0.3,
) -> list[Bifurcation]:
    """检测分岔点：临界集发生质变的位置。

    质变标准（任一满足即为分岔点）：
    1. 临界数量变化超过 threshold * 当前临界数
    2. 新临界单形的平均权重显著高于旧临界（高权重区块进入临界集）
    3. Betti 数变化（拓扑类型改变）

    当多种标准同时满足时 type = "compound"。

    Parameters
    ----------
    sequence : list[MorseSnapshot]
        build_temporal_morse_sequence 的输出。
    threshold : float
        临界数量相对变化的阈值（默认 0.3 = 30%）。

    Returns
    -------
    list[Bifurcation]
        分岔点列表，按步骤编号排序。
    """
    if len(sequence) < 2:
        return []

    bifurcations: list[Bifurcation] = []

    for i in range(1, len(sequence)):
        prev = sequence[i - 1]
        curr = sequence[i]

        triggers: list[tuple[str, float, str, dict]] = []

        # 标准1：临界数量相对变化
        if prev.critical_count > 0:
            delta = abs(curr.critical_count - prev.critical_count)
            relative_change = delta / prev.critical_count
            if relative_change > threshold:
                triggers.append((
                    "critical_count",
                    relative_change,
                    f"临界数量变化 {prev.critical_count} -> {curr.critical_count} "
                    f"(相对变化 {relative_change:.2%})",
                    {
                        "prev_count": prev.critical_count,
                        "curr_count": curr.critical_count,
                        "delta": curr.critical_count - prev.critical_count,
                        "relative_change": relative_change,
                    },
                ))
        elif curr.critical_count > 0:
            # 从 0 到非零——绝对分岔
            triggers.append((
                "critical_count",
                1.0,
                f"临界数量从 0 变为 {curr.critical_count}",
                {
                    "prev_count": 0,
                    "curr_count": curr.critical_count,
                    "delta": curr.critical_count,
                    "relative_change": 1.0,
                },
            ))

        # 标准2：临界边权重偏移
        if prev.critical_edge_weights and curr.critical_edge_weights:
            prev_avg = sum(prev.critical_edge_weights) / len(prev.critical_edge_weights)
            curr_avg = sum(curr.critical_edge_weights) / len(curr.critical_edge_weights)
            if prev_avg > 0:
                weight_shift = (curr_avg - prev_avg) / prev_avg
                # 权重显著升高 = 高权重（否定性）关系进入临界集
                if abs(weight_shift) > threshold:
                    triggers.append((
                        "weight_shift",
                        abs(weight_shift),
                        f"临界边平均权重偏移 {prev_avg:.2f} -> {curr_avg:.2f} "
                        f"(变化 {weight_shift:+.2%})",
                        {
                            "prev_avg_weight": prev_avg,
                            "curr_avg_weight": curr_avg,
                            "weight_shift": weight_shift,
                        },
                    ))

        # 标准3：Betti 数变化
        betti_delta = (
            abs(curr.betti_0 - prev.betti_0)
            + abs(curr.betti_1 - prev.betti_1)
            + abs(curr.betti_2 - prev.betti_2)
        )
        if betti_delta > 0:
            triggers.append((
                "betti_change",
                float(betti_delta),
                f"Betti 数变化: "
                f"({prev.betti_0},{prev.betti_1},{prev.betti_2}) -> "
                f"({curr.betti_0},{curr.betti_1},{curr.betti_2})",
                {
                    "prev_betti": (prev.betti_0, prev.betti_1, prev.betti_2),
                    "curr_betti": (curr.betti_0, curr.betti_1, curr.betti_2),
                    "delta_betti_0": curr.betti_0 - prev.betti_0,
                    "delta_betti_1": curr.betti_1 - prev.betti_1,
                    "delta_betti_2": curr.betti_2 - prev.betti_2,
                    "total_delta": betti_delta,
                },
            ))

        if not triggers:
            continue

        # 确定分岔类型
        trigger_types = [t[0] for t in triggers]
        if len(trigger_types) > 1:
            bif_type = "compound"
        else:
            bif_type = trigger_types[0]

        # 综合幅度 = 各标准幅度之和
        total_magnitude = sum(t[1] for t in triggers)

        # 综合描述
        descriptions = [t[2] for t in triggers]
        description = " | ".join(descriptions)

        # 合并 details
        merged_details: dict = {}
        for t in triggers:
            merged_details[t[0]] = t[3]

        bifurcations.append(Bifurcation(
            t=curr.t,
            genealogy_key=curr.genealogy_key,
            type=bif_type,
            magnitude=total_magnitude,
            description=description,
            details=merged_details,
        ))

    return bifurcations


# ─────────────────────────────────────────────────────────────────────────────
# 谱系标注
# ─────────────────────────────────────────────────────────────────────────────

def _parse_frontmatter(text: str) -> dict:
    """简易 YAML frontmatter 解析（不依赖 PyYAML）。

    只处理简单的 key: value 和 key: [list] 格式。
    """
    result: dict = {}
    lines = text.strip().split("\n")
    for line in lines:
        line = line.strip()
        if not line or line == "---":
            continue
        m = re.match(r'^(\w+)\s*:\s*(.*)$', line)
        if not m:
            continue
        key = m.group(1)
        value = m.group(2).strip()
        # 去除引号
        if value.startswith('"') and value.endswith('"'):
            value = value[1:-1]
        # 解析列表
        if value.startswith('[') and value.endswith(']'):
            inner = value[1:-1].strip()
            if not inner:
                result[key] = []
            else:
                items = [item.strip().strip('"').strip("'") for item in inner.split(",")]
                result[key] = items
        else:
            result[key] = value
    return result


def annotate_bifurcations_with_genealogy(
    bifurcations: list[Bifurcation],
    settled_dir: str,
) -> list[dict]:
    """将分岔点的谱系编号映射到具体的谱系事件。

    对每个分岔点，读取对应谱系的 frontmatter，标注是否为已知的"关键否定"。

    Parameters
    ----------
    bifurcations : list[Bifurcation]
        detect_bifurcations 的输出。
    settled_dir : str
        settled 谱系目录路径。

    Returns
    -------
    list[dict]
        每个分岔点附加谱系标注后的字典列表。
    """
    settled_p = Path(settled_dir)
    annotated: list[dict] = []

    # 预加载目录中的文件名，建立谱系编号到文件路径的映射
    file_map: dict[str, Path] = {}
    if settled_p.exists():
        for f in settled_p.iterdir():
            if f.suffix == ".md":
                # 文件名格式：001-degenerate-segment.md → key = "001"
                name_key = f.stem.split("-")[0]
                file_map[name_key] = f

    for bif in bifurcations:
        entry: dict = {
            "t": bif.t,
            "genealogy_key": bif.genealogy_key,
            "type": bif.type,
            "magnitude": bif.magnitude,
            "description": bif.description,
            "details": bif.details,
            "genealogy_annotation": None,
        }

        # 查找对应谱系文件
        file_path = file_map.get(bif.genealogy_key)
        if file_path is not None and file_path.exists():
            text = file_path.read_text(encoding="utf-8")
            # 提取 frontmatter
            fm_match = re.match(r'^---\s*\n(.*?)\n---', text, re.DOTALL)
            if fm_match:
                fm = _parse_frontmatter(fm_match.group(1))
                is_negation = bool(
                    fm.get("negates")
                    or fm.get("negated_by")
                )
                entry["genealogy_annotation"] = {
                    "title": fm.get("title", ""),
                    "status": fm.get("status", ""),
                    "type": fm.get("type", ""),
                    "negates": fm.get("negates", []),
                    "negated_by": fm.get("negated_by", []),
                    "is_critical_negation": is_negation,
                    "file": str(file_path),
                }

        annotated.append(entry)

    return annotated


# ─────────────────────────────────────────────────────────────────────────────
# 总结
# ─────────────────────────────────────────────────────────────────────────────

def cerf_summary(
    sequence: list[MorseSnapshot],
    bifurcations: list[Bifurcation],
) -> dict:
    """生成 Cerf 分析总结。

    Parameters
    ----------
    sequence : list[MorseSnapshot]
        加权 Morse 快照序列。
    bifurcations : list[Bifurcation]
        分岔点列表。

    Returns
    -------
    dict
        包含：total_steps, total_bifurcations, bifurcation_rate,
        type_distribution, top_bifurcations, epistemological_level。
    """
    total = len(sequence)

    # 分岔类型分布
    type_dist: dict[str, int] = {}
    for bif in bifurcations:
        type_dist[bif.type] = type_dist.get(bif.type, 0) + 1

    # Top-10 最大分岔
    sorted_bifs = sorted(bifurcations, key=lambda b: b.magnitude, reverse=True)
    top_10 = [
        {
            "t": b.t,
            "genealogy_key": b.genealogy_key,
            "type": b.type,
            "magnitude": b.magnitude,
            "description": b.description,
        }
        for b in sorted_bifs[:10]
    ]

    # 序列统计
    if sequence:
        final = sequence[-1]
        initial = sequence[0]
        final_info = {
            "genealogy_key": final.genealogy_key,
            "complex_size": final.complex_size,
            "critical_count": final.critical_count,
            "betti": (final.betti_0, final.betti_1, final.betti_2),
        }
        initial_info = {
            "genealogy_key": initial.genealogy_key,
            "complex_size": initial.complex_size,
            "critical_count": initial.critical_count,
            "betti": (initial.betti_0, initial.betti_1, initial.betti_2),
        }
    else:
        final_info = {}
        initial_info = {}

    return {
        "total_steps": total,
        "total_bifurcations": len(bifurcations),
        "bifurcation_rate": len(bifurcations) / total if total > 0 else 0.0,
        "type_distribution": type_dist,
        "top_10_bifurcations": top_10,
        "initial_snapshot": initial_info,
        "final_snapshot": final_info,
        "epistemological_level": "L0 (Cerf 定义) + L1 (合成验证)",
        "validity_domain": (
            "分岔点检测在离散子复形序列上代数正确。"
            "与编排者直觉（关键否定事件）的匹配度属 L2 范畴，"
            "本步骤在真实 relations.jsonl 上运行但结论标注为 L1/L2 边界。"
        ),
    }


# ─────────────────────────────────────────────────────────────────────────────
# 主入口
# ─────────────────────────────────────────────────────────────────────────────

def main() -> None:
    repo_root = Path(__file__).resolve().parent.parent
    relations_path = repo_root / ".chanlun" / "block-topology" / "relations.jsonl"
    meta_path = repo_root / ".chanlun" / "block-topology" / "meta.json"
    settled_dir = repo_root / ".chanlun" / "genealogy" / "settled"

    if not relations_path.exists():
        print(f"Error: {relations_path} not found")
        sys.exit(1)
    if not meta_path.exists():
        print(f"Error: {meta_path} not found")
        sys.exit(1)

    print("=== 离散 Cerf 分岔点检测（阶段 B-M 第三步）===\n")
    print(f"认识论等级: L0 (Cerf 定义) + L1 (合成验证)")
    print(f"有效域: 分岔点检测代数正确，与编排者直觉的匹配度待 L2 验证\n")

    # 1. 构造加权 Morse 时间序列
    print("构造加权 Morse 时间序列...")
    sequence = build_temporal_morse_sequence(
        str(relations_path),
        meta_path=str(meta_path),
    )
    print(f"  总步骤数: {len(sequence)}")
    if sequence:
        final = sequence[-1]
        print(f"  最终复形: V={final.num_vertices} E={final.num_edges} T={final.num_triangles}")
        print(f"  最终临界集: m0={final.critical_0} m1={final.critical_1} m2={final.critical_2}")
        print(f"  最终 Betti: ({final.betti_0}, {final.betti_1}, {final.betti_2})")

    # 2. 检测分岔点
    print("\n检测分岔点（threshold=0.3）...")
    bifurcations = detect_bifurcations(sequence, threshold=0.3)
    print(f"  分岔点数: {len(bifurcations)}")

    # 3. 谱系标注
    print("\n谱系标注...")
    annotated = annotate_bifurcations_with_genealogy(bifurcations, str(settled_dir))
    negation_count = sum(
        1 for a in annotated
        if a.get("genealogy_annotation", {})
        and a["genealogy_annotation"].get("is_critical_negation")
    )
    print(f"  标注完成: {negation_count}/{len(annotated)} 为关键否定事件")

    # 4. 总结
    summary = cerf_summary(sequence, bifurcations)

    # 输出分岔点列表
    print(f"\n=== 分岔点列表 ===\n")
    print(f"{'步骤':>4s} {'谱系号':>6s} | {'类型':>14s} | {'幅度':>8s} | 描述")
    print("-" * 90)
    for bif in bifurcations:
        print(f"{bif.t:4d} {bif.genealogy_key:>6s} | {bif.type:>14s} | "
              f"{bif.magnitude:8.3f} | {bif.description[:60]}")

    # Top-10
    print(f"\n=== Top-10 最大分岔 ===\n")
    for rank, entry in enumerate(summary["top_10_bifurcations"], 1):
        print(f"  #{rank:2d} [{entry['genealogy_key']:>5s}] "
              f"幅度={entry['magnitude']:.3f} 类型={entry['type']}")

    # 5. 保存结果
    output_dir = repo_root / "experiments" / "discrete_morse"
    output_dir.mkdir(parents=True, exist_ok=True)
    output_path = output_dir / "cerf_bifurcation_results.json"

    # 序列化 MorseSnapshot（去掉 frozenset 字段）
    def snapshot_to_dict(s: MorseSnapshot) -> dict:
        return {
            "t": s.t,
            "genealogy_key": s.genealogy_key,
            "complex_size": s.complex_size,
            "complex": {
                "vertices": s.num_vertices,
                "edges": s.num_edges,
                "triangles": s.num_triangles,
            },
            "critical": {
                "count": s.critical_count,
                "m0": s.critical_0,
                "m1": s.critical_1,
                "m2": s.critical_2,
            },
            "critical_edge_weights": list(s.critical_edge_weights),
            "betti": {
                "beta_0": s.betti_0,
                "beta_1": s.betti_1,
                "beta_2": s.betti_2,
            },
        }

    out = {
        "experiment": "Cerf Bifurcation Detection (B-M Step 3)",
        "epistemological_level": summary["epistemological_level"],
        "validity_domain": summary["validity_domain"],
        "summary": {
            "total_steps": summary["total_steps"],
            "total_bifurcations": summary["total_bifurcations"],
            "bifurcation_rate": summary["bifurcation_rate"],
            "type_distribution": summary["type_distribution"],
        },
        "top_10_bifurcations": summary["top_10_bifurcations"],
        "annotated_bifurcations": annotated,
        "snapshots": [snapshot_to_dict(s) for s in sequence],
    }

    with open(output_path, "w", encoding="utf-8") as f:
        json.dump(out, f, indent=2, ensure_ascii=False, default=str)
    print(f"\n结果已保存到: {output_path}")


if __name__ == "__main__":
    main()
