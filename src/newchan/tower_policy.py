"""`tower-base` 的 Python 读取端（#1626，qujiantao §14.3）。

Rust 与 Python 读取同一份 `rust/src/theta_v0/policy_records/tower-base.json`。
缺记录、未签或非法值都停止，不回退 `segment`；`kline3` / `trade3` 在当前没有真实构件
与数据 profile 时是 typed reject，不用线段输入冒充。
"""

from __future__ import annotations

import json
from dataclasses import dataclass
from functools import lru_cache
from pathlib import Path
from typing import Literal

TowerBase = Literal["segment", "kline3", "trade3"]
RECORD_ID = "tower-base"
RECORD_PATH = (
    Path(__file__).resolve().parents[2]
    / "rust"
    / "src"
    / "theta_v0"
    / "policy_records"
    / "tower-base.json"
)


class TowerPolicyError(RuntimeError):
    """记录缺失、未签、非法或当前数据面不支持。"""


@dataclass(frozen=True)
class TowerBasePolicy:
    record_id: str
    version: int
    tower_base: TowerBase

    @property
    def label(self) -> str:
        return f"{self.record_id}/{self.version}"

    @property
    def base_level(self) -> int:
        if self.tower_base != "segment":
            raise TowerPolicyError(f"`{self.label}` 的 {self.tower_base} 尚无真实构件与数据 profile")
        return 1


def parse_tower_base(text: str | None) -> TowerBasePolicy:
    if text is None:
        raise TowerPolicyError(f"缺记录 `{RECORD_ID}`")
    try:
        root = json.loads(text)
    except json.JSONDecodeError as e:
        raise TowerPolicyError(f"`{RECORD_ID}` 不是合法 JSON：{e}") from e
    if not isinstance(root, dict):
        raise TowerPolicyError(f"`{RECORD_ID}` 须为 JSON 对象")
    if root.get("id") != RECORD_ID:
        raise TowerPolicyError(f"记录 id `{root.get('id')}` 与期望 `{RECORD_ID}` 不符")
    version = root.get("version")
    if version is None:
        raise TowerPolicyError(f"`{RECORD_ID}.version` 未签（AwaitingPolicy）")
    if isinstance(version, bool) or not isinstance(version, int) or version < 1:
        raise TowerPolicyError(f"`{RECORD_ID}.version` 须为正整数，得 {version!r}")
    fields = root.get("fields")
    if not isinstance(fields, dict):
        raise TowerPolicyError(f"`{RECORD_ID}.fields` 缺失或非对象")
    entry = fields.get("tower_base")
    if entry is None or entry == "AwaitingPolicy":
        raise TowerPolicyError(f"`{RECORD_ID}.tower_base` 未签（AwaitingPolicy）")
    if not isinstance(entry, dict):
        raise TowerPolicyError(f"`{RECORD_ID}.tower_base` 须为带 value 的对象，得 {entry!r}")
    value = entry.get("value")
    if value is None or value == "AwaitingPolicy":
        raise TowerPolicyError(f"`{RECORD_ID}.tower_base` 未签（AwaitingPolicy）")
    if value not in ("segment", "kline3", "trade3"):
        raise TowerPolicyError(f"`{RECORD_ID}.tower_base` 取值非法，得 {value!r}")
    return TowerBasePolicy(record_id=RECORD_ID, version=version, tower_base=value)


@lru_cache(maxsize=1)
def active_tower_base() -> TowerBasePolicy:
    text = RECORD_PATH.read_text(encoding="utf-8") if RECORD_PATH.is_file() else None
    return parse_tower_base(text)
