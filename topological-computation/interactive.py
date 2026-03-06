"""Interactive traversal — question-driven directed traversal + answer generation.

User asks a question -> extract concepts -> locate in complex -> local traversal -> generate answer.

CLI usage:
    python interactive.py --complex experiment_phenomenology_full.json
    python interactive.py  # defaults to building Hegel Phenomenology from chapters

Pure Python, no external dependencies beyond this project's modules.
"""

from __future__ import annotations

import argparse
import json
import os
import random
import re
import sys
import time
from dataclasses import dataclass, field
from pathlib import Path

from engine import Graph, Vertex, Edge, EdgeType, VertexStatus, compute_beta_1
from morse import compute_terrain, critical_neighbors
from concept_registry import Registry, build_registry, _tokenize
from query_interface import _compute_f, _compute_f_decomposition, _shortest_path, _classify_zone
from traversal import TraversalEngine, EncounterType, Encounter, StepLog
from encounter_log import EncounterLog


# ---------------------------------------------------------------------------
# Stop words (common English words that don't map to concepts)
# ---------------------------------------------------------------------------

_STOP_WORDS: frozenset[str] = frozenset({
    "the", "is", "are", "was", "were", "be", "been", "being",
    "have", "has", "had", "do", "does", "did", "will", "would",
    "could", "should", "may", "might", "shall", "can",
    "and", "but", "or", "nor", "not", "no", "yes",
    "in", "on", "at", "to", "for", "of", "with", "by", "from",
    "up", "out", "off", "over", "under", "between", "through",
    "about", "into", "during", "before", "after", "above", "below",
    "it", "its", "this", "that", "these", "those",
    "he", "she", "they", "we", "you", "me", "him", "her", "us", "them",
    "my", "your", "his", "our", "their",
    "what", "which", "who", "whom", "whose", "when", "where", "why", "how",
    "if", "then", "than", "so", "as", "just", "also", "very",
    "an", "each", "every", "all", "both", "few", "more", "most",
    "some", "any", "many", "much", "such", "only",
    "does", "don", "doesn", "didn", "won", "wouldn", "couldn",
    "there", "here",
})


# ---------------------------------------------------------------------------
# Data structures
# ---------------------------------------------------------------------------

@dataclass(frozen=True, slots=True)
class ConceptMatch:
    """A concept found in the question, mapped to a vertex."""
    keyword: str
    vertex_id: str
    content: str


@dataclass
class TraversalRecord:
    """Record of a local traversal run."""
    start_vertices: list[str]
    steps: list[StepLog]
    encounters: list[dict]  # enriched encounter dicts
    total_steps: int
    beta_1_start: int
    beta_1_end: int


@dataclass
class Answer:
    """Structured answer to a question."""
    question: str
    concepts_found: list[ConceptMatch]
    concepts_not_found: list[str]
    pair_info: dict | None  # f value, path, etc. for two-concept queries
    traversal: TraversalRecord | None
    narrative: str


# ---------------------------------------------------------------------------
# InteractiveTraversal
# ---------------------------------------------------------------------------

class InteractiveTraversal:
    """Question-driven interactive traversal system."""

    def __init__(
        self,
        graph: Graph,
        registry: Registry,
        terrain: dict[tuple[str, str], str] | None = None,
        concept_names: dict[str, str] | None = None,
        seed: int = 42,
    ):
        self.graph = graph
        self.registry = registry
        self.terrain = terrain or compute_terrain(graph)
        self.concept_names = concept_names or {}
        self.encounter_log = EncounterLog()
        self.conversation_history: list[Answer] = []
        self._seed = seed
        self._last_traversal_positions: list[str] = []

        # Build concept_names from graph if not provided
        if not self.concept_names:
            for vid, v in graph.vertices.items():
                if v.content:
                    self.concept_names[vid] = v.content

    def ask(self, question: str, max_steps: int = 100) -> Answer:
        """Process a question, return structured answer.

        1. Extract concept keywords from question
        2. Locate corresponding vertices in the complex
        3. Run local traversal from those vertices
        4. Generate answer from traversal results
        """
        # Extract concepts
        keywords = self._extract_concepts(question)

        # Locate vertices
        found: list[ConceptMatch] = []
        not_found: list[str] = []
        for kw in keywords:
            matches = self.registry.lookup(kw)
            if matches:
                vid, content = matches[0]
                # Avoid duplicates
                if not any(m.vertex_id == vid for m in found):
                    found.append(ConceptMatch(keyword=kw, vertex_id=vid, content=content))
            else:
                not_found.append(kw)

        # If continuing from previous question and no concepts found,
        # use last traversal positions
        if not found and self._last_traversal_positions:
            for vid in self._last_traversal_positions[:3]:
                content = self.concept_names.get(vid, "")
                found.append(ConceptMatch(keyword="(continued)", vertex_id=vid, content=content))

        # Pair info for two-concept queries
        pair_info = None
        if len(found) >= 2:
            pair_info = self._compute_pair_info(found[0].vertex_id, found[1].vertex_id)

        # Local traversal
        traversal = None
        if found:
            start_vids = [m.vertex_id for m in found]
            traversal = self._local_traversal(start_vids, max_steps)
            if traversal and traversal.steps:
                self._last_traversal_positions = [
                    traversal.steps[-1].position,
                ]

        # Generate answer
        narrative = self._generate_answer(question, found, not_found, pair_info, traversal)
        answer = Answer(
            question=question,
            concepts_found=found,
            concepts_not_found=not_found,
            pair_info=pair_info,
            traversal=traversal,
            narrative=narrative,
        )
        self.conversation_history.append(answer)
        return answer

    # -- concept extraction --------------------------------------------------

    def _extract_concepts(self, question: str) -> list[str]:
        """Extract concept keywords from a question.

        Strategy:
        1. First try hyphenated compounds (e.g. "sense-certainty") as whole tokens
        2. Then try bigrams from the raw words
        3. Then individual tokens (skipping words already consumed by compounds)
        """
        q_lower = question.lower()
        q_clean = q_lower.replace("?", "").replace("!", "").replace(".", "").replace(",", "")
        words = q_clean.split()

        keywords: list[str] = []
        seen: set[str] = set()  # vertex IDs already matched
        consumed_words: set[int] = set()  # word indices consumed by compound matches

        # Pass 1: find hyphenated compounds in the original question
        # e.g. "sense-certainty" stays as one token
        hyphenated = re.findall(r'\b[a-z]+-[a-z]+(?:-[a-z]+)*\b', q_lower)
        for compound in hyphenated:
            matches = self.registry.lookup(compound)
            if matches:
                vid, _ = matches[0]
                if vid not in seen:
                    keywords.append(compound)
                    seen.add(vid)
                    # Mark constituent words as consumed
                    for part in compound.split("-"):
                        for i, w in enumerate(words):
                            if w == compound or w.replace("-", "") == part:
                                consumed_words.add(i)

        # Pass 2: try bigrams (space-separated pairs), skip if both words are stop words
        for i in range(len(words) - 1):
            if i in consumed_words or (i + 1) in consumed_words:
                continue
            # Skip bigrams made entirely of stop words
            if words[i] in _STOP_WORDS and words[i + 1] in _STOP_WORDS:
                continue
            for sep in ["_", "-", " "]:
                bigram = words[i] + sep + words[i + 1]
                matches = self.registry.lookup(bigram)
                if matches:
                    vid, _ = matches[0]
                    if vid not in seen:
                        keywords.append(bigram)
                        seen.add(vid)
                        consumed_words.add(i)
                        consumed_words.add(i + 1)
                    break

        # Pass 3: individual tokens (skip tokens that are parts of already-matched compounds)
        consumed_tokens: set[str] = set()
        for kw in keywords:
            # Split compound keywords into constituent parts
            for part in re.split(r'[-_ ]', kw):
                if len(part) >= 2:
                    consumed_tokens.add(part.lower())

        tokens = _tokenize(question)
        for token in tokens:
            if token in _STOP_WORDS or token in consumed_tokens:
                continue
            matches = self.registry.lookup(token)
            if matches:
                vid, _ = matches[0]
                if vid not in seen:
                    keywords.append(token)
                    seen.add(vid)

        return keywords

    # -- vertex location -----------------------------------------------------

    def _locate_vertices(self, concepts: list[str]) -> list[tuple[str, str]]:
        """Locate concept keywords in the complex.

        Returns [(vertex_id, content), ...].
        """
        results: list[tuple[str, str]] = []
        seen_vids: set[str] = set()
        for kw in concepts:
            matches = self.registry.lookup(kw)
            if matches:
                vid, content = matches[0]
                if vid not in seen_vids:
                    results.append((vid, content))
                    seen_vids.add(vid)
        return results

    # -- pair info -----------------------------------------------------------

    def _compute_pair_info(self, vid_a: str, vid_b: str) -> dict:
        """Compute topological relationship between two vertices."""
        decomp = _compute_f_decomposition(self.graph, vid_a, vid_b)
        f_val = decomp["f"]

        nbs_a = set(self.graph.neighbors(vid_a))
        nbs_b = set(self.graph.neighbors(vid_b))
        shared = (nbs_a & nbs_b) - {vid_a, vid_b}
        are_neighbors = vid_b in nbs_a

        path = None
        if not are_neighbors:
            raw_path = _shortest_path(self.graph, vid_a, vid_b)
            if raw_path:
                path = [
                    {"vertex_id": v, "content": self.concept_names.get(v, "")}
                    for v in raw_path
                ]

        shared_info = [
            {"vertex_id": s, "content": self.concept_names.get(s, "")}
            for s in sorted(shared)
        ]

        return {
            "f_value": f_val,
            "c": decomp["c"],
            "n_loop": decomp["n_loop"],
            "m": decomp["m"],
            "zone": _classify_zone(f_val),
            "are_neighbors": are_neighbors,
            "shared_neighbors": shared_info,
            "path": path,
        }

    # -- local traversal -----------------------------------------------------

    def _local_traversal(
        self,
        start_vertices: list[str],
        max_steps: int,
    ) -> TraversalRecord:
        """Run a local traversal from the given start vertices.

        Unlike full TraversalEngine traversal:
        - Starts from question-determined vertices (not highest degree)
        - Stays within N-hop neighborhood of start vertices
        - Alternates between multiple start vertices if present
        - Focuses on paths between start vertices for relationship queries
        """
        if not start_vertices:
            return TraversalRecord(
                start_vertices=[], steps=[], encounters=[],
                total_steps=0, beta_1_start=0, beta_1_end=0,
            )

        # Determine the local neighborhood (3-hop from all start vertices)
        neighborhood: set[str] = set()
        active_set = set(self.graph.active_vertex_ids())
        for sv in start_vertices:
            if sv not in active_set:
                continue
            # BFS to 3 hops
            layer = {sv}
            for _ in range(3):
                next_layer: set[str] = set()
                for v in layer:
                    for nb in self.graph.neighbors(v):
                        if nb in active_set:
                            next_layer.add(nb)
                layer = next_layer - neighborhood
                neighborhood.update(layer)
            neighborhood.add(sv)

        if not neighborhood:
            return TraversalRecord(
                start_vertices=start_vertices, steps=[], encounters=[],
                total_steps=0, beta_1_start=0, beta_1_end=0,
            )

        # Use the first start vertex as initial position
        initial = start_vertices[0] if start_vertices[0] in active_set else next(iter(neighborhood))

        engine = TraversalEngine(
            self.graph,
            start=initial,
            settlement_threshold=5,
            seed=self._seed,
        )

        beta_start = compute_beta_1(self.graph)
        encounters: list[dict] = []

        # Track which start vertices we've visited from
        start_idx = 0

        for step_num in range(max_steps):
            # Every 20 steps, if we have multiple start vertices, jump to the next one
            if len(start_vertices) > 1 and step_num > 0 and step_num % 20 == 0:
                start_idx = (start_idx + 1) % len(start_vertices)
                next_start = start_vertices[start_idx]
                if next_start in active_set:
                    engine.position = next_start
                    engine.visit_history.append(next_start)

            log = engine.run_step()

            # Record encounters (not walk steps)
            if log.operation != "walk":
                enc_record = {
                    "step": log.step,
                    "position": log.position,
                    "position_name": self.concept_names.get(log.position, log.position),
                    "operation": log.operation,
                    "encounter": log.encounter,
                    "f_value": log.f_value,
                    "beta_1_before": log.beta_1_before,
                    "beta_1_after": log.beta_1_after,
                    "blocked": log.blocked,
                }
                encounters.append(enc_record)

            # Constrain to neighborhood: if walker leaves, redirect back
            if engine.position not in neighborhood:
                # Pick a random vertex in the neighborhood
                engine.position = random.Random(self._seed + step_num).choice(
                    sorted(neighborhood)
                )
                engine.visit_history.append(engine.position)

        beta_end = compute_beta_1(engine.k_active)

        return TraversalRecord(
            start_vertices=start_vertices,
            steps=engine.logs,
            encounters=encounters,
            total_steps=max_steps,
            beta_1_start=beta_start,
            beta_1_end=beta_end,
        )

    # -- answer generation ---------------------------------------------------

    def _generate_answer(
        self,
        question: str,
        found: list[ConceptMatch],
        not_found: list[str],
        pair_info: dict | None,
        traversal: TraversalRecord | None,
    ) -> str:
        """Generate a structured answer from traversal results."""
        lines: list[str] = []

        # 1. Concept location
        lines.append("CONCEPT LOCATION:")
        if found:
            for m in found:
                # Get local topology
                nbs = self.graph.neighbors(m.vertex_id)
                avg_f = 0.0
                if nbs:
                    f_vals = [_compute_f(self.graph, m.vertex_id, nb) for nb in nbs]
                    avg_f = sum(f_vals) / len(f_vals)
                zone = _classify_zone(avg_f)
                v = self.graph.vertex(m.vertex_id)
                status = v.status.value if v else "unknown"
                lines.append(
                    f"  '{m.keyword}' -> '{m.content}' "
                    f"({len(nbs)} neighbors, avg_f={avg_f:.1f}, zone={zone}, status={status})"
                )
        if not_found:
            for kw in not_found:
                lines.append(f"  '{kw}' -> NOT FOUND in complex")
        lines.append("")

        # 2. Pair relationship (if two concepts)
        if pair_info and len(found) >= 2:
            lines.append("RELATIONSHIP:")
            a_name = found[0].content
            b_name = found[1].content
            f_val = pair_info["f_value"]
            zone = pair_info["zone"]
            lines.append(f"  f({a_name}, {b_name}) = {f_val} ({zone})")
            lines.append(f"  Components: c={pair_info['c']}, n_loop={pair_info['n_loop']}, m={pair_info['m']}")
            lines.append(f"  Direct neighbors: {pair_info['are_neighbors']}")

            if pair_info["shared_neighbors"]:
                shared_names = [s["content"] or s["vertex_id"][:16] for s in pair_info["shared_neighbors"]]
                lines.append(f"  Shared neighbors ({len(shared_names)}): {', '.join(shared_names[:8])}")
                if len(shared_names) > 8:
                    lines.append(f"    (+{len(shared_names) - 8} more)")

            if pair_info["path"]:
                path_names = [p["content"] or p["vertex_id"][:16] for p in pair_info["path"]]
                lines.append(f"  Shortest path ({len(path_names)} steps): {' -> '.join(path_names)}")
            lines.append("")

        # 3. Traversal discoveries
        if traversal and traversal.encounters:
            lines.append(f"TRAVERSAL ({traversal.total_steps} steps, {len(traversal.encounters)} encounters):")
            for enc in traversal.encounters:
                step = enc["step"]
                op = enc["operation"]
                pos_name = enc["position_name"]
                f_val = enc["f_value"]
                blocked = enc["blocked"]
                b_before = enc["beta_1_before"]
                b_after = enc["beta_1_after"]

                if blocked:
                    lines.append(
                        f"  Step {step}: {op} at '{pos_name}' (f={f_val}) "
                        f"-- BLOCKED (would destroy settled cycle)"
                    )
                else:
                    delta = b_after - b_before
                    delta_str = f"+{delta}" if delta >= 0 else str(delta)
                    lines.append(
                        f"  Step {step}: {op} at '{pos_name}' (f={f_val}) "
                        f"beta_1 {b_before}->{b_after} ({delta_str})"
                    )
            lines.append(
                f"  Summary: beta_1 {traversal.beta_1_start} -> {traversal.beta_1_end}"
            )
            lines.append("")
        elif traversal:
            lines.append(f"TRAVERSAL ({traversal.total_steps} steps, no encounters)")
            lines.append("")

        # 4. Synthesized answer
        lines.append("ANSWER:")
        answer_text = self._synthesize_answer(question, found, not_found, pair_info, traversal)
        lines.append(answer_text)

        return "\n".join(lines)

    def _synthesize_answer(
        self,
        question: str,
        found: list[ConceptMatch],
        not_found: list[str],
        pair_info: dict | None,
        traversal: TraversalRecord | None,
    ) -> str:
        """Produce the final answer text from traversal data."""
        if not found:
            return "  No matching concepts found in the complex. The system has not yet absorbed these concepts."

        parts: list[str] = []

        # Single concept query
        if len(found) == 1:
            m = found[0]
            nbs = self.graph.neighbors(m.vertex_id)
            nb_names = [self.concept_names.get(nb, nb) for nb in nbs[:10]]
            v = self.graph.vertex(m.vertex_id)

            parts.append(f"  '{m.content}' is an active concept in the complex with {len(nbs)} connections.")
            if v and v.status == VertexStatus.CONTESTED:
                parts.append(f"  It is currently CONTESTED (subject to active contradiction).")
            if nb_names:
                parts.append(f"  Connected to: {', '.join(nb_names)}.")

            # Check for traversal findings
            if traversal and traversal.encounters:
                neg_count = sum(1 for e in traversal.encounters if "negate" in e["operation"])
                fold_count = sum(1 for e in traversal.encounters if "fold" in e["operation"])
                block_count = sum(1 for e in traversal.encounters if e["blocked"])
                if neg_count > 0:
                    parts.append(f"  Traversal found {neg_count} negation(s) in its neighborhood.")
                if fold_count > 0:
                    parts.append(f"  Traversal found {fold_count} fold(s) (structural equivalences).")
                if block_count > 0:
                    parts.append(f"  {block_count} operation(s) were blocked by settled cycles (load-bearing structures).")

        # Two-concept relationship query
        elif len(found) >= 2 and pair_info:
            a = found[0]
            b = found[1]
            f_val = pair_info["f_value"]
            zone = pair_info["zone"]
            shared = pair_info["shared_neighbors"]

            if zone == "fold_zone":
                parts.append(
                    f"  '{a.content}' and '{b.content}' are topologically near-equivalent (f={f_val}, fold zone)."
                )
                parts.append(f"  They could potentially be folded (identified as the same concept).")
            elif zone == "gray_zone":
                parts.append(
                    f"  '{a.content}' and '{b.content}' have moderate topological distance (f={f_val}, gray zone)."
                )
                parts.append(f"  They are related but not equivalent.")
            elif zone == "negate_zone":
                parts.append(
                    f"  '{a.content}' and '{b.content}' are in topological tension (f={f_val}, negate zone)."
                )
                parts.append(f"  They are contradictory or dialectically opposed.")
            else:
                parts.append(
                    f"  '{a.content}' and '{b.content}' have f={f_val} ({zone})."
                )

            if shared:
                shared_names = [s["content"] or s["vertex_id"][:16] for s in shared[:5]]
                parts.append(
                    f"  Connected through {len(shared)} shared neighbor(s): {', '.join(shared_names)}."
                )

            if pair_info["path"] and not pair_info["are_neighbors"]:
                path_names = [p["content"] or p["vertex_id"][:16] for p in pair_info["path"]]
                parts.append(f"  Path: {' -> '.join(path_names)}.")

            # Traversal findings
            if traversal and traversal.encounters:
                blocked_folds = [
                    e for e in traversal.encounters
                    if "fold" in e["operation"] and e["blocked"]
                ]
                if blocked_folds:
                    parts.append(
                        f"  Traversal attempted to fold them but was blocked by settled cycle(s) "
                        f"-- their connection is load-bearing."
                    )
                negations = [e for e in traversal.encounters if "negate" in e["operation"]]
                if negations:
                    parts.append(
                        f"  {len(negations)} negation(s) detected in their shared neighborhood."
                    )

        return "\n".join(parts)


# ---------------------------------------------------------------------------
# Complex building helpers
# ---------------------------------------------------------------------------

def build_hegel_phenomenology() -> tuple[Graph, dict[str, str]]:
    """Build the Hegel Phenomenology complex from chapter functions."""
    from experiment_phenomenology_full import ALL_CHAPTERS

    graph = Graph()
    for ch_fn in ALL_CHAPTERS:
        _, ch_vertices, ch_edges = ch_fn()
        existing_vids = set(graph.vertices.keys())
        for v in ch_vertices:
            if v.id not in existing_vids:
                graph = graph.add_vertex(v)
        existing_edges = {(e.source, e.target, e.edge_type) for e in graph.edges}
        for e in ch_edges:
            key = (e.source, e.target, e.edge_type)
            if key not in existing_edges:
                if e.source in graph.vertices and e.target in graph.vertices:
                    graph = graph.add_edge(e)
                    existing_edges.add(key)

    concept_names = {}
    for vid, v in graph.vertices.items():
        if v.content:
            concept_names[vid] = v.content

    return graph, concept_names


def build_registry_for_graph(graph: Graph) -> Registry:
    """Build a registry directly from graph vertex content (no blocks needed)."""
    from concept_registry import _tokenize

    summaries: dict[str, str] = {}
    index: dict[str, list[str]] = {}

    for vid, vertex in graph.vertices.items():
        if vertex.status == VertexStatus.FOLDED:
            continue
        summary = vertex.content or vid
        summaries[vid] = summary

        tokens = _tokenize(summary)
        for token in set(tokens):
            index.setdefault(token, []).append(vid)

    return Registry(_summaries=summaries, _index=index)


# ---------------------------------------------------------------------------
# CLI
# ---------------------------------------------------------------------------

def run_interactive(graph: Graph, concept_names: dict[str, str]) -> None:
    """Run the interactive CLI loop."""
    registry = build_registry_for_graph(graph)
    terrain = compute_terrain(graph)

    system = InteractiveTraversal(
        graph=graph,
        registry=registry,
        terrain=terrain,
        concept_names=concept_names,
    )

    n_verts = len(graph.active_vertex_ids())
    n_edges = len(graph.active_edges())
    beta_1 = compute_beta_1(graph)

    print(f"Interactive Traversal Engine")
    print(f"Complex: {n_verts} vertices, {n_edges} edges, beta_1={beta_1}")
    print(f"Type a question, or 'quit' to exit.\n")

    while True:
        try:
            question = input("> ").strip()
        except (EOFError, KeyboardInterrupt):
            print("\nExiting.")
            break

        if not question:
            continue
        if question.lower() in ("quit", "exit", "q"):
            break

        answer = system.ask(question)
        print()
        print(answer.narrative)
        print()


def run_test(output_path: str | None = None) -> str:
    """Run three test questions and return the full interaction transcript."""
    graph, concept_names = build_hegel_phenomenology()
    registry = build_registry_for_graph(graph)
    terrain = compute_terrain(graph)

    system = InteractiveTraversal(
        graph=graph,
        registry=registry,
        terrain=terrain,
        concept_names=concept_names,
    )

    test_questions = [
        "What is sense-certainty?",
        "What is the relationship between sense-certainty and perception?",
        "Why does sense-certainty fail?",
    ]

    transcript_lines: list[str] = []
    transcript_lines.append("=" * 70)
    transcript_lines.append("INTERACTIVE TRAVERSAL TEST")
    transcript_lines.append("=" * 70)

    n_verts = len(graph.active_vertex_ids())
    n_edges = len(graph.active_edges())
    beta_1 = compute_beta_1(graph)
    transcript_lines.append(f"Complex: {n_verts} vertices, {n_edges} edges, beta_1={beta_1}")
    transcript_lines.append("")

    for i, question in enumerate(test_questions, 1):
        transcript_lines.append("-" * 70)
        transcript_lines.append(f"QUESTION {i}: {question}")
        transcript_lines.append("-" * 70)
        transcript_lines.append("")

        answer = system.ask(question)
        transcript_lines.append(answer.narrative)
        transcript_lines.append("")

    transcript = "\n".join(transcript_lines)

    if output_path:
        Path(output_path).parent.mkdir(parents=True, exist_ok=True)
        with open(output_path, "w", encoding="utf-8") as f:
            f.write(transcript)
        print(f"Test transcript saved to {output_path}", file=sys.stderr)

    return transcript


def main() -> None:
    parser = argparse.ArgumentParser(description="Interactive traversal engine")
    parser.add_argument(
        "--test", action="store_true",
        help="Run test questions instead of interactive mode",
    )
    parser.add_argument(
        "--output", type=str, default=None,
        help="Output path for test transcript (default: tmp/interactive_test.txt)",
    )
    args = parser.parse_args()

    # Change to script directory for imports
    script_dir = os.path.dirname(os.path.abspath(__file__))
    os.chdir(script_dir)

    if args.test:
        output = args.output
        if output is None:
            output = os.path.join(os.path.dirname(script_dir), "tmp", "interactive_test.txt")
        transcript = run_test(output)
        print(transcript)
    else:
        print("Building Hegel Phenomenology complex...", file=sys.stderr)
        graph, concept_names = build_hegel_phenomenology()
        run_interactive(graph, concept_names)


if __name__ == "__main__":
    main()
