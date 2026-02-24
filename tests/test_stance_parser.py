"""Tests for scripts/stance_parser.py — 立场声明解析器

测试从 Gemini/Codex 回复文本中提取结构化立场声明的能力。
编排者约束：无法修改外部模型内部推理，只能控制 prompt 和输出协议。
"""

from __future__ import annotations

import pytest

from scripts.consensus_trigger import StanceDeclaration
from scripts.stance_parser import (
    STANCE_OUTPUT_PROTOCOL_CODEX,
    STANCE_OUTPUT_PROTOCOL_GEMINI,
    parse_stance_declaration,
    parse_stance_sequence,
)


# ── parse_stance_declaration tests ──


class TestParseStanceDeclaration:
    def test_basic_parse(self):
        text = """\
这是 Gemini 的回复正文。

---stance-declaration---
verdict: fail
stances:
  dep_chain: contradictory
  completeness: accept
concessions: []
---end-stance---
"""
        sd = parse_stance_declaration(text, round_number=1)
        assert sd is not None
        assert sd.verdict == "fail"
        assert sd.stances == {"dep_chain": "contradictory", "completeness": "accept"}
        assert sd.round_number == 1
        assert sd.self_reported_concessions == []

    def test_with_concessions(self):
        text = """\
回复正文。

---stance-declaration---
verdict: conditional
stances:
  api_surface: accept
concessions:
  - hook_design
  - error_handling
---end-stance---
"""
        sd = parse_stance_declaration(text, round_number=2)
        assert sd is not None
        assert sd.verdict == "conditional"
        assert sd.self_reported_concessions == ["hook_design", "error_handling"]

    def test_pass_verdict(self):
        text = """\
无否定。

---stance-declaration---
verdict: pass
stances:
  overall: no_issues
concessions: []
---end-stance---
"""
        sd = parse_stance_declaration(text, round_number=1)
        assert sd is not None
        assert sd.verdict == "pass"

    def test_no_stance_block_returns_none(self):
        text = "这是一个没有立场声明块的普通回复。"
        sd = parse_stance_declaration(text)
        assert sd is None

    def test_invalid_yaml_returns_none(self):
        text = """\
---stance-declaration---
{invalid yaml: [[[
---end-stance---
"""
        sd = parse_stance_declaration(text)
        assert sd is None

    def test_invalid_verdict_returns_none(self):
        text = """\
---stance-declaration---
verdict: unknown_value
stances:
  a: b
---end-stance---
"""
        sd = parse_stance_declaration(text)
        assert sd is None

    def test_missing_verdict_returns_none(self):
        text = """\
---stance-declaration---
stances:
  a: b
---end-stance---
"""
        sd = parse_stance_declaration(text)
        assert sd is None

    def test_empty_stances(self):
        text = """\
---stance-declaration---
verdict: pass
stances: {}
concessions: []
---end-stance---
"""
        sd = parse_stance_declaration(text)
        assert sd is not None
        assert sd.stances == {}

    def test_stances_values_coerced_to_string(self):
        """stances 的 key/value 都应被强制转为字符串"""
        text = """\
---stance-declaration---
verdict: fail
stances:
  123: 456
  true: false
---end-stance---
"""
        sd = parse_stance_declaration(text)
        assert sd is not None
        assert sd.stances == {"123": "456", "True": "False"}

    def test_missing_concessions_defaults_to_empty(self):
        text = """\
---stance-declaration---
verdict: fail
stances:
  issue: reject
---end-stance---
"""
        sd = parse_stance_declaration(text)
        assert sd is not None
        assert sd.self_reported_concessions == []

    def test_default_round_number(self):
        text = """\
---stance-declaration---
verdict: pass
stances: {}
---end-stance---
"""
        sd = parse_stance_declaration(text)
        assert sd is not None
        assert sd.round_number == 0

    def test_block_embedded_in_markdown(self):
        """立场声明块嵌入在 markdown 代码块内时也能解析"""
        text = """\
## 结论

质询通过。

```yaml
---stance-declaration---
verdict: pass
stances:
  logic: sound
---end-stance---
```
"""
        sd = parse_stance_declaration(text, round_number=3)
        assert sd is not None
        assert sd.verdict == "pass"
        assert sd.stances == {"logic": "sound"}

    def test_multiple_blocks_uses_first(self):
        """如果回复中意外出现多个声明块，使用第一个"""
        text = """\
---stance-declaration---
verdict: fail
stances:
  a: reject
---end-stance---

---stance-declaration---
verdict: pass
stances:
  b: accept
---end-stance---
"""
        sd = parse_stance_declaration(text)
        assert sd is not None
        assert sd.verdict == "fail"


# ── parse_stance_sequence tests ──


class TestParseStanceSequence:
    def test_multi_round(self):
        texts = [
            "Round 1\n---stance-declaration---\nverdict: fail\nstances:\n  x: reject\n---end-stance---\n",
            "Round 2\n---stance-declaration---\nverdict: pass\nstances:\n  x: accept\nconcessions:\n  - x\n---end-stance---\n",
        ]
        seq = parse_stance_sequence(texts, start_round=1)
        assert len(seq) == 2
        assert seq[0].round_number == 1
        assert seq[0].verdict == "fail"
        assert seq[1].round_number == 2
        assert seq[1].verdict == "pass"
        assert seq[1].self_reported_concessions == ["x"]

    def test_skips_rounds_without_block(self):
        texts = [
            "Round 1 无立场声明块",
            "Round 2\n---stance-declaration---\nverdict: fail\nstances:\n  a: reject\n---end-stance---\n",
            "Round 3 也没有",
        ]
        seq = parse_stance_sequence(texts, start_round=1)
        assert len(seq) == 1
        assert seq[0].round_number == 2

    def test_empty_texts(self):
        assert parse_stance_sequence([]) == []

    def test_all_invalid(self):
        texts = ["no block here", "also no block"]
        assert parse_stance_sequence(texts) == []


# ── Prompt protocol strings ──


class TestPromptProtocols:
    def test_gemini_protocol_contains_markers(self):
        assert "---stance-declaration---" in STANCE_OUTPUT_PROTOCOL_GEMINI
        assert "---end-stance---" in STANCE_OUTPUT_PROTOCOL_GEMINI
        assert "verdict" in STANCE_OUTPUT_PROTOCOL_GEMINI
        assert "stances" in STANCE_OUTPUT_PROTOCOL_GEMINI

    def test_codex_protocol_contains_markers(self):
        assert "---stance-declaration---" in STANCE_OUTPUT_PROTOCOL_CODEX
        assert "---end-stance---" in STANCE_OUTPUT_PROTOCOL_CODEX
        assert "verdict" in STANCE_OUTPUT_PROTOCOL_CODEX
        assert "stances" in STANCE_OUTPUT_PROTOCOL_CODEX

    def test_protocols_are_different(self):
        """两个协议文本有不同的说明（面向不同场景）"""
        assert STANCE_OUTPUT_PROTOCOL_GEMINI != STANCE_OUTPUT_PROTOCOL_CODEX
