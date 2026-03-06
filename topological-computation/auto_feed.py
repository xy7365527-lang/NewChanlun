"""auto_feed: Autonomous feeding module for TopologicalDaemon.

Three-layer search (Wikipedia -> Semantic Scholar -> Brave) + topological
quality control. Called from daemon.py gap callbacks.

Pure Python, only stdlib for HTTP (urllib). No external dependencies.
"""

from __future__ import annotations

import json
import os
import sys
import urllib.error
import urllib.parse
import urllib.request
from dataclasses import dataclass

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

from engine import Graph


# ---------------------------------------------------------------------------
# Search layer 1: Wikipedia REST API
# ---------------------------------------------------------------------------

_WIKI_SEARCH_URL = (
    "https://en.wikipedia.org/w/api.php"
    "?action=query&list=search&srsearch={query}&format=json&srlimit=1"
)
_WIKI_SUMMARY_URL = (
    "https://en.wikipedia.org/api/rest_v1/page/summary/{title}"
)

_REQUEST_TIMEOUT = 5  # seconds


def search_wikipedia(query: str) -> str | None:
    """Search Wikipedia and return the top result's extract (up to 2000 chars).

    Returns None on failure or no results.
    """
    try:
        search_url = _WIKI_SEARCH_URL.format(query=urllib.parse.quote(query))
        req = urllib.request.Request(
            search_url,
            headers={"User-Agent": "TopologicalDaemon/1.0 (research)"},
        )
        with urllib.request.urlopen(req, timeout=_REQUEST_TIMEOUT) as resp:
            data = json.loads(resp.read().decode("utf-8"))

        results = data.get("query", {}).get("search", [])
        if not results:
            return None

        title = results[0]["title"]

        summary_url = _WIKI_SUMMARY_URL.format(
            title=urllib.parse.quote(title, safe="")
        )
        req2 = urllib.request.Request(
            summary_url,
            headers={"User-Agent": "TopologicalDaemon/1.0 (research)"},
        )
        with urllib.request.urlopen(req2, timeout=_REQUEST_TIMEOUT) as resp2:
            summary_data = json.loads(resp2.read().decode("utf-8"))

        extract = summary_data.get("extract", "")
        return extract[:2000] if extract else None

    except (urllib.error.URLError, urllib.error.HTTPError, OSError, json.JSONDecodeError):
        return None


# ---------------------------------------------------------------------------
# Search layer 2: Semantic Scholar Graph API
# ---------------------------------------------------------------------------

_SS_SEARCH_URL = (
    "https://api.semanticscholar.org/graph/v1/paper/search"
    "?query={query}&limit=3&fields=abstract"
)


def search_semantic_scholar(query: str) -> str | None:
    """Search Semantic Scholar and return concatenated abstracts (up to 2000 chars).

    Free tier, no API key required. Returns None on failure or no results.
    """
    try:
        url = _SS_SEARCH_URL.format(query=urllib.parse.quote(query))
        req = urllib.request.Request(
            url,
            headers={"User-Agent": "TopologicalDaemon/1.0 (research)"},
        )
        with urllib.request.urlopen(req, timeout=_REQUEST_TIMEOUT) as resp:
            data = json.loads(resp.read().decode("utf-8"))

        papers = data.get("data", [])
        if not papers:
            return None

        abstracts = []
        for paper in papers:
            abstract = paper.get("abstract")
            if abstract:
                abstracts.append(abstract)

        if not abstracts:
            return None

        combined = " ".join(abstracts)
        return combined[:2000]

    except (urllib.error.URLError, urllib.error.HTTPError, OSError, json.JSONDecodeError):
        return None


# ---------------------------------------------------------------------------
# Search layer 3: Brave Search API (optional, needs key)
# ---------------------------------------------------------------------------

_BRAVE_SEARCH_URL = (
    "https://api.search.brave.com/res/v1/web/search?q={query}&count=3"
)


def search_brave(query: str, api_key: str | None = None) -> str | None:
    """Search Brave and return concatenated descriptions (up to 2000 chars).

    Requires BRAVE_API_KEY env var or explicit api_key. Returns None if no key
    or on failure.
    """
    key = api_key or os.environ.get("BRAVE_API_KEY")
    if not key:
        return None

    try:
        url = _BRAVE_SEARCH_URL.format(query=urllib.parse.quote(query))
        req = urllib.request.Request(
            url,
            headers={
                "Accept": "application/json",
                "Accept-Encoding": "gzip",
                "X-Subscription-Token": key,
            },
        )
        with urllib.request.urlopen(req, timeout=_REQUEST_TIMEOUT) as resp:
            data = json.loads(resp.read().decode("utf-8"))

        results = data.get("web", {}).get("results", [])
        if not results:
            return None

        descriptions = []
        for r in results:
            desc = r.get("description")
            if desc:
                descriptions.append(desc)

        if not descriptions:
            return None

        combined = " ".join(descriptions)
        return combined[:2000]

    except (urllib.error.URLError, urllib.error.HTTPError, OSError, json.JSONDecodeError):
        return None


# ---------------------------------------------------------------------------
# Three-layer search
# ---------------------------------------------------------------------------

@dataclass(frozen=True, slots=True)
class SearchResult:
    """Result of a three-layer search."""
    source: str      # 'wikipedia', 'semantic_scholar', 'brave', or 'none'
    text: str | None
    query: str


def three_layer_search(query: str) -> SearchResult:
    """Try Wikipedia -> Semantic Scholar -> Brave in order.

    Returns the first successful result.
    """
    text = search_wikipedia(query)
    if text:
        return SearchResult(source="wikipedia", text=text, query=query)

    text = search_semantic_scholar(query)
    if text:
        return SearchResult(source="semantic_scholar", text=text, query=query)

    text = search_brave(query)
    if text:
        return SearchResult(source="brave", text=text, query=query)

    return SearchResult(source="none", text=None, query=query)


# ---------------------------------------------------------------------------
# Topological quality control
# ---------------------------------------------------------------------------

def _normalize_label(label: str) -> str:
    """Lowercase and strip for matching."""
    return label.lower().strip()


def quality_check(sub_graph: Graph, main_graph: Graph) -> tuple[float, str]:
    """Topological quality control: compute match rate between sub and main.

    match_rate = |V(sub) ∩ V(main)| / |V(sub)|

    Matching uses substring containment on vertex labels (content field).
    Labels shorter than 4 characters are excluded from matching to avoid
    trivial overlaps on common words ("i", "it", "the").

    Returns (match_rate, verdict):
    - match_rate < 0.10:  "garbage"    (almost no connection to existing structure)
    - 0.10 <= rate <= 0.7: "nutritious" (has connections + new content)
    - match_rate > 0.7:    "redundant"  (mostly already exists)
    """
    _MIN_LABEL_LEN = 4

    sub_vids = sub_graph.active_vertex_ids()
    if not sub_vids:
        return 0.0, "garbage"

    # Collect normalized labels from main graph (skip short ones)
    main_labels: list[str] = []
    for vid in main_graph.active_vertex_ids():
        v = main_graph.vertex(vid)
        if v and v.content:
            label = _normalize_label(v.content)
            if len(label) >= _MIN_LABEL_LEN:
                main_labels.append(label)

    if not main_labels:
        # Empty main graph: everything is nutritious (bootstrapping)
        return 0.0, "nutritious"

    # Count sub-graph vertices whose label appears in (or contains) a main label
    matched = 0
    matchable = 0
    for vid in sub_vids:
        v = sub_graph.vertex(vid)
        if not v or not v.content:
            continue
        sub_label = _normalize_label(v.content)
        if len(sub_label) < _MIN_LABEL_LEN:
            continue  # skip trivial labels
        matchable += 1
        for main_label in main_labels:
            if sub_label in main_label or main_label in sub_label:
                matched += 1
                break

    if matchable == 0:
        return 0.0, "garbage"

    match_rate = matched / matchable

    if match_rate < 0.10:
        verdict = "garbage"
    elif match_rate > 0.7:
        verdict = "redundant"
    else:
        verdict = "nutritious"

    return match_rate, verdict


# ---------------------------------------------------------------------------
# Feed from gap
# ---------------------------------------------------------------------------

@dataclass(frozen=True, slots=True)
class FeedRecord:
    """Record of a single feed attempt."""
    accepted: bool
    source: str
    query: str
    vertices_added: int
    edges_added: int
    match_rate: float
    verdict: str
    reason: str  # empty if accepted, otherwise explains rejection


def feed_from_gap(gap: dict | object, daemon: object) -> FeedRecord:
    """Complete feed flow: search -> phi_L -> quality check -> inject.

    gap can be a GapInfo dataclass or a dict with 'search_query' key.
    daemon must have .k_active (Graph) and .feed(text) method.

    Returns a FeedRecord documenting the attempt.
    """
    # Extract query from gap
    if hasattr(gap, "search_query"):
        query = gap.search_query
    elif isinstance(gap, dict):
        query = gap.get("search_query", "")
    else:
        query = str(gap)

    if not query:
        return FeedRecord(
            accepted=False, source="none", query="",
            vertices_added=0, edges_added=0,
            match_rate=0.0, verdict="no_query", reason="no search query provided",
        )

    # Three-layer search
    result = three_layer_search(query)

    if result.text is None:
        return FeedRecord(
            accepted=False, source=result.source, query=query,
            vertices_added=0, edges_added=0,
            match_rate=0.0, verdict="no_results", reason="all search layers returned nothing",
        )

    # phi_L processing
    from phi_L import phi_L
    sub = phi_L(result.text)

    sub_v_count = len(sub.active_vertex_ids())
    sub_e_count = len(sub.active_edges())

    if sub_v_count == 0:
        return FeedRecord(
            accepted=False, source=result.source, query=query,
            vertices_added=0, edges_added=0,
            match_rate=0.0, verdict="empty_parse", reason="phi_L produced empty graph from search text",
        )

    # Quality check
    match_rate, verdict = quality_check(sub, daemon.k_active)

    if verdict == "nutritious":
        daemon.feed(result.text)
        return FeedRecord(
            accepted=True, source=result.source, query=query,
            vertices_added=sub_v_count, edges_added=sub_e_count,
            match_rate=match_rate, verdict=verdict, reason="",
        )
    else:
        return FeedRecord(
            accepted=False, source=result.source, query=query,
            vertices_added=sub_v_count, edges_added=sub_e_count,
            match_rate=match_rate, verdict=verdict,
            reason=f"quality check: {verdict} (match_rate={match_rate:.3f})",
        )


# ---------------------------------------------------------------------------
# Test runner
# ---------------------------------------------------------------------------

def run_test() -> str:
    """Run auto_feed tests with the Hegel complex and output results."""
    from daemon import TopologicalDaemon, _build_graph_from_chapters

    graph, concept_names = _build_graph_from_chapters()
    daemon = TopologicalDaemon(graph=graph, settlement_threshold=15, seed=42)

    lines: list[str] = []
    lines.append("=" * 70)
    lines.append("AUTO_FEED TEST REPORT")
    lines.append("=" * 70)
    lines.append("")

    status = daemon.status()
    lines.append(f"Initial complex: {status['vertices_active']} vertices, "
                 f"{status['edges_active']} edges, beta_1={status['beta_1']}")
    lines.append("")

    # --- Test 1: Known gap queries from Hegel complex ---
    lines.append("--- Test 1: Gap queries from Hegel complex ---")
    lines.append("")

    test_queries = [
        "Bacchanalian revel Hegel",
        "pure being philosophy",
        "the bud dialectics",
        "sense-certainty Hegel",
        "Aufhebung sublation",
        "determinate negation Hegel",
    ]

    for query in test_queries:
        lines.append(f"Query: {query!r}")
        result = three_layer_search(query)
        lines.append(f"  Source: {result.source}")
        if result.text:
            preview = result.text[:120].replace("\n", " ")
            lines.append(f"  Text preview: {preview}...")

            from phi_L import phi_L
            sub = phi_L(result.text)
            match_rate, verdict = quality_check(sub, daemon.k_active)
            lines.append(f"  phi_L: {len(sub.active_vertex_ids())} vertices, "
                         f"{len(sub.active_edges())} edges")
            lines.append(f"  Quality: match_rate={match_rate:.3f}, verdict={verdict}")
        else:
            lines.append(f"  Text: (none)")
        lines.append("")

    # --- Test 2: Out-of-domain queries (should be garbage) ---
    lines.append("--- Test 2: Out-of-domain queries (expect garbage) ---")
    lines.append("")

    ood_queries = [
        "quantum entanglement physics",
        "machine learning transformer attention",
    ]

    for query in ood_queries:
        lines.append(f"Query: {query!r}")
        result = three_layer_search(query)
        lines.append(f"  Source: {result.source}")
        if result.text:
            from phi_L import phi_L
            sub = phi_L(result.text)
            match_rate, verdict = quality_check(sub, daemon.k_active)
            lines.append(f"  phi_L: {len(sub.active_vertex_ids())} vertices, "
                         f"{len(sub.active_edges())} edges")
            lines.append(f"  Quality: match_rate={match_rate:.3f}, verdict={verdict}")
        else:
            lines.append(f"  Text: (none)")
        lines.append("")

    # --- Test 3: Full feed_from_gap flow ---
    lines.append("--- Test 3: Full feed_from_gap flow ---")
    lines.append("")

    status_before = daemon.status()
    lines.append(f"Before feeding: {status_before['vertices_active']} vertices, "
                 f"beta_1={status_before['beta_1']}")
    lines.append("")

    feed_query = "dialectics Hegel philosophy"
    from daemon import GapInfo
    test_gap = GapInfo(
        vertex_id="test",
        content=feed_query,
        degree=1,
        avg_degree=5.0,
        search_query=feed_query,
    )

    record = feed_from_gap(test_gap, daemon)
    lines.append(f"Feed query: {feed_query!r}")
    lines.append(f"  Accepted: {record.accepted}")
    lines.append(f"  Source: {record.source}")
    lines.append(f"  Vertices added: {record.vertices_added}")
    lines.append(f"  Edges added: {record.edges_added}")
    lines.append(f"  Match rate: {record.match_rate:.3f}")
    lines.append(f"  Verdict: {record.verdict}")
    if record.reason:
        lines.append(f"  Reason: {record.reason}")
    lines.append("")

    status_after = daemon.status()
    lines.append(f"After feeding: {status_after['vertices_active']} vertices, "
                 f"beta_1={status_after['beta_1']}")

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
    out_path = os.path.join(out_dir, "auto_feed_test.txt")
    with open(out_path, "w", encoding="utf-8") as f:
        f.write(report)
    print(f"\nSaved to {out_path}", file=sys.stderr)
