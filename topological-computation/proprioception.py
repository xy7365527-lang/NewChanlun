"""Proprioception — system self-sensing vertices and self-reflexive norms.

The system's runtime state (settled_cycle_count, total_vertices, etc.) is
injected into K_active as vertices. Self-reflexive norms map theoretical
concepts to operational conditions. When the traverser visits a proprioception
vertex, it checks whether norms are violated and produces encounters.

Pure Python, no external dependencies beyond this project's modules.
"""

from __future__ import annotations

from dataclasses import dataclass
from engine import Graph, Vertex, Edge, EdgeType, VertexStatus


# ---------------------------------------------------------------------------
# Proprioception vertex management
# ---------------------------------------------------------------------------

PROPRIOCEPTION_PREFIX = "[proprioception]"
SELF_NORM_PREFIX = "[self-norm]"


@dataclass(frozen=True, slots=True)
class ProprioceptionMetrics:
    """Snapshot of system runtime state."""
    settled_cycle_count: int
    total_vertices: int
    total_edges: int
    nothing_streak: int
    expression_pressure: int
    blocked_streak: int


def collect_metrics(
    k_active: Graph,
    settlement_settled_count: int,
    nothing_streak: int,
    blocked_streak: int,
    unreported_count: int,
) -> ProprioceptionMetrics:
    """Collect current system metrics from engine state."""
    return ProprioceptionMetrics(
        settled_cycle_count=settlement_settled_count,
        total_vertices=len(k_active.active_vertex_ids()),
        total_edges=len(k_active.active_edges()),
        nothing_streak=nothing_streak,
        expression_pressure=unreported_count,
        blocked_streak=blocked_streak,
    )


def update_proprioception_vertices(
    graph: Graph,
    metrics: ProprioceptionMetrics,
    step: int,
) -> Graph:
    """Update (or create) proprioception vertices in the graph.

    Each metric becomes a vertex with id "proprioception:<key>".
    Graph is immutable, so returns a new Graph with updated vertices.
    """
    metric_items = {
        "settled_cycle_count": metrics.settled_cycle_count,
        "total_vertices": metrics.total_vertices,
        "total_edges": metrics.total_edges,
        "nothing_streak": metrics.nothing_streak,
        "expression_pressure": metrics.expression_pressure,
        "blocked_streak": metrics.blocked_streak,
    }

    for key, value in metric_items.items():
        vid = f"proprioception:{key}"
        content = f"{PROPRIOCEPTION_PREFIX} {key} = {value}"
        existing = graph.vertex(vid)
        if existing is not None:
            # Vertex exists — replace with updated content.
            # Since Vertex is frozen, we create a new one and replace via
            # the internal dict approach (add_vertex overwrites by id).
            new_v = Vertex(vid, existing.status, content, existing.created_at)
        else:
            new_v = Vertex(vid, VertexStatus.ACTIVE, content, step)
        graph = graph.add_vertex(new_v)

    return graph


# ---------------------------------------------------------------------------
# Self-reflexive norms
# ---------------------------------------------------------------------------

@dataclass(frozen=True, slots=True)
class SelfReflexiveNorm:
    """A mapping from theoretical concept to operational condition."""
    theoretical_concept: str
    operational_norm: str
    diagnosis: str


SELF_REFLEXIVE_NORMS: tuple[SelfReflexiveNorm, ...] = (
    SelfReflexiveNorm(
        theoretical_concept="矛盾是运动的动力",
        operational_norm="unsettled_count > 0",
        diagnosis="所有cycle被settled = 没有矛盾 = 没有运动 = 热寂",
    ),
    SelfReflexiveNorm(
        theoretical_concept="走势终完美",
        operational_norm="settled_cycle_count increases over time",
        diagnosis="settlement停滞 = 穿越无法产出新认知",
    ),
    SelfReflexiveNorm(
        theoretical_concept="Aufhebung同时是新矛盾的起点",
        operational_norm="每次settlement的residue非空",
        diagnosis="空residue = closure without transformation",
    ),
)


def inject_self_reflexive_norms(graph: Graph, step: int) -> Graph:
    """Inject self-reflexive norm vertices and edges into the graph.

    Each norm becomes a vertex. An edge connects it to the relevant
    proprioception vertex, creating the theoretical→operational mapping.

    Only injects norms not already present (idempotent).
    """
    for norm in SELF_REFLEXIVE_NORMS:
        vid = f"norm:{norm.theoretical_concept}"
        if graph.vertex(vid) is not None:
            continue

        content = (
            f"{SELF_NORM_PREFIX} {norm.theoretical_concept} "
            f"→ {norm.operational_norm}. "
            f"违反诊断: {norm.diagnosis}"
        )
        v = Vertex(vid, VertexStatus.ACTIVE, content, step)
        graph = graph.add_vertex(v)

        # Connect norm to relevant proprioception vertex
        prop_target = _norm_to_proprioception_target(norm)
        if prop_target and graph.vertex(prop_target) is not None:
            edge = Edge(vid, prop_target, EdgeType.REFERENCE, step)
            graph = graph.add_edge(edge)

    return graph


def _norm_to_proprioception_target(norm: SelfReflexiveNorm) -> str | None:
    """Map a norm to its primary proprioception vertex."""
    concept = norm.theoretical_concept
    if concept == "矛盾是运动的动力":
        return "proprioception:settled_cycle_count"
    if concept == "走势终完美":
        return "proprioception:settled_cycle_count"
    if concept == "Aufhebung同时是新矛盾的起点":
        return "proprioception:expression_pressure"
    return None


# ---------------------------------------------------------------------------
# Self-diagnosis: check norms against current metrics
# ---------------------------------------------------------------------------

@dataclass(frozen=True, slots=True)
class NormViolation:
    """A detected violation of a self-reflexive norm."""
    norm: SelfReflexiveNorm
    actual_state: str
    severity: str  # "warning" or "critical"


def check_norms(
    metrics: ProprioceptionMetrics,
    settlement_total_cycles: int,
) -> list[NormViolation]:
    """Check all self-reflexive norms against current metrics.

    Returns list of violations (empty if all norms satisfied).
    """
    violations: list[NormViolation] = []

    # Norm 1: 矛盾是运动的动力 — unsettled_count > 0
    unsettled = settlement_total_cycles - metrics.settled_cycle_count
    if unsettled <= 0 and metrics.settled_cycle_count > 0:
        violations.append(NormViolation(
            norm=SELF_REFLEXIVE_NORMS[0],
            actual_state=f"settled={metrics.settled_cycle_count}, unsettled={unsettled}",
            severity="critical",
        ))

    # Norm 2: 走势终完美 — settled_cycle_count increases over time
    # This is checked via nothing_streak as a proxy: prolonged nothing = no new settlement
    if metrics.nothing_streak >= 20 and metrics.settled_cycle_count > 0:
        violations.append(NormViolation(
            norm=SELF_REFLEXIVE_NORMS[1],
            actual_state=f"nothing_streak={metrics.nothing_streak}",
            severity="warning",
        ))

    # Norm 3: Aufhebung同时是新矛盾的起点 — residue non-empty
    # expression_pressure = 0 after settlements means no residue articulation
    if metrics.expression_pressure == 0 and metrics.settled_cycle_count > 5:
        violations.append(NormViolation(
            norm=SELF_REFLEXIVE_NORMS[2],
            actual_state=f"expression_pressure=0, settled={metrics.settled_cycle_count}",
            severity="warning",
        ))

    return violations
