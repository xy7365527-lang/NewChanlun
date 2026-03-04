"""
DM2: 离散 Morse 时间演化实验

按谱系编号递增构造子复形序列 K_1 ⊂ K_2 ⊂ ... ⊂ K_N，
对每个 K_n 计算离散 Morse 函数的临界集，
记录临界集变化的时间序列。跳跃点对应关键否定事件。

认识论等级：L1（管线正确性验证——子复形序列来自真实 relations.jsonl，
但时间演化的解释需 L2 验证：与编排者直觉对比，即 DM3）。

谱系依据：351号（离散 Morse 理论研究线）
"""

from __future__ import annotations

import json
import re
import sys
from dataclasses import dataclass
from pathlib import Path
from typing import FrozenSet, Tuple

# DM1 模块复用
sys.path.insert(0, str(Path(__file__).resolve().parent.parent / "experiments" / "discrete_morse"))
from simplicial_complex import (
    SimplicialComplex,
    build_simplicial_complex,
    compute_betti_numbers,
    discrete_morse_greedy,
    verify_morse_inequalities,
)


# ─────────────────────────────────────────────────────────────────────────────
# 谱系编号排序
# ─────────────────────────────────────────────────────────────────────────────

def parse_genealogy_key(key: str) -> tuple[int, str]:
    """将谱系编号解析为 (数字部分, 后缀) 用于排序。

    例：'005' -> (5, ''), '005a' -> (5, 'a'), '073b' -> (73, 'b')
    """
    m = re.match(r"(\d+)(.*)", key)
    if not m:
        return (999999, key)
    return (int(m.group(1)), m.group(2))


def sorted_genealogy_keys(id_mapping: dict[str, str]) -> list[str]:
    """按谱系编号排序（数字优先，后缀次之）。"""
    return sorted(id_mapping.keys(), key=parse_genealogy_key)


# ─────────────────────────────────────────────────────────────────────────────
# 子复形序列构造
# ─────────────────────────────────────────────────────────────────────────────

@dataclass(frozen=True)
class TemporalSnapshot:
    """某一时刻的拓扑快照。"""
    step: int                  # 步骤编号（0-based）
    genealogy_key: str         # 当前步添加的谱系编号
    num_vertices: int
    num_edges: int
    num_triangles: int
    beta_0: int
    beta_1: int
    beta_2: int
    euler_char: int
    m0: int
    m1: int
    m2: int
    morse_all_pass: bool
    # 增量
    delta_vertices: int
    delta_edges: int
    delta_triangles: int
    delta_m0: int
    delta_m1: int
    delta_m2: int


def build_subcomplex_relations(
    all_relations: list[dict],
    included_hashes: set[str],
) -> list[dict]:
    """过滤出仅涉及 included_hashes 中节点的关系。"""
    return [
        rel for rel in all_relations
        if rel["from"] in included_hashes and rel["to"] in included_hashes
    ]


def run_temporal_evolution(
    relations_path: Path,
    meta_path: Path,
    *,
    skip_unchanged: bool = True,
) -> list[TemporalSnapshot]:
    """执行时间演化实验。

    Parameters
    ----------
    relations_path : Path to relations.jsonl
    meta_path : Path to meta.json
    skip_unchanged : bool
        如果 True，跳过子复形未发生变化的步骤（节省计算）。
        输出中仍保留所有步骤——未变化步骤复用上一步快照。
    """
    # 1. 加载数据
    with open(meta_path, "r", encoding="utf-8") as f:
        meta = json.load(f)
    id_mapping: dict[str, str] = meta["id_mapping"]

    all_relations: list[dict] = []
    with open(relations_path, "r", encoding="utf-8") as f:
        for line in f:
            line = line.strip()
            if not line:
                continue
            all_relations.append(json.loads(line))

    # 2. 按谱系编号排序
    sorted_keys = sorted_genealogy_keys(id_mapping)

    # 3. 预计算：为每条关系标记其 from/to 对应的谱系编号
    #    这样可以快速判断某条关系在哪一步首次出现
    hash_to_key: dict[str, str] = {}
    for key, h in id_mapping.items():
        hash_to_key[h] = key

    # 收集所有关系中涉及的 hash（包括 from, to, created_by）
    # 注意：有些 hash 可能不在 id_mapping 中（可能是中间产物或未映射区块）
    # 对于不在 id_mapping 中的 hash，我们不作为"谱系节点"追踪

    # 4. 增量构造子复形
    snapshots: list[TemporalSnapshot] = []
    included_hashes: set[str] = set()
    prev_snap: TemporalSnapshot | None = None

    # 预计算：每个谱系编号对应的 hash
    key_to_hash = id_mapping

    # 预计算：对于每条关系，确定其 from 和 to 的谱系编号
    # 只保留 from 和 to 都在 id_mapping 中的关系
    mapped_relations: list[dict] = []
    for rel in all_relations:
        if rel["from"] in hash_to_key and rel["to"] in hash_to_key:
            mapped_relations.append(rel)

    print(f"总关系数: {len(all_relations)}, 映射内关系数: {len(mapped_relations)}")
    print(f"谱系编号数: {len(sorted_keys)}")
    print()

    for step, key in enumerate(sorted_keys):
        block_hash = key_to_hash[key]
        included_hashes.add(block_hash)

        # 过滤当前步的关系
        current_relations = build_subcomplex_relations(mapped_relations, included_hashes)

        # 构造子复形
        # 注意：如果没有关系，build_simplicial_complex 仍需要至少一个虚拟关系
        # 来包含所有节点。我们需要修改方法或手动添加孤立节点。
        # 更好的办法：构造一个包含所有 included_hashes 作为节点的关系列表
        # 即使没有边，也要把节点纳入

        # 构造 "虚拟" 关系列表：自环（会被排除但节点会被纳入）
        node_relations = [{"from": h, "to": h} for h in included_hashes]
        combined_relations = node_relations + current_relations

        sc = build_simplicial_complex(combined_relations)

        # 检查是否与上一步相同
        if (skip_unchanged and prev_snap is not None
                and sc.num_vertices == prev_snap.num_vertices
                and sc.num_edges == prev_snap.num_edges
                and sc.num_triangles == prev_snap.num_triangles):
            # 复用上一步快照，只更新 step 和 key
            snap = TemporalSnapshot(
                step=step,
                genealogy_key=key,
                num_vertices=prev_snap.num_vertices,
                num_edges=prev_snap.num_edges,
                num_triangles=prev_snap.num_triangles,
                beta_0=prev_snap.beta_0,
                beta_1=prev_snap.beta_1,
                beta_2=prev_snap.beta_2,
                euler_char=prev_snap.euler_char,
                m0=prev_snap.m0,
                m1=prev_snap.m1,
                m2=prev_snap.m2,
                morse_all_pass=prev_snap.morse_all_pass,
                delta_vertices=0,
                delta_edges=0,
                delta_triangles=0,
                delta_m0=0,
                delta_m1=0,
                delta_m2=0,
            )
            snapshots.append(snap)
            prev_snap = snap
            continue

        # 计算 Betti 数和 Morse 函数
        betti = compute_betti_numbers(sc)
        morse = discrete_morse_greedy(sc)
        verification = verify_morse_inequalities(betti, morse)

        d_v = sc.num_vertices - (prev_snap.num_vertices if prev_snap else 0)
        d_e = sc.num_edges - (prev_snap.num_edges if prev_snap else 0)
        d_t = sc.num_triangles - (prev_snap.num_triangles if prev_snap else 0)
        d_m0 = morse.m0 - (prev_snap.m0 if prev_snap else 0)
        d_m1 = morse.m1 - (prev_snap.m1 if prev_snap else 0)
        d_m2 = morse.m2 - (prev_snap.m2 if prev_snap else 0)

        snap = TemporalSnapshot(
            step=step,
            genealogy_key=key,
            num_vertices=sc.num_vertices,
            num_edges=sc.num_edges,
            num_triangles=sc.num_triangles,
            beta_0=betti.beta_0,
            beta_1=betti.beta_1,
            beta_2=betti.beta_2,
            euler_char=betti.euler_characteristic(),
            m0=morse.m0,
            m1=morse.m1,
            m2=morse.m2,
            morse_all_pass=verification["all_pass"],
            delta_vertices=d_v,
            delta_edges=d_e,
            delta_triangles=d_t,
            delta_m0=d_m0,
            delta_m1=d_m1,
            delta_m2=d_m2,
        )
        snapshots.append(snap)
        prev_snap = snap

        # 进度输出（每 50 步或复形有变化时）
        if step % 50 == 0 or abs(d_m0) + abs(d_m1) + abs(d_m2) > 5:
            print(f"  [{step:3d}] {key:>5s} | V={sc.num_vertices:4d} E={sc.num_edges:4d} "
                  f"T={sc.num_triangles:4d} | β₀={betti.beta_0} β₁={betti.beta_1} "
                  f"β₂={betti.beta_2} | m₀={morse.m0} m₁={morse.m1} m₂={morse.m2} "
                  f"| Δm=({d_m0:+d},{d_m1:+d},{d_m2:+d})")

    return snapshots


# ─────────────────────────────────────────────────────────────────────────────
# 跳跃点检测
# ─────────────────────────────────────────────────────────────────────────────

@dataclass(frozen=True)
class JumpPoint:
    """Morse 临界集的跳跃点。"""
    step: int
    genealogy_key: str
    delta_m0: int
    delta_m1: int
    delta_m2: int
    magnitude: int  # |Δm₀| + |Δm₁| + |Δm₂|


def detect_jumps(
    snapshots: list[TemporalSnapshot],
    threshold: int = 3,
) -> list[JumpPoint]:
    """检测 Morse 临界集变化的跳跃点。

    threshold: 只报告 |Δm₀|+|Δm₁|+|Δm₂| >= threshold 的步骤。
    """
    jumps: list[JumpPoint] = []
    for snap in snapshots:
        mag = abs(snap.delta_m0) + abs(snap.delta_m1) + abs(snap.delta_m2)
        if mag >= threshold:
            jumps.append(JumpPoint(
                step=snap.step,
                genealogy_key=snap.genealogy_key,
                delta_m0=snap.delta_m0,
                delta_m1=snap.delta_m1,
                delta_m2=snap.delta_m2,
                magnitude=mag,
            ))
    return jumps


# ─────────────────────────────────────────────────────────────────────────────
# 序列化
# ─────────────────────────────────────────────────────────────────────────────

def snapshots_to_dict(snapshots: list[TemporalSnapshot]) -> list[dict]:
    """将快照序列转为可 JSON 序列化的字典列表。"""
    return [
        {
            "step": s.step,
            "genealogy_key": s.genealogy_key,
            "complex": {
                "vertices": s.num_vertices,
                "edges": s.num_edges,
                "triangles": s.num_triangles,
            },
            "betti": {
                "beta_0": s.beta_0,
                "beta_1": s.beta_1,
                "beta_2": s.beta_2,
                "euler_characteristic": s.euler_char,
            },
            "morse": {
                "m0": s.m0,
                "m1": s.m1,
                "m2": s.m2,
            },
            "deltas": {
                "vertices": s.delta_vertices,
                "edges": s.delta_edges,
                "triangles": s.delta_triangles,
                "m0": s.delta_m0,
                "m1": s.delta_m1,
                "m2": s.delta_m2,
            },
            "morse_all_pass": s.morse_all_pass,
        }
        for s in snapshots
    ]


def jumps_to_dict(jumps: list[JumpPoint]) -> list[dict]:
    """将跳跃点序列转为可 JSON 序列化的字典列表。"""
    return [
        {
            "step": j.step,
            "genealogy_key": j.genealogy_key,
            "delta_m0": j.delta_m0,
            "delta_m1": j.delta_m1,
            "delta_m2": j.delta_m2,
            "magnitude": j.magnitude,
        }
        for j in jumps
    ]


# ─────────────────────────────────────────────────────────────────────────────
# 主入口
# ─────────────────────────────────────────────────────────────────────────────

def main() -> None:
    repo_root = Path(__file__).resolve().parent.parent
    relations_path = repo_root / ".chanlun" / "block-topology" / "relations.jsonl"
    meta_path = repo_root / ".chanlun" / "block-topology" / "meta.json"

    if not relations_path.exists():
        print(f"Error: {relations_path} not found")
        sys.exit(1)
    if not meta_path.exists():
        print(f"Error: {meta_path} not found")
        sys.exit(1)

    print("=== DM2 离散 Morse 时间演化实验 ===\n")
    print(f"数据源: {relations_path}")
    print(f"元数据: {meta_path}")
    print()

    # 运行时间演化
    snapshots = run_temporal_evolution(relations_path, meta_path)

    # 检测跳跃点
    jumps = detect_jumps(snapshots, threshold=3)

    # 输出跳跃点
    print(f"\n=== 跳跃点检测（阈值: |Δm| >= 3）===\n")
    print(f"共 {len(jumps)} 个跳跃点:\n")
    print(f"{'步骤':>4s} {'谱系号':>6s} | {'Δm₀':>5s} {'Δm₁':>5s} {'Δm₂':>5s} | {'幅度':>4s}")
    print("-" * 50)
    for j in jumps:
        print(f"{j.step:4d} {j.genealogy_key:>6s} | {j.delta_m0:+5d} {j.delta_m1:+5d} "
              f"{j.delta_m2:+5d} | {j.magnitude:4d}")

    # Top-10 最大跳跃
    top_jumps = sorted(jumps, key=lambda j: j.magnitude, reverse=True)[:10]
    print(f"\n=== Top-10 最大跳跃 ===\n")
    for rank, j in enumerate(top_jumps, 1):
        print(f"  #{rank:2d} [{j.genealogy_key:>5s}] 幅度={j.magnitude:4d} "
              f"Δm=({j.delta_m0:+d},{j.delta_m1:+d},{j.delta_m2:+d})")

    # 最终快照与 DM1 对比
    final = snapshots[-1]
    print(f"\n=== 最终快照（谱系 {final.genealogy_key}）===")
    print(f"  V={final.num_vertices} E={final.num_edges} T={final.num_triangles}")
    print(f"  β₀={final.beta_0} β₁={final.beta_1} β₂={final.beta_2} χ={final.euler_char}")
    print(f"  m₀={final.m0} m₁={final.m1} m₂={final.m2}")
    print(f"  Morse 不等式全部通过: {final.morse_all_pass}")

    # Morse 不等式一致性
    all_pass = all(s.morse_all_pass for s in snapshots)
    print(f"\n  全序列 Morse 不等式一致性: {'通过' if all_pass else '失败'}")
    if not all_pass:
        failures = [s for s in snapshots if not s.morse_all_pass]
        print(f"  失败步骤: {[s.genealogy_key for s in failures]}")

    # 保存结果
    output_path = Path(__file__).resolve().parent.parent / "experiments" / "discrete_morse" / "dm2_results.json"
    output_path.parent.mkdir(parents=True, exist_ok=True)

    result = {
        "experiment": "DM2",
        "description": "离散 Morse 时间演化——子复形序列的临界集变化",
        "epistemological_level": "L1",
        "total_steps": len(snapshots),
        "total_jumps": len(jumps),
        "jump_threshold": 3,
        "all_morse_pass": all_pass,
        "final_snapshot": {
            "genealogy_key": final.genealogy_key,
            "vertices": final.num_vertices,
            "edges": final.num_edges,
            "triangles": final.num_triangles,
            "betti": {"beta_0": final.beta_0, "beta_1": final.beta_1, "beta_2": final.beta_2},
            "morse": {"m0": final.m0, "m1": final.m1, "m2": final.m2},
        },
        "jumps": jumps_to_dict(jumps),
        "top_10_jumps": jumps_to_dict(top_jumps),
        "snapshots": snapshots_to_dict(snapshots),
    }

    with open(output_path, "w", encoding="utf-8") as f:
        json.dump(result, f, indent=2, ensure_ascii=False)
    print(f"\n结果已保存到: {output_path}")


if __name__ == "__main__":
    main()
