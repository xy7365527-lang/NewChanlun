"""R6 Gemini derive call — 局部穿越范围/停止条件/拓扑查询语言"""
import sys
sys.path.insert(0, r"C:\Users\hanju\NewChanlun")

from dotenv import load_dotenv
load_dotenv(r"C:\Users\hanju\NewChanlun\.env", override=True)

from newchan.gemini.modes import GeminiChallenger

SUBJECT = """\
R6: From batch traversal to dialogue — formalizing question-driven traversal.

Three questions requiring L0 (pure algebraic/topological) analysis:

Q-R6-2: Optimal scope of local traversal.
User asks "What is the relationship between A and B?" Traversal starts from vertices A and B.
How large should the traversal scope be?
Candidates:
(1) Union of k-hop neighborhoods of A and B, with k adaptive.
(2) Topological closure of {A,B}: {A} ∪ {B} ∪ N(A) ∪ N(B) ∪ edges between all these vertices.
(3) ε-tubular neighborhood of the shortest path from A to B.
Which has an intrinsic topological definition (not parameter-dependent)?
Does the scope depend on f(A,B) where f is some topological distance — small f implies small scope, large f requires larger scope to find connections?

Q-R6-3: Stopping condition for question-driven traversal.
Autonomous traversal uses global β₁ convergence to stop. What stops question-driven traversal?
Candidates:
(1) Local β₁ convergence (β₁ of the sub-complex stops changing).
(2) Coverage of the starting vertex set reaches a threshold.
(3) Encounter density falls below a threshold (local exhaustion).
Is there an intrinsic topological stopping criterion that requires NO parameters?

Q-R6-6: Topological query language.
"What is X" → star neighborhood. "What is the relation between X and Y" → path query. "What contradicts X" → negation neighborhood.
Does there exist a set of atomic topological query operations such that any natural language question can be decomposed into a composition of these atomic operations?
Analogy: SQL for relational databases — is there a "TopoQL" for simplicial complexes?

All analysis at L0 (pure definitions/algebra, no empirical data needed).
"""

CONTEXT = """\
Background — Topological Computation Engine (already built):

Architecture:
- Discrete simplicial complex K_active built from concept genealogy
- Traversal = random walk on K_active with encounter detection
- Two paths: Equivalence (Fold — collapse equivalent simplices) and Contradiction (Negate → Sublate)
- Morse theory: Poset Morse (not Forman) with terrain classification (critical/tree)
- β₁ tracks topological complexity (Betti number = independent cycles)
- Negation edges inscribe contradictions as cycles in K_active
- LLM = perceiver (encounter detection), not decision-maker
- Settlement = intrinsic impossibility (not timeout)

Key definitions:
- K_active: Active simplicial complex (vertices = concepts, edges = relations)
- fold(σ,τ): Collapse equivalent simplices σ~τ into one
- negate(σ,τ): Add negation edge creating a cycle (Δβ₁ ≥ 1)
- sublate(σ,τ,ν): Aufhebung — create new vertex ν that absorbs the contradiction
- β₁(K): First Betti number = rank of H₁(K;Z) = number of independent cycles
- Star(v) = {σ ∈ K : v ∈ σ}: Star neighborhood
- Lk(v) = {τ ∈ K : τ ∩ Star(v) = ∅, τ ∪ {v} ∈ K}: Link
- d(A,B): Graph distance (shortest path length) in 1-skeleton

Phase 3 results on real data (2524 vertices, 6751 edges):
- 59 successful folds, 47 settled, β₁: 2325→2395
- Hub effect resolved via Morse terrain (zero parameters)
- Self-reference oscillation fixed by excluding synthetic prefixes
- 62% encounter density, 190 walk steps
"""

challenger = GeminiChallenger()
result = challenger.derive(SUBJECT, CONTEXT, domain="Algebraic Topology / Combinatorial Topology")

# Save output
output_path = r"C:\Users\hanju\NewChanlun\tmp\gemini-R6-dialogue-Q1-3.md"
with open(output_path, "w", encoding="utf-8") as f:
    f.write(f"# Gemini R6: From Batch to Dialogue — Question-Driven Traversal Formalization\n\n")
    f.write(f"**Model**: {result.model}\n")
    f.write(f"**Mode**: {result.mode}\n")
    f.write(f"**Domain**: Algebraic Topology / Combinatorial Topology\n\n")
    f.write("---\n\n")
    f.write(result.response)

print(f"Saved to {output_path}")
print(f"Model: {result.model}")
print(f"Response length: {len(result.response)} chars")
print("\n" + "="*80)
print(result.response[:3000])
print("..." if len(result.response) > 3000 else "")
