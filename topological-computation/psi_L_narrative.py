"""psi_L_narrative: Traversal log -> filtered narrative text.

Reads step logs from a TraversalEngine run, filters out walk steps,
and produces a readable narrative of encounter events using templates.
"""

from __future__ import annotations

import sys

from engine import Graph


def _name(vertex_id: str, concept_names: dict[str, str] | None) -> str:
    """Resolve vertex id to readable name."""
    if concept_names and vertex_id in concept_names:
        return concept_names[vertex_id]
    return vertex_id


def _format_fold(step: dict, names: dict[str, str] | None) -> str:
    """Template for successful fold."""
    a = _name(step.get("target_a", step["position"]), names)
    b = _name(step.get("target_b", ""), names)
    f_val = step.get("f_value", -99)
    before = step["beta_1_before"]
    after = step["beta_1_after"]
    return (
        f"Step {step['step']}: '{a}' and '{b}' identified as structurally "
        f"equivalent (f={f_val}). Folded. \u03b2\u2081 {before}\u2192{after}."
    )


def _format_fold_blocked(step: dict, names: dict[str, str] | None) -> str:
    """Template for blocked fold."""
    a = _name(step.get("target_a", step["position"]), names)
    b = _name(step.get("target_b", ""), names)
    f_val = step.get("f_value", -99)
    return (
        f"Step {step['step']}: Attempted fold of '{a}' and '{b}' "
        f"(f={f_val}), but blocked \u2014 would destroy a load-bearing cycle."
    )


def _format_negate(step: dict, names: dict[str, str] | None) -> str:
    """Template for negation (negate_a or negate_b)."""
    a = _name(step.get("target_a", step["position"]), names)
    b_raw = step.get("target_b", "")
    b = _name(b_raw, names) if b_raw else "(new antithesis)"
    f_val = step.get("f_value", -99)
    before = step["beta_1_before"]
    after = step["beta_1_after"]
    delta = step["delta_beta_1"]
    sign = f"+{delta}" if delta >= 0 else str(delta)
    return (
        f"Step {step['step']}: Contradiction detected between '{a}' and "
        f"'{b}' (f={f_val}). Negation inscribed. "
        f"\u03b2\u2081 {before}\u2192{after} ({sign})."
    )


def _format_sublate(step: dict, names: dict[str, str] | None) -> str:
    """Template for sublation."""
    a = _name(step.get("target_a", step["position"]), names)
    b = _name(step.get("target_b", ""), names)
    # synthesis vertex = syn_{a}_{b}_{step}
    syn_id = f"syn_{step.get('target_a', '')}_{step.get('target_b', '')}_{step['step']}"
    syn_name = _name(syn_id, names)
    before = step["beta_1_before"]
    after = step["beta_1_after"]
    return (
        f"Step {step['step']}: Synthesis of '{a}' and '{b}' \u2192 new concept "
        f"'{syn_name}'. \u03b2\u2081 {before}\u2192{after}."
    )


_ENCOUNTER_OPS = {
    "fold": _format_fold,
    "fold_blocked": _format_fold_blocked,
    "negate_a": _format_negate,
    "negate_b": _format_negate,
    "negate_blocked": _format_negate,
    "sublate": _format_sublate,
    "sublate_blocked": _format_sublate,
}


def _event_signature(step: dict) -> tuple:
    """Return a signature for deduplication of consecutive identical events."""
    return (step.get("operation"), step.get("target_a"), step.get("target_b"))


def generate_narrative(
    step_logs: list[dict],
    graph: Graph,
    concept_names: dict[str, str] | None = None,
) -> str:
    """Generate narrative from traversal step logs.

    step_logs: list of StepLog dicts (from TraversalEngine.logs)
    graph: current Graph (used to resolve vertex content as names)
    concept_names: optional vertex_id -> readable name override
    """
    if not step_logs:
        return "=== Traversal Narrative ===\nNo steps recorded.\n"

    # Build concept_names from graph vertex content if not provided
    if concept_names is None:
        concept_names = {}
    # Merge graph vertex content as fallback names
    names = dict(concept_names)
    for vid, v in graph.vertices.items():
        if vid not in names and v.content:
            names[vid] = v.content

    # Pre-pass: group consecutive identical encounter events for compression
    encounter_steps = [s for s in step_logs if s["operation"] in _ENCOUNTER_OPS]
    runs: list[tuple[dict, int, int]] = []  # (first_step, count, last_step_num)
    for s in encounter_steps:
        sig = _event_signature(s)
        if runs and _event_signature(runs[-1][0]) == sig:
            runs[-1] = (runs[-1][0], runs[-1][1] + 1, s["step"])
        else:
            runs.append((s, 1, s["step"]))
    # Build a set of steps to skip (compressed into a summary line)
    compressed: dict[int, tuple[int, int]] = {}  # first_step_num -> (count, last_step_num)
    skip_steps: set[int] = set()
    for first, count, last_step_num in runs:
        if count > 2:
            compressed[first["step"]] = (count, last_step_num)
            # Mark all but the first step in this run for skipping
            for s in encounter_steps:
                if _event_signature(s) == _event_signature(first) and \
                   first["step"] < s["step"] <= last_step_num:
                    skip_steps.add(s["step"])

    lines: list[str] = []
    total = len(step_logs)

    # Track quiet stretches and periodic summaries
    quiet_start: int | None = None
    section_events = 0
    section_beta_start = step_logs[0]["beta_1_before"]

    for i, step in enumerate(step_logs):
        op = step["operation"]
        step_num = step["step"]

        # Check for section boundary (every 100 steps)
        section_idx = (step_num - 1) // 100
        prev_section_idx = (step_logs[i - 1]["step"] - 1) // 100 if i > 0 else -1

        if section_idx != prev_section_idx and i > 0:
            # Flush quiet streak before section summary
            if quiet_start is not None:
                quiet_end = step_logs[i - 1]["step"]
                if quiet_end - quiet_start >= 50:
                    lines.append(
                        f"--- Quiet traversal: steps {quiet_start}-{quiet_end}, "
                        f"no encounters ---"
                    )
                quiet_start = None

            # Section summary
            prev_step = step_logs[i - 1]
            s_start = prev_section_idx * 100 + 1
            s_end = prev_step["step"]
            beta_end = prev_step["beta_1_after"]
            settled_end = prev_step.get("settled_count", 0)
            lines.append(
                f"--- Steps {s_start}-{s_end}: {section_events} events, "
                f"\u03b2\u2081 {section_beta_start}\u2192{beta_end}, "
                f"{settled_end} cycles settled ---"
            )
            lines.append("")
            section_events = 0
            section_beta_start = step["beta_1_before"]

        # Is this an encounter event?
        if op in _ENCOUNTER_OPS:
            # Skip steps that are compressed (but break quiet streak)
            if step_num in skip_steps:
                quiet_start = None
                section_events += 1
                continue

            # Flush quiet streak
            if quiet_start is not None:
                quiet_end = step_logs[i - 1]["step"] if i > 0 else step_num
                span = quiet_end - quiet_start
                if span >= 50:
                    lines.append(
                        f"--- Quiet traversal: steps {quiet_start}-{quiet_end}, "
                        f"no encounters ---"
                    )
                quiet_start = None

            # Render encounter
            formatter = _ENCOUNTER_OPS[op]
            enriched = _enrich_step(step)
            line = formatter(enriched, names)

            # Append compression note if this is the start of a compressed run
            if step_num in compressed:
                count, last_sn = compressed[step_num]
                line += f" (x{count}, steps {step_num}-{last_sn})"

            lines.append(line)
            section_events += 1
        else:
            # Walk step — track quiet stretch
            if quiet_start is None:
                quiet_start = step_num

    # Flush final quiet streak
    if quiet_start is not None:
        quiet_end = step_logs[-1]["step"]
        if quiet_end - quiet_start >= 50:
            lines.append(
                f"--- Quiet traversal: steps {quiet_start}-{quiet_end}, "
                f"no encounters ---"
            )

    # Final section summary
    last = step_logs[-1]
    s_start = ((last["step"] - 1) // 100) * 100 + 1
    lines.append("")
    lines.append(
        f"--- Steps {s_start}-{last['step']}: {section_events} events, "
        f"\u03b2\u2081 {section_beta_start}\u2192{last['beta_1_after']}, "
        f"{last.get('settled_count', 0)} cycles settled ---"
    )

    # Header
    event_count = len(encounter_steps)
    header = (
        f"=== Traversal Narrative ===\n"
        f"Total steps: {total} | Encounter events: {event_count} | "
        f"Final \u03b2\u2081: {last['beta_1_after']}\n"
    )

    return header + "\n" + "\n".join(lines)


def _enrich_step(step: dict) -> dict:
    """Add target_a / target_b fields from encounter context if missing."""
    # StepLog has position but not target_a/target_b directly.
    # The narrative generator gets enriched logs from _run_and_capture().
    return step


def _run_and_capture() -> tuple[list[dict], Graph, dict[str, str]]:
    """Re-run the phenomenology experiment and capture step logs.

    Returns (step_log_dicts, final_graph, concept_names).
    """
    from experiment_phenomenology_full import ALL_CHAPTERS
    from traversal import TraversalEngine, Encounter
    from engine import compute_beta_1
    from morse import compute_terrain

    graph = Graph()
    engine = None
    cumulative_steps = 0

    # Capture encounter targets by wrapping detect_encounter
    encounter_targets: dict[int, tuple[str | None, str | None]] = {}

    for ch_idx, ch_fn in enumerate(ALL_CHAPTERS):
        chapter_name, ch_vertices, ch_edges = ch_fn()

        # Inject vertices
        existing_vids = set(graph.vertices.keys())
        for v in ch_vertices:
            if v.id not in existing_vids:
                graph = graph.add_vertex(v)

        # Inject edges
        existing_edges = {(e.source, e.target, e.edge_type) for e in graph.edges}
        for e in ch_edges:
            key = (e.source, e.target, e.edge_type)
            if key not in existing_edges:
                if e.source in graph.vertices and e.target in graph.vertices:
                    graph = graph.add_edge(e)
                    existing_edges.add(key)

        # Create or update engine
        if engine is None:
            start = graph.active_vertex_ids()[0]
            engine = TraversalEngine(
                graph, start=start, settlement_threshold=10, seed=42,
            )
            # Wrap detect_encounter to capture targets without extra RNG effects
            original_detect = engine.detect_encounter.__func__

            def wrapped_detect(self_eng, _orig=original_detect):
                enc = _orig(self_eng)
                encounter_targets[self_eng.step] = (enc.target_a, enc.target_b)
                return enc

            import types
            engine.detect_encounter = types.MethodType(wrapped_detect, engine)
        else:
            engine.k_active = graph
            engine.k_full = graph
            engine.terrain = compute_terrain(engine.k_active)
            if engine.position not in graph.vertices:
                engine.position = graph.active_vertex_ids()[0]

        # Digest: run until beta_1 stable for 50 consecutive steps
        stable_count = 0
        last_beta = compute_beta_1(engine.k_active)
        steps_this_chapter = 0
        max_steps = 2000

        while stable_count < 50 and steps_this_chapter < max_steps:
            engine.run_step()
            steps_this_chapter += 1
            cumulative_steps += 1
            current_beta = compute_beta_1(engine.k_active)
            if current_beta == last_beta:
                stable_count += 1
            else:
                stable_count = 0
                last_beta = current_beta

        graph = engine.k_active

    # Convert StepLog dataclasses to dicts with encounter targets
    step_dicts = []
    for log in engine.logs:
        d = {
            "step": log.step,
            "position": log.position,
            "encounter": log.encounter,
            "operation": log.operation,
            "beta_1_before": log.beta_1_before,
            "beta_1_after": log.beta_1_after,
            "delta_beta_1": log.delta_beta_1,
            "settled_count": log.settled_count,
            "blocked": log.blocked,
            "vertices_active": log.vertices_active,
            "edges_active": log.edges_active,
            "f_value": log.f_value,
        }
        targets = encounter_targets.get(log.step, (None, None))
        d["target_a"] = targets[0]
        d["target_b"] = targets[1]
        step_dicts.append(d)

    # Build concept names from final graph vertex content
    concept_names = {}
    for vid, v in engine.k_active.vertices.items():
        if v.content:
            concept_names[vid] = v.content

    return step_dicts, engine.k_active, concept_names


def main():
    """CLI: re-run experiment and generate narrative."""
    import os

    print("Re-running phenomenology experiment to capture step logs...",
          file=sys.stderr)

    # Change to script directory so imports work
    script_dir = os.path.dirname(os.path.abspath(__file__))
    os.chdir(script_dir)

    step_logs, graph, concept_names = _run_and_capture()

    print(f"Captured {len(step_logs)} steps. Generating narrative...",
          file=sys.stderr)

    narrative = generate_narrative(step_logs, graph, concept_names)
    print(narrative)


if __name__ == "__main__":
    main()
