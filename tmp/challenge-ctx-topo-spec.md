# 拓扑计算架构规格 Gemini 审查上下文

## 任务说明

对 `topological-computation-spec.md` 中的拓扑计算架构规格进行数学审查。
聚焦四个核心问题 Q-S1 到 Q-S4。

**重要**：使用 `derive` 模式（temperature=0.1），对每个问题展开完整数学推理。
如果发现规格中的数学错误或过强声称，明确指出。所有结论标注认识论等级（L0/L1/L2/L3）。

---

## 规格全文

### Topological Computation: Architecture Specification

#### 0. What This Is

A computational paradigm in which reasoning is performed through topological operations on simplicial complexes, not through vector arithmetic in continuous spaces. The system thinks by creating, destroying, and restructuring topological features — holes, cycles, connected components — in a discrete combinatorial structure. Each reasoning step irreversibly changes the topology of the computation space, and the accumulated topological history constitutes the system's knowledge.

This is not a neural network. Not a symbolic AI. Not a hybrid. It is a third kind.

#### 1. Computation Space

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

#### 2. Operations

##### 2.1 Fold
Identify S ⊂ V_active (|S| ≥ 2) as equivalent.

Δβ₁ = (c - 1) + n_loop

where c = connected components of lower link l⁻(p), n_loop = edges within S becoming self-loops.

##### 2.2 Negate
Add vertex w and negation edge w → v, removing v from K_active.

##### 2.3 Sublate
Add vertex w with revision edge w → v. v remains active but topological neighborhood changes.

#### 3. Effect Prediction

Every operation's topological effect is analytically predictable before execution. O(|E|) for fold, O(|V|+|E|) for negate/sublate.

#### 4. Selection Criterion

Dual-objective tension:
- Objective A: β₁ growth (topological production)
- Objective B: settlement density (irreversibility accumulation)

T(op) = Δg(op) × (1 - s) + Δs(op) × g

where g(t) = β₁(K_active(t))/t, s(t) = |settled cycles| / β₁(K_active(t))

Select max T. If all T ≤ 0, wait.

##### 4.2 Automatic Settlement
Cycle settled when: persisted N steps + survived negation attempt + structurally integrated.

##### 4.3 Intrinsic Impossibility
Settled cycles impose constraints emerging from topology, not external rules.

#### 5. Candidate Generation

LLM as pruning engine (not reasoner). Proposes M≈10-20 candidates. Topological layer evaluates (Δβ₁, T, settlement constraints) and executes.

Isomorphic to AlphaGo: Policy network (LLM) → Value network (Δβ₁ + T) → Game engine (topological layer).

#### 6. Self-Reference

β₁(K_active) computed at every step, enters criterion T through g(t). System observes own topological state → determines next operation → changes state → observes again.

History entropy: β₁(K_full) - β₁(K_active).

#### 7. Properties

- Irreversibility: every operation appends to K_full
- History: complete record of every operation
- Contradiction as Structure: cycles from negation edges (β₁ contributions), not loss to minimize
- Intrinsic Impossibility: settled topology excludes operations architecturally

#### 8. Comparison

| Dimension | Neural Network | Symbolic AI | Topological Computation |
|-----------|---------------|-------------|------------------------|
| State space | Continuous vector | Discrete propositions | Typed directed simplicial complex |
| Effect predictability | Opaque | Transparent | Analytic (Δβ₁ formula) |
| History | None during inference | Derivation tree | Complete, append-only, irreversible |
| Contradiction | Minimized | Forbidden | Structural feature |
| Self-reference | Impossible | Limited (Gödel) | Native (β₁ from own structure) |
| Impossibility | External (RLHF) | External (axioms) | Intrinsic (settled topology) |

#### 9. Minimal Experiment

8-10 vertices, 12-15 typed directed edges. 20-30 steps. Success criteria: β₁ increases, settlement occurs, no degeneration, T non-trivially distributed, at least one operation rejected by settlement.

#### 10. Open Questions

1. Is T optimal? Nonlinear coupling? Adaptive forms?
2. Multi-step planning (MCTS over topological operations)?
3. Scale (incremental β₁ algorithms)?
4. Natural language interface?
5. Domain independence?
6. Persistent homology of the reasoning process itself?

---

## 背景研究成果（R1-R3）

### R1 核心结论（已证）
- K_active = Mapper Nerve（L0，形式证明）
- β₁ 差值分解：β₁(K_full) - β₁(K_active) = "被否定环" + "版本链内部环"（L0）
- Vineyard 失效，需用 Zigzag 持续同调（L0，逻辑必然）
- Forman 离散 Morse 直接应用失效（L0）

### R2 核心结论（已证）
- φ 信息损失有界：d_B(PD(S), PD(K)) ≤ 2·d_GH(S, K)（L0）
- ε* 不唯一，是有限区间并集（L0）
- ψ∘φ 状态空间无限扩张，不收敛静态均衡（修正后 L0）
- β₁(t)/t 增长率：在局部平稳假设下收敛（Kingman，条件性 L0）

### R3 核心结论（已证/修正）
- Q-R3-2 商映射折叠的 Morse 理论是开放研究问题（Forman 3.3 不适用于商映射，collapse ≠ quotient）
- 分层 Morse 理论（SMT, Goresky-MacPherson 1988）在最小例子上成功（L1 验证）
- 折叠 Δβ₁ 完整公式：(c-1) + n_loop（L0，下链接连通分量数 + 自环数）
- 否定 = Cerf 分岔（1-handle 附着）（L0）
- 扬弃 = 不稳定流形重构（条件性 L0，依赖 SMT 框架）

---

## 四个核心审查问题

### Q-S1：选择准则 T 的数学性质

T(op) = Δg(op) × (1 - s) + Δs(op) × g

其中：
- g(t) = β₁(K_active(t))/t（β₁ 增长率）
- s(t) = |settled cycles| / β₁(K_active(t))（结算密度）
- Δg(op) = g 在执行 op 后的变化
- Δs(op) = s 在执行 op 后的变化

**需要推理的子问题**：

1. **不动点存在性**：是否存在 g*, s* 使得所有候选操作的 T = 0？即系统是否有"停止选择"状态？

2. **交叉耦合的退化防止**：
   - g→0 时 T 的行为？（β₁ 增长停止时，系统是否必然趋向结算？）
   - s→1 时 T 的行为？（全部结算后，系统是否能继续生产新 β₁？）
   - 交叉耦合 Δg × (1-s) + Δs × g 是否真正防止退化？

3. **变分原理**：T 的函数形式是否有变分原理支持？是否是某个泛函的梯度（或次梯度）？

4. **等待状态的含义**：T ≤ 0 的"等待"状态是否等价于 Q11 修正后的"非平稳增长过程的暂停"？即等待 ≠ 停止，而是增长率局部为负的暂态？

**背景约束**：
- g 和 s 的乘积对称性（双线性耦合）
- 从 R3-Q11 修正知：系统长期 β₁ → ∞，增长率 λ = lim β₁(t)/t 存在（局部平稳假设下）

### Q-S2：Negate 和 Sublate 的拓扑效果精确化

规格第 2.2 节（Negate）和 2.3 节（Sublate）的描述是定性的。需要精确化。

**背景（来自 R3）**：
- Fold 的精确公式：Δβ₁ = (c-1) + n_loop（已由 SMT 证明，L0）
- Negate = 1-handle 附着 = Cerf 分岔（R3-Q3 结论）
- Sublate = 不稳定流形重构（R3-Q4 结论）

**需要推理的子问题**：

1. **Negate 的精确公式**：
   - 规格描述："v on k independent cycles, sole passage point for each: β₁ decreases by k"
   - "sole passage point" 有精确的拓扑定义吗？（cut vertex? bridge in graph theory? articulation point?）
   - Negate 是否总是 β₁ 增加（1-handle 附着），还是在某些情况下 β₁ 减少（否定移除某个 cycle 的一部分）？
   - 给出类似 Fold 公式的解析表达：Δβ₁(Negate) = ?

2. **Sublate 的精确公式**：
   - 规格描述："may create new cycles"——这个"may"的条件是什么？
   - Sublate 中 v 保持 active，但邻域改变——Δβ₁ 的变化是确定的还是依赖图结构？
   - 给出：Δβ₁(Sublate) = ?

**重要约束**：
- Negate 添加的是 w → v（否定边），v 进入 K_full 但退出 K_active
- 这意味着涉及 v 的已有环在 K_active 中消失，但在 K_full 中保留

### Q-S3：AlphaGo 类比的严格性

规格第 5 节声称：LLM = 策略网络，Δβ₁ + T = 价值网络 + MCTS，拓扑层 = 游戏引擎。

**需要推理的子问题**：

1. **训练类比的断裂**：
   - AlphaGo 的策略网络和价值网络通过自对弈训练
   - 这个架构的 LLM 不需要训练（它是外部预训练模型），还是需要用不同方式训练？
   - 如果 LLM 不需要训练，类比在哪里断裂？

2. **状态空间类比的断裂**：
   - AlphaGo 的状态空间固定（19×19 棋盘，有限状态数）
   - 这个架构的状态空间 append-only 持续增长（从 R2-Q11 修正知：状态空间无限）
   - 这对 MCTS 类比的适用性有什么影响？（MCTS 需要从当前状态模拟未来状态——无限扩张的状态空间如何处理？）

3. **价值函数类比**：
   - AlphaGo 的价值网络输出 [-1, 1] 区间的期望获胜概率
   - T(op) 的值域是什么？是否有界？
   - T 的最大化是否等价于某个博弈论意义上的最优策略？

### Q-S4：最小实验充分性

规格第 9 节：8-10 顶点，12-15 边，20-30 步。

**需要推理的子问题**：

1. **成功标准的充分性**：
   - 成功标准 5 项：β₁ increases / settlement occurs / no degeneration / T non-trivially distributed / at least one operation rejected by settlement
   - 对于 "settlement occurs"：一个 cycle 需要 persisted N steps + survived negation attempt + structurally integrated。在 20-30 步内，N 设为多少合理？20 步中如何分配给不同类型操作？
   - 对于 "at least one operation rejected by settlement"：这需要 settlement 先发生，再有否定尝试被拒绝。两个事件的先后需要多少步？

2. **β₁ 必须非零的初始图设计**：
   - 规格没有约束初始图——8-10 顶点 + 12-15 边的初始图可能是树（β₁ = 0），则前几步只能从 β₁ = 0 开始生产
   - 是否需要保证初始图至少有一个环（β₁ ≥ 1）？还是从 β₁ = 0 开始也能在 20-30 步内完成所有 5 个成功标准？

3. **规模充分性**：
   - 8 个顶点的图中，最多有多少个独立 β₁ 环？（上限 = |E| - |V| + 1 = 12 - 8 + 1 = 5，树时为 0）
   - 对于展示架构的"第三种范式"特性，5 个潜在环是否足够？

**背景约束**：来自 R3-Q2 的 SMT 框架，每次 Fold 操作的 β₁ 贡献 = (c-1) + n_loop。在 8 顶点图上，典型的 Fold 能产生多大的 β₁ 跳变？

---

## 输出要求

对每个问题展开完整数学推理，给出：
1. 推理链（步骤化）
2. 结论（明确）
3. 认识论等级（L0/L1/L2/L3）
4. 边界条件（在什么情况下结论翻转）
5. 如果规格有数学错误或过强声称，明确指出

**产出大小约束**：总回复 ≤ 8KB。超出部分写入 `tmp/` 附件文件，正文保留摘要 + 附件路径引用。
