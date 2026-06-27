"""D′ 波1 T4：ceremony_scan 读 current_goal（roadmap 之前，goal 是当前交易性承诺）。

测试 ceremony_scan.py 输出 JSON 含 current_goal 字段。
无 goal events（events.jsonl 不存在）时 current_goal 为 None——seed bootloader 在 goal 未设定前不阻塞。
"""
import json
import subprocess


ROOT = "/Users/silencehan/Projects/NewChanlun"


def test_scan_reads_current_goal():
    out = subprocess.run(
        ["python", "scripts/ceremony_scan.py"],
        cwd=ROOT, capture_output=True, text=True,
    )
    assert out.returncode == 0, f"scan 退出非0: {out.stderr}"
    data = json.loads(out.stdout)
    assert "current_goal" in data  # 新增字段（无 goal events 时为 None）
