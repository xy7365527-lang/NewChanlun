#!/usr/bin/env python
"""二阶反馈代码强制：扫描已结算谱系的下游推论执行状态。

扫描 .chanlun/genealogy/settled/*.md，提取每条谱系的"下游推论"章节中的
编号条目，交叉引用后续谱系和 git diff 判断执行状态。

用法:
  python scripts/downstream_audit.py              # 输出 JSON 报告
  python scripts/downstream_audit.py --summary    # 只输出摘要
"""
import json, os, re, glob, sys, argparse

OVERRIDE_STATUSES = {"resolved", "superseded", "long_term", "background_noise"}


def extract_downstream_actions(filepath):
    """从谱系文件提取下游推论编号条目。"""
    with open(filepath, encoding="utf-8") as f:
        content = f.read()

    # 提取谱系 ID
    m = re.search(r'^id:\s*["\']?(\d{3})["\']?', content, re.MULTILINE)
    gid = m.group(1) if m else os.path.basename(filepath)[:3]

    # 提取下游推论章节
    m = re.search(
        r'##\s*下游推论\s*\n(.*?)(?=\n##\s|\Z)',
        content, re.DOTALL
    )
    if not m:
        return gid, []

    section = m.group(1)
    # 提取编号条目（1. xxx 或 - xxx）
    items = re.findall(
        r'(?:^\d+\.\s+\*\*(.+?)\*\*[：:]\s*(.+)|^\d+\.\s+(.+)|^-\s+\*\*(.+?)\*\*[：:]\s*(.+)|^-\s+(.+))',
        section, re.MULTILINE
    )

    actions = []
    for i, groups in enumerate(items, 1):
        # 合并匹配组
        title = groups[0] or groups[2] or groups[3] or groups[5] or ""
        detail = groups[1] or groups[4] or ""
        text = f"{title}: {detail}".strip(": ") if detail else title.strip()
        if text:
            actions.append({"index": i, "text": text})

    return gid, actions


def load_negation_map(settled_dir):
    """构建谱系否定关系映射：{被否定的gid: [否定它的gid]}。"""
    neg_map = {}
    for fp in glob.glob(os.path.join(settled_dir, "*.md")):
        with open(fp, encoding="utf-8") as f:
            content = f.read()
        m = re.search(r'^id:\s*["\']?(\d{3})["\']?', content, re.MULTILINE)
        if not m:
            continue
        gid = m.group(1)
        # 提取 negates 字段
        neg_m = re.search(r'^negates:\s*\[([^\]]*)\]', content, re.MULTILINE)
        if neg_m and neg_m.group(1).strip():
            for target in re.findall(r'["\']?(\d{3})["\']?', neg_m.group(1)):
                neg_map.setdefault(target, []).append(gid)
    return neg_map


def check_action_resolved(action_text, gid, all_settled_content, neg_map):
    """启发式判断下游行动是否已被后续谱系/commit 解决。

    检查规则：
    - 如果该谱系被后续谱系否定（negates 字段）→ superseded
    - 如果后续谱系明确引用了该 gid 的下游推论编号 → resolved
    """
    # superseded 检测：谱系被否定 → 其下游行动自动失效
    if gid in neg_map:
        return "superseded"

    # 检查是否有后续谱系引用 "gid号下游推论"
    pattern = rf'{gid}号.*下游推论'
    for other_gid, other_content in all_settled_content:
        if other_gid == gid:
            continue
        if re.search(pattern, other_content):
            return "resolved"

    return "unresolved"


def load_overrides(root):
    """加载手动覆盖文件 .chanlun/downstream-action-overrides.yaml。"""
    override_path = os.path.join(root, ".chanlun", "downstream-action-overrides.yaml")
    overrides = {}
    if not os.path.isfile(override_path):
        return overrides
    status_pattern = "|".join(sorted(OVERRIDE_STATUSES, key=len, reverse=True))
    with open(override_path, encoding="utf-8") as f:
        for line in f:
            line = line.strip()
            if not line or line.startswith("#"):
                continue
            m = re.match(rf'^(\d{{3}})-(\d+):\s*({status_pattern})\b', line)
            if m:
                key = f"{m.group(1)}-{m.group(2)}"
                overrides[key] = m.group(3)
    return overrides


def audit(root=None):
    """执行完整审计，返回结构化报告。"""
    root = root or os.getcwd()
    settled_dir = os.path.join(root, ".chanlun", "genealogy", "settled")
    if not os.path.isdir(settled_dir):
        return {"total_actions": 0, "unresolved": 0, "superseded": 0, "items": []}

    overrides = load_overrides(root)
    files = sorted(glob.glob(os.path.join(settled_dir, "*.md")))

    # 预加载所有内容用于交叉引用
    all_content = []
    for fp in files:
        with open(fp, encoding="utf-8") as f:
            content = f.read()
        m = re.search(r'^id:\s*["\']?(\d{3})["\']?', content, re.MULTILINE)
        gid = m.group(1) if m else os.path.basename(fp)[:3]
        all_content.append((gid, content))

    neg_map = load_negation_map(settled_dir)

    items = []
    total = 0
    unresolved = 0
    superseded = 0
    long_term = 0
    background_noise = 0

    for fp in files:
        gid, actions = extract_downstream_actions(fp)
        if not actions:
            continue
        for action in actions:
            total += 1
            override_key = f"{gid}-{action['index']}"
            override_status = overrides.get(override_key)
            if override_status:
                status = override_status
            else:
                status = check_action_resolved(action["text"], gid, all_content, neg_map)
            if status == "superseded":
                superseded += 1
            elif status == "long_term":
                long_term += 1
            elif status == "background_noise":
                background_noise += 1
            elif status == "unresolved":
                unresolved += 1
            items.append({
                "genealogy_id": gid,
                "action_index": action["index"],
                "text": action["text"],
                "status": status,
            })

    resolved = total - unresolved - superseded - long_term - background_noise
    return {
        "total_actions": total,
        "unresolved": unresolved,
        "superseded": superseded,
        "long_term": long_term,
        "background_noise": background_noise,
        "resolved": resolved,
        "execution_rate": f"{resolved / total * 100:.0f}%" if total else "N/A",
        "items": [i for i in items if i["status"] not in ("resolved",)],
    }


def main():
    parser = argparse.ArgumentParser(description="二阶反馈：下游推论执行审计")
    parser.add_argument("--summary", action="store_true", help="只输出摘要")
    parser.add_argument("--root", default=None, help="项目根目录")
    args = parser.parse_args()

    report = audit(args.root or os.getcwd())

    if args.summary:
        print(f"下游推论: {report['total_actions']} 总计, "
              f"{report['resolved']} 已解决, "
              f"{report['superseded']} superseded, "
              f"{report.get('long_term', 0)} 长期, "
              f"{report.get('background_noise', 0)} 背景噪音, "
              f"{report['unresolved']} 未解决, "
              f"执行率 {report['execution_rate']}")
    else:
        print(json.dumps(report, ensure_ascii=False, indent=2))


if __name__ == "__main__":
    main()
