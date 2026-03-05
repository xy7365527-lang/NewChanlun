"""tests for scripts/gangmu_inject.py — 纲目自动注入工具。"""
from __future__ import annotations

import copy
import json
import os

import pytest
import yaml

from scripts.gangmu_inject import (
    InjectReport,
    find_existing_sources,
    inject_action,
    inject_all,
    load_gangmu,
    save_gangmu,
    _dedup_key,
    _make_target,
    _find_active_mu_in_gang,
    _find_any_active_mu,
)


# ═══════════════════════════════════════════════════════════════
# Fixtures
# ═══════════════════════════════════════════════════════════════

MINIMAL_GANGMU = {
    "version": "1.0",
    "gang": [
        {
            "id": "gang-a",
            "name": "纲A",
            "mu": [
                {
                    "id": "mu-active",
                    "name": "活跃目",
                    "status": "active",
                    "opened_at": "100",
                    "next_actions": [
                        {
                            "type": "engineering",
                            "target": "existing-action",
                            "description": "已有动作",
                            "blocked_by": None,
                            "completion_check": {
                                "type": "file_exists",
                                "path": "scripts/test.py",
                            },
                        },
                    ],
                },
                {
                    "id": "mu-closed",
                    "name": "已关闭目",
                    "status": "closed",
                    "next_actions": [],
                },
            ],
        },
        {
            "id": "gang-b",
            "name": "纲B",
            "mu": [
                {
                    "id": "mu-b-active",
                    "name": "B纲活跃目",
                    "status": "active",
                    "opened_at": "200",
                    "next_actions": [],
                },
            ],
        },
    ],
}


def _proposal(source="360号", field="downstream_implications[1]",
              text="测试推论文本", coverage="not_covered",
              gang="gang-a"):
    """构造一条 proposed_new_mu。"""
    return {
        "source": source,
        "field": field,
        "text": text,
        "coverage_status": coverage,
        "suggested_gang": gang,
    }


@pytest.fixture
def gangmu():
    """返回深拷贝的测试纲目。"""
    return copy.deepcopy(MINIMAL_GANGMU)


@pytest.fixture
def gangmu_file(tmp_path, gangmu):
    """写入临时 gangmu.yaml 并返回路径。"""
    path = tmp_path / ".chanlun" / "gangmu.yaml"
    path.parent.mkdir(parents=True)
    with open(path, "w", encoding="utf-8") as f:
        yaml.dump(gangmu, f, allow_unicode=True, default_flow_style=False)
    return str(path)


# ═══════════════════════════════════════════════════════════════
# _dedup_key / _make_target
# ═══════════════════════════════════════════════════════════════

class TestHelpers:
    def test_dedup_key(self):
        assert _dedup_key("360号", "downstream_implications[1]") == "360号:downstream_implications[1]"

    def test_make_target_standard(self):
        assert _make_target("360号", "downstream_implications[2]") == "genealogy-360-di2"

    def test_make_target_no_index(self):
        assert _make_target("100号", "some_field") == "genealogy-100-di0"

    def test_make_target_bare_number(self):
        assert _make_target("42", "downstream_implications[5]") == "genealogy-42-di5"


# ═══════════════════════════════════════════════════════════════
# find_existing_sources
# ═══════════════════════════════════════════════════════════════

class TestFindExistingSources:
    def test_finds_targets(self, gangmu):
        sources = find_existing_sources(gangmu)
        assert "existing-action" in sources

    def test_finds_injected_from(self):
        data = copy.deepcopy(MINIMAL_GANGMU)
        data["gang"][0]["mu"][0]["next_actions"].append({
            "type": "engineering",
            "target": "genealogy-360-di1",
            "injected_from": "360号:downstream_implications[1]",
        })
        sources = find_existing_sources(data)
        assert "360号:downstream_implications[1]" in sources
        assert "genealogy-360-di1" in sources

    def test_empty_gangmu(self):
        assert find_existing_sources({}) == set()
        assert find_existing_sources({"gang": []}) == set()


# ═══════════════════════════════════════════════════════════════
# _find_active_mu_in_gang
# ═══════════════════════════════════════════════════════════════

class TestFindActiveMu:
    def test_finds_active_in_target_gang(self, gangmu):
        gang, mu = _find_active_mu_in_gang(gangmu, "gang-a")
        assert gang["id"] == "gang-a"
        assert mu["id"] == "mu-active"

    def test_finds_active_in_gang_b(self, gangmu):
        gang, mu = _find_active_mu_in_gang(gangmu, "gang-b")
        assert gang["id"] == "gang-b"
        assert mu["id"] == "mu-b-active"

    def test_no_active_in_nonexistent_gang(self, gangmu):
        gang, mu = _find_active_mu_in_gang(gangmu, "nonexistent")
        assert gang is None
        assert mu is None

    def test_fallback_finds_any_active(self, gangmu):
        gang, mu = _find_any_active_mu(gangmu)
        assert gang is not None
        assert mu["status"] == "active"


# ═══════════════════════════════════════════════════════════════
# inject_action — 单条注入
# ═══════════════════════════════════════════════════════════════

class TestInjectAction:
    def test_inject_single(self, gangmu):
        proposal = _proposal()
        new_gangmu, result = inject_action(gangmu, proposal)
        assert result[0] == "injected"
        _, gang_id, mu_id, target = result
        assert gang_id == "gang-a"
        assert mu_id == "mu-active"
        assert target == "genealogy-360-di1"

        # 检查注入后的 action
        actions = new_gangmu["gang"][0]["mu"][0]["next_actions"]
        injected = [a for a in actions if a["target"] == "genealogy-360-di1"]
        assert len(injected) == 1
        assert "360号" in injected[0]["description"]
        assert injected[0]["injected_from"] == "360号:downstream_implications[1]"

    def test_immutability(self, gangmu):
        """inject_action 不修改输入。"""
        original_len = len(gangmu["gang"][0]["mu"][0]["next_actions"])
        new_gangmu, _ = inject_action(gangmu, _proposal())
        assert len(gangmu["gang"][0]["mu"][0]["next_actions"]) == original_len
        assert len(new_gangmu["gang"][0]["mu"][0]["next_actions"]) == original_len + 1

    def test_skip_covered(self, gangmu):
        proposal = _proposal(coverage="covered")
        new_gangmu, result = inject_action(gangmu, proposal)
        assert result[0] == "skipped"
        assert new_gangmu is gangmu  # 未修改，返回同一对象

    def test_skip_duplicate_by_target(self, gangmu):
        """注入一次后再注入相同 proposal，应跳过。"""
        new_gangmu, result1 = inject_action(gangmu, _proposal())
        assert result1[0] == "injected"

        new_gangmu2, result2 = inject_action(new_gangmu, _proposal())
        assert result2[0] == "skipped"
        assert "已存在" in result2[3]

    def test_skip_duplicate_by_injected_from(self, gangmu):
        """已有 injected_from 标记的条目不重复注入。"""
        gangmu["gang"][0]["mu"][0]["next_actions"].append({
            "type": "engineering",
            "target": "genealogy-360-di1",
            "injected_from": "360号:downstream_implications[1]",
        })
        _, result = inject_action(gangmu, _proposal())
        assert result[0] == "skipped"

    def test_inject_to_suggested_gang(self, gangmu):
        """注入到 suggested_gang 对应的 active 目。"""
        proposal = _proposal(gang="gang-b")
        _, result = inject_action(gangmu, proposal)
        assert result[0] == "injected"
        assert result[1] == "gang-b"
        assert result[2] == "mu-b-active"

    def test_fallback_when_no_active_in_gang(self, gangmu):
        """suggested_gang 中没有 active 目时 fallback。"""
        proposal = _proposal(gang="nonexistent-gang")
        _, result = inject_action(gangmu, proposal)
        assert result[0] == "injected"
        # 应该 fallback 到某个 active 目
        assert result[1] in ("gang-a", "gang-b")

    def test_error_when_no_active_mu(self):
        """所有目都 closed 时返回 error。"""
        data = {
            "version": "1.0",
            "gang": [{"id": "g", "mu": [{"id": "m", "status": "closed", "next_actions": []}]}],
        }
        _, result = inject_action(data, _proposal())
        assert result[0] == "error"
        assert "无 active 目" in result[3]

    def test_error_missing_source(self, gangmu):
        proposal = _proposal(source="", field="")
        _, result = inject_action(gangmu, proposal)
        assert result[0] == "error"


# ═══════════════════════════════════════════════════════════════
# inject_all — 批量注入
# ═══════════════════════════════════════════════════════════════

class TestInjectAll:
    def test_inject_multiple(self, gangmu):
        proposals = [
            _proposal(source="360号", field="downstream_implications[0]"),
            _proposal(source="360号", field="downstream_implications[1]"),
            _proposal(source="361号", field="downstream_implications[0]"),
        ]
        new_gangmu, report = inject_all(gangmu, proposals)
        assert len(report.injected) == 3
        assert len(report.skipped) == 0
        assert len(report.errors) == 0

    def test_mixed_results(self, gangmu):
        proposals = [
            _proposal(source="360号", field="downstream_implications[0]"),
            _proposal(source="360号", field="downstream_implications[0]"),  # duplicate
            _proposal(source="", field=""),  # error
            _proposal(coverage="covered"),  # skipped
        ]
        _, report = inject_all(gangmu, proposals)
        assert len(report.injected) == 1
        assert len(report.skipped) == 2  # duplicate + covered
        assert len(report.errors) == 1

    def test_empty_proposals(self, gangmu):
        new_gangmu, report = inject_all(gangmu, [])
        assert len(report.injected) == 0
        assert new_gangmu is gangmu  # 无修改

    def test_immutability_of_original(self, gangmu):
        original = copy.deepcopy(gangmu)
        proposals = [_proposal()]
        new_gangmu, _ = inject_all(gangmu, proposals)
        assert gangmu == original  # 原始未变
        assert new_gangmu != original  # 新的有变化

    def test_report_to_dict(self, gangmu):
        proposals = [_proposal()]
        _, report = inject_all(gangmu, proposals)
        d = report.to_dict()
        assert d["injected_count"] == 1
        assert d["skipped_count"] == 0
        assert d["error_count"] == 0
        assert isinstance(d["injected"], list)
        assert d["injected"][0]["target"] == "genealogy-360-di1"


# ═══════════════════════════════════════════════════════════════
# load_gangmu / save_gangmu — 文件 I/O
# ═══════════════════════════════════════════════════════════════

class TestFileIO:
    def test_load_roundtrip(self, gangmu_file):
        data = load_gangmu(gangmu_file)
        assert data["version"] == "1.0"
        assert len(data["gang"]) == 2

    def test_save_roundtrip(self, gangmu_file, gangmu):
        gangmu["gang"].append({"id": "new-gang", "mu": []})
        save_gangmu(gangmu_file, gangmu)
        reloaded = load_gangmu(gangmu_file)
        assert len(reloaded["gang"]) == 3

    def test_load_nonexistent(self, tmp_path):
        with pytest.raises(FileNotFoundError):
            load_gangmu(str(tmp_path / "nonexistent.yaml"))

    def test_save_preserves_header(self, gangmu_file, gangmu):
        save_gangmu(gangmu_file, gangmu)
        with open(gangmu_file, encoding="utf-8") as f:
            content = f.read()
        assert "# .chanlun/gangmu.yaml" in content
        assert "# 纲目——总方针的展开结构" in content


# ═══════════════════════════════════════════════════════════════
# CLI integration (end-to-end)
# ═══════════════════════════════════════════════════════════════

class TestCLI:
    def test_json_input(self, tmp_path, gangmu):
        """--json 直接传入 proposals。"""
        gm_path = tmp_path / ".chanlun" / "gangmu.yaml"
        gm_path.parent.mkdir(parents=True)
        with open(gm_path, "w", encoding="utf-8") as f:
            yaml.dump(gangmu, f, allow_unicode=True, default_flow_style=False)

        proposals = [_proposal()]
        import subprocess
        result = subprocess.run(
            ["python", "scripts/gangmu_inject.py",
             "--root", str(tmp_path),
             "--json", json.dumps(proposals, ensure_ascii=False)],
            capture_output=True, text=True,
            cwd=os.path.join(os.path.dirname(__file__), ".."),
        )
        assert result.returncode == 0, result.stderr
        output = json.loads(result.stdout)
        assert output["injected_count"] == 1

    def test_dry_run(self, tmp_path, gangmu):
        """--dry-run 不写入文件。"""
        gm_path = tmp_path / ".chanlun" / "gangmu.yaml"
        gm_path.parent.mkdir(parents=True)
        with open(gm_path, "w", encoding="utf-8") as f:
            yaml.dump(gangmu, f, allow_unicode=True, default_flow_style=False)

        original_content = gm_path.read_text(encoding="utf-8")
        proposals = [_proposal()]
        import subprocess
        result = subprocess.run(
            ["python", "scripts/gangmu_inject.py",
             "--root", str(tmp_path),
             "--dry-run",
             "--json", json.dumps(proposals, ensure_ascii=False)],
            capture_output=True, text=True,
            cwd=os.path.join(os.path.dirname(__file__), ".."),
        )
        assert result.returncode == 0, result.stderr
        output = json.loads(result.stdout)
        assert output["dry_run"] is True
        assert output["injected_count"] == 1
        # 文件未变
        assert gm_path.read_text(encoding="utf-8") == original_content

    def test_no_proposals(self, tmp_path, gangmu):
        """空 proposals 输出 no_proposals。"""
        gm_path = tmp_path / ".chanlun" / "gangmu.yaml"
        gm_path.parent.mkdir(parents=True)
        with open(gm_path, "w", encoding="utf-8") as f:
            yaml.dump(gangmu, f, allow_unicode=True, default_flow_style=False)

        import subprocess
        result = subprocess.run(
            ["python", "scripts/gangmu_inject.py",
             "--root", str(tmp_path),
             "--json", "[]"],
            capture_output=True, text=True,
            cwd=os.path.join(os.path.dirname(__file__), ".."),
        )
        assert result.returncode == 0, result.stderr
        output = json.loads(result.stdout)
        assert output["status"] == "no_proposals"

    def test_scan_output_format(self, tmp_path, gangmu):
        """接受完整 ceremony_scan 输出格式（dict with proposed_new_mu key）。"""
        gm_path = tmp_path / ".chanlun" / "gangmu.yaml"
        gm_path.parent.mkdir(parents=True)
        with open(gm_path, "w", encoding="utf-8") as f:
            yaml.dump(gangmu, f, allow_unicode=True, default_flow_style=False)

        scan_output = {
            "workstations": [],
            "proposed_new_mu": [_proposal()],
        }
        import subprocess
        result = subprocess.run(
            ["python", "scripts/gangmu_inject.py",
             "--root", str(tmp_path),
             "--json", json.dumps(scan_output, ensure_ascii=False)],
            capture_output=True, text=True,
            cwd=os.path.join(os.path.dirname(__file__), ".."),
        )
        assert result.returncode == 0, result.stderr
        output = json.loads(result.stdout)
        assert output["injected_count"] == 1
