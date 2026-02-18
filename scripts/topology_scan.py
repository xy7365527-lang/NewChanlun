#!/usr/bin/env python3
"""拓扑扫描：从分布式文件引用中提取知识图谱并分析连通性。

图已经分布式地存在于所有文件的交叉引用中：
- 定义文件 (.chanlun/definitions/*.md) 引用 spec、src
- 谱系条目 (.chanlun/genealogy/**/*.md) 引用定义、其他谱系
- Skill 文件 (.claude/skills/**/SKILL.md) 有 genealogy_source frontmatter
- Spec 文件 (docs/spec/*.md) 引用其他 spec、定义

本工具不创建图，只读取已存在的图并分析连通性。

用法：
    python scripts/topology_scan.py
"""

from __future__ import annotations

import re
from collections import defaultdict
from pathlib import Path


ROOT = Path(__file__).resolve().parent.parent

# ── 节点类型 ──

NODE_TYPES = {
    "definition": ".chanlun/definitions",
    "genealogy_settled": ".chanlun/genealogy/settled",
    "genealogy_pending": ".chanlun/genealogy/pending",
    "skill": ".claude/skills",
    "spec": "docs/spec",
    "source": "src/newchan",
}


def _scan_md_files(rel_dir: str, recursive: bool = False) -> list[Path]:
    """扫描目录下的 .md 文件。"""
    d = ROOT / rel_dir
    if not d.exists():
        return []
    pattern = "**/*.md" if recursive else "*.md"
    return sorted(d.glob(pattern))


def _node_id(path: Path) -> str:
    """文件路径 → 节点 ID。"""
    return str(path.relative_to(ROOT))


def _extract_references(content: str, source_id: str) -> list[tuple[str, str, str]]:
    """从文件内容中提取对其他节点的引用。

    返回 (source, target, edge_type) 三元组列表。
    """
    edges = []

    # 1. YAML frontmatter: genealogy_source
    m = re.search(r"genealogy_source:\s*[\"']?(\d+\w*)[\"']?", content)
    if m:
        gen_id = m.group(1)
        # 尝试匹配谱系文件
        target = _find_genealogy_file(gen_id)
        if target:
            edges.append((source_id, target, "genealogy_source"))
        else:
            edges.append((source_id, f"MISSING:genealogy/{gen_id}", "genealogy_source_broken"))

    # 2. 谱系条目中的引用: "前置: 022-xxx" 或 "关联: xxx"
    for pattern, edge_type in [
        (r"前置:\s*(.+)", "depends_on"),
        (r"关联:\s*(.+)", "related_to"),
    ]:
        m = re.search(pattern, content)
        if m:
            refs = m.group(1)
            for ref in re.split(r"[,，]\s*", refs):
                ref = ref.strip()
                if not ref or ref == "无":
                    continue
                target = _resolve_reference(ref)
                edges.append((source_id, target, edge_type))

    # 3. 显式文件路径引用: `docs/spec/xxx.md`, `src/newchan/xxx.py`
    for fpath in re.findall(r"`((?:docs/spec|src/newchan|\.chanlun/\w+)/[\w./\-]+\.\w+)`", content):
        # 截断 :: 后的函数名，只取文件路径
        target = fpath.split("::")[0].strip()
        if (ROOT / target).exists():
            edges.append((source_id, target, "references"))
        else:
            edges.append((source_id, f"MISSING:{target}", "reference_broken"))

    # 4. 谱系号引用: NNN号谱系
    for gen_num in re.findall(r"(\d{3})号谱系", content):
        target = _find_genealogy_file(gen_num)
        if target:
            edges.append((source_id, target, "cites_genealogy"))

    # 5. 定义文件引用: xxx.md (在 .chanlun/definitions/ 中)
    for def_ref in re.findall(r"(?:定义文件|definitions/)([\w]+)\.md", content):
        target = f".chanlun/definitions/{def_ref}.md"
        if (ROOT / target).exists():
            edges.append((source_id, target, "references_definition"))

    return edges


def _find_genealogy_file(gen_id: str) -> str | None:
    """根据谱系号找到对应文件。"""
    gen_id = gen_id.strip()
    for subdir in ["settled", "pending"]:
        d = ROOT / ".chanlun" / "genealogy" / subdir
        if not d.exists():
            continue
        for f in d.glob("*.md"):
            if f.name.startswith(gen_id):
                return str(f.relative_to(ROOT))
    return None


def _resolve_reference(ref: str) -> str:
    """解析引用文本为节点 ID。"""
    ref = ref.strip()

    # 谱系编号: 022-xxx
    m = re.match(r"(\d{3})-[\w-]+", ref)
    if m:
        target = _find_genealogy_file(m.group(1))
        return target or f"MISSING:genealogy/{ref}"

    # 定义名: bi.md, dengjia.md
    if ref.endswith(".md"):
        for d in [".chanlun/definitions", "docs/spec"]:
            target = f"{d}/{ref}"
            if (ROOT / target).exists():
                return target
        return f"MISSING:{ref}"

    # 文件路径
    if (ROOT / ref).exists():
        return ref

    return f"UNRESOLVED:{ref}"


def build_graph() -> tuple[set[str], list[tuple[str, str, str]]]:
    """构建知识图谱。返回 (节点集, 边列表)。"""
    nodes: set[str] = set()
    edges: list[tuple[str, str, str]] = []

    # 扫描所有节点来源
    sources = [
        (".chanlun/definitions", False),
        (".chanlun/genealogy/settled", False),
        (".chanlun/genealogy/pending", False),
        ("docs/spec", False),
        (".claude/skills", True),
    ]

    for rel_dir, recursive in sources:
        for path in _scan_md_files(rel_dir, recursive):
            node_id = _node_id(path)
            nodes.add(node_id)
            content = path.read_text(encoding="utf-8")
            file_edges = _extract_references(content, node_id)
            edges.extend(file_edges)
            # 被引用的目标也是节点
            for _, target, _ in file_edges:
                nodes.add(target)

    return nodes, edges


def analyze_connectivity(
    nodes: set[str], edges: list[tuple[str, str, str]]
) -> dict:
    """分析图的连通性。"""
    out_edges: dict[str, list[tuple[str, str]]] = defaultdict(list)
    in_edges: dict[str, list[tuple[str, str]]] = defaultdict(list)

    for src, tgt, etype in edges:
        out_edges[src].append((tgt, etype))
        in_edges[tgt].append((src, etype))

    # 悬空节点: 有入边无出边（被引用但不引用别人——可能是叶子节点）
    # 孤立节点: 无入边无出边（完全断连）
    # 缺失节点: 以 MISSING: 开头的虚拟节点
    real_nodes = {n for n in nodes if not n.startswith(("MISSING:", "UNRESOLVED:"))}
    missing_nodes = {n for n in nodes if n.startswith("MISSING:")}
    unresolved_nodes = {n for n in nodes if n.startswith("UNRESOLVED:")}

    isolated = {n for n in real_nodes if n not in out_edges and n not in in_edges}
    leaf_only = {n for n in real_nodes if n not in out_edges and n in in_edges}
    root_only = {n for n in real_nodes if n in out_edges and n not in in_edges}

    return {
        "total_nodes": len(real_nodes),
        "total_edges": len(edges),
        "missing_references": sorted(missing_nodes),
        "unresolved_references": sorted(unresolved_nodes),
        "isolated_nodes": sorted(isolated),
        "root_only": sorted(root_only),  # 只有出边（引用别人但没人引用）
        "leaf_only": sorted(leaf_only),   # 只有入边（被引用但不引用别人）
    }


def print_report(nodes: set[str], edges: list[tuple[str, str, str]], analysis: dict):
    """输出连通性报告。"""
    print("=" * 60)
    print("  知识拓扑连通性分析")
    print("=" * 60)

    print(f"\n节点: {analysis['total_nodes']}  边: {analysis['total_edges']}")

    # 按类型统计节点
    type_counts: dict[str, int] = defaultdict(int)
    for n in nodes:
        if n.startswith("MISSING:") or n.startswith("UNRESOLVED:"):
            continue
        if n.startswith(".chanlun/definitions/"):
            type_counts["定义"] += 1
        elif n.startswith(".chanlun/genealogy/settled/"):
            type_counts["谱系(已结算)"] += 1
        elif n.startswith(".chanlun/genealogy/pending/"):
            type_counts["谱系(生成态)"] += 1
        elif n.startswith("docs/spec/"):
            type_counts["规范"] += 1
        elif n.startswith(".claude/skills/"):
            type_counts["Skill"] += 1
        elif n.startswith("src/"):
            type_counts["代码"] += 1
        else:
            type_counts["其他"] += 1

    print("\n节点分布:")
    for t, c in sorted(type_counts.items()):
        print(f"  {t}: {c}")

    # 边类型统计
    edge_counts: dict[str, int] = defaultdict(int)
    for _, _, etype in edges:
        edge_counts[etype] += 1
    print("\n边类型:")
    for t, c in sorted(edge_counts.items()):
        print(f"  {t}: {c}")

    # 问题报告
    if analysis["missing_references"]:
        print(f"\n⚠ 断裂引用 ({len(analysis['missing_references'])}):")
        for n in analysis["missing_references"]:
            print(f"  {n}")

    if analysis["unresolved_references"]:
        print(f"\n⚠ 未解析引用 ({len(analysis['unresolved_references'])}):")
        for n in analysis["unresolved_references"]:
            print(f"  {n}")

    if analysis["isolated_nodes"]:
        print(f"\n🔴 孤立节点 ({len(analysis['isolated_nodes'])}):")
        print("  （无入边无出边——完全断连，可能是未结晶的知识）")
        for n in analysis["isolated_nodes"]:
            print(f"  {n}")

    if analysis["root_only"]:
        print(f"\n🟡 根节点 ({len(analysis['root_only'])}):")
        print("  （引用别人但没人引用——可能是入口点或悬空引用源）")
        for n in analysis["root_only"]:
            print(f"  {n}")

    print(f"\n{'=' * 60}")
    if not analysis["missing_references"] and not analysis["isolated_nodes"]:
        print("✅ 无断裂引用，无孤立节点")
    else:
        total_issues = len(analysis["missing_references"]) + len(analysis["isolated_nodes"])
        print(f"📌 {total_issues} 个问题需要关注")


def main():
    nodes, edges = build_graph()
    analysis = analyze_connectivity(nodes, edges)
    print_report(nodes, edges, analysis)


if __name__ == "__main__":
    main()
