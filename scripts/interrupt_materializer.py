"""D′ 开口③：中断点 materializer——从 reducer 输出机械生成 ≤50 行 projection 指针。

630 开口③（D 严格解=机械生成）：手搓中断点会漂移/过时（活体证据：interrupt-point
停在旧 HEAD，实际 git 已多个 commit 后；本次恢复差点照搬过时 session）。根因=中断点
被当真相源手写，与 events.jsonl/git 双写不一致。

修复：中断点降为 projection/cache（spec §1.1/§4：state 不再是真相源，过时也无害，因为
reducer 从 goal契约+git+genealogy 重算）。materializer 机械生成 projection 指针，叙事
剥离——叙事归三处单写者：git commit（做了什么）+ genealogy（概念发现史）+ events.jsonl
（目的论运行史）。中断点只留指针（goal_id/base_head/status/ready_workstations/恢复入口），
不留叙事，消除漂移根因。

interrupt_projection 是纯函数（reducer 输出 + git_head → markdown 文本），可独立测、确定性。
materialize 是 IO 入口（读真实 events → reduce → 写 .interrupt-point.md）。

有效域诚实标注（231 L 级）：本 materializer 是 L1 管线正确性（reducer 输出 → projection
文本的确定性转化）。「机械生成消除漂移」的 L2 验证=下次真实 ceremony 恢复时 projection
与 git 真相一致，待验。
"""
import json
import os
import subprocess
import sys

_SCRIPT_DIR = os.path.dirname(os.path.abspath(__file__))
if _SCRIPT_DIR not in sys.path:
    sys.path.insert(0, _SCRIPT_DIR)


def interrupt_projection(goal_result: dict, git_head: str) -> str:
    """从 reducer 输出机械生成 ≤50 行中断点 projection（markdown 文本）。

    叙事剥离：只含指针（goal_id/base_head/status/ready_workstations/恢复入口），无叙事
    （叙事归 git commit + genealogy + events.jsonl）。确定性：同输入同输出，正文不渗入
    时间/随机（git_head 由调用方传入，是地面真相对账值，非时间戳）。

    goal 未设定（None）→ 给 bootstrap 冷启动指引（无 goal 时不崩）。
    """
    cg = (goal_result or {}).get("current_goal")
    skipped = (goal_result or {}).get("skipped_event_lines", 0)
    lines = []
    if cg is None:
        # goal 未设定：seed bootloader 冷启动指引（无 goal 时不崩）。
        lines.append("<!-- D′ projection（机械生成，叙事剥离）。无 current goal：bootstrap 冷启动 -->")
        lines.append("goal_id: (未设定)")
        lines.append(f"git_head: {git_head}")
        lines.append("---")
        lines.append("# 中断点 projection（无 goal）")
        lines.append("")
        lines.append("current goal 未设定（events.jsonl 无未 supersede 的 GOAL_SET）。")
        lines.append("恢复=从 roadmap/中断点推导候选 goal，用 /goal 设定后进 reducer loop。")
        lines.append("恢复入口：`python scripts/ceremony_scan.py`（seed bootloader 不阻塞冷启动）。")
        return "\n".join(lines)

    stale = cg.get("base_head_stale", False)
    base_head = cg.get("base_head") or "(无)"
    acc = cg.get("acceptance", [])
    acc_passed = sum(1 for a in acc if a.get("passed"))
    ready = goal_result.get("ready_workstations", [])
    ready_details = {d["id"]: d["desc"] for d in goal_result.get("ready_details", [])}
    blocked = goal_result.get("blocked", [])

    # frontmatter（恢复时校验，D′ 元数据）。derived=true：machine-generated projection。
    lines.append("<!-- D′ projection（机械生成，叙事剥离）。叙事归 git commit+genealogy+events.jsonl。")
    lines.append("     恢复时核 git 真相：reducer 从 goal+git 重算，不信任本快照（base_head 不匹配即降级） -->")
    lines.append(f"goal_id: {cg['goal_id']}")
    lines.append(f"base_head: {base_head}")
    lines.append(f"git_head: {git_head}")
    lines.append(f"base_head_stale: {str(stale).lower()}")
    lines.append("derived: true")
    lines.append("---")
    lines.append(f"# 中断点 projection：{cg['goal_id']}")
    lines.append("")
    lines.append(f"- status: **{cg['status']}** | acceptance: {acc_passed}/{len(acc)} passed")
    if skipped:
        lines.append(f"- ⚠️ **数据完整性降级**：events.jsonl 有 {skipped} 行损坏被跳过"
                     "——空 ready 可能是损坏导致而非结构性，核 events.jsonl")
    if stale:
        lines.append(f"- ⚠️ **base_head stale**（base_head={base_head} ≠ git_head={git_head}）"
                     "：本快照降级为参考，reducer 重算为准（spec §9）")
    desc = cg.get("description", "")
    if desc:
        lines.append(f"- goal: {desc[:120]}")
    lines.append("")
    lines.append("## 验收（可证伪）")
    for a in acc:
        mark = "✅" if a.get("passed") else "⬜"
        lines.append(f"- {mark} {a.get('check', '')[:110]}")
    lines.append("")
    lines.append("## ready_workstations（reducer 算，spawn 列表）")
    if ready:
        for sid in ready:
            d = ready_details.get(sid, "")
            lines.append(f"- `{sid}`：{d[:90]}")
    else:
        lines.append("- （空——退化数据或全 blocked；核 events.jsonl 结构化 DECOMPOSE）")
    if blocked:
        lines.append("")
        lines.append("## blocked")
        for b in blocked:
            lines.append(f"- `{b['id']}`：{str(b.get('blocker'))[:90]}")
    lines.append("")
    lines.append("## 恢复入口（reducer 重算，非叙事）")
    lines.append("`python scripts/ceremony_scan.py` → current_goal + goal_driven workstations。")
    lines.append("叙事查 events.jsonl（目的论史）/ git log（做了什么）/ genealogy（发现史）。")
    return "\n".join(lines)


def _load_events(ev_path: str) -> tuple[list, int]:
    """读 events.jsonl，逐行容错（对齐 ceremony_scan._load_current_goal）。

    返回 (events, skipped)：events 是解析成功的事件列表，skipped 是损坏被跳过的行数。
    skipped 透出供 projection 标注数据完整性——损坏导致的空 ready 与结构性空 ready 不可
    在 ready_workstations 上区分，须显式暴露（never silently swallow errors）。
    """
    if not os.path.isfile(ev_path):
        return [], 0
    events = []
    skipped = 0
    with open(ev_path, encoding="utf-8") as f:
        for lineno, line in enumerate(f, 1):
            if not line.strip():
                continue
            try:
                events.append(json.loads(line))
            except (json.JSONDecodeError, ValueError) as exc:
                skipped += 1
                print(f"[interrupt_materializer] WARNING: events.jsonl line {lineno} 损坏，跳过: {exc}",
                      file=sys.stderr)
    return events, skipped


def materialize(root: str | None = None) -> str:
    """IO 入口：读真实 events → reduce → 生成 projection → 写 .interrupt-point.md，返回文本。

    取代手搓中断点（消除漂移/过时根因）。git_head 取真实 HEAD 做 base_head 对账。
    """
    if root is None:
        root = os.path.dirname(_SCRIPT_DIR)
    from goal_reducer import reduce_goal

    ev_path = os.path.join(root, ".chanlun", "goals", "events.jsonl")
    events, skipped = _load_events(ev_path)
    proc = subprocess.run(["git", "rev-parse", "HEAD"], capture_output=True, text=True, cwd=root)
    git_head = proc.stdout.strip() if proc.returncode == 0 else ""
    goal_result = reduce_goal(events, {"git_head": git_head}) if events else \
        {"current_goal": None, "ready_workstations": [], "ready_details": [],
         "blocked": [], "terminated": False}
    if skipped:
        goal_result["skipped_event_lines"] = skipped

    text = interrupt_projection(goal_result, git_head)
    out_path = os.path.join(root, ".chanlun", ".interrupt-point.md")
    with open(out_path, "w", encoding="utf-8") as f:
        f.write(text + "\n")
    return text


if __name__ == "__main__":
    print(materialize())
