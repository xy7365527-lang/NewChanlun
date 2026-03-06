"""psi_L_importance: Event importance I(e_i) + context-aware L2 narrative.

Based on Gemini R5 Q-R5-2:
- I(e_i) derived from persistent homology lifetime (Death - Birth)
- Simplified: I = |delta_beta_1| * (step - last_beta_change_step) + blocked_penalty
- Context annotations: region collision count, beta_1 growth rate, settled density
"""

from __future__ import annotations

import statistics
import sys

from engine import Graph, compute_beta_1
from psi_L_narrative import generate_narrative, _ENCOUNTER_OPS, _name, _enrich_step


# ---------------------------------------------------------------------------
# Importance computation
# ---------------------------------------------------------------------------

_OP_BASE_IMPORTANCE: dict[str, float] = {
    "sublate": 3.0,
    "sublate_blocked": 2.0,
    "negate_a": 2.0,
    "negate_b": 1.5,
    "negate_blocked": 1.5,
    "fold": 1.0,
    "fold_blocked": 1.0,
}


def compute_importance(
    step_logs: list[dict],
    settlement_tracker=None,
) -> list[float]:
    """Compute importance score I for each step.

    I = base(op) + |delta_beta_1| * persistence_gap + blocked_penalty - repeat_decay

    base(op): intrinsic weight per operation type (sublate > negate > fold).
    persistence_gap: steps since last beta_1 change (proxy for homology lifetime).
    blocked_penalty: scaled by settled cycle count (first occurrence is significant).
    repeat_decay: repeated identical operations at similar positions decay rapidly.

    For walk steps, I = 0.
    """
    if not step_logs:
        return []

    importance: list[float] = []
    last_beta_change_step = 0
    # Track repeat counts per (operation, target_a, target_b) signature
    signature_counts: dict[tuple, int] = {}

    for s in step_logs:
        step_num = s["step"]
        op = s["operation"]
        delta = abs(s.get("delta_beta_1", 0))
        blocked = s.get("blocked", False)

        if op == "walk" or op not in _ENCOUNTER_OPS:
            importance.append(0.0)
            if delta > 0:
                last_beta_change_step = step_num
            continue

        base = _OP_BASE_IMPORTANCE.get(op, 1.0)
        persistence_gap = step_num - last_beta_change_step
        score = base + delta * persistence_gap

        if blocked:
            settled_count = s.get("settled_count", 0)
            score += max(settled_count, 1) * 2.0

        # Repeat decay: same operation signature seen before loses importance
        sig = (op, s.get("target_a"), s.get("target_b"))
        prev_count = signature_counts.get(sig, 0)
        if prev_count > 0:
            # First repeat keeps 50%, second keeps 25%, etc.
            score = score / (2 ** prev_count)
        signature_counts[sig] = prev_count + 1

        importance.append(float(score))

        if delta > 0:
            last_beta_change_step = step_num

    return importance


# ---------------------------------------------------------------------------
# Filtering
# ---------------------------------------------------------------------------

def filter_by_importance(
    step_logs: list[dict],
    importance: list[float],
    theta: float | str = "auto",
) -> list[dict]:
    """Keep only steps with I >= theta.

    theta='auto': use 75th percentile of non-zero importance values.
                  This typically retains the top ~25% most significant events.
    theta='mean': use mean of non-zero importance values.
    """
    if isinstance(theta, str):
        nonzero = sorted(v for v in importance if v > 0)
        if not nonzero:
            return [s for s in step_logs if s["operation"] in _ENCOUNTER_OPS]
        if theta == "mean":
            threshold = statistics.mean(nonzero)
        else:
            # 75th percentile: keep top quarter of events
            idx = int(len(nonzero) * 0.75)
            threshold = nonzero[min(idx, len(nonzero) - 1)]
    else:
        threshold = theta

    return [
        s for s, imp in zip(step_logs, importance)
        if imp >= threshold
    ]


# ---------------------------------------------------------------------------
# Context statistics
# ---------------------------------------------------------------------------

def _region_collision_count(
    step_logs: list[dict],
    current_idx: int,
    hop_radius: int = 2,
    graph: Graph | None = None,
) -> int:
    """Count blocked events near the current position within hop_radius."""
    current = step_logs[current_idx]
    pos = current["position"]

    if graph is not None:
        local_verts, _ = graph.local_subgraph(pos, radius=hop_radius)
        region = set(local_verts)
    else:
        region = {pos}

    count = 0
    for s in step_logs[:current_idx]:
        if s.get("blocked", False) and s["position"] in region:
            count += 1
    return count


def _beta_growth_rate(
    step_logs: list[dict],
    current_idx: int,
    window: int = 50,
) -> tuple[float, float]:
    """Compute beta_1 growth rate in sliding window vs global average.

    Returns (local_rate, global_rate).
    """
    if current_idx < 1:
        return 0.0, 0.0

    # Global rate
    first_beta = step_logs[0]["beta_1_before"]
    last_beta = step_logs[current_idx]["beta_1_after"]
    total_steps = step_logs[current_idx]["step"] - step_logs[0]["step"]
    global_rate = (last_beta - first_beta) / max(total_steps, 1)

    # Local rate (last `window` steps)
    window_start = max(0, current_idx - window)
    local_first = step_logs[window_start]["beta_1_before"]
    local_last = step_logs[current_idx]["beta_1_after"]
    local_steps = step_logs[current_idx]["step"] - step_logs[window_start]["step"]
    local_rate = (local_last - local_first) / max(local_steps, 1)

    return local_rate, global_rate


def _settled_density(
    step: dict,
    graph: Graph | None = None,
    hop_radius: int = 2,
) -> int:
    """Count settled cycles whose edges involve vertices in 2-hop neighborhood."""
    settled_count = step.get("settled_count", 0)
    return settled_count


# ---------------------------------------------------------------------------
# Context annotation
# ---------------------------------------------------------------------------

def _annotate_step(
    step_logs: list[dict],
    idx: int,
    graph: Graph | None = None,
) -> list[str]:
    """Generate context annotations for a step."""
    annotations: list[str] = []
    step = step_logs[idx]
    step_num = step["step"]

    # Region collision count
    collisions = _region_collision_count(step_logs, idx, graph=graph)
    if collisions > 0:
        annotations.append(
            f"[region: {collisions} prior blocked event(s) near this position]"
        )

    # Beta_1 growth rate
    local_rate, global_rate = _beta_growth_rate(step_logs, idx)
    if global_rate > 0 and local_rate > global_rate * 2.0:
        annotations.append(
            f"[beta_1 growth: local rate {local_rate:.4f}/step is "
            f"{local_rate/global_rate:.1f}x the global average]"
        )
    elif global_rate > 0 and local_rate < global_rate * 0.3:
        annotations.append(
            f"[beta_1 growth: local rate {local_rate:.4f}/step is "
            f"unusually slow vs global {global_rate:.4f}/step]"
        )

    # Settled density
    settled = _settled_density(step, graph=graph)
    if settled > 0:
        annotations.append(f"[settled cycles: {settled} active]")

    # Source tracking (cross-text annotation)
    target_a = step.get("target_a")
    target_b = step.get("target_b")
    if target_a and target_b:
        # Detect if concepts originate from different creation steps (proxy for
        # different text sources in multi-chapter experiments)
        if graph is not None:
            va = graph.vertex(target_a)
            vb = graph.vertex(target_b)
            if va and vb and va.created_at != vb.created_at:
                annotations.append(
                    f"[cross-origin: '{target_a}' (step {va.created_at}) "
                    f"vs '{target_b}' (step {vb.created_at})]"
                )

    return annotations


# ---------------------------------------------------------------------------
# Layer-2 narrative generation
# ---------------------------------------------------------------------------

def generate_narrative_l2(
    step_logs: list[dict],
    graph: Graph,
    theta: float | str = "auto",
    concept_names: dict[str, str] | None = None,
) -> str:
    """Layer-2 narrative: importance-filtered + context-annotated.

    Compared to layer-1 (psi_L_narrative.generate_narrative):
    - Filters by importance I(e_i) > theta
    - Adds context annotations per event
    - Reports filtering statistics
    """
    if not step_logs:
        return "=== L2 Traversal Narrative ===\nNo steps recorded.\n"

    # Build concept names from graph vertex content
    if concept_names is None:
        concept_names = {}
    names = dict(concept_names)
    for vid, v in graph.vertices.items():
        if vid not in names and v.content:
            names[vid] = v.content

    # Compute importance
    importance = compute_importance(step_logs)

    # Filter
    filtered = filter_by_importance(step_logs, importance, theta)

    # Build index of filtered steps for annotation lookup
    filtered_indices: dict[int, int] = {}
    for orig_idx, s in enumerate(step_logs):
        if s in filtered:
            filtered_indices[s["step"]] = orig_idx

    # Compute actual theta used
    nonzero_imp = sorted(v for v in importance if v > 0)
    if isinstance(theta, str) and nonzero_imp:
        if theta == "mean":
            actual_theta = statistics.mean(nonzero_imp)
        else:
            idx = int(len(nonzero_imp) * 0.75)
            actual_theta = nonzero_imp[min(idx, len(nonzero_imp) - 1)]
    elif isinstance(theta, (int, float)):
        actual_theta = theta
    else:
        actual_theta = 0.0

    # Total encounter count for comparison
    total_encounters = sum(1 for s in step_logs if s["operation"] in _ENCOUNTER_OPS)

    # Format each filtered step
    lines: list[str] = []

    # Section tracking
    section_events = 0
    section_start_beta = filtered[0]["beta_1_before"] if filtered else 0
    last_section = -1

    for s in filtered:
        step_num = s["step"]
        op = s["operation"]
        orig_idx = filtered_indices.get(step_num, 0)

        # Section boundary (every 200 steps for L2 — coarser than L1's 100)
        section_idx = (step_num - 1) // 200
        if section_idx != last_section and last_section >= 0:
            s_start = last_section * 200 + 1
            s_end = step_num - 1
            lines.append(
                f"--- Section {s_start}-{s_end}: {section_events} significant "
                f"events, beta_1 {section_start_beta}"
                f"-> {s.get('beta_1_before', '?')} ---"
            )
            lines.append("")
            section_events = 0
            section_start_beta = s["beta_1_before"]
        last_section = section_idx

        # Format the event
        enriched = _enrich_step(s)
        imp_val = importance[orig_idx] if orig_idx < len(importance) else 0.0

        line = _format_l2_step(enriched, names, imp_val)
        lines.append(line)

        # Context annotations
        annotations = _annotate_step(step_logs, orig_idx, graph=graph)
        for ann in annotations:
            lines.append(f"    {ann}")

        section_events += 1

    # Final section summary
    if filtered:
        last = filtered[-1]
        s_start = last_section * 200 + 1 if last_section >= 0 else 1
        lines.append("")
        lines.append(
            f"--- Section {s_start}-{last['step']}: {section_events} significant "
            f"events, beta_1 {section_start_beta}"
            f"-> {last['beta_1_after']} ---"
        )

    # Header with statistics
    last_step = step_logs[-1] if step_logs else {"step": 0, "beta_1_after": 0}
    header = (
        f"=== L2 Traversal Narrative (Importance-Filtered) ===\n"
        f"Total steps: {len(step_logs)} | "
        f"Total encounters: {total_encounters} | "
        f"Significant (I > {actual_theta:.1f}): {len(filtered)} | "
        f"Compression: {1 - len(filtered)/max(total_encounters, 1):.0%}\n"
        f"Final beta_1: {last_step['beta_1_after']}\n"
    )

    return header + "\n" + "\n".join(lines)


def _format_l2_step(step: dict, names: dict[str, str] | None, importance: float) -> str:
    """Format a single L2 step with importance score."""
    op = step["operation"]
    step_num = step["step"]
    a = _name(step.get("target_a", step["position"]), names)
    b_raw = step.get("target_b", "")
    b = _name(b_raw, names) if b_raw else ""
    f_val = step.get("f_value", -99)
    before = step["beta_1_before"]
    after = step["beta_1_after"]
    delta = step.get("delta_beta_1", 0)
    blocked = step.get("blocked", False)

    imp_tag = f"I={importance:.0f}" if importance > 0 else "I=0"

    if op == "fold":
        return (
            f"Step {step_num} [{imp_tag}]: FOLD '{a}' ~ '{b}' "
            f"(f={f_val}). beta_1 {before}->{after}."
        )
    elif op == "fold_blocked":
        return (
            f"Step {step_num} [{imp_tag}]: FOLD BLOCKED '{a}' ~ '{b}' "
            f"(f={f_val}) -- would destroy settled cycle."
        )
    elif op in ("negate_a", "negate_b"):
        sign = f"+{delta}" if delta >= 0 else str(delta)
        return (
            f"Step {step_num} [{imp_tag}]: NEGATE '{a}' vs '{b}' "
            f"(f={f_val}). beta_1 {before}->{after} ({sign})."
        )
    elif op == "negate_blocked":
        return (
            f"Step {step_num} [{imp_tag}]: NEGATE BLOCKED '{a}' vs '{b}' "
            f"(f={f_val}) -- would destroy settled cycle."
        )
    elif op == "sublate":
        syn_id = f"syn_{step.get('target_a', '')}_{step.get('target_b', '')}_{step_num}"
        syn_name = _name(syn_id, names)
        return (
            f"Step {step_num} [{imp_tag}]: SUBLATE '{a}' + '{b}' -> '{syn_name}'. "
            f"beta_1 {before}->{after}."
        )
    elif op == "sublate_blocked":
        return (
            f"Step {step_num} [{imp_tag}]: SUBLATE BLOCKED '{a}' + '{b}' "
            f"-- would destroy settled cycle."
        )
    else:
        return f"Step {step_num} [{imp_tag}]: {op} at '{a}'. beta_1 {before}->{after}."


# ---------------------------------------------------------------------------
# CLI: run experiment, generate L1 vs L2 comparison
# ---------------------------------------------------------------------------

def main():
    """Re-run phenomenology experiment, produce L2 narrative, compare with L1."""
    import os

    print("Re-running phenomenology experiment to capture step logs...",
          file=sys.stderr)

    script_dir = os.path.dirname(os.path.abspath(__file__))
    os.chdir(script_dir)

    from psi_L_narrative import _run_and_capture, generate_narrative

    step_logs, graph, concept_names = _run_and_capture()

    print(f"Captured {len(step_logs)} steps. Generating narratives...",
          file=sys.stderr)

    # L1 narrative
    l1 = generate_narrative(step_logs, graph, concept_names)

    # L2 narrative
    l2 = generate_narrative_l2(step_logs, graph, theta="auto",
                               concept_names=concept_names)

    # Write outputs
    l1_lines = l1.strip().split("\n")
    l2_lines = l2.strip().split("\n")

    output_dir = os.path.join(os.path.dirname(script_dir), "tmp")
    os.makedirs(output_dir, exist_ok=True)

    l2_path = os.path.join(output_dir, "hegel_narrative_l2.txt")
    with open(l2_path, "w", encoding="utf-8") as f:
        f.write(l2)

    print(f"\nL2 narrative written to {l2_path}", file=sys.stderr)

    # Comparison summary
    comparison = (
        f"=== L1 vs L2 Narrative Comparison ===\n"
        f"L1 lines: {len(l1_lines)}\n"
        f"L2 lines: {len(l2_lines)}\n"
        f"Compression ratio: {1 - len(l2_lines)/max(len(l1_lines), 1):.0%}\n"
    )
    print(comparison)
    print(l2)


if __name__ == "__main__":
    main()
