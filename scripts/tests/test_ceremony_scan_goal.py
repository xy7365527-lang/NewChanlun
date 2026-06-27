"""D′ 波1 T4：ceremony_scan 读 current_goal（roadmap 之前，goal 是当前交易性承诺）。

测试 ceremony_scan.py 输出 JSON 含 current_goal 字段。
无 goal events（events.jsonl 不存在）时 current_goal 为 None——seed bootloader 在 goal 未设定前不阻塞。

健壮性测试（T4 quality review）：_load_current_goal 对损坏的 events.jsonl 逐行容错——
无文件/全损坏 → None；单行损坏 → 写 stderr 跳过，有效行仍处理。对齐 _load_settlement_requests 风格。
"""
import json
import subprocess

from ceremony_scan import _load_current_goal


ROOT = "/Users/silencehan/Projects/NewChanlun"


def test_scan_reads_current_goal():
    out = subprocess.run(
        ["python", "scripts/ceremony_scan.py"],
        cwd=ROOT, capture_output=True, text=True,
    )
    assert out.returncode == 0, f"scan 退出非0: {out.stderr}"
    data = json.loads(out.stdout)
    assert "current_goal" in data  # 新增字段（无 goal events 时为 None）


def test_load_current_goal_missing_file_returns_none(tmp_path):
    assert _load_current_goal(str(tmp_path / "nope.jsonl")) is None


def test_load_current_goal_valid_events(tmp_path):
    p = tmp_path / "events.jsonl"
    p.write_text('{"event": "GOAL_SET", "goal_id": "g1", "description": "x", '
                 '"acceptance": [{"check": "c", "falsifiable": true}], "base_head": "abc", "ts": "t0"}\n',
                 encoding="utf-8")
    out = _load_current_goal(str(p))
    assert out is not None
    assert out["current_goal"]["goal_id"] == "g1"


def test_load_current_goal_corrupt_line_skipped_not_crash(tmp_path, capsys):
    p = tmp_path / "events.jsonl"
    p.write_text('{"event": "GOAL_SET", "goal_id": "g1", "description": "x", '
                 '"acceptance": [{"check": "c", "falsifiable": true}], "base_head": "abc", "ts": "t0"}\n'
                 '{ this is not valid json }\n', encoding="utf-8")
    out = _load_current_goal(str(p))  # 不应抛异常
    assert out["current_goal"]["goal_id"] == "g1"  # 有效行仍被处理
    err = capsys.readouterr().err
    assert "损坏" in err  # 警告写到 stderr


def test_load_current_goal_all_corrupt_returns_none(tmp_path):
    p = tmp_path / "events.jsonl"
    p.write_text('{ bad }\nnot json\n', encoding="utf-8")
    assert _load_current_goal(str(p)) is None  # 全损坏 → None，不崩溃
