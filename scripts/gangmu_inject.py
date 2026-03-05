#!/usr/bin/env python
"""纲目自动注入工具——将 ceremony_scan 检测到的 proposed_new_mu 注入 gangmu.yaml。

ceremony_scan.py 检测已结算谱系的下游推论中未被 gangmu 覆盖的条目，
输出 proposed_new_mu 列表。本脚本将这些提议自动注入到对应纲的 active 目中。

用法:
  # 从 ceremony_scan 的 JSON 输出管道传入
  python scripts/ceremony_scan.py | python scripts/gangmu_inject.py --stdin

  # 从文件读取
  python scripts/gangmu_inject.py --file scan_output.json

  # 直接传入 JSON
  python scripts/gangmu_inject.py --json '[{"source":"360号","field":"downstream_implications[1]","text":"...","coverage_status":"not_covered","suggested_gang":"shipen-bihuan"}]'

设计约束：
  - 纯函数设计：load/inject/save 分离
  - 幂等性：用 source+field 去重，已存在的条目不重复注入
  - 不可变：inject_action 返回新 dict，不原地修改
  - 原子写入：tempfile + rename（复用 gangmu_update.py 模式）
  - 只注入到 active 目的 next_actions——不创建新纲或新目
"""
import argparse
import copy
import json
import os
import re
import sys
import tempfile
from dataclasses import dataclass, field
from typing import Any

import yaml


@dataclass(frozen=True)
class InjectReport:
    """注入结果报告（不可变）。"""
    injected: tuple  # 新注入的 (gang_id, mu_id, target) 列表
    skipped: tuple   # 跳过的 (source, field, reason) 列表
    errors: tuple    # 错误的 (source, field, error) 列表

    def to_dict(self):
        return {
            "injected_count": len(self.injected),
            "skipped_count": len(self.skipped),
            "error_count": len(self.errors),
            "injected": [{"gang_id": g, "mu_id": m, "target": t}
                         for g, m, t in self.injected],
            "skipped": [{"source": s, "field": f, "reason": r}
                        for s, f, r in self.skipped],
            "errors": [{"source": s, "field": f, "error": e}
                       for s, f, e in self.errors],
        }


def load_gangmu(gangmu_path: str) -> dict:
    """读取 gangmu.yaml，返回 data dict。"""
    if not os.path.isfile(gangmu_path):
        raise FileNotFoundError(f"gangmu.yaml 不存在: {gangmu_path}")
    with open(gangmu_path, encoding="utf-8") as f:
        data = yaml.safe_load(f)
    if not isinstance(data, dict):
        raise ValueError("gangmu.yaml 格式无效：顶层不是 dict")
    return data


def save_gangmu(gangmu_path: str, data: dict) -> None:
    """原子写入 gangmu.yaml（tempfile + rename）。"""
    dir_name = os.path.dirname(gangmu_path)
    fd, tmp_path = tempfile.mkstemp(suffix=".yaml", dir=dir_name)
    try:
        with os.fdopen(fd, "w", encoding="utf-8") as f:
            f.write("# .chanlun/gangmu.yaml\n")
            f.write("# 纲目——总方针的展开结构\n")
            f.write("#\n")
            f.write("# 由 gangmu_update.py 自动更新\n\n")
            yaml.dump(data, f, allow_unicode=True, default_flow_style=False,
                      sort_keys=False, width=120)
        os.replace(tmp_path, gangmu_path)
    except Exception:
        if os.path.exists(tmp_path):
            os.unlink(tmp_path)
        raise


def _dedup_key(source: str, field_name: str) -> str:
    """从 proposal 的 source+field 生成去重键。"""
    return f"{source}:{field_name}"


def _make_target(source: str, field_name: str) -> str:
    """从 proposal 生成 action target 标识。

    格式: genealogy-{编号}-{推论索引}
    例: source="360号" field="downstream_implications[2]" → "genealogy-360-di2"
    """
    num_match = re.match(r"(\d+)", source)
    num = num_match.group(1) if num_match else source.replace("号", "")

    idx_match = re.search(r"\[(\d+)\]", field_name)
    idx = idx_match.group(1) if idx_match else "0"

    return f"genealogy-{num}-di{idx}"


def find_existing_sources(gangmu: dict) -> set:
    """收集 gangmu 中所有 action 的 injected_from 标记，用于去重。

    同时收集 target 名称匹配 genealogy-{N}-di{M} 模式的条目。
    """
    sources = set()
    gangs = gangmu.get("gang", [])
    if not isinstance(gangs, list):
        return sources
    for gang in gangs:
        if not isinstance(gang, dict):
            continue
        for mu in gang.get("mu", []):
            if not isinstance(mu, dict):
                continue
            for action in mu.get("next_actions", []):
                if not isinstance(action, dict):
                    continue
                # 方式1：显式 injected_from 标记
                injected_from = action.get("injected_from", "")
                if injected_from:
                    sources.add(injected_from)
                # 方式2：target 名称匹配
                target = action.get("target", "")
                if target:
                    sources.add(target)
    return sources


def _find_active_mu_in_gang(gangmu: dict, gang_id: str) -> tuple:
    """在指定纲中找到第一个 active 目。返回 (gang_dict, mu_dict) 或 (None, None)。"""
    gangs = gangmu.get("gang", [])
    if not isinstance(gangs, list):
        return None, None
    for gang in gangs:
        if not isinstance(gang, dict):
            continue
        if gang.get("id") != gang_id:
            continue
        for mu in gang.get("mu", []):
            if not isinstance(mu, dict):
                continue
            if mu.get("status") == "active":
                return gang, mu
    return None, None


def _find_any_active_mu(gangmu: dict) -> tuple:
    """在任意纲中找到第一个 active 目（fallback）。返回 (gang_dict, mu_dict) 或 (None, None)。"""
    gangs = gangmu.get("gang", [])
    if not isinstance(gangs, list):
        return None, None
    for gang in gangs:
        if not isinstance(gang, dict):
            continue
        for mu in gang.get("mu", []):
            if not isinstance(mu, dict):
                continue
            if mu.get("status") == "active":
                return gang, mu
    return None, None


def inject_action(gangmu: dict, proposal: dict) -> tuple:
    """将单条 proposal 注入 gangmu，返回 (new_gangmu, result)。

    不修改输入 gangmu。result 是 ("injected", gang_id, mu_id, target)
    或 ("skipped", source, field, reason) 或 ("error", source, field, msg)。

    Parameters
    ----------
    gangmu : 纲目数据（不可变——函数内部深拷贝）
    proposal : ceremony_scan 输出的单条 proposed_new_mu

    Returns
    -------
    (new_gangmu_dict, result_tuple)
    """
    source = proposal.get("source", "")
    field_name = proposal.get("field", "")
    text = proposal.get("text", "")
    coverage = proposal.get("coverage_status", "")
    suggested_gang = proposal.get("suggested_gang", "")

    if coverage != "not_covered":
        return gangmu, ("skipped", source, field_name, f"coverage_status={coverage}")

    if not source or not field_name:
        return gangmu, ("error", source, field_name, "缺少 source 或 field")

    target = _make_target(source, field_name)
    dedup = _dedup_key(source, field_name)

    # 检查去重
    existing = find_existing_sources(gangmu)
    if dedup in existing or target in existing:
        return gangmu, ("skipped", source, field_name, "已存在")

    # 深拷贝以保持不可变
    new_gangmu = copy.deepcopy(gangmu)

    # 找到目标 mu
    gang, mu = _find_active_mu_in_gang(new_gangmu, suggested_gang)
    if gang is None or mu is None:
        # fallback: 任意 active 目
        gang, mu = _find_any_active_mu(new_gangmu)
    if gang is None or mu is None:
        return gangmu, ("error", source, field_name, "无 active 目可注入")

    gang_id = gang.get("id", "")
    mu_id = mu.get("id", "")

    # 构造 action 条目
    action = {
        "type": "engineering",
        "target": target,
        "description": f"{text}（来源：{source} {field_name}）",
        "blocked_by": None,
        "injected_from": dedup,
        "completion_check": {
            "type": "genealogy_settled",
            "keyword": target,
        },
    }

    # 追加到 next_actions
    if not isinstance(mu.get("next_actions"), list):
        mu["next_actions"] = []
    mu["next_actions"].append(action)

    return new_gangmu, ("injected", gang_id, mu_id, target)


def inject_all(gangmu: dict, proposals: list) -> tuple:
    """批量注入所有 proposals，返回 (new_gangmu, InjectReport)。

    按顺序处理每条 proposal，每次在上一次的结果上继续注入。
    """
    current = gangmu
    injected = []
    skipped = []
    errors = []

    for proposal in proposals:
        new_gangmu, result = inject_action(current, proposal)
        kind = result[0]
        if kind == "injected":
            current = new_gangmu
            injected.append(result[1:])  # (gang_id, mu_id, target)
        elif kind == "skipped":
            skipped.append(result[1:])   # (source, field, reason)
        elif kind == "error":
            errors.append(result[1:])    # (source, field, msg)

    report = InjectReport(
        injected=tuple(injected),
        skipped=tuple(skipped),
        errors=tuple(errors),
    )
    return current, report


def main():
    parser = argparse.ArgumentParser(
        description="纲目自动注入——将 proposed_new_mu 注入 gangmu.yaml")
    parser.add_argument(
        "--root", default=None,
        help="项目根目录（默认当前目录）")

    source_group = parser.add_mutually_exclusive_group(required=True)
    source_group.add_argument(
        "--stdin", action="store_true",
        help="从 stdin 读取 ceremony_scan 的完整 JSON 输出")
    source_group.add_argument(
        "--file", type=str,
        help="从文件读取 ceremony_scan 的完整 JSON 输出")
    source_group.add_argument(
        "--json", type=str,
        help="直接传入 proposed_new_mu JSON 数组")

    parser.add_argument(
        "--dry-run", action="store_true",
        help="只输出报告，不写入文件")

    args = parser.parse_args()
    root = args.root or os.getcwd()
    gangmu_path = os.path.join(root, ".chanlun", "gangmu.yaml")

    # 解析输入
    if args.stdin:
        raw = sys.stdin.read()
    elif args.file:
        with open(args.file, encoding="utf-8") as f:
            raw = f.read()
    else:
        raw = args.json

    parsed = json.loads(raw)

    # 支持两种格式：完整 scan 输出（含 proposed_new_mu 字段）或直接的数组
    if isinstance(parsed, dict):
        proposals = parsed.get("proposed_new_mu", [])
    elif isinstance(parsed, list):
        proposals = parsed
    else:
        print(json.dumps({"error": "输入格式无效：需要 dict 或 list"},
                         ensure_ascii=False))
        sys.exit(1)

    if not proposals:
        print(json.dumps({"status": "no_proposals", "message": "无 proposed_new_mu"},
                         ensure_ascii=False))
        return

    # 加载 → 注入 → 保存
    gangmu = load_gangmu(gangmu_path)
    new_gangmu, report = inject_all(gangmu, proposals)

    if not args.dry_run and report.injected:
        save_gangmu(gangmu_path, new_gangmu)

    result = report.to_dict()
    result["dry_run"] = args.dry_run
    print(json.dumps(result, ensure_ascii=False, indent=2))


if __name__ == "__main__":
    main()
