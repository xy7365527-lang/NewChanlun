#!/usr/bin/env python
"""张力扬弃（Aufhebung）处理器。

当 tensions_with 边被解决时，不是简单标记 resolved，
而是产生 sublation 事件：{negated, preserved, elevated}。
elevated 自动注入纲目新条目（通过 gangmu_inject 管线）。

089号谱系定义扬弃三维度：否定 + 保留 + 提升。
本模块将此定义应用于张力处理流程。

用法:
  python scripts/tension_sublation.py                    # 扫描并产出 sublation 事件
  python scripts/tension_sublation.py --dry-run          # 只输出不写入
  python scripts/tension_sublation.py --apply            # 写入 tension-audit.yaml
"""
import json
import os
import sys
from datetime import datetime, timezone
from typing import Any

import yaml


def load_tension_audit(root: str) -> dict:
    """读取 tension-audit.yaml。"""
    path = os.path.join(root, ".chanlun", "tension-audit.yaml")
    if not os.path.isfile(path):
        return {"tensions": [], "summary": {}}
    with open(path, encoding="utf-8") as f:
        data = yaml.safe_load(f)
    return data if isinstance(data, dict) else {"tensions": [], "summary": {}}


def load_gangmu(root: str) -> dict:
    """读取 gangmu.yaml。"""
    path = os.path.join(root, ".chanlun", "gangmu.yaml")
    if not os.path.isfile(path):
        return {}
    with open(path, encoding="utf-8") as f:
        return yaml.safe_load(f) or {}


def _read_genealogy_frontmatter(root: str, gid: str) -> dict:
    """读取谱系条目的 YAML frontmatter。"""
    path = os.path.join(root, ".chanlun", "genealogy", "settled", f"{gid}*.md")
    import glob as globmod
    matches = globmod.glob(path)
    if not matches:
        return {}
    with open(matches[0], encoding="utf-8") as f:
        content = f.read(4000)
    if not content.startswith("---"):
        return {}
    end = content.find("---", 3)
    if end < 0:
        return {}
    try:
        return yaml.safe_load(content[3:end]) or {}
    except yaml.YAMLError:
        return {}


def derive_sublation_event(tension: dict, root: str) -> dict | None:
    """从一条 resolved 张力推导 sublation 事件。

    返回 sublation 事件 dict，或 None（非 resolved 张力不产生事件）。

    扬弃三维度（089号）：
      negated  — 原张力被否定（不再是活跃矛盾）
      preserved — 张力揭示的认识被保留（what was learned）
      elevated  — 张力的解决在更高层级开启了什么（跨层级传播）
    """
    classification = tension.get("classification", "")
    if classification != "resolved":
        return None

    from_id = tension.get("from", "")
    to_id = tension.get("to", "")
    resolved_by = tension.get("resolved_by", "")
    evidence = tension.get("evidence", "")
    description = tension.get("description", "")

    # 读取解决者谱系的 frontmatter 以提取层级信息
    resolver_fm = _read_genealogy_frontmatter(root, resolved_by) if resolved_by else {}
    resolver_title = resolver_fm.get("title", f"{resolved_by}号")

    # negated: 原张力本身
    negated = {
        "tension_edge": f"{from_id}↔{to_id}",
        "description": description,
        "reason": f"被 {resolved_by}号 否定——不再是活跃矛盾",
    }

    # preserved: 张力过程中产出的认识
    preserved = {
        "insight": evidence,
        "from_entry": from_id,
        "to_entry": to_id,
        "resolver": resolved_by,
    }

    # elevated: 解决张力在更高层级开启了什么
    # 从 resolver 的 downstream_implications 或 depends_on 推导
    elevated = _derive_elevation(tension, resolver_fm, root)

    return {
        "type": "sublation",
        "tension_from": from_id,
        "tension_to": to_id,
        "resolved_by": resolved_by,
        "negated": negated,
        "preserved": preserved,
        "elevated": elevated,
        "timestamp": datetime.now(timezone.utc).isoformat(),
    }


def _derive_elevation(tension: dict, resolver_fm: dict, root: str) -> dict:
    """推导张力解决后的提升维度。

    提升 = 张力的解决不是终止点，而是在更高层级开启/结算了什么。
    """
    resolved_by = tension.get("resolved_by", "")
    from_id = tension.get("from", "")
    to_id = tension.get("to", "")

    # 读取 resolver 谱系的完整内容，提取 downstream_implications
    downstream = _extract_downstream_implications(root, resolved_by)

    if downstream:
        return {
            "level": "cross_layer",
            "resolver": resolved_by,
            "resolver_title": resolver_fm.get("title", ""),
            "implications": downstream,
            "description": (
                f"{resolved_by}号解决 {from_id}↔{to_id} 张力后，"
                f"在更高层级产生了 {len(downstream)} 条推论"
            ),
        }

    # 无显式下游推论时，提升为结构性认识
    return {
        "level": "structural_insight",
        "resolver": resolved_by,
        "resolver_title": resolver_fm.get("title", ""),
        "implications": [],
        "description": (
            f"{resolved_by}号通过综合 {from_id} 和 {to_id} 的矛盾，"
            f"产生了结构性认识——张力不是错误而是互补维度"
        ),
    }


def _extract_downstream_implications(root: str, gid: str) -> list[str]:
    """从谱系条目提取 downstream_implications 列表。"""
    import glob as globmod
    pattern = os.path.join(
        root, ".chanlun", "genealogy", "settled", f"{gid}-*.md"
    )
    # 也尝试纯数字前缀
    pattern2 = os.path.join(
        root, ".chanlun", "genealogy", "settled", f"0{gid}-*.md"
    )
    matches = globmod.glob(pattern) + globmod.glob(pattern2)
    if not matches:
        return []

    with open(matches[0], encoding="utf-8") as f:
        content = f.read()

    # 提取 ## downstream_implications 或 ## 下游推论 段落
    implications = []
    in_section = False
    for line in content.split("\n"):
        lower = line.lower().strip()
        if lower.startswith("## downstream") or lower.startswith("## 下游推论"):
            in_section = True
            continue
        if in_section:
            if line.startswith("## "):
                break
            stripped = line.strip()
            if stripped.startswith(("- ", "* ", "1.", "2.", "3.", "4.", "5.")):
                # 去掉列表标记
                text = stripped.lstrip("-*0123456789. ").strip()
                if text:
                    implications.append(text)
    return implications


def scan_sublation_events(root: str) -> list[dict]:
    """扫描 tension-audit.yaml，对所有 resolved 张力产出 sublation 事件。"""
    audit = load_tension_audit(root)
    tensions = audit.get("tensions", [])
    events = []
    for tension in tensions:
        event = derive_sublation_event(tension, root)
        if event is not None:
            events.append(event)
    return events


def scan_historical_sublation_events(root: str) -> list[dict]:
    """扫描 historical 张力，产出 historical sublation 事件。

    historical 张力的扬弃形式不同：被框架否定（valid_until），
    张力随框架一起被扬弃。
    """
    audit = load_tension_audit(root)
    tensions = audit.get("tensions", [])
    events = []
    for tension in tensions:
        if tension.get("classification") != "historical":
            continue
        valid_until = tension.get("valid_until", "")
        from_id = tension.get("from", "")
        to_id = tension.get("to", "")
        description = tension.get("description", "")
        events.append({
            "type": "historical_sublation",
            "tension_from": from_id,
            "tension_to": to_id,
            "valid_until": valid_until,
            "negated": {
                "tension_edge": f"{from_id}↔{to_id}",
                "description": description,
                "reason": f"框架被 {valid_until}号 否定，张力随框架失效",
            },
            "preserved": {
                "insight": tension.get("evidence", ""),
                "historical_record": True,
            },
            "elevated": {
                "level": "paradigm_shift",
                "description": (
                    f"{valid_until}号范式转换扬弃了整个框架，"
                    f"张力 {from_id}↔{to_id} 不再有意义但保留为历史记录"
                ),
            },
            "timestamp": datetime.now(timezone.utc).isoformat(),
        })
    return events


def scan_ongoing_tensions(root: str) -> list[dict]:
    """扫描 ongoing 张力，返回未解决张力列表（这些不产生 sublation 事件）。"""
    audit = load_tension_audit(root)
    tensions = audit.get("tensions", [])
    return [t for t in tensions if t.get("classification") == "ongoing"]


def generate_elevated_gangmu_candidates(
    sublation_events: list[dict],
) -> list[dict]:
    """从 sublation 事件中提取可注入纲目的 elevated 条目候选。

    只提取 cross_layer 级别的 elevation（有具体下游推论的），
    structural_insight 级别不注入纲目（不携带工程行动）。
    """
    candidates = []
    for event in sublation_events:
        elevated = event.get("elevated", {})
        if elevated.get("level") != "cross_layer":
            continue
        if not elevated.get("implications"):
            continue
        candidates.append({
            "source_tension": f"{event['tension_from']}↔{event['tension_to']}",
            "resolved_by": event.get("resolved_by", ""),
            "resolver_title": elevated.get("resolver_title", ""),
            "implications": elevated["implications"],
            "description": elevated.get("description", ""),
        })
    return candidates


def update_tension_audit_with_sublation(
    root: str, sublation_events: list[dict]
) -> dict:
    """将 sublation 事件写回 tension-audit.yaml。

    为每条 resolved 张力添加 sublation 字段，记录扬弃事件。
    """
    audit_path = os.path.join(root, ".chanlun", "tension-audit.yaml")
    audit = load_tension_audit(root)
    tensions = audit.get("tensions", [])

    # 构建 sublation 索引：(from, to) -> event
    sublation_index: dict[tuple[str, str], dict] = {}
    for event in sublation_events:
        key = (event["tension_from"], event["tension_to"])
        sublation_index[key] = event

    updated_count = 0
    for tension in tensions:
        key = (tension.get("from", ""), tension.get("to", ""))
        if key in sublation_index:
            event = sublation_index[key]
            tension["sublation"] = {
                "negated": event["negated"]["reason"],
                "preserved": event["preserved"]["insight"],
                "elevated": event["elevated"]["description"],
                "timestamp": event["timestamp"],
            }
            updated_count += 1

    # 更新 summary
    summary = audit.get("summary", {})
    summary["sublation_events"] = len(sublation_events)
    audit["summary"] = summary

    # 写回
    with open(audit_path, "w", encoding="utf-8") as f:
        f.write("# Tension Audit — .chanlun/tension-audit.yaml\n")
        f.write(f"# Updated: {datetime.now().strftime('%Y-%m-%d')} "
                f"(sublation events added)\n")
        yaml.dump(audit, f, allow_unicode=True, default_flow_style=False,
                  sort_keys=False, width=120)

    return {"updated_count": updated_count, "total_events": len(sublation_events)}


def main():
    """CLI 入口。"""
    import argparse

    parser = argparse.ArgumentParser(
        description="张力扬弃（Aufhebung）处理器")
    parser.add_argument(
        "--dry-run", action="store_true",
        help="只输出 sublation 事件，不写入文件")
    parser.add_argument(
        "--apply", action="store_true",
        help="将 sublation 事件写回 tension-audit.yaml")
    parser.add_argument(
        "--root", default=None,
        help="项目根目录（默认当前目录）")
    args = parser.parse_args()

    root = args.root or os.getcwd()

    # 扫描 resolved 张力
    resolved_events = scan_sublation_events(root)

    # 扫描 historical 张力
    historical_events = scan_historical_sublation_events(root)

    # 扫描 ongoing 张力
    ongoing = scan_ongoing_tensions(root)

    # 生成纲目候选
    gangmu_candidates = generate_elevated_gangmu_candidates(resolved_events)

    output = {
        "sublation_events": {
            "resolved": resolved_events,
            "historical": historical_events,
        },
        "ongoing_tensions": [
            {
                "from": t.get("from", ""),
                "to": t.get("to", ""),
                "description": t.get("description", ""),
            }
            for t in ongoing
        ],
        "gangmu_candidates": gangmu_candidates,
        "summary": {
            "resolved_sublation_count": len(resolved_events),
            "historical_sublation_count": len(historical_events),
            "ongoing_count": len(ongoing),
            "gangmu_injection_candidates": len(gangmu_candidates),
        },
    }

    if args.apply and resolved_events:
        result = update_tension_audit_with_sublation(root, resolved_events)
        output["apply_result"] = result

    print(json.dumps(output, ensure_ascii=False, indent=2))


if __name__ == "__main__":
    main()
