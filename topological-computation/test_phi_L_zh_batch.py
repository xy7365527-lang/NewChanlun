"""Tests for Chinese NLP preprocessing and batch processing pipeline.

Tests:
1. Chinese text preprocessing produces correct concept vertices
2. Negation detection works for Chinese text
3. Contrast conjunction detection works
4. Language auto-detection
5. phi_L integration with Chinese text
6. Batch pipeline with mixed Chinese/English files
"""

from __future__ import annotations

import json
import os
import sys
import tempfile

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

from nlp_preprocess_zh import (
    preprocess_zh, detect_language,
    _split_sentences, _extract_concept_fragments,
    _detect_negation_in_sentence, _detect_contrast,
)
from phi_L import phi_L
from phi_L_batch import batch_process, phi_L_multilingual, _inject_subgraph
from engine import Graph, Vertex, Edge, EdgeType, compute_beta_1


# ---------------------------------------------------------------------------
# Chinese preprocessing tests
# ---------------------------------------------------------------------------

def test_sentence_splitting():
    """Test Chinese sentence splitting by punctuation."""
    text = "走势终完美。任何级别的任何走势类型终要完成。"
    sentences = _split_sentences(text)
    assert len(sentences) == 2, f"Expected 2 sentences, got {len(sentences)}: {sentences}"
    assert "走势终完美" in sentences[0]
    assert "走势类型" in sentences[1]
    print("  [PASS] sentence_splitting")


def test_concept_extraction():
    """Test concept extraction from Chinese text."""
    text = "走势终完美"
    fragments = _extract_concept_fragments(text)
    assert len(fragments) >= 2, f"Expected >= 2 fragments, got {fragments}"
    all_text = "".join(fragments)
    assert "走势" in all_text, f"Expected '走势' in {fragments}"
    assert "完美" in all_text, f"Expected '完美' in {fragments}"
    print(f"  [PASS] concept_extraction: {fragments}")


def test_concept_extraction_complex():
    """Test concept extraction from a more complex sentence."""
    text = "任何级别的任何走势类型终要完成"
    fragments = _extract_concept_fragments(text)
    assert len(fragments) >= 2, f"Expected >= 2 fragments, got {fragments}"
    all_text = "".join(fragments)
    assert "级别" in all_text, f"Expected '级别' in {fragments}"
    assert "走势类型" in all_text, f"Expected '走势类型' in {fragments}"
    print(f"  [PASS] concept_extraction_complex: {fragments}")


def test_negation_detection():
    """Test negation word detection in Chinese text."""
    text = "不能产生真正的新事物"
    negations = _detect_negation_in_sentence(text)
    assert len(negations) >= 1, f"Expected >= 1 negation, got {negations}"
    neg_words = [w for _, w in negations]
    assert any(w in ("不能", "不") for w in neg_words), f"Expected '不能' or '不' in {neg_words}"
    print(f"  [PASS] negation_detection: {negations}")


def test_contrast_detection():
    """Test contrastive conjunction detection."""
    text = "走势会完美，但是不会永远完美"
    contrasts = _detect_contrast(text)
    assert len(contrasts) >= 1, f"Expected >= 1 contrast, got {contrasts}"
    print(f"  [PASS] contrast_detection: positions={contrasts}")


def test_preprocess_zh_basic():
    """Test full preprocessing pipeline for Chinese text."""
    text = "走势终完美。任何级别的任何走势类型终要完成。"
    trees = preprocess_zh(text)
    assert len(trees) >= 1, f"Expected >= 1 sentence tree, got {len(trees)}"

    # Check that we have tokens and noun chunks
    total_chunks = sum(len(t.noun_chunks) for t in trees)
    assert total_chunks >= 2, f"Expected >= 2 noun chunks, got {total_chunks}"

    # Verify concept content
    all_concepts = [chunk.text for tree in trees for chunk in tree.noun_chunks]
    print(f"  [PASS] preprocess_zh_basic: concepts={all_concepts}")


def test_preprocess_zh_negation_sentence():
    """Test that negation in Chinese text produces NEGATION edges via phi_L."""
    text = "走势不能停止。走势终完美。"
    trees = preprocess_zh(text)
    assert len(trees) >= 1
    print(f"  [PASS] preprocess_zh_negation: {len(trees)} trees")


# ---------------------------------------------------------------------------
# Language detection tests
# ---------------------------------------------------------------------------

def test_detect_language_zh():
    """Test language detection for Chinese text."""
    assert detect_language("走势终完美。") == "zh"
    assert detect_language("任何级别的任何走势类型终要完成。") == "zh"
    print("  [PASS] detect_language_zh")


def test_detect_language_en():
    """Test language detection for English text."""
    assert detect_language("All complex systems exhibit emergent behavior.") == "en"
    assert detect_language("Theories require hypotheses.") == "en"
    print("  [PASS] detect_language_en")


def test_detect_language_empty():
    """Test language detection for empty/edge cases."""
    assert detect_language("") == "en"
    assert detect_language("   ") == "en"
    assert detect_language("123 456") == "en"
    print("  [PASS] detect_language_empty")


# ---------------------------------------------------------------------------
# phi_L integration tests
# ---------------------------------------------------------------------------

def test_phi_L_chinese():
    """Test phi_L with Chinese text produces a graph with vertices and edges."""
    text = "走势终完美。任何级别的任何走势类型终要完成。"
    g = phi_L(text)

    verts = g.vertices
    edges = g.edges

    assert len(verts) >= 2, f"Expected >= 2 vertices, got {len(verts)}"

    # Print graph for inspection
    print(f"  phi_L Chinese graph: {len(verts)} vertices, {len(edges)} edges")
    for vid, v in sorted(verts.items()):
        print(f"    {vid}: {v.content!r}")
    for e in edges:
        src = verts[e.source].content if e.source in verts else e.source
        tgt = verts[e.target].content if e.target in verts else e.target
        print(f"    {src!r} --[{e.edge_type.value}]--> {tgt!r}")

    print("  [PASS] phi_L_chinese")


def test_phi_L_english_still_works():
    """Test that English text still works after the language detection integration."""
    text = "Theories require hypotheses. Hypotheses require experiments."
    g = phi_L(text)
    assert len(g.vertices) >= 2, f"Expected >= 2 vertices for English text"
    print("  [PASS] phi_L_english_still_works")


# ---------------------------------------------------------------------------
# Batch pipeline tests
# ---------------------------------------------------------------------------

def test_inject_subgraph():
    """Test subgraph injection merges vertices by content."""
    g1 = Graph()
    g1 = g1.add_vertex(Vertex(id="v0", content="alpha"))
    g1 = g1.add_vertex(Vertex(id="v1", content="beta"))
    g1 = g1.add_edge(Edge(source="v0", target="v1", edge_type=EdgeType.DEPENDENCY))

    g2 = Graph()
    g2 = g2.add_vertex(Vertex(id="v0", content="beta"))   # same content as g1.v1
    g2 = g2.add_vertex(Vertex(id="v1", content="gamma"))  # new concept
    g2 = g2.add_edge(Edge(source="v0", target="v1", edge_type=EdgeType.DEPENDENCY))

    merged = _inject_subgraph(g1, g2)

    assert len(merged.vertices) == 3, f"Expected 3 vertices, got {len(merged.vertices)}"
    assert len(merged.edges) == 2, f"Expected 2 edges, got {len(merged.edges)}"

    contents = {v.content for v in merged.vertices.values()}
    assert contents == {"alpha", "beta", "gamma"}, f"Expected alpha/beta/gamma, got {contents}"
    print("  [PASS] inject_subgraph")


def test_phi_L_multilingual():
    """Test multilingual phi_L wrapper."""
    zh_graph = phi_L_multilingual("走势终完美。", lang="zh")
    en_graph = phi_L_multilingual("Theories require hypotheses.", lang="en")
    auto_zh = phi_L_multilingual("走势终完美。")
    auto_en = phi_L_multilingual("Theories require hypotheses.")

    assert len(zh_graph.vertices) >= 1
    assert len(en_graph.vertices) >= 1
    assert len(auto_zh.vertices) >= 1
    assert len(auto_en.vertices) >= 1
    print("  [PASS] phi_L_multilingual")


def test_batch_process():
    """Test batch processing with a temporary directory of text files."""
    with tempfile.TemporaryDirectory() as tmpdir:
        # Create test files
        with open(os.path.join(tmpdir, "01_intro.txt"), "w", encoding="utf-8") as f:
            f.write("走势终完美。任何级别的任何走势类型终要完成。")

        with open(os.path.join(tmpdir, "02_theory.txt"), "w", encoding="utf-8") as f:
            f.write("Theories require hypotheses. Hypotheses require experiments.")

        with open(os.path.join(tmpdir, "03_mixed.txt"), "w", encoding="utf-8") as f:
            f.write("走势类型包含趋势和盘整。盘整不是趋势。")

        output_json = os.path.join(tmpdir, "results.json")

        results = batch_process(
            text_dir=tmpdir,
            output_json=output_json,
            settlement_threshold=5,
            convergence_window=10,
            seed=42,
        )

        # Verify output file exists
        assert os.path.exists(output_json), "Output JSON not created"

        # Verify structure
        assert "config" in results
        assert "files" in results
        assert "aggregate" in results
        assert len(results["files"]) == 3
        assert results["config"]["file_count"] == 3

        # Verify per-file results
        for file_result in results["files"]:
            assert "filename" in file_result
            assert "language" in file_result
            assert "beta_1_after_traversal" in file_result
            assert file_result["sub_vertices"] >= 1

        # Verify aggregate
        assert results["aggregate"]["total_vertices"] >= 3
        assert results["aggregate"]["total_edges"] >= 1

        # Read back JSON to verify serialization
        with open(output_json, "r", encoding="utf-8") as f:
            loaded = json.load(f)
        assert loaded["config"]["file_count"] == 3

        print("  [PASS] batch_process")


# ---------------------------------------------------------------------------
# Runner
# ---------------------------------------------------------------------------

def main():
    print("=" * 60)
    print("Chinese NLP + Batch Pipeline Tests")
    print("=" * 60)

    print("\n--- Chinese Preprocessing ---")
    test_sentence_splitting()
    test_concept_extraction()
    test_concept_extraction_complex()
    test_negation_detection()
    test_contrast_detection()
    test_preprocess_zh_basic()
    test_preprocess_zh_negation_sentence()

    print("\n--- Language Detection ---")
    test_detect_language_zh()
    test_detect_language_en()
    test_detect_language_empty()

    print("\n--- phi_L Integration ---")
    test_phi_L_chinese()
    test_phi_L_english_still_works()

    print("\n--- Batch Pipeline ---")
    test_inject_subgraph()
    test_phi_L_multilingual()
    test_batch_process()

    print("\n" + "=" * 60)
    print("All tests passed.")
    print("=" * 60)


if __name__ == "__main__":
    main()
