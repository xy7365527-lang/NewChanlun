"""tests for ceremony_scan.py — _scan_genealogy_proposals + _check_completion glob fix."""
from __future__ import annotations

import os
import textwrap

import pytest

from scripts.ceremony_scan import _scan_genealogy_proposals, _check_completion


# ═══════════════════════════════════════════════════════════════
# Helpers
# ═══════════════════════════════════════════════════════════════


def _write_settled(root, filename: str, content: str) -> str:
    settled_dir = os.path.join(root, ".chanlun", "genealogy", "settled")
    os.makedirs(settled_dir, exist_ok=True)
    path = os.path.join(settled_dir, filename)
    with open(path, "w", encoding="utf-8") as f:
        f.write(content)
    return path


GANGMU_WITH_TARGETS = {
    "gang": [
        {
            "id": "shipen-bihuan",
            "name": "实盘闭环",
            "mu": [
                {
                    "id": "code-pipeline",
                    "status": "active",
                    "next_actions": [
                        {"target": "G1-position-management", "description": "仓位管理"},
                        {"target": "G2-fugue-engine", "description": "赋格引擎"},
                    ],
                }
            ],
        },
        {
            "id": "swarm-infra",
            "name": "蜂群基础设施",
            "mu": [
                {
                    "id": "spec-gap-upgrade",
                    "status": "active",
                    "next_actions": [
                        {"target": "spec-gap-bidirectional", "description": "双向检测"},
                    ],
                }
            ],
        },
    ]
}

GENEALOGY_WITH_UNCOVERED = textwrap.dedent("""\
    ---
    id: '400'
    title: 测试谱系
    type: 概念定义
    status: settled
    date: 2026-03-05
    depends_on: []
    ---

    # 400号：测试谱系

    ## 下游推论

    1. **新功能A**：需要开发一个全新的模块
    2. **G1-position-management 扩展**：仓位管理模块需要增加风控
    3. **已完成的推论**：这条已执行
""")

GENEALOGY_ALL_COVERED = textwrap.dedent("""\
    ---
    id: '401'
    title: 全覆盖谱系
    type: 概念定义
    status: settled
    date: 2026-03-05
    depends_on: []
    ---

    # 401号：全覆盖谱系

    ## 下游推论

    1. **spec-gap-bidirectional 更新**：双向检测已有 gangmu target
""")

GENEALOGY_NUMBERED_SECTION = textwrap.dedent("""\
    ---
    id: '402'
    title: 编号节标题谱系
    type: 概念定义
    status: settled
    date: 2026-03-05
    depends_on: []
    ---

    # 402号：编号节标题

    ## 5. 下游推论

    1. **独立工位X**：需要一个全新工位
""")

GENEALOGY_NO_DOWNSTREAM = textwrap.dedent("""\
    ---
    id: '403'
    title: 无下游推论谱系
    type: 验证报告
    status: settled
    date: 2026-03-05
    ---

    # 403号：无下游推论

    ## 结论

    纯验证，无推论。
""")

GENEALOGY_MALFORMED = textwrap.dedent("""\
    这不是一个合法的 markdown 文件
    没有 frontmatter 也没有 ## 下游推论
""")

GENEALOGY_RESOLVED_ITEMS = textwrap.dedent("""\
    ---
    id: '404'
    title: 全部已执行
    type: 概念定义
    status: settled
    date: 2026-03-05
    depends_on: []
    ---

    # 404号：全部已执行

    ## 下游推论

    1. **功能Y**：某功能 — **已执行**（已集成到管线中）
    2. **功能Z**：另一功能 — resolved
""")


# ═══════════════════════════════════════════════════════════════
# _scan_genealogy_proposals tests
# ═══════════════════════════════════════════════════════════════


class TestScanGenealogyProposals:
    """_scan_genealogy_proposals 基本功能测试。"""

    def test_uncovered_proposals_detected(self, tmp_path):
        """未被 gangmu 覆盖的下游推论应被提议。"""
        _write_settled(tmp_path, "400-test.md", GENEALOGY_WITH_UNCOVERED)
        proposals = _scan_genealogy_proposals(str(tmp_path), GANGMU_WITH_TARGETS)

        # "新功能A" 未被覆盖，应出现在提议中
        uncovered_texts = [p["text"] for p in proposals]
        assert any("新功能A" in t for t in uncovered_texts)

    def test_covered_proposals_filtered(self, tmp_path):
        """已被 gangmu target 覆盖的推论不应出现在提议中。"""
        _write_settled(tmp_path, "400-test.md", GENEALOGY_WITH_UNCOVERED)
        proposals = _scan_genealogy_proposals(str(tmp_path), GANGMU_WITH_TARGETS)

        # "G1-position-management 扩展" 包含已有 target，应被过滤
        uncovered_texts = [p["text"] for p in proposals]
        assert not any("G1-position-management" in t for t in uncovered_texts)

    def test_resolved_items_skipped(self, tmp_path):
        """标记为已执行/resolved 的推论不应出现。"""
        _write_settled(tmp_path, "400-test.md", GENEALOGY_WITH_UNCOVERED)
        proposals = _scan_genealogy_proposals(str(tmp_path), GANGMU_WITH_TARGETS)

        uncovered_texts = [p["text"] for p in proposals]
        assert not any("已完成的推论" in t for t in uncovered_texts)

    def test_all_covered_returns_empty(self, tmp_path):
        """所有推论均被覆盖时返回空列表。"""
        _write_settled(tmp_path, "401-covered.md", GENEALOGY_ALL_COVERED)
        proposals = _scan_genealogy_proposals(str(tmp_path), GANGMU_WITH_TARGETS)
        assert proposals == []

    def test_numbered_section_header(self, tmp_path):
        """支持 '## N. 下游推论' 格式的节标题。"""
        _write_settled(tmp_path, "402-numbered.md", GENEALOGY_NUMBERED_SECTION)
        proposals = _scan_genealogy_proposals(str(tmp_path), GANGMU_WITH_TARGETS)
        assert len(proposals) == 1
        assert "独立工位X" in proposals[0]["text"]

    def test_no_downstream_section(self, tmp_path):
        """无下游推论节的谱系不产生提议。"""
        _write_settled(tmp_path, "403-none.md", GENEALOGY_NO_DOWNSTREAM)
        proposals = _scan_genealogy_proposals(str(tmp_path), GANGMU_WITH_TARGETS)
        assert proposals == []

    def test_all_resolved_returns_empty(self, tmp_path):
        """所有推论都标记为已执行时返回空列表。"""
        _write_settled(tmp_path, "404-resolved.md", GENEALOGY_RESOLVED_ITEMS)
        proposals = _scan_genealogy_proposals(str(tmp_path), GANGMU_WITH_TARGETS)
        assert proposals == []

    def test_empty_gangmu(self, tmp_path):
        """空 gangmu（无 target）时，所有推论都视为未覆盖。"""
        _write_settled(tmp_path, "402-numbered.md", GENEALOGY_NUMBERED_SECTION)
        proposals = _scan_genealogy_proposals(str(tmp_path), {})
        assert len(proposals) == 1

    def test_malformed_file_resilience(self, tmp_path):
        """格式错误的文件不阻塞扫描。"""
        _write_settled(tmp_path, "999-bad.md", GENEALOGY_MALFORMED)
        _write_settled(tmp_path, "402-good.md", GENEALOGY_NUMBERED_SECTION)
        proposals = _scan_genealogy_proposals(str(tmp_path), GANGMU_WITH_TARGETS)
        # bad 文件被跳过，good 文件正常解析
        assert len(proposals) == 1

    def test_proposal_fields(self, tmp_path):
        """提议条目包含必要字段。"""
        _write_settled(tmp_path, "400-test.md", GENEALOGY_WITH_UNCOVERED)
        proposals = _scan_genealogy_proposals(str(tmp_path), GANGMU_WITH_TARGETS)
        assert len(proposals) >= 1
        p = proposals[0]
        assert "source" in p
        assert "field" in p
        assert "text" in p
        assert "coverage_status" in p
        assert p["coverage_status"] == "not_covered"
        assert "suggested_gang" in p

    def test_recent_20_limit(self, tmp_path):
        """只扫描最近 20 条谱系。"""
        # 写入 25 条，每条都有未覆盖推论
        for i in range(1, 26):
            content = textwrap.dedent(f"""\
                ---
                id: '{i}'
                title: 测试谱系{i}
                status: settled
                date: 2026-03-05
                ---

                # {i}号

                ## 下游推论

                1. **唯一推论{i}**：描述{i}
            """)
            _write_settled(tmp_path, f"{i:03d}-test{i}.md", content)

        proposals = _scan_genealogy_proposals(str(tmp_path), {})
        # 最多来自 20 条谱系（编号 6-25，跳过 1-5）
        source_nums = {p["source"] for p in proposals}
        assert len(source_nums) <= 20
        # 最大编号应被包含（最近）
        assert "25号" in source_nums

    def test_missing_settled_dir(self, tmp_path):
        """settled 目录不存在时返回空列表。"""
        proposals = _scan_genealogy_proposals(str(tmp_path), GANGMU_WITH_TARGETS)
        assert proposals == []


# ═══════════════════════════════════════════════════════════════
# _check_completion glob fix tests
# ═══════════════════════════════════════════════════════════════


class TestCheckCompletionGlob:
    """_check_completion 的 test_file_exists glob 模式支持。"""

    def test_exact_path_still_works(self, tmp_path):
        """精确路径仍然正常工作。"""
        test_file = os.path.join(tmp_path, "tests", "test_foo.py")
        os.makedirs(os.path.dirname(test_file), exist_ok=True)
        with open(test_file, "w") as f:
            f.write("# test")

        check = {"type": "test_file_exists", "pattern": "tests/test_foo.py"}
        assert _check_completion(str(tmp_path), check) is True

    def test_glob_pattern_matches(self, tmp_path):
        """glob 通配符模式正确匹配。"""
        test_file = os.path.join(tmp_path, "tests", "test_backtest_engine.py")
        os.makedirs(os.path.dirname(test_file), exist_ok=True)
        with open(test_file, "w") as f:
            f.write("# test")

        check = {"type": "test_file_exists", "pattern": "tests/test_backtest_*.py"}
        assert _check_completion(str(tmp_path), check) is True

    def test_glob_pattern_no_match(self, tmp_path):
        """glob 通配符无匹配时返回 False。"""
        os.makedirs(os.path.join(tmp_path, "tests"), exist_ok=True)

        check = {"type": "test_file_exists", "pattern": "tests/test_nonexistent_*.py"}
        assert _check_completion(str(tmp_path), check) is False

    def test_glob_multiple_matches(self, tmp_path):
        """glob 匹配多个文件时返回 True。"""
        tests_dir = os.path.join(tmp_path, "tests")
        os.makedirs(tests_dir, exist_ok=True)
        for name in ("test_backtest_a.py", "test_backtest_b.py"):
            with open(os.path.join(tests_dir, name), "w") as f:
                f.write("# test")

        check = {"type": "test_file_exists", "pattern": "tests/test_backtest_*.py"}
        assert _check_completion(str(tmp_path), check) is True

    def test_empty_pattern(self, tmp_path):
        """空 pattern 返回 False。"""
        check = {"type": "test_file_exists", "pattern": ""}
        assert _check_completion(str(tmp_path), check) is False
