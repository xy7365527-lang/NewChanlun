"""D′ 开口②：ceremony_scan 消费 reducer 的 ready_workstations 驱动 spawn。

630 开口②：reducer 算的 ready_workstations 从未被 ceremony spawn 消费——ceremony_scan
一直 fallback（roadmap/session）。本测试锁定：reducer 的 ready_details 转化为 result
['workstations'] 中 source='goal_reducer' 的工位，且 goal 工位优先于 roadmap（spec §6：
goal 是当前交易性承诺，roadmap 是 backlog）。

纯函数 goal_driven_workstations 可独立测（无 IO）；端到端测真实 events.jsonl 经 scan
后 workstations 含 goal 驱动工位。
"""
import json
import subprocess

from ceremony_scan import goal_driven_workstations


ROOT = "/Users/silencehan/Projects/NewChanlun"


def test_goal_driven_workstations_maps_ready_details():
    # ready_details → workstation dict（name 含 sub_goal_id，description=desc，source 标记）。
    cg_result = {
        "current_goal": {"goal_id": "g1", "status": "active", "base_head_stale": False},
        "ready_workstations": ["sizing-fix", "nautilus-integration"],
        "ready_details": [
            {"id": "sizing-fix", "desc": "修 sizing 坍缩"},
            {"id": "nautilus-integration", "desc": "接 Nautilus"},
        ],
        "blocked": [],
        "terminated": False,
    }
    ws = goal_driven_workstations(cg_result)
    assert len(ws) == 2
    assert all(w["source"] == "goal_reducer" for w in ws)
    names = [w["name"] for w in ws]
    assert any("sizing-fix" in n for n in names)
    assert any("nautilus-integration" in n for n in names)
    descs = [w["description"] for w in ws]
    assert "修 sizing 坍缩" in descs
    # 携带 goal_id 与 sub_goal_id，供下游回写 EVIDENCE/CHECK_PASS 定位
    assert all(w.get("goal_id") == "g1" for w in ws)
    assert {w["sub_goal_id"] for w in ws} == {"sizing-fix", "nautilus-integration"}


def test_goal_driven_workstations_none_goal_empty():
    # goal 未设定（None）→ 无 goal 驱动工位（seed bootloader 不阻塞冷启动）。
    assert goal_driven_workstations(None) == []
    assert goal_driven_workstations({"current_goal": None, "ready_workstations": [],
                                     "ready_details": [], "blocked": [], "terminated": False}) == []


def test_goal_driven_workstations_none_goal_id_empty():
    # HIGH-3：current_goal 非 None 但 goal_id=None（reducer 容错路径，退化事件两字段皆缺）
    # → 不产 goal 工位（否则 name=goal[None]、goal_id=None 进 spawn 列表，下游回写
    # CHECK_PASS 用 goal_id=None 触发 writer 校验失败，错误延迟到下游）。
    cg_result = {
        "current_goal": {"goal_id": None, "status": "active", "base_head_stale": False},
        "ready_workstations": ["x"],
        "ready_details": [{"id": "x", "desc": "d"}],
        "blocked": [], "terminated": False,
    }
    assert goal_driven_workstations(cg_result) == []


def test_goal_driven_workstations_closed_goal_empty():
    # goal 已闭合（terminated）→ 无 ready 工位（不再驱动 spawn）。
    cg_result = {
        "current_goal": {"goal_id": "g1", "status": "closed", "base_head_stale": False},
        "ready_workstations": [], "ready_details": [], "blocked": [], "terminated": True,
    }
    assert goal_driven_workstations(cg_result) == []


def test_scan_emits_goal_driven_workstations_end_to_end():
    # 真实 events.jsonl 经 scan → workstations 含 source=goal_reducer 的工位（开口②闭合）。
    out = subprocess.run(
        ["python", "scripts/ceremony_scan.py"],
        cwd=ROOT, capture_output=True, text=True,
    )
    assert out.returncode == 0, f"scan 退出非0: {out.stderr}"
    data = json.loads(out.stdout)
    goal_ws = [w for w in data["workstations"] if w.get("source") == "goal_reducer"]
    assert goal_ws, "reducer 驱动的工位未进 workstations（开口②未闭合）"
    # goal 工位优先：在 workstations 列表最前（spec §6 goal 先于 roadmap backlog）
    assert data["workstations"][0]["source"] == "goal_reducer"


def test_materialize_interrupt_flag_writes_projection(tmp_path):
    # 开口③闭环：--materialize-interrupt 机械刷新中断点 projection（取代手搓）。
    # 输出 projection 文本到 stdout（≤50 行，含 goal_id），并写 .interrupt-point.md。
    out = subprocess.run(
        ["python", "scripts/ceremony_scan.py", "--materialize-interrupt"],
        cwd=ROOT, capture_output=True, text=True,
    )
    assert out.returncode == 0, f"materialize 退出非0: {out.stderr}"
    # 测试隔离：projection 含真实 events.jsonl 当前 goal 的 goal_id（不硬编码具体值——
    # 否则真实 goal 一变就脆断，违反 test isolation）。从 reducer 取当前 goal_id 对照。
    from ceremony_scan import _load_current_goal  # noqa: E402（运行时 sys.path 已含 scripts/）
    gr = _load_current_goal()
    goal_id = (gr or {}).get("current_goal", {}).get("goal_id") if gr else None
    if goal_id:
        assert goal_id in out.stdout, f"projection 未含当前 goal_id {goal_id}"
    assert len(out.stdout.splitlines()) <= 51  # ≤50 行 + 可能末尾换行
