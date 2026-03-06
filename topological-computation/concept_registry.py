"""Concept Registry — bidirectional mapping between human-readable concept names and vertex IDs.

Pure Python, no external dependencies.
"""

from __future__ import annotations

import re
from dataclasses import dataclass, field

from engine import Graph
from genealogy_loader import _summarize_content, load_blocks


# ---------------------------------------------------------------------------
# Tokenization
# ---------------------------------------------------------------------------

# Split on whitespace, punctuation, brackets, special chars
_SPLIT_RE = re.compile(r'[\s,.:;!?\[\](){}/<>|=+\-_#@&*^~`\'"\\]+')


def _tokenize(text: str) -> list[str]:
    """Split text into keyword tokens. Handles Chinese and English."""
    tokens: list[str] = []
    for piece in _SPLIT_RE.split(text):
        piece = piece.strip()
        if len(piece) >= 2:
            tokens.append(piece.lower())
    return tokens


# ---------------------------------------------------------------------------
# Registry
# ---------------------------------------------------------------------------

@dataclass
class Registry:
    """Bidirectional concept name <-> vertex ID mapping with inverted index."""

    # vertex_id -> content summary
    _summaries: dict[str, str] = field(default_factory=dict)
    # token -> list of vertex_ids containing that token
    _index: dict[str, list[str]] = field(default_factory=dict)

    def lookup(self, keyword: str) -> list[tuple[str, str]]:
        """Fuzzy match keyword against content summaries.

        Returns [(vertex_id, content_summary), ...] sorted by relevance (substring match first).
        """
        keyword_lower = keyword.lower()
        results: list[tuple[str, str, int]] = []

        # Strategy 1: exact token match via inverted index
        token_hits: set[str] = set()
        if keyword_lower in self._index:
            token_hits.update(self._index[keyword_lower])

        # Strategy 2: substring match against all summaries
        for vid, summary in self._summaries.items():
            if keyword_lower in summary.lower():
                # Score: lower = better. Exact token match scores 0, substring-only scores 1.
                score = 0 if vid in token_hits else 1
                results.append((vid, summary, score))
            elif vid in token_hits:
                results.append((vid, summary, 2))

        results.sort(key=lambda x: (x[2], x[1]))
        return [(vid, summary) for vid, summary, _ in results]

    def reverse(self, vertex_id: str) -> str:
        """Vertex ID -> content summary."""
        return self._summaries.get(vertex_id, "")


def build_registry(graph: Graph, blocks_dir: str) -> Registry:
    """Build a Registry from graph vertices and block JSON files.

    Uses block JSON content for richer summaries. Falls back to vertex.content
    for vertices not found in blocks.
    """
    blocks = load_blocks(blocks_dir)
    summaries: dict[str, str] = {}
    index: dict[str, list[str]] = {}

    for vid, vertex in graph.vertices.items():
        # Prefer block-based summary (richer), fall back to vertex.content
        if vid in blocks:
            summary = _summarize_content(blocks[vid])
        else:
            summary = vertex.content or ""
        summaries[vid] = summary

        # Tokenize and index
        tokens = _tokenize(summary)
        for token in set(tokens):
            index.setdefault(token, []).append(vid)

    return Registry(_summaries=summaries, _index=index)
