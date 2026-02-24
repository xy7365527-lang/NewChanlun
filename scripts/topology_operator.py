#!/usr/bin/env python3
"""矛盾→拓扑操作自动映射（147号谱系下游推论1 + 178号-2升格）。

读取谱系文件，根据否定类型执行对应的拓扑操作——写入 block-topology。

否定形式→拓扑操作映射（040号 + 147号）：
  - waiting（等待型）  → freeze（冻结路径）
  - expansion（扩张型）→ split（分裂节点）
  - separation（分离型）→ sever（切断连接）

输入来源优先级（141号结论1：retrospective > prospective）：
  1. topo_effect 字段（结构化格式 type:target:scope）— 直接执行
  2. negation_form 字段 — 推导拓扑操作类型（需要 target/scope 参数补充）

178号-2 升格：
  - topology_operator 不再读写 dag.yaml，改为通过 block_topology.py 写入区块+关系
  - dag.yaml 冻结在迁移时刻的快照，不再更新

用法:
  # 从谱系文件读取 topo_effect 并执行
  python scripts/topology_operator.py --genealogy .chanlun/genealogy/settled/147-xxx.md

  # 直接指定拓扑操作（需要 SHA256 id）
  python scripts/topology_operator.py --effect freeze:062:downstream --base .chanlun/block-topology

  # dry-run 模式（只报告，不写入 block-topology）
  python scripts/topology_operator.py --genealogy .chanlun/genealogy/settled/147-xxx.md --dry-run

  # 从 negation_form 推导（需要补充 target 和 scope）
  python scripts/topology_operator.py --genealogy .chanlun/genealogy/settled/090-xxx.md \\
    --target 086 --scope local
"""
from __future__ import annotations

import argparse
import datetime
import re
import sys
from pathlib import Path
from typing import Any

import yaml

from scripts.block_topology import (
    DEFAULT_BASE,
    compute_block_id,
    make_relation,
    read_all_relations,
    read_meta,
    write_block_with_relations,
)


# ── 否定形式→拓扑操作映射表（040号 + 147号） ──

NEGATION_FORM_TO_TOPO: dict[str, str] = {
    "waiting": "freeze",
    "expansion": "split",
    "separation": "sever",
}

TOPO_DESCRIPTIONS: dict[str, str] = {
    "freeze": "冻结路径（等待型否定）——冻结目标节点及其下游依赖，直到后续回溯规定解冻",
    "split": "分裂节点（扩张型否定）——记录此处发生了概念分裂事件",
    "sever": "切断连接（分离型否定）——切断目标节点与原路径的连接，形成独立路径",
}

VALID_TOPO_TYPES = frozenset({"freeze", "split", "sever"})
VALID_SCOPES = frozenset({"local", "downstream"})


def parse_topo_effect(effect_str: str) -> tuple[str, str, str] | None:
    """解析 topo_effect 结构化格式 'type:target:scope'。

    Returns:
        (effect_type, target_id, scope) 或 None（非结构化格式）。
    """
    if not effect_str or ":" not in effect_str:
        return None
    parts = effect_str.strip().strip('"').strip("'").split(":")
    if len(parts) != 3:
        return None
    effect_type, target_id, scope = parts
    if effect_type not in VALID_TOPO_TYPES:
        return None
    return (effect_type, target_id, scope)


def extract_genealogy_fields(content: str) -> dict[str, Any]:
    """从谱系文件内容中提取关键字段。"""
    fields: dict[str, Any] = {}

    # 提取 YAML frontmatter
    fm_keys: set[str] = set()
    fm_match = re.match(r"^---\s*\n(.+?)\n---", content, re.DOTALL)
    if fm_match:
        try:
            fm = yaml.safe_load(fm_match.group(1))
            if isinstance(fm, dict):
                fields.update(fm)
                fm_keys = set(fm.keys())
        except yaml.YAMLError:
            pass

    # 补充从正文提取（有些字段在正文中以 markdown 格式出现）
    # 当 frontmatter 中的值为空时也从正文补充
    if not fields.get("negation_form"):
        nf_match = re.search(
            r"\*\*negation_form\*\*:\s*(.+?)(?:\n|$)", content
        )
        if nf_match:
            raw = nf_match.group(1).strip()
            # 提取括号前的关键词: "expansion（xxx）" → "expansion"
            keyword = re.match(r"(\w+)", raw)
            if keyword:
                fields["negation_form"] = keyword.group(1)

    # topo_effect: 仅在 frontmatter 中没有该 key 时从正文提取
    # （frontmatter 中 topo_effect: "" 表示显式声明无拓扑效果，不应被正文覆盖）
    if "topo_effect" not in fm_keys and not fields.get("topo_effect"):
        te_match = re.search(
            r"topo_effect:\s*[\"']?(.+?)[\"']?\s*(?:\n|$)", content
        )
        if te_match:
            fields["topo_effect"] = te_match.group(1).strip()

    return fields


def infer_topo_type_from_negation_form(negation_form: str) -> str | None:
    """从 negation_form 推导拓扑操作类型。

    negation_form 可能是纯关键词（"expansion"）或带描述
    （"expansion（xxx）"）。提取首个关键词匹配。
    """
    if not negation_form:
        return None
    keyword = re.match(r"(\w+)", negation_form.strip())
    if not keyword:
        return None
    form = keyword.group(1).lower()
    return NEGATION_FORM_TO_TOPO.get(form)


# ── id 解析 ──

def resolve_genealogy_id(
    genealogy_id: str, base: Path = DEFAULT_BASE
) -> str | None:
    """通过 meta.json 的 id_mapping 将旧谱系 id 解析为区块 SHA256 id。

    Args:
        genealogy_id: 旧格式谱系 id（如 "001", "062"）。
        base: block-topology 根路径。

    Returns:
        SHA256 区块 id，或 None（id 不在映射表中）。
    """
    meta = read_meta(base)
    if meta is None:
        return None
    return meta.get("id_mapping", {}).get(str(genealogy_id))


# ── 拓扑操作写入（block-topology） ──

def write_freeze(
    source_id: str,
    target_id: str,
    scope: str,
    base: Path = DEFAULT_BASE,
) -> dict:
    """冻结拓扑操作——写入 event 区块 + freezes 关系。

    scope=downstream 的语义由查询方保证：查询时看到 scope: downstream
    就知道该区块的所有下游也被冻结。写入时不展开 downstream——区块不可变，
    不能回去改已有区块的 frozen 字段。

    Args:
        source_id: 触发操作的谱系区块 SHA256 id。
        target_id: 被冻结的目标区块 SHA256 id。
        scope: "local" | "downstream"。
        base: block-topology 根路径。

    Returns:
        写入的 event 区块 dict。
    """
    content = {"action": "freeze", "target": target_id, "scope": scope}
    refs = [source_id]
    block_id = compute_block_id("event", "cc", content, refs)
    relations = [
        make_relation(block_id, target_id, "freezes", order=1,
                      created_by=block_id),
    ]
    return write_block_with_relations(
        "event", "cc", content, refs, relations, base=base,
    )


def write_split(
    source_id: str,
    target_id: str,
    scope: str,
    base: Path = DEFAULT_BASE,
) -> dict:
    """分裂拓扑操作——写入 event 区块 + splits 关系。

    语义约束（编排者决断）：split 记录的是"此处发生了分裂"，不是"产生了
    两个新实体"。分支的独立演化是后续谱系事件——如果分支A被否定而分支B存活，
    这通过后续 event 区块引用 split event 来间接表达。

    Args:
        source_id: 触发操作的谱系区块 SHA256 id。
        target_id: 被分裂的目标区块 SHA256 id。
        scope: "local" | "downstream"。
        base: block-topology 根路径。

    Returns:
        写入的 event 区块 dict。
    """
    content = {"action": "split", "target": target_id, "scope": scope}
    refs = [source_id]
    block_id = compute_block_id("event", "cc", content, refs)
    relations = [
        make_relation(block_id, target_id, "splits", order=1,
                      created_by=block_id),
    ]
    return write_block_with_relations(
        "event", "cc", content, refs, relations, base=base,
    )


def write_sever(
    source_id: str,
    target_id: str,
    scope: str,
    base: Path = DEFAULT_BASE,
) -> dict:
    """切断连接拓扑操作——写入 event 区块 + severs 关系。

    Args:
        source_id: 触发操作的谱系区块 SHA256 id。
        target_id: 被切断连接的目标区块 SHA256 id。
        scope: "local" | "downstream"。
        base: block-topology 根路径。

    Returns:
        写入的 event 区块 dict。
    """
    content = {"action": "sever", "target": target_id, "scope": scope}
    refs = [source_id]
    block_id = compute_block_id("event", "cc", content, refs)
    relations = [
        make_relation(block_id, target_id, "severs", order=1,
                      created_by=block_id),
    ]
    return write_block_with_relations(
        "event", "cc", content, refs, relations, base=base,
    )


TOPO_WRITERS = {
    "freeze": write_freeze,
    "split": write_split,
    "sever": write_sever,
}


def execute_topo_effect(
    source_id: str,
    effect_type: str,
    target_id: str,
    scope: str,
    base: Path = DEFAULT_BASE,
) -> dict:
    """执行拓扑操作——调度到具体写入函数。

    Args:
        source_id: 触发操作的谱系区块 SHA256 id。
        effect_type: freeze | split | sever。
        target_id: 操作目标区块 SHA256 id。
        scope: local | downstream。
        base: block-topology 根路径。

    Returns:
        {"executed": bool, "log": list[str], "block": dict|None}
    """
    writer = TOPO_WRITERS.get(effect_type)
    if writer is None:
        return {
            "executed": False,
            "log": [f"错误: 未知拓扑操作类型 '{effect_type}'"],
            "block": None,
        }
    block = writer(source_id, target_id, scope, base)
    desc = TOPO_DESCRIPTIONS.get(effect_type, effect_type)
    return {
        "executed": True,
        "log": [f"{desc}：{target_id}（by {source_id}，scope={scope}）"],
        "block": block,
    }


# ── 承重点评分（从 block-topology relations 构建图） ──

def compute_load_bearing_score(
    block_id: str, base: Path = DEFAULT_BASE
) -> dict:
    """基于 block-topology relations 的承重点评分（176号下游推论3）。

    从 relations.jsonl 构建 depends_on 图，计算后代节点数 + 级联否定影响范围。

    正确分母 = 拓扑图实际参与者（出现在 depends_on/negates 关系的 from 或 to
    中的区块），不包含 rewrite/consensus/residue 等非谱系区块。

    Args:
        block_id: 目标区块 SHA256 id。
        base: block-topology 根路径。

    Returns:
        {"descendants": N, "cascade_impact": M, "score": N+M,
         "is_load_bearing": bool, "threshold": T}
    """
    all_rels = read_all_relations(base)

    # 构建 depends_on 图（children_of）和 negates 图
    # depends_on: from depends on to → to 是 parent, from 是 child
    children_of: dict[str, set[str]] = {}
    negates_targets: dict[str, set[str]] = {}
    topo_participants: set[str] = set()

    for rel in all_rels:
        rel_type = rel.get("relation")
        if rel_type == "depends_on" and rel.get("order") == 1:
            parent = rel["to"]
            child = rel["from"]
            children_of.setdefault(parent, set()).add(child)
            topo_participants.add(parent)
            topo_participants.add(child)
        elif rel_type == "negates" and rel.get("order") == 1:
            negates_targets.setdefault(rel["from"], set()).add(rel["to"])
            topo_participants.add(rel["from"])
            topo_participants.add(rel["to"])

    total_nodes = len(topo_participants)

    # BFS 后代计算
    descendants: set[str] = set()
    queue = list(children_of.get(block_id, set()))
    while queue:
        current = queue.pop()
        if current not in descendants:
            descendants.add(current)
            queue.extend(children_of.get(current, set()) - descendants)

    # 级联否定影响：被此节点否定的节点 + 它们的所有后代
    negated = negates_targets.get(block_id, set())
    cascade_nodes: set[str] = set()
    for target in negated:
        cascade_nodes.add(target)
        q = list(children_of.get(target, set()))
        while q:
            c = q.pop()
            if c not in cascade_nodes:
                cascade_nodes.add(c)
                q.extend(children_of.get(c, set()) - cascade_nodes)

    # 去重
    cascade_only = cascade_nodes - descendants

    desc_count = len(descendants)
    cascade_count = len(cascade_only)
    score = desc_count + cascade_count
    threshold = max(1, int(total_nodes * 0.10))

    return {
        "descendants": desc_count,
        "cascade_impact": cascade_count,
        "score": score,
        "is_load_bearing": score >= threshold,
        "threshold": threshold,
    }


# ── 谱系文件回写 ──

def _write_topo_executed_at(genealogy_path: Path, date_str: str) -> None:
    """在谱系文件的 YAML frontmatter 中追加 topo_executed_at 字段。

    在第二个 '---' 行之前插入新字段行。
    """
    content = genealogy_path.read_text(encoding="utf-8")
    lines = content.split("\n")

    # 找到 frontmatter 的结束位置（第二个 '---'）
    fm_end_idx = None
    dash_count = 0
    for i, line in enumerate(lines):
        if line.strip() == "---":
            dash_count += 1
            if dash_count == 2:
                fm_end_idx = i
                break

    if fm_end_idx is None:
        return

    # 在第二个 '---' 之前插入
    new_line = f'topo_executed_at: "{date_str}"'
    lines.insert(fm_end_idx, new_line)

    genealogy_path.write_text("\n".join(lines), encoding="utf-8")


# ── RTAS 循环入口 ──

def auto_execute_from_file(
    genealogy_path: str, base: str | Path = DEFAULT_BASE
) -> dict:
    """RTAS 循环调用入口：从谱系文件提取 topo_effect 并写入 block-topology。

    流程：
    1. 读取谱系文件，提取 frontmatter
    2. 检查 topo_executed_at 是否已存在（已执行则跳过）
    3. 提取 topo_effect，验证是结构化格式（type:target:scope）
    4. 无 topo_effect 或非结构化 → 返回 executed=False
    5. 通过 meta.json id_mapping 将旧谱系 id 解析为 SHA256
    6. 检查目标节点承重性（compute_load_bearing_score）
    7. 承重点 → 返回 executed=False, load_bearing=True, warning
    8. 非承重点 → 执行拓扑操作（写入 block-topology）
    9. 执行后在谱系文件 frontmatter 追加 topo_executed_at

    Returns:
        {"executed": bool, "effect": str|None, "log": list[str],
         "load_bearing": bool, "source_id": str}
    """
    gpath = Path(genealogy_path)
    bpath = Path(base)

    result: dict[str, Any] = {
        "executed": False,
        "effect": None,
        "log": [],
        "load_bearing": False,
        "source_id": "unknown",
    }

    # 1. 读取谱系文件
    if not gpath.exists():
        result["log"].append(f"谱系文件不存在: {genealogy_path}")
        return result

    content = gpath.read_text(encoding="utf-8")
    fields = extract_genealogy_fields(content)
    source_id = str(fields.get("id", "unknown"))
    result["source_id"] = source_id

    # 2. 检查是否已执行
    if fields.get("topo_executed_at"):
        result["log"].append(
            f"谱系 {source_id}: 已执行"
            f"（topo_executed_at={fields['topo_executed_at']}），跳过"
        )
        return result

    # 3. 提取结构化 topo_effect
    te_str = str(fields.get("topo_effect", "")).strip()
    parsed = parse_topo_effect(te_str)
    if parsed is None:
        result["log"].append(
            f"谱系 {source_id}: 无结构化 topo_effect（'{te_str}'），跳过"
        )
        return result

    effect_type, target_id, scope = parsed
    result["effect"] = f"{effect_type}:{target_id}:{scope}"

    # 4. 解析旧谱系 id 为 SHA256
    source_sha = resolve_genealogy_id(source_id, bpath)
    target_sha = resolve_genealogy_id(target_id, bpath)

    if source_sha is None:
        result["log"].append(
            f"谱系 {source_id}: source id '{source_id}' "
            f"不在 meta.json id_mapping 中"
        )
        return result

    if target_sha is None:
        result["log"].append(
            f"谱系 {source_id}: target id '{target_id}' "
            f"不在 meta.json id_mapping 中"
        )
        return result

    # 5. 检查承重性
    lb = compute_load_bearing_score(target_sha, bpath)
    result["load_bearing"] = lb["is_load_bearing"]

    if lb["is_load_bearing"]:
        result["log"].append(
            f"谱系 {source_id}: 目标 {target_id} 是承重点"
            f"（score={lb['score']}, threshold={lb['threshold']}），"
            f"拒绝自动执行 {effect_type}"
        )
        return result

    # 6. 执行拓扑操作（写入 block-topology）
    exec_result = execute_topo_effect(
        source_sha, effect_type, target_sha, scope, bpath,
    )
    result["log"].extend(exec_result["log"])

    if not exec_result["executed"]:
        return result

    # 7. 回写 topo_executed_at 到谱系文件 frontmatter
    today = datetime.date.today().isoformat()
    _write_topo_executed_at(gpath, today)
    result["log"].append(
        f"谱系文件 {source_id}: 写入 topo_executed_at: \"{today}\""
    )

    result["executed"] = True
    return result


def main() -> int:
    parser = argparse.ArgumentParser(
        description="矛盾→拓扑操作自动映射（147号谱系 + 178号-2升格）"
    )
    parser.add_argument(
        "--genealogy",
        help="谱系文件路径（从中提取 topo_effect / negation_form）",
    )
    parser.add_argument(
        "--effect",
        help="直接指定拓扑操作（格式: type:target:scope）",
    )
    parser.add_argument(
        "--target",
        help="补充目标节点 ID（当仅有 negation_form 时需要）",
    )
    parser.add_argument(
        "--scope",
        default="local",
        choices=sorted(VALID_SCOPES),
        help="操作范围（默认 local）",
    )
    parser.add_argument(
        "--base",
        default=str(DEFAULT_BASE),
        help=f"block-topology 根路径（默认 {DEFAULT_BASE}）",
    )
    parser.add_argument(
        "--dry-run",
        action="store_true",
        help="只报告将执行的操作，不写入 block-topology",
    )
    args = parser.parse_args()

    if not args.genealogy and not args.effect:
        parser.error("必须提供 --genealogy 或 --effect 之一")
        return 1

    # 确定拓扑操作参数
    source_id = "unknown"
    effect_type: str | None = None
    target_id: str | None = None
    scope: str = args.scope

    if args.effect:
        parsed = parse_topo_effect(args.effect)
        if parsed is None:
            print(
                f"错误: --effect 格式不合法: '{args.effect}'，"
                "需要 'type:target:scope'（type=freeze|split|sever）",
                file=sys.stderr,
            )
            return 1
        effect_type, target_id, scope = parsed

    if args.genealogy:
        genealogy_path = Path(args.genealogy)
        if not genealogy_path.exists():
            print(f"错误: 谱系文件不存在: {args.genealogy}", file=sys.stderr)
            return 1

        content = genealogy_path.read_text(encoding="utf-8")
        fields = extract_genealogy_fields(content)
        source_id = str(fields.get("id", "unknown"))

        # 优先使用 topo_effect（retrospective，141号结论1）
        te_str = fields.get("topo_effect", "")
        if te_str and not args.effect:
            parsed = parse_topo_effect(str(te_str))
            if parsed:
                effect_type, target_id, scope = parsed
                print(
                    f"[retrospective] 从 topo_effect 字段提取: "
                    f"{effect_type}:{target_id}:{scope}"
                )

        # 降级：从 negation_form 推导（prospective）
        if effect_type is None:
            nf = str(fields.get("negation_form", ""))
            inferred = infer_topo_type_from_negation_form(nf)
            if inferred:
                effect_type = inferred
                print(
                    f"[prospective] 从 negation_form '{nf}' 推导: "
                    f"{effect_type}（{TOPO_DESCRIPTIONS.get(effect_type, '')}）"
                )
                if args.target:
                    target_id = args.target
            else:
                print(
                    f"谱系 {source_id}: negation_form='{nf}' "
                    "不在映射表中（waiting/expansion/separation），"
                    "且无结构化 topo_effect——无拓扑操作可执行"
                )
                return 0

    if effect_type is None:
        print("错误: 无法确定拓扑操作类型", file=sys.stderr)
        return 1

    if target_id is None:
        print(
            f"谱系 {source_id}: 拓扑操作类型={effect_type}，"
            "但缺少目标节点 ID（--target 参数）——无法执行",
            file=sys.stderr,
        )
        return 1

    # 解析 id → SHA256（如果 base 中有 meta.json）
    base = Path(args.base)
    source_sha = resolve_genealogy_id(source_id, base)
    target_sha = resolve_genealogy_id(target_id, base)

    # 如果没有 id_mapping，使用原始 id（CLI 可能直接传 SHA256）
    if source_sha is None:
        source_sha = source_id
    if target_sha is None:
        target_sha = target_id

    # 报告
    desc = TOPO_DESCRIPTIONS.get(effect_type, effect_type)
    print(f"\n{'=' * 60}")
    print(f"  矛盾→拓扑操作（147号 + 178号-2）")
    print(f"{'=' * 60}")
    print(f"  来源谱系: {source_id}")
    print(f"  操作类型: {effect_type} — {desc}")
    print(f"  目标节点: {target_id}")
    print(f"  操作范围: {scope}")
    print(f"  模式: {'dry-run（不修改）' if args.dry_run else '执行'}")
    print(f"  block-topology: {base}")
    print(f"{'=' * 60}\n")

    if args.dry_run:
        print(f"[dry-run] 以上操作未实际执行")
        return 0

    # 执行
    exec_result = execute_topo_effect(
        source_sha, effect_type, target_sha, scope, base,
    )
    for entry in exec_result["log"]:
        print(f"  {entry}")

    if exec_result["executed"]:
        block = exec_result["block"]
        print(f"\nblock-topology 已写入: event block {block['id'][:16]}...")
    else:
        print(f"\n执行失败")
        return 1

    return 0


if __name__ == "__main__":
    sys.exit(main())
