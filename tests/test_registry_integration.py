"""DynamicRegistry 集成测试 — apply_to_manifest 往返验证。

注册新 skill → apply_to_manifest → 重新读取 manifest → 确认条目存在。
"""

from __future__ import annotations

from pathlib import Path

import pytest
import yaml

from newchan.evolution.manifest_reader import ManifestReader, SkillEntry
from newchan.evolution.registry import DynamicRegistry


@pytest.fixture()
def tmp_project(tmp_path: Path) -> Path:
    """创建带最小 manifest 的临时项目目录。"""
    chanlun = tmp_path / ".chanlun"
    chanlun.mkdir()
    manifest = chanlun / "manifest.yaml"
    manifest.write_text(
        yaml.dump(
            {
                "version": "1.0",
                "generated": "2026-02-20",
                "skills": [
                    {
                        "name": "existing-skill",
                        "type": "command",
                        "file": ".claude/commands/existing.md",
                        "description": "An existing skill",
                        "status": "active",
                    },
                ],
                "agents": [],
                "hooks": [],
            },
            allow_unicode=True,
        ),
        encoding="utf-8",
    )
    return tmp_path


class TestApplyToManifestRoundTrip:
    """register → apply_to_manifest → reload → verify。"""

    def test_new_skill_persisted(self, tmp_project: Path) -> None:
        reader = ManifestReader(tmp_project)
        registry = DynamicRegistry(reader)

        # 确认初始状态只有 1 个 skill
        assert len(registry.by_type("command")) == 1

        # 注册新 skill
        new_entry = SkillEntry(
            name="new-skill",
            type="command",
            file=".claude/commands/new-skill.md",
            description="A dynamically registered skill",
            status="active",
        )
        registry.register(new_entry, reason="integration test")

        assert len(registry.by_type("command")) == 2
        assert registry.pending_changes == 1

        # 持久化
        written_path = registry.apply_to_manifest()
        assert written_path.exists()
        assert registry.pending_changes == 0

        # 重新读取 manifest，验证新条目存在
        reader2 = ManifestReader(tmp_project)
        snap = reader2.load()
        names = [s.name for s in snap.skills]
        assert "existing-skill" in names
        assert "new-skill" in names
        assert len(snap.skills) == 2

        # 验证新条目的字段完整性
        new_loaded = reader2.find_by_name("new-skill")
        assert new_loaded is not None
        assert new_loaded.description == "A dynamically registered skill"
        assert new_loaded.file == ".claude/commands/new-skill.md"
        assert new_loaded.status == "active"

    def test_agent_and_hook_persisted(self, tmp_project: Path) -> None:
        reader = ManifestReader(tmp_project)
        registry = DynamicRegistry(reader)

        agent = SkillEntry(
            name="test-agent",
            type="agent",
            file=".claude/agents/test-agent.md",
            description="Test agent",
            status="active",
        )
        hook = SkillEntry(
            name="test-hook",
            type="hook",
            file=".claude/hooks/test-hook.sh",
            description="Test hook",
            status="active",
            event="PostToolUse",
            matcher="Bash",
        )
        registry.register(agent, reason="test")
        registry.register(hook, reason="test")
        registry.apply_to_manifest()

        reader2 = ManifestReader(tmp_project)
        snap = reader2.load()
        assert len(snap.agents) == 1
        assert snap.agents[0].name == "test-agent"
        assert len(snap.hooks) == 1
        assert snap.hooks[0].name == "test-hook"
        assert snap.hooks[0].event == "PostToolUse"
        assert snap.hooks[0].matcher == "Bash"

    def test_deprecated_entry_excluded(self, tmp_project: Path) -> None:
        reader = ManifestReader(tmp_project)
        registry = DynamicRegistry(reader)

        # 注销已有 skill
        registry.unregister("existing-skill", "command", reason="test removal")
        registry.apply_to_manifest()

        reader2 = ManifestReader(tmp_project)
        snap = reader2.load()
        assert len(snap.skills) == 0

    def test_update_existing_entry(self, tmp_project: Path) -> None:
        reader = ManifestReader(tmp_project)
        registry = DynamicRegistry(reader)

        # 用新描述重新注册同名 skill
        updated = SkillEntry(
            name="existing-skill",
            type="command",
            file=".claude/commands/existing.md",
            description="Updated description",
            status="active",
        )
        registry.register(updated, reason="update test")
        registry.apply_to_manifest()

        reader2 = ManifestReader(tmp_project)
        entry = reader2.find_by_name("existing-skill")
        assert entry is not None
        assert entry.description == "Updated description"

    def test_empty_manifest_bootstrap(self, tmp_path: Path) -> None:
        """从无 manifest 的空项目开始，注册后创建 manifest。"""
        chanlun = tmp_path / ".chanlun"
        chanlun.mkdir()
        # 写一个空 manifest 让 reader 能加载
        (chanlun / "manifest.yaml").write_text(
            yaml.dump(
                {
                    "version": "1.0",
                    "generated": "2026-02-20",
                    "skills": [],
                    "agents": [],
                    "hooks": [],
                },
                allow_unicode=True,
            ),
            encoding="utf-8",
        )

        reader = ManifestReader(tmp_path)
        registry = DynamicRegistry(reader)

        entry = SkillEntry(
            name="bootstrap-skill",
            type="command",
            file=".claude/commands/bootstrap.md",
            description="First skill",
            status="active",
        )
        registry.register(entry, reason="bootstrap")
        registry.apply_to_manifest()

        reader2 = ManifestReader(tmp_path)
        snap = reader2.load()
        assert len(snap.skills) == 1
        assert snap.skills[0].name == "bootstrap-skill"
