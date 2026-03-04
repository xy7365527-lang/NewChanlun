#!/usr/bin/env python
"""下游推论状态更新工具——在谱系 .md 文件和 overrides.yaml 中更新下游推论状态。

解决持存断裂的第二因素：上轮工位评估了下游推论状态但谱系文件未被更新，
导致下轮 scan 重复输出已处理的推论。

两种操作：
  --resolve "330:4" --reason "..."    标记下游推论为 resolved
  --update-status "335:1" --status "blocked:具体阻塞项"

更新策略（优先级链 P6修复）：
  1. YAML frontmatter downstream_inferences（如果存在）→ 就地更新
  2. downstream-action-overrides.yaml → 追加/更新条目
  3. Markdown 内联标记 → 不修改（只读数据源）

设计约束：
  - 不修改 ceremony_scan.py 的扫描逻辑
  - 只修改 ceremony_scan 消费的数据源
  - 谱系 .md 文件就地修改（不创建新文件）
  - overrides.yaml 追加操作是 append-only（不删除已有条目）
"""
import argparse
import json
import os
import re
import sys
from datetime import date

import yaml


VALID_STATUSES = {"resolved", "superseded", "blocked", "background_noise"}


def _find_genealogy_file(root, genealogy_id):
    """查找谱系文件路径。"""
    settled_dir = os.path.join(root, ".chanlun", "genealogy", "settled")
    if not os.path.isdir(settled_dir):
        raise FileNotFoundError(f"settled 目录不存在: {settled_dir}")

    # 精确匹配：{id}-*.md
    padded_id = genealogy_id.zfill(3)
    for fname in os.listdir(settled_dir):
        if fname.startswith(f"{padded_id}-") and fname.endswith(".md"):
            return os.path.join(settled_dir, fname)

    raise FileNotFoundError(
        f"未找到谱系文件: {padded_id}-*.md in {settled_dir}")


def _parse_yaml_frontmatter(content):
    """解析 YAML frontmatter，返回 (fm_dict, fm_start, fm_end) 或 (None, -1, -1)。"""
    if not content.startswith("---"):
        return None, -1, -1
    end = content.find("\n---", 3)
    if end < 0:
        return None, -1, -1
    fm_end = end + 4  # include "\n---"
    try:
        fm = yaml.safe_load(content[3:end])
        return fm, 0, fm_end
    except Exception:
        return None, -1, -1


def _update_yaml_frontmatter_status(content, genealogy_id, action_index,
                                     new_status, reason):
    """尝试更新 YAML frontmatter 中的 downstream_inferences 状态。

    返回 (updated_content, success)。
    """
    fm, fm_start, fm_end = _parse_yaml_frontmatter(content)
    if fm is None or not isinstance(fm, dict):
        return content, False

    inferences = fm.get("downstream_inferences")
    if not inferences or not isinstance(inferences, list):
        return content, False

    target_id = f"{genealogy_id}-{action_index}"
    found = False
    for item in inferences:
        if not isinstance(item, dict):
            continue
        if str(item.get("id", "")) == target_id:
            status_value = new_status
            if reason:
                status_value = f"{new_status}（{reason}）"
            item["status"] = status_value
            item["updated_at"] = str(date.today())
            found = True
            break

    if not found:
        return content, False

    # 重新序列化 frontmatter
    rest = content[fm_end:]
    new_fm = yaml.dump(fm, allow_unicode=True, default_flow_style=False,
                       sort_keys=False, width=120)
    new_content = f"---\n{new_fm}---{rest}"
    return new_content, True


def _update_overrides_file(root, genealogy_id, action_index, status, reason):
    """在 downstream-action-overrides.yaml 中追加/更新条目。"""
    override_path = os.path.join(
        root, ".chanlun", "downstream-action-overrides.yaml")

    padded_id = genealogy_id.zfill(3)
    key = f"{padded_id}-{action_index}"
    comment = f"  # {reason}" if reason else ""
    new_line = f"{key}: {status}{comment}\n"

    lines = []
    if os.path.isfile(override_path):
        with open(override_path, encoding="utf-8") as f:
            lines = f.readlines()

    # 检查是否已存在
    key_pattern = re.compile(rf'^{re.escape(key)}:\s')
    updated = False
    for i, line in enumerate(lines):
        if key_pattern.match(line):
            lines[i] = new_line
            updated = True
            break

    if not updated:
        # 查找该谱系编号的 section，在末尾追加
        section_header = f"# === {padded_id}号"
        section_idx = -1
        for i, line in enumerate(lines):
            if section_header in line:
                section_idx = i
                break

        if section_idx >= 0:
            # 找到 section，在 section 末尾追加
            insert_idx = section_idx + 1
            while insert_idx < len(lines):
                next_line = lines[insert_idx].strip()
                if next_line.startswith("# ===") and insert_idx > section_idx + 1:
                    break
                insert_idx += 1
            lines.insert(insert_idx, new_line)
        else:
            # 新 section
            if lines and not lines[-1].endswith("\n"):
                lines.append("\n")
            lines.append(f"\n# === {padded_id}号 ===\n")
            lines.append(new_line)

    with open(override_path, "w", encoding="utf-8") as f:
        f.writelines(lines)

    return {"method": "overrides", "key": key, "status": status}


def resolve_downstream(root, genealogy_id, action_index, reason):
    """标记下游推论为 resolved。

    优先更新 YAML frontmatter，fallback 到 overrides.yaml。
    """
    padded_id = genealogy_id.zfill(3)

    # 尝试更新谱系文件的 YAML frontmatter
    try:
        filepath = _find_genealogy_file(root, genealogy_id)
        with open(filepath, encoding="utf-8") as f:
            content = f.read()

        new_content, success = _update_yaml_frontmatter_status(
            content, padded_id, action_index, "resolved", reason)

        if success:
            with open(filepath, "w", encoding="utf-8") as f:
                f.write(new_content)
            return {"status": "resolved", "genealogy_id": padded_id,
                    "action_index": action_index, "method": "yaml_frontmatter",
                    "reason": reason}
    except FileNotFoundError:
        pass  # fallback to overrides

    # Fallback: 更新 overrides.yaml
    result = _update_overrides_file(
        root, genealogy_id, action_index, "resolved", reason)
    result.update({"status": "resolved", "genealogy_id": padded_id,
                   "action_index": action_index, "reason": reason})
    return result


def update_status(root, genealogy_id, action_index, status, reason):
    """更新下游推论状态（通用接口）。"""
    base_status = status.split(":")[0].strip()
    if base_status not in VALID_STATUSES:
        raise ValueError(
            f"无效状态: {base_status}，合法值: {VALID_STATUSES}")

    padded_id = genealogy_id.zfill(3)

    # 尝试更新谱系文件的 YAML frontmatter
    try:
        filepath = _find_genealogy_file(root, genealogy_id)
        with open(filepath, encoding="utf-8") as f:
            content = f.read()

        new_content, success = _update_yaml_frontmatter_status(
            content, padded_id, action_index, base_status, reason)

        if success:
            with open(filepath, "w", encoding="utf-8") as f:
                f.write(new_content)
            return {"status": base_status, "genealogy_id": padded_id,
                    "action_index": action_index, "method": "yaml_frontmatter",
                    "reason": reason}
    except FileNotFoundError:
        pass

    # Fallback: 更新 overrides.yaml
    override_status = status if ":" in status else base_status
    result = _update_overrides_file(
        root, genealogy_id, action_index, override_status, reason)
    result.update({"status": base_status, "genealogy_id": padded_id,
                   "action_index": action_index, "reason": reason})
    return result


def batch_update(root, updates):
    """批量更新下游推论状态。

    updates 是列表，每项: {"genealogy_id": "330", "action_index": 4,
                          "status": "resolved", "reason": "..."}
    """
    results = []
    for u in updates:
        gid = str(u.get("genealogy_id", ""))
        idx = u.get("action_index", 0)
        status = u.get("status", "resolved")
        reason = u.get("reason", "")
        try:
            if status == "resolved":
                result = resolve_downstream(root, gid, idx, reason)
            else:
                result = update_status(root, gid, idx, status, reason)
            results.append(result)
        except Exception as exc:
            results.append({"genealogy_id": gid, "action_index": idx,
                            "status": "error", "error": str(exc)})
    return results


def main():
    parser = argparse.ArgumentParser(
        description="下游推论状态更新——谱系 .md 文件和 overrides.yaml 更新")
    parser.add_argument(
        "--root", default=None,
        help="项目根目录（默认当前目录）")

    sub = parser.add_subparsers(dest="command", required=True)

    # resolve 子命令
    p_resolve = sub.add_parser(
        "resolve", help='标记下游推论为 resolved，格式: genealogy_id:action_index')
    p_resolve.add_argument("spec", help="格式: genealogy_id:action_index")
    p_resolve.add_argument("--reason", required=True, help="解决原因")

    # update-status 子命令
    p_update = sub.add_parser(
        "update-status", help='更新下游推论状态')
    p_update.add_argument("spec", help="格式: genealogy_id:action_index")
    p_update.add_argument("--status", required=True, help="新状态")
    p_update.add_argument("--reason", default="", help="原因说明")

    # batch 子命令
    p_batch = sub.add_parser(
        "batch", help='批量更新下游推论状态')
    p_batch.add_argument(
        "--json", required=True,
        help='JSON 格式的更新列表')

    args = parser.parse_args()
    root = args.root or os.getcwd()

    if args.command == "resolve":
        parts = args.spec.split(":", 1)
        if len(parts) != 2 or not parts[1].isdigit():
            print(json.dumps(
                {"error": "格式错误，应为 genealogy_id:action_index"},
                ensure_ascii=False))
            sys.exit(1)
        gid, idx = parts[0], int(parts[1])
        result = resolve_downstream(root, gid, idx, args.reason)
    elif args.command == "update-status":
        parts = args.spec.split(":", 1)
        if len(parts) != 2 or not parts[1].isdigit():
            print(json.dumps(
                {"error": "格式错误，应为 genealogy_id:action_index"},
                ensure_ascii=False))
            sys.exit(1)
        gid, idx = parts[0], int(parts[1])
        result = update_status(root, gid, idx, args.status, args.reason)
    elif args.command == "batch":
        updates = json.loads(args.json)
        result = batch_update(root, updates)
    else:
        parser.print_help()
        sys.exit(1)

    print(json.dumps(result, ensure_ascii=False, indent=2))


if __name__ == "__main__":
    main()
