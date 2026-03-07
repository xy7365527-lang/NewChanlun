"""Ingest DeepSeek-V3 and DeepSeek-R1 papers into topological graphs.

Downloads PDFs from arXiv, extracts text, processes through phi_L,
and saves as JSON files compatible with daemon.py's graph_to_dict format.

Usage:
    python ingest_deepseek_papers.py
"""

from __future__ import annotations

import json
import os
import sys
import urllib.request

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

from feeds.pdf_to_text import extract_text
from phi_L import phi_L
from daemon import graph_to_dict
from engine import compute_beta_1


PAPERS = [
    {
        "arxiv_id": "2412.19437",
        "name": "DeepSeek-V3",
        "filename": "deepseek_v3",
    },
    {
        "arxiv_id": "2501.12948",
        "name": "DeepSeek-R1",
        "filename": "deepseek_r1",
    },
]

ARXIV_PDF_URL = "https://arxiv.org/pdf/{arxiv_id}"
TMP_DIR = os.path.join(os.path.dirname(os.path.abspath(__file__)), "tmp")
OUTPUT_DIR = os.path.join(os.path.dirname(os.path.abspath(__file__)), "data")


def download_pdf(arxiv_id: str, save_path: str) -> str:
    """Download PDF from arXiv. Returns local path."""
    url = ARXIV_PDF_URL.format(arxiv_id=arxiv_id)
    os.makedirs(os.path.dirname(save_path), exist_ok=True)
    if os.path.exists(save_path) and os.path.getsize(save_path) > 10000:
        print(f"  [skip] PDF already exists: {save_path} ({os.path.getsize(save_path)} bytes)")
        return save_path
    print(f"  Downloading {url} ...")
    urllib.request.urlretrieve(url, save_path)
    print(f"  Saved: {save_path} ({os.path.getsize(save_path)} bytes)")
    return save_path


def truncate_for_phi_l(text: str, max_chars: int = 15000) -> str:
    """Truncate text to a manageable size for phi_L.

    phi_L does NLP dependency parsing which is O(n^2) per sentence.
    We take the abstract + introduction + key architecture sections.
    """
    # Split into sections and take the most informative parts
    lines = text.split("\n")
    sections: list[str] = []
    current_section: list[str] = []
    total_chars = 0

    for line in lines:
        current_section.append(line)
        total_chars += len(line) + 1
        if total_chars >= max_chars:
            break

    return "\n".join(current_section)


def ingest_paper(paper: dict) -> dict:
    """Ingest a single paper: download -> extract -> phi_L -> save."""
    name = paper["name"]
    arxiv_id = paper["arxiv_id"]
    filename = paper["filename"]

    print(f"\n{'='*60}")
    print(f"Processing: {name} (arXiv:{arxiv_id})")
    print(f"{'='*60}")

    # Step 1: Download PDF
    pdf_path = os.path.join(TMP_DIR, f"{filename}.pdf")
    download_pdf(arxiv_id, pdf_path)

    # Step 2: Extract text
    print("  Extracting text from PDF...")
    full_text = extract_text(pdf_path)
    print(f"  Full text: {len(full_text)} chars")

    # Step 3: Truncate for phi_L processing
    text = truncate_for_phi_l(full_text, max_chars=15000)
    print(f"  Truncated for phi_L: {len(text)} chars")

    # Step 4: Process through phi_L
    print("  Running phi_L...")
    graph = phi_L(text)

    n_vertices = len(graph.active_vertex_ids())
    n_edges = len(graph.active_edges())
    beta_1 = compute_beta_1(graph)

    print(f"  Result: {n_vertices} vertices, {n_edges} edges, beta_1={beta_1}")

    # Step 5: Save as JSON
    os.makedirs(OUTPUT_DIR, exist_ok=True)
    output_path = os.path.join(OUTPUT_DIR, f"{filename}_graph.json")
    output_data = {
        "source": f"arXiv:{arxiv_id}",
        "paper_name": name,
        "text_chars": len(text),
        "full_text_chars": len(full_text),
        "graph_data": graph_to_dict(graph),
        "stats": {
            "vertices": n_vertices,
            "edges": n_edges,
            "beta_1": beta_1,
        },
    }
    with open(output_path, "w", encoding="utf-8") as f:
        json.dump(output_data, f, indent=2, ensure_ascii=False)
    print(f"  Saved: {output_path}")

    return output_data["stats"]


def main() -> None:
    os.makedirs(TMP_DIR, exist_ok=True)
    os.makedirs(OUTPUT_DIR, exist_ok=True)

    total_vertices = 0
    total_edges = 0

    for paper in PAPERS:
        stats = ingest_paper(paper)
        total_vertices += stats["vertices"]
        total_edges += stats["edges"]

    print(f"\n{'='*60}")
    print(f"SUMMARY")
    print(f"{'='*60}")
    print(f"  Total vertices: {total_vertices}")
    print(f"  Total edges:    {total_edges}")
    print(f"  Papers:         {len(PAPERS)}")
    print(f"  Output dir:     {OUTPUT_DIR}")


if __name__ == "__main__":
    main()
