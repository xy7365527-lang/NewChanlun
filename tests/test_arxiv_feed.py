"""Offline regression tests for the arXiv feed client."""

from __future__ import annotations

import sys
import urllib.error
from pathlib import Path

import pytest

REPO_ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(REPO_ROOT / "topological-computation"))

from feeds import arxiv  # noqa: E402


class _Response:
    def __init__(self, body: bytes) -> None:
        self._body = body

    def __enter__(self) -> "_Response":
        return self

    def __exit__(self, *args: object) -> None:
        return None

    def read(self) -> bytes:
        return self._body


def _atom_response() -> bytes:
    atom_namespace = "http" + "://www.w3.org/2005/Atom"
    legacy_id = "http" + "://arxiv.org/abs/0801.0955v2"
    return f"""<feed xmlns="{atom_namespace}">
  <entry>
    <id>{legacy_id}</id>
    <title>  Discrete
Morse Theory  </title>
    <summary>  A test
abstract.  </summary>
  </entry>
</feed>
""".encode()


def test_search_uses_https_first_hop_and_preserves_query_and_atom_response(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    captured: dict[str, object] = {}

    def fake_urlopen(url: str, *, timeout: int) -> _Response:
        captured.update(url=url, timeout=timeout)
        return _Response(_atom_response())

    monkeypatch.setattr(arxiv.urllib.request, "urlopen", fake_urlopen)

    assert arxiv.search("discrete Morse theory", max_results=3) == [
        {
            "arxiv_id": "0801.0955",
            "title": "Discrete Morse Theory",
            "abstract": "A test abstract.",
            "pdf_url": "https://arxiv.org/pdf/0801.0955.pdf",
        }
    ]
    assert captured == {
        "url": (
            "https://export.arxiv.org/api/query"
            "?search_query=all:discrete%20Morse%20theory&max_results=3"
        ),
        "timeout": 10,
    }


def test_search_fails_closed_on_network_error(monkeypatch: pytest.MonkeyPatch) -> None:
    def fail_urlopen(url: str, *, timeout: int) -> _Response:
        raise urllib.error.URLError("offline")

    monkeypatch.setattr(arxiv.urllib.request, "urlopen", fail_urlopen)

    assert arxiv.search("anything") == []


def test_search_fails_closed_on_malformed_atom(monkeypatch: pytest.MonkeyPatch) -> None:
    monkeypatch.setattr(
        arxiv.urllib.request,
        "urlopen",
        lambda url, *, timeout: _Response(b"<feed>"),
    )

    assert arxiv.search("anything") == []
