# Topological Computation: Architecture Specification

## 0. What This Is

A computational paradigm in which reasoning is performed through topological operations on simplicial complexes, not through vector arithmetic in continuous spaces. The system thinks by creating, destroying, and restructuring topological features — holes, cycles, connected components — in a discrete combinatorial structure. Each reasoning step irreversibly changes the topology of the computation space, and the accumulated topological history constitutes the system's knowledge.

This is not a neural network. Not a symbolic AI. Not a hybrid. It is a third kind.

## 1. Computation Space

The state of the system at time t is a typed directed simplicial complex:

K(t) = (V(t), E(t), τ, σ)

- V(t): vertex set (concepts, propositions, or any discrete knowledge units)
- E(t): edge set (typed directed relations between vertices)
- τ: E → T: type function, where T is a finite set of relation types
- σ: V → {active, negated, folded}: status function

Dual-view structure:
- K_full(t): complete history — append-only, never shrinks
- K_active(t): current working state — quotient space of K_full(t)

Primary observable: β₁(K_active) — number of independent irreducible cycles.

## 2. Operations

### 2.1 Fold
Identify S ⊂ V_active (|S| ≥ 2) as equivalent.

Δβ₁ = (c - 1) + n_loop

where c = connected components of lower link l⁻(p), n_loop = edges within S becoming self-loops.

### 2.2 Negate
Add vertex w and negation edge w → v, removing v from K_active.

### 2.3 Sublate
Add vertex w with revision edge w → v. v remains active but topological neighborhood changes.

## 3. Effect Prediction

Every operation's topological effect is analytically predictable before execution. O(|E|) for fold, O(|V|+|E|) for negate/sublate.

## 4. Selection Criterion

Dual-objective tension:
- Objective A: β₁ growth (topological production)
- Objective B: settlement density (irreversibility accumulation)

T(op) = Δg(op) × (1 - s) + Δs(op) × g

where g(t) = β₁(K_active(t))/t, s(t) = |settled cycles| / β₁(K_active(t))

Select max T. If all T ≤ 0, wait.

### 4.2 Automatic Settlement
Cycle settled when: persisted N steps + survived negation attempt + structurally integrated.

### 4.3 Intrinsic Impossibility
Settled cycles impose constraints emerging from topology, not external rules.

## 5. Candidate Generation

LLM as pruning engine (not reasoner). Proposes M≈10-20 candidates. Topological layer evaluates (Δβ₁, T, settlement constraints) and executes.

Isomorphic to AlphaGo: Policy network (LLM) → Value network (Δβ₁ + T) → Game engine (topological layer).

## 6. Self-Reference

β₁(K_active) computed at every step, enters criterion T through g(t). System observes own topological state → determines next operation → changes state → observes again.

History entropy: β₁(K_full) - β₁(K_active).

## 7. Properties

- Irreversibility: every operation appends to K_full
- History: complete record of every operation
- Contradiction as Structure: cycles from negation edges (β₁ contributions), not loss to minimize
- Intrinsic Impossibility: settled topology excludes operations architecturally

## 8. Comparison

| Dimension | Neural Network | Symbolic AI | Topological Computation |
|-----------|---------------|-------------|------------------------|
| State space | Continuous vector | Discrete propositions | Typed directed simplicial complex |
| Effect predictability | Opaque | Transparent | Analytic (Δβ₁ formula) |
| History | None during inference | Derivation tree | Complete, append-only, irreversible |
| Contradiction | Minimized | Forbidden | Structural feature |
| Self-reference | Impossible | Limited (Gödel) | Native (β₁ from own structure) |
| Impossibility | External (RLHF) | External (axioms) | Intrinsic (settled topology) |

## 9. Minimal Experiment

8-10 vertices, 12-15 typed directed edges. 20-30 steps. Success criteria: β₁ increases, settlement occurs, no degeneration, T non-trivially distributed, at least one operation rejected by settlement.

## 10. Open Questions

1. Is T optimal? Nonlinear coupling? Adaptive forms?
2. Multi-step planning (MCTS over topological operations)?
3. Scale (incremental β₁ algorithms)?
4. Natural language interface?
5. Domain independence?
6. Persistent homology of the reasoning process itself?
