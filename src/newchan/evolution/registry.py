"""DynamicRegistry — skill/agent/hook 的运行时注册与发现。

manifest.yaml 是持久化层，DynamicRegistry 是运行时层。
注册操作先写入 registry，再由 apply 方法同步到 manifest.yaml。

概念溯源: [新缠论] — 043号自生长回路（动态 manifest 必须 git 版本控制）
"""

from __future__ import annotations

import logging
from dataclasses import dataclass, replace
from pathlib import Path
from typing import Literal

import yaml

from newchan.evolution.manifest_reader import (
    EntryStatus,
    EntryType,
    ManifestReader,
    ManifestSnapshot,
    SkillEntry,
)

logger = logging.getLogger(__name__)

RegistryAction = Literal["register", "update", "unregister"]


@dataclass(frozen=True, slots=True)
class RegistryEvent:
    """注册表变更事件（不可变，用于审计追踪）。"""

    action: RegistryAction
    entry: SkillEntry
    reason: str = ""


class DynamicRegistry:
    """运行时能力注册表。

    从 ManifestReader 加载初始状态，支持运行时注册/注销，
    最终通过 apply_to_manifest() 持久化到 manifest.yaml。

    Parameters
    ----------
    reader : ManifestReader
        manifest 读取器（提供初始状态）。
    """

    def __init__(self, reader: ManifestReader) -> None:
        self._reader = reader
        self._entries: dict[str, SkillEntry] = {}
        self._events: list[RegistryEvent] = []
        self._load_from_manifest()

    def _load_from_manifest(self) -> None:
        """从 manifest 加载初始状态。"""
        try:
            snap = self._reader.load()
        except FileNotFoundError:
            logger.warning("manifest.yaml 不存在，registry 以空状态启动")
            return

        for entry in (*snap.skills, *snap.agents, *snap.hooks):
            self._entries[_key(entry)] = entry

    def register(
        self,
        entry: SkillEntry,
        reason: str = "",
    ) -> SkillEntry:
        """注册新条目或更新已有条目。返回注册后的条目。"""
        key = _key(entry)
        existing = self._entries.get(key)

        if existing is not None:
            action: RegistryAction = "update"
        else:
            action = "register"

        self._entries[key] = entry
        self._events.append(RegistryEvent(
            action=action, entry=entry, reason=reason,
        ))
        logger.info(
            "registry %s: %s/%s (%s)",
            action, entry.type, entry.name, reason or "no reason",
        )
        return entry

    def unregister(
        self,
        name: str,
        entry_type: EntryType,
        reason: str = "",
    ) -> SkillEntry | None:
        """注销条目。返回被注销的条目，不存在时返回 None。"""
        key = f"{entry_type}:{name}"
        entry = self._entries.pop(key, None)
        if entry is not None:
            # 标记为 deprecated 而非直接删除（保留审计追踪）
            deprecated = replace(entry, status="deprecated")
            self._events.append(RegistryEvent(
                action="unregister", entry=deprecated, reason=reason,
            ))
            logger.info(
                "registry unregister: %s/%s (%s)",
                entry_type, name, reason or "no reason",
            )
        return entry

    def lookup(self, name: str) -> SkillEntry | None:
        """按名称查找（跨类型）。"""
        for key, entry in self._entries.items():
            if entry.name == name:
                return entry
        return None

    def lookup_typed(self, name: str, entry_type: EntryType) -> SkillEntry | None:
        """按名称和类型查找。"""
        return self._entries.get(f"{entry_type}:{name}")

    def all_entries(self) -> tuple[SkillEntry, ...]:
        """返回所有条目。"""
        return tuple(self._entries.values())

    def by_type(self, entry_type: EntryType) -> tuple[SkillEntry, ...]:
        """按类型过滤。"""
        return tuple(
            e for e in self._entries.values() if e.type == entry_type
        )

    def by_status(self, status: EntryStatus) -> tuple[SkillEntry, ...]:
        """按状态过滤。"""
        return tuple(
            e for e in self._entries.values() if e.status == status
        )

    @property
    def events(self) -> tuple[RegistryEvent, ...]:
        """返回所有变更事件（审计追踪）。"""
        return tuple(self._events)

    @property
    def pending_changes(self) -> int:
        """未持久化的变更数量。"""
        return len(self._events)

    def _collect_active_entries(self) -> dict[str, list[SkillEntry]]:
        """按类型收集非 deprecated 条目，排序后返回。"""
        result: dict[str, list[SkillEntry]] = {
            "command": [], "agent": [], "hook": [],
        }
        for e in self._entries.values():
            if e.status != "deprecated" and e.type in result:
                result[e.type].append(e)
        for lst in result.values():
            lst.sort(key=lambda e: e.name)
        return result

    def _build_manifest_content(
        self, entries: dict[str, list[SkillEntry]],
    ) -> str:
        """将条目构建为 YAML 字符串。"""
        snap = self._reader.snapshot
        data = {
            "version": snap.version,
            "generated": snap.generated,
            "skills": [_entry_to_dict(e) for e in entries["command"]],
            "agents": [_entry_to_dict(e) for e in entries["agent"]],
            "hooks": [_entry_to_dict(e) for e in entries["hook"]],
        }
        header = (
            "# USM — Unified Skill Manifest\n"
            "# Auto-generated by DynamicRegistry.apply_to_manifest()\n"
            "# Do not edit manually.\n"
        )
        return header + yaml.dump(
            data, default_flow_style=False,
            allow_unicode=True, sort_keys=False,
        )

    def apply_to_manifest(self, manifest_path: Path | None = None) -> Path:
        """将当前 registry 状态写入 manifest.yaml。

        Returns 写入的文件路径。
        """
        path = manifest_path or (
            self._reader._root / ".chanlun" / "manifest.yaml"
        )

        entries = self._collect_active_entries()
        content = self._build_manifest_content(entries)

        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(content, encoding="utf-8")

        logger.info(
            "manifest written: %d skills, %d agents, %d hooks → %s",
            len(entries["command"]), len(entries["agent"]),
            len(entries["hook"]), path,
        )
        self._events.clear()
        return path


def _key(entry: SkillEntry) -> str:
    """生成 registry 内部键。"""
    return f"{entry.type}:{entry.name}"


def _entry_to_dict(entry: SkillEntry) -> dict:
    """将 SkillEntry 转为 YAML-serializable dict。"""
    d: dict = {
        "name": entry.name,
        "type": entry.type,
        "file": entry.file,
        "description": entry.description,
        "status": entry.status,
    }
    if entry.type == "hook":
        d["event"] = entry.event
        d["matcher"] = entry.matcher
    return d
