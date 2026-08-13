"""Stop-Guard check3 必须注入 018号四分法，不得注入自创框架。"""

from __future__ import annotations

from pathlib import Path

HOOK = Path(__file__).resolve().parents[1] / ".claude" / "hooks" / "ceremony-completion-guard.sh"


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
