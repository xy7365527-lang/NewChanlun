"""tests 全局 fixture。

#324：真实 relations.jsonl（.chanlun/block-topology/relations.jsonl，125 MB）由
Git LFS 跟踪。CI 的 checkout 不带 LFS 时该路径落下的是 LFS pointer 文本，
文件存在但不是 JSONL ⟹ 只查 exists() 的守卫会放测试进去然后炸
JSONDecodeError。守卫口径：存在 + 首行可解析 JSON 才跑，否则显式 skip
并打印原因（090：不许静默 skip）。
"""

from __future__ import annotations

import json
from pathlib import Path

import pytest

REAL_RELATIONS = Path(".chanlun/block-topology/relations.jsonl")


def real_relations_usable(path: Path) -> tuple[bool, str]:
    """真实 relations.jsonl 可用性：存在 + 首行是可解析 JSON。"""
    if not path.exists():
        return False, f"真实数据文件不存在: {path}"
    try:
        with open(path, encoding="utf-8") as f:
            first = f.readline()
    except OSError as exc:
        return False, f"真实数据文件不可读: {path} ({exc})"
    if not first.strip():
        return False, f"真实数据文件为空: {path}"
    try:
        json.loads(first)
    except json.JSONDecodeError:
        head = first.strip()[:60]
        return False, (
            f"真实数据文件首行不是 JSON（疑 Git LFS pointer 未 smudge）: "
            f"{path} 首行={head!r}"
        )
    return True, ""


@pytest.fixture
def real_relations() -> Path:
    """真实 relations.jsonl 路径；不可用时以具体原因 skip（#324）。"""
    usable, reason = real_relations_usable(REAL_RELATIONS)
    if not usable:
        pytest.skip(reason)
    return REAL_RELATIONS
