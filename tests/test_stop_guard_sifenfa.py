"""Stop-Guard check3 必须注入 018号四分法，不得注入自创框架。"""

from __future__ import annotations

from pathlib import Path

HOOK = Path(__file__).resolve().parents[1] / ".claude" / "hooks" / "ceremony-completion-guard.sh"
PENDING_483 = (
    Path(__file__).resolve().parents[1]
    / ".chanlun"
    / "genealogy"
    / "pending"
    / "483-meta-observation-v69-swarm.md"
)
SETTLED_483 = (
    Path(__file__).resolve().parents[1]
    / ".chanlun"
    / "genealogy"
    / "settled"
    / "483-meta-observation-v69-swarm.md"
)


def test_check3_injects_018_four_way_classification():
    text = HOOK.read_text(encoding="utf-8")
    assert "018号四分法分类（定理/选择/语法记录/行动）" in text
    # 注入给 agent 的路由不得把非法四分法当作待执行分类
    reason_line = next(
        line for line in text.splitlines()
        if "谱系有" in line and "生成态矛盾待处理" in line
    )
    assert "吸收/修正/分裂/废弃" not in reason_line
    assert "定理/选择/语法记录/行动" in reason_line


def test_483_pending_duplicate_removed_after_theorem_settlement():
    """483 正文自证定理类；pending 副本仅为状态迁移残留。"""
    assert not PENDING_483.exists()
    assert SETTLED_483.exists()
    assert "status: 已结算" in SETTLED_483.read_text(encoding="utf-8")
