"""线段记录 `segment-second` 与 `second-seq-scan-budget` 的 Python 读取端（#1604，#1586 A-12 / C-04）。

正本是 Rust 侧 `rust/src/theta_v0/policy_records/{segment-second,second-seq-scan-budget}.json`
（Rust 以 `include_str!` 编入）；Python 读同一份文件，不另存副本，故 Rust 与 Python 各级只读这一个
`tangency` 值与这一个 `tail_window` 值（#804）。文件按仓库相对路径定位，Python 包只能在 checkout
内运行；装成独立包后找不到记录即按缺记录报错（不回退）。

缺记录、字段未签（`null` / `"AwaitingPolicy"`）、取值非法一律抛 `SegmentPolicyError` 停止，
不回退任何默认值（#799，ADR 0041）。
"""

from __future__ import annotations

import json
from dataclasses import dataclass
from functools import lru_cache
from pathlib import Path
from typing import Literal

Tangency = Literal["closed", "open"]

RECORD_ID = "segment-second"
RECORDS_DIR = Path(__file__).resolve().parents[2] / "rust" / "src" / "theta_v0" / "policy_records"
RECORD_PATH = RECORDS_DIR / "segment-second.json"
SCAN_BUDGET_ID = "second-seq-scan-budget"
SCAN_BUDGET_PATH = RECORDS_DIR / "second-seq-scan-budget.json"


class SegmentPolicyError(RuntimeError):
    """记录缺失 / 未签 / 非法。"""


@dataclass(frozen=True)
class SegmentConstruction:
    record_id: str
    version: int
    tangency: Tangency

    @property
    def label(self) -> str:
        return f"{self.record_id}/{self.version}"

    def meets(self, lo: float, hi: float) -> bool:
        """区间端点 `lo`、`hi` 是否接触：`closed` 为 `lo <= hi`，`open` 为 `lo < hi`。"""
        return lo <= hi if self.tangency == "closed" else lo < hi

    def separates(self, lo: float, hi: float) -> bool:
        """`lo` 越过 `hi`（缺口）：`closed` 为 `lo > hi`，`open` 为 `lo >= hi`。"""
        return lo > hi if self.tangency == "closed" else lo >= hi


def parse_segment_construction(text: str | None) -> SegmentConstruction:
    """解析记录文本；`None` 表示记录缺失。口径与 Rust `parse_segment_construction` 一致。"""
    if text is None:
        raise SegmentPolicyError(f"缺记录 `{RECORD_ID}`")
    try:
        root = json.loads(text)
    except json.JSONDecodeError as e:
        raise SegmentPolicyError(f"`{RECORD_ID}` 不是合法 JSON：{e}") from e
    if not isinstance(root, dict):
        raise SegmentPolicyError(f"`{RECORD_ID}` 须为 JSON 对象")
    if root.get("id") != RECORD_ID:
        raise SegmentPolicyError(f"记录 id `{root.get('id')}` 与期望 `{RECORD_ID}` 不符")
    version = root.get("version")
    if version is None:
        raise SegmentPolicyError(f"`{RECORD_ID}.version` 未签（AwaitingPolicy）")
    if isinstance(version, bool) or not isinstance(version, int) or version < 1:
        raise SegmentPolicyError(f"`{RECORD_ID}.version` 须为正整数，得 {version!r}")
    fields = root.get("fields")
    if not isinstance(fields, dict):
        raise SegmentPolicyError(f"`{RECORD_ID}.fields` 缺失或非对象")
    entry = fields.get("tangency")
    if entry is None or entry == "AwaitingPolicy":
        raise SegmentPolicyError(f"`{RECORD_ID}.tangency` 未签（AwaitingPolicy）")
    if not isinstance(entry, dict):
        raise SegmentPolicyError(f"`{RECORD_ID}.tangency` 须为带 value 的对象，得 {entry!r}")
    value = entry.get("value")
    if value is None or value == "AwaitingPolicy":
        raise SegmentPolicyError(f"`{RECORD_ID}.tangency` 未签（AwaitingPolicy）")
    if value not in ("closed", "open"):
        raise SegmentPolicyError(f"`{RECORD_ID}.tangency` 须为 closed 或 open，得 {value!r}")
    return SegmentConstruction(record_id=RECORD_ID, version=version, tangency=value)


@lru_cache(maxsize=1)
def active_segment_construction() -> SegmentConstruction:
    """当前生效记录（进程内解析一次）。文件不存在即按缺记录抛错。"""
    text = RECORD_PATH.read_text(encoding="utf-8") if RECORD_PATH.is_file() else None
    return parse_segment_construction(text)


@dataclass(frozen=True)
class ScanBudget:
    """`second-seq-scan-budget` 记录（C-04）。两个窗口 0 = 不设上限。"""

    record_id: str
    version: int
    second_seq_window: int
    tail_window: int

    @property
    def label(self) -> str:
        return f"{self.record_id}/{self.version}"


def _read_nonneg_int(fields: dict, name: str) -> int:
    entry = fields.get(name)
    if entry is None or entry == "AwaitingPolicy":
        raise SegmentPolicyError(f"`{SCAN_BUDGET_ID}.{name}` 未签（AwaitingPolicy）")
    if not isinstance(entry, dict):
        raise SegmentPolicyError(f"`{SCAN_BUDGET_ID}.{name}` 须为带 value 的对象，得 {entry!r}")
    value = entry.get("value")
    if value is None or value == "AwaitingPolicy":
        raise SegmentPolicyError(f"`{SCAN_BUDGET_ID}.{name}` 未签（AwaitingPolicy）")
    if isinstance(value, bool) or not isinstance(value, int) or value < 0 or value > 0xFFFF_FFFF:
        raise SegmentPolicyError(f"`{SCAN_BUDGET_ID}.{name}` 须为非负 u32，得 {value!r}")
    return value


def parse_scan_budget(text: str | None) -> ScanBudget:
    """解析预算记录；口径与 Rust `parse_second_seq_scan_budget` 一致（v1 无 `tail_window`，判非法）。"""
    if text is None:
        raise SegmentPolicyError(f"缺记录 `{SCAN_BUDGET_ID}`")
    try:
        root = json.loads(text)
    except json.JSONDecodeError as e:
        raise SegmentPolicyError(f"`{SCAN_BUDGET_ID}` 不是合法 JSON：{e}") from e
    if not isinstance(root, dict):
        raise SegmentPolicyError(f"`{SCAN_BUDGET_ID}` 须为 JSON 对象")
    if root.get("id") != SCAN_BUDGET_ID:
        raise SegmentPolicyError(f"记录 id `{root.get('id')}` 与期望 `{SCAN_BUDGET_ID}` 不符")
    version = root.get("version")
    if version is None:
        raise SegmentPolicyError(f"`{SCAN_BUDGET_ID}.version` 未签（AwaitingPolicy）")
    if isinstance(version, bool) or not isinstance(version, int) or version < 1:
        raise SegmentPolicyError(f"`{SCAN_BUDGET_ID}.version` 须为正整数，得 {version!r}")
    if version == 1:
        raise SegmentPolicyError(f"`{SCAN_BUDGET_ID}` v1 未承载 tail_window（#1604 起为 v2）")
    fields = root.get("fields")
    if not isinstance(fields, dict):
        raise SegmentPolicyError(f"`{SCAN_BUDGET_ID}.fields` 缺失或非对象")
    return ScanBudget(
        record_id=SCAN_BUDGET_ID,
        version=version,
        second_seq_window=_read_nonneg_int(fields, "second_seq_window"),
        tail_window=_read_nonneg_int(fields, "tail_window"),
    )


@lru_cache(maxsize=1)
def active_scan_budget() -> ScanBudget:
    """当前生效预算记录（进程内解析一次）。文件不存在即按缺记录抛错。"""
    text = SCAN_BUDGET_PATH.read_text(encoding="utf-8") if SCAN_BUDGET_PATH.is_file() else None
    return parse_scan_budget(text)
