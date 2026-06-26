Reading additional input from stdin...
OpenAI Codex v0.125.0 (research preview)
--------
workdir: /Users/silencehan/Projects/NewChanlun
model: gpt-5.5
provider: openai
approval: never
sandbox: read-only
reasoning effort: xhigh
reasoning summaries: none
session id: 019f02a0-7f08-7ae1-9d1b-33d2e553ca2a
--------
user
# Codex Diagnose Context: Chanlun Foundation Recursion Abstraction Fidelity

## Task

You are asked to give a strict second-opinion judgment on the following code architecture question.
This is a heterogeneous review — your job is to find what a Claude-family reviewer would miss.

**Binary question (no hedge allowed):**
Is the current Foundation `Fstep : D n -> D (n+1)` abstraction faithfully welded to Chanlun's sequence recursion (≥3 distinct sub-moves → compose), OR is there a structural contradiction that cannot be resolved no matter how you choose `D n`?

Answer: **A (faithful weld exists)** or **B (true structural contradiction)**. You must choose one.

---

## The Foundation Abstraction (CompleteClassification.lean)

```lean
-- Foundation/CompleteClassification.lean
section LevelRecursion

variable {H : Type}
variable (D : Nat -> Type)
variable (F0 : H -> D 0)
variable (Fstep : (n : Nat) -> D n -> D (n + 1))

def recAt : (n : Nat) -> H -> D n
  | 0, h => F0 h
  | n + 1, h => Fstep n (recAt n h)

def RecSpec (h : H) : (n : Nat) -> D n -> Prop
  | 0, d => d = F0 h
  | n + 1, d =>
      Exists (fun prev : D n => RecSpec h n prev /\ d = Fstep n prev)

theorem recSpec_complete_unique (h : H) (n : Nat) :
    ExistsUnique (fun d : D n => RecSpec D F0 Fstep h n d)
```

Key signatures:
- `D : Nat -> Type` — the level-indexed type family (completely free, can be any type)
- `F0 : H -> D 0` — base: maps a "history" seed to a level-0 object
- `Fstep : (n : Nat) -> D n -> D (n + 1)` — SINGLE-ARGUMENT step: takes ONE `D n` object, produces ONE `D (n+1)` object
- `recSpec_complete_unique` — proves that the recursion output at each level is unique

The `Fstep` has arity 1 on `D n`. This is the subject of the question.

---

## Chanlun's True Recursion (Strict/Recursive.lean)

```lean
-- Strict/Recursive.lean
-- Can (f₂): ≥3 lower-level moves → compose into upper-level move
def canOp (n : Nat) (xs : {l : List (U n) // l.length ≥ 3}) : U (n+1) :=
  Move.compose xs.val (canG xs.val) (n+1)

-- Boundary: extract lower-level move sequence from upper-level move
def bdry (n : Nat) : U (n+1) -> List (U n)
  | Move.compose subs _ _ => subs
  | Move.segment _ _ _ => []
```

Chanlun's recursive law (Chanlun theory, lesson 17): An n+1 level trend is composed of **≥3 DISTINCT n-level trends**. The inputs are a sequence `{xs // xs.length ≥ 3}`, not a single object.

---

## The Instantiation Attempt (Foundation/ChanlunInstantiation.lean §1)

The current implementation in ChanlunInstantiation.lean §1:

```lean
-- D n := List Move (sequence of level-n moves — the key carrier choice)
abbrev ChanD (_ : Nat) : Type := List Move

-- composeStep: the Fstep instantiation
def composeStep (n : Nat) (xs : ChanD n) : ChanD (n + 1) :=
  if 3 ≤ xs.length then [Move.compose xs (chanCenters xs) (n + 1)] else []
```

The file's own comments diagnose the OLD degenerate implementation:

> ★载体修正（no-patch，de-degenerate 核心）：旧桩取 `ChanD n := Move`（单个走势），
> 被迫把 `Fstep n m := compose [m,m,m] …` 复制同一走势三份——违反 `MovesComposedFrom` 的
> `StrictlyIncreasing` 起点 + 中枢由**不同**子走势窗口派生，是退化桩。

The NEW implementation:
- `ChanD n := List Move` (a LIST of n-level moves, not a single move)
- `composeStep` takes the whole list `xs`, checks `xs.length ≥ 3`, and produces `[Move.compose xs ...]`
- This packs the "sequence" information INTO the `D n` type itself

The core claim in the file:

> 诊断：Foundation `Fstep : D n → D (n+1)` 的「单参数」指「一个 `D n` 类型的对象」，而
> `D n` 类型**完全自由**——可取**序列** `List Move`。缠论第 n+1 级走势由 ≥3 个**不同**的
> 第 n 级走势 compose（走势分解定理二），其载体是**下级走势序列**，不是单个走势。故忠实
> 载体 = `ChanD n := List Move`（第 n 级走势序列），`Fstep` 取整个序列、规范聚合为上级序列。
> **矛盾不存在**——是旧桩选错了载体（单走势 vs 序列）。

---

## The Critical Question

Is this a **faithful weld** (Answer A), or does the structural contradiction persist even with `D n := List Move` (Answer B)?

### What Answer A would look like:
`D n := List Move` genuinely captures "the sequence of n-level moves from which the next level is built." `Fstep` = `composeStep` faithfully represents Chanlun's "take ≥3 distinct lower moves, compose them into one upper move." The sequence information is not lost — it's just packaged as the input type. The `[m,m,m]` degeneracy is eliminated because `xs` is the REAL input sequence.

### What Answer B would look like:
Even with `D n := List Move`, there's a hidden problem. For example:
- `recAt n h` generates a unique `D n` value via the unique recursion path from `h`. But Chanlun's level-n state is NOT uniquely determined by the seed `h` — different subsets of time series data form different level-n move sequences. So `recAt` is modeling a different concept than Chanlun's recursion.
- Or: the `recSpec_complete_unique` theorem guarantees uniqueness of `D n` given `h`, but in Chanlun, a given seed (time series window) can form DIFFERENT valid level-n sequences depending on analysis context — so uniqueness is wrong for the domain.
- Or: `Fstep n xs` consumes ALL of `xs` as one batch into one upper-level move, but Chanlun's recursion produces MULTIPLE upper-level moves from a longer sequence (not just one) — so the level-(n+1) output should also be `List Move`, not `[single move]`. The `composeStep` only handles the case of one exact ≥3 block, but real Chanlun sequences have multiple non-overlapping ≥3 blocks at each step.

### Judgment criterion (from the original question):
The key is NOT whether it compiles. It's: **Can `Fstep`'s single-parameter arity express Chanlun's recursion without losing the information "≥3 distinct lower-level moves"?**

And the deeper question: **Is packing "lower sequence" into `D n` itself a faithful weld, or is it just moving the degeneracy to a different place?**

---

## What You Should Analyze

1. **Type-level arity**: `Fstep : D n -> D(n+1)` takes ONE `D n` and produces ONE `D(n+1)`. With `D n = List Move`, this is `List Move -> List Move`. Is this the right type for Chanlun's recursion?

2. **Uniqueness vs. Non-uniqueness**: `recSpec_complete_unique` proves that for any seed `h` and level `n`, there is EXACTLY ONE `D n` value satisfying the recursion. Does this fit Chanlun's recursion semantics? In Chanlun theory, given a time series, can there be more than one valid n-level move sequence?

3. **`composeStep` behavior**: 
   - Input: `xs : List Move` where `xs.length ≥ 3`
   - Output: `[Move.compose xs (chanCenters xs) (n+1)]` — a singleton list containing ONE composed move
   - If `xs.length < 3`: returns `[]` (empty — level terminates)
   - Real question: In Chanlun, does a level-n sequence `xs` of length, say, 7, produce ONE level-(n+1) move or potentially MULTIPLE level-(n+1) moves?

4. **The seed `H` and `F0`**: What is `H` in the Chanlun instantiation? The file says `F0 := id` and seed is the level-0 move sequence. But does this match how Chanlun's recursion actually starts?

5. **Bottom line**: Is there any structural property of Chanlun's recursion that `D n := List Move` + `Fstep n := composeStep n` CANNOT express, regardless of how you choose the types?

---

## Required Output Format

```
VERDICT: A or B

REASONING:
[Step-by-step type-level reasoning, not intuition]

IF A (faithful weld):
- Precise form of D n and Fstep that eliminates degeneracy
- Why uniqueness (recSpec_complete_unique) is still valid

IF B (structural contradiction):
- Precise statement of the irreconcilable point
- Which arity/cardinality of WHAT conflicts with WHAT
- Why no choice of D n can fix it
- What this means for "build-on Foundation"
```
codex
I’ll use the codebase-design skill here because the question is about whether an interface abstraction matches the domain recursion. I’m going to verify the local Lean definitions first, then give the binary judgment from the type shape rather than from the comments.
exec
/bin/zsh -lc "sed -n '1,220p' /Users/silencehan/.agents/skills/codebase-design/SKILL.md" in /Users/silencehan/Projects/NewChanlun
 succeeded in 0ms:
---
name: codebase-design
description: Shared vocabulary for designing deep modules. Use when the user wants to design or improve a module's interface, find deepening opportunities, decide where a seam goes, make code more testable or AI-navigable, or when another skill needs the deep-module vocabulary.
---

# Codebase Design

Design **deep modules**: a lot of behaviour behind a small interface, placed at a clean seam, testable through that interface. Use this language and these principles wherever code is being designed or restructured. The aim is leverage for callers, locality for maintainers, and testability for everyone.

## Glossary

Use these terms exactly — don't substitute "component," "service," "API," or "boundary." Consistent language is the whole point.

**Module** — anything with an interface and an implementation. Deliberately scale-agnostic: a function, class, package, or tier-spanning slice. _Avoid_: unit, component, service.

**Interface** — everything a caller must know to use the module correctly: the type signature, but also invariants, ordering constraints, error modes, required configuration, and performance characteristics. _Avoid_: API, signature (too narrow — they refer only to the type-level surface).

**Implementation** — what's inside a module, its body of code. Distinct from **Adapter**: a thing can be a small adapter with a large implementation (a Postgres repo) or a large adapter with a small implementation (an in-memory fake). Reach for "adapter" when the seam is the topic; "implementation" otherwise.

**Depth** — leverage at the interface: the amount of behaviour a caller (or test) can exercise per unit of interface they have to learn. A module is **deep** when a large amount of behaviour sits behind a small interface, **shallow** when the interface is nearly as complex as the implementation.

**Seam** _(Michael Feathers)_ — a place where you can alter behaviour without editing in that place; the *location* at which a module's interface lives. Where to put the seam is its own design decision, distinct from what goes behind it. _Avoid_: boundary (overloaded with DDD's bounded context).

**Adapter** — a concrete thing that satisfies an interface at a seam. Describes *role* (what slot it fills), not substance (what's inside).

**Leverage** — what callers get from depth: more capability per unit of interface they learn. One implementation pays back across N call sites and M tests.

**Locality** — what maintainers get from depth: change, bugs, knowledge, and verification concentrate in one place rather than spreading across callers. Fix once, fixed everywhere.

## Deep vs shallow

**Deep module** = small interface + lots of implementation:

```
┌─────────────────────┐
│   Small Interface   │  ← Few methods, simple params
├─────────────────────┤
│                     │
│  Deep Implementation│  ← Complex logic hidden
│                     │
└─────────────────────┘
```

**Shallow module** = large interface + little implementation (avoid):

```
┌─────────────────────────────────┐
│       Large Interface           │  ← Many methods, complex params
├─────────────────────────────────┤
│  Thin Implementation            │  ← Just passes through
└─────────────────────────────────┘
```

When designing an interface, ask:

- Can I reduce the number of methods?
- Can I simplify the parameters?
- Can I hide more complexity inside?

## Principles

- **Depth is a property of the interface, not the implementation.** A deep module can be internally composed of small, mockable, swappable parts — they just aren't part of the interface. A module can have **internal seams** (private to its implementation, used by its own tests) as well as the **external seam** at its interface.
- **The deletion test.** Imagine deleting the module. If complexity vanishes, it was a pass-through. If complexity reappears across N callers, it was earning its keep.
- **The interface is the test surface.** Callers and tests cross the same seam. If you want to test *past* the interface, the module is probably the wrong shape.
- **One adapter means a hypothetical seam. Two adapters means a real one.** Don't introduce a seam unless something actually varies across it.

## Designing for testability

Good interfaces make testing natural:

1. **Accept dependencies, don't create them.**

   ```typescript
   // Testable
   function processOrder(order, paymentGateway) {}

   // Hard to test
   function processOrder(order) {
     const gateway = new StripeGateway();
   }
   ```

2. **Return results, don't produce side effects.**

   ```typescript
   // Testable
   function calculateDiscount(cart): Discount {}

   // Hard to test
   function applyDiscount(cart): void {
     cart.total -= discount;
   }
   ```

3. **Small surface area.** Fewer methods = fewer tests needed. Fewer params = simpler test setup.

## Relationships

- A **Module** has exactly one **Interface** (the surface it presents to callers and tests).
- **Depth** is a property of a **Module**, measured against its **Interface**.
- A **Seam** is where a **Module**'s **Interface** lives.
- An **Adapter** sits at a **Seam** and satisfies the **Interface**.
- **Depth** produces **Leverage** for callers and **Locality** for maintainers.

## Rejected framings

- **Depth as ratio of implementation-lines to interface-lines** (Ousterhout): rewards padding the implementation. We use depth-as-leverage instead.
- **"Interface" as the TypeScript `interface` keyword or a class's public methods**: too narrow — interface here includes every fact a caller must know.
- **"Boundary"**: overloaded with DDD's bounded context. Say **seam** or **interface**.

## Going deeper

- **Deepening a cluster given its dependencies** — see [DEEPENING.md](DEEPENING.md): dependency categories, seam discipline, and replace-don't-layer testing.
- **Exploring alternative interfaces** — see [DESIGN-IT-TWICE.md](DESIGN-IT-TWICE.md): spin up parallel sub-agents to design the interface several radically different ways, then compare on depth, locality, and seam placement.

exec
/bin/zsh -lc "sed -n '1,240p' Foundation/CompleteClassification.lean" in /Users/silencehan/Projects/NewChanlun
 exited 1 in 0ms:
sed: Foundation/CompleteClassification.lean: No such file or directory

exec
/bin/zsh -lc "sed -n '1,260p' Strict/Recursive.lean" in /Users/silencehan/Projects/NewChanlun
 exited 1 in 0ms:
sed: Strict/Recursive.lean: No such file or directory

exec
/bin/zsh -lc "sed -n '1,260p' Foundation/ChanlunInstantiation.lean" in /Users/silencehan/Projects/NewChanlun
 exited 1 in 0ms:
sed: Foundation/ChanlunInstantiation.lean: No such file or directory

codex
The paths in the prompt are conceptual or relative to a subdirectory; they don’t exist from the repo root as written. I’m locating the actual files now so the judgment is grounded in the implementation, not just the excerpt.
exec
/bin/zsh -lc "rg --files | rg 'CompleteClassification|ChanlunInstantiation|Recursive\\.lean"'$|Foundation|Strict'"'" in /Users/silencehan/Projects/NewChanlun
 succeeded in 0ms:
formal/Strict/opentail-extra-from-cc-divergence.md
formal/Strict/Fugue.lean
formal/Strict/RiskProj.lean
formal/Strict/Causal.lean
formal/Strict/Recursive.lean
formal/Strict/Parse.lean
formal/Strict/OpenTail.lean
formal/Strict/Center.lean
formal/Strict/HybridStep.lean
formal/Strict/Decomp.lean
formal/Strict/Trend.lean
formal/Strict/BSP.lean
formal/Strict/Nest.lean
formal/Strict/Classification.lean
formal/Strict/Op.lean
formal/Strict/LevelState.lean
formal/Strict/ClassificationFamily.lean
formal/Strict/HybridAssembly.lean
formal/Strict/spiral-extra-from-cc-spiral.md
formal/Strict/StrategyFamily.lean
formal/Strict/Chain.lean
formal/Foundation/HybridStateMachine.lean
formal/Foundation/ChanlunInstantiation.lean
formal/Foundation/CompleteClassification.lean
topological-computation/signifier_net/corpora/wittgenstein/Wittgenstein_Remarks_on_the_Foundations_of_Mathematics.md
topological-computation/signifier_net/corpora/badiou/Badiou_Saint_Paul_Foundation_of_Universalism.md
topological-computation/signifier_net/corpora/kant/Kant_Metaphysical_Foundations_of_Natural_Science.md

exec
/bin/zsh -lc "find . -maxdepth 3 -type f -name '*.lean' | sed 's#"'^./##'"' | sort | head -200" in /Users/silencehan/Projects/NewChanlun
 succeeded in 108ms:
formal/Formal.lean
formal/Formal/BSPLabels.lean
formal/Formal/CenterTrichotomy.lean
formal/Formal/ConstitutiveLadder.lean
formal/Formal/DivergenceNesting.lean
formal/Formal/EvalSoundness.lean
formal/Formal/OperationalSemantics.lean
formal/Formal/RStarNonSpecial.lean
formal/Formal/RecursiveConstruction.lean
formal/Formal/TrendTrichotomy.lean
formal/Foundation/ChanlunInstantiation.lean
formal/Foundation/CompleteClassification.lean
formal/Foundation/HybridStateMachine.lean
formal/Phase2/Claim10_SegmentV1.lean
formal/Phase2/Claim5_ConstitutiveLadder.lean
formal/Phase2/Claim6_OperationalSemantics.lean
formal/Phase2/Claim7_DivergenceNesting.lean
formal/Phase2/Claim9_CenterPosition.lean
formal/Strict/BSP.lean
formal/Strict/Causal.lean
formal/Strict/Center.lean
formal/Strict/Chain.lean
formal/Strict/Classification.lean
formal/Strict/ClassificationFamily.lean
formal/Strict/Decomp.lean
formal/Strict/Fugue.lean
formal/Strict/HybridAssembly.lean
formal/Strict/HybridStep.lean
formal/Strict/LevelState.lean
formal/Strict/Nest.lean
formal/Strict/Op.lean
formal/Strict/OpenTail.lean
formal/Strict/Parse.lean
formal/Strict/Recursive.lean
formal/Strict/RiskProj.lean
formal/Strict/StrategyFamily.lean
formal/Strict/Trend.lean
formal/Tlayers/Accounting.lean
formal/Tlayers/Dynamics.lean
formal/Tlayers/Operational.lean
formal/Tlayers/Payoff.lean
formal/Tlayers/Signal.lean
formal/Tlayers/Spiral.lean

codex
I found the actual files under `formal/`. Next I’m reading the relevant definitions around `LevelRecursion`, `canOp`, `bdry`, and the Chanlun adapter so I can separate the generic foundation type from the current concrete adapter.
exec
/bin/zsh -lc "sed -n '1,260p' formal/Foundation/CompleteClassification.lean" in /Users/silencehan/Projects/NewChanlun
 succeeded in 0ms:
/-!
Lean 4 formalization for a strict complete-classification strategy skeleton.

This file proves the parts that are purely mathematical:

1. A priority decision classification is a genuine partition.
2. A recursive level construction by total functions is unique at every level.
3. Per-voice complete classifications compose into a unique global
   multi-voice classification.
4. If every strategy stage is total and single-valued as a relation, then the
   whole Rec -> Class -> Voice -> Risk -> Exec strategy has a unique order.

The Chanlun-specific predicates and construction functions are intentionally
parameters. The proof states exactly which obligations the implementation must
discharge.
-/

namespace NewChanlun

universe u

def ExistsUnique {A : Type u} (p : A -> Prop) : Prop :=
  Exists (fun x => p x /\ forall y, p y -> y = x)

/-! ## Priority classification -/

inductive PriorityClass where
  | emergencyExit
  | ancestorClosed
  | localStop
  | reverseNest
  | sameNest
  | hold
deriving DecidableEq, Repr

section Priority

variable {X : Type}
variable (P1 P2 P3 P4 P5 : X -> Prop)

def IsPriorityClass (x : X) : PriorityClass -> Prop
  | PriorityClass.emergencyExit => P1 x
  | PriorityClass.ancestorClosed => Not (P1 x) /\ P2 x
  | PriorityClass.localStop => Not (P1 x) /\ Not (P2 x) /\ P3 x
  | PriorityClass.reverseNest =>
      Not (P1 x) /\ Not (P2 x) /\ Not (P3 x) /\ P4 x
  | PriorityClass.sameNest =>
      Not (P1 x) /\ Not (P2 x) /\ Not (P3 x) /\ Not (P4 x) /\ P5 x
  | PriorityClass.hold =>
      Not (P1 x) /\ Not (P2 x) /\ Not (P3 x) /\ Not (P4 x) /\ Not (P5 x)

def choosePriorityClass
    [DecidablePred P1] [DecidablePred P2] [DecidablePred P3]
    [DecidablePred P4] [DecidablePred P5] (x : X) : PriorityClass :=
  if P1 x then
    PriorityClass.emergencyExit
  else if P2 x then
    PriorityClass.ancestorClosed
  else if P3 x then
    PriorityClass.localStop
  else if P4 x then
    PriorityClass.reverseNest
  else if P5 x then
    PriorityClass.sameNest
  else
    PriorityClass.hold

theorem priority_class_iff_chosen
    [DecidablePred P1] [DecidablePred P2] [DecidablePred P3]
    [DecidablePred P4] [DecidablePred P5] (x : X) (c : PriorityClass) :
    IsPriorityClass P1 P2 P3 P4 P5 x c <->
      c = choosePriorityClass P1 P2 P3 P4 P5 x := by
  by_cases h1 : P1 x <;>
  by_cases h2 : P2 x <;>
  by_cases h3 : P3 x <;>
  by_cases h4 : P4 x <;>
  by_cases h5 : P5 x <;>
  cases c <;>
  simp [IsPriorityClass, choosePriorityClass, h1, h2, h3, h4, h5]

theorem priority_class_complete_unique
    [DecidablePred P1] [DecidablePred P2] [DecidablePred P3]
    [DecidablePred P4] [DecidablePred P5] (x : X) :
    ExistsUnique (fun c => IsPriorityClass P1 P2 P3 P4 P5 x c) := by
  refine ⟨choosePriorityClass P1 P2 P3 P4 P5 x, ?_, ?_⟩
  · exact
      (priority_class_iff_chosen P1 P2 P3 P4 P5 x
        (choosePriorityClass P1 P2 P3 P4 P5 x)).2 rfl
  · intro c hc
    exact (priority_class_iff_chosen P1 P2 P3 P4 P5 x c).1 hc

theorem priority_classes_disjoint
    [DecidablePred P1] [DecidablePred P2] [DecidablePred P3]
    [DecidablePred P4] [DecidablePred P5] {x : X}
    {c d : PriorityClass}
    (hc : IsPriorityClass P1 P2 P3 P4 P5 x c)
    (hd : IsPriorityClass P1 P2 P3 P4 P5 x d) :
    c = d := by
  have hc' := (priority_class_iff_chosen P1 P2 P3 P4 P5 x c).1 hc
  have hd' := (priority_class_iff_chosen P1 P2 P3 P4 P5 x d).1 hd
  exact hc'.trans hd'.symm

end Priority

/-! ## Recursive level construction -/

section LevelRecursion

variable {H : Type}
variable (D : Nat -> Type)
variable (F0 : H -> D 0)
variable (Fstep : (n : Nat) -> D n -> D (n + 1))

def recAt : (n : Nat) -> H -> D n
  | 0, h => F0 h
  | n + 1, h => Fstep n (recAt n h)

def RecSpec (h : H) : (n : Nat) -> D n -> Prop
  | 0, d => d = F0 h
  | n + 1, d =>
      Exists (fun prev : D n => RecSpec h n prev /\ d = Fstep n prev)

theorem recAt_spec (h : H) (n : Nat) :
    RecSpec D F0 Fstep h n (recAt D F0 Fstep n h) := by
  induction n with
  | zero =>
      simp [RecSpec, recAt]
  | succ n ih =>
      exact ⟨recAt D F0 Fstep n h, ih, rfl⟩

theorem recSpec_complete_unique (h : H) (n : Nat) :
    ExistsUnique (fun d : D n => RecSpec D F0 Fstep h n d) := by
  induction n with
  | zero =>
      refine ⟨F0 h, ?_, ?_⟩
      · rfl
      · intro y hy
        exact hy
  | succ n ih =>
      rcases ih with ⟨prev, hprev, hprevUnique⟩
      refine ⟨Fstep n prev, ?_, ?_⟩
      · exact ⟨prev, hprev, rfl⟩
      · intro y hy
        rcases hy with ⟨prev', hprev', hyEq⟩
        have hprevEq : prev' = prev := hprevUnique prev' hprev'
        subst prev'
        exact hyEq

theorem recAt_causal
    (ObsEq : H -> H -> Prop)
    (hF0 : forall {h h' : H}, ObsEq h h' -> F0 h = F0 h')
    {h h' : H} (hh : ObsEq h h') (n : Nat) :
    recAt D F0 Fstep n h = recAt D F0 Fstep n h' := by
  induction n with
  | zero =>
      simpa [recAt] using hF0 hh
  | succ n ih =>
      simp [recAt, ih]

end LevelRecursion

/-! ## Global multi-voice classification -/

section GlobalVoice

variable {X Voice LocalClass : Type}
variable (Local : Voice -> X -> LocalClass -> Prop)
variable (hLocal : forall (v : Voice) (x : X),
  ExistsUnique (fun c : LocalClass => Local v x c))

def GlobalClassSpec (x : X) (g : Voice -> LocalClass) : Prop :=
  forall v : Voice, Local v x (g v)

noncomputable def globalClass (x : X) : Voice -> LocalClass :=
  fun v => Classical.choose (hLocal v x)

theorem globalClass_spec (x : X) (v : Voice) :
    Local v x ((globalClass Local hLocal x) v) := by
  unfold globalClass
  exact (Classical.choose_spec (hLocal v x)).1

theorem global_class_complete_unique
    (hLocal : forall (v : Voice) (x : X),
      ExistsUnique (fun c : LocalClass => Local v x c))
    (x : X) :
    ExistsUnique (fun g : Voice -> LocalClass => GlobalClassSpec Local x g) := by
  classical
  refine ⟨globalClass Local hLocal x, ?_, ?_⟩
  · intro v
    exact globalClass_spec Local hLocal x v
  · intro g hg
    funext v
    rcases hLocal v x with ⟨c, hc, hUnique⟩
    have hgEq : g v = c := hUnique (g v) (hg v)
    have hChosenEq : (globalClass Local hLocal x) v = c :=
      hUnique ((globalClass Local hLocal x) v)
        (globalClass_spec Local hLocal x v)
    exact hgEq.trans hChosenEq.symm

end GlobalVoice

/-! ## Relational strategy pipeline -/

section Strategy

variable {X D C Q QStar O : Type}
variable (RecR : X -> D -> Prop)
variable (ClassR : X -> D -> C -> Prop)
variable (VoiceR : X -> C -> Q -> Prop)
variable (RiskR : X -> Q -> QStar -> Prop)
variable (ExecR : X -> QStar -> O -> Prop)

def StrategySpec (x : X) (o : O) : Prop :=
  Exists (fun d : D =>
  Exists (fun c : C =>
  Exists (fun q : Q =>
  Exists (fun qstar : QStar =>
    RecR x d /\
    ClassR x d c /\
    VoiceR x c q /\
    RiskR x q qstar /\
    ExecR x qstar o))))

theorem strategy_spec_total_unique
    (hRec : forall x : X, ExistsUnique (fun d : D => RecR x d))
    (hClass : forall (x : X) (d : D), RecR x d ->
      ExistsUnique (fun c : C => ClassR x d c))
    (hVoice : forall (x : X) (c : C), ClassR x
      (Classical.choose (hRec x)) c ->
      ExistsUnique (fun q : Q => VoiceR x c q))
    (hRisk : forall (x : X) (q : Q), VoiceR x
      (Classical.choose
        (hClass x (Classical.choose (hRec x))
          (Classical.choose_spec (hRec x)).1)) q ->
      ExistsUnique (fun qstar : QStar => RiskR x q qstar))
    (hExec : forall (x : X) (qstar : QStar), RiskR x
      (Classical.choose
        (hVoice x
          (Classical.choose
            (hClass x (Classical.choose (hRec x))
              (Classical.choose_spec (hRec x)).1))
          (Classical.choose_spec
            (hClass x (Classical.choose (hRec x))
              (Classical.choose_spec (hRec x)).1)).1)) qstar ->
      ExistsUnique (fun o : O => ExecR x qstar o)) :
    forall x : X, ExistsUnique
      (fun o : O => StrategySpec RecR ClassR VoiceR RiskR ExecR x o) := by
  classical
  intro x
  let d : D := Classical.choose (hRec x)
  have hd : RecR x d := (Classical.choose_spec (hRec x)).1
  let c : C := Classical.choose (hClass x d hd)
  have hc : ClassR x d c := (Classical.choose_spec (hClass x d hd)).1
  let q : Q := Classical.choose (hVoice x c hc)
  have hq : VoiceR x c q := (Classical.choose_spec (hVoice x c hc)).1
  let qstar : QStar := Classical.choose (hRisk x q hq)
  have hqstar : RiskR x q qstar := (Classical.choose_spec (hRisk x q hq)).1
  let o : O := Classical.choose (hExec x qstar hqstar)
  have ho : ExecR x qstar o := (Classical.choose_spec (hExec x qstar hqstar)).1
  refine ⟨o, ?_, ?_⟩

exec
/bin/zsh -lc "sed -n '1,280p' formal/Strict/Recursive.lean" in /Users/silencehan/Projects/NewChanlun
 succeeded in 0ms:
/-
  Strict/Recursive.lean — 严格完全分类·第2部分 Eval 健全 + 第4部分 RecursiveKernel Can
  （task #61, RTAS 蜂群 T-recursive 工位；填补 codex#2 gapmatrix 判定的"完全缺失"两项）

  ── 工位定位 ────────────────────────────────────────────────────────────────
  本文件 import 标准内核库 `Classification`（Strict.* 结构定义），对**走势递归**
  （`Formal.RecursiveConstruction.Move`）实例化两个标准结构，从而把 codex#2 差距矩阵中
  判定为「C 完全缺失」的两项补成「带证明的实例」：

  - 标准第 2 部分（DecompositionSystem）：codex 判「没有 `Eval : Tₙ → Xₙ`，没有
    `Eval (Dₙ h)=h`」。本文件建 `moveDecomp` 实例，证 **重构健全** `Eval(D(d h))=d h`
    （分解能重构原走势）+ **分解唯一** `unique_mod_rel`（规范分解是唯一合法分解）。
  - 标准第 4 部分（RecursiveKernel）：codex 判「没有 `Can` 的存在/唯一边界/规范化幂等定理；
    没有 f₁/f₂ 分离的防循环命题」。本文件建 `moveKernel` 实例，证 `Can` 存在（全函数 witness）
    + **边界健全** `can_sound: Boundary(Can xs)=xs.val` + **边界唯一/幂等** `can_unique`
    + m≥3（子类型编码 + 显式读出）+ **f₁≠f₂ anti-circularity**（base 与 Can 构造子分离，
    防中枢↔走势循环定义）。

  ── 认识论等级（formalization-validity-domain 强制标注）─────────────────────────
  全部 **L0**（纯定义/结构归纳，不依赖数据）。`lake env lean` 通过 = 这些 round-trip /
  单射 / 分离命题在定义层成立，**不是**任何实证有效域断言。`eval_sound`/`can_sound` 是
  纯结构归纳，信息增量 = 同义反复（L0），不冒充 L1+。

  ── 载体选择（no-workaround：直面 indexed-inductive 约束，非补丁）──────────────
  分级宇宙 `U : Nat → Type` 取 `U n := Move`——级别由 `Move.level` 字段区分，**不是**
  type index。理由（严格，非妥协）：Lean kernel 拒绝「indexed inductive family + nested
  `List`」（`inductive G : Nat → Type | comp : List (G n) → G (n+1)` 报 "nested inductive
  parameters cannot contain local variables"）。这是 kernel 的硬约束，不可 workaround。
  Move 已是非 indexed inductive（segment/compose），级别作字段——与 `RStarNonSpecial.TowerLevel`
  「顶层性是字段不是 type index」的设计哲学同构（见 RStarNonSpecial.lean:42-57）。
  RecursiveKernel 标准结构 `RecursiveKernel (U : Nat → Type)` 接受**任意** `U`，`U n := Move`
  合法。`Can n` 输出 `U (n+1) = Move`、`Boundary n` 读 `U (n+1) = Move`——级别契约由 `Can`
  写入 `Move.compose ... (n+1)` 的 level 字段携带。

  ── 分工边界（no-patch-mentality：诚实声明本文件不做什么）────────────────────
  - `RelT := Eq`（非膨胀）：本分解系统 `evalT`/`decompM` 是**互逆双射**（两向 round-trip
    都证），故合法分解唯一、无多义性，∼ₙ 退化为 Eq 是**诚实的**（系统本无多义性，不是用
    Eq 假装覆盖了结合律等价）。结合律/多义性 ∼ₙ 商集（`|𝒟ₙ/∼ₙ|=1` 但 `|𝒟ₙ|>1`）是
    task #62/#63（T-decomp-uniq / C14 螺旋 Setoid+Quot.lift）的职责，不在本文件。
  - `Boundary`/`Can` 只契约**下级走势序列**（subs）的可还原性 + 单射；`Can` 内部 centers
    由 `canG : List Move → List Center`（subs 的函数）派生——centers 与 subs 的**生成忠实性**
    （`CentersDerivedFrom` witness+顺序 soundness）是 `RecursiveConstruction.WellFormed` 层
    的契约（见 RecursiveConstruction.lean:144-167），不在 RecursiveKernel 边界算子层重复。
    本文件不声明 `Can` 验证了 centers 派生忠实性。

  谱系：598（真完全分类元判据）→ 603（递归 vs 轴范式）→ 615（Layer1 ⊊ Layer2）。
        本文件 = 615 概念分离在递归核与分解系统两处的 Layer2 落实。
  禁 sorry/admit/axiom。
-/

import Strict.Classification
import Formal.RecursiveConstruction

namespace Strict.Recursive

open Strict
open Formal.TrendTrichotomy (Direction)
open Formal.CenterTrichotomy (Center)
open Formal.RecursiveConstruction

/-! ## ① 标准第 2 部分：DecompositionSystem — Eval 健全（重构）+ 分解唯一

  对象层 `X := Move`（第 n 级完整走势结构状态）；历史 `H := Move`，取值映射 `d := id`
  （历史就是走势数据本身，恒等取值）。分解 `T := DecompTree` 镜像 `Move` 的递归结构。
  `Eval : T → X` 把结构树求值回走势，`D : X → T` 把走势分解为结构树。
-/

/--
  走势的结构分解树（镜像 `Move` 的递归形态）。
  - `leaf`：镜像 `Move.segment`（归纳基底，线段）。
  - `branch`：镜像 `Move.compose`（递归分支，子树序列 + 中枢 + 级别）。

  这是分解 `Dₙ(h)` 的具体载体——保留重构原走势所需的**全部**结构信息
  （子走势树 + 中枢 + 级别），故 `Eval`（求值）能还原原走势（健全）。
-/
inductive DecompTree where
  | leaf (dir : Direction) (lo hi : Int)
  | branch (subtrees : List DecompTree) (centers : List Center) (level : Nat)

/-! 分解树求值回走势（递归求值子树序列）。`Eval : Tₙ → Xₙ`。 -/
mutual
  def evalT : DecompTree → Move
    | DecompTree.leaf d lo hi => Move.segment d lo hi
    | DecompTree.branch ts cs lv => Move.compose (evalList ts) cs lv
  def evalList : List DecompTree → List Move
    | [] => []
    | t :: ts => evalT t :: evalList ts
end

/-! 走势分解为结构树（递归分解子走势序列）。`D : Xₙ → Tₙ`。 -/
mutual
  def decompM : Move → DecompTree
    | Move.segment d lo hi => DecompTree.leaf d lo hi
    | Move.compose subs cs lv => DecompTree.branch (decompList subs) cs lv
  def decompList : List Move → List DecompTree
    | [] => []
    | m :: ms => decompM m :: decompList ms
end

/-!
  ★重构健全 `Eval ∘ D = id`（标准第 2 部分核心，codex 判「完全缺失」项）：
  分解再求值还原原走势。结构归纳（与 `decompList` 互递归）。
-/
mutual
  theorem eval_decomp : ∀ m : Move, evalT (decompM m) = m
    | Move.segment _ _ _ => rfl
    | Move.compose subs _ _ => by
        simp only [decompM, evalT]; rw [eval_decomp_list subs]
  theorem eval_decomp_list : ∀ ms : List Move, evalList (decompList ms) = ms
    | [] => rfl
    | m :: ms => by
        simp only [decompList, evalList]; rw [eval_decomp m, eval_decomp_list ms]
end

/-!
  ★反向 round-trip `D ∘ Eval = id`（分解唯一性的根据）：求值再分解还原原树。
  这证明 `evalT`/`decompM` 是**互逆双射**——合法分解唯一、无多义性，故 ∼ₙ 退化为 Eq
  是诚实的（非声明膨胀）。结构归纳（与 `decompList` 互递归）。
-/
mutual
  theorem decomp_eval : ∀ t : DecompTree, decompM (evalT t) = t
    | DecompTree.leaf _ _ _ => rfl
    | DecompTree.branch ts _ _ => by
        simp only [evalT, decompM]; rw [decomp_eval_list ts]
  theorem decomp_eval_list : ∀ ts : List DecompTree, decompList (evalList ts) = ts
    | [] => rfl
    | t :: ts => by
        simp only [evalList, decompList]; rw [decomp_eval t, decomp_eval_list ts]
end

/--
  ★走势分解系统实例（标准 `DecompositionSystem`，第 2 部分完整实例化）。

  - `d := id`：历史→走势的恒等取值。
  - `D := decompM` / `Eval := evalT`：分解 / 求值。
  - `ValidDecomp x t := (evalT t = x)`：合法分解 ⟺ 求值回原走势（语义合法性）。
  - `RelT := Eq`：∼ₙ = 字面相等（本系统无多义性，见文件头分工边界）。
  - `eval_sound`：重构健全（`eval_decomp`）。
  - `D_valid`：规范分解合法（即 `evalT (decompM (d h)) = d h`，仍是 `eval_decomp`）。
  - `unique_mod_rel`：任何合法分解都 = 规范分解（由 `decomp_eval` 反向 round-trip：
    `evalT t = h ⟹ t = decompM (evalT t) = decompM h`）。
-/
def moveDecomp : DecompositionSystem Move Move DecompTree where
  d := id
  D := decompM
  Eval := evalT
  ValidDecomp := fun x t => evalT t = x
  RelT := Eq
  rel_equiv := ⟨fun _ => rfl, Eq.symm, Eq.trans⟩
  eval_sound := fun h => eval_decomp h
  D_valid := fun h => eval_decomp h
  unique_mod_rel := fun h t hv => by
    show t = decompM h
    simp only [id_eq] at hv
    rw [← hv, decomp_eval]

/-! ### ① 两层定理（Layer1 语法 round-trip → Layer2 系统级健全/唯一） -/

/--
  ★Layer1（语法层）：分解树与走势互逆——`evalT`/`decompM` 双射。
  这是「语法生成无遗漏」的递归版：每棵分解树恰对应一个走势，每个走势恰对应一棵规范分解树。
-/
theorem L1_decomp_bijective :
    (∀ m : Move, evalT (decompM m) = m) ∧ (∀ t : DecompTree, decompM (evalT t) = t) :=
  ⟨eval_decomp, decomp_eval⟩

/--
  ★Layer2（语义/系统层）：分解系统满足重构健全 ∧ 分解唯一（∼ₙ 下 `|𝒟ₙ(h)/∼ₙ|=1`）。
  这是标准第 2 部分的系统级陈述——从 `moveDecomp` 实例的证明义务字段读出，
  确认 Eval 健全（codex「完全缺失」项）已被实例化为带证明的命题。
-/
theorem L2_decomp_sound_and_unique :
    (∀ h, moveDecomp.Eval (moveDecomp.D (moveDecomp.d h)) = moveDecomp.d h)
    ∧ (∀ h t, moveDecomp.ValidDecomp (moveDecomp.d h) t
        → moveDecomp.RelT t (moveDecomp.D (moveDecomp.d h))) :=
  ⟨moveDecomp.eval_sound, moveDecomp.unique_mod_rel⟩

/-! ## ② 标准第 4 部分：RecursiveKernel — Can 存在/边界唯一/幂等 + m≥3 + f₁≠f₂

  分级宇宙 `U n := Move`（级别作字段，见文件头载体选择）。
  - `base`（f₁，起始构造）：`Atom → U 0`，segment 嵌入第 0 级（线段层）。
  - `Can n`（f₂，逐级 compose）：`{xs // len ≥ 3} → U (n+1)`，≥3 段装箱为第 n+1 级走势。
  - `Boundary n`：`U (n+1) → List (U n)`，拆箱取回构成的下级走势序列。
-/

/-- 分级宇宙：第 n 级对象类型（级别由 `Move.level` 字段区分，非 type index）。 -/
abbrev U (_ : Nat) := Move

/--
  ★`Can` 内部中枢派生函数（subs 的函数，保证 `Can` 是 subs 的函数 → 边界单射）。
  此处取恒定空——`Boundary` 不读 centers，centers 的**生成忠实性**由
  `RecursiveConstruction.CentersDerivedFrom`（WellFormed 层）契约，不在边界算子层重复
  （见文件头分工边界，no-patch-mentality 诚实声明）。
-/
def canG (_ : List Move) : List Center := []

/-- f₁：原始构件（segment 三元组）嵌入第 0 级。`base : Atom → U 0`。 -/
def baseOp : Direction × Int × Int → U 0
  | (d, lo, hi) => Move.segment d lo hi

/--
  f₂：≥3 段第 n 级走势规范化装箱为第 n+1 级走势（`Can n`）。
  级别契约 `n+1` 写入 `Move.compose` 的 level 字段；≥3 约束由子类型 `{xs // len ≥ 3}` 在
  类型层强制（标准 m≥3 的结构化落实）。
-/
def canOp (n : Nat) (xs : {l : List (U n) // l.length ≥ 3}) : U (n+1) :=
  Move.compose xs.val (canG xs.val) (n+1)

/-- 边界算子：取回构成第 n+1 级走势的下级走势序列（拆箱）。segment 无边界（空）。 -/
def bdry (n : Nat) : U (n+1) → List (U n)
  | Move.compose subs _ _ => subs
  | Move.segment _ _ _ => []

/--
  ★走势递归核实例（标准 `RecursiveKernel`，第 4 部分完整实例化）。

  - `can_sound`：边界健全 `Boundary(Can xs) = xs.val`（装箱后取边界还原原序列，rfl）。
  - `can_unique`：边界唯一/幂等 `Can xs = Can ys ⟹ xs.val = ys.val`
    （`Can` 单射于边界——`canOp` 是 `Move.compose` 应用，构造子单射 + injection 取第一分量）。
-/
def moveKernel : RecursiveKernel U where
  Atom := Direction × Int × Int
  base := baseOp
  Can := canOp
  Boundary := bdry
  can_sound := fun _ _ => rfl
  can_unique := fun _ _ _ h => by
    unfold canOp at h
    exact (Move.compose.injEq _ _ _ _ _ _).mp h |>.1

/-! ### ② Can 存在 + m≥3 显式读出 + 两层定理 -/

/--
  ★Can 存在（全函数 witness，codex「没有 Can 的存在」项）：对任意 ≥3 段下级序列，
  `Can n` 给出一个第 n+1 级对象。`Can` 是 Lean 全函数即编码存在性——此命题显式见证之
  （`canOp n xs` 是该对象的具体构造）。
-/
theorem can_exists (n : Nat) (xs : {l : List (U n) // l.length ≥ 3}) :
    ∃ u : U (n+1), moveKernel.Can n xs = u :=
  ⟨canOp n xs, rfl⟩

/--
  ★m≥3 结构化落实（标准「至少由三段以上次级别构成」）：`Can` 输入的下级序列长度 ≥ 3，
  且其边界（= 该下级序列）长度 ≥ 3。≥3 由子类型在类型层强制，此处显式读出。
-/
theorem can_boundary_length_ge_three (n : Nat) (xs : {l : List (U n) // l.length ≥ 3}) :
    (moveKernel.Boundary n (moveKernel.Can n xs)).length ≥ 3 := by
  rw [moveKernel.can_sound n xs]
  exact xs.property

/--
  ★Layer1（语法层）：`Can` 边界健全 ∧ 边界单射——`Can` 是「装箱/拆箱」互逆于边界。
  这是「语法生成无遗漏 + 语法相等下构造子互斥」的递归核版（codex#1 命名约束 Layer1）。
-/
theorem K_L1_boundary_iso :
    (∀ n xs, moveKernel.Boundary n (moveKernel.Can n xs)
        = (xs : {l : List (U n) // l.length ≥ 3}).val)
    ∧ (∀ n xs ys, moveKernel.Can n xs = moveKernel.Can n ys
        → (xs : {l : List (U n) // l.length ≥ 3}).val = ys.val) :=
  ⟨moveKernel.can_sound, moveKernel.can_unique⟩

/--
  ★Layer2（系统层）：递归核满足存在 ∧ 边界唯一 ∧ m≥3——标准第 4 部分系统级陈述。
  确认 `Can` 的存在/唯一边界/规范化（codex「完全缺失」项）已实例化为带证明命题。
-/
theorem K_L2_kernel_complete :
    (∀ n xs, ∃ u : U (n+1), moveKernel.Can n xs = u)
    ∧ (∀ n xs ys, moveKernel.Can n xs = moveKernel.Can n ys
        → (xs : {l : List (U n) // l.length ≥ 3}).val = ys.val)
    ∧ (∀ n xs, (moveKernel.Boundary n (moveKernel.Can n xs)).length ≥ 3) :=
  ⟨can_exists, moveKernel.can_unique, can_boundary_length_ge_three⟩

/-! ### ② f₁ ≠ f₂ 分离（anti-circularity：防中枢↔走势循环定义） -/

/--
  ★f₁ ≠ f₂ 构造子分离（codex「没有 f₁/f₂ 分离的防循环命题」项）：

  `base`（f₁，起始 segment 构造）的产物与 `Can`（f₂，逐级 compose 构造）的产物
  **构造子不同**（segment ≠ compose），故 HEq 不可能成立。这是 anti-circularity 的核心：
  走势（compose 产物）**不能**等同于线段基底（segment 产物）——高级别对象必须经 `Can`

exec
/bin/zsh -lc "sed -n '1,320p' formal/Foundation/ChanlunInstantiation.lean" in /Users/silencehan/Projects/NewChanlun
 succeeded in 0ms:
/-
  Foundation/ChanlunInstantiation.lean — 用缠论具体结构忠实实例化 Foundation 抽象参数
  （task #87, cc-foundation-inst 工位；编排者「从一开始的完全分类的严格推导出发」纲领）

  ── 工位定位 ────────────────────────────────────────────────────────────────
  `Foundation/CompleteClassification.lean`（namespace NewChanlun）是 GPT「推导完全分类」
  的严格形式化地基，把完全分类的各部件**抽象参数化**：
    - LevelRecursion：`D : Nat → Type` / `F0 : H → D 0` / `Fstep : (n) → D n → D (n+1)`，
      证 `recSpec_complete_unique`（递归级唯一）+ `recAt_causal`（因果，ObsEq 下不变）。
    - Priority：`P1..P5 : X → Prop`，证 `priority_class_complete_unique`（6 类互斥穷尽）。
    - GlobalVoice：`Local : Voice → X → LocalClass → Prop`，证 `global_class_complete_unique`
      （多声部分类存在唯一）。
    - Strategy：`RecR/ClassR/VoiceR/RiskR/ExecR` 五段关系，证 `strategy_spec_total_unique`
      （π 流水线存在唯一）。

  本文件**不**重证这些抽象定理（它们已在 Foundation 证毕）。本文件做的是**实例化**：
  用我们已证的缠论具体 Lean 结构（Strict.Recursive / Strict.Trend / Strict.Op）填入抽象
  参数，证明具体缠论形式化**满足** Foundation 的抽象义务——即把 Foundation 的存在唯一定理
  **应用于**缠论具体类型，得到缠论层的存在唯一结论。

  完全分类是**缠论公理给定的**（走势终完美 = X/∼≅P 双射存在，编排者裁定「忠实编码，禁评级/
  禁证 no-go」）。本文件忠实编码该给定分类的**具体实例**，不把完备性当要证/评级的定理。

  ── 四个实例化（每个 Foundation 抽象义务 ← 缠论具体结构）─────────────────────
  1. LevelRecursion ← 缠论 走势递归（`Strict.Recursive`/`RStarNonSpecial` 的逐级 compose）：
       `D n := List Move`（第 n 级**走势序列**——载体是序列，非单个走势）；
       `F0 := id`（第 0 级 = 种子走势序列本身，线段/起始层）；
       `Fstep n := composeStep n`（把第 n 级 ≥3 段走势序列聚合为第 n+1 级走势序列，
       忠实于 `RecursiveConstruction.MovesComposedFrom` 关系——非复制、用真实下级序列）。
     得 `recSpec_complete_unique`（缠论走势递归级唯一）+ `recAt_causal`（同种子同级序列相等）。
     ★忠实性根据：`composeStep` 的聚合窗口是 `UpperMoveSound`/`MovesComposedFrom` 的真实窗口
       结构 witness（`composeStep_window_sound`，见 §1 证明）——boundary 层窗口忠实，centers
       生成义务由 WellFormed 层契约（Recursive.lean 分工边界），本层不冒充（声明膨胀禁止）。
  2. Priority ← 缠论动作优先级（`Strict.Op.StrictState` 的持仓×信号决定动作）：
       X := StrictState；P1..P5 用缠论操作谓词刻画 6 类优先动作。
     得 `priority_class_complete_unique`（缠论动作优先级 6 类互斥穷尽）。
  3. GlobalVoice ← 缠论**多级别**走势分类器（`Strict.Trend` 真裁决 + 级别索引走势族）：
       Voice := Level（声部 = 级别）；对象 X := `ChanVoice → WalkX`（**每级一个走势对象**，
       多级别共振的忠实载体）；LocalClass := TrendKind；`Local v x c := IsKind c (x v)`
       （第 v 声部读**它自己的**走势对象 `x v`——不同级别读不同走势，真多声部）。
     由 `Strict.Trend.trend_classifies` 对每个 `x v` 导出 `hLocal : ExistsUnique`，
     得 `global_class_complete_unique`（缠论多级别走势分类存在唯一）。
  4. Strategy ← 缠论分类→应对流水线（`Strict.Trend` 分类 + `Strict.Op` 策略 + `Strict.RiskProj`）：
       五段关系用缠论分类器/策略的函数图（graph）刻画，各段 `ExistsUnique` 由「函数图存在
       唯一」给出，得 `strategy_spec_total_unique`（缠论 Rec→Class→Voice→Risk→Exec 存在唯一）。
       ★Risk 段类型层为恒等（StrictState 类型层不变），**数量维度投影**由 `Strict.RiskProj`
       的 `riskproj_exists_unique` 在 Θ_risk 网格层承载——边界由 `risk_quantity_out_of_typelayer`
       定理证出（数量 = L3 运行时，诚实裁定不入 L0 类型层流水线，非恒等桩糊过）。

  ── 认识论等级（formalization-validity-domain 强制标注）─────────────────────────
  全部 **L0**（纯定义/结构归纳，把已证 L0 结构填入已证 L0 抽象定理，不依赖数据）。
  实例化的信息增量 = 确认「缠论具体结构满足 Foundation 抽象义务」这一**结构事实**，
  不是任何实证有效域断言。完备性是缠论公理给定的，本文件忠实编码，不冒充 L1+ 经验验证。

  ── 依赖方向（单向无环）──────────────────────────────────────────────────────
  ChanlunInstantiation → {CompleteClassification, Strict.Recursive, Strict.Trend, Strict.Op,
  Strict.RiskProj} → {Formal.*}。Foundation 内部 srcDir="Foundation" 用裸名
  `import CompleteClassification`；引 Strict 用前缀 `import Strict.X`。

  谱系：598（真完全分类元判据）→ 603（递归 vs 轴范式）→ 615（Layer1 ⊊ Layer2）。
        本文件 = 把缠论具体结构与 GPT 完全分类地基对接的实例化层（build on Foundation）。
        §1 忠实化（D n := List Move + composeStep）的根据 = `RStarNonSpecial.GeneratedStep`/
        `RecursiveConstruction.MovesComposedFrom`（缠论真实序列递归，非单参数复制）。
  禁 sorry/admit/axiom。验证：`cd formal && lake build`。
-/

import CompleteClassification
import Strict.Recursive
import Strict.Trend
import Strict.Op
import Strict.RiskProj

namespace NewChanlun.ChanlunInstantiation

open Formal.RecursiveConstruction
  (Move classifyMove MovesComposedFrom UpperMoveWitness windowSlice UpperMoveSound)
open Formal.TrendTrichotomy (Direction TrendKind)

/-! ════════════════════════════════════════════════════════════════════════
    § 1. LevelRecursion 实例化 ← 缠论走势递归（逐级 compose，序列载体）

    Foundation 的 `recAt`/`RecSpec`/`recSpec_complete_unique`/`recAt_causal` 参数化于
    `D : Nat → Type`、`F0 : H → D 0`、`Fstep : (n) → D n → D (n+1)`。

    ★载体修正（no-patch，de-degenerate 核心）：旧桩取 `ChanD n := Move`（单个走势），
    被迫把 `Fstep n m := compose [m,m,m] …` 复制同一走势三份——违反 `MovesComposedFrom` 的
    `StrictlyIncreasing` 起点 + 中枢由**不同**子走势窗口派生，是退化桩。

    诊断：Foundation `Fstep : D n → D (n+1)` 的「单参数」指「一个 `D n` 类型的对象」，而
    `D n` 类型**完全自由**——可取**序列** `List Move`。缠论第 n+1 级走势由 ≥3 个**不同**的
    第 n 级走势 compose（走势分解定理二，zoushi 第17课），其载体是**下级走势序列**，不是
    单个走势。故忠实载体 = `ChanD n := List Move`（第 n 级走势序列），`Fstep` 取整个序列、
    规范聚合为上级序列。矛盾不存在——是旧桩选错了载体（单走势 vs 序列）。

    `Fstep n := composeStep n`：第 n 级 ≥3 段走势序列 `xs` 规范聚合为第 n+1 级序列
    `[Move.compose xs (canG xs) (n+1)]`（用**真实下级序列** xs，中枢由 `canG xs` 派生，
    级别契约 n+1）；不足 3 段 ⟹ `[]`（缠论级别递归**自然终止**，`CanBuildNext` 假，对应
    `RStarNonSpecial.terminated`——空上级序列是终止吸收态的忠实编码，非退化）。
    `composeStep` 的聚合窗口是 `UpperMoveSound`/`MovesComposedFrom` 的真实窗口结构 witness
    （`composeStep_window_sound` 机器证明）。`F0 := id`（第 0 级 = 种子走势序列本身 = 线段层）。
    ════════════════════════════════════════════════════════════════════════ -/

open Formal.CenterTrichotomy (Center)

/-- 分级宇宙：第 n 级缠论走势**序列**类型（`D n := List Move`，对应 Foundation `D`）。
    载体是序列——缠论第 n+1 级走势由 ≥3 个不同第 n 级走势 compose，序列是其忠实原料。 -/
abbrev ChanD (_ : Nat) : Type := List Move

/-- 派生中枢（复用 `Strict.Recursive.canG`：边界算子层不读 centers，centers 生成忠实性由
    `RecursiveConstruction.CentersDerivedFrom`（WellFormed 层）契约——见 Recursive.lean
    分工边界 + RStarNonSpecial.GeneratedStep。本层与 §1 聚合一致，不重复承载派生证明）。 -/
def chanCenters (xs : List Move) : List Center := Strict.Recursive.canG xs

/--
  ★缠论递归步 `Fstep`（序列实例）：第 n 级走势序列 `xs` 规范聚合为第 n+1 级走势序列。

  缠论「逐级 compose」（走势分解定理二）：当下级序列 `xs` ≥3 段时，把**整个序列**聚合为
  **一个**第 n+1 级走势 `Move.compose xs (canG xs) (n+1)`（用真实下级序列 xs，非复制；中枢
  `canG xs` 由该序列派生；级别契约 n+1 写入 `Move.level` 字段），返回单元素上级序列。
  不足 3 段（缠论 `CanBuildNext` 假）⟹ 返回 `[]`：级别递归自然终止（`RStarNonSpecial`
  `terminated`/`termination_absorbing`），空上级序列忠实编码终止吸收态。

  这是 Foundation `Fstep : (n) → ChanD n → ChanD (n+1)` 的缠论确定性实例——Lean 全函数
  （每个 xs 给出唯一上级序列），契合 Foundation 递归级唯一所需，且忠实于
  `MovesComposedFrom` 的窗口结构（见 `composeStep_window_sound`）。
-/
def composeStep (n : Nat) (xs : ChanD n) : ChanD (n + 1) :=
  if 3 ≤ xs.length then [Move.compose xs (chanCenters xs) (n + 1)] else []

/--
  ★`composeStep` 的窗口 sound 见证（L0，de-degenerate 的核心忠实性证明）：

  对 ≥3 段下级序列 `xs`，`composeStep n xs` 产出的**单个**上级走势由 `xs` 的**真实连续窗口**
  `[0, xs.length)` 经 `Move.compose` 封装（级别 n+1）——即满足 `UpperMoveSound xs n` 的三件：
  窗口长 ≥3、窗口在界内、上级走势 = 该窗口下级序列 compose。这是 `MovesComposedFrom` 的
  **窗口结构 witness**（`∃ ws` 那一支）在 `composeStep` 上成立的机器证明。

  ★这关死旧 `[m,m,m]` 复制桩的退化：聚合序列 = **真实** `xs`（窗口 `[0, xs.length)`，非
  复制同一走势），级别 n+1（非塌缩）。旧桩 `[m,m,m]` 三份同一走势在 `windowSlice` 上无法
  对应真实下级窗口（三份起点相同，违反窗口不重叠），本忠实化用真实序列窗口替换。

  ★诚实边界（no-patch + Recursive.lean 分工边界）：本定理证 `composeStep` 是 **boundary 层**
  窗口 sound（`UpperMoveSound`，不读 centers 的生成忠实性）。`MovesComposedFrom` 还内嵌
  `WellFormed`（含 `centers.length ≥ 1` + `CentersDerivedFrom`）——那是 **WellFormed 层契约**
  （`canG _ := []` 在 Recursive.lean 已诚实声明边界层不承载 centers 派生）。本层**不**冒充
  WellFormed 的 centers 生成义务（声明膨胀禁止）——只证窗口结构忠实（真实序列、级别+1、
  消费约束），这正是 boundary 层应承载的、且足以驱动 Foundation 递归级唯一。
-/
theorem composeStep_window_sound (n : Nat) (xs : ChanD n) (h : 3 ≤ xs.length) :
    ∃ w : UpperMoveWitness,
      UpperMoveSound xs n (Move.compose xs (chanCenters xs) (n + 1)) w := by
  refine ⟨⟨0, xs.length, chanCenters xs⟩, ?_, ?_, ?_⟩
  · -- w.len ≥ 3：xs.length ≥ 3
    exact h
  · -- w.start + w.len ≤ lowerMoves.length：0 + xs.length ≤ xs.length
    simp
  · -- m = compose (windowSlice xs ⟨0, xs.length, _⟩) centers (n+1)
    -- windowSlice xs ⟨0, xs.length, _⟩ = (xs.drop 0).take xs.length = xs
    unfold windowSlice
    simp

/--
  ★`composeStep` 用真实下级序列（L0，反退化见证）：聚合产物的下级窗口 = 原序列 `xs` 本身，
  **不是**复制。这是「`composeStep` 非 `[m,m,m]` 退化」的直接机器证明——
  `windowSlice xs ⟨0, xs.length, _⟩ = xs`，聚合走势 `compose xs (canG xs) (n+1)` 的子走势
  序列恰为真实 `xs`（含其全部不同元素），而非同一走势的三份复制。
-/
theorem composeStep_uses_real_subs (n : Nat) (xs : ChanD n) (h : 3 ≤ xs.length) :
    composeStep n xs = [Move.compose xs (chanCenters xs) (n + 1)]
    ∧ windowSlice xs ⟨0, xs.length, chanCenters xs⟩ = xs := by
  refine ⟨?_, ?_⟩
  · unfold composeStep; rw [if_pos h]
  · unfold windowSlice; simp

/-- 种子取值：历史 = 第 0 级走势序列本身，恒等取值（对应 `Strict.Recursive` 的 `d := id`）。
    种子类型 `H := List Move`（第 0 级走势序列 = 线段/起始层序列）。 -/
def chanSeed : List Move → ChanD 0 := id

/--
  ★缠论走势递归层读出（`recAt` 的缠论实例）：从种子序列 h 经 n 步规范聚合到第 n 级序列。
  这把 Foundation 抽象 `recAt ChanD chanSeed composeStep n h` 钉为缠论具体的「逐级聚合塔」
  （每步把当前级序列规范聚合为上一级序列，忠实于 `MovesComposedFrom`/`GeneratedStep`）。
-/
def chanRecAt (n : Nat) (h : List Move) : ChanD n :=
  NewChanlun.recAt ChanD chanSeed composeStep n h

/--
  ★缠论走势递归级唯一（Foundation `recSpec_complete_unique` 应用于缠论递归）。

  对每个种子序列 h 和级别 n，满足缠论递归规格 `RecSpec`（第 0 级 = 种子序列，第 n+1 级 =
  规范聚合前级）的第 n 级走势序列**存在且唯一**。这是 GPT 完全分类「递归级唯一」义务在缠论
  走势递归上的实例化——缠论的逐级聚合塔在每级唯一确定（每层序列由前层规范聚合唯一得出）。

  ★忠实性：本定理**不**重证 Foundation 的归纳，只把缠论具体的 `ChanD`/`chanSeed`/`composeStep`
  填入 Foundation 已证定理。结论是缠论层的：缠论走势序列递归在每级唯一。
-/
theorem chan_rec_level_unique (h : List Move) (n : Nat) :
    NewChanlun.ExistsUnique
      (fun d : ChanD n => NewChanlun.RecSpec ChanD chanSeed composeStep h n d) :=
  NewChanlun.recSpec_complete_unique ChanD chanSeed composeStep h n

/--
  ★缠论走势递归因果（Foundation `recAt_causal` 应用于缠论递归，ObsEq := Eq）。

  观察等价取 `Eq`（两种子序列相等）。`chanSeed = id` 平凡满足 `hF0`（相等种子给相等第 0 级）。
  结论：相等的种子序列经任意级数规范聚合得相等的第 n 级序列——缠论递归塔是种子的确定性函数
  （因果：当下递归层只依赖种子，无前视）。
-/
theorem chan_rec_causal {h h' : List Move} (hh : h = h') (n : Nat) :
    chanRecAt n h = chanRecAt n h' :=
  NewChanlun.recAt_causal ChanD chanSeed composeStep (fun a b => a = b)
    (fun he => congrArg chanSeed he) hh n

/-! ════════════════════════════════════════════════════════════════════════
    § 2. Priority 实例化 ← 缠论动作优先级（持仓 × 信号决定优先动作）

    Foundation 的 `IsPriorityClass`/`priority_class_complete_unique` 参数化于谓词
    `P1..P5 : X → Prop`，把 X 分为 6 类（emergencyExit/ancestorClosed/localStop/
    reverseNest/sameNest/hold），互斥穷尽。

    缠论动作优先级（`Strict.Op.StrictState` = 持仓 Pos × 信号 Sig）：缠师操作纪律的优先级
    序（zoushi.md 第 17/18 课操作律 + 缠论操盘三段论）。把缠论操作谓词填入 P1..P5：
      - P1 (emergencyExit 急退)  ← 持空遇买侧（空头腿需平，最高优先）：pos=short ∧ sig=buySide
      - P2 (ancestorClosed)      ← 持空遇卖侧（空头翻转/止损）：pos=short ∧ sig=sellSide
      - P3 (localStop 局部止)    ← 持多遇卖侧（降成本减仓）：pos=long ∧ sig=sellSide
      - P4 (reverseNest)         ← 持多遇买侧（降成本买回/反向加）：pos=long ∧ sig=buySide
      - P5 (sameNest 同向)       ← 空仓遇买侧（顺向建仓）：pos=flat ∧ sig=buySide
      - hold（其余）              ← 持仓不动 / 空仓观望（无上述触发）

    这把 Foundation 的抽象 6 类优先级钉为缠论操作律的具体优先序——满足某高优先谓词即归该类，
    否则降级，最终 hold。互斥穷尽由 Foundation 已证保证。
    ════════════════════════════════════════════════════════════════════════ -/

open Strict.Op (Pos Sig StrictState)

/-- P1 急退：持空遇买侧（空头腿需平，最高优先）。 -/
def chanP1 (s : StrictState) : Prop := s.pos = Pos.short ∧ s.sig = Sig.buySide
/-- P2：持空遇卖侧（空头翻转/止损）。 -/
def chanP2 (s : StrictState) : Prop := s.pos = Pos.short ∧ s.sig = Sig.sellSide
/-- P3 局部止：持多遇卖侧（降成本减仓）。 -/
def chanP3 (s : StrictState) : Prop := s.pos = Pos.long ∧ s.sig = Sig.sellSide
/-- P4：持多遇买侧（降成本买回/反向加）。 -/
def chanP4 (s : StrictState) : Prop := s.pos = Pos.long ∧ s.sig = Sig.buySide
/-- P5 同向：空仓遇买侧（顺向建仓）。 -/
def chanP5 (s : StrictState) : Prop := s.pos = Pos.flat ∧ s.sig = Sig.buySide

instance : DecidablePred chanP1 := fun s => by unfold chanP1; infer_instance
instance : DecidablePred chanP2 := fun s => by unfold chanP2; infer_instance
instance : DecidablePred chanP3 := fun s => by unfold chanP3; infer_instance
instance : DecidablePred chanP4 := fun s => by unfold chanP4; infer_instance
instance : DecidablePred chanP5 := fun s => by unfold chanP5; infer_instance

/--
  ★缠论动作优先级 6 类互斥穷尽（Foundation `priority_class_complete_unique` 应用于缠论操作律）。

  对每个完整结构状态 s（持仓 × 信号），缠论动作优先级类（emergencyExit..hold）**存在且唯一**：
  缠论操作律的优先序把每个状态唯一归入一个优先动作类。这是 GPT 完全分类「优先级互斥穷尽」
  义务在缠论操作纪律上的实例化。

  ★忠实性：本定理把缠论具体谓词 `chanP1..chanP5` 填入 Foundation 已证定理，得缠论层结论：
  缠论操作优先级是 StrictState 上的良定义互斥穷尽分类。
-/
theorem chan_priority_complete_unique (s : StrictState) :
    NewChanlun.ExistsUnique
      (fun c => NewChanlun.IsPriorityClass chanP1 chanP2 chanP3 chanP4 chanP5 s c) :=
  NewChanlun.priority_class_complete_unique chanP1 chanP2 chanP3 chanP4 chanP5 s

/--
  ★缠论动作优先级类互不重叠（Foundation `priority_classes_disjoint` 应用于缠论操作律）：
  同一状态满足两个优先级类判据 ⟹ 两类相等（优先序无歧义）。
-/
theorem chan_priority_disjoint {s : StrictState} {c d : NewChanlun.PriorityClass}
    (hc : NewChanlun.IsPriorityClass chanP1 chanP2 chanP3 chanP4 chanP5 s c)
    (hd : NewChanlun.IsPriorityClass chanP1 chanP2 chanP3 chanP4 chanP5 s d) :
    c = d :=
  NewChanlun.priority_classes_disjoint chanP1 chanP2 chanP3 chanP4 chanP5 hc hd

/-! ════════════════════════════════════════════════════════════════════════
    § 3. GlobalVoice 实例化 ← 缠论走势分类器（多级别真裁决判据）

    Foundation 的 `GlobalClassSpec`/`global_class_complete_unique` 参数化于
    `Local : Voice → X → LocalClass → Prop` 与 `hLocal : ∀ v x, ExistsUnique (Local v x ·)`，
    得「每个声部唯一分类」组合为「全局多声部分类存在唯一」。

    缠论走势分类器（`Strict.Trend`）：`I : WalkX → TrendKind`，真裁决判据
    `IsKind c x := outcomeToKind (classifyMove x.centers) = some c`。已证
    `trend_classifies : Strict.Classifies WalkX TrendKind I IsKind`（total/sound/complete/
    disjoint/realized 五件套）。`Classifies` 的 total+complete ⟹ 每个走势唯一分类 ⟹
    `ExistsUnique (IsKind · x)`，正是 Foundation `hLocal` 所需。

    ★多声部载体修正（no-patch，de-degenerate 核心）：旧桩 `chanLocal _ x c := IsKind c x`
    **忽略 voice 参数**——所有声部读同一标签，多级别共振退化为单声部。诊断：Foundation
    `Local v x c` 让**每个声部 v 独立分类**，不同声部可给不同标签（这正是多声部的存在论
    意义）。要让 voice 真起作用，对象 `x` 必须携带「每级（每声部）各自的走势」。旧桩对象
    `WalkX` 是**单级**走势对象，结构上无法承载 per-voice 区分——是载体选错（单走势 vs
    级别族）。

    忠实化：对象 `X := ChanVoice → WalkX`（**级别索引的走势族**，每个级别一个走势对象——
    缠论多级别共振的忠实载体：5 分钟级别上涨 / 30 分钟级别盘整，各级读**各自的中枢序列**）。
    `Local v x c := IsKind c (x v)`（第 v 声部读**它自己的**走势对象 `x v`——voice 真正
    选取不同级别的走势，不同级别 → 不同 centers → 不同标签，真多声部）。`hLocal` 由
    `trend_classifies` 对每个 `x v` 给出。这把 Foundation `GlobalVoice` 抽象忠实承载缠论
    多级别共振——矛盾不存在（Foundation `GlobalVoice` 本就为多声部设计）。
    ════════════════════════════════════════════════════════════════════════ -/

open Strict.Trend (WalkX I IsKind trend_classifies)

/-- 声部 = 缠论级别（多级别共振，Nat 级别索引）。 -/
abbrev ChanVoice : Type := Nat

/-- 多级别走势族 `VoiceFamily`：每个声部（级别）一个走势对象——缠论多级别共振的忠实载体
    （不同级别携带各自的中枢序列/走势分类，体现力度区分）。 -/
abbrev VoiceFamily : Type := ChanVoice → WalkX

/--
  ★缠论局部走势分类关系 `Local`（忠实多声部，voice 真起作用）：声部（级别）v 下，
  多级别走势族 `x` 在该级的走势 `x v` 的局部分类是 c。

  `chanLocal v x c := IsKind c (x v)`——第 v 声部读**它自己的**走势对象 `x v`（不同级别
  读不同走势的真裁决判据 `outcomeToKind (classifyMove (x v).centers) = some c`）。voice 参数

exec
/bin/zsh -lc "rg -n \"def composeStep|abbrev ChanD|def canOp|def bdry|recSpec_complete_unique|LevelRecursion|MovesComposedFrom|StrictlyIncreasing|Move\\.compose\" formal -S" in /Users/silencehan/Projects/NewChanlun
 succeeded in 0ms:
formal/lakefile.toml:58:#     递归级唯一(recSpec_complete_unique/recAt_causal) + 优先级互斥穷尽 + π 流水线存在唯一。
formal/lakefile.toml:64:#     实例化 CompleteClassification 抽象参数——LevelRecursion ← Strict.Recursive 走势递归（canStep
formal/Strict/Recursive.lean:33:  写入 `Move.compose ... (n+1)` 的 level 字段携带。
formal/Strict/Recursive.lean:71:  - `branch`：镜像 `Move.compose`（递归分支，子树序列 + 中枢 + 级别）。
formal/Strict/Recursive.lean:84:    | DecompTree.branch ts cs lv => Move.compose (evalList ts) cs lv
formal/Strict/Recursive.lean:94:    | Move.compose subs cs lv => DecompTree.branch (decompList subs) cs lv
formal/Strict/Recursive.lean:107:    | Move.compose subs _ _ => by
formal/Strict/Recursive.lean:203:  级别契约 `n+1` 写入 `Move.compose` 的 level 字段；≥3 约束由子类型 `{xs // len ≥ 3}` 在
formal/Strict/Recursive.lean:206:def canOp (n : Nat) (xs : {l : List (U n) // l.length ≥ 3}) : U (n+1) :=
formal/Strict/Recursive.lean:207:  Move.compose xs.val (canG xs.val) (n+1)
formal/Strict/Recursive.lean:210:def bdry (n : Nat) : U (n+1) → List (U n)
formal/Strict/Recursive.lean:211:  | Move.compose subs _ _ => subs
formal/Strict/Recursive.lean:219:    （`Can` 单射于边界——`canOp` 是 `Move.compose` 应用，构造子单射 + injection 取第一分量）。
formal/Strict/Recursive.lean:229:    exact (Move.compose.injEq _ _ _ _ _ _).mp h |>.1
formal/Strict/Recursive.lean:300:    ∧ (∃ subs cs lv, moveKernel.Can n xs = Move.compose subs cs lv) := by
formal/Strict/Decomp.lean:62:  连接 `⊕` = 二元 `Move.compose [a, b] [] 1`（次级别走势序列拼接的二元包装，第36课）。
formal/Strict/Decomp.lean:69:def connect (a b : Move) : Move := Move.compose [a, b] [] 1
formal/Strict/Decomp.lean:83:  | Move.compose subs centers lvl =>
formal/Strict/Decomp.lean:86:      | _, _, _ => [Move.compose subs centers lvl]
formal/Strict/Decomp.lean:103:    flattenConnect (Move.compose subs (c :: cs) lvl) = [Move.compose subs (c :: cs) lvl] := by
formal/Strict/Decomp.lean:164:    ¬ StructEqN (Move.compose subs [c₁] lvl) (Move.compose subs [c₂] lvl) := by
formal/Strict/Decomp.lean:234:  let t₁ := Move.compose [s1, s2, s3] [c₁] 1
formal/Strict/Decomp.lean:235:  let t₂ := Move.compose [s1, s2, s3] [c₂] 1
formal/Strict/Decomp.lean:244:    show leaves (Move.compose [s1, s2, s3] [c₁] 1) = [s1, s2, s3]
formal/Strict/Decomp.lean:247:    show leaves (Move.compose [s1, s2, s3] [c₂] 1) = [s1, s2, s3]
formal/Tlayers/Spiral.lean:118:    Formal.RecursiveConstruction.outcome? (Move.compose subs centers lvl)
formal/Foundation/CompleteClassification.lean:107:section LevelRecursion
formal/Foundation/CompleteClassification.lean:131:theorem recSpec_complete_unique (h : H) (n : Nat) :
formal/Foundation/CompleteClassification.lean:160:end LevelRecursion
formal/Foundation/ChanlunInstantiation.lean:8:    - LevelRecursion：`D : Nat → Type` / `F0 : H → D 0` / `Fstep : (n) → D n → D (n+1)`，
formal/Foundation/ChanlunInstantiation.lean:9:      证 `recSpec_complete_unique`（递归级唯一）+ `recAt_causal`（因果，ObsEq 下不变）。
formal/Foundation/ChanlunInstantiation.lean:25:  1. LevelRecursion ← 缠论 走势递归（`Strict.Recursive`/`RStarNonSpecial` 的逐级 compose）：
formal/Foundation/ChanlunInstantiation.lean:29:       忠实于 `RecursiveConstruction.MovesComposedFrom` 关系——非复制、用真实下级序列）。
formal/Foundation/ChanlunInstantiation.lean:30:     得 `recSpec_complete_unique`（缠论走势递归级唯一）+ `recAt_causal`（同种子同级序列相等）。
formal/Foundation/ChanlunInstantiation.lean:31:     ★忠实性根据：`composeStep` 的聚合窗口是 `UpperMoveSound`/`MovesComposedFrom` 的真实窗口
formal/Foundation/ChanlunInstantiation.lean:63:        `RecursiveConstruction.MovesComposedFrom`（缠论真实序列递归，非单参数复制）。
formal/Foundation/ChanlunInstantiation.lean:76:  (Move classifyMove MovesComposedFrom UpperMoveWitness windowSlice UpperMoveSound)
formal/Foundation/ChanlunInstantiation.lean:80:    § 1. LevelRecursion 实例化 ← 缠论走势递归（逐级 compose，序列载体）
formal/Foundation/ChanlunInstantiation.lean:82:    Foundation 的 `recAt`/`RecSpec`/`recSpec_complete_unique`/`recAt_causal` 参数化于
formal/Foundation/ChanlunInstantiation.lean:86:    被迫把 `Fstep n m := compose [m,m,m] …` 复制同一走势三份——违反 `MovesComposedFrom` 的
formal/Foundation/ChanlunInstantiation.lean:87:    `StrictlyIncreasing` 起点 + 中枢由**不同**子走势窗口派生，是退化桩。
formal/Foundation/ChanlunInstantiation.lean:96:    `[Move.compose xs (canG xs) (n+1)]`（用**真实下级序列** xs，中枢由 `canG xs` 派生，
formal/Foundation/ChanlunInstantiation.lean:99:    `composeStep` 的聚合窗口是 `UpperMoveSound`/`MovesComposedFrom` 的真实窗口结构 witness
formal/Foundation/ChanlunInstantiation.lean:107:abbrev ChanD (_ : Nat) : Type := List Move
formal/Foundation/ChanlunInstantiation.lean:118:  **一个**第 n+1 级走势 `Move.compose xs (canG xs) (n+1)`（用真实下级序列 xs，非复制；中枢
formal/Foundation/ChanlunInstantiation.lean:125:  `MovesComposedFrom` 的窗口结构（见 `composeStep_window_sound`）。
formal/Foundation/ChanlunInstantiation.lean:127:def composeStep (n : Nat) (xs : ChanD n) : ChanD (n + 1) :=
formal/Foundation/ChanlunInstantiation.lean:128:  if 3 ≤ xs.length then [Move.compose xs (chanCenters xs) (n + 1)] else []
formal/Foundation/ChanlunInstantiation.lean:134:  `[0, xs.length)` 经 `Move.compose` 封装（级别 n+1）——即满足 `UpperMoveSound xs n` 的三件：
formal/Foundation/ChanlunInstantiation.lean:135:  窗口长 ≥3、窗口在界内、上级走势 = 该窗口下级序列 compose。这是 `MovesComposedFrom` 的
formal/Foundation/ChanlunInstantiation.lean:143:  窗口 sound（`UpperMoveSound`，不读 centers 的生成忠实性）。`MovesComposedFrom` 还内嵌
formal/Foundation/ChanlunInstantiation.lean:151:      UpperMoveSound xs n (Move.compose xs (chanCenters xs) (n + 1)) w := by
formal/Foundation/ChanlunInstantiation.lean:169:    composeStep n xs = [Move.compose xs (chanCenters xs) (n + 1)]
formal/Foundation/ChanlunInstantiation.lean:182:  （每步把当前级序列规范聚合为上一级序列，忠实于 `MovesComposedFrom`/`GeneratedStep`）。
formal/Foundation/ChanlunInstantiation.lean:188:  ★缠论走势递归级唯一（Foundation `recSpec_complete_unique` 应用于缠论递归）。
formal/Foundation/ChanlunInstantiation.lean:200:  NewChanlun.recSpec_complete_unique ChanD chanSeed composeStep h n
formal/Formal/RStarNonSpecial.lean:32:open Formal.RecursiveConstruction (Move classifyMove CentersDerivedFrom MovesComposedFrom)
formal/Formal/RStarNonSpecial.lean:85:  ∧ MovesComposedFrom cur.moves cur.levelIdx next.moves
formal/Formal/RStarNonSpecial.lean:228:  ★生成步上级走势是封装对象（codex R5：MovesComposedFrom，非原样复制）。
formal/Formal/RStarNonSpecial.lean:234:    MovesComposedFrom cur.moves cur.levelIdx next.moves := h.2.2.2
formal/Phase2/Claim5_ConstitutiveLadder.lean:58:def StrictlyIncreasing : List Nat → Prop
formal/Phase2/Claim5_ConstitutiveLadder.lean:61:  | a :: b :: rest => a < b ∧ StrictlyIncreasing (b :: rest)
formal/Phase2/Claim5_ConstitutiveLadder.lean:499:  | Move.compose _ centers _ => classifyMove centers
formal/Phase2/Claim5_ConstitutiveLadder.lean:538:    ∧ StrictlyIncreasing starts
formal/Phase2/Claim5_ConstitutiveLadder.lean:556:  | Move.compose subs centers lvl =>
formal/Phase2/Claim5_ConstitutiveLadder.lean:661:    (h : MoveWellFormed (Move.compose subs centers lvl)) :
formal/Phase2/Claim5_ConstitutiveLadder.lean:672:    (h : MoveWellFormed (Move.compose subs centers lvl)) :
formal/Phase2/Claim5_ConstitutiveLadder.lean:682:    (h : MoveWellFormed (Move.compose subs centers lvl)) :
formal/Phase2/Claim5_ConstitutiveLadder.lean:718:    (lvl : Nat) (h : MoveWellFormed (Move.compose subs centers lvl)) : centers ≠ [] := by
formal/Phase2/Claim5_ConstitutiveLadder.lean:727:  这是 codex 修正1 的形式化见证：`Move.compose` 删除了自由 `kind` 字段。
formal/Phase2/Claim5_ConstitutiveLadder.lean:731:    trendKind? (Move.compose subs centers lvl) = classifyMove centers := rfl
formal/Phase2/Claim5_ConstitutiveLadder.lean:738:    trendKind? (Move.compose subs [c] lvl) = some TrendKind.consolidation := rfl
formal/Formal/ConstitutiveLadder.lean:33:def StrictlyIncreasing : List Nat → Prop
formal/Formal/ConstitutiveLadder.lean:36:  | a :: b :: rest => a < b ∧ StrictlyIncreasing (b :: rest)
formal/Formal/EvalSoundness.lean:75:  | Move.compose subs _ _ => subs.flatMap leaves
formal/Formal/EvalSoundness.lean:84:    leaves (Move.compose subs centers lvl) = subs.flatMap leaves := by
formal/Formal/EvalSoundness.lean:96:      motive (Move.compose subs centers level))
formal/Formal/EvalSoundness.lean:169:      Move.compose chunk spec.centers spec.level :: groupBySpecs specs remaining
formal/Formal/EvalSoundness.lean:233:    leaves (Move.compose subs centers lvl) = subs := by
formal/Formal/EvalSoundness.lean:258:  refine ⟨Move.compose
formal/Formal/EvalSoundness.lean:295:    (leaves (Move.compose subs centers lvl)).length ≥ subs.length := by
formal/Formal/EvalSoundness.lean:315:    (h : WellFormed (Move.compose subs centers lvl)) :
formal/Formal/EvalSoundness.lean:316:    leaves (Move.compose subs centers lvl) ≠ [Move.compose subs centers lvl] := by
formal/Formal/EvalSoundness.lean:323:  have hge : (leaves (Move.compose subs centers lvl)).length ≥ subs.length :=
formal/Formal/EvalSoundness.lean:334:    (leaves (Move.compose subs centers lvl)).length = subs.length := by
formal/Formal/EvalSoundness.lean:351:    (h : WellFormed (Move.compose subs centers lvl)) :
formal/Formal/EvalSoundness.lean:367:    (h : WellFormed (Move.compose subs centers lvl))
formal/Formal/EvalSoundness.lean:369:    (leaves (Move.compose subs centers lvl)).length ≥ 3 := by
formal/Formal/EvalSoundness.lean:381:    (h : WellFormed (Move.compose subs centers lvl)) :
formal/Formal/EvalSoundness.lean:382:    (leaves (Move.compose subs centers lvl)).length ≥ 3 := by
formal/Formal/EvalSoundness.lean:386:  have hge : (leaves (Move.compose subs centers lvl)).length ≥ subs.length :=
formal/Formal/EvalSoundness.lean:538:  Move.compose subs centers lvl
formal/Formal/RecursiveConstruction.lean:100:  | Move.compose _ centers _ => some (classifyMove centers)
formal/Formal/RecursiveConstruction.lean:133:def StrictlyIncreasing : List Nat → Prop
formal/Formal/RecursiveConstruction.lean:136:  | a :: b :: rest => a < b ∧ StrictlyIncreasing (b :: rest)
formal/Formal/RecursiveConstruction.lean:147:    ∧ StrictlyIncreasing starts
formal/Formal/RecursiveConstruction.lean:162:  | Move.compose subs centers lvl =>
formal/Formal/RecursiveConstruction.lean:194:  ∧ m = Move.compose (windowSlice lowerMoves w) w.centers (lowerLevel + 1)
formal/Formal/RecursiveConstruction.lean:201:  复用**（codex R6 反例：3 个相同 compose 重复消费同 3 段——被 `StrictlyIncreasing` 起点 +
formal/Formal/RecursiveConstruction.lean:206:  - witness 起点 `StrictlyIncreasing`（时间顺序，无重排）；
formal/Formal/RecursiveConstruction.lean:210:def MovesComposedFrom (lowerMoves : List Move) (lowerLevel : Nat)
formal/Formal/RecursiveConstruction.lean:217:      ∧ StrictlyIncreasing (ws.map UpperMoveWitness.start)
formal/Formal/RecursiveConstruction.lean:250:    outcome? (Move.compose subs centers lvl) = some (classifyMove centers) := rfl
formal/Formal/RecursiveConstruction.lean:281:    (h : WellFormed (Move.compose subs centers lvl)) :
formal/Formal/RecursiveConstruction.lean:297:    (h : WellFormed (Move.compose subs centers lvl)) :
formal/Formal/RecursiveConstruction.lean:305:    (h : WellFormed (Move.compose subs centers lvl)) :

codex
The local adapter really does implement the excerpt: `ChanD n := List Move` and `composeStep` maps a whole lower sequence to a singleton upper sequence. I’m checking the actual `MovesComposedFrom` relation now, because that is where “one block vs many blocks” is formalized.
exec
/bin/zsh -lc "sed -n '1,340p' formal/Formal/RecursiveConstruction.lean" in /Users/silencehan/Projects/NewChanlun
 succeeded in 0ms:
/-
  走势递归构造 = 初代数 μF（006 走势分解定理二 + level_recursion + 603 §二/§三）
  ★codex 异质审计修正版 R2（session 019eff21，R1 发现 1/2/3/5 + R2 发现 3/4/5/全链 已吸收）

  缠论定理（已结算）：
  - 走势分解定理二（zoushi.md:28，第17课）："任何级别的任何走势类型，都至少由三段以上
    次级别走势类型构成。" → 递归构造 `Move(ℓ) := f(Move(ℓ-1) list, len ≥ 3)`。
  - 归纳基底（level_recursion.md:21）：`Move[0] = Segment`（线段，codex 硬修正1）。
  - 趋势定义（zoushi 第17课）："至少两个以上**依次同向**中枢"——★全链同向，非仅首两个。
  - 盘整定义：恰好 1 中枢。完成走势 **必含 ≥1 中枢**（原理二）——0 中枢非盘整。
  - 级别之分有极限（zhongshu.md:18）：递归有底。

  ★codex 审计修正（faithfulness）：
  - R1#1：segment 不携带 TrendKind（线段无中枢）。`trendKind?` segment ↦ none。
  - R1#2：level 字段 + WellFormed 强制 `∀ sub, sub.level = lvl-1`（级别递减，Lean 可接受替代）。
  - R1#3 + R2#3：kind 不自由携带——由中枢计算；且 `CentersDerivedFrom subs centers`（中枢由子走势生成）
    + `centers.length ≥ 1`（完成走势必含中枢）关死"伪造 centers"漏洞。
  - R2#5：级别扩张 ↦ `MoveOutcome.higherCenterCandidate`，**不**直接 ↦ consolidation。
  - R2 全链：趋势 = **所有相邻中枢同向一致**（`AllAdjacent`），非只看首两中枢。
-/

import Formal.TrendTrichotomy
import Formal.CenterTrichotomy

namespace Formal.RecursiveConstruction

open Formal.TrendTrichotomy
open Formal.CenterTrichotomy
  (Center CenterRelation classify center_trichotomy MoveOutcome relationToOutcome)

/--
  ★走势的初代数 μF（递归数据类型，006，codex 审计修正 R1/R2）。
  - `segment`：归纳基底 `Move[0] = Segment`（无 TrendKind，R1#1）。
  - `compose subs centers level`：递归分支。走势裁决由 `centers` **计算**（不自由携带，R1#3）；
    级别递减 + ≥3 段 + 中枢派生一致性由 `WellFormed` 强制（R2#3）。
-/
inductive Move where
  | segment (dir : Direction) (lo hi : Int)
  | compose (subs : List Move) (centers : List Center) (level : Nat)

namespace Move

def level : Move → Nat
  | segment _ _ _ => 0
  | compose _ _ lvl => lvl

/-- 走势的价格区间（segment 直接带；compose 由其中枢外缘合成）。 -/
def interval : Move → (Int × Int)
  | segment _ lo hi => (lo, hi)
  | compose _ centers _ =>
      let los := centers.map (·.dd)
      let his := centers.map (·.gg)
      (los.foldr min 0, his.foldr max 0)

/-- 走势区间下沿。 -/
def lo (m : Move) : Int := m.interval.1
/-- 走势区间上沿。 -/
def hi (m : Move) : Int := m.interval.2

end Move

open Move
open Formal.CenterTrichotomy.CenterRelation
open Formal.CenterTrichotomy.MoveOutcome

/--
  ★所有相邻中枢同向一致（codex R2 全链问题）：趋势 = "依次同向"中枢链。

  `allAdjacent rel centers` = 中枢序列中每对相邻中枢的 `classify` 都等于 `rel`。
  趋势要求全链同向（全 upContinuation 或全 downContinuation），不是只看首两个——
  否则 `[up, up, expansion]` 或 `[up, down]` 会被误判为趋势。
-/
def allAdjacent (rel : CenterRelation) : List Center → Bool
  | [] => true
  | [_] => true
  | c0 :: c1 :: rest =>
      (decide (classify c0 c1 = rel)) && allAdjacent rel (c1 :: rest)

/--
  ★走势裁决（codex R1#3 + R2#3/全链：由中枢序列计算，全链一致）。

  - 0 中枢：`higherCenterCandidate`（**非盘整**——完成走势必含 ≥1 中枢，R2#3；
    0 中枢是退化/未完成，不归盘整侧，归"待定/升级"）。
  - 1 中枢：consolidation（盘整定义）。
  - ≥2 中枢全链上涨延续：trend up；全链下跌延续：trend down。
  - ≥2 中枢非全链一致（含扩张或方向混合）：higherCenterCandidate（本级终结，交父级）。
-/
def classifyMove (centers : List Center) : MoveOutcome :=
  match centers with
  | [] => higherCenterCandidate
  | [_] => consolidation
  | c0 :: c1 :: rest =>
      if allAdjacent upContinuation (c0 :: c1 :: rest) then trend Direction.up
      else if allAdjacent downContinuation (c0 :: c1 :: rest) then trend Direction.down
      else higherCenterCandidate

/-- 走势裁决读出（codex R1#1：segment 无裁决，none）。 -/
def outcome? : Move → Option MoveOutcome
  | Move.segment _ _ _ => none
  | Move.compose _ centers _ => some (classifyMove centers)

/--
  ★中枢在某起点 `i` 由子走势窗口生成（codex R3#2 + R4：核心 **和外缘** 都 sound）。

  一个中枢 `c` 在起点 `i` 由 `subs` 生成 ⟺ 连续三段 `subs[i..i+2]` 的区间重叠产生 `c`：
  - 核心：`c.zd = max(三段 lo)`、`c.zg = min(三段 hi)`（zhongshu.md / formal_axioms §3.2）；
  - ★外缘（codex R4 必修）：`c.dd = min(三段 lo)`、`c.gg = max(三段 hi)`。
    外缘 dd/gg **驱动 up/down/expansion 判定**——不约束外缘则可伪造外缘翻转分类。

  （三段窗口版；延伸中枢的 start/finish 区间版留待 Rust 引擎，Lean 给三段 sound contract。）
-/
def CenterDerivedAt (subs : List Move) (c : Center) (i : Nat) : Prop :=
  i + 2 < subs.length
    ∧ (∀ (h0 : i < subs.length) (h1 : i + 1 < subs.length) (h2 : i + 2 < subs.length),
        c.zd = max (subs[i].lo) (max (subs[i+1].lo) (subs[i+2].lo))
        ∧ c.zg = min (subs[i].hi) (min (subs[i+1].hi) (subs[i+2].hi))
        ∧ c.dd = min (subs[i].lo) (min (subs[i+1].lo) (subs[i+2].lo))
        ∧ c.gg = max (subs[i].hi) (max (subs[i+1].hi) (subs[i+2].hi)))

/-- 存在某起点窗口生成该中枢（核心+外缘 sound）。 -/
def CenterDerivedFromWindow (subs : List Move) (c : Center) : Prop :=
  ∃ i, CenterDerivedAt subs c i

/--
  ★中枢序列由子走势生成（codex R3#2 + R4：witness 列表 + **时间顺序**）。

  `CentersDerivedFrom subs centers` ⟺ 存在一个 witness 起点列表 `starts`：
  - 长度等于 centers；
  - ★严格递增（codex R4：中枢时间顺序——趋势"依次同向中枢"依赖此序，否则可重排 centers 翻转分类）；
  - 每个 center 在对应起点处核心+外缘 sound。
  + subs 非空。这把"中枢非凭空"升级为 witness soundness + 顺序 contract（codex R3#2 + R4）。
-/
def StrictlyIncreasing : List Nat → Prop
  | [] => True
  | [_] => True
  | a :: b :: rest => a < b ∧ StrictlyIncreasing (b :: rest)

/-- 逐对谓词（无 Mathlib 的 Forall₂ 替代）：两 list 等长且逐位满足 R（多态）。 -/
def PairwiseRel {α β : Type} (R : α → β → Prop) : List α → List β → Prop
  | [], [] => True
  | a :: as, b :: bs => R a b ∧ PairwiseRel R as bs
  | _, _ => False

def CentersDerivedFrom (subs : List Move) (centers : List Center) : Prop :=
  subs ≠ [] ∧ ∃ starts : List Nat,
    starts.length = centers.length
    ∧ StrictlyIncreasing starts
    ∧ PairwiseRel (fun c i => CenterDerivedAt subs c i) centers starts

/--
  走势良构（WellFormed，走势分解定理二 + codex R1#2/#3 + R2#3）。
  `compose subs centers lvl` 良构 ⟺
    (a) `subs.length ≥ 3`（走势分解定理二）；
    (b) `lvl ≥ 1`；
    (c) `∀ m∈subs, m.level = lvl-1`（级别递减，R1#2）；
    (d) `centers.length ≥ 1`（完成走势必含中枢，R2#3）；
    (e) `CentersDerivedFrom subs centers`（中枢由子走势生成，非伪造，R2#3）；
    (f) `∀ m∈subs, WellFormed m`（递归）。
-/
def WellFormed : Move → Prop
  | Move.segment _ _ _ => True
  | Move.compose subs centers lvl =>
      subs.length ≥ 3 ∧ lvl ≥ 1
      ∧ (∀ m ∈ subs, m.level = lvl - 1)
      ∧ centers.length ≥ 1
      ∧ CentersDerivedFrom subs centers
      ∧ (∀ m ∈ subs, WellFormed m)

/--
  ★单个上级走势的窗口 witness（codex R6：有序、连续、消费约束）。

  一个 upper move `m` 由 `lowerMoves` 的 **连续窗口** `[start, start+len)` 封装：
  - `len ≥ 3`（走势分解定理二，每上级走势 ≥3 下级走势）；
  - `start + len ≤ lowerMoves.length`（窗口在界内）；
  - `m = compose (lowerMoves 的 [start, start+len) 切片) centers (lowerLevel+1)`
    （上级走势 = 该窗口下级走势 compose，级别 +1）。

  `subsSlice` 用 `List.drop start |>.take len` 提取连续窗口（无 Mathlib extract 依赖）。
-/
structure UpperMoveWitness where
  start : Nat
  len : Nat
  centers : List Center

/-- 窗口提取：lowerMoves 的连续片段 [start, start+len)。 -/
def windowSlice (lowerMoves : List Move) (w : UpperMoveWitness) : List Move :=
  (lowerMoves.drop w.start).take w.len

/-- 单个 upper move 由窗口 witness sound 封装（连续窗口 + 级别 +1 + compose）。 -/
def UpperMoveSound (lowerMoves : List Move) (lowerLevel : Nat)
    (m : Move) (w : UpperMoveWitness) : Prop :=
  w.len ≥ 3
  ∧ w.start + w.len ≤ lowerMoves.length
  ∧ m = Move.compose (windowSlice lowerMoves w) w.centers (lowerLevel + 1)

/--
  ★上级走势由下级走势封装而成（codex R6：有序窗口 witness + 消费/数量约束，关死"重复封装"漏洞）。

  级别递归的封装步（封装恒等式 Move(k)≡Level-(k+1) 笔，rec_t types.rs:81）：
  `upperMoves` 由 `lowerMoves` 的 **有序、不重叠、连续窗口** 逐个封装而成，**不是集合成员式
  复用**（codex R6 反例：3 个相同 compose 重复消费同 3 段——被 `StrictlyIncreasing` 起点 +
  不重叠 + 数量约束关死）。约束：
  - `upperMoves ≠ []`（生成非空）；
  - 每个 upper move 良构（`WellFormed`）；
  - witness 列表与 upperMoves 等长，逐位 `UpperMoveSound`（连续窗口、级别+1、compose）；
  - witness 起点 `StrictlyIncreasing`（时间顺序，无重排）；
  - ★`upperMoves.length * 3 ≤ lowerMoves.length`（消费约束：每上级走势吃 ≥3 下级，
    有限下级走势不能无限封装——保证级别递归自然收敛，codex R6 核心）。
-/
def MovesComposedFrom (lowerMoves : List Move) (lowerLevel : Nat)
    (upperMoves : List Move) : Prop :=
  upperMoves ≠ []
  ∧ (∀ m ∈ upperMoves, WellFormed m)
  ∧ upperMoves.length * 3 ≤ lowerMoves.length
  ∧ ∃ ws : List UpperMoveWitness,
      ws.length = upperMoves.length
      ∧ StrictlyIncreasing (ws.map UpperMoveWitness.start)
      ∧ PairwiseRel
          (fun m w => UpperMoveSound lowerMoves lowerLevel m w) upperMoves ws

/--
  ★真完全分类·全级别（裁决结果穷尽，结构归纳，L0，codex R2 修正后）：

  任意 compose 走势的裁决必属 {trend up, trend down, consolidation, higherCenterCandidate}。
  这是 `classifyMove` 的全函数判定完备。注意：**良构** compose（centers≥1）排除 0 中枢的
  higherCenterCandidate-退化分支，故良构走势裁决 ∈ {trend, consolidation, higherCenterCandidate}
  且 consolidation/trend 是终值、higherCenterCandidate 交父级——无第五种结果。
-/
theorem outcome_total (centers : List Center) :
    classifyMove centers = trend Direction.up
    ∨ classifyMove centers = trend Direction.down
    ∨ classifyMove centers = consolidation
    ∨ classifyMove centers = higherCenterCandidate := by
  unfold classifyMove
  match centers with
  | [] => simp
  | [_] => simp
  | c0 :: c1 :: rest =>
      by_cases hu : allAdjacent upContinuation (c0 :: c1 :: rest) = true
      · simp [hu]
      · by_cases hd : allAdjacent downContinuation (c0 :: c1 :: rest) = true
        · simp [hu, hd]
        · simp [hu, hd]

/--
  ★裁决由中枢决定（codex R1#3 + R2#3：不自由携带 + 中枢派生）。
  `outcome?` 完全由 `centers` 决定——不存在"自由携带与中枢不一致的裁决"。
-/
theorem outcome_determined_by_centers (subs : List Move) (centers : List Center) (lvl : Nat) :
    outcome? (Move.compose subs centers lvl) = some (classifyMove centers) := rfl

/--
  ★≥2 全链上涨延续中枢 ⟹ 上涨趋势（codex R2 全链：检查所有相邻中枢）。
-/
theorem all_up_centers_is_uptrend (c0 c1 : Center) (rest : List Center)
    (h : allAdjacent upContinuation (c0 :: c1 :: rest) = true) :
    classifyMove (c0 :: c1 :: rest) = trend Direction.up := by
  simp [classifyMove, h]

/-- ★非全链一致 ⟹ higherCenterCandidate（含方向混合/扩张，本级终结交父级）。 -/
theorem mixed_centers_is_higher_candidate (c0 c1 : Center) (rest : List Center)
    (hu : allAdjacent upContinuation (c0 :: c1 :: rest) = false)
    (hd : allAdjacent downContinuation (c0 :: c1 :: rest) = false) :
    classifyMove (c0 :: c1 :: rest) = higherCenterCandidate := by
  simp [classifyMove, hu, hd]

/-- ★单中枢 ⟹ 盘整（zoushi 第17课盘整定义）。 -/
theorem one_center_is_consolidation (c : Center) :
    classifyMove [c] = consolidation := rfl

/-- ★0 中枢 ⟹ 非盘整（codex R2#3：完成走势必含中枢，0 中枢归 higherCenterCandidate）。 -/
theorem zero_centers_not_consolidation :
    classifyMove [] = higherCenterCandidate := rfl

/--
  ★良构走势必含 ≥1 中枢（codex R2#3）：良构 compose 的裁决不可能是 0 中枢退化分支。
  即良构走势裁决 ∈ {trend up, trend down, consolidation, higherCenterCandidate（来自≥2 混合）}，
  其中 consolidation 来自恰 1 中枢——0 中枢被 `centers.length ≥ 1` 排除。
-/
theorem wellformed_has_center (subs : List Move) (centers : List Center) (lvl : Nat)
    (h : WellFormed (Move.compose subs centers lvl)) :
    centers.length ≥ 1 := by
  unfold WellFormed at h
  obtain ⟨_, _, _, hc, _, _⟩ := h
  exact hc

/-- ★base case = Segment 无裁决（codex 硬修正1 + R1#1）。 -/
theorem segment_no_outcome (d : Direction) (lo hi : Int) :
    outcome? (Move.segment d lo hi) = none := rfl

/-- segment 级别为 0（递归底，zhongshu.md:18）。 -/
theorem segment_is_base (d : Direction) (lo hi : Int) :
    (Move.segment d lo hi).level = 0 := rfl

/-- ★级别递减约束生效（codex R1#2）：良构 compose 每个子走势级别 = 父级 -1。 -/
theorem wellformed_subs_one_lower (subs : List Move) (centers : List Center) (lvl : Nat)
    (h : WellFormed (Move.compose subs centers lvl)) :
    ∀ m ∈ subs, m.level = lvl - 1 := by
  unfold WellFormed at h
  obtain ⟨_, _, hlevel, _, _, _⟩ := h
  exact hlevel

/-- ★中枢由子走势生成（codex R2#3）：良构 compose 满足 CentersDerivedFrom。 -/
theorem wellformed_centers_derived (subs : List Move) (centers : List Center) (lvl : Nat)
    (h : WellFormed (Move.compose subs centers lvl)) :
    CentersDerivedFrom subs centers := by
  unfold WellFormed at h
  obtain ⟨_, _, _, _, hderiv, _⟩ := h
  exact hderiv

/--
  ★伪造中枢无 witness 窗口（codex R3#2：CentersDerivedFrom 真约束，非恒真）。

  子走势不足 3 段时（subs.length < 3），任何中枢都 **无** 生成窗口——
  `CenterDerivedFromWindow` 的存在量词 `i + 2 < subs.length` 无解。这证明该关系
  **真的会拒绝** 伪造的中枢，不是恒真占位（关死 codex R3#2 漏洞的可证伪性见证）。
-/
theorem no_window_when_too_short (subs : List Move) (c : Center)
    (h : subs.length < 3) : ¬ CenterDerivedFromWindow subs c := by
  unfold CenterDerivedFromWindow CenterDerivedAt
  intro ⟨i, hi, _⟩
  omega

/--
  ★实时走势候选（标准第5部分：未完成走势是 μF 的当下状态，不是新构造子）。
  `settled : Bool` 标记结构是否已封闭——`true`=已完成（可给最终结果），
  `false`=未完成（只给当下状态，不给最终结果）。
-/
structure CandidateMove where
  subs : List Move
  centers : List Center
  level : Nat
  settled : Bool

/--
  ★走势的当下状态（标准第5部分，μF 状态层：未完成走势只给状态，不给最终结果）。

  - `completed (o : MoveOutcome)`：**已完成**走势——结构已封闭，可给最终结果 `o`
    （trend/consolidation/higherCenterCandidate）。这是标准"已完成结构"侧。
  - `pending (tail : MoveOutcome)`：**未完成**走势——只刻画当下尾部倾向 `tail`

exec
/bin/zsh -lc "sed -n '1,300p' formal/Formal/RStarNonSpecial.lean" in /Users/silencehan/Projects/NewChanlun
 succeeded in 0ms:
/-
  最高走势类型 r* 非特殊 = 终余代数 νG（603 §四 + level_recursion + 597）
  ★codex 异质审计修正版（session 019eff21，发现 8/9 已吸收，no-patch：重写非补丁）

  缠论定理（已结算）：
  - 级别向上无结构上限（zhongshu.md:18，level_recursion.md:249）：级别递归向上无限
    unfold，只有 `settled moves < 3` 的 **自然终止**，`max_levels` 是安全上限非缠论约束。
  - r* = unfold 到当前的最深层，与任意 Move(ℓ) 用 **同一三构造子**（603 §四，codex Q2 PASS）。

  ★codex 审计修正（faithfulness）：
  - 发现8：`Tower := Nat → TowerLevel` + `∃ next, tower(k+1)=next` 只证"Nat 无最大元"
    （平凡），不等价于"级别无结构上限/r* 非特殊"，且未保证 k+1 层由 k 层构造。
    修正：用 `ValidTower`（带生成约束）；r* 非特殊重述为 **操作函数对"是否顶层"不可观察**
    + **扩展 tower 不改变既有层的操作**（extensionality）。
  - 发现9：`operate_uniform` 只因 `nonempty` 得 true，没证"不查 has_higher/protect_core"。
    修正：证 operate 只依赖 (kind, settled, BSP)——extensionality：两层若这三者相同则
    operate 相同；且 tower 扩展不改任意已有层 operate 结果。

  范式重铸（603）：级别递归是 **终余代数（coinductive）**——向上无限 unfold，无"最后一步"。
  r* 非特殊 = 操作语义对顶层性不可观察 ⟹ 支撑 597 无边界例外（无 protect_core 特例）。
-/

import Formal.TrendTrichotomy
import Formal.CenterTrichotomy
import Formal.RecursiveConstruction
import Formal.BSPLabels

namespace Formal.RStarNonSpecial

open Formal.TrendTrichotomy
open Formal.CenterTrichotomy (Center MoveOutcome)
open Formal.RecursiveConstruction (Move classifyMove CentersDerivedFrom MovesComposedFrom)

/--
  级别塔的一层。关键（codex 审计发现8/9）：**没有 `is_top` / `has_higher` 字段**——
  顶层性不是层的内禀属性。

  ★codex R4：携带本层 `moves`（已结算次级别走势）+ `centers`（本层中枢）——
  本层走势类型 `outcome` **由本层 centers 计算**（`classifyMove`），不继承下层 kind。
  本层 `bsp` 是本层端点标签集（不从下层字段复制）。这使生成关系忠实（codex R4 方案 B）。
-/
structure TowerLevel where
  /-- 本层级别索引（递归深度）。 -/
  levelIdx : Nat
  /-- 本层走势（本层级别的走势，构造上层中枢的原料）。 -/
  moves : List Move
  /-- 本层中枢序列（由 moves 生成，见 GeneratedStep）。 -/
  centers : List Center
  /-- 本层端点买卖点标签集。 -/
  bsp : Formal.BSPLabels.BSPLabelSet
  /-- ★本层标签集合法（codex R5：避免 malformed label set 进入 operate）。 -/
  bsp_wf : Formal.BSPLabels.WellFormedBSPLabelSet bsp

namespace TowerLevel
/-- ★本层走势裁决由本层中枢计算（codex R4：不继承下层 kind）。 -/
def outcome (lvl : TowerLevel) : MoveOutcome := classifyMove lvl.centers
end TowerLevel

/--
  ★当前层可继续构造下一层（codex R3#8 + R4：自然终止判据，level_recursion.md:249）。
  本层已结算走势数 ≥ 3 才能向上构造更高级别中枢。这是 **termination soundness**
  （codex R4：可保留作终止判据，但不单独作 generation soundness）。
-/
def CanBuildNext (lvl : TowerLevel) : Prop := lvl.moves.length ≥ 3

/--
  ★层间生成步（codex R4 方案 B：强关系 contract，kind/bsp 不继承）。

  `GeneratedStep cur next` ⟺
  - `CanBuildNext cur`（本层有足够走势向上构造）；
  - ★下一层中枢由本层走势 **生成**（`CentersDerivedFrom cur.moves next.centers`，
    沿用 RecursiveConstruction 的 witness+顺序 soundness contract）；
  - ★下一层走势是本层走势的封装序列（`next.moves = cur.moves`——封装恒等式
    Move(k)≡Level-(k+1) 笔的最小关系：上一层走势成为下一层构造的原料）。

  关键（codex R4）：下一层 `outcome` **不出现在 GeneratedStep 里**——它由
  `next.centers` 经 `classifyMove` **自行计算**（`TowerLevel.outcome`），不继承 `cur`。
  `next.bsp` 也不从 `cur.bsp` 复制（无 inheritance 约束）。伪造的 next.centers
  （与 cur.moves 无生成关系）不满足 `CentersDerivedFrom`。
-/
def GeneratedStep (cur next : TowerLevel) : Prop :=
  CanBuildNext cur
  ∧ next.levelIdx = cur.levelIdx + 1
  ∧ CentersDerivedFrom cur.moves next.centers
  ∧ MovesComposedFrom cur.moves cur.levelIdx next.moves

/--
  ★有效级别塔（终余代数 + 生成约束，codex 审计 R1#8 + R2#8）。

  不是任意 `Nat → TowerLevel`，而是带 **生成约束** 的有效塔：
  - `level k` 给出第 k 层（向上无限可读 = 终余代数 unfold）。
  - `terminated k`：第 k 层是否因 `settled moves < 3` **自然终止**（level_recursion.md:249）——
    缠论级别上界来源（数据涌现），非结构上限。
  - `termination_absorbing`：终止吸收态（一旦终止，更高层保持终止）。
  - ★`valid_step`（R2#8 生成约束）：若第 k 层未终止，则它可构造下一层（CanBuildNext）
    且第 k+1 层由第 k 层生成（GeneratedStep）。关死"k+1 层未必由 k 层构造"漏洞。
  - ★`terminated_iff`（R2#8）：终止 ⟺ 不可构造下一层（terminated k ↔ ¬CanBuildNext）。
-/
structure ValidTower where
  level : Nat → TowerLevel
  terminated : Nat → Bool
  termination_absorbing : ∀ k, terminated k = true → terminated (k + 1) = true
  /-- ★生成约束：未终止 ⟹ 可构造下一层 ∧ 下一层由本层生成（codex R2#8）。 -/
  valid_step : ∀ k, terminated k = false →
    CanBuildNext (level k) ∧ GeneratedStep (level k) (level (k + 1))
  /-- ★终止 ⟺ 不可构造下一层（codex R2#8：terminated 不是独立标志，由 CanBuildNext 决定）。 -/
  terminated_iff : ∀ k, terminated k = true ↔ ¬ CanBuildNext (level k)

/--
  ★r* 非特殊·操作语义不可观察顶层性（codex 审计发现9，603 §四）。

  操作函数 `operate` 只依赖一层的本征数据 (kind, settled, bsp)——**没有"是否顶层 /
  是否有更高级别"参数**。这形式化"r* 真顶减仓"与 Move(ℓ<r*) 用同一规则，无 protect_core
  的 `if has_higher_level` 特判。

  操作输出 = (走势类型, 操作是否有效)；有效性由端点标签集非空（升跌完备性）决定。
-/
def operate (lvl : TowerLevel) : MoveOutcome × Bool :=
  (lvl.outcome, lvl.bsp.labels ≠ [])

/--
  ★操作 extensionality（codex 审计发现9）：两层若 (kind, settled, bsp) 相同则 operate 相同。

  这是"operate 只观察本征数据"的精确表述——operate **不可能** 区分"顶层"与"非顶层"，
  因为 TowerLevel 根本没有顶层性字段。r* 与任意级别用同一 operate。
-/
theorem operate_blind_to_topness (a b : TowerLevel)
    (hk : a.outcome = b.outcome) (hb : a.bsp = b.bsp) :
    operate a = operate b := by
  unfold operate
  rw [hk, hb]

/--
  ★扩展 tower 不改变既有层的操作（codex 审计发现8：r* 非特殊的核心命题）。

  给一个有效塔，在任意更高层继续 unfold（得到新塔 `ext`，只要它在前缀上与原塔一致），
  则原塔每个已有层 `k` 的 operate 结果 **不变**。这形式化"r* 不是特殊的顶"——
  发现更高级别（tower 扩展）不会回头改变已涌现层的操作，顶部无边界例外。
-/
theorem extension_preserves_operate (t ext : ValidTower)
    (k : Nat) (h_prefix : ext.level k = t.level k) :
    operate (ext.level k) = operate (t.level k) := by
  rw [h_prefix]

/--
  ★r* 同构造子（L0，codex Q2 PASS）：任意级别（含 r* 最深层）的走势类型属三构造子之一。
  与任意其他级别完全同构——没有"r* 用特殊构造子"分支。
-/
theorem rstar_same_constructors (t : ValidTower) (k : Nat) :
    (t.level k).outcome = MoveOutcome.trend Direction.up
    ∨ (t.level k).outcome = MoveOutcome.trend Direction.down
    ∨ (t.level k).outcome = MoveOutcome.consolidation
    ∨ (t.level k).outcome = MoveOutcome.higherCenterCandidate :=
  Formal.RecursiveConstruction.outcome_total (t.level k).centers

/--
  ★级别上界由自然终止决定，非结构上限（zhongshu.md:18 + 601 离散二分消除）。

  级别是否有上界由 `terminated`（settled moves < 3 数据涌现）决定，且终止是吸收态——
  这不是"Move 构造子的离散字段 has_higher∈{有,无}"，是终余代数的 unfold 何时自然停。
  形式化：一旦某层终止，所有更高层终止——级别上界是数据的连续涌现边界，非结构离散二分。
-/
theorem termination_is_upward_closed (t : ValidTower) (k j : Nat)
    (hk : t.terminated k = true) (hj : k ≤ j) :
    t.terminated j = true := by
  induction j with
  | zero =>
      have : k = 0 := Nat.le_zero.mp hj
      rw [this] at hk; exact hk
  | succ n ih =>
      rcases Nat.lt_or_ge k (n + 1) with hlt | hge
      · exact t.termination_absorbing n (ih (Nat.le_of_lt_succ hlt))
      · -- k ≥ n+1 且 k ≤ n+1 ⟹ k = n+1
        have : k = n + 1 := Nat.le_antisymm hj hge
        rw [this] at hk; exact hk

/--
  ★操作有效性由端点标签集非空保证（支撑 597 无边界例外）。
  任意级别（含 r*）operate 的有效位 = 端点标签集非空——由升跌完备性 totality（010）
  保证为真，不引入 protect_core 分支。r* 与任意级别一致有效。
-/
theorem operate_valid_uniform (t : ValidTower) (k : Nat) :
    (operate (t.level k)).2 = true := by
  unfold operate
  exact decide_eq_true (t.level k).bsp.nonempty

/--
  ★未终止层可向上 unfold（codex R2#8 生成约束的使用）：

  对任意未终止层 k，它可构造下一层（CanBuildNext）且下一层由它生成（GeneratedStep）。
  这形式化"r* 不是结构上限"——只要未自然终止（settled moves ≥ 3），递归就能继续 unfold，
  没有"最后一层"的结构性禁止。r* 是数据涌现的当前最深层，非类型上的特殊顶。
-/
theorem active_level_unfolds (t : ValidTower) (k : Nat) (h : t.terminated k = false) :
    CanBuildNext (t.level k) ∧ GeneratedStep (t.level k) (t.level (k + 1)) :=
  t.valid_step k h

/--
  ★操作限制在 active 层（codex R2#8 caveat）：

  对一个 active（未终止）层，其操作既由 (kind, bsp) 决定（不查顶层性，`operate_blind_to_topness`），
  又因该层可继续 unfold（`active_level_unfolds`）而无需"是否到顶"判断——r* 非特殊的完整表述：
  active 层的操作语义与"是否为最深层"完全无关。
-/
theorem active_operate_independent_of_top (t : ValidTower) (k : Nat)
    (h : t.terminated k = false) :
    (operate (t.level k)).2 = true ∧ CanBuildNext (t.level k) :=
  ⟨operate_valid_uniform t k, (t.valid_step k h).1⟩

/--
  ★生成步蕴含中枢 soundness（codex R4 方案 B：next.centers 由 cur.moves 生成）。
  若 `GeneratedStep cur next`，则 `next.centers` 满足 `CentersDerivedFrom cur.moves`——
  伪造的 next.centers（与 cur.moves 无 witness+顺序生成关系）不满足。关死"随意填下层"漏洞。
-/
theorem generatedStep_centers_sound (cur next : TowerLevel)
    (h : GeneratedStep cur next) :
    CentersDerivedFrom cur.moves next.centers := h.2.2.1

/--
  ★生成步级别递进（codex R5：next 级别 = cur 级别 +1，非塌缩）。
  关死"每层复制同一 moves"——上一层被封装为更高级别，级别索引严格 +1。
-/
theorem generatedStep_level_up (cur next : TowerLevel)
    (h : GeneratedStep cur next) :
    next.levelIdx = cur.levelIdx + 1 := h.2.1

/--
  ★生成步上级走势是封装对象（codex R5：MovesComposedFrom，非原样复制）。
  `next.moves` 由 `cur.moves` compose 而成（非空、良构、级别+1、每个是 compose 且 subs⊆cur.moves）——
  关死 codex R5"next.moves = cur.moves 原样复制"漏洞。tower 层级语义不塌缩。
-/
theorem generatedStep_moves_composed (cur next : TowerLevel)
    (h : GeneratedStep cur next) :
    MovesComposedFrom cur.moves cur.levelIdx next.moves := h.2.2.2

/-- 生成步上级走势非空（封装产生新走势）。 -/
theorem generatedStep_next_nonempty (cur next : TowerLevel)
    (h : GeneratedStep cur next) : next.moves ≠ [] :=
  (generatedStep_moves_composed cur next h).1

/--
  ★生成步消费约束（codex R6 核心：每上级走势吃 ≥3 下级走势，有限下级不能无限封装）。

  `next.moves.length * 3 ≤ cur.moves.length`——上级走势数 × 3 ≤ 下级走势数。这关死 codex R6
  反例（3 个相同 compose 重复消费同 3 段）：3 个 upper move 需 ≥9 个 lower move，3 个 lower
  无法满足。保证级别递归逐层减少、自然收敛终止（level_recursion.md:249）。
-/
theorem generatedStep_consumption_bound (cur next : TowerLevel)
    (h : GeneratedStep cur next) :
    next.moves.length * 3 ≤ cur.moves.length :=
  (generatedStep_moves_composed cur next h).2.2.1

/--
  ★级别递归严格收敛（codex R6 反例的根本关闭）：上级走势数 **严格小于** 下级走势数。

  因 `CanBuildNext cur`（cur.moves.length ≥ 3）+ 消费约束（next.moves.length * 3 ≤ cur.moves.length），
  得 `next.moves.length < cur.moves.length`。走势数沿塔严格递减 ⟹ 不能无限 build ⟹ 自然终止
  （level_recursion.md:249）。这关死 codex R6"3 段重复封装成 3 段无限递归"的伪 tower——
  级别语义不塌缩，r* 是数据涌现的有限最深层。
-/
theorem generatedStep_strictly_decreases (cur next : TowerLevel)
    (h : GeneratedStep cur next) :
    next.moves.length < cur.moves.length := by
  have hcan : CanBuildNext cur := h.1
  have hbound : next.moves.length * 3 ≤ cur.moves.length :=
    generatedStep_consumption_bound cur next h
  unfold CanBuildNext at hcan
  omega

/--
  ★下一层裁决由下一层中枢自算（codex R4：不继承 cur）。
  生成步成立时，`next.outcome` = `classifyMove next.centers`（其本征定义），**与 cur.outcome 无关**——
  父级走势类型由父级中枢分类，不继承子级（codex R4 问题一的修正见证）。
-/
theorem next_outcome_self_classified (cur next : TowerLevel)
    (_ : GeneratedStep cur next) :
    next.outcome = classifyMove next.centers := rfl

/--
  ★不可构造时无生成步（codex R3#8 + R4：自然终止 ⟹ 无下一层）。
  若当前层走势不足 3（CanBuildNext 假），则不存在 next 使 GeneratedStep 成立——
  级别在此自然终止（level_recursion.md:249），非结构截断。
-/
theorem no_step_when_terminated (cur next : TowerLevel)
    (h : ¬ CanBuildNext cur) : ¬ GeneratedStep cur next := by
  unfold GeneratedStep
  intro ⟨hcan, _, _, _⟩
  exact h hcan

end Formal.RStarNonSpecial

exec
/bin/zsh -lc "sed -n '1,260p' formal/Strict/LevelState.lean" in /Users/silencehan/Projects/NewChanlun
 succeeded in 0ms:
/-
  Strict/LevelState.lean — 每级参数化完全分类：R 6 态 + 买卖点 bit-vector + LevelState
  ★cc-levelstate 工位（π_Θ 推导第 3-4 步，RTAS 严格完全分类蜂群）

  ════════════════════════════════════════════════════════════════════════
  本文件的两个独立分类（不可混淆——这是本工位的核心诚实点）
  ════════════════════════════════════════════════════════════════════════

  缠论"每级状态"由两个**结构上不同**的分类拼成，它们的代数性质相反：

  | 分类对象 | 形式 | 互斥穷尽? | 范式 | 依据 |
  |----------|------|-----------|------|------|
  | **R：位置态** 相对最后中枢 Z_ℓ | 6 态 sum type `RLevel` | **是**（Σ𝟙=1） | 互斥三分的状态机精化 | Claim9 三态：within→I；above/below 各按 3B/3S 二分；+⊥ |
  | **E：信号** 买卖点 | bit-vector `BSPVector`={0,1}⁶ | **否**（2/3 类可共存） | 非互斥 subset | BSPLabels twoB_threeB_can_coincide |

  ★★关键区分（编排者明确："买卖点不能强行 6 互斥，2/3 类可共存"）：
  位置态 R 是互斥的（一个点相对一个中枢只能在一个位置），故是 sum type；
  买卖点 E 是非互斥的（一个端点可同时是 2 买与 3 买，V 型反转），故是 bit-vector。
  把 E 也做成 6 互斥类是**错误**——会冒充 BSPLabels 已证否的互斥性
  （`twoB_threeB_can_coincide` / `bsp_not_exclusive_trichotomy`）。bit-vector 是正面形式：
  买卖点是 subset，不是 partition。

  ════════════════════════════════════════════════════════════════════════
  R 6 态 `RLevel = {⊥, I, U⁰, U¹, D⁰, D¹}`（相对最后中枢 Z_ℓ）
  ════════════════════════════════════════════════════════════════════════

  - `bot`（⊥）：无最后中枢（ExistsZ = false）——尚未形成中枢，位置无定义。
  - `inside`（I）：当下在最后中枢核心区间 [ZD, ZG] 内（Claim9 within，闭区间）。
  - `aboveNo3B`（U⁰）：当下在 Z_ℓ 上方（Claim9 above，p > ZG）且**无**第三类买点。
  - `aboveB3`（U¹）：当下在 Z_ℓ 上方且**有**第三类买点（3B 成立，离开中枢回试不入）。
  - `belowNo3S`（D⁰）：当下在 Z_ℓ 下方（Claim9 below，p < ZD）且**无**第三类卖点。
  - `belowS3`（D¹）：当下在 Z_ℓ 下方且**有**第三类卖点（3S 成立）。

  这是 Claim9 中枢位置三态（below/within/above）的**状态机精化**（非统一乘积）：
  - within 直接 ⟶ I（**不**按第三类拆分）；
  - above 按第三类买点有/无裂为 U⁰/U¹（2 态）；
  - below 按第三类卖点有/无裂为 D⁰/D¹（2 态）；
  - 再并入 ⊥（无中枢）。计数：1(within) + 2(above) + 2(below) + 1(⊥) = 6 态。
  ★注意非 3×2+1：within 不拆，只有 above/below 各按 Bool 二分，故是「逐态精化」非「全三态统一乘 Bool」。

  ════════════════════════════════════════════════════════════════════════
  诚实标注（gatekeeper）
  ════════════════════════════════════════════════════════════════════════

  ★标签：**StructurePartitionOnly**（R 6 态是结构事实——给定最后中枢与第三类事件后，
        位置态的 fiber 划分是 Θ-参数化全函数的原像划分，**非**语义双射；
        E bit-vector 是 subset 编码，非分类）。

  ★Θ-参数化运行分类（codex 研究结论，ClassificationFamily.lean 同构）：
  R 6 态**不**由缠论结构公理单独导出——它依赖两个 Θ 参数：
  - **Θ_parse**：「最后中枢 Z_ℓ」的定义（哪个中枢是"最后"、边界 ZD/ZG 在哪——
    `Center` 已固化 ZD/ZG，但"最后中枢"的选取是 Θ_parse 的 canonical 选择器）。
  - **Θ_signal**：第三类买点/卖点事件判据（3B/3S 在哪触发——离开中枢回试不入的阈值）。
  缠论公理给「中枢位置三态」（Claim9，无参数 L0），但「相对**最后**中枢 + 带**第三类信号**
  的 6 态」需要 Θ_parse + Θ_signal。给定 (Z_ℓ, B₃判定, S₃判定) 后，`rlevelOf` 是全函数，
  其 6 态划分自动互斥穷尽（`rlevel_exhaustive_exclusive`）。**给定** Θ 后 L0；Θ 本身非缠论可导。

  认识论等级：全部 **L0**（给定 Θ 后从 Claim9 已结算定义 + Bool 穷尽推导，不依赖经验数据）。
  formalization-validity-domain：Lean build 通过 = 给定 Θ 后逻辑正确（L0），
  **不**膨胀为「6 态是缠论无参数完全分类」（前件是 Θ）。

  ★依赖方向（单向 Strict→{Formal, Phase2}，无环）：
  本文件 import `Classification`（内核）+ `Claim9_CenterPosition`（位置三态）
  + `Formal.BSPLabels`（买卖点标签）。三者均**不** import Strict。

  谱系：598（真完全分类元判据）→ 603（递归 vs 轴范式）→ 615（Layer1⊊Layer2 概念分离）
       → 616（完全分类⊬唯一策略）→ 617（C 与 π 都 Θ-参数化）。
       Claim9（中枢位置三态）+ BSPLabels（买卖点非互斥标签集，twoB_threeB_can_coincide）。
-/

import Strict.Classification   -- Strict 内核（命名空间 = Strict）：Classifies / SemanticQuotient
import Claim9_CenterPosition    -- 分类B：中枢位置三态（命名空间 = Formal.CenterPosition）
import Formal.BSPLabels         -- 买卖点标签集（命名空间 = Formal.BSPLabels）

namespace Strict.LevelState

open Formal.CenterTrichotomy (Center)
open Formal.CenterPosition
open Formal.BSPLabels

/-! ════════════════════════════════════════════════════════════════════════
    § Part 1 — R 6 态：相对最后中枢 Z_ℓ 的位置状态机（互斥穷尽）
    ════════════════════════════════════════════════════════════════════════ -/

/--
  ★R 6 态（相对最后中枢 Z_ℓ 的位置态，互斥 sum type）。

  Claim9 中枢位置三态（below/within/above）的状态机精化：
  - above 按第三类买点有/无 ⟶ U⁰(aboveNo3B) / U¹(aboveB3)；
  - below 按第三类卖点有/无 ⟶ D⁰(belowNo3S) / D¹(belowS3)；
  - within ⟶ I(inside)；并入 ⊥(bot, 无中枢)。

  ★互斥性来源（非态计数）：位置（Claim9 三态互斥）、第三类事件（确定 Bool）、
  中枢存在性（确定 some/none）——任两不同态的判据在某一维度冲突，故 6 态互斥
  （见 `rlevel_exhaustive_exclusive`）。
-/
inductive RLevel where
  | bot         -- ⊥：无最后中枢（ExistsZ = false）
  | inside      -- I：当下在 Z_ℓ 核心区间 [ZD, ZG] 内（Claim9 within）
  | aboveNo3B   -- U⁰：Z_ℓ 上方（p > ZG）且无第三类买点
  | aboveB3     -- U¹：Z_ℓ 上方且有第三类买点（3B 成立）
  | belowNo3S   -- D⁰：Z_ℓ 下方（p < ZD）且无第三类卖点
  | belowS3     -- D¹：Z_ℓ 下方且有第三类卖点（3S 成立）
deriving DecidableEq, Repr

open RLevel

/--
  ★最后中枢上下文（Θ-参数化运行输入，L0）。

  R 6 态的判定不只看价格点——它需要三个 Θ-参数化输入：
  - `lastCenter : Option Center`：最后中枢 Z_ℓ（`none` = 无中枢 ⟶ ⊥；`some c` = 有中枢 c）。
    「哪个中枢是最后」是 Θ_parse 选择器的产物（缠论不无参数地钉死"最后"）。
  - `price : Int`：当下价格点 p（相对 Z_ℓ 核心区间定位）。
  - `b3 : Bool`：第三类买点事件是否成立（Θ_signal 判据：离开中枢上破后回试不入）。
  - `s3 : Bool`：第三类卖点事件是否成立（Θ_signal 判据：离开中枢下破后回抽不入）。

  ★这是 `ClassificationFamily.ClassifierFamily` 的 `X`（运行对象）在本级位置态上的实例：
  给定 Θ（= 如何选最后中枢 + 如何判第三类信号）后，从 `RContext` 到 `RLevel` 是全函数。
-/
structure RContext where
  lastCenter : Option Center
  price : Int
  b3 : Bool
  s3 : Bool

namespace RContext

/-- 是否存在最后中枢（⊥ 态的判据：无中枢 ⟶ ⊥）。 -/
def ExistsZ (ctx : RContext) : Prop := ctx.lastCenter.isSome = true

instance (ctx : RContext) : Decidable ctx.ExistsZ := by
  unfold ExistsZ; infer_instance

end RContext

open RContext

/--
  ★R 6 态判定函数（给定 Θ 后的全函数 C_Θ，L0）。

  `rlevelOf ctx` 把运行上下文映射到 6 态之一：
  - 无中枢 ⟶ `bot`；
  - 有中枢 c 时，按 Claim9 `classify c p` 三态分流：
    · within ⟶ `inside`；
    · above ⟶ `b3` 真则 `aboveB3` 否则 `aboveNo3B`；
    · below ⟶ `s3` 真则 `belowS3` 否则 `belowNo3S`。

  ★全函数性：对每个 `ctx` 返回确定的 `RLevel`（Lean 全函数，编码"完全分类每级位置都有态"）。
-/
def rlevelOf (ctx : RContext) : RLevel :=
  match ctx.lastCenter with
  | none => bot
  | some c =>
    match Formal.CenterPosition.classify c ctx.price with
    | RelativePosition.within => inside
    | RelativePosition.above => if ctx.b3 then aboveB3 else aboveNo3B
    | RelativePosition.below => if ctx.s3 then belowS3 else belowNo3S

/-! ### 6 态谓词（显式刻画每态的判据，供互斥穷尽定理引用） -/

/-- ⊥ 谓词：无最后中枢。 -/
def IsBot (ctx : RContext) : Prop := ctx.lastCenter = none
/-- I 谓词：有中枢且当下在核心区间内（Claim9 within）。 -/
def IsInside (ctx : RContext) : Prop :=
  ∃ c, ctx.lastCenter = some c ∧ Formal.CenterPosition.IsWithin c ctx.price
/-- U⁰ 谓词：有中枢、当下在上方、无第三类买点。 -/
def IsAboveNo3B (ctx : RContext) : Prop :=
  ∃ c, ctx.lastCenter = some c ∧ Formal.CenterPosition.IsAbove c ctx.price ∧ ctx.b3 = false
/-- U¹ 谓词：有中枢、当下在上方、有第三类买点。 -/
def IsAboveB3 (ctx : RContext) : Prop :=
  ∃ c, ctx.lastCenter = some c ∧ Formal.CenterPosition.IsAbove c ctx.price ∧ ctx.b3 = true
/-- D⁰ 谓词：有中枢、当下在下方、无第三类卖点。 -/
def IsBelowNo3S (ctx : RContext) : Prop :=
  ∃ c, ctx.lastCenter = some c ∧ Formal.CenterPosition.IsBelow c ctx.price ∧ ctx.s3 = false
/-- D¹ 谓词：有中枢、当下在下方、有第三类卖点。 -/
def IsBelowS3 (ctx : RContext) : Prop :=
  ∃ c, ctx.lastCenter = some c ∧ Formal.CenterPosition.IsBelow c ctx.price ∧ ctx.s3 = true

/-! ### rlevelOf 忠实于 6 态谓词（每态判据与判定函数一致） -/

/-- 辅助：classify 三态忠实（Claim9 已证，这里桥接到本文件谓词）。 -/
private theorem classify_within_iff (c : Center) (p : Int) :
    Formal.CenterPosition.classify c p = RelativePosition.within
      ↔ Formal.CenterPosition.IsWithin c p :=
  Formal.CenterPosition.classify_eq_within c p

private theorem classify_above_iff (c : Center) (p : Int) :
    Formal.CenterPosition.classify c p = RelativePosition.above
      ↔ Formal.CenterPosition.IsAbove c p :=
  Formal.CenterPosition.classify_eq_above c p

private theorem classify_below_iff (c : Center) (p : Int) :
    Formal.CenterPosition.classify c p = RelativePosition.below
      ↔ Formal.CenterPosition.IsBelow c p :=
  Formal.CenterPosition.classify_eq_below c p

/-- `rlevelOf = bot ↔ IsBot`。 -/
theorem rlevelOf_eq_bot (ctx : RContext) : rlevelOf ctx = bot ↔ IsBot ctx := by
  unfold rlevelOf IsBot
  cases h : ctx.lastCenter with
  | none => simp
  | some c =>
    cases hcl : Formal.CenterPosition.classify c ctx.price <;>
      cases hb : ctx.b3 <;> cases hs : ctx.s3 <;> simp_all

/-- `rlevelOf = inside ↔ IsInside`。 -/
theorem rlevelOf_eq_inside (ctx : RContext) : rlevelOf ctx = inside ↔ IsInside ctx := by
  unfold rlevelOf IsInside
  cases h : ctx.lastCenter with
  | none => simp
  | some c =>
    simp only [Option.some.injEq, exists_eq_left']
    rw [← classify_within_iff c ctx.price]
    cases hcl : Formal.CenterPosition.classify c ctx.price <;>
      cases hb : ctx.b3 <;> cases hs : ctx.s3 <;> simp_all

/-- `rlevelOf = aboveB3 ↔ IsAboveB3`。 -/
theorem rlevelOf_eq_aboveB3 (ctx : RContext) : rlevelOf ctx = aboveB3 ↔ IsAboveB3 ctx := by
  unfold rlevelOf IsAboveB3
  cases h : ctx.lastCenter with
  | none => simp
  | some c =>
    simp only [Option.some.injEq, exists_eq_left']
    rw [← classify_above_iff c ctx.price]
    cases hcl : Formal.CenterPosition.classify c ctx.price <;>
      cases hb : ctx.b3 <;> cases hs : ctx.s3 <;> simp_all

/-- `rlevelOf = aboveNo3B ↔ IsAboveNo3B`。 -/
theorem rlevelOf_eq_aboveNo3B (ctx : RContext) :
    rlevelOf ctx = aboveNo3B ↔ IsAboveNo3B ctx := by
  unfold rlevelOf IsAboveNo3B
  cases h : ctx.lastCenter with
  | none => simp
  | some c =>
    simp only [Option.some.injEq, exists_eq_left']
    rw [← classify_above_iff c ctx.price]
    cases hcl : Formal.CenterPosition.classify c ctx.price <;>
      cases hb : ctx.b3 <;> cases hs : ctx.s3 <;> simp_all

/-- `rlevelOf = belowS3 ↔ IsBelowS3`。 -/
theorem rlevelOf_eq_belowS3 (ctx : RContext) : rlevelOf ctx = belowS3 ↔ IsBelowS3 ctx := by
  unfold rlevelOf IsBelowS3
  cases h : ctx.lastCenter with
  | none => simp
  | some c =>
    simp only [Option.some.injEq, exists_eq_left']
    rw [← classify_below_iff c ctx.price]
    cases hcl : Formal.CenterPosition.classify c ctx.price <;>
      cases hb : ctx.b3 <;> cases hs : ctx.s3 <;> simp_all

/-- `rlevelOf = belowNo3S ↔ IsBelowNo3S`。 -/
theorem rlevelOf_eq_belowNo3S (ctx : RContext) :
    rlevelOf ctx = belowNo3S ↔ IsBelowNo3S ctx := by
  unfold rlevelOf IsBelowNo3S
  cases h : ctx.lastCenter with
  | none => simp
  | some c =>
    simp only [Option.some.injEq, exists_eq_left']
    rw [← classify_below_iff c ctx.price]

exec
/bin/zsh -lc 'rg -n "MovesComposedFrom|GeneratedStep|CanBuildNext|terminated|UpperMoveWitness|UpperMoveSound|disjoint|window" formal/Formal formal/Strict -S' in /Users/silencehan/Projects/NewChanlun
 succeeded in 0ms:
formal/Formal/TrendTrichotomy.lean:26:  `trend_dichotomy_total` / `trend_consolidation_disjoint` 是 **P 侧（标签集）** 的覆盖+互斥，
formal/Formal/TrendTrichotomy.lean:107:theorem trend_consolidation_disjoint (t : TrendKind) :
formal/Formal/RStarNonSpecial.lean:32:open Formal.RecursiveConstruction (Move classifyMove CentersDerivedFrom MovesComposedFrom)
formal/Formal/RStarNonSpecial.lean:47:  /-- 本层中枢序列（由 moves 生成，见 GeneratedStep）。 -/
formal/Formal/RStarNonSpecial.lean:64:def CanBuildNext (lvl : TowerLevel) : Prop := lvl.moves.length ≥ 3
formal/Formal/RStarNonSpecial.lean:69:  `GeneratedStep cur next` ⟺
formal/Formal/RStarNonSpecial.lean:70:  - `CanBuildNext cur`（本层有足够走势向上构造）；
formal/Formal/RStarNonSpecial.lean:76:  关键（codex R4）：下一层 `outcome` **不出现在 GeneratedStep 里**——它由
formal/Formal/RStarNonSpecial.lean:81:def GeneratedStep (cur next : TowerLevel) : Prop :=
formal/Formal/RStarNonSpecial.lean:82:  CanBuildNext cur
formal/Formal/RStarNonSpecial.lean:85:  ∧ MovesComposedFrom cur.moves cur.levelIdx next.moves
formal/Formal/RStarNonSpecial.lean:92:  - `terminated k`：第 k 层是否因 `settled moves < 3` **自然终止**（level_recursion.md:249）——
formal/Formal/RStarNonSpecial.lean:95:  - ★`valid_step`（R2#8 生成约束）：若第 k 层未终止，则它可构造下一层（CanBuildNext）
formal/Formal/RStarNonSpecial.lean:96:    且第 k+1 层由第 k 层生成（GeneratedStep）。关死"k+1 层未必由 k 层构造"漏洞。
formal/Formal/RStarNonSpecial.lean:97:  - ★`terminated_iff`（R2#8）：终止 ⟺ 不可构造下一层（terminated k ↔ ¬CanBuildNext）。
formal/Formal/RStarNonSpecial.lean:101:  terminated : Nat → Bool
formal/Formal/RStarNonSpecial.lean:102:  termination_absorbing : ∀ k, terminated k = true → terminated (k + 1) = true
formal/Formal/RStarNonSpecial.lean:104:  valid_step : ∀ k, terminated k = false →
formal/Formal/RStarNonSpecial.lean:105:    CanBuildNext (level k) ∧ GeneratedStep (level k) (level (k + 1))
formal/Formal/RStarNonSpecial.lean:106:  /-- ★终止 ⟺ 不可构造下一层（codex R2#8：terminated 不是独立标志，由 CanBuildNext 决定）。 -/
formal/Formal/RStarNonSpecial.lean:107:  terminated_iff : ∀ k, terminated k = true ↔ ¬ CanBuildNext (level k)
formal/Formal/RStarNonSpecial.lean:159:  级别是否有上界由 `terminated`（settled moves < 3 数据涌现）决定，且终止是吸收态——
formal/Formal/RStarNonSpecial.lean:164:    (hk : t.terminated k = true) (hj : k ≤ j) :
formal/Formal/RStarNonSpecial.lean:165:    t.terminated j = true := by
formal/Formal/RStarNonSpecial.lean:190:  对任意未终止层 k，它可构造下一层（CanBuildNext）且下一层由它生成（GeneratedStep）。
formal/Formal/RStarNonSpecial.lean:194:theorem active_level_unfolds (t : ValidTower) (k : Nat) (h : t.terminated k = false) :
formal/Formal/RStarNonSpecial.lean:195:    CanBuildNext (t.level k) ∧ GeneratedStep (t.level k) (t.level (k + 1)) :=
formal/Formal/RStarNonSpecial.lean:206:    (h : t.terminated k = false) :
formal/Formal/RStarNonSpecial.lean:207:    (operate (t.level k)).2 = true ∧ CanBuildNext (t.level k) :=
formal/Formal/RStarNonSpecial.lean:212:  若 `GeneratedStep cur next`，则 `next.centers` 满足 `CentersDerivedFrom cur.moves`——
formal/Formal/RStarNonSpecial.lean:216:    (h : GeneratedStep cur next) :
formal/Formal/RStarNonSpecial.lean:224:    (h : GeneratedStep cur next) :
formal/Formal/RStarNonSpecial.lean:228:  ★生成步上级走势是封装对象（codex R5：MovesComposedFrom，非原样复制）。
formal/Formal/RStarNonSpecial.lean:233:    (h : GeneratedStep cur next) :
formal/Formal/RStarNonSpecial.lean:234:    MovesComposedFrom cur.moves cur.levelIdx next.moves := h.2.2.2
formal/Formal/RStarNonSpecial.lean:238:    (h : GeneratedStep cur next) : next.moves ≠ [] :=
formal/Formal/RStarNonSpecial.lean:249:    (h : GeneratedStep cur next) :
formal/Formal/RStarNonSpecial.lean:256:  因 `CanBuildNext cur`（cur.moves.length ≥ 3）+ 消费约束（next.moves.length * 3 ≤ cur.moves.length），
formal/Formal/RStarNonSpecial.lean:262:    (h : GeneratedStep cur next) :
formal/Formal/RStarNonSpecial.lean:264:  have hcan : CanBuildNext cur := h.1
formal/Formal/RStarNonSpecial.lean:267:  unfold CanBuildNext at hcan
formal/Formal/RStarNonSpecial.lean:276:    (_ : GeneratedStep cur next) :
formal/Formal/RStarNonSpecial.lean:281:  若当前层走势不足 3（CanBuildNext 假），则不存在 next 使 GeneratedStep 成立——
formal/Formal/RStarNonSpecial.lean:284:theorem no_step_when_terminated (cur next : TowerLevel)
formal/Formal/RStarNonSpecial.lean:285:    (h : ¬ CanBuildNext cur) : ¬ GeneratedStep cur next := by
formal/Formal/RStarNonSpecial.lean:286:  unfold GeneratedStep
formal/Formal/ConstitutiveLadder.lean:7:  范式（沿 Phase 1 ordered window witness 标准）：构成性阶梯不是"特征枚举"，是 **构成性递归
formal/Formal/ConstitutiveLadder.lean:14:  - 复核2（ordered window）：层间关系不能是 membership——分型=相邻3根 K 窗口、笔=相邻异性分型、
formal/Formal/ConstitutiveLadder.lean:15:    线段=连续覆盖笔（formal_axioms I3）。已升级为 ordered window witness（沿 Phase 1 标准）。
formal/Formal/ConstitutiveLadder.lean:144:/-! ## 阶梯层间构成性关系（★ordered window witness，非 membership，复核2 修正） -/
formal/Formal/ConstitutiveLadder.lean:147:  ★分型由 K 线相邻三根窗口构成（复核2：ordered window，非 membership，fenxing 定义）。
formal/Formal/ConstitutiveLadder.lean:160:  ★笔由相邻异性分型构成（复核2：ordered window + 桥接 wellFormed）。
formal/Formal/ConstitutiveLadder.lean:200:theorem fractal_window_wellformed (bars : List MergedBar) (f : Fractal)
formal/Formal/ConstitutiveLadder.lean:215:theorem segment_window_contiguous (strokes : List Stroke) (seg : Segment)
formal/Strict/Parse.lean:338:    互斥穷尽（那由 OpenTail.lean 的 `branch_total`/`branch_disjoint` 承载，不在本文件重证）。
formal/Strict/Parse.lean:510:       （后者需 `Strict.Classifies` 的 total/sound/complete/disjoint/realized 五项，本文件不证）。
formal/Strict/OpenTail.lean:19:     填入标准内核结构，**证明三项分支义务**（branch_total / branch_disjoint / branch_sound）。
formal/Strict/OpenTail.lean:196:/-! ## 三项分支证明义务（branch_total / branch_disjoint / branch_sound） -/
formal/Strict/OpenTail.lean:209:  ★分支互斥 `branch_disjoint`（标准第5部分，L0）：一个延伸不落入两个不同分支。
formal/Strict/OpenTail.lean:214:theorem branch_disjoint (h h' : OpenHist) (b₁ b₂ : Branch)
formal/Strict/OpenTail.lean:224:  这保证分支集不会把"非 h 的未来"误纳入（与 total/disjoint 一起构成互斥穷尽分割）。
formal/Strict/OpenTail.lean:239:  OpenTailSystem 结构"——证明义务（branch_total/disjoint/sound）全部机器可检验地满足。
formal/Strict/OpenTail.lean:246:  branch_disjoint := branch_disjoint
formal/Formal/BSPLabels.lean:369:theorem first_second_disjoint (x : EndpointSituation) :
formal/Formal/BSPLabels.lean:378:theorem first_third_disjoint (x : EndpointSituation) :
formal/Strict/Center.lean:72:  - disjoint：一个点不能同时满足两个不同标签谓词（三 disjoint + core_valid）。
formal/Strict/Center.lean:98:  disjoint := by
formal/Strict/Center.lean:105:          | exact below_within_disjoint x.1 x.2 ⟨hP1, hP2⟩
formal/Strict/Center.lean:106:          | exact below_within_disjoint x.1 x.2 ⟨hP2, hP1⟩
formal/Strict/Center.lean:107:          | exact below_above_disjoint x.1 x.2 ⟨hP1, hP2⟩
formal/Strict/Center.lean:108:          | exact below_above_disjoint x.1 x.2 ⟨hP2, hP1⟩
formal/Strict/Center.lean:109:          | exact within_above_disjoint x.1 x.2 ⟨hP1, hP2⟩
formal/Strict/Center.lean:110:          | exact within_above_disjoint x.1 x.2 ⟨hP2, hP1⟩)
formal/Strict/Center.lean:280:  disjoint := by
formal/Strict/Center.lean:341:  - IsLevelExpansion → classify=expansion（用 disjoint：扩张 ⟹ ¬up ∧ ¬down ⟹ classify=expansion，无条件）
formal/Strict/Center.lean:361:  用 disjoint：IsLevelExpansion ⟹ ¬IsUpContinuation ∧ ¬IsDownContinuation ⟹ classify=expansion。
formal/Strict/Center.lean:367:    up_expansion_disjoint prev next ⟨hup, hexp⟩
formal/Strict/Center.lean:369:    down_expansion_disjoint prev next ⟨hdown, hexp⟩
formal/Strict/Center.lean:383:  - disjoint：up/down/expansion 两两互斥（三 disjoint 定理）。
formal/Strict/Center.lean:403:    · -- classify=down → IsDownContinuation：由 down_imp_classify 的逆不直接，用谓词穷尽 + disjoint
formal/Strict/Center.lean:408:    · -- classify=expansion → IsLevelExpansion：同上用穷尽 + disjoint
formal/Strict/Center.lean:419:  disjoint := by
formal/Strict/Center.lean:425:          | exact up_down_disjoint p.val.1 p.val.2 ⟨hP1, hP2⟩
formal/Strict/Center.lean:426:          | exact up_down_disjoint p.val.1 p.val.2 ⟨hP2, hP1⟩
formal/Strict/Center.lean:427:          | exact up_expansion_disjoint p.val.1 p.val.2 ⟨hP1, hP2⟩
formal/Strict/Center.lean:428:          | exact up_expansion_disjoint p.val.1 p.val.2 ⟨hP2, hP1⟩
formal/Strict/Center.lean:429:          | exact down_expansion_disjoint p.val.1 p.val.2 ⟨hP1, hP2⟩
formal/Strict/Center.lean:430:          | exact down_expansion_disjoint p.val.1 p.val.2 ⟨hP2, hP1⟩)
formal/Formal/RecursiveConstruction.lean:180:structure UpperMoveWitness where
formal/Formal/RecursiveConstruction.lean:186:def windowSlice (lowerMoves : List Move) (w : UpperMoveWitness) : List Move :=
formal/Formal/RecursiveConstruction.lean:190:def UpperMoveSound (lowerMoves : List Move) (lowerLevel : Nat)
formal/Formal/RecursiveConstruction.lean:191:    (m : Move) (w : UpperMoveWitness) : Prop :=
formal/Formal/RecursiveConstruction.lean:194:  ∧ m = Move.compose (windowSlice lowerMoves w) w.centers (lowerLevel + 1)
formal/Formal/RecursiveConstruction.lean:205:  - witness 列表与 upperMoves 等长，逐位 `UpperMoveSound`（连续窗口、级别+1、compose）；
formal/Formal/RecursiveConstruction.lean:210:def MovesComposedFrom (lowerMoves : List Move) (lowerLevel : Nat)
formal/Formal/RecursiveConstruction.lean:215:  ∧ ∃ ws : List UpperMoveWitness,
formal/Formal/RecursiveConstruction.lean:217:      ∧ StrictlyIncreasing (ws.map UpperMoveWitness.start)
formal/Formal/RecursiveConstruction.lean:219:          (fun m w => UpperMoveSound lowerMoves lowerLevel m w) upperMoves ws
formal/Formal/RecursiveConstruction.lean:318:theorem no_window_when_too_short (subs : List Move) (c : Center)
formal/Formal/CenterTrichotomy.lean:152:theorem up_down_disjoint (prev next : Center) :
formal/Formal/CenterTrichotomy.lean:160:theorem up_expansion_disjoint (prev next : Center) :
formal/Formal/CenterTrichotomy.lean:168:theorem down_expansion_disjoint (prev next : Center) :
formal/Formal/CenterTrichotomy.lean:177:theorem continuations_disjoint (prev next : Center) :
formal/Formal/CenterTrichotomy.lean:179:  up_down_disjoint prev next
formal/Strict/Trend.lean:11:  `trend_consolidation_disjoint` 已证 `TrendKind` 三构造子穷尽互斥（L0）。
formal/Strict/Trend.lean:199:    disjoint/realized 五件套。这**不是**重言（谓词不是 `I x = c`，而是「真裁决桥接 = c」），
formal/Strict/Trend.lean:221:  - disjoint：互斥——不可同时满足两类真裁决判据（`outcomeToKind ∘ classifyMove` 是函数，
formal/Strict/Trend.lean:240:  disjoint := by
formal/Formal/DivergenceNesting.lean:493:theorem branches_disjoint {t : OpenTail} (ω : ExtEvent t) :
formal/Strict/BSP.lean:38:        谓词族 {type1↦IsFirst, type2↦IsSecond, type3↦IsThird} 的 `disjoint` 必然失败
formal/Strict/BSP.lean:85:  内核 `Classifies` 要求 `disjoint`：∀ x c₁ c₂, P c₁ x → P c₂ x → c₁ = c₂（互斥/≤1 重叠）。
formal/Strict/BSP.lean:87:  而 `type2 ≠ type3` ⟹ disjoint 必然失败 ⟹ 任何 I 都不能构成 `Classifies`。
formal/Strict/BSP.lean:96:  -- x_2b3b 同时是二买与三买，触发 disjoint 矛盾 type2 = type3
formal/Strict/BSP.lean:99:  have heq : type2 = type3 := hcls.disjoint h2 h3
formal/Strict/BSP.lean:213:  disjoint := by
formal/Strict/BSP.lean:269:  2B/3B 可重合，见 first_second_disjoint / first_third_disjoint / coincidence_iff）：
formal/Strict/BSP.lean:293:  合法端点含 1B ⟹ 不含 2B/3B，见 first_second_disjoint）。
formal/Strict/BSP.lean:334:/-- 合法端点含 1B ⟹ 不含 2B/3B（first_second_disjoint + first_third_disjoint 的合并，L0）。 -/
formal/Strict/BSP.lean:337:  ⟨fun h2 => first_second_disjoint x.val ⟨hf, h2⟩,
formal/Strict/BSP.lean:338:   fun h3 => first_third_disjoint x.val ⟨hf, h3⟩⟩
formal/Strict/BSP.lean:359:  ★refinedPred 唯一确定签名（complete/disjoint 的共享引理，L0）：
formal/Strict/BSP.lean:362:  first_second_disjoint / first_third_disjoint 判否。
formal/Strict/BSP.lean:369:  have hns_of_f : e.IsFirst → ¬ e.IsSecond := fun hf hs => first_second_disjoint e ⟨hf, hs⟩
formal/Strict/BSP.lean:370:  have hnt_of_f : e.IsFirst → ¬ e.IsThird := fun hf ht => first_third_disjoint e ⟨hf, ht⟩
formal/Strict/BSP.lean:390:  disjoint := fun {x c₁ c₂} hP1 hP2 =>
formal/Strict/ClassificationFamily.lean:111:  - **互斥**（disjoint）：若 x 同时落在 fiber s₁ 与 s₂，则 s₁ = s₂（不重叠）。
formal/Strict/ClassificationFamily.lean:140:  ★fiber 互斥（disjoint，L0，标准① 的「互斥」侧）：
formal/Strict/ClassificationFamily.lean:141:  `fiber_disjoint` — 一个对象不能落在两个不同标签的 fiber（不重叠，>1 重叠的否定）。
formal/Strict/ClassificationFamily.lean:146:theorem fiber_disjoint {X S : Type} (C : X → S) {s₁ s₂ : S} {x : X} :
formal/Strict/ClassificationFamily.lean:197:  - 互斥（disjoint）：`∀ x s₁ s₂, θFiber F θ s₁ x → θFiber F θ s₂ x → s₁ = s₂`。
formal/Strict/ClassificationFamily.lean:207:   fun _ _ _ h1 h2 => fiber_disjoint (F.C θ) h1 h2⟩
formal/Strict/ClassificationFamily.lean:366:  2. fiber 原像划分（标准① 最干净形式）：`Fiber` + `fiber_total`（覆盖）+ `fiber_disjoint`（互斥）
formal/Strict/Classification.lean:14:  它们枚举了"严格完全分类"需要证明哪些命题（total/sound/complete/disjoint/...），
formal/Strict/Classification.lean:40:  - `disjoint`：互斥——一个对象不能同时满足两个不同标签的谓词（>1 重叠的否定）。
formal/Strict/Classification.lean:50:  disjoint : ∀ {x : X} {c₁ c₂ : C}, P c₁ x → P c₂ x → c₁ = c₂
formal/Strict/Classification.lean:171:  - `branch_disjoint`：互斥——一个延伸不能落入两个不同分支。
formal/Strict/Classification.lean:182:  branch_disjoint :
formal/Strict/LevelState.lean:309:  ★6 态两两互斥（pairwise disjoint，L0）：任意上下文最多满足一个态谓词。
formal/Strict/LevelState.lean:314:theorem rlevel_pairwise_disjoint (ctx : RContext) :
formal/Strict/LevelState.lean:333:  形式：穷尽（`rlevel_predicates_total`）+ 两两互斥（`rlevel_pairwise_disjoint`）的合取——
formal/Strict/LevelState.lean:353:   (rlevel_pairwise_disjoint ctx).1,
formal/Strict/LevelState.lean:354:   (rlevel_pairwise_disjoint ctx).2.1,
formal/Strict/LevelState.lean:355:   (rlevel_pairwise_disjoint ctx).2.2.1,
formal/Strict/LevelState.lean:356:   (rlevel_pairwise_disjoint ctx).2.2.2.1,
formal/Strict/LevelState.lean:357:   (rlevel_pairwise_disjoint ctx).2.2.2.2⟩
formal/Strict/Chain.lean:362:  - 互斥（disjoint）：`∀ x s₁ s₂, … → … → s₁ = s₂`（不重叠，>1 重叠的否定）。

codex
The key split is now clear: Foundation’s unary step can carry a whole sequence because the one argument is an arbitrary state object; the stronger concerns are about what invariants that state carries and whether the adapter chooses a canonical next sequence. I’m grabbing exact line references for the final answer.
exec
/bin/zsh -lc "nl -ba formal/Foundation/CompleteClassification.lean | sed -n '107,150p'" in /Users/silencehan/Projects/NewChanlun
 succeeded in 0ms:
   107	section LevelRecursion
   108	
   109	variable {H : Type}
   110	variable (D : Nat -> Type)
   111	variable (F0 : H -> D 0)
   112	variable (Fstep : (n : Nat) -> D n -> D (n + 1))
   113	
   114	def recAt : (n : Nat) -> H -> D n
   115	  | 0, h => F0 h
   116	  | n + 1, h => Fstep n (recAt n h)
   117	
   118	def RecSpec (h : H) : (n : Nat) -> D n -> Prop
   119	  | 0, d => d = F0 h
   120	  | n + 1, d =>
   121	      Exists (fun prev : D n => RecSpec h n prev /\ d = Fstep n prev)
   122	
   123	theorem recAt_spec (h : H) (n : Nat) :
   124	    RecSpec D F0 Fstep h n (recAt D F0 Fstep n h) := by
   125	  induction n with
   126	  | zero =>
   127	      simp [RecSpec, recAt]
   128	  | succ n ih =>
   129	      exact ⟨recAt D F0 Fstep n h, ih, rfl⟩
   130	
   131	theorem recSpec_complete_unique (h : H) (n : Nat) :
   132	    ExistsUnique (fun d : D n => RecSpec D F0 Fstep h n d) := by
   133	  induction n with
   134	  | zero =>
   135	      refine ⟨F0 h, ?_, ?_⟩
   136	      · rfl
   137	      · intro y hy
   138	        exact hy
   139	  | succ n ih =>
   140	      rcases ih with ⟨prev, hprev, hprevUnique⟩
   141	      refine ⟨Fstep n prev, ?_, ?_⟩
   142	      · exact ⟨prev, hprev, rfl⟩
   143	      · intro y hy
   144	        rcases hy with ⟨prev', hprev', hyEq⟩
   145	        have hprevEq : prev' = prev := hprevUnique prev' hprev'
   146	        subst prev'
   147	        exact hyEq
   148	
   149	theorem recAt_causal
   150	    (ObsEq : H -> H -> Prop)

exec
/bin/zsh -lc "nl -ba formal/Foundation/ChanlunInstantiation.lean | sed -n '107,205p'" in /Users/silencehan/Projects/NewChanlun
 succeeded in 0ms:
   107	abbrev ChanD (_ : Nat) : Type := List Move
   108	
   109	/-- 派生中枢（复用 `Strict.Recursive.canG`：边界算子层不读 centers，centers 生成忠实性由
   110	    `RecursiveConstruction.CentersDerivedFrom`（WellFormed 层）契约——见 Recursive.lean
   111	    分工边界 + RStarNonSpecial.GeneratedStep。本层与 §1 聚合一致，不重复承载派生证明）。 -/
   112	def chanCenters (xs : List Move) : List Center := Strict.Recursive.canG xs
   113	
   114	/--
   115	  ★缠论递归步 `Fstep`（序列实例）：第 n 级走势序列 `xs` 规范聚合为第 n+1 级走势序列。
   116	
   117	  缠论「逐级 compose」（走势分解定理二）：当下级序列 `xs` ≥3 段时，把**整个序列**聚合为
   118	  **一个**第 n+1 级走势 `Move.compose xs (canG xs) (n+1)`（用真实下级序列 xs，非复制；中枢
   119	  `canG xs` 由该序列派生；级别契约 n+1 写入 `Move.level` 字段），返回单元素上级序列。
   120	  不足 3 段（缠论 `CanBuildNext` 假）⟹ 返回 `[]`：级别递归自然终止（`RStarNonSpecial`
   121	  `terminated`/`termination_absorbing`），空上级序列忠实编码终止吸收态。
   122	
   123	  这是 Foundation `Fstep : (n) → ChanD n → ChanD (n+1)` 的缠论确定性实例——Lean 全函数
   124	  （每个 xs 给出唯一上级序列），契合 Foundation 递归级唯一所需，且忠实于
   125	  `MovesComposedFrom` 的窗口结构（见 `composeStep_window_sound`）。
   126	-/
   127	def composeStep (n : Nat) (xs : ChanD n) : ChanD (n + 1) :=
   128	  if 3 ≤ xs.length then [Move.compose xs (chanCenters xs) (n + 1)] else []
   129	
   130	/--
   131	  ★`composeStep` 的窗口 sound 见证（L0，de-degenerate 的核心忠实性证明）：
   132	
   133	  对 ≥3 段下级序列 `xs`，`composeStep n xs` 产出的**单个**上级走势由 `xs` 的**真实连续窗口**
   134	  `[0, xs.length)` 经 `Move.compose` 封装（级别 n+1）——即满足 `UpperMoveSound xs n` 的三件：
   135	  窗口长 ≥3、窗口在界内、上级走势 = 该窗口下级序列 compose。这是 `MovesComposedFrom` 的
   136	  **窗口结构 witness**（`∃ ws` 那一支）在 `composeStep` 上成立的机器证明。
   137	
   138	  ★这关死旧 `[m,m,m]` 复制桩的退化：聚合序列 = **真实** `xs`（窗口 `[0, xs.length)`，非
   139	  复制同一走势），级别 n+1（非塌缩）。旧桩 `[m,m,m]` 三份同一走势在 `windowSlice` 上无法
   140	  对应真实下级窗口（三份起点相同，违反窗口不重叠），本忠实化用真实序列窗口替换。
   141	
   142	  ★诚实边界（no-patch + Recursive.lean 分工边界）：本定理证 `composeStep` 是 **boundary 层**
   143	  窗口 sound（`UpperMoveSound`，不读 centers 的生成忠实性）。`MovesComposedFrom` 还内嵌
   144	  `WellFormed`（含 `centers.length ≥ 1` + `CentersDerivedFrom`）——那是 **WellFormed 层契约**
   145	  （`canG _ := []` 在 Recursive.lean 已诚实声明边界层不承载 centers 派生）。本层**不**冒充
   146	  WellFormed 的 centers 生成义务（声明膨胀禁止）——只证窗口结构忠实（真实序列、级别+1、
   147	  消费约束），这正是 boundary 层应承载的、且足以驱动 Foundation 递归级唯一。
   148	-/
   149	theorem composeStep_window_sound (n : Nat) (xs : ChanD n) (h : 3 ≤ xs.length) :
   150	    ∃ w : UpperMoveWitness,
   151	      UpperMoveSound xs n (Move.compose xs (chanCenters xs) (n + 1)) w := by
   152	  refine ⟨⟨0, xs.length, chanCenters xs⟩, ?_, ?_, ?_⟩
   153	  · -- w.len ≥ 3：xs.length ≥ 3
   154	    exact h
   155	  · -- w.start + w.len ≤ lowerMoves.length：0 + xs.length ≤ xs.length
   156	    simp
   157	  · -- m = compose (windowSlice xs ⟨0, xs.length, _⟩) centers (n+1)
   158	    -- windowSlice xs ⟨0, xs.length, _⟩ = (xs.drop 0).take xs.length = xs
   159	    unfold windowSlice
   160	    simp
   161	
   162	/--
   163	  ★`composeStep` 用真实下级序列（L0，反退化见证）：聚合产物的下级窗口 = 原序列 `xs` 本身，
   164	  **不是**复制。这是「`composeStep` 非 `[m,m,m]` 退化」的直接机器证明——
   165	  `windowSlice xs ⟨0, xs.length, _⟩ = xs`，聚合走势 `compose xs (canG xs) (n+1)` 的子走势
   166	  序列恰为真实 `xs`（含其全部不同元素），而非同一走势的三份复制。
   167	-/
   168	theorem composeStep_uses_real_subs (n : Nat) (xs : ChanD n) (h : 3 ≤ xs.length) :
   169	    composeStep n xs = [Move.compose xs (chanCenters xs) (n + 1)]
   170	    ∧ windowSlice xs ⟨0, xs.length, chanCenters xs⟩ = xs := by
   171	  refine ⟨?_, ?_⟩
   172	  · unfold composeStep; rw [if_pos h]
   173	  · unfold windowSlice; simp
   174	
   175	/-- 种子取值：历史 = 第 0 级走势序列本身，恒等取值（对应 `Strict.Recursive` 的 `d := id`）。
   176	    种子类型 `H := List Move`（第 0 级走势序列 = 线段/起始层序列）。 -/
   177	def chanSeed : List Move → ChanD 0 := id
   178	
   179	/--
   180	  ★缠论走势递归层读出（`recAt` 的缠论实例）：从种子序列 h 经 n 步规范聚合到第 n 级序列。
   181	  这把 Foundation 抽象 `recAt ChanD chanSeed composeStep n h` 钉为缠论具体的「逐级聚合塔」
   182	  （每步把当前级序列规范聚合为上一级序列，忠实于 `MovesComposedFrom`/`GeneratedStep`）。
   183	-/
   184	def chanRecAt (n : Nat) (h : List Move) : ChanD n :=
   185	  NewChanlun.recAt ChanD chanSeed composeStep n h
   186	
   187	/--
   188	  ★缠论走势递归级唯一（Foundation `recSpec_complete_unique` 应用于缠论递归）。
   189	
   190	  对每个种子序列 h 和级别 n，满足缠论递归规格 `RecSpec`（第 0 级 = 种子序列，第 n+1 级 =
   191	  规范聚合前级）的第 n 级走势序列**存在且唯一**。这是 GPT 完全分类「递归级唯一」义务在缠论
   192	  走势递归上的实例化——缠论的逐级聚合塔在每级唯一确定（每层序列由前层规范聚合唯一得出）。
   193	
   194	  ★忠实性：本定理**不**重证 Foundation 的归纳，只把缠论具体的 `ChanD`/`chanSeed`/`composeStep`
   195	  填入 Foundation 已证定理。结论是缠论层的：缠论走势序列递归在每级唯一。
   196	-/
   197	theorem chan_rec_level_unique (h : List Move) (n : Nat) :
   198	    NewChanlun.ExistsUnique
   199	      (fun d : ChanD n => NewChanlun.RecSpec ChanD chanSeed composeStep h n d) :=
   200	  NewChanlun.recSpec_complete_unique ChanD chanSeed composeStep h n
   201	
   202	/--
   203	  ★缠论走势递归因果（Foundation `recAt_causal` 应用于缠论递归，ObsEq := Eq）。
   204	
   205	  观察等价取 `Eq`（两种子序列相等）。`chanSeed = id` 平凡满足 `hF0`（相等种子给相等第 0 级）。

exec
/bin/zsh -lc "nl -ba formal/Formal/RecursiveConstruction.lean | sed -n '180,220p'" in /Users/silencehan/Projects/NewChanlun
 succeeded in 0ms:
   180	structure UpperMoveWitness where
   181	  start : Nat
   182	  len : Nat
   183	  centers : List Center
   184	
   185	/-- 窗口提取：lowerMoves 的连续片段 [start, start+len)。 -/
   186	def windowSlice (lowerMoves : List Move) (w : UpperMoveWitness) : List Move :=
   187	  (lowerMoves.drop w.start).take w.len
   188	
   189	/-- 单个 upper move 由窗口 witness sound 封装（连续窗口 + 级别 +1 + compose）。 -/
   190	def UpperMoveSound (lowerMoves : List Move) (lowerLevel : Nat)
   191	    (m : Move) (w : UpperMoveWitness) : Prop :=
   192	  w.len ≥ 3
   193	  ∧ w.start + w.len ≤ lowerMoves.length
   194	  ∧ m = Move.compose (windowSlice lowerMoves w) w.centers (lowerLevel + 1)
   195	
   196	/--
   197	  ★上级走势由下级走势封装而成（codex R6：有序窗口 witness + 消费/数量约束，关死"重复封装"漏洞）。
   198	
   199	  级别递归的封装步（封装恒等式 Move(k)≡Level-(k+1) 笔，rec_t types.rs:81）：
   200	  `upperMoves` 由 `lowerMoves` 的 **有序、不重叠、连续窗口** 逐个封装而成，**不是集合成员式
   201	  复用**（codex R6 反例：3 个相同 compose 重复消费同 3 段——被 `StrictlyIncreasing` 起点 +
   202	  不重叠 + 数量约束关死）。约束：
   203	  - `upperMoves ≠ []`（生成非空）；
   204	  - 每个 upper move 良构（`WellFormed`）；
   205	  - witness 列表与 upperMoves 等长，逐位 `UpperMoveSound`（连续窗口、级别+1、compose）；
   206	  - witness 起点 `StrictlyIncreasing`（时间顺序，无重排）；
   207	  - ★`upperMoves.length * 3 ≤ lowerMoves.length`（消费约束：每上级走势吃 ≥3 下级，
   208	    有限下级走势不能无限封装——保证级别递归自然收敛，codex R6 核心）。
   209	-/
   210	def MovesComposedFrom (lowerMoves : List Move) (lowerLevel : Nat)
   211	    (upperMoves : List Move) : Prop :=
   212	  upperMoves ≠ []
   213	  ∧ (∀ m ∈ upperMoves, WellFormed m)
   214	  ∧ upperMoves.length * 3 ≤ lowerMoves.length
   215	  ∧ ∃ ws : List UpperMoveWitness,
   216	      ws.length = upperMoves.length
   217	      ∧ StrictlyIncreasing (ws.map UpperMoveWitness.start)
   218	      ∧ PairwiseRel
   219	          (fun m w => UpperMoveSound lowerMoves lowerLevel m w) upperMoves ws
   220	

exec
/bin/zsh -lc "nl -ba formal/Formal/RStarNonSpecial.lean | sed -n '64,88p'" in /Users/silencehan/Projects/NewChanlun
 succeeded in 0ms:
    64	def CanBuildNext (lvl : TowerLevel) : Prop := lvl.moves.length ≥ 3
    65	
    66	/--
    67	  ★层间生成步（codex R4 方案 B：强关系 contract，kind/bsp 不继承）。
    68	
    69	  `GeneratedStep cur next` ⟺
    70	  - `CanBuildNext cur`（本层有足够走势向上构造）；
    71	  - ★下一层中枢由本层走势 **生成**（`CentersDerivedFrom cur.moves next.centers`，
    72	    沿用 RecursiveConstruction 的 witness+顺序 soundness contract）；
    73	  - ★下一层走势是本层走势的封装序列（`next.moves = cur.moves`——封装恒等式
    74	    Move(k)≡Level-(k+1) 笔的最小关系：上一层走势成为下一层构造的原料）。
    75	
    76	  关键（codex R4）：下一层 `outcome` **不出现在 GeneratedStep 里**——它由
    77	  `next.centers` 经 `classifyMove` **自行计算**（`TowerLevel.outcome`），不继承 `cur`。
    78	  `next.bsp` 也不从 `cur.bsp` 复制（无 inheritance 约束）。伪造的 next.centers
    79	  （与 cur.moves 无生成关系）不满足 `CentersDerivedFrom`。
    80	-/
    81	def GeneratedStep (cur next : TowerLevel) : Prop :=
    82	  CanBuildNext cur
    83	  ∧ next.levelIdx = cur.levelIdx + 1
    84	  ∧ CentersDerivedFrom cur.moves next.centers
    85	  ∧ MovesComposedFrom cur.moves cur.levelIdx next.moves
    86	
    87	/--
    88	  ★有效级别塔（终余代数 + 生成约束，codex 审计 R1#8 + R2#8）。

codex
VERDICT: A

REASONING:
The single argument of `Fstep : D n -> D (n+1)` is one *state object*, not one lower-level move. Since `D` is fully free in [CompleteClassification.lean](/Users/silencehan/Projects/NewChanlun/formal/Foundation/CompleteClassification.lean:110), `D n` may be a whole lower-level sequence, a canonical parser state, or even a set of possible parses. So type-level arity does not contradict “≥3 lower moves compose into an upper move.”

The faithful shape is:

```lean
D n := { xs : List Move // ∀ m ∈ xs, WellFormed m ∧ m.level = n }
-- optionally plus Θ / canonical parse policy / window witnesses

Fstep n xs :=
  canonicalWindows xs |>.map
    (fun w => Move.compose (windowSlice xs w) w.centers (n+1))
```

That still has type `D n -> D (n+1)`, while expressing multiple lower moves, ordered windows, derived centers, and possibly multiple upper moves. The existing `composeStep` is the one-window specialization: it returns `[Move.compose xs ...]` when `xs.length ≥ 3` in [ChanlunInstantiation.lean](/Users/silencehan/Projects/NewChanlun/formal/Foundation/ChanlunInstantiation.lean:127). It removes the old `[m,m,m]` degeneracy because the actual `xs` is preserved as the composed move’s boundary.

`recSpec_complete_unique` is also not a contradiction. It proves uniqueness of the result of a chosen deterministic recursion function, not uniqueness of every semantically valid Chanlun parse. If raw market data admits multiple valid parses, then either `H` must include the chosen Θ/canonical policy, or `D n` must represent the whole space of valid parses. Then the unique object is the canonical result or the unique set/state of possibilities.

Strict caveat: the current `composeStep` only proves a boundary/window witness, not the full `MovesComposedFrom` obligation. `MovesComposedFrom` also requires well-formed upper moves and window-list constraints in [RecursiveConstruction.lean](/Users/silencehan/Projects/NewChanlun/formal/Formal/RecursiveConstruction.lean:210). That is an adapter-proof gap, not a Foundation-level structural contradiction.

So: build on Foundation is valid, but the Chanlun adapter should strengthen `D`/`Fstep` with well-formedness, center derivation, and canonical windowing if it wants full fidelity beyond the one-window boundary case.
2026-06-26T06:34:11.643631Z ERROR codex_core::session: failed to record rollout items: thread 019f02a0-7f08-7ae1-9d1b-33d2e553ca2a not found
tokens used
78,607
VERDICT: A

REASONING:
The single argument of `Fstep : D n -> D (n+1)` is one *state object*, not one lower-level move. Since `D` is fully free in [CompleteClassification.lean](/Users/silencehan/Projects/NewChanlun/formal/Foundation/CompleteClassification.lean:110), `D n` may be a whole lower-level sequence, a canonical parser state, or even a set of possible parses. So type-level arity does not contradict “≥3 lower moves compose into an upper move.”

The faithful shape is:

```lean
D n := { xs : List Move // ∀ m ∈ xs, WellFormed m ∧ m.level = n }
-- optionally plus Θ / canonical parse policy / window witnesses

Fstep n xs :=
  canonicalWindows xs |>.map
    (fun w => Move.compose (windowSlice xs w) w.centers (n+1))
```

That still has type `D n -> D (n+1)`, while expressing multiple lower moves, ordered windows, derived centers, and possibly multiple upper moves. The existing `composeStep` is the one-window specialization: it returns `[Move.compose xs ...]` when `xs.length ≥ 3` in [ChanlunInstantiation.lean](/Users/silencehan/Projects/NewChanlun/formal/Foundation/ChanlunInstantiation.lean:127). It removes the old `[m,m,m]` degeneracy because the actual `xs` is preserved as the composed move’s boundary.

`recSpec_complete_unique` is also not a contradiction. It proves uniqueness of the result of a chosen deterministic recursion function, not uniqueness of every semantically valid Chanlun parse. If raw market data admits multiple valid parses, then either `H` must include the chosen Θ/canonical policy, or `D n` must represent the whole space of valid parses. Then the unique object is the canonical result or the unique set/state of possibilities.

Strict caveat: the current `composeStep` only proves a boundary/window witness, not the full `MovesComposedFrom` obligation. `MovesComposedFrom` also requires well-formed upper moves and window-list constraints in [RecursiveConstruction.lean](/Users/silencehan/Projects/NewChanlun/formal/Formal/RecursiveConstruction.lean:210). That is an adapter-proof gap, not a Foundation-level structural contradiction.

So: build on Foundation is valid, but the Chanlun adapter should strengthen `D`/`Fstep` with well-formedness, center derivation, and canonical windowing if it wants full fidelity beyond the one-window boundary case.
