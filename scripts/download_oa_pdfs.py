#!/usr/bin/env python3
"""
Download OA (Open Access) PDFs for S_net corpus.
Only downloads from legitimate free sources (author pages, Project Gutenberg, academic archives).

Usage:
    python scripts/download_oa_pdfs.py [--output-dir DIR]
"""

import argparse
import os
import time
import urllib.request
import urllib.error


# Sources verified as legitimate free downloads (not pirated)
OA_SOURCES = [
    # Academic papers (author/university hosted)
    {
        "id": "38-forman-morse",
        "title": "Forman - Morse Theory for Cell Complexes (1998)",
        "url": "https://webhomes.maths.ed.ac.uk/~v1ranick/papers/forman5.pdf",
        "filename": "forman_morse_theory_cell_complexes_1998.pdf",
        "category": "math",
    },
    # Free textbooks (author-provided)
    {
        "id": "39-hatcher-algtop",
        "title": "Hatcher - Algebraic Topology (free by author)",
        "url": "https://pi.math.cornell.edu/~hatcher/AT/AT.pdf",
        "filename": "hatcher_algebraic_topology.pdf",
        "category": "math",
    },
    # Academic archive PDFs
    {
        "id": "42-kozlov-cat",
        "title": "Kozlov - Combinatorial Algebraic Topology",
        "url": "https://webhomes.maths.ed.ac.uk/~v1ranick/papers/kozlov.pdf",
        "filename": "kozlov_combinatorial_algebraic_topology.pdf",
        "category": "math",
    },
    # Diestel free electronic edition (older version, author-sanctioned)
    {
        "id": "41-diestel-graph",
        "title": "Diestel - Graph Theory (Electronic Edition 2010)",
        "url": "https://www.math.uni-hamburg.de/home/diestel/books/graph.theory/GraphTheoryIII.pdf",
        "filename": "diestel_graph_theory_electronic_2010.pdf",
        "category": "math",
    },
    # Public domain: Project Gutenberg
    {
        "id": "32a-nietzsche-genealogy",
        "title": "Nietzsche - Genealogy of Morals (Gutenberg)",
        "url": "https://www.gutenberg.org/files/52319/52319-h/52319-h.htm",
        "filename": "nietzsche_genealogy_of_morals.html",
        "category": "philosophy-pd",
    },
    {
        "id": "35-spinoza-ethics",
        "title": "Spinoza - Ethics complete (Gutenberg)",
        "url": "https://www.gutenberg.org/files/3800/3800-h/3800-h.htm",
        "filename": "spinoza_ethics_complete.html",
        "category": "philosophy-pd",
    },
    # Public domain: marxists.org
    {
        "id": "27-marx-capital-v1",
        "title": "Marx - Capital Vol 1 (marxists.org PDF)",
        "url": "https://www.marxists.org/archive/marx/works/download/pdf/Capital-Volume-I.pdf",
        "filename": "marx_capital_vol1.pdf",
        "category": "philosophy-pd",
    },
    # Academic sharing (scholarly hosted)
    {
        "id": "05-bottomore-marxist-dict",
        "title": "Bottomore - A Dictionary of Marxist Thought",
        "url": "https://gruppegrundrisse.files.wordpress.com/2012/06/bottomore-a-dictionary-of-marxist-thought.pdf",
        "filename": "bottomore_dictionary_marxist_thought.pdf",
        "category": "philosophy-dict",
    },
]


def download_file(url: str, filepath: str) -> bool:
    """Download a file from url to filepath. Returns True on success."""
    try:
        req = urllib.request.Request(
            url,
            headers={"User-Agent": "Mozilla/5.0 (academic-corpus-builder)"},
        )
        with urllib.request.urlopen(req, timeout=120) as response:
            with open(filepath, "wb") as f:
                while True:
                    chunk = response.read(8192)
                    if not chunk:
                        break
                    f.write(chunk)
        return True
    except (urllib.error.URLError, urllib.error.HTTPError, OSError) as e:
        print(f"  ERROR: {e}")
        return False


def main():
    parser = argparse.ArgumentParser(description="Download OA PDFs for S_net corpus")
    parser.add_argument(
        "--output-dir",
        default="tmp/oa-downloads",
        help="Output directory for downloaded files (default: tmp/oa-downloads)",
    )
    parser.add_argument(
        "--category",
        choices=["math", "philosophy-pd", "philosophy-dict", "all"],
        default="all",
        help="Only download specific category",
    )
    args = parser.parse_args()

    os.makedirs(args.output_dir, exist_ok=True)

    sources = OA_SOURCES
    if args.category != "all":
        sources = [s for s in sources if s["category"] == args.category]

    print(f"Downloading {len(sources)} files to {args.output_dir}/\n")

    results = {"success": [], "failed": []}

    for src in sources:
        filepath = os.path.join(args.output_dir, src["filename"])
        if os.path.exists(filepath):
            print(f"[SKIP] {src['title']} (already exists)")
            results["success"].append(src["id"])
            continue

        print(f"[DOWN] {src['title']}")
        print(f"       {src['url']}")

        ok = download_file(src["url"], filepath)
        if ok:
            size_mb = os.path.getsize(filepath) / (1024 * 1024)
            print(f"  OK ({size_mb:.1f} MB)")
            results["success"].append(src["id"])
        else:
            results["failed"].append(src["id"])

        # Be polite: wait between requests
        time.sleep(1)

    print(f"\n--- Summary ---")
    print(f"Success: {len(results['success'])}/{len(sources)}")
    print(f"Failed:  {len(results['failed'])}/{len(sources)}")
    if results["failed"]:
        print(f"Failed IDs: {', '.join(results['failed'])}")


if __name__ == "__main__":
    main()
