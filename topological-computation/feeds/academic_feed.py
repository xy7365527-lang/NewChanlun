"""Academic feed pipeline: search -> rank -> download -> phi_L -> quality check -> inject.

Replaces auto_feed.py's generic search for academic gap signals.
Pure stdlib for HTTP. PDF extraction via PyMuPDF or pdftotext.
"""

from __future__ import annotations

import os
import sys
import tempfile
from dataclasses import dataclass

sys.path.insert(0, os.path.join(os.path.dirname(os.path.abspath(__file__)), ".."))

from feeds import semantic_scholar, arxiv, unpaywall, pdf_to_text


@dataclass(frozen=True, slots=True)
class AcademicFeedRecord:
    """Record of a single academic feed attempt."""
    accepted: bool
    source: str          # 'semantic_scholar', 'arxiv_pdf', 'unpaywall_pdf', 'abstract', 'none'
    query: str
    paper_title: str
    vertices_added: int
    edges_added: int
    match_rate: float
    verdict: str
    reason: str


def build_academic_query(gap_signal: dict) -> str:
    """Build search query from gap signal.

    Combines gap vertex content + neighbor contents for context.
    """
    parts: list[str] = []

    content = gap_signal.get("content", "")
    if content:
        parts.append(content)

    neighbors = gap_signal.get("neighbor_contents", [])
    for n in neighbors:
        if n and isinstance(n, str):
            parts.append(n)

    query = " ".join(parts)
    # Trim to reasonable length for API queries
    if len(query) > 200:
        query = query[:200]
    return query.strip()


def _get_full_text(paper: dict) -> tuple[str, str]:
    """Try to get full text for a paper.

    Attempts in order:
    1. arXiv PDF (if arxiv_id available)
    2. Unpaywall OA PDF (if DOI available)
    3. Fall back to abstract

    Returns (text, source) tuple.
    """
    arxiv_id = paper.get("arxivId", "")
    doi = paper.get("doi", "")
    abstract = paper.get("abstract", "") or ""

    # Try arXiv PDF
    if arxiv_id:
        try:
            with tempfile.NamedTemporaryFile(suffix=".pdf", delete=False) as tmp:
                tmp_path = tmp.name
            arxiv.download_pdf(arxiv_id, tmp_path)
            text = pdf_to_text.extract_text(tmp_path)
            os.unlink(tmp_path)
            if text and len(text) > len(abstract):
                return text[:10000], "arxiv_pdf"
        except (OSError, Exception):
            # Clean up on failure
            try:
                os.unlink(tmp_path)
            except (OSError, UnboundLocalError):
                pass

    # Try Unpaywall
    if doi:
        oa_url = unpaywall.get_oa_url(doi)
        if oa_url and oa_url.endswith(".pdf"):
            try:
                import urllib.request
                with tempfile.NamedTemporaryFile(suffix=".pdf", delete=False) as tmp:
                    tmp_path = tmp.name
                urllib.request.urlretrieve(oa_url, tmp_path)
                text = pdf_to_text.extract_text(tmp_path)
                os.unlink(tmp_path)
                if text and len(text) > len(abstract):
                    return text[:10000], "unpaywall_pdf"
            except (OSError, Exception):
                try:
                    os.unlink(tmp_path)
                except (OSError, UnboundLocalError):
                    pass

    # Fall back to abstract
    if abstract:
        return abstract, "abstract"

    return "", "none"


def feed_from_academic_gap(gap_signal: dict, daemon: object) -> AcademicFeedRecord:
    """Complete academic feed pipeline.

    1. Build academic query from gap signal
    2. Search Semantic Scholar (sort by citations)
    3. For top 3 papers: try full text, fall back to abstract
    4. phi_L process text
    5. Quality check (connection_check against K_active)
    6. If nutritious -> inject into daemon
    7. Return feed record
    """
    from phi_L import phi_L
    from auto_feed import quality_check

    query = gap_signal.get("search_query", "") or build_academic_query(gap_signal)
    if not query:
        return AcademicFeedRecord(
            accepted=False, source="none", query="",
            paper_title="", vertices_added=0, edges_added=0,
            match_rate=0.0, verdict="no_query", reason="no search query provided",
        )

    # Search Semantic Scholar (already sorted by citationCount)
    papers = semantic_scholar.search(query, limit=5)
    if not papers:
        return AcademicFeedRecord(
            accepted=False, source="none", query=query,
            paper_title="", vertices_added=0, edges_added=0,
            match_rate=0.0, verdict="no_results",
            reason="Semantic Scholar returned no papers",
        )

    # Try top 3 papers
    for paper in papers[:3]:
        text, source = _get_full_text(paper)
        if not text:
            continue

        sub = phi_L(text)
        sub_v = len(sub.active_vertex_ids())
        sub_e = len(sub.active_edges())

        if sub_v == 0:
            continue

        match_rate, verdict = quality_check(sub, daemon.k_active)

        if verdict == "nutritious":
            from daemon_api import feed_via_snet
            feed_via_snet(daemon, text, source_type="academic_feed")
            return AcademicFeedRecord(
                accepted=True, source=source, query=query,
                paper_title=paper.get("title", ""),
                vertices_added=sub_v, edges_added=sub_e,
                match_rate=match_rate, verdict=verdict, reason="",
            )

    # None of the top papers were nutritious — report best attempt
    best_paper = papers[0]
    text, source = _get_full_text(best_paper)
    if text:
        sub = phi_L(text)
        sub_v = len(sub.active_vertex_ids())
        sub_e = len(sub.active_edges())
        match_rate, verdict = quality_check(sub, daemon.k_active)
    else:
        sub_v, sub_e, match_rate, verdict = 0, 0, 0.0, "no_text"

    return AcademicFeedRecord(
        accepted=False, source=source, query=query,
        paper_title=best_paper.get("title", ""),
        vertices_added=sub_v, edges_added=sub_e,
        match_rate=match_rate, verdict=verdict,
        reason=f"no nutritious paper found in top 3 (best: {verdict}, rate={match_rate:.3f})",
    )
