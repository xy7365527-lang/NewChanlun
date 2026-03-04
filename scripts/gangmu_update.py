#!/usr/bin/env python
"""纲目动态更新工具——gangmu.yaml 的原子 read-modify-write 操作。

解决持存断裂根因：工位完成后 gangmu.yaml 的 next_actions 静态不变，
导致 ceremony_scan 输出工位为空 → 不动点终止 → 持存断裂。

三种操作：
  --complete "mu_id:action_target"          标记 action 完成（添加 completed_at 字段）
  --append "mu_id" --action '{...}'         追加新 action 到目的 next_actions
  --transition "mu_id" --to "closed" --reason "..."  目状态转换

设计约束（gangmu.yaml 注释）：
  - ceremony_scan.py 提议转换，本工具执行转换
  - 写操作是原子的（读→改→写，tempfile+rename）
  - 不创建新的纲或目——纲目结构由编排者定义
"""
import argparse
import json
import os
import sys
import tempfile
from datetime import date

import yaml


VALID_STATUSES = {"active", "blocked", "closed"}


def _load_gangmu(path):
    """读取 gangmu.yaml，返回 (data_dict, raw_path)。"""
    if not os.path.isfile(path):
        raise FileNotFoundError(f"gangmu.yaml 不存在: {path}")
    with open(path, encoding="utf-8") as f:
        data = yaml.safe_load(f)
    if not isinstance(data, dict):
        raise ValueError("gangmu.yaml 格式无效：顶层不是 dict")
    return data


def _write_gangmu(path, data):
    """原子写入 gangmu.yaml（tempfile + rename）。"""
    dir_name = os.path.dirname(path)
    fd, tmp_path = tempfile.mkstemp(suffix=".yaml", dir=dir_name)
    try:
        with os.fdopen(fd, "w", encoding="utf-8") as f:
            # 保留注释头
            f.write("# .chanlun/gangmu.yaml\n")
            f.write("# 纲目——总方针的展开结构\n")
            f.write("#\n")
            f.write("# 由 gangmu_update.py 自动更新\n\n")
            yaml.dump(data, f, allow_unicode=True, default_flow_style=False,
                      sort_keys=False, width=120)
        os.replace(tmp_path, path)
    except Exception:
        if os.path.exists(tmp_path):
            os.unlink(tmp_path)
        raise


def _find_mu(data, mu_id):
    """在 gangmu.yaml 中查找指定 mu_id 的 mu 节点。

    返回 (gang_dict, mu_dict) 或 raises ValueError。
    """
    gangs = data.get("gang", [])
    if not isinstance(gangs, list):
        raise ValueError("gangmu.yaml 中 gang 不是 list")
    for gang in gangs:
        if not isinstance(gang, dict):
            continue
        mus = gang.get("mu", [])
        if not isinstance(mus, list):
            continue
        for mu in mus:
            if not isinstance(mu, dict):
                continue
            if mu.get("id") == mu_id:
                return gang, mu
    raise ValueError(f"未找到目: {mu_id}")


def complete_action(gangmu_path, mu_id, action_target):
    """标记目的某个 action 为已完成（添加 completed_at 字段）。

    不删除 action（保留可追溯性），但添加 completed_at 使
    ceremony_scan._check_completion 之外的消费者也能知道完成状态。
    """
    data = _load_gangmu(gangmu_path)
    _gang, mu = _find_mu(data, mu_id)

    next_actions = mu.get("next_actions", [])
    if not isinstance(next_actions, list):
        raise ValueError(f"目 {mu_id} 的 next_actions 不是 list")

    found = False
    for action in next_actions:
        if not isinstance(action, dict):
            continue
        if action.get("target") == action_target:
            if action.get("completed_at"):
                return {"status": "already_completed", "mu_id": mu_id,
                        "target": action_target}
            action["completed_at"] = str(date.today())
            found = True
            break

    if not found:
        raise ValueError(
            f"目 {mu_id} 中未找到 target={action_target} 的 action")

    _write_gangmu(gangmu_path, data)
    return {"status": "completed", "mu_id": mu_id, "target": action_target,
            "completed_at": str(date.today())}


def append_action(gangmu_path, mu_id, action_dict):
    """向目的 next_actions 追加新 action。"""
    data = _load_gangmu(gangmu_path)
    _gang, mu = _find_mu(data, mu_id)

    if not isinstance(action_dict, dict):
        raise ValueError("action 必须是 dict")

    required_keys = {"target", "completion_check"}
    missing = required_keys - set(action_dict.keys())
    if missing:
        raise ValueError(f"action 缺少必需字段: {missing}")

    # 设置默认值
    action_dict.setdefault("type", "engineering")
    action_dict.setdefault("blocked_by", None)
    action_dict.setdefault("description", "")

    next_actions = mu.get("next_actions")
    if not isinstance(next_actions, list):
        mu["next_actions"] = []
        next_actions = mu["next_actions"]

    # 检查重复
    for existing in next_actions:
        if isinstance(existing, dict) and existing.get("target") == action_dict["target"]:
            raise ValueError(
                f"目 {mu_id} 中已存在 target={action_dict['target']} 的 action")

    next_actions.append(action_dict)
    _write_gangmu(gangmu_path, data)
    return {"status": "appended", "mu_id": mu_id,
            "target": action_dict["target"]}


def transition_mu(gangmu_path, mu_id, to_status, reason):
    """执行目的状态转换。"""
    data = _load_gangmu(gangmu_path)
    _gang, mu = _find_mu(data, mu_id)

    if to_status not in VALID_STATUSES:
        raise ValueError(
            f"无效的目标状态: {to_status}，合法值: {VALID_STATUSES}")

    from_status = mu.get("status", "")
    if from_status == to_status:
        return {"status": "no_change", "mu_id": mu_id,
                "current": from_status}

    mu["status"] = to_status
    if to_status == "closed":
        mu["closed_at"] = str(date.today())
        mu["closed_reason"] = reason
    elif to_status == "blocked":
        mu["blocked_by"] = reason

    _write_gangmu(gangmu_path, data)
    return {"status": "transitioned", "mu_id": mu_id,
            "from": from_status, "to": to_status, "reason": reason}


def apply_proposed_transitions(gangmu_path, proposed_transitions):
    """批量执行 ceremony_scan 输出的 proposed_transitions。

    每个 transition 是 {"line": mu_id, "from": ..., "to": ..., "reason": ...}。
    """
    results = []
    for pt in proposed_transitions:
        mu_id = pt.get("line", "")
        to_status = pt.get("to", "")
        reason = pt.get("reason", "")
        if not mu_id or not to_status:
            results.append({"mu_id": mu_id, "status": "skipped",
                            "reason": "missing line or to"})
            continue
        try:
            result = transition_mu(gangmu_path, mu_id, to_status, reason)
            results.append(result)
        except Exception as exc:
            results.append({"mu_id": mu_id, "status": "error",
                            "error": str(exc)})
    return results


def main():
    parser = argparse.ArgumentParser(
        description="纲目动态更新——gangmu.yaml 的原子读-改-写操作")
    parser.add_argument(
        "--root", default=None,
        help="项目根目录（默认当前目录）")

    sub = parser.add_subparsers(dest="command", required=True)

    # complete 子命令
    p_complete = sub.add_parser(
        "complete", help='标记 action 完成，格式: mu_id:action_target')
    p_complete.add_argument(
        "spec", help='格式: mu_id:action_target')

    # append 子命令
    p_append = sub.add_parser(
        "append", help='向目追加新 action')
    p_append.add_argument("mu_id", help="目 ID")
    p_append.add_argument(
        "--action", required=True,
        help='JSON 格式的 action dict')

    # transition 子命令
    p_transition = sub.add_parser(
        "transition", help='目状态转换')
    p_transition.add_argument("mu_id", help="目 ID")
    p_transition.add_argument("--to", required=True, help="目标状态")
    p_transition.add_argument("--reason", required=True, help="转换原因")

    # apply-proposed 子命令
    p_apply = sub.add_parser(
        "apply-proposed",
        help='批量执行 ceremony_scan 输出的 proposed_transitions')
    p_apply.add_argument(
        "--json", required=True,
        help='JSON 格式的 proposed_transitions 列表')

    args = parser.parse_args()
    root = args.root or os.getcwd()
    gangmu_path = os.path.join(root, ".chanlun", "gangmu.yaml")

    if args.command == "complete":
        parts = args.spec.split(":", 1)
        if len(parts) != 2:
            print(json.dumps({"error": "格式错误，应为 mu_id:action_target"},
                             ensure_ascii=False))
            sys.exit(1)
        mu_id, target = parts
        result = complete_action(gangmu_path, mu_id, target)
    elif args.command == "append":
        action_dict = json.loads(args.action)
        result = append_action(gangmu_path, mu_id=args.mu_id,
                               action_dict=action_dict)
    elif args.command == "transition":
        result = transition_mu(gangmu_path, args.mu_id, args.to, args.reason)
    elif args.command == "apply-proposed":
        transitions = json.loads(args.json)
        result = apply_proposed_transitions(gangmu_path, transitions)
    else:
        parser.print_help()
        sys.exit(1)

    print(json.dumps(result, ensure_ascii=False, indent=2))


if __name__ == "__main__":
    main()
