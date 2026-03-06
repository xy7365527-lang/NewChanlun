"""arXiv API client: search + download PDF.

Pure stdlib (urllib, xml.etree).
"""

from __future__ import annotations

import os
import urllib.error
import urllib.parse
import urllib.request
import xml.etree.ElementTree as ET

_BASE_URL = "http://export.arxiv.org/api/query"
_PDF_URL = "https://arxiv.org/pdf/{arxiv_id}.pdf"
_REQUEST_TIMEOUT = 10
_ATOM_NS = "{http://www.w3.org/2005/Atom}"


def search(query: str, max_results: int = 5) -> list[dict]:
    """Search arXiv.

    Returns list of dicts with keys: arxiv_id, title, abstract, pdf_url.
    """
    url = (
        f"{_BASE_URL}"
        f"?search_query=all:{urllib.parse.quote(query)}"
        f"&max_results={max_results}"
    )
    try:
        with urllib.request.urlopen(url, timeout=_REQUEST_TIMEOUT) as resp:
            xml_bytes = resp.read()
    except (urllib.error.URLError, urllib.error.HTTPError, OSError):
        return []

    try:
        root = ET.fromstring(xml_bytes)
    except ET.ParseError:
        return []

    results = []
    for entry in root.findall(f"{_ATOM_NS}entry"):
        # Extract arXiv ID from the <id> URL
        id_elem = entry.find(f"{_ATOM_NS}id")
        if id_elem is None or id_elem.text is None:
            continue
        # id_elem.text looks like "http://arxiv.org/abs/0801.0955v1"
        raw_id = id_elem.text.strip().rsplit("/", 1)[-1]
        # Strip version suffix for stable ID
        arxiv_id = raw_id.split("v")[0] if "v" in raw_id else raw_id

        title_elem = entry.find(f"{_ATOM_NS}title")
        title = (title_elem.text or "").strip().replace("\n", " ") if title_elem is not None else ""

        summary_elem = entry.find(f"{_ATOM_NS}summary")
        abstract = (summary_elem.text or "").strip().replace("\n", " ") if summary_elem is not None else ""

        results.append({
            "arxiv_id": arxiv_id,
            "title": title,
            "abstract": abstract,
            "pdf_url": _PDF_URL.format(arxiv_id=arxiv_id),
        })

    return results


def download_pdf(arxiv_id: str, save_path: str) -> str:
    """Download PDF from arXiv. Returns local path.

    Creates parent directories if needed.
    Raises on network/IO failure.
    """
    pdf_url = _PDF_URL.format(arxiv_id=arxiv_id)
    os.makedirs(os.path.dirname(os.path.abspath(save_path)), exist_ok=True)
    urllib.request.urlretrieve(pdf_url, save_path)
    return save_path
