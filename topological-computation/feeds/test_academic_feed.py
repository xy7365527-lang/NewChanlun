"""Integration test for academic feed pipeline.

Tests search functionality without daemon injection.
Output saved to tmp/academic_feed_test.txt.
"""

from __future__ import annotations

import os
import sys
import time

sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

from feeds import semantic_scholar, arxiv, unpaywall, pdf_to_text


def run_test() -> str:
    lines: list[str] = []
    lines.append("=" * 70)
    lines.append("ACADEMIC FEED PIPELINE TEST REPORT")
    lines.append("=" * 70)
    lines.append("")

    # --- Test 1: Semantic Scholar search ---
    lines.append("--- Test 1: Semantic Scholar search ---")
    lines.append('Query: "discrete Morse theory"')
    lines.append("")

    try:
        papers = semantic_scholar.search("discrete Morse theory", limit=5)
        lines.append(f"  Results: {len(papers)} papers")
        for i, p in enumerate(papers):
            lines.append(f"  [{i+1}] {p['title']}")
            lines.append(f"      Citations: {p['citationCount']}")
            lines.append(f"      DOI: {p['doi']}")
            lines.append(f"      arXiv: {p['arxivId']}")
            if p['abstract']:
                lines.append(f"      Abstract: {p['abstract'][:150]}...")
            lines.append("")
    except Exception as e:
        lines.append(f"  ERROR: {e}")
    lines.append("")

    # Rate limit pause
    time.sleep(1)

    # --- Test 2: arXiv search ---
    lines.append("--- Test 2: arXiv search ---")
    lines.append('Query: "discrete Morse theory"')
    lines.append("")

    try:
        arxiv_papers = arxiv.search("discrete Morse theory", max_results=3)
        lines.append(f"  Results: {len(arxiv_papers)} papers")
        for i, p in enumerate(arxiv_papers):
            lines.append(f"  [{i+1}] {p['title']}")
            lines.append(f"      arXiv ID: {p['arxiv_id']}")
            lines.append(f"      PDF URL: {p['pdf_url']}")
            if p['abstract']:
                lines.append(f"      Abstract: {p['abstract'][:150]}...")
            lines.append("")
    except Exception as e:
        lines.append(f"  ERROR: {e}")
    lines.append("")

    # --- Test 3: Unpaywall (Forman 1998 DOI) ---
    lines.append("--- Test 3: Unpaywall ---")
    forman_doi = "10.1006/aima.1997.1650"
    lines.append(f"DOI: {forman_doi} (Forman, Morse Theory for Cell Complexes)")
    lines.append("")

    try:
        oa_url = unpaywall.get_oa_url(forman_doi)
        if oa_url:
            lines.append(f"  OA URL: {oa_url}")
        else:
            lines.append("  No OA URL found")
    except Exception as e:
        lines.append(f"  ERROR: {e}")
    lines.append("")

    # --- Test 4: PDF extraction backend ---
    lines.append("--- Test 4: PDF extraction ---")
    backend = pdf_to_text.is_available()
    lines.append(f"  Available backend: {backend}")

    # Try downloading + extracting if arXiv returned results
    if arxiv_papers and backend != "none":
        test_id = arxiv_papers[0]["arxiv_id"]
        lines.append(f"  Attempting download + extract for arXiv:{test_id}")
        tmp_dir = os.path.join(
            os.path.dirname(os.path.dirname(os.path.abspath(__file__))),
            "tmp",
        )
        os.makedirs(tmp_dir, exist_ok=True)
        tmp_pdf = os.path.join(tmp_dir, f"test_{test_id.replace('/', '_')}.pdf")

        try:
            arxiv.download_pdf(test_id, tmp_pdf)
            text = pdf_to_text.extract_text(tmp_pdf)
            lines.append(f"  PDF downloaded: {os.path.getsize(tmp_pdf)} bytes")
            lines.append(f"  Extracted text length: {len(text)} chars")
            if text:
                preview = text[:500].replace("\n", " ")
                lines.append(f"  First 500 chars: {preview}")
            # Clean up
            os.unlink(tmp_pdf)
        except Exception as e:
            lines.append(f"  Download/extract ERROR: {e}")
            try:
                os.unlink(tmp_pdf)
            except OSError:
                pass
    elif backend == "none":
        lines.append("  Skipping PDF download test (no extraction backend)")
    else:
        lines.append("  Skipping PDF download test (no arXiv results)")
    lines.append("")

    # --- Test 5: Semantic Scholar get_paper ---
    lines.append("--- Test 5: Semantic Scholar get_paper ---")
    if papers:
        test_paper_id = papers[0]["paperId"]
        lines.append(f"  Paper ID: {test_paper_id}")
        time.sleep(1)
        try:
            detail = semantic_scholar.get_paper(test_paper_id)
            lines.append(f"  Title: {detail.get('title', '(none)')}")
            lines.append(f"  Citations: {detail.get('citationCount', '(none)')}")
        except Exception as e:
            lines.append(f"  ERROR: {e}")
    else:
        lines.append("  Skipping (no papers from search)")
    lines.append("")

    lines.append("=" * 70)
    lines.append("END OF REPORT")
    lines.append("=" * 70)

    return "\n".join(lines)


if __name__ == "__main__":
    report = run_test()
    print(report)

    # Save to tmp/
    out_dir = os.path.join(
        os.path.dirname(os.path.dirname(os.path.abspath(__file__))),
        "tmp",
    )
    os.makedirs(out_dir, exist_ok=True)
    out_path = os.path.join(out_dir, "academic_feed_test.txt")
    with open(out_path, "w", encoding="utf-8") as f:
        f.write(report)
    print(f"\nSaved to {out_path}", file=sys.stderr)
