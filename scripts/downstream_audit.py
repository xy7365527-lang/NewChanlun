#!/usr/bin/env python
"""二阶反馈代码强制：扫描已结算谱系的下游推论执行状态。

扫描 .chanlun/genealogy/settled/*.md，提取每条谱系的"下游推论"章节中的
编号条目，交叉引用后续谱系和 git diff 判断执行状态。

154号-2 优化：verification_hint 机制——在报告 unresolved 前先用 hint
检查实际文件内容，减少假阳性。
156号扩展：支持 expect: absent（pattern 不应出现在文件中）。
P6修复：解析 YAML frontmatter downstream_inferences 字段，消除假阳性。

用法:
  python scripts/downstream_audit.py              # 输出 JSON 报告
  python scripts/downstream_audit.py --summary    # 只输出摘要
"""
import json, os, re, glob, sys, argparse, yaml

OVERRIDE_STATUSES = {"resolved", "superseded", "blocked", "background_noise"}
# 157号：long_term 已被否定，所有原 long_term 迁移为 blocked


def parse_yaml_frontmatter(content):
    """解析文件的 YAML frontmatter，返回 parsed dict 或 None。"""
    if not content.startswith("---"):
        return None
    end = content.find("\n---", 3)
    if end < 0:
        return None
    try:
        return yaml.safe_load(content[3:end])
    except Exception:
        return None


def extract_yaml_downstream_statuses(content):
    """从 YAML frontmatter 的 downstream_inferences 提取 {id: status} 映射。

    P6修复：YAML frontmatter 中的 downstream_inferences 是结构化数据源，
    其 status 字段优先级高于 markdown 内联标记和启发式判断。

    返回 dict: {"228-1": "resolved", "228-2": "resolved", ...}
    """
    fm = parse_yaml_frontmatter(content)
    if not fm or not isinstance(fm, dict):
        return {}
    inferences = fm.get("downstream_inferences")
    if not inferences or not isinstance(inferences, list):
        return {}
    result = {}
    for item in inferences:
        if not isinstance(item, dict):
            continue
        item_id = str(item.get("id", ""))
        item_status = str(item.get("status", ""))
        if item_id and item_status:
            result[item_id] = item_status
    return result


def extract_yaml_only_actions(content, gid):
    """从仅有 YAML frontmatter downstream_inferences（无 markdown 章节）的文件提取 actions。

    当文件有 YAML downstream_inferences 但没有 ## 下游推论 章节时，
    YAML 是唯一的数据源——直接从 YAML 构建 actions 列表。
    """
    fm = parse_yaml_frontmatter(content)
    if not fm or not isinstance(fm, dict):
        return []
    inferences = fm.get("downstream_inferences")
    if not inferences or not isinstance(inferences, list):
        return []
    actions = []
    for i, item in enumerate(inferences, 1):
        if not isinstance(item, dict):
            continue
        item_id = str(item.get("id", ""))
        desc = item.get("description") or item.get("content") or ""
        if not desc:
            continue
        # 从 id 提取 index（如 "180-2" → 2），fallback 到枚举序号
        index = i
        if item_id:
            parts = item_id.rsplit("-", 1)
            if len(parts) == 2 and parts[1].isdigit():
                index = int(parts[1])
        actions.append({"index": index, "text": desc, "resolved_inline": False})
    return actions


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
        # P6修复：无 markdown 章节时，尝试从 YAML frontmatter 提取
        yaml_actions = extract_yaml_only_actions(content, gid)
        return gid, yaml_actions

    section = m.group(1)
    # 提取编号条目（1. xxx 或 - xxx）
    items = re.findall(
        r'(?:^\d+\.\s+\*\*(.+?)\*\*[：:]\s*(.+)|^\d+\.\s+(.+)|^-\s+\*\*(.+?)\*\*[：:]\s*(.+)|^-\s+(.+))',
        section, re.MULTILINE
    )

    actions = []
    # 逐行扫描原始文本，用于检测删除线/已执行标记
    # 只取顶级编号行（\d+.），排除子列表项（- xxx）避免索引错位
    raw_lines = [
        line for line in section.split("\n")
        if re.match(r'^(\d+\.\s+|-\s+)', line)
    ]
    for i, groups in enumerate(items, 1):
        # 合并匹配组
        title = groups[0] or groups[2] or groups[3] or groups[5] or ""
        detail = groups[1] or groups[4] or ""
        text = f"{title}: {detail}".strip(": ") if detail else title.strip()
        if text:
            # 检测内联已解决标记（删除线 / 已执行 / 已确认）
            raw = raw_lines[i - 1].strip() if i - 1 < len(raw_lines) else ""
            resolved_inline = False
            if raw.lstrip("0123456789.-) ").startswith("~~"):
                resolved_inline = True
            elif "**已执行**" in raw or "**已确认**" in raw or "**已完成**" in raw or "**已结算" in raw:
                resolved_inline = True
            elif "已执行——" in raw or "已确认——" in raw or "已完成——" in raw:
                resolved_inline = True
            elif "→ [resolved:" in raw or "`[resolved:" in raw:
                resolved_inline = True
            elif "→ [blocked:" in raw or "`[blocked:" in raw:
                resolved_inline = "blocked"
            elif "`[acknowledged:" in raw or "→ [acknowledged:" in raw:
                resolved_inline = "blocked"
            actions.append({"index": i, "text": text, "resolved_inline": resolved_inline})

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


def load_verification_hints(root):
    """加载 verification hints 文件 .chanlun/downstream-verification-hints.yaml。

    154号-2 优化：为下游推论提供轻量级内容验证，减少假阳性。
    156号扩展：支持 expect: absent（pattern 不应出现在文件中）。

    格式:
      hints:
        "155-1":
          - file: ".claude/agents/claude-challenger.md"
            pattern: "Codex"
        "156-1":
          - file: ".claude/commands/ceremony.md"
            pattern: "定理/行动类"
            expect: absent  # pattern 不应出现在文件中

    每个 hint 条目包含 file（相对于 root 的路径）、pattern（grep 关键字串）、
    可选的 expect（"present" 或 "absent"，默认 "present"）。
    所有 hint 条目都匹配时视为 resolved。
    """
    hints_path = os.path.join(root, ".chanlun", "downstream-verification-hints.yaml")
    if not os.path.isfile(hints_path):
        return {}
    try:
        with open(hints_path, encoding="utf-8") as f:
            data = yaml.safe_load(f)
        return data.get("hints", {}) if isinstance(data, dict) else {}
    except Exception:
        return {}


def verify_by_hint(root, hints_for_key):
    """对一组 verification hints 逐个检查文件内容。

    返回 True 当且仅当所有 hint 条件都满足。

    每个 hint 支持 expect 字段：
    - expect: "present"（默认）— pattern 必须在文件中存在
    - expect: "absent" — pattern 必须不在文件中出现（156号：否定性验证）

    对于 expect: "present"：文件不存在或 pattern 未匹配 → False。
    对于 expect: "absent"：文件不存在视为满足（pattern 确实不在）；
                          文件存在且 pattern 出现 → False。
    """
    for hint in hints_for_key:
        target_file = hint.get("file", "")
        pattern = hint.get("pattern", "")
        if not target_file or not pattern:
            continue
        expect = hint.get("expect", "present")
        filepath = os.path.join(root, target_file)
        file_exists = os.path.isfile(filepath)

        if expect == "absent":
            # 文件不存在 → pattern 不可能出现 → 满足 absent 条件
            if not file_exists:
                continue
            try:
                with open(filepath, encoding="utf-8") as f:
                    content = f.read()
                if pattern in content:
                    return False  # pattern 不应出现但出现了
            except Exception:
                continue  # 读取失败视为 pattern 不可观测 → 满足 absent
        else:
            # expect: "present"（默认）
            if not file_exists:
                return False
            try:
                with open(filepath, encoding="utf-8") as f:
                    content = f.read()
                if pattern not in content:
                    return False
            except Exception:
                return False
    return True


def check_action_resolved(action_text, gid, all_settled_content, neg_map):
    """启发式判断下游行动是否已被后续谱系/commit 解决。

    检查规则：
    1. 如果该谱系被后续谱系否定（negates 字段）→ superseded
    2. 如果后续谱系明确引用了该 gid 的下游推论编号 → resolved
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
    verification_hints = load_verification_hints(root)
    files = sorted(glob.glob(os.path.join(settled_dir, "*.md")))

    # 预加载所有内容用于交叉引用
    all_content = []
    # P6修复：预加载 YAML frontmatter downstream_inferences 状态
    yaml_statuses = {}  # {"{gid}-{index}": status}
    for fp in files:
        with open(fp, encoding="utf-8") as f:
            content = f.read()
        m = re.search(r'^id:\s*["\']?(\d{3})["\']?', content, re.MULTILINE)
        gid = m.group(1) if m else os.path.basename(fp)[:3]
        all_content.append((gid, content))
        yaml_statuses.update(extract_yaml_downstream_statuses(content))

    neg_map = load_negation_map(settled_dir)

    items = []
    total = 0
    unresolved = 0
    superseded = 0
    long_term = 0  # deprecated by 157号, kept for backward compat
    blocked = 0
    background_noise = 0

    for fp in files:
        gid, actions = extract_downstream_actions(fp)
        if not actions:
            continue
        for action in actions:
            total += 1
            override_key = f"{gid}-{action['index']}"
            # P6修复：优先级链 yaml_statuses > overrides > inline > heuristic > verification_hints
            yaml_status = yaml_statuses.get(override_key)
            override_status = overrides.get(override_key)
            if yaml_status and yaml_status in OVERRIDE_STATUSES:
                status = yaml_status
            elif override_status:
                status = override_status
            elif action.get("resolved_inline"):
                status = "blocked" if action["resolved_inline"] == "blocked" else "resolved"
            else:
                status = check_action_resolved(action["text"], gid, all_content, neg_map)
                # 154号-2 优化：heuristic 判定 unresolved 时，
                # 用 verification hint 检查实际文件内容
                if status == "unresolved" and override_key in verification_hints:
                    if verify_by_hint(root, verification_hints[override_key]):
                        status = "resolved"
            if status == "superseded":
                superseded += 1
            elif status == "long_term":
                long_term += 1
            elif status == "blocked":
                blocked += 1
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

    resolved = total - unresolved - superseded - long_term - blocked - background_noise
    return {
        "total_actions": total,
        "unresolved": unresolved,
        "superseded": superseded,
        "blocked": blocked,
        "long_term": long_term,  # deprecated by 157号
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
              f"{report.get('blocked', 0)} 阻塞, "
              f"{report.get('long_term', 0)} 长期(deprecated), "
              f"{report.get('background_noise', 0)} 背景噪音, "
              f"{report['unresolved']} 未解决, "
              f"执行率 {report['execution_rate']}")
    else:
        print(json.dumps(report, ensure_ascii=False, indent=2))


if __name__ == "__main__":
    main()
