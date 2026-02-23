#!/usr/bin/env python3
"""矛盾→拓扑操作自动映射（147号谱系下游推论1）。

读取谱系文件，根据否定类型执行对应的拓扑操作。

否定形式→拓扑操作映射（040号 + 147号）：
  - waiting（等待型）  → freeze（冻结路径）
  - expansion（扩张型）→ split（分裂节点）
  - separation（分离型）→ sever（切断连接）

输入来源优先级（141号结论1：retrospective > prospective）：
  1. topo_effect 字段（结构化格式 type:target:scope）— 直接执行
  2. negation_form 字段 — 推导拓扑操作类型（需要 target/scope 参数补充）

用法:
  # 从谱系文件读取 topo_effect 并执行
  python scripts/topology_operator.py --genealogy .chanlun/genealogy/settled/147-xxx.md

  # 直接指定拓扑操作
  python scripts/topology_operator.py --effect freeze:062:downstream

  # dry-run 模式（只报告，不修改 dag.yaml）
  python scripts/topology_operator.py --genealogy .chanlun/genealogy/settled/147-xxx.md --dry-run

  # 从 negation_form 推导（需要补充 target 和 scope）
  python scripts/topology_operator.py --genealogy .chanlun/genealogy/settled/090-xxx.md \\
    --target 086 --scope local
"""
from __future__ import annotations

import argparse
import copy
import re
import sys
from pathlib import Path
from typing import Any

import yaml


# ── 否定形式→拓扑操作映射表（040号 + 147号） ──

NEGATION_FORM_TO_TOPO: dict[str, str] = {
    "waiting": "freeze",
    "expansion": "split",
    "separation": "sever",
}

TOPO_DESCRIPTIONS: dict[str, str] = {
    "freeze": "冻结路径（等待型否定）——冻结目标节点及其下游依赖，直到后续回溯规定解冻",
    "split": "分裂节点（扩张型否定）——目标节点分裂为两个：原规定 + 违反记录",
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


def load_dag(dag_path: Path) -> dict:
    """加载 dag.yaml。"""
    with open(dag_path, encoding="utf-8") as f:
        return yaml.safe_load(f)


def save_dag(dag: dict, dag_path: Path) -> None:
    """保存 dag.yaml。"""
    with open(dag_path, "w", encoding="utf-8") as f:
        yaml.dump(
            dag,
            f,
            allow_unicode=True,
            default_flow_style=False,
            sort_keys=False,
        )


def apply_freeze(
    dag: dict, source_id: str, target_id: str, scope: str
) -> list[str]:
    """冻结路径操作（等待型否定）。

    Returns:
        操作日志列表。
    """
    log: list[str] = []
    nodes = dag.get("nodes", [])
    edges = dag.get("edges", {})

    # 冻结目标节点
    found = False
    for n in nodes:
        if str(n["id"]) == target_id:
            n["frozen"] = True
            n["frozen_by"] = source_id
            log.append(f"冻结节点 {target_id}（by {source_id}）")
            found = True
            break

    if not found:
        log.append(f"警告: 目标节点 {target_id} 不存在于 dag.yaml")
        return log

    # 如果 scope=downstream，冻结所有以 target 为起点的 depends_on 边
    if scope == "downstream":
        for e in edges.get("depends_on", []):
            if str(e.get("to", "")) == target_id:
                e["frozen_by"] = source_id
                log.append(
                    f"冻结边: {e.get('from')} -> {target_id}（下游冻结）"
                )

    return log


def apply_split(
    dag: dict, source_id: str, target_id: str, _scope: str
) -> list[str]:
    """分裂节点操作（扩张型否定）。

    Returns:
        操作日志列表。
    """
    log: list[str] = []
    nodes = dag.get("nodes", [])

    original = None
    for n in nodes:
        if str(n["id"]) == target_id:
            original = n
            break

    if original is None:
        log.append(f"警告: 目标节点 {target_id} 不存在于 dag.yaml")
        return log

    node_a = {
        **copy.deepcopy(original),
        "id": f"{target_id}-a",
        "split_from": target_id,
        "split_by": source_id,
    }
    node_b = {
        **copy.deepcopy(original),
        "id": f"{target_id}-b",
        "split_from": target_id,
        "split_by": source_id,
    }
    original["split_into"] = [f"{target_id}-a", f"{target_id}-b"]
    original["split_by"] = source_id

    nodes.append(node_a)
    nodes.append(node_b)
    log.append(
        f"分裂节点 {target_id} → {target_id}-a + {target_id}-b（by {source_id}）"
    )

    return log


def apply_sever(
    dag: dict, source_id: str, target_id: str, _scope: str
) -> list[str]:
    """切断连接操作（分离型否定）。

    Returns:
        操作日志列表。
    """
    log: list[str] = []
    edges = dag.get("edges", {})

    # 有向边
    for edge_type in ("depends_on", "negates"):
        for e in edges.get(edge_type, []):
            if (
                str(e.get("from", "")) == target_id
                or str(e.get("to", "")) == target_id
            ):
                e["severed_by"] = source_id
                log.append(
                    f"切断 {edge_type} 边（涉及 {target_id}，by {source_id}）"
                )

    # 无向边
    for edge_type in ("related", "tensions_with"):
        for e in edges.get(edge_type, []):
            pair = e.get("between", [])
            if target_id in [str(p) for p in pair]:
                e["severed_by"] = source_id
                log.append(
                    f"切断 {edge_type} 边（涉及 {target_id}，by {source_id}）"
                )

    if not log:
        log.append(f"无涉及 {target_id} 的边需要切断")

    return log


TOPO_OPERATORS = {
    "freeze": apply_freeze,
    "split": apply_split,
    "sever": apply_sever,
}


def execute_topo_effect(
    dag: dict,
    source_id: str,
    effect_type: str,
    target_id: str,
    scope: str,
) -> list[str]:
    """执行拓扑操作。

    Args:
        dag: dag.yaml 数据。
        source_id: 触发操作的谱系 ID。
        effect_type: freeze | split | sever。
        target_id: 操作目标节点 ID。
        scope: local | downstream。

    Returns:
        操作日志列表。
    """
    operator = TOPO_OPERATORS.get(effect_type)
    if operator is None:
        return [f"错误: 未知拓扑操作类型 '{effect_type}'"]
    return operator(dag, source_id, target_id, scope)


def main() -> int:
    parser = argparse.ArgumentParser(
        description="矛盾→拓扑操作自动映射（147号谱系）"
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
        "--dag",
        default=".chanlun/genealogy/dag.yaml",
        help="dag.yaml 路径（默认 .chanlun/genealogy/dag.yaml）",
    )
    parser.add_argument(
        "--dry-run",
        action="store_true",
        help="只报告将执行的操作，不修改 dag.yaml",
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

    # 加载 DAG
    dag_path = Path(args.dag)
    if not dag_path.exists():
        print(f"错误: dag.yaml 不存在: {args.dag}", file=sys.stderr)
        return 1

    dag = load_dag(dag_path)

    # 报告
    desc = TOPO_DESCRIPTIONS.get(effect_type, effect_type)
    print(f"\n{'=' * 60}")
    print(f"  矛盾→拓扑操作（147号）")
    print(f"{'=' * 60}")
    print(f"  来源谱系: {source_id}")
    print(f"  操作类型: {effect_type} — {desc}")
    print(f"  目标节点: {target_id}")
    print(f"  操作范围: {scope}")
    print(f"  模式: {'dry-run（不修改）' if args.dry_run else '执行'}")
    print(f"{'=' * 60}\n")

    # 执行
    log = execute_topo_effect(dag, source_id, effect_type, target_id, scope)
    for entry in log:
        print(f"  {entry}")

    if not args.dry_run:
        save_dag(dag, dag_path)
        print(f"\ndag.yaml 已更新: {len(dag.get('nodes', []))} 节点")
    else:
        print(f"\n[dry-run] 以上操作未实际执行")

    return 0


if __name__ == "__main__":
    sys.exit(main())
