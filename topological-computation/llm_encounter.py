"""LLM-based encounter detection for Phase 2.

Uses Anthropic API to let an LLM decide encounters based on vertex content
semantics, rather than topological rules.
"""

from __future__ import annotations

import os
import re
from dataclasses import dataclass
from typing import Optional

import anthropic

from engine import Graph, EdgeType, VertexStatus


@dataclass(frozen=True, slots=True)
class LLMEncounterResult:
    """Parsed result from LLM encounter detection."""
    action: str          # FOLD, NEGATE, SUBLATE, WALK
    target_a: Optional[str] = None
    target_b: Optional[str] = None
    reasoning: str = ""


def format_prompt(
    position: str,
    graph: Graph,
    terrain: dict[tuple[str, str], str],
    beta_1: int,
    settled_count: int,
    pending_negations: list[tuple[str, str]],
    conservative: bool = False,
    visit_history: list[str] | None = None,
) -> str:
    """Format local context into LLM prompt.

    The LLM sees vertex content and must decide based on semantic relationships,
    not graph structure. Includes recent traversal history for fold detection.
    """
    v = graph.vertex(position)
    pos_content = v.content if v and v.content else "(no content)"
    pos_status = v.status.value if v else "unknown"

    # Gather neighbors with content
    neighbors = graph.neighbors(position)
    neighbor_lines = []
    for nb_id in neighbors:
        nb = graph.vertex(nb_id)
        nb_content = nb.content if nb and nb.content else "(no content)"
        nb_status = nb.status.value if nb else "unknown"

        # Find edge types between position and neighbor
        edge_types = []
        for e in graph.active_edges():
            if (e.source == position and e.target == nb_id) or \
               (e.source == nb_id and e.target == position):
                direction = "outgoing" if e.source == position else "incoming"
                mark = terrain.get((e.source, e.target), "unknown")
                edge_types.append(f"{e.edge_type.value}({direction},{mark})")

        neighbor_lines.append(
            f"  - {nb_id} [{nb_status}]: \"{nb_content}\" | edges: {', '.join(edge_types)}"
        )

    neighbor_block = "\n".join(neighbor_lines) if neighbor_lines else "  (none)"

    # Recent traversal history (for fold detection — Nachträglichkeit)
    history_block = "  (none)"
    if visit_history:
        seen: set[str] = set()
        history_lines = []
        # Last 15 unique visited vertices (excluding current position and neighbors)
        neighbor_set = set(neighbors) | {position}
        for vid in reversed(visit_history):
            if vid in seen or vid in neighbor_set:
                continue
            seen.add(vid)
            hv = graph.vertex(vid)
            if hv and hv.status.value != "folded":
                hv_content = hv.content if hv.content else "(no content)"
                history_lines.append(f"  - {vid}: \"{hv_content}\"")
            if len(history_lines) >= 15:
                break
        if history_lines:
            history_block = "\n".join(history_lines)

    # Pending negations
    pending_lines = []
    for thesis, antithesis in pending_negations:
        t_v = graph.vertex(thesis)
        a_v = graph.vertex(antithesis)
        t_content = t_v.content if t_v and t_v.content else "(no content)"
        a_content = a_v.content if a_v and a_v.content else "(no content)"
        pending_lines.append(
            f"  - {thesis} (\"{t_content}\") vs {antithesis} (\"{a_content}\")"
        )
    pending_block = "\n".join(pending_lines) if pending_lines else "  (none)"

    conservative_block = ""
    if conservative:
        conservative_block = (
            "IMPORTANT: Be conservative. If there is no clear structural identity "
            "or contradiction at this position, report WALK. Do not force connections. "
            "Most steps should be WALK — only report FOLD/NEGATE/SUBLATE when the "
            "evidence is strong. Two propositions that are merely related or complementary "
            "are NOT contradictions. Two propositions that discuss similar topics but make "
            "different claims are NOT equivalent (not FOLD candidates).\n\n"
        )

    return f"""You are a philosophical observer walking through a graph of propositions.

YOUR CURRENT POSITION:
  Vertex: {position} [{pos_status}]
  Content: "{pos_content}"

NEIGHBORS:
{neighbor_block}

RECENTLY VISITED (traversal history — check for structural identity with current position):
{history_block}

GLOBAL STATE:
  Betti number (β₁, cycle count): {beta_1}
  Settled cycles: {settled_count}

PENDING CONTRADICTIONS (awaiting sublation):
{pending_block}

AVAILABLE ACTIONS:
  FOLD(a, b) — Identify two vertices as saying essentially the same thing. Use when two propositions are semantically equivalent or one subsumes the other. 'a' must be your current position. 'b' can be a neighbor OR a recently visited vertex from the traversal history above.
  NEGATE(a, b) — Declare a contradiction between two propositions. Use when two vertices make claims that cannot both be true. a must be your current position, b must be a neighbor.
  SUBLATE(a, b) — Synthesize two contradicting propositions into a higher-level insight. Only valid when a and b are in the pending contradictions list above.
  WALK — Move on without taking action. Use when no semantic relationship warrants an operation.

INSTRUCTIONS:
Examine the CONTENT of the propositions. Based on their MEANING:
1. Does your current position say essentially the same thing as any neighbor OR any recently visited vertex? → FOLD
2. Does your position contradict a neighbor? → NEGATE
3. Can you synthesize any pending contradiction? → SUBLATE
4. Nothing notable? → WALK

{conservative_block}Respond with EXACTLY ONE line in one of these formats:
  FOLD(vertexA, vertexB)
  NEGATE(vertexA, vertexB)
  SUBLATE(vertexA, vertexB)
  WALK

Follow that with a brief reasoning line starting with "Reason:".
"""


def parse_response(text: str) -> LLMEncounterResult:
    """Parse LLM response into an encounter result."""
    lines = text.strip().split("\n")
    action_line = ""
    reasoning = ""

    for line in lines:
        stripped = line.strip()
        if stripped.startswith("Reason:"):
            reasoning = stripped[len("Reason:"):].strip()
        elif not action_line and stripped:
            action_line = stripped

    # Parse action
    fold_match = re.match(r"FOLD\(\s*(\w+)\s*,\s*(\w+)\s*\)", action_line)
    if fold_match:
        return LLMEncounterResult("FOLD", fold_match.group(1), fold_match.group(2), reasoning)

    negate_match = re.match(r"NEGATE\(\s*(\w+)\s*,\s*(\w+)\s*\)", action_line)
    if negate_match:
        return LLMEncounterResult("NEGATE", negate_match.group(1), negate_match.group(2), reasoning)

    sublate_match = re.match(r"SUBLATE\(\s*(\w+)\s*,\s*(\w+)\s*\)", action_line)
    if sublate_match:
        return LLMEncounterResult("SUBLATE", sublate_match.group(1), sublate_match.group(2), reasoning)

    if action_line.strip().startswith("WALK"):
        return LLMEncounterResult("WALK", reasoning=reasoning)

    # Fallback: couldn't parse → WALK
    return LLMEncounterResult("WALK", reasoning=f"(parse fallback) {action_line}")


def call_llm(prompt: str, model: str = "claude-sonnet-4-20250514") -> str:
    """Call Anthropic API synchronously.

    Uses ANTHROPIC_API_KEY from environment.
    """
    client = anthropic.Anthropic()  # reads ANTHROPIC_API_KEY from env
    message = client.messages.create(
        model=model,
        max_tokens=256,
        messages=[{"role": "user", "content": prompt}],
    )
    return message.content[0].text


def detect_encounter_llm(
    position: str,
    graph: Graph,
    terrain: dict[tuple[str, str], str],
    beta_1: int,
    settled_count: int,
    pending_negations: list[tuple[str, str]],
    model: str = "claude-sonnet-4-20250514",
    conservative: bool = False,
    visit_history: list[str] | None = None,
) -> LLMEncounterResult:
    """Full pipeline: format prompt → call LLM → parse response."""
    prompt = format_prompt(
        position, graph, terrain, beta_1, settled_count, pending_negations,
        conservative=conservative,
        visit_history=visit_history,
    )
    response_text = call_llm(prompt, model=model)
    return parse_response(response_text)
