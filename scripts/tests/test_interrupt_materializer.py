"""D′ 开口③：中断点 materializer（机械生成 projection 指针，叙事剥离）。

630 开口③（D 严格解=机械生成）：手搓中断点会漂移/过时（活体证据：interrupt-point 停在
旧 HEAD，实际 git 多个 commit 后）。本 materializer 从 reducer 输出机械生成 ≤50 行
projection 指针（goal_id/base_head/status/ready_workstations/恢复入口），叙事剥离——
叙事归 git commit + genealogy + events.jsonl，中断点只是 projection/cache（spec §1.1/§4：
state 降为 projection，不再是真相源，过时也无害因为 reducer 从 goal+git 重算）。

纯函数 interrupt_projection 可独立测（无 IO）。
"""
from interrupt_materializer import interrupt_projection, _load_events


def _goal_result():
    return {
        "current_goal": {
            "goal_id": "g-l2-nautilus-production",
            "description": "严格完成 L2 → 接 Nautilus → 生产引擎 → 性能最大化",
            "acceptance": [
                {"check": "L2 严格完成", "falsifiable": True, "passed": False},
                {"check": "接 Nautilus", "falsifiable": True, "passed": False},
            ],
            "base_head": "dcc9e25f95fe459147cf679eeafb04249df33b63",
            "status": "active",
            "base_head_stale": False,
        },
        "ready_workstations": ["nautilus-integration", "sizing-fix"],
        "ready_details": [
            {"id": "nautilus-integration", "desc": "接 Nautilus"},
            {"id": "sizing-fix", "desc": "修 sizing 坍缩"},
        ],
        "blocked": [],
        "terminated": False,
    }


def test_projection_contains_goal_id_and_status():
    out = interrupt_projection(_goal_result(), git_head="dcc9e25f95fe459147cf679eeafb04249df33b63")
    assert "g-l2-nautilus-production" in out
    assert "active" in out
    assert "base_head" in out


def test_projection_lists_ready_workstations():
    out = interrupt_projection(_goal_result(), git_head="dcc9e25f95fe459147cf679eeafb04249df33b63")
    assert "nautilus-integration" in out
    assert "sizing-fix" in out


def test_projection_at_most_50_lines():
    # ≤50 行 projection 指针（叙事剥离的硬约束——叙事归 events.jsonl/commit/genealogy）。
    out = interrupt_projection(_goal_result(), git_head="dcc9e25f95fe459147cf679eeafb04249df33b63")
    assert len(out.splitlines()) <= 50


def test_projection_has_recovery_entry():
    # 含恢复入口指针（reducer 重算入口，不是叙事）。
    out = interrupt_projection(_goal_result(), git_head="dcc9e25f95fe459147cf679eeafb04249df33b63")
    assert "ceremony_scan" in out or "reduce" in out.lower()


def test_projection_flags_stale_when_head_mismatch():
    # base_head 与当前 git_head 不匹配 → projection 标记 stale（spec §9 快照降级信号）。
    gr = _goal_result()
    gr["current_goal"]["base_head_stale"] = True
    out = interrupt_projection(gr, git_head="deadbeef99")
    assert "stale" in out.lower() or "不匹配" in out or "降级" in out


def test_projection_none_goal_bootstrap_hint():
    # goal 未设定（None）→ projection 提示 bootstrap（无 goal 时不崩，给冷启动指引）。
    gr = {"current_goal": None, "ready_workstations": [], "ready_details": [],
          "blocked": [], "terminated": False}
    out = interrupt_projection(gr, git_head="abc123")
    assert len(out.splitlines()) <= 50
    assert "goal" in out.lower()  # 提及 goal 未设定/bootstrap


def test_projection_is_deterministic():
    # 同输入同输出（machine-generated，无随机/时间渗入正文）。
    gr = _goal_result()
    a = interrupt_projection(gr, git_head="dcc9e25f95")
    b = interrupt_projection(gr, git_head="dcc9e25f95")
    assert a == b


def test_projection_shows_blocked():
    gr = _goal_result()
    gr["blocked"] = [{"id": "perf-max", "blocker": "缺前置"}]
    out = interrupt_projection(gr, git_head="dcc9e25f95fe459147cf679eeafb04249df33b63")
    assert "perf-max" in out


def test_load_events_returns_skipped_count(tmp_path):
    # HIGH-2：损坏行透出 skipped 计数（不静默吞没）。
    p = tmp_path / "events.jsonl"
    p.write_text('{"event": "CLOSED", "goal_id": "g1", "ts": "t0"}\n'
                 '{ bad json }\n'
                 'also not json\n', encoding="utf-8")
    events, skipped = _load_events(str(p))
    assert len(events) == 1
    assert skipped == 2


def test_load_events_missing_file(tmp_path):
    events, skipped = _load_events(str(tmp_path / "nope.jsonl"))
    assert events == []
    assert skipped == 0


def test_projection_flags_data_integrity_when_skipped():
    # HIGH-2：projection 标注数据完整性降级——空 ready 可能是损坏导致而非结构性。
    gr = _goal_result()
    gr["ready_workstations"] = []
    gr["ready_details"] = []
    gr["skipped_event_lines"] = 3
    out = interrupt_projection(gr, git_head="dcc9e25f95fe459147cf679eeafb04249df33b63")
    assert "数据完整性" in out or "损坏" in out
    assert "3" in out
