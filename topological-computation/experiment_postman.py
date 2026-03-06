"""Neil Postman — collected works incremental traversal experiment.

Each work is injected as a conceptual complex (vertices + edges).
After injection, the traversal engine "digests" the work
(runs until beta_1 is stable for 50 consecutive steps).
Then the next work is injected.

This models the progressive development of Postman's media ecology
and educational thought as an incremental topological growth process.

Covers ~11 works spanning 1969-1999.
"""

from __future__ import annotations

import json
import sys
import time

sys.path.insert(0, ".")

from engine import Graph, Vertex, Edge, EdgeType, VertexStatus, compute_beta_1
from traversal import TraversalEngine
from morse import compute_terrain


# ---------------------------------------------------------------------------
# Shared / recurring vertex IDs across works:
#   medium, message, television, entertainment, discourse,
#   typography, information, childhood, technology, technopoly,
#   education, epistemology, public_discourse, reason, democracy,
#   culture, narrative, metaphor, language, print_culture,
#   image, spectacle, news, advertising, propaganda,
#   Enlightenment, conversation, meaning, context, judgment
# ---------------------------------------------------------------------------


def work_01_subversive_activity():
    """1969: Teaching as a Subversive Activity (with Charles Weingartner).

    The inquiry method. The medium is the message in education.
    Against the banking model. The crap detector. Students as
    meaning-makers, not passive receivers.
    """
    vertices = [
        Vertex("education", content="education — the process of meaning-making, not information transfer"),
        Vertex("inquiry", content="inquiry — the subversive method of asking questions"),
        Vertex("medium", content="medium — the environment that shapes perception and thought"),
        Vertex("message", content="message — the content carried by, and transformed by, the medium"),
        Vertex("meaning", content="meaning — constructed by the learner, not delivered by the teacher"),
        Vertex("language", content="language — the tool and environment of thought"),
        Vertex("metaphor", content="metaphor — the hidden comparison that structures understanding"),
        Vertex("crap_detector", content="the crap detector — the capacity to distinguish sense from nonsense"),
    ]
    edges = [
        Edge("education", "inquiry", EdgeType.DEPENDENCY),
        Edge("medium", "message", EdgeType.DEPENDENCY),
        Edge("inquiry", "meaning", EdgeType.DEPENDENCY),
        Edge("language", "meaning", EdgeType.DEPENDENCY),
        Edge("metaphor", "language", EdgeType.DEPENDENCY),
        Edge("crap_detector", "inquiry", EdgeType.DEPENDENCY),
        Edge("education", "medium", EdgeType.REFERENCE),
        Edge("crap_detector", "meaning", EdgeType.DEPENDENCY),
    ]
    return "1969: Teaching as a Subversive Activity", vertices, edges


def work_02_soft_revolution():
    """1971: The Soft Revolution — A Student Handbook for Turning Schools Around.

    Practical sequel to Subversive Activity. Students as agents
    of educational change. The school as a medium.
    """
    vertices = [
        Vertex("school", content="the school — an environment (medium) that shapes what can be thought"),
        Vertex("student_agency", content="student agency — learners as active participants in institutional change"),
    ]
    edges = [
        Edge("school", "medium", EdgeType.DEPENDENCY),
        Edge("school", "education", EdgeType.DEPENDENCY),
        Edge("student_agency", "inquiry", EdgeType.DEPENDENCY),
        Edge("student_agency", "education", EdgeType.DEPENDENCY),
    ]
    return "1971: The Soft Revolution", vertices, edges


def work_03_crazy_talk():
    """1976: Crazy Talk, Stupid Talk.

    The ecology of language. Semantic environments. How language
    creates and destroys meaning. The confusion of levels.
    """
    vertices = [
        Vertex("semantic_environment", content="semantic environment — the context that determines what words mean"),
        Vertex("crazy_talk", content="crazy talk — language that confuses levels of abstraction"),
        Vertex("stupid_talk", content="stupid talk — language that is irrelevant to its purpose"),
        Vertex("abstraction", content="levels of abstraction — the hierarchy from particular to general"),
    ]
    edges = [
        Edge("semantic_environment", "language", EdgeType.DEPENDENCY),
        Edge("semantic_environment", "meaning", EdgeType.DEPENDENCY),
        Edge("crazy_talk", "semantic_environment", EdgeType.NEGATION),
        Edge("stupid_talk", "semantic_environment", EdgeType.NEGATION),
        Edge("abstraction", "language", EdgeType.DEPENDENCY),
        Edge("crazy_talk", "abstraction", EdgeType.NEGATION),
        Edge("semantic_environment", "metaphor", EdgeType.REFERENCE),
    ]
    return "1976: Crazy Talk, Stupid Talk", vertices, edges


def work_04_teaching_as_conserving():
    """1979: Teaching as a Conserving Activity.

    The thermostatic view of education. When culture overheats
    with information, education must cool it down. Against his
    own earlier radicalism — now culture needs conservation.
    """
    vertices = [
        Vertex("thermostat", content="the thermostatic function — education as cultural counterbalance"),
        Vertex("information_glut", content="information glut — too much information, too little meaning"),
        Vertex("conservation", content="conservation — preserving coherent meaning against information flood"),
        Vertex("culture", content="culture — the shared system of meaning and narrative"),
    ]
    edges = [
        Edge("thermostat", "education", EdgeType.DEPENDENCY),
        Edge("thermostat", "culture", EdgeType.DEPENDENCY),
        Edge("information_glut", "meaning", EdgeType.NEGATION),
        Edge("conservation", "thermostat", EdgeType.DEPENDENCY),
        Edge("conservation", "culture", EdgeType.DEPENDENCY),
        Edge("information_glut", "medium", EdgeType.DEPENDENCY),
        Edge("thermostat", "inquiry", EdgeType.NEGATION),
        Edge("culture", "narrative", EdgeType.DEPENDENCY),
    ]
    return "1979: Teaching as a Conserving Activity", vertices, edges


def work_05_disappearance_childhood():
    """1982: The Disappearance of Childhood.

    Childhood is a social invention of print culture. Television
    dissolves the boundary between adult and child by making all
    information available. Without secrets, no childhood.
    """
    vertices = [
        Vertex("childhood", content="childhood — a social construction created by print literacy"),
        Vertex("print_culture", content="print culture — the world organized by typography and reading"),
        Vertex("television", content="television — the medium that dissolves print-culture distinctions"),
        Vertex("secrecy", content="secrecy — the adult monopoly on information that creates childhood"),
        Vertex("image", content="image — the visual, immediate, non-sequential mode of television"),
        Vertex("narrative", content="narrative — the sequential, coherent story that organizes meaning"),
    ]
    edges = [
        Edge("childhood", "print_culture", EdgeType.DEPENDENCY),
        Edge("childhood", "secrecy", EdgeType.DEPENDENCY),
        Edge("television", "childhood", EdgeType.NEGATION),
        Edge("television", "secrecy", EdgeType.NEGATION),
        Edge("television", "image", EdgeType.DEPENDENCY),
        Edge("print_culture", "narrative", EdgeType.DEPENDENCY),
        Edge("image", "narrative", EdgeType.NEGATION),
        Edge("television", "medium", EdgeType.DEPENDENCY),
        Edge("print_culture", "medium", EdgeType.DEPENDENCY),
    ]
    return "1982: The Disappearance of Childhood", vertices, edges


def work_06_amusing_ourselves():
    """1985: Amusing Ourselves to Death — Public Discourse in the Age of Show Business.

    Postman's masterwork. All public discourse becomes entertainment
    under television. Huxley was right, not Orwell. The epistemology
    of television versus the epistemology of typography.
    """
    vertices = [
        Vertex("entertainment", content="entertainment — the supra-ideology of television"),
        Vertex("public_discourse", content="public discourse — the conversation a culture has with itself"),
        Vertex("typography", content="typography — the culture of print, sequential thought, and argument"),
        Vertex("epistemology", content="epistemology — the theory of knowledge implicit in every medium"),
        Vertex("show_business", content="show business — the transformation of all discourse into spectacle"),
        Vertex("Huxley", content="Huxley — Brave New World, control through pleasure"),
        Vertex("Orwell", content="Orwell — 1984, control through pain"),
        Vertex("reason", content="reason — the capacity for sustained, sequential, evidence-based argument"),
        Vertex("democracy", content="democracy — requires informed citizens engaged in rational discourse"),
    ]
    edges = [
        Edge("entertainment", "television", EdgeType.DEPENDENCY),
        Edge("entertainment", "public_discourse", EdgeType.NEGATION),
        Edge("public_discourse", "democracy", EdgeType.DEPENDENCY),
        Edge("typography", "reason", EdgeType.DEPENDENCY),
        Edge("typography", "public_discourse", EdgeType.DEPENDENCY),
        Edge("epistemology", "medium", EdgeType.DEPENDENCY),
        Edge("show_business", "entertainment", EdgeType.DEPENDENCY),
        Edge("television", "typography", EdgeType.NEGATION),
        Edge("television", "reason", EdgeType.NEGATION),
        Edge("Huxley", "Orwell", EdgeType.NEGATION),
        Edge("Huxley", "entertainment", EdgeType.REFERENCE),
        Edge("democracy", "reason", EdgeType.DEPENDENCY),
        Edge("epistemology", "culture", EdgeType.DEPENDENCY),
        Edge("entertainment", "image", EdgeType.DEPENDENCY),
        Edge("show_business", "public_discourse", EdgeType.NEGATION),
    ]
    return "1985: Amusing Ourselves to Death", vertices, edges


def work_07_conscientious_objections():
    """1988: Conscientious Objections — Stirring Up Trouble About Language, Technology, and Education.

    Essay collection. Media ecology refined. The Faustian bargain
    of every technology. The bias of communication.
    """
    vertices = [
        Vertex("technology", content="technology — not neutral but carrying ideology and epistemological bias"),
        Vertex("Faustian_bargain", content="the Faustian bargain — every technology gives and takes away"),
        Vertex("bias", content="bias of communication — every medium privileges certain forms of content"),
    ]
    edges = [
        Edge("technology", "medium", EdgeType.DEPENDENCY),
        Edge("Faustian_bargain", "technology", EdgeType.DEPENDENCY),
        Edge("bias", "medium", EdgeType.DEPENDENCY),
        Edge("bias", "epistemology", EdgeType.DEPENDENCY),
        Edge("Faustian_bargain", "culture", EdgeType.DEPENDENCY),
        Edge("technology", "entertainment", EdgeType.REFERENCE),
    ]
    return "1988: Conscientious Objections", vertices, edges


def work_08_technopoly():
    """1992: Technopoly — The Surrender of Culture to Technology.

    Three stages: tool-using, technocracy, technopoly. In technopoly,
    technology deifies itself and eliminates all alternative sources
    of meaning. Information as garbage without context.
    """
    vertices = [
        Vertex("technopoly", content="technopoly — the totalitarian rule of technology over culture"),
        Vertex("tool_using", content="tool-using culture — technology subordinate to theology or philosophy"),
        Vertex("technocracy", content="technocracy — technology challenges but coexists with traditional beliefs"),
        Vertex("information", content="information — data without context, meaning, or purpose"),
        Vertex("context", content="context — the framework that gives information meaning"),
        Vertex("tradition", content="tradition — the inherited framework of meaning and value"),
        Vertex("judgment", content="judgment — the capacity to evaluate, needing context and tradition"),
    ]
    edges = [
        Edge("technopoly", "technology", EdgeType.DEPENDENCY),
        Edge("technopoly", "culture", EdgeType.NEGATION),
        Edge("tool_using", "technology", EdgeType.DEPENDENCY),
        Edge("technocracy", "tool_using", EdgeType.SUBLATION),
        Edge("technopoly", "technocracy", EdgeType.SUBLATION),
        Edge("information", "meaning", EdgeType.NEGATION),
        Edge("information", "context", EdgeType.NEGATION),
        Edge("context", "meaning", EdgeType.DEPENDENCY),
        Edge("tradition", "context", EdgeType.DEPENDENCY),
        Edge("technopoly", "tradition", EdgeType.NEGATION),
        Edge("judgment", "context", EdgeType.DEPENDENCY),
        Edge("judgment", "tradition", EdgeType.DEPENDENCY),
        Edge("technopoly", "information_glut", EdgeType.DEPENDENCY),
        Edge("information", "technology", EdgeType.DEPENDENCY),
    ]
    return "1992: Technopoly", vertices, edges


def work_09_how_to_watch_tv_news():
    """1992: How to Watch TV News (with Steve Powers).

    The grammar of television news. News as entertainment.
    The economic logic that shapes what counts as 'news'.
    """
    vertices = [
        Vertex("news", content="news — the constructed spectacle presenting itself as reality"),
        Vertex("advertising", content="advertising — the economic substrate that shapes news content"),
    ]
    edges = [
        Edge("news", "television", EdgeType.DEPENDENCY),
        Edge("news", "entertainment", EdgeType.DEPENDENCY),
        Edge("news", "public_discourse", EdgeType.NEGATION),
        Edge("advertising", "news", EdgeType.DEPENDENCY),
        Edge("advertising", "show_business", EdgeType.REFERENCE),
        Edge("news", "information", EdgeType.REFERENCE),
        Edge("news", "context", EdgeType.NEGATION),
    ]
    return "1992: How to Watch TV News", vertices, edges


def work_10_end_of_education():
    """1995: The End of Education — Redefining the Value of School.

    Education needs a narrative (a 'god'). Five narratives:
    spaceship earth, fallen angel, American experiment, diversity,
    word weavers. Without narrative, school is meaningless.
    """
    vertices = [
        Vertex("end_of_education", content="the end (purpose) of education — what narrative justifies schooling?"),
        Vertex("god", content="'god' — the narrative or grand story that gives education meaning"),
        Vertex("spaceship_earth", content="spaceship earth — the ecological narrative for education"),
        Vertex("fallen_angel", content="the fallen angel — education as the correction of human error"),
    ]
    edges = [
        Edge("end_of_education", "education", EdgeType.DEPENDENCY),
        Edge("end_of_education", "narrative", EdgeType.DEPENDENCY),
        Edge("god", "narrative", EdgeType.DEPENDENCY),
        Edge("god", "meaning", EdgeType.DEPENDENCY),
        Edge("spaceship_earth", "god", EdgeType.REFERENCE),
        Edge("fallen_angel", "god", EdgeType.REFERENCE),
        Edge("end_of_education", "culture", EdgeType.DEPENDENCY),
        Edge("end_of_education", "technopoly", EdgeType.NEGATION),
    ]
    return "1995: The End of Education", vertices, edges


def work_11_bridge_to_18th():
    """1999: Building a Bridge to the 18th Century — How the Past Can Improve Our Future.

    Postman's last major work. Return to Enlightenment values.
    Reason, skepticism, narrative, childhood, democracy — all
    threatened by the digital revolution. The 18th century as
    the source of resistance.
    """
    vertices = [
        Vertex("Enlightenment", content="the Enlightenment — the source of reason, democracy, and childhood"),
        Vertex("digital_revolution", content="the digital revolution — the new threat to Enlightenment values"),
        Vertex("skepticism", content="skepticism — the Enlightenment virtue of questioning all claims"),
        Vertex("conversation", content="conversation — the Enlightenment form of public discourse"),
    ]
    edges = [
        Edge("Enlightenment", "reason", EdgeType.DEPENDENCY),
        Edge("Enlightenment", "democracy", EdgeType.DEPENDENCY),
        Edge("Enlightenment", "childhood", EdgeType.DEPENDENCY),
        Edge("digital_revolution", "technopoly", EdgeType.DEPENDENCY),
        Edge("digital_revolution", "Enlightenment", EdgeType.NEGATION),
        Edge("skepticism", "crap_detector", EdgeType.REFERENCE),
        Edge("skepticism", "reason", EdgeType.DEPENDENCY),
        Edge("conversation", "public_discourse", EdgeType.DEPENDENCY),
        Edge("conversation", "typography", EdgeType.DEPENDENCY),
        Edge("digital_revolution", "television", EdgeType.SUBLATION),
        Edge("Enlightenment", "tradition", EdgeType.REFERENCE),
    ]
    return "1999: Building a Bridge to the 18th Century", vertices, edges


# ---------------------------------------------------------------------------
# Work list (chronological)
# ---------------------------------------------------------------------------

ALL_WORKS = [
    work_01_subversive_activity,       # 1969
    work_02_soft_revolution,           # 1971
    work_03_crazy_talk,                # 1976
    work_04_teaching_as_conserving,    # 1979
    work_05_disappearance_childhood,   # 1982
    work_06_amusing_ourselves,         # 1985
    work_07_conscientious_objections,  # 1988
    work_08_technopoly,                # 1992
    work_09_how_to_watch_tv_news,      # 1992
    work_10_end_of_education,          # 1995
    work_11_bridge_to_18th,            # 1999
]


# ---------------------------------------------------------------------------
# Incremental injection + digestion
# ---------------------------------------------------------------------------


def run_experiment():
    total_works = len(ALL_WORKS)
    print("=" * 70)
    print("NEIL POSTMAN — COLLECTED WORKS INCREMENTAL TRAVERSAL")
    print(f"{total_works} works, incremental injection, digestion until beta_1 stable")
    print("=" * 70)

    graph = Graph()
    engine = None
    work_results = []
    beta_1_curve = []
    cumulative_steps = 0

    for w_idx, w_fn in enumerate(ALL_WORKS):
        work_name, w_vertices, w_edges = w_fn()
        print(f"\n{'─' * 60}")
        print(f"Work {w_idx + 1}/{total_works}: {work_name}")
        print(f"{'─' * 60}")

        # 1. Inject vertices
        new_v_count = 0
        existing_vids = set(graph.vertices.keys())
        for v in w_vertices:
            if v.id not in existing_vids:
                graph = graph.add_vertex(v)
                new_v_count += 1

        # 2. Inject edges (skip duplicates)
        existing_edges = {(e.source, e.target, e.edge_type) for e in graph.edges}
        new_e_count = 0
        for e in w_edges:
            key = (e.source, e.target, e.edge_type)
            if key not in existing_edges:
                if e.source in graph.vertices and e.target in graph.vertices:
                    graph = graph.add_edge(e)
                    existing_edges.add(key)
                    new_e_count += 1

        beta_before = compute_beta_1(graph)
        print(f"  Injected: {new_v_count} vertices, {new_e_count} edges")
        print(f"  Complex now: {len(graph.active_vertex_ids())} V, {len(graph.active_edges())} E")
        print(f"  beta_1 after injection: {beta_before}")

        # 3. Create or update engine
        if engine is None:
            start = graph.active_vertex_ids()[0]
            engine = TraversalEngine(
                graph, start=start, settlement_threshold=10, seed=42,
            )
        else:
            engine.k_active = graph
            engine.k_full = graph
            engine.terrain = compute_terrain(engine.k_active)
            if engine.position not in graph.vertices:
                engine.position = graph.active_vertex_ids()[0]

        # 4. Digest: run until beta_1 stable for 50 consecutive steps
        stable_count = 0
        last_beta = compute_beta_1(engine.k_active)
        steps_this_work = 0
        max_steps = 2000

        while stable_count < 50 and steps_this_work < max_steps:
            engine.run_step()
            steps_this_work += 1
            cumulative_steps += 1
            current_beta = compute_beta_1(engine.k_active)
            if current_beta == last_beta:
                stable_count += 1
            else:
                stable_count = 0
                last_beta = current_beta

        beta_after = compute_beta_1(engine.k_active)
        converged = stable_count >= 50

        settled_count = len(engine.settlement.settled_cycles)
        blocked_count = len(engine.settlement.blocked_log)

        print(f"  Steps to digest: {steps_this_work} "
              f"{'(CONVERGED)' if converged else '(MAX REACHED)'}")
        print(f"  beta_1 after digestion: {beta_after} "
              f"(delta from injection: {beta_after - beta_before})")
        print(f"  Settled cycles: {settled_count}")
        print(f"  Active complex: {len(engine.k_active.active_vertex_ids())} V, "
              f"{len(engine.k_active.active_edges())} E")

        # Count operations in this work's digestion
        work_logs = engine.logs[-steps_this_work:] if steps_this_work > 0 else []
        op_counts = {}
        for log in work_logs:
            op_counts[log.operation] = op_counts.get(log.operation, 0) + 1

        # Record
        result = {
            "work": work_name,
            "work_index": w_idx + 1,
            "vertices_added": new_v_count,
            "edges_added": new_e_count,
            "vertices_total": len(engine.k_active.active_vertex_ids()),
            "edges_total": len(engine.k_active.active_edges()),
            "beta_1_before_injection": beta_before,
            "beta_1_after_digestion": beta_after,
            "delta_beta_1": beta_after - beta_before,
            "settled_cycles": settled_count,
            "blocked_in_work": blocked_count,
            "steps_to_digest": steps_this_work,
            "converged": converged,
            "cumulative_steps": cumulative_steps,
            "operation_counts": op_counts,
        }
        work_results.append(result)

        beta_1_curve.append({
            "work_index": w_idx + 1,
            "work_name": work_name,
            "beta_1": beta_after,
            "cumulative_steps": cumulative_steps,
        })

        # Update graph reference for next work injection
        graph = engine.k_active

    # ---------------------------------------------------------------------------
    # Final summary
    # ---------------------------------------------------------------------------
    print("\n" + "=" * 70)
    print("FINAL SUMMARY")
    print("=" * 70)

    final_beta = compute_beta_1(engine.k_active)
    final_v = len(engine.k_active.active_vertex_ids())
    final_e = len(engine.k_active.active_edges())
    final_settled = len(engine.settlement.settled_cycles)

    print(f"\n  Total steps: {cumulative_steps}")
    print(f"  Final beta_1: {final_beta}")
    print(f"  Final complex: {final_v} V, {final_e} E")
    print(f"  Settled cycles: {final_settled}")

    # Beta_1 growth curve
    print(f"\n  beta_1 growth curve (by work):")
    for entry in beta_1_curve:
        bar = "#" * min(entry["beta_1"], 80)
        print(f"    W{entry['work_index']:2d}: beta_1={entry['beta_1']:4d} "
              f"steps={entry['cumulative_steps']:5d} {bar}")
        print(f"          {entry['work_name']}")

    # Biggest jumps
    deltas = [(r["work"], r["delta_beta_1"], r["work_index"]) for r in work_results]
    deltas_sorted = sorted(deltas, key=lambda x: -x[1])
    print(f"\n  Largest beta_1 jumps by work:")
    for name, delta, idx in deltas_sorted[:5]:
        print(f"    W{idx:2d} {name}: +{delta}")

    # Digestion effort
    print(f"\n  Digestion effort by work:")
    for r in work_results:
        print(f"    W{r['work_index']:2d} {r['work']}: "
              f"{r['steps_to_digest']} steps {'OK' if r['converged'] else 'MAX'}")

    # Settled cycles detail
    if engine.settlement.settled_cycles:
        print(f"\n  Settled cycles ({final_settled}):")
        for i, sc in enumerate(engine.settlement.settled_cycles[:20]):
            print(f"    [{i+1}] settled@step={sc.settled_at_step}: "
                  f"{sorted(sc.edges)[:3]}...")

    # Final terrain
    final_terrain = compute_terrain(engine.k_active)
    ft_counts = {"tree": 0, "critical": 0}
    for mark in final_terrain.values():
        ft_counts[mark] = ft_counts.get(mark, 0) + 1
    crit_ratio = (ft_counts["critical"] / (ft_counts["tree"] + ft_counts["critical"])
                  if (ft_counts["tree"] + ft_counts["critical"]) > 0 else 0)
    print(f"\n  Final terrain: {ft_counts}")
    print(f"  Critical edge ratio: {crit_ratio:.4f}")

    # Edge type distribution
    type_counts = {}
    for e in engine.k_active.active_edges():
        type_counts[e.edge_type.value] = type_counts.get(e.edge_type.value, 0) + 1
    print(f"\n  Edge type distribution:")
    for et, count in sorted(type_counts.items()):
        print(f"    {et}: {count}")

    # Negation density
    neg_count = type_counts.get("negation", 0)
    neg_density = neg_count / final_e if final_e > 0 else 0
    print(f"\n  Negation density: {neg_density:.4f} ({neg_count}/{final_e})")

    # ---------------------------------------------------------------------------
    # Save
    # ---------------------------------------------------------------------------
    output = {
        "experiment": "postman_collected_works_incremental",
        "source": "Neil Postman — collected works, 11 entries (1969-1999)",
        "methodology": "manual conceptual complex per work, incremental injection, "
                        "digestion until 50-step beta_1 stability",
        "work_results": work_results,
        "beta_1_curve": beta_1_curve,
        "final_complex": {
            "vertices_active": final_v,
            "edges_active": final_e,
            "beta_1": final_beta,
            "settled_cycles": final_settled,
            "edge_type_distribution": type_counts,
            "terrain_distribution": ft_counts,
            "critical_edge_ratio": crit_ratio,
            "negation_density": neg_density,
        },
        "total_steps": cumulative_steps,
        "settled_cycles_detail": [
            {"edges": sorted(sc.edges), "settled_at_step": sc.settled_at_step}
            for sc in engine.settlement.settled_cycles
        ],
        "blocked_log": engine.settlement.blocked_log,
    }

    output_path = "experiment_postman.json"
    with open(output_path, "w", encoding="utf-8") as f:
        json.dump(output, f, indent=2, ensure_ascii=False)

    print(f"\n  Results saved to {output_path}")
    return output


if __name__ == "__main__":
    run_experiment()
