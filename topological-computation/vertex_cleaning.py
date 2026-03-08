"""Three-layer concept graph architecture — vertex cleaning protocol.

K_active vertices have exactly three legal sources:
  Layer 1 (Core)       — hand-written concept graphs (philosophy, chanlun, code topology)
  Layer 2 (Growth)     — sublation synthesis vertices (syn_ prefix)
  Layer 3 (Connection) — encounter-discovered cross-domain edges

phi_L is NOT a concept generator — it only does whitelist term matching.
This module implements is_garbage() and the full cleaning pipeline,
including encounter_log + settlement_history writes.

Pure Python, no external dependencies beyond engine.py / traversal_checkpoint.py.
"""

from __future__ import annotations

import json
import re
import time
from dataclasses import dataclass, asdict
from pathlib import Path
from typing import Optional

from file_lock import locked_append


# ---------------------------------------------------------------------------
# Core whitelist — chanlun terms that phi_L is allowed to anchor
# ---------------------------------------------------------------------------

CHANLUN_WHITELIST: frozenset[str] = frozenset({
    "笔", "段", "中枢", "走势", "走势类型", "背驰", "盘整背驰", "趋势背驰",
    "区间套", "级别", "递归", "买点", "卖点", "第一类买点", "第二类买点", "第三类买点",
    "第一类卖点", "第二类卖点", "第三类卖点", "盘整", "趋势", "上涨", "下跌",
    "线段", "特征序列", "分型", "顶分型", "底分型", "包含关系",
    "中枢扩展", "中枢新生", "中枢扩张", "同级别分解", "结合律",
    "力度", "MACD", "面积", "斜率",
})

# Philosophy vertex ID prefixes (hand-written experiment graphs)
_PHILOSOPHY_PREFIXES: tuple[str, ...] = (
    "hegel_", "lacan_", "marx_", "derrida_", "deleuze_", "nietzsche_",
    "foucault_", "spinoza_", "schelling_", "wittgenstein_", "heidegger_",
    "mao_", "lenin_", "holderlin_", "simmel_", "merleau_", "postman_",
    "zizek_", "adorno_", "benjamin_",
)


# ---------------------------------------------------------------------------
# Source classification — infer source from vertex ID/content patterns
# ---------------------------------------------------------------------------

def _classify_source(vid: str, content: str) -> str:
    """Infer the layer source of a vertex from its ID and content patterns.

    Returns one of: 'core', 'sublation', 'articulation_bridge', 'encounter', 'phi_L', 'unknown'.
    """
    # Layer 2: sublation synthesis vertices
    if vid.startswith("syn_") or vid.startswith("anti_"):
        return "sublation"

    # Articulation bridge vertices (from orphan signifier bridging)
    if vid.startswith("bridge_"):
        return "articulation_bridge"

    # Code topology vertices (Layer 1 core — code structure)
    if ":" in vid:
        return "core"

    # Block topology vertices (64-char hex hash from .chanlun/block-topology)
    if re.match(r"^[0-9a-f]{64}$", vid):
        return "core"

    # phi_L extracted vertices (c_ + 12 hex chars)
    if re.match(r"^c_[0-9a-f]{12}$", vid):
        return "phi_L"

    # Philosophy graph vertices (descriptive IDs with known prefixes)
    if any(vid.startswith(p) for p in _PHILOSOPHY_PREFIXES):
        return "core"

    # Hand-written philosophy/experiment graph vertices (descriptive IDs)
    # These come from experiment_*.py files — human-readable IDs
    # like "truth_whole", "substance", "dialectic_method", etc.
    if re.match(r"^[a-zA-Z_][a-zA-Z0-9_]*$", vid) and len(vid) < 100:
        return "core"

    # Chinese descriptive IDs from hand-written graphs
    if vid and not vid.startswith("c_") and len(vid) < 100:
        return "core"

    return "unknown"


# ---------------------------------------------------------------------------
# Garbage detection
# ---------------------------------------------------------------------------

_META_LEAK_PATTERNS = [
    re.compile(r"\[rewrite\]", re.IGNORECASE),
    re.compile(r"\[meta\]", re.IGNORECASE),
    re.compile(r"keys=\["),
    re.compile(r"https?://\S+"),
    # Pure date strings
    re.compile(r"^\d{2,4}[-/]\d"),
]


def is_garbage(
    vid: str,
    content: str,
    degree: int,
    core_whitelist: frozenset[str] = CHANLUN_WHITELIST,
) -> tuple[bool, str]:
    """Determine whether a vertex is garbage.

    Returns (is_garbage: bool, reason: str).
    Empty reason means the vertex is clean.

    Protection rules (never garbage):
      - Code topology vertices (ID contains ':')
      - Synthesis vertices (syn_/anti_ prefix)
      - Block topology vertices (64-char hex hash)
      - Philosophy graph vertices (known prefixes or descriptive IDs)

    Garbage rules (in priority order):
      1. Empty or trivially short content (<=2 chars)
      2. Metadata leakage patterns
      3. phi_L vertex whose content is NOT in the core whitelist
      4. Degree-0 non-core vertices (isolated, contributes nothing)
      5. Unknown source
    """
    source = _classify_source(vid, content)

    # Protected sources — never garbage
    if source == "core":
        return False, ""
    if source == "sublation":
        return False, ""
    if source == "articulation_bridge":
        return False, ""

    # Rule 1: Empty or trivially short content
    if not content or not content.strip():
        return True, "empty_content"
    stripped = content.strip()
    if len(stripped) <= 2:
        return True, f"trivially_short: {stripped!r}"

    # Rule 2: Metadata leakage
    for pattern in _META_LEAK_PATTERNS:
        if pattern.search(content):
            return True, f"metadata_leakage: {pattern.pattern}"

    # Rule 3: phi_L vertices — whitelist check
    if source == "phi_L":
        if stripped in core_whitelist:
            return False, ""
        return True, f"phi_L_not_in_whitelist: {stripped!r}"

    # Rule 4: Degree-0 non-core vertices (isolated nodes contribute nothing)
    if degree == 0:
        return True, f"zero_degree_non_core: source={source}"

    # Rule 5: Unknown source
    if source == "unknown":
        return True, f"unknown_source: id={vid!r}"

    return False, ""


# ---------------------------------------------------------------------------
# Cleaning report
# ---------------------------------------------------------------------------

@dataclass(frozen=True, slots=True)
class CleaningReport:
    """Result of cleaning a merged graph."""
    total_vertices: int
    total_edges: int
    garbage_vertices: int
    kept_vertices: int
    removed_edges: int
    kept_edges: int
    garbage_reasons: dict[str, int]
    garbage_samples: list[dict]


# ---------------------------------------------------------------------------
# Cleaning pipeline
# ---------------------------------------------------------------------------

def clean_graph(
    merged: dict,
    core_whitelist: frozenset[str] = CHANLUN_WHITELIST,
) -> tuple[dict, CleaningReport]:
    """Clean a merged graph (k_merged.json format).

    Args:
        merged: dict with 'vertices' (list) and 'edges' (list)
        core_whitelist: set of allowed Chinese terms for phi_L vertices

    Returns:
        (cleaned_graph, report)
    """
    vertices = merged.get("vertices", [])
    edges = merged.get("edges", [])

    # Compute degrees
    degree: dict[str, int] = {}
    for e in edges:
        src = e.get("source", "")
        tgt = e.get("target", "")
        degree[src] = degree.get(src, 0) + 1
        degree[tgt] = degree.get(tgt, 0) + 1

    # Classify each vertex
    garbage_ids: set[str] = set()
    reason_counts: dict[str, int] = {}
    garbage_samples: list[dict] = []

    for v in vertices:
        vid = v.get("id", "")
        content = v.get("content", "") or ""
        deg = degree.get(vid, 0)

        is_bad, reason = is_garbage(vid, content, deg, core_whitelist)
        if is_bad:
            garbage_ids.add(vid)
            # Aggregate reason category (before the colon)
            category = reason.split(":")[0].strip()
            reason_counts[category] = reason_counts.get(category, 0) + 1
            if len(garbage_samples) < 50:
                garbage_samples.append({
                    "id": vid[:40],
                    "content": content[:80],
                    "reason": reason,
                    "degree": deg,
                })

    # Filter vertices and edges
    kept_vertices = [v for v in vertices if v["id"] not in garbage_ids]
    kept_edges = [
        e for e in edges
        if e["source"] not in garbage_ids and e["target"] not in garbage_ids
    ]

    cleaned = {
        "vertices": kept_vertices,
        "edges": kept_edges,
    }

    report = CleaningReport(
        total_vertices=len(vertices),
        total_edges=len(edges),
        garbage_vertices=len(garbage_ids),
        kept_vertices=len(kept_vertices),
        removed_edges=len(edges) - len(kept_edges),
        kept_edges=len(kept_edges),
        garbage_reasons=reason_counts,
        garbage_samples=garbage_samples,
    )

    return cleaned, report


# ---------------------------------------------------------------------------
# Encounter log + settlement history writes
# ---------------------------------------------------------------------------

def _write_cleaning_encounter(
    encounter_path: Path,
    garbage_count: int,
    kept_count: int,
    removed_edges: int,
    reasons: dict[str, int],
) -> None:
    """Append a cleaning event to the encounter log (JSONL)."""
    entry = {
        "ts": time.time(),
        "step": -1,  # cleaning is a meta-operation, not a traversal step
        "operation": "vertex_cleaning",
        "position": "k_merged",
        "beta_1_before": -1,
        "beta_1_after": -1,
        "f_value": 0,
        "blocked": False,
        "context": (
            f"[cleaning] removed {garbage_count} garbage vertices, "
            f"{removed_edges} orphaned edges. "
            f"kept {kept_count} vertices. "
            f"reasons: {json.dumps(reasons, ensure_ascii=False)}"
        )[:200],
    }
    with locked_append(encounter_path) as fh:
        fh.write(json.dumps(entry, ensure_ascii=False) + "\n")


def _write_cleaning_settlement(
    settlement_path: Path,
    garbage_count: int,
    reasons: dict[str, int],
) -> None:
    """Append a cleaning settlement record (JSONL)."""
    entry = {
        "ts": time.time(),
        "type": "cleaning",
        "step": -1,
        "garbage_removed": garbage_count,
        "reasons": reasons,
        "ruling": (
            "Three-layer vertex cleaning: phi_L whitelist enforced, "
            "metadata leakage removed, zero-degree non-core vertices pruned."
        ),
    }
    with locked_append(settlement_path) as fh:
        fh.write(json.dumps(entry, ensure_ascii=False) + "\n")


# ---------------------------------------------------------------------------
# CLI entry point
# ---------------------------------------------------------------------------

def main() -> None:
    """Load k_merged.json, clean it, save results + logs."""
    import sys

    swarm_dir = Path.home() / ".swarm"
    merged_path = swarm_dir / "k_merged.json"
    output_dir = swarm_dir / "output"
    output_dir.mkdir(parents=True, exist_ok=True)
    checkpoint_dir = swarm_dir / "checkpoint"

    if not merged_path.is_file():
        print(f"ERROR: {merged_path} not found")
        sys.exit(1)

    print(f"Loading {merged_path} ...")
    with open(merged_path, "r", encoding="utf-8") as f:
        merged = json.load(f)

    cleaned, report = clean_graph(merged)

    # Save cleaning report
    report_path = output_dir / "cleaning_report.json"
    with open(report_path, "w", encoding="utf-8") as f:
        json.dump(asdict(report), f, ensure_ascii=False, indent=2)
    print(f"Cleaning report saved to {report_path}")

    # Save cleaned graph
    cleaned_path = swarm_dir / "k_merged_cleaned.json"
    with open(cleaned_path, "w", encoding="utf-8") as f:
        json.dump(cleaned, f, ensure_ascii=False)
    print(f"Cleaned graph saved to {cleaned_path}")

    # Write encounter log entry
    encounter_path = checkpoint_dir / "default_encounters.jsonl"
    _write_cleaning_encounter(
        encounter_path,
        garbage_count=report.garbage_vertices,
        kept_count=report.kept_vertices,
        removed_edges=report.removed_edges,
        reasons=report.garbage_reasons,
    )
    print(f"Encounter log appended to {encounter_path}")

    # Write settlement history entry
    settlement_path = checkpoint_dir / "default_settlements.jsonl"
    _write_cleaning_settlement(
        settlement_path,
        garbage_count=report.garbage_vertices,
        reasons=report.garbage_reasons,
    )
    print(f"Settlement history appended to {settlement_path}")

    # Print summary
    print(f"\n{'='*60}")
    print(f"Cleaning Summary")
    print(f"{'='*60}")
    print(f"Total vertices:   {report.total_vertices}")
    print(f"Garbage removed:  {report.garbage_vertices}")
    print(f"Kept vertices:    {report.kept_vertices}")
    print(f"Total edges:      {report.total_edges}")
    print(f"Removed edges:    {report.removed_edges}")
    print(f"Kept edges:       {report.kept_edges}")
    print(f"\nGarbage by reason:")
    for reason, count in sorted(report.garbage_reasons.items(), key=lambda x: -x[1]):
        print(f"  {reason}: {count}")
    print(f"\nSample garbage vertices (first 10):")
    for s in report.garbage_samples[:10]:
        print(f"  [{s['reason']}] id={s['id']} content={s['content']}")


if __name__ == "__main__":
    main()
