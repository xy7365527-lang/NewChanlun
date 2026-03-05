"""调用 Gemini derive 审查拓扑计算架构规格 — Q-S1/Q-S2/Q-S3/Q-S4 四个核心问题。

用法: python tmp/gemini_topo_spec_review.py
输出: tmp/gemini-topo-spec-review-result.md
"""

import sys
import os
import datetime

# 确保项目根在 sys.path
ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
sys.path.insert(0, os.path.join(ROOT, "src"))

from dotenv import load_dotenv
load_dotenv(os.path.join(ROOT, ".env"), override=True)

from newchan.gemini.modes import GeminiChallenger

CONTEXT_FILE = os.path.join(ROOT, "tmp", "challenge-ctx-topo-spec.md")
OUTPUT_FILE = os.path.join(ROOT, "tmp", "gemini-topo-spec-review-result.md")

SUBJECT = """\
Mathematical Review of Topological Computation Architecture Specification (10 chapters).

Four core questions requiring complete mathematical reasoning:

Q-S1: Selection Criterion T — mathematical properties.
  T(op) = Δg(op)×(1-s) + Δs(op)×g where g=β₁/t, s=|settled|/β₁.
  (a) Fixed points: does T=0 for all candidates have solutions? Is there a "stop selecting" state?
  (b) Cross-coupling degeneracy prevention: g→0 behavior? s→1 behavior?
  (c) Variational principle: is T the gradient of some functional?
  (d) Wait state (T≤0) meaning: equivalent to non-stationary growth pause?

Q-S2: Negate and Sublate topological effect formulas.
  Fold has precise formula Δβ₁=(c-1)+n_loop (proven L0).
  (a) Negate: "v on k independent cycles, sole passage point for each: β₁ decreases by k" — \
is "sole passage point" precisely definable? Give Δβ₁(Negate)=? formula.
  (b) Sublate: "may create new cycles" — under what conditions exactly? Give Δβ₁(Sublate)=? formula.
  (c) Are the O(|V|+|E|) complexity claims for negate/sublate correct?

Q-S3: AlphaGo analogy strictness.
  Spec §5: LLM=policy network, Δβ₁+T=value network, topological layer=game engine.
  (a) Training analogy break: AlphaGo trains via self-play; does this architecture train the LLM?
  (b) State space analogy break: AlphaGo has fixed 19×19 board; this system is append-only infinite.
  (c) Value function: T's range — is it bounded? Game-theoretic optimality?

Q-S4: Minimal experiment sufficiency.
  Spec §9: 8-10 vertices, 12-15 edges, 20-30 steps.
  (a) Can all 5 success criteria be met in 20-30 steps?
  (b) Initial graph design: should β₁≥1 be required?
  (c) Scale sufficiency: max 5 independent cycles — enough to demonstrate "third paradigm"?

For each question: complete reasoning chain, conclusion, epistemological level (L0/L1/L2/L3), \
boundary conditions, and flag any mathematical errors or overstrong claims in the spec.
"""

DOMAIN = "Algebraic Topology and Discrete Mathematics"


def main():
    print(f"Reading context from: {CONTEXT_FILE}")
    with open(CONTEXT_FILE, encoding="utf-8") as f:
        context = f.read()

    print(f"Context size: {len(context)} chars")
    print("Calling Gemini derive (temperature=0.1, thinking_budget=unlimited)...")
    print("This may take several minutes...")

    challenger = GeminiChallenger()
    result = challenger.derive(SUBJECT, context, DOMAIN)

    print(f"Model used: {result.model}")
    print(f"Response length: {len(result.response)} chars")

    # 写入结果文件
    timestamp = datetime.datetime.now().strftime("%Y%m%d-%H%M%S")
    header = f"""\
---
trigger: "v166-swarm genealogist: 拓扑计算架构规格数学审查"
target: "topological-computation-spec.md 四个核心问题 Q-S1~Q-S4"
mode: "derive"
model_used: "{result.model}"
timestamp: "{timestamp}"
---

"""
    with open(OUTPUT_FILE, "w", encoding="utf-8") as f:
        f.write(header)
        f.write(result.response)

    print(f"Result written to: {OUTPUT_FILE}")


if __name__ == "__main__":
    main()
