"""tests/test_gangju_analysis.py — gangju_analysis 的单元测试。

覆盖：
  - compute_residue_status 三态检测（183号-3）
  - check_residue_empty 向后兼容
  - _block_has_substantive_concessions 辅助函数
  - generate_pending_skeleton 骨架生成（187号目E）
  - _next_genealogy_id 编号递增
  - derive_mu 返回 residue_status
"""
import json
import os
import re
import tempfile

import pytest

from scripts.gangju_analysis import (
    _block_has_substantive_concessions,
    _next_genealogy_id,
    check_residue_empty,
    compute_residue_status,
    derive_mu,
    generate_pending_skeleton,
)


# ---------------------------------------------------------------------------
# Fixtures
# ---------------------------------------------------------------------------

@pytest.fixture
def tmp_root(tmp_path):
    """创建具有基本目录结构的临时项目根目录。"""
    blocks_dir = tmp_path / ".chanlun" / "block-topology" / "blocks"
    blocks_dir.mkdir(parents=True)
    (tmp_path / ".chanlun" / "genealogy" / "settled").mkdir(parents=True)
    (tmp_path / ".chanlun" / "genealogy" / "pending").mkdir(parents=True)
    return tmp_path


def _write_block(blocks_dir, block_id, block_type, content):
    """辅助：写入一个区块 JSON 文件。"""
    block = {
        "id": block_id,
        "type": block_type,
        "timestamp": "2026-02-24T00:00:00+00:00",
        "source": "test",
        "content": content,
        "refs": [],
        "git_ref": "",
    }
    fp = blocks_dir / f"{block_id}.json"
    fp.write_text(json.dumps(block, ensure_ascii=False), encoding="utf-8")


# ---------------------------------------------------------------------------
# _block_has_substantive_concessions
# ---------------------------------------------------------------------------

class TestBlockHasSubstantiveConcessions:
    def test_empty_content(self):
        assert not _block_has_substantive_concessions({})

    def test_empty_lists(self):
        content = {"gemini_conceded": [], "codex_conceded": [], "reasons": {}}
        assert not _block_has_substantive_concessions(content)

    def test_gemini_conceded_nonempty(self):
        content = {"gemini_conceded": ["item1"], "codex_conceded": [], "reasons": {}}
        assert _block_has_substantive_concessions(content)

    def test_codex_conceded_nonempty(self):
        content = {"gemini_conceded": [], "codex_conceded": ["item1"], "reasons": {}}
        assert _block_has_substantive_concessions(content)

    def test_reasons_nonempty(self):
        content = {"gemini_conceded": [], "codex_conceded": [], "reasons": {"a": "reason"}}
        assert _block_has_substantive_concessions(content)

    def test_reasons_all_falsy(self):
        content = {"gemini_conceded": [], "codex_conceded": [], "reasons": {"a": "", "b": None}}
        assert not _block_has_substantive_concessions(content)


# ---------------------------------------------------------------------------
# compute_residue_status
# ---------------------------------------------------------------------------

class TestComputeResidueStatus:
    def test_no_blocks_dir(self, tmp_path):
        """无 block-topology 目录 → no_residue。"""
        assert compute_residue_status(str(tmp_path)) == "no_residue"

    def test_no_residue_blocks(self, tmp_root):
        """有区块目录但无 residue 类型区块 → no_residue。"""
        blocks_dir = tmp_root / ".chanlun" / "block-topology" / "blocks"
        _write_block(blocks_dir, "aaa", "consensus", {"conclusion": "test"})
        assert compute_residue_status(str(tmp_root)) == "no_residue"

    def test_empty_shell(self, tmp_root):
        """residue 区块存在但让步内容全为空 → empty_shell。"""
        blocks_dir = tmp_root / ".chanlun" / "block-topology" / "blocks"
        _write_block(
            blocks_dir, "bbb", "residue",
            {"gemini_conceded": [], "codex_conceded": [], "reasons": {}},
        )
        assert compute_residue_status(str(tmp_root)) == "empty_shell"

    def test_substantive_via_residue(self, tmp_root):
        """residue 区块包含非空让步 → substantive。"""
        blocks_dir = tmp_root / ".chanlun" / "block-topology" / "blocks"
        _write_block(
            blocks_dir, "ccc", "residue",
            {"gemini_conceded": ["concession1"], "codex_conceded": [], "reasons": {}},
        )
        assert compute_residue_status(str(tmp_root)) == "substantive"

    def test_substantive_via_consensus_conceded(self, tmp_root):
        """consensus 区块携带 conceded 字段且非空 → substantive。"""
        blocks_dir = tmp_root / ".chanlun" / "block-topology" / "blocks"
        _write_block(
            blocks_dir, "ddd", "consensus",
            {
                "conclusion": "test",
                "gemini_conceded": ["item"],
                "codex_conceded": [],
                "reasons": {},
            },
        )
        assert compute_residue_status(str(tmp_root)) == "substantive"

    def test_consensus_with_empty_conceded_no_residue_block(self, tmp_root):
        """consensus 有 conceded 字段但全空，无 residue 区块 → no_residue。

        空列表 conceded 不构成 residue 信号——只有非空 conceded 才表明 consensus 携带让步。
        """
        blocks_dir = tmp_root / ".chanlun" / "block-topology" / "blocks"
        _write_block(
            blocks_dir, "eee", "consensus",
            {
                "conclusion": "test",
                "gemini_conceded": [],
                "codex_conceded": [],
                "reasons": {},
            },
        )
        assert compute_residue_status(str(tmp_root)) == "no_residue"

    def test_mixed_blocks_first_substantive_wins(self, tmp_root):
        """多个 residue 区块，其中一个有实质内容 → substantive。"""
        blocks_dir = tmp_root / ".chanlun" / "block-topology" / "blocks"
        _write_block(
            blocks_dir, "fff", "residue",
            {"gemini_conceded": [], "codex_conceded": [], "reasons": {}},
        )
        _write_block(
            blocks_dir, "ggg", "residue",
            {"gemini_conceded": [], "codex_conceded": ["yes"], "reasons": {}},
        )
        assert compute_residue_status(str(tmp_root)) == "substantive"


# ---------------------------------------------------------------------------
# check_residue_empty（向后兼容）
# ---------------------------------------------------------------------------

class TestCheckResidueEmpty:
    def test_empty_returns_true(self, tmp_root):
        blocks_dir = tmp_root / ".chanlun" / "block-topology" / "blocks"
        _write_block(
            blocks_dir, "hhh", "residue",
            {"gemini_conceded": [], "codex_conceded": [], "reasons": {}},
        )
        assert check_residue_empty(str(tmp_root)) is True

    def test_substantive_returns_false(self, tmp_root):
        blocks_dir = tmp_root / ".chanlun" / "block-topology" / "blocks"
        _write_block(
            blocks_dir, "iii", "residue",
            {"gemini_conceded": ["x"], "codex_conceded": [], "reasons": {}},
        )
        assert check_residue_empty(str(tmp_root)) is False


# ---------------------------------------------------------------------------
# _next_genealogy_id
# ---------------------------------------------------------------------------

class TestNextGenealogyId:
    def test_empty_dirs(self, tmp_root):
        """空目录 → ID 为 1。"""
        assert _next_genealogy_id(str(tmp_root)) == 1

    def test_with_existing_files(self, tmp_root):
        """有已有文件 → max(id) + 1。"""
        settled_dir = tmp_root / ".chanlun" / "genealogy" / "settled"
        (settled_dir / "042-test.md").write_text("test", encoding="utf-8")
        (settled_dir / "187-other.md").write_text("test", encoding="utf-8")
        pending_dir = tmp_root / ".chanlun" / "genealogy" / "pending"
        (pending_dir / "188-pending.md").write_text("test", encoding="utf-8")
        assert _next_genealogy_id(str(tmp_root)) == 189


# ---------------------------------------------------------------------------
# generate_pending_skeleton
# ---------------------------------------------------------------------------

class TestGeneratePendingSkeleton:
    def test_empty_new_mu_returns_none(self, tmp_root):
        assert generate_pending_skeleton(str(tmp_root), []) is None

    def test_generates_file(self, tmp_root):
        new_mu = [
            {"mu": "多轮质询管道", "reason": "residue 内容全为空"},
            {"mu": "异步自指实现", "reason": "无相关区块"},
        ]
        result = generate_pending_skeleton(str(tmp_root), new_mu)
        assert result is not None
        assert os.path.isfile(result)

        # 验证文件名格式
        basename = os.path.basename(result)
        assert re.match(r"^\d{3}-gangju-auto-\d{8}-\d{4}\.md$", basename)

        # 验证文件内容
        content = open(result, encoding="utf-8").read()
        assert "多轮质询管道" in content
        assert "异步自指实现" in content
        assert "status: 生成态" in content
        assert "source: gangju_analysis.py" in content
        assert "待多轮质询填充" in content

    def test_id_increments_from_existing(self, tmp_root):
        """生成的 ID 应该是现有最大 ID + 1。"""
        settled_dir = tmp_root / ".chanlun" / "genealogy" / "settled"
        (settled_dir / "200-test.md").write_text("test", encoding="utf-8")

        new_mu = [{"mu": "测试目", "reason": "测试原因"}]
        result = generate_pending_skeleton(str(tmp_root), new_mu)
        basename = os.path.basename(result)
        assert basename.startswith("201-")

    def test_frontmatter_parseable(self, tmp_root):
        """生成的 frontmatter 应该是合法 YAML。"""
        import yaml

        new_mu = [{"mu": "测试目", "reason": "测试原因"}]
        result = generate_pending_skeleton(str(tmp_root), new_mu)
        content = open(result, encoding="utf-8").read()
        fm_match = re.match(r"^---\s*\n(.+?)\n---", content, re.DOTALL)
        assert fm_match is not None
        fm = yaml.safe_load(fm_match.group(1))
        assert fm["type"] == "待定"
        assert fm["status"] == "生成态"
        assert "audit_targets" in fm


# ---------------------------------------------------------------------------
# derive_mu 返回 residue_status
# ---------------------------------------------------------------------------

class TestDeriveMuResidueStatus:
    def test_returns_four_values(self, tmp_root):
        """derive_mu 现在返回四元组，第四个是 residue_status。"""
        block_stats = {
            "type_counts": {},
            "delta_blocks": 0,
            "migration_block_count": 0,
            "total_blocks": 0,
            "relation_counts": {},
        }
        genealogy_stats = {
            "recent_type_distribution": {},
            "pending_count": 0,
            "settled_count": 0,
        }
        result = derive_mu(block_stats, genealogy_stats, str(tmp_root))
        assert len(result) == 4
        filled_mu, empty_mu, new_mu, residue_status = result
        assert residue_status in ("no_residue", "empty_shell", "substantive")

    def test_residue_status_propagated(self, tmp_root):
        """有 residue 空壳时 residue_status 应为 empty_shell。"""
        blocks_dir = tmp_root / ".chanlun" / "block-topology" / "blocks"
        _write_block(
            blocks_dir, "jjj", "residue",
            {"gemini_conceded": [], "codex_conceded": [], "reasons": {}},
        )
        _write_block(
            blocks_dir, "kkk", "consensus",
            {"conclusion": "test"},
        )

        block_stats = {
            "type_counts": {"consensus": 1, "residue": 1},
            "delta_blocks": 0,
            "migration_block_count": 2,
            "total_blocks": 2,
            "relation_counts": {},
        }
        genealogy_stats = {
            "recent_type_distribution": {},
            "pending_count": 0,
            "settled_count": 0,
        }
        _, _, _, residue_status = derive_mu(
            block_stats, genealogy_stats, str(tmp_root)
        )
        assert residue_status == "empty_shell"
