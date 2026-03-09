#!/usr/bin/env python
"""异步自指调度器：t 审查 t-1（183号目B）。

t = 当前 RTAS 循环（最新 session）
t-1 = 上一个 RTAS 循环（前一个 session）

确定性部分：
- 读取上一轮 session 文件，提取 settled 计数
- 比较 t-1 和 t 的 settled 计数差
- 检测 t-1 产出的谱系是否在 t 中被引用（depends_on 关系）
- 检测 t-1 的下游推论在 t 中的解决率

非确定性部分标记为 TODO（由调用方决定是否调 LLM）。

用法:
  python scripts/async_self_reference.py              # 输出 JSON 审查报告
  python scripts/async_self_reference.py --root DIR   # 指定项目根目录
"""
import argparse
import json
import glob
import os
import re
import subprocess
import yaml


def get_sorted_sessions(root):
    """按修改时间排序返回 session 文件路径列表（最旧在前）。"""
    sessions = glob.glob(os.path.join(root, ".chanlun/sessions/*-session.md"))
    sessions.sort(key=os.path.getmtime)
    return sessions


def extract_settled_count(session_path):
    """从 session 文件提取已结算谱系数。

    匹配 "已结算: N 个" 或 "- 已结算: N 个" 格式。
    返回 int 或 None（无法提取时）。
    """
    try:
        with open(session_path, encoding="utf-8") as f:
            content = f.read(4000)
        m = re.search(r'已结算:\s*(\d+)\s*个', content)
        if m:
            return int(m.group(1))
    except Exception:
        pass
    return None


def extract_session_time(session_path):
    """从 session 文件提取时间戳字符串。

    匹配 "**时间**: YYYY-MM-DD-HHMM" 格式。
    """
    try:
        with open(session_path, encoding="utf-8") as f:
            content = f.read(1000)
        m = re.search(r'\*\*时间\*\*:\s*(.+)', content)
        if m:
            return m.group(1).strip()
    except Exception:
        pass
    return os.path.basename(session_path)


def get_settled_ids_in_range(root, low_exclusive, high_inclusive):
    """返回编号在 (low_exclusive, high_inclusive] 范围内的 settled 谱系 ID 集合。

    这些是在 t-1 轮次中新增的谱系。
    """
    settled_dir = os.path.join(root, ".chanlun/genealogy/settled")
    if not os.path.isdir(settled_dir):
        return set()

    ids = set()
    for filepath in glob.glob(os.path.join(settled_dir, "*.md")):
        basename = os.path.basename(filepath)
        m = re.match(r'^(\d+)-', basename)
        if not m:
            continue
        num = int(m.group(1))
        if low_exclusive < num <= high_inclusive:
            ids.add(str(num).zfill(3))
    return ids


def check_depends_on_references(root, t_minus_1_ids):
    """检测 t-1 产出的谱系是否在后续谱系中被 depends_on 引用。

    通过 block-topology 的 relations.jsonl + meta.json 中的 id_mapping
    将 SHA 映射回谱系编号，检查 t-1 产出的编号是否出现在
    depends_on 关系的 to 端。

    返回 dict: {谱系编号: [引用它的谱系编号列表]}
    """
    meta_path = os.path.join(root, ".chanlun/block-topology/meta.json")
    relations_path = os.path.join(root, ".chanlun/block-topology/relations.jsonl")

    if not os.path.isfile(meta_path) or not os.path.isfile(relations_path):
        return {}

    # 构建 id→sha 和 sha→id 映射
    try:
        with open(meta_path, encoding="utf-8") as f:
            meta = json.load(f)
    except Exception:
        return {}

    id_to_sha = meta.get("id_mapping", {})
    sha_to_id = {}
    for gid, sha in id_to_sha.items():
        sha_to_id[sha] = gid

    # t-1 产出的 SHA 集合
    target_shas = set()
    for gid in t_minus_1_ids:
        # 尝试去零前缀和保留零前缀两种形式
        sha = id_to_sha.get(gid) or id_to_sha.get(str(int(gid)))
        if sha:
            target_shas.add(sha)

    if not target_shas:
        return {}

    # 扫描 relations.jsonl 中的 depends_on 关系
    references = {}  # {target_id: [referrer_id, ...]}
    try:
        with open(relations_path, encoding="utf-8") as f:
            for line in f:
                line = line.strip()
                if not line:
                    continue
                rel = json.loads(line)
                if rel.get("relation") != "depends_on":
                    continue
                if rel["to"] in target_shas:
                    target_id = sha_to_id.get(rel["to"], rel["to"][:8])
                    referrer_id = sha_to_id.get(rel["from"], rel["from"][:8])
                    references.setdefault(target_id, [])
                    if referrer_id not in references[target_id]:
                        references[target_id].append(referrer_id)
    except Exception:
        pass

    return references


def compute_downstream_resolution_rate(root, t_minus_1_ids):
    """检测 t-1 产出的谱系的下游推论在后续轮次中的解决率。

    复用 downstream_audit 的逻辑，筛选 t-1 产出的谱系。
    返回 dict 或 None。
    """
    try:
        from scripts.downstream_audit import audit
        report = audit(root)
    except Exception:
        return None

    if not report or report["total_actions"] == 0:
        return None

    # 过滤 t-1 产出的谱系
    # t_minus_1_ids 是 zero-padded "001" 格式
    t_minus_1_items = [
        item for item in report.get("items", [])
        if item.get("genealogy_id") in t_minus_1_ids
    ]

    if not t_minus_1_items:
        return None

    total = len(t_minus_1_items)
    unresolved = sum(1 for i in t_minus_1_items if i["status"] == "unresolved")
    resolved = total - unresolved

    return {
        "total_downstream_actions": total,
        "resolved": resolved,
        "unresolved": unresolved,
        "resolution_rate": f"{resolved / total * 100:.0f}%" if total else "N/A",
        "unresolved_items": [
            {"genealogy_id": i["genealogy_id"], "text": i["text"][:120]}
            for i in t_minus_1_items if i["status"] == "unresolved"
        ],
    }


def check_git_activity(root, t_minus_1_session, t_session):
    """检查两个 session 之间是否有 git commit 活动。

    从 session 文件的修改时间推断时间范围，用 git log --after/--before
    统计 commit 数量。有 commit 说明 swarm 有产出（即使 settled 没变化）。

    返回 (commit_count: int, has_activity: bool)。
    异常时返回 (0, False)，不阻塞主流程。
    """
    try:
        t_minus_1_mtime = os.path.getmtime(t_minus_1_session)
        t_mtime = os.path.getmtime(t_session)
        # git log --after 使用 ISO 格式
        import datetime
        after = datetime.datetime.fromtimestamp(t_minus_1_mtime).strftime("%Y-%m-%dT%H:%M:%S")
        before = datetime.datetime.fromtimestamp(t_mtime).strftime("%Y-%m-%dT%H:%M:%S")
        result = subprocess.run(
            ["git", "log", "--oneline", f"--after={after}", f"--before={before}"],
            capture_output=True, text=True, cwd=root, timeout=10,
        )
        if result.returncode == 0:
            lines = [l for l in result.stdout.strip().split("\n") if l.strip()]
            return len(lines), len(lines) > 0
    except Exception:
        pass
    return 0, False


def classify_finding(finding_type, detail):
    """构造一个标准 finding 条目。"""
    return {"type": finding_type, "detail": detail}


def audit(root=None):
    """执行异步自指审查：t 审查 t-1。

    返回结构化 JSON 报告。
    """
    root = root or os.getcwd()
    sessions = get_sorted_sessions(root)

    if len(sessions) < 2:
        return {
            "error": "需要至少 2 个 session 文件才能执行 t/t-1 审查",
            "session_count": len(sessions),
            "t_minus_1_summary": None,
            "self_audit_findings": [],
            "genealogy_needed": False,
        }

    t_session = sessions[-1]
    t_minus_1_session = sessions[-2]

    t_time = extract_session_time(t_session)
    t_minus_1_time = extract_session_time(t_minus_1_session)

    t_settled = extract_settled_count(t_session)
    t_minus_1_settled = extract_settled_count(t_minus_1_session)

    # 构建 t-1 摘要
    t_minus_1_summary = {
        "session_file": os.path.basename(t_minus_1_session),
        "session_time": t_minus_1_time,
        "settled_count": t_minus_1_settled,
    }

    findings = []

    # 1. settled 计数差分析
    delta_settled = None
    if t_settled is not None and t_minus_1_settled is not None:
        delta_settled = t_settled - t_minus_1_settled
        t_minus_1_summary["delta_settled"] = delta_settled

        if delta_settled == 0:
            # 406号修复：检查 git commit 活动——settled 不变但有代码产出不算停滞
            git_commits, has_git_activity = check_git_activity(root, t_minus_1_session, t_session)
            if has_git_activity:
                findings.append(classify_finding(
                    "progression",
                    f"t-1({t_minus_1_time}) 到 t({t_time}) 之间 settled 计数未变化"
                    f"（均为 {t_settled}），但有 {git_commits} 个 git commit，"
                    f"swarm 产出为基础设施/代码工作",
                ))
            else:
                findings.append(classify_finding(
                    "stagnation",
                    f"t-1({t_minus_1_time}) 到 t({t_time}) 之间 settled 计数未变化"
                    f"（均为 {t_settled}）且无 git commit 活动，RTAS 循环可能停滞",
                ))
        elif delta_settled < 0:
            findings.append(classify_finding(
                "anomaly",
                f"t 的 settled 计数({t_settled}) 低于 t-1({t_minus_1_settled})，"
                f"delta={delta_settled}，可能存在谱系回退或文件删除",
            ))
        else:
            findings.append(classify_finding(
                "progression",
                f"t-1 到 t 新增 {delta_settled} 个 settled 谱系"
                f"（{t_minus_1_settled} → {t_settled}）",
            ))

    # 2. 检测 t-1 产出的谱系是否被 t 中的谱系引用
    t_minus_1_new_ids = set()
    if t_settled is not None and t_minus_1_settled is not None and delta_settled is not None:
        # 估算 t-1 新增的谱系编号范围
        # t-1 的 settled 是 t_minus_1_settled，假设之前是 t_minus_1_settled - delta_prev
        # 简化：t-1 产出的谱系编号 = (t_minus_1_settled - delta_from_prev, t_minus_1_settled]
        # 但我们没有 t-2 的数据。用另一种方式：
        # t-1 产出 = 编号 in (t_minus_1_settled - N, t_minus_1_settled] where N = delta at t-1
        # 退化到：所有编号 > t_minus_1_settled 且 <= t_settled 是 t 产出的
        # 所有编号 <= t_minus_1_settled 是 t-1 及之前的
        # 要找 t-1 产出的，我们需要 t-2 的 settled 计数
        #
        # 安全做法：如果有 3 个 session，用 t-2 的 settled 计数确定 t-1 产出范围
        if len(sessions) >= 3:
            t_minus_2_settled = extract_settled_count(sessions[-3])
            if t_minus_2_settled is not None:
                t_minus_1_new_ids = get_settled_ids_in_range(
                    root, t_minus_2_settled, t_minus_1_settled,
                )
        # fallback：没有 t-2 时，使用 t_minus_1_settled 附近的几个编号
        if not t_minus_1_new_ids and t_minus_1_settled is not None:
            # 取 t_minus_1_settled 自身作为估计（至少检查最新一个）
            candidate_id = str(t_minus_1_settled).zfill(3)
            settled_path = os.path.join(
                root, ".chanlun/genealogy/settled",
                f"{candidate_id}-*.md",
            )
            if glob.glob(settled_path.replace("*", "") + "*.md") or \
               glob.glob(os.path.join(root, ".chanlun/genealogy/settled", f"{int(candidate_id):03d}-*.md")):
                t_minus_1_new_ids = {candidate_id}

    if t_minus_1_new_ids:
        t_minus_1_summary["new_genealogy_ids"] = sorted(t_minus_1_new_ids)

        # 检查 depends_on 引用
        references = check_depends_on_references(root, t_minus_1_new_ids)
        referenced_ids = set(references.keys())
        unreferenced_ids = t_minus_1_new_ids - referenced_ids

        if unreferenced_ids:
            findings.append(classify_finding(
                "stagnation",
                f"t-1 产出的谱系 {sorted(unreferenced_ids)} 在 t 中未被任何后续谱系 depends_on 引用",
            ))
        if referenced_ids:
            findings.append(classify_finding(
                "progression",
                f"t-1 产出的谱系 {sorted(referenced_ids)} 在 t 中被引用: "
                + "; ".join(f"{k}→{v}" for k, v in sorted(references.items())),
            ))

        # 3. 下游推论解决率
        downstream = compute_downstream_resolution_rate(root, t_minus_1_new_ids)
        if downstream is not None:
            t_minus_1_summary["downstream_resolution"] = downstream
            if downstream["unresolved"] > 0:
                findings.append(classify_finding(
                    "regression",
                    f"t-1 产出的谱系有 {downstream['unresolved']} 条下游推论未解决"
                    f"（解决率 {downstream['resolution_rate']}）",
                ))

    # 判断是否需要写入新谱系
    genealogy_needed = any(
        f["type"] in ("regression", "stagnation", "anomaly")
        for f in findings
    )

    return {
        "t_session": os.path.basename(t_session),
        "t_time": t_time,
        "t_minus_1_summary": t_minus_1_summary,
        "self_audit_findings": findings,
        "genealogy_needed": genealogy_needed,
        # TODO: 非确定性审查（由调用方决定是否调 LLM）
        # - 深层语义审查：t 是否在重复 t-1 的模式？
        # - 概念层退化检测：新谱系的否定深度是否在下降？
    }


def main():
    parser = argparse.ArgumentParser(description="异步自指调度器：t 审查 t-1")
    parser.add_argument("--root", default=None, help="项目根目录")
    args = parser.parse_args()

    report = audit(args.root or os.getcwd())
    print(json.dumps(report, ensure_ascii=False, indent=2))


if __name__ == "__main__":
    main()
