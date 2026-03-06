"""Unpaywall API: DOI -> open access full text URL.

Free, needs email for identification.
Pure stdlib (urllib).
"""

from __future__ import annotations

import json
import urllib.error
import urllib.parse
import urllib.request

_BASE_URL = "https://api.unpaywall.org/v2"
_REQUEST_TIMEOUT = 10
_DEFAULT_EMAIL = "topo-compute@example.com"


def get_oa_url(doi: str, email: str = _DEFAULT_EMAIL) -> str | None:
    """Query Unpaywall for open access URL.

    Returns best available OA URL, or None if no OA version found.
    """
    url = (
        f"{_BASE_URL}/{urllib.parse.quote(doi, safe='')}"
        f"?email={urllib.parse.quote(email, safe='@.')}"
    )
    req = urllib.request.Request(url)
    try:
        with urllib.request.urlopen(req, timeout=_REQUEST_TIMEOUT) as resp:
            data = json.loads(resp.read().decode("utf-8"))
    except (urllib.error.URLError, urllib.error.HTTPError, OSError,
            json.JSONDecodeError):
        return None

    # Try best_oa_location first, then iterate oa_locations
    best = data.get("best_oa_location")
    if best:
        url_for_pdf = best.get("url_for_pdf") or best.get("url")
        if url_for_pdf:
            return url_for_pdf

    for loc in data.get("oa_locations", []):
        url_for_pdf = loc.get("url_for_pdf") or loc.get("url")
        if url_for_pdf:
            return url_for_pdf

    return None
