# Topological Computation: Architecture Specification (v2)

## 0. What This Is

A computational paradigm in which reasoning is topological traversal — movement through a discrete combinatorial structure that is irreversibly changed by the movement itself. The system does not search for optimal solutions. It walks through its own topology, encounters structural features, and responds to encounters with operations that permanently alter the space. The accumulated topological changes constitute the system's knowledge.

Not a neural network. Not a symbolic AI. Not an optimization system. A fourth kind.

## 1. Computation Space

K(t) = (V(t), E(t), τ, σ)
- V(t): vertex set
- E(t): edge set (typed directed)
- τ: E → T: type function
- σ: V → {active, contested, folded}: status function

K_full(t): Complete history. Append-only.
K_active(t): Current working state. Quotient of K_full(t) by folding. Contested vertices remain visible.

Primary observable: β₁(K_active)

## 2. Traversal

System has current position in K_active. Reasoning = movement along edges.

2.1 Walk: Move along edges, guided by Morse terrain (tree/critical edge marking). Critical edges more salient.

2.2 Encounter: During traversal, encounters arise — structural identity (→fold), contradiction (→negate), refinement (→sublate), or nothing.

2.3 LLM as Perception: LLM receives local subgraph, reports what it perceives. Does not evaluate.

## 3. Operations

3.1 Fold (Equivalence Path): Identify S ⊂ V_active as equivalent. Δβ₁ = (c-1) + n_loop.

3.2 Negate (Contradiction Path Step 1): Add antithesis w, negation edge w→v. Both remain in K_active. v marked contested. If path v→...→w exists, negation edge creates cycle → β₁ may increase.

3.3 Sublate (Contradiction Path Step 2): After negation pair (A,B), add synthesis C with C→A, C→B. Contradiction preserved. New structure grows on top.

Two paths: Equivalence (Fold) and Contradiction (Negate→Sublate).

## 4. Constraints

4.1 Settlement: Cycle marked settled after persistence + survived blocked negation + structural integration.

4.2 Topological Impossibility: Operations destroying settled cycles are blocked. Structural, not policy.

4.3 Settlement Review: Rare, costly cascading review to un-settle.

## 5. Self-Reference

β₁ computed each step, available as context. Morse terrain updated after each operation. History entropy β₁(full)-β₁(active) computable.

## 6. Dynamics

No goal function. Movement self-sustains: operation → topology change → terrain change → new encounters → new operations. Good reasoning = sustained productive traversal (β₁ grows, some cycles settle, no degeneration).

## 7. Properties

Irreversibility, History, Contradiction as structure, Intrinsic impossibility, Self-reference, Encounter over search.

## 8. Minimal Experiment

5 vertices, 7 edges, β₁≥1. 30-50 steps. Success: β₁ grows, contradiction path executes, settlement occurs, blocking occurs, coverage ≥80%.

## 9. Open Questions

Traversal policy, LLM perception quality, settlement criteria, scale, multi-agent, domain independence.
