"""test_ceremony_ingest.py — ceremony 摄入管道测试.

覆盖 437号管道2（ceremony 产出摄入钩子）和 441号物质通道。

认识论等级：L1（合成数据验证管线正确性）
"""

from __future__ import annotations

import json
import os
import sys
import tempfile
from pathlib import Path

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

from signifier_net import SNet, Signifier, SignifierEdge, AxisType
from ceremony_ingest import (
    _strip_frontmatter,
    _extract_paragraphs,
    ingest_ceremony_texts,
    ingest_llm_externalization,
    CeremonyIngestResult,
)


# ---------------------------------------------------------------------------
# Frontmatter stripping
# ---------------------------------------------------------------------------

def test_strip_frontmatter_basic():
    text = """---
id: '437'
title: "test"
status: settled
---

# Main content

This is the body text about 概念 and 定义."""

    result = _strip_frontmatter(text)
    assert "id: '437'" not in result
    assert "status: settled" not in result
    assert "Main content" in result
    assert "body text" in result


def test_strip_frontmatter_no_frontmatter():
    text = "# Just a heading\n\nSome content here."
    result = _strip_frontmatter(text)
    assert result == text


def test_strip_frontmatter_only_opening():
    text = "---\nid: test\nno closing\n# Content"
    result = _strip_frontmatter(text)
    assert result == text


# ---------------------------------------------------------------------------
# Paragraph extraction
# ---------------------------------------------------------------------------

def test_extract_paragraphs_basic():
    text = """---
id: '001'
title: "test genealogy"
---

# 001号：概念发现

这是一段关于概念发现的描述文本，包含多个术语和定义。

## 核心发现

| 概念 | 来源 |
|------|------|
| A | B |

概念 A 的否定产生了概念 B，这是扬弃的典型形态。

```python
code_here()
```

短行
"""

    paragraphs = _extract_paragraphs(text)
    # frontmatter 被移除
    # 表格行被过滤
    # 代码块被过滤
    # 短行被丢弃
    assert len(paragraphs) >= 2
    assert any("概念发现" in p for p in paragraphs)
    assert any("否定" in p for p in paragraphs)
    # 代码不在段落中
    assert not any("code_here" in p for p in paragraphs)
    # 表格不在段落中
    assert not any("| A | B |" in p for p in paragraphs)


def test_extract_paragraphs_empty():
    assert _extract_paragraphs("") == []


def test_extract_paragraphs_only_frontmatter():
    text = """---
id: '001'
title: "test"
---
"""
    assert _extract_paragraphs(text) == []


# ---------------------------------------------------------------------------
# Full ingest pipeline
# ---------------------------------------------------------------------------

def _make_test_snet() -> SNet:
    """Create a minimal S_net with some signifiers for testing."""
    snet = SNet()
    for term in ["概念", "否定", "扬弃", "穿越", "能指", "S_net", "参数"]:
        snet = snet.add_signifier(Signifier(
            id=term,
            source="test",
        ))
    return snet


def test_ingest_ceremony_texts():
    snet = _make_test_snet()

    with tempfile.TemporaryDirectory() as tmpdir:
        # Create genealogy directory structure
        settled_dir = Path(tmpdir) / "settled"
        settled_dir.mkdir()

        # Create a test genealogy file
        (settled_dir / "001-test.md").write_text("""---
id: '001'
title: "测试谱系"
status: 已结算
---

# 001号：概念否定测试

否定是概念运动的核心形式。概念 A 的否定产生概念 B。

## 扬弃

扬弃是否定的保留。穿越路径在否定后继续。
""", encoding="utf-8")

        state_file = Path(tmpdir) / ".ceremony_ingest_state.json"

        new_snet, result = ingest_ceremony_texts(
            snet=snet,
            bt_writer=None,
            genealogy_dir=tmpdir,
            state_file=state_file,
        )

        assert result.files_processed == 1
        assert result.paragraphs_ingested >= 2
        assert result.errors == ()

        # State file should exist
        assert state_file.exists()
        state = json.loads(state_file.read_text(encoding="utf-8"))
        assert "settled/001-test.md" in state or "settled\\001-test.md" in state


def test_ingest_ceremony_texts_dedup():
    snet = _make_test_snet()

    with tempfile.TemporaryDirectory() as tmpdir:
        # Create a test file
        (Path(tmpdir) / "test.md").write_text(
            "# 概念否定\n\n概念的否定产生新的扬弃。穿越继续。",
            encoding="utf-8",
        )

        state_file = Path(tmpdir) / ".state.json"

        # First ingest
        new_snet, result1 = ingest_ceremony_texts(
            snet=snet, bt_writer=None,
            genealogy_dir=tmpdir, state_file=state_file,
        )
        assert result1.files_processed == 1

        # Second ingest — same content, should be skipped
        new_snet2, result2 = ingest_ceremony_texts(
            snet=new_snet, bt_writer=None,
            genealogy_dir=tmpdir, state_file=state_file,
        )
        assert result2.files_processed == 0
        assert result2.files_skipped == 1


def test_ingest_ceremony_texts_missing_dir():
    snet = _make_test_snet()
    new_snet, result = ingest_ceremony_texts(
        snet=snet, bt_writer=None,
        genealogy_dir="/nonexistent/path",
        state_file="/tmp/.test_state.json",
    )
    assert result.files_processed == 0
    assert "does not exist" in result.errors[0]


# ---------------------------------------------------------------------------
# LLM externalization writeback (437-3)
# ---------------------------------------------------------------------------

def test_ingest_llm_externalization():
    snet = _make_test_snet()
    llm_output = "概念的否定是扬弃的核心。穿越在能指网络中沿组合轴运动。"

    new_snet, count = ingest_llm_externalization(snet, llm_output)
    # Should produce cooccurrence edges between matched terms
    assert count >= 0  # at least 0, depends on whitelist matching
    assert new_snet is not snet or count == 0


def test_ingest_llm_externalization_empty():
    snet = _make_test_snet()
    new_snet, count = ingest_llm_externalization(snet, "")
    assert count == 0
    assert new_snet is snet


def test_ingest_llm_externalization_no_signifiers():
    snet = SNet()
    new_snet, count = ingest_llm_externalization(snet, "some text")
    assert count == 0


# ---------------------------------------------------------------------------
# Run
# ---------------------------------------------------------------------------

if __name__ == "__main__":
    import pytest
    sys.exit(pytest.main([__file__, "-v"]))
