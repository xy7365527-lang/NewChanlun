"""Manifest 读取器 — 从 .chanlun/manifest.yaml 读取能力拓扑。

Architect agent 通过此模块获取当前系统的完整能力清单，
作为进化决策的输入。

概念溯源: [新缠论] — 043号自生长回路（manifest 是 ceremony 自动加载的入口）
"""

from __future__ import annotations

import logging
from dataclasses import dataclass
from pathlib import Path
from typing import Literal

import yaml

logger = logging.getLogger(__name__)

EntryType = Literal["command", "agent", "hook"]
EntryStatus = Literal["active", "planned", "deprecated"]


@dataclass(frozen=True, slots=True)
class SkillEntry:
    """manifest 中的单个条目（skill/agent/hook 统一抽象）。"""

    name: str
    type: EntryType
    file: str
    description: str
    status: EntryStatus
    event: str = ""      # hook only
    matcher: str = ""    # hook only


@dataclass(frozen=True, slots=True)
class ManifestSnapshot:
    """manifest 的不可变快照。"""

    version: str
    generated: str
    skills: tuple[SkillEntry, ...]
    agents: tuple[SkillEntry, ...]
    hooks: tuple[SkillEntry, ...]


class ManifestReader:
    """从 manifest.yaml 读取并缓存能力拓扑。

    Parameters
    ----------
    project_root : Path | str
        项目根目录（包含 .chanlun/ 的目录）。
    """

    def __init__(self, project_root: Path | str) -> None:
        self._root = Path(project_root)
        self._manifest_path = self._root / ".chanlun" / "manifest.yaml"
        self._snapshot: ManifestSnapshot | None = None

    def load(self) -> ManifestSnapshot:
        """读取 manifest.yaml 并返回不可变快照。每次调用重新读取。"""
        if not self._manifest_path.exists():
            raise FileNotFoundError(
                f"manifest.yaml 不存在: {self._manifest_path}"
            )

        raw = yaml.safe_load(self._manifest_path.read_text(encoding="utf-8"))
        snapshot = ManifestSnapshot(
            version=str(raw.get("version", "0.0")),
            generated=str(raw.get("generated", "")),
            skills=tuple(
                _parse_entry(e, "command")
                for e in raw.get("skills", [])
            ),
            agents=tuple(
                _parse_entry(e, "agent")
                for e in raw.get("agents", [])
            ),
            hooks=tuple(
                _parse_entry(e, "hook")
                for e in raw.get("hooks", [])
            ),
        )
        self._snapshot = snapshot
        logger.info(
            "manifest loaded: %d skills, %d agents, %d hooks",
            len(snapshot.skills), len(snapshot.agents), len(snapshot.hooks),
        )
        return snapshot

    @property
    def snapshot(self) -> ManifestSnapshot:
        """返回缓存的快照。未加载时自动加载。"""
        if self._snapshot is None:
            return self.load()
        return self._snapshot

    def find_by_name(self, name: str) -> SkillEntry | None:
        """按名称查找条目（跨 skills/agents/hooks）。"""
        snap = self.snapshot
        for entry in (*snap.skills, *snap.agents, *snap.hooks):
            if entry.name == name:
                return entry
        return None

    def active_entries(self) -> tuple[SkillEntry, ...]:
        """返回所有 status=active 的条目。"""
        snap = self.snapshot
        return tuple(
            e for e in (*snap.skills, *snap.agents, *snap.hooks)
            if e.status == "active"
        )

    def topology_summary(self) -> str:
        """生成能力拓扑的文本摘要（供 Gemini decide() 使用）。"""
        snap = self.snapshot
        lines = [
            f"Manifest v{snap.version} ({snap.generated})",
            f"Skills: {len(snap.skills)} | Agents: {len(snap.agents)} | Hooks: {len(snap.hooks)}",
            "",
            "Active skills:",
        ]
        for s in snap.skills:
            if s.status == "active":
                lines.append(f"  - {s.name}: {s.description}")
        lines.append("\nActive agents:")
        for a in snap.agents:
            if a.status == "active":
                lines.append(f"  - {a.name}: {a.description}")
        lines.append("\nActive hooks:")
        for h in snap.hooks:
            if h.status == "active":
                lines.append(
                    f"  - {h.name} [{h.event}:{h.matcher or '*'}]: {h.description}"
                )
        return "\n".join(lines)


def _parse_entry(raw: dict, default_type: EntryType) -> SkillEntry:
    """从 YAML dict 解析单个条目。"""
    return SkillEntry(
        name=str(raw.get("name", "")),
        type=raw.get("type", default_type),
        file=str(raw.get("file", "")),
        description=str(raw.get("description", "")),
        status=raw.get("status", "active"),
        event=str(raw.get("event", "")),
        matcher=str(raw.get("matcher", "")),
    )
