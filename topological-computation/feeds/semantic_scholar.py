"""Semantic Scholar Graph API client.

Free, 100 requests/5min, no API key needed.
Pure stdlib (urllib).
"""

from __future__ import annotations

import json
import urllib.error
import urllib.parse
import urllib.request

_BASE_URL = "https://api.semanticscholar.org/graph/v1/paper"
_REQUEST_TIMEOUT = 10
_USER_AGENT = "TopologicalDaemon/1.0 (research)"
_FIELDS = "title,abstract,citationCount,externalIds"


def search(query: str, limit: int = 10) -> list[dict]:
    """Search papers.

    Returns list of dicts with keys:
        paperId, title, abstract, citationCount, doi, arxivId
    Sorted by citationCount descending.
    """
    url = (
        f"{_BASE_URL}/search"
        f"?query={urllib.parse.quote(query)}"
        f"&limit={limit}"
        f"&fields={_FIELDS}"
    )
    req = urllib.request.Request(
        url, headers={"User-Agent": _USER_AGENT}
    )
    try:
        with urllib.request.urlopen(req, timeout=_REQUEST_TIMEOUT) as resp:
            data = json.loads(resp.read().decode("utf-8"))
    except (urllib.error.URLError, urllib.error.HTTPError, OSError,
            json.JSONDecodeError):
        return []

    papers = data.get("data", [])
    results = []
    for p in papers:
        ext = p.get("externalIds") or {}
        results.append({
            "paperId": p.get("paperId", ""),
            "title": p.get("title", ""),
            "abstract": p.get("abstract", ""),
            "citationCount": p.get("citationCount", 0) or 0,
            "doi": ext.get("DOI", ""),
            "arxivId": ext.get("ArXiv", ""),
        })
    results.sort(key=lambda r: r["citationCount"], reverse=True)
    return results


def get_paper(paper_id: str) -> dict:
    """Get paper details by Semantic Scholar paper ID.

    Returns dict with paperId, title, abstract, citationCount, doi, arxivId.
    Returns empty dict on failure.
    """
    url = (
        f"{_BASE_URL}/{urllib.parse.quote(paper_id, safe='')}"
        f"?fields={_FIELDS}"
    )
    req = urllib.request.Request(
        url, headers={"User-Agent": _USER_AGENT}
    )
    try:
        with urllib.request.urlopen(req, timeout=_REQUEST_TIMEOUT) as resp:
            data = json.loads(resp.read().decode("utf-8"))
    except (urllib.error.URLError, urllib.error.HTTPError, OSError,
            json.JSONDecodeError):
        return {}

    ext = data.get("externalIds") or {}
    return {
        "paperId": data.get("paperId", ""),
        "title": data.get("title", ""),
        "abstract": data.get("abstract", ""),
        "citationCount": data.get("citationCount", 0) or 0,
        "doi": ext.get("DOI", ""),
        "arxivId": ext.get("ArXiv", ""),
    }
