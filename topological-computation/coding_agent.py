"""Topological Coding Agent: task -> locate -> traverse -> transplant -> execute -> verify.

Programming is not text generation -- it's topological transplant in code space.

Pure Python, only stdlib. No external dependencies.
"""

from __future__ import annotations

import json
import subprocess
from pathlib import Path

from engine import Graph, compute_beta_1, EdgeType
from code_ingest import ingest_directory
from code_search import find_pattern, search_by_keyword, search_by_topology
from code_transplant import transplant_function, extract_function_ast
from execution_callback import run_tests


class CodingAgent:
    """Interactive agent that operates on code through topological operations."""

    def __init__(self, workspace: str) -> None:
        """Initialize with a workspace directory (code to work on)."""
        self.workspace = str(Path(workspace).resolve())
        self.graph = ingest_directory(self.workspace)
        self.history: list[dict] = []

    def task(self, description: str) -> dict:
        """Execute a coding task.

        1. Parse task description -> extract what to do
        2. Search code complex for relevant patterns
        3. Plan: what to add/modify/remove
        4. Report findings (does not auto-modify in initial mode)
        """
        # Step 1: Search for relevant code
        search_results = find_pattern(self.graph, description)

        # Step 2: Analyze the graph around matches
        analysis: list[dict] = []
        for hit in search_results[:5]:
            vid = hit["id"]
            neighbors = self.graph.neighbors(vid)
            in_n = self.graph.in_neighbors(vid)
            out_n = self.graph.out_neighbors(vid)
            analysis.append({
                "vertex": vid,
                "content_preview": hit.get("content_preview", "")[:100],
                "in_neighbors": in_n,
                "out_neighbors": out_n,
                "match_keywords": hit.get("match_keywords", []),
            })

        # Step 3: Compute graph stats
        beta_1 = compute_beta_1(self.graph)
        active_count = len(self.graph.active_vertex_ids())
        edge_count = len(self.graph.active_edges())

        result = {
            "task": description,
            "search_hits": len(search_results),
            "top_matches": analysis,
            "graph_stats": {
                "active_vertices": active_count,
                "active_edges": edge_count,
                "beta_1": beta_1,
            },
            "status": "analysis_complete",
        }

        self.history.append(result)
        return result

    def analyze(self) -> dict:
        """Analyze workspace code: fold candidates, potential gaps, code graph statistics."""
        active_vids = self.graph.active_vertex_ids()
        active_edges = self.graph.active_edges()
        beta_1 = compute_beta_1(self.graph)

        # Find fold candidates: vertices with high topological similarity
        # Sample up to 50 non-import vertices to keep O(n) manageable
        fold_candidates: list[dict] = []
        checked: set[frozenset[str]] = set()
        candidate_vids = [v for v in active_vids if not v.startswith("<import>.")]
        sample = candidate_vids[:50]

        for vid in sample:
            similar = search_by_topology(self.graph, vid, max_results=3)
            for hit in similar:
                pair = frozenset((vid, hit["id"]))
                if pair in checked:
                    continue
                checked.add(pair)
                if hit["similarity_score"] > 0.7:
                    fold_candidates.append({
                        "vertex_a": vid,
                        "vertex_b": hit["id"],
                        "similarity": hit["similarity_score"],
                    })

        # Find isolated vertices (no edges = potential gaps)
        isolated: list[str] = []
        for vid in active_vids:
            if not self.graph.neighbors(vid):
                isolated.append(vid)

        # Edge type distribution
        type_counts: dict[str, int] = {}
        for e in active_edges:
            type_counts[e.edge_type.value] = type_counts.get(e.edge_type.value, 0) + 1

        # Module breakdown
        modules: dict[str, int] = {}
        for vid in active_vids:
            module = vid.split(".")[0] if "." in vid else vid
            modules[module] = modules.get(module, 0) + 1

        return {
            "graph_stats": {
                "active_vertices": len(active_vids),
                "active_edges": len(active_edges),
                "beta_1": beta_1,
            },
            "modules": modules,
            "edge_type_distribution": type_counts,
            "fold_candidates": fold_candidates[:10],
            "isolated_vertices": isolated[:20],
            "total_isolated": len(isolated),
        }

    def execute_command(self, cmd: str, timeout: int = 30) -> dict:
        """Run shell command, return result."""
        try:
            result = subprocess.run(
                cmd,
                shell=True,
                capture_output=True,
                text=True,
                timeout=timeout,
                cwd=self.workspace,
            )
            return {
                "returncode": result.returncode,
                "stdout": result.stdout[:2000],
                "stderr": result.stderr[:2000],
            }
        except subprocess.TimeoutExpired:
            return {
                "returncode": -1,
                "stdout": "",
                "stderr": f"Command timed out after {timeout}s",
            }

    def run_tests(self, test_file: str | None = None) -> dict:
        """Run pytest in workspace and return structured results."""
        return run_tests(test_dir=self.workspace, test_file=test_file)


# ---------------------------------------------------------------------------
# CLI
# ---------------------------------------------------------------------------

if __name__ == "__main__":
    import sys

    workspace = sys.argv[1] if len(sys.argv) > 1 else "."
    agent = CodingAgent(workspace)

    print(f"Coding Agent initialized on {workspace}")
    print(
        f"Code complex: {len(agent.graph.active_vertex_ids())} vertices, "
        f"{len(agent.graph.active_edges())} edges"
    )

    while True:
        try:
            task_desc = input("\n> ")
        except (EOFError, KeyboardInterrupt):
            break
        if task_desc.lower() in ("quit", "exit"):
            break
        if task_desc.lower() == "analyze":
            result = agent.analyze()
            print(json.dumps(result, indent=2))
        elif task_desc.lower().startswith("!"):
            result = agent.execute_command(task_desc[1:].strip())
            print(json.dumps(result, indent=2))
        else:
            result = agent.task(task_desc)
            print(json.dumps(result, indent=2))
