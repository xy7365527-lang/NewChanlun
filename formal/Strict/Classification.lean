/-
  Strict/Classification.lean — 缠论式完全分类·标准内核库（task #57, RTAS 蜂群 T-kernel 工位）

  上游标准：`tmp/chanlun-strict-classification-standard.md`（最严格完全分类标准，三方收敛靶子）
            + `/tmp/codex_gapmatrix_answer.md`「升级清单」段（codex 异质审计的七结构骨架）。

  本文件是所有 Layer2 工位的**共享内核**——把"缠论式完全分类"6 部分标准的严格陈述
  转写为可复用的 Lean 4 结构定义。各工位（T-trend / T-center / T-bsp / T-op / T-eval /
  T-decomp-uniq / T-causal / T-opentail / T-kernel-rec）import 本库，对各自分类对象实例化
  这些结构，从而把"声明了完全分类"变成"实例化了带证明义务的结构"。

  ★认识论等级（formalization-validity-domain 强制标注）：
  本文件全部为 **L0**（纯定义层，不依赖数据）。这些 structure 是**证明义务的容器**：
  它们枚举了"严格完全分类"需要证明哪些命题（total/sound/complete/disjoint/...），
  但**本文件不提供任何这些命题的证明**——证明义务转嫁给实例化它们的下游工位。
  lake build 通过 = 这些结构定义语法合法、字段类型自洽，**不**是任何分类已被证明完全。

  ★命名约束（codex#1 硬规则，标准 §第一部分 A）：
  μF/inductive 只证「语法生成无遗漏 + 语法相等下构造子互斥」（Layer1），**不自动**给
  语义 ∼ 下的不变性/完备性/可实现性（Layer2）。凡只在 `x ∼ y ↔ I x = I y` 下成立的分类，
  必须显式实例化为 `SemanticQuotient`（按标签商分类），不得冒充 `Classifies`（语义双射）。

  范式：纯 Prop/Type 结构定义，不依赖 Mathlib（`Equivalence` 来自 Lean core）。
  禁 sorry/admit/axiom——这些是结构定义，不需要证明，天然编译通过。

  谱系：598（真完全分类元判据）→ 603（递归 vs 轴范式）→ 615（Layer1 ⊊ Layer2 概念分离）。
-/

namespace Strict

/--
  **结构 1 / Classifies** — 静态语义双射分类（标准 §第一部分 A，6 部分标准第 1 部分的严格形式）。

  给定对象类型 `X`、标签类型 `C`、不变量 `I : X → C`、谓词族 `P : C → X → Prop`，
  `Classifies X C I P` 收集"`I` 在语义层是 `X` 到 `C` 的完全分类"所需的五项证明义务：

  - `total`：穷尽——每个对象至少落入一类（=0 遗漏的否定）。
  - `sound`：健全——`I x` 给出的标签确实满足谓词（不变量与谓词一致）。
  - `complete`：完备/商集单射——同标签蕴含 `I` 相等（标准称"最危险的待证项"）。
  - `disjoint`：互斥——一个对象不能同时满足两个不同标签的谓词（>1 重叠的否定）。
  - `realized`：可实现——每个标签都有对象见证（无空标签）。

  ★这是最严格的基线。下游若 `complete` 证不出（如走势 {趋势,盘整} 对完整结构太粗），
  应**诚实降级**为 `SemanticQuotient`（按标签商分类）+ 刻画 complete 失败反例，不强证。
-/
structure Classifies (X C : Type) (I : X → C) (P : C → X → Prop) : Prop where
  total : ∀ x, ∃ c, P c x
  sound : ∀ x, P (I x) x
  complete : ∀ {x : X} {c : C}, P c x → I x = c
  disjoint : ∀ {x : X} {c₁ c₂ : C}, P c₁ x → P c₂ x → c₁ = c₂
  realized : ∀ c, ∃ x, P c x

/--
  **结构 2 / SemanticQuotient** — 按标签商分类（codex#1 命名约束的诚实降级目标）。

  当语义等价 `Rel : X → X → Prop` 是独立给定的，且不变量 `I : X → P` 满足：
  - `equiv`：`Rel` 是等价关系（自反/对称/传递）。
  - `invariant`：等价的对象同标签（`x ∼ y → I x = I y`）。
  - `complete`：同标签蕴含等价（`I x = I y → x ∼ y`，即诱导商集 `X/∼` 上 `I` 单射）。
  - `realized`：每个标签可实现。

  则 `I` 诱导双射 `X/∼ ≅ P`。这是"按标签商分类"的精确形式——**不**自动等于 `Classifies`，
  因为 `Rel` 可能正是"分类结果相同"（此时 complete 是同义反复，无独立语义内容，标准 §A 警示）。
  下游实例化时必须声明 `Rel` 是独立语义等价（结构等价/操作等价），否则只是平凡商。
-/
structure SemanticQuotient (X P : Type) (I : X → P) (Rel : X → X → Prop) : Prop where
  equiv : Equivalence Rel
  invariant : ∀ {x y : X}, Rel x y → I x = I y
  complete : ∀ {x y : X}, I x = I y → Rel x y
  realized : ∀ p, ∃ x, I x = p

/--
  **结构 3 / DecompositionSystem** — 唯一递归分解（标准 6 部分第 2 部分）。

  - `d : H → X`：历史 `H` 到第 n 级完整结构状态 `X` 的取值映射。
  - `D : X → T`：结构状态到其分解 `T`。
  - `Eval : T → X`：分解的求值（重构）。
  - `ValidDecomp : X → T → Prop`：合法分解谓词。
  - `RelT : T → T → Prop`：分解间的等价（结合律/多义性等价 ∼ₙ）。
  证明义务：
  - `rel_equiv`：`RelT` 是等价关系。
  - `eval_sound`：**重构健全** `Eval (D (d h)) = d h`（标准"分解能重构原对象"，当前 Lean 完全缺失项）。
  - `D_valid`：规范分解 `D (d h)` 确实合法。
  - `unique_mod_rel`：**∼ₙ 下唯一** 任何合法分解都与规范分解等价（`|𝒟ₙ(h)/∼ₙ| = 1`）。
-/
structure DecompositionSystem (H X T : Type) where
  d : H → X
  D : X → T
  Eval : T → X
  ValidDecomp : X → T → Prop
  RelT : T → T → Prop
  rel_equiv : Equivalence RelT
  eval_sound : ∀ h, Eval (D (d h)) = d h
  D_valid : ∀ h, ValidDecomp (d h) (D (d h))
  unique_mod_rel : ∀ h t, ValidDecomp (d h) t → RelT t (D (d h))

/--
  **结构 4 / OnlineSystem** — 在线状态转移（标准 6 部分第 3 部分，δ 全函数版）。

  - `append : Hist → E → Hist`：历史追加一个事件。
  - `C : Hist → S`：历史的当下结构状态。
  - `δ : S → E → S`：全函数状态转移（`δ` 是 Lean 全函数，自动满足"∀(s,e)∃!s'"的全/确定性）。
  - `step_sound`：转移健全 `C (append h e) = δ (C h) e`（追加事件后的状态 = 转移函数施加于旧状态）。

  ★`δ : S → E → S` 写成 Lean 全函数本身就编码了"全函数 + 确定"——无需额外证明义务。
  无前视的**因果性**（前缀相等 ⟹ 当下状态相等）由 `CausalOnlineSystem` 单独刻画。
-/
structure OnlineSystem (Hist S E : Type) where
  append : Hist → E → Hist
  C : Hist → S
  δ : S → E → S
  step_sound : ∀ h e, C (append h e) = δ (C h) e

/--
  前缀相等：两条事件流 `ω` `ω'` 在时刻 `t` 及之前逐点相等。

  缠论"无前视"的形式化前提——当下状态只能依赖已出现（索引 ≤ t）的数据。
-/
def PrefixEq {E : Type} (ω ω' : Nat → E) (t : Nat) : Prop :=
  ∀ i, i ≤ t → ω i = ω' i

/--
  **结构 5 / CausalOnlineSystem** — 无前视因果系统（标准 6 部分第 3 部分，因果约束版）。

  - `Cstream : (Nat → E) → Nat → S`：从完整事件流读出每个时刻的状态。
  - `causal`：**因果无前视** `PrefixEq ω ω' t → Cstream ω t = Cstream ω' t`
    （时刻 t 的状态只依赖 t 及之前的事件——未来不影响当下，无事后修正）。

  这是缠论"当下判断不能用未来数据"的精确结构形式（standard 第 3 部分的核心命题，当前 Lean 缺失）。
-/
structure CausalOnlineSystem (S E : Type) where
  Cstream : (Nat → E) → Nat → S
  causal : ∀ ω ω' t, PrefixEq ω ω' t → Cstream ω t = Cstream ω' t

/--
  **结构 6 / RecursiveKernel** — 递归核（标准 6 部分第 4 部分，规范化唯一边界算子）。

  - `U : Nat → Type`：分级宇宙——`U n` = 第 n 级对象类型（如 `U 0` = 线段，`U (n+1)` = 第 n+1 级走势）。
  - `Atom`：最低级原始构件（𝒜）。
  - `base : Atom → U 0`：原始构件嵌入第 0 级。
  - `Can n : {xs : List (U n) // xs.length ≥ 3} → U (n+1)`：规范化构造算子，
    把 **≥ 3 段**（标准"至少由三段以上次级别构成"）第 n 级对象组装为第 n+1 级对象。
  - `Boundary n : U (n+1) → List (U n)`：边界算子（取回构成第 n+1 级对象的下级序列）。
  证明义务：
  - `can_sound`：边界健全 `Boundary n (Can n xs) = xs.val`（组装后取边界还原原序列）。
  - `can_unique`：**规范化唯一边界** `Can n xs = Can n ys → xs.val = ys.val`
    （同一对象只有唯一边界分解——`Can` 单射于边界，防止中枢↔走势循环定义）。

  ★`Can` 的 ≥ 3 约束直接编码在子类型 `{xs // xs.length ≥ 3}` 中——非法（< 3 段）输入
  在类型层即被排除，无需运行时检查（标准 m ≥ 3 的结构化落实）。
-/
structure RecursiveKernel (U : Nat → Type) where
  Atom : Type
  base : Atom → U 0
  Can : ∀ n, {xs : List (U n) // xs.length ≥ 3} → U (n + 1)
  Boundary : ∀ n, U (n + 1) → List (U n)
  can_sound : ∀ n xs, Boundary n (Can n xs) = (xs : {xs : List (U n) // xs.length ≥ 3}).val
  can_unique : ∀ n xs ys, Can n xs = Can n ys → xs.val = ys.val

/--
  **结构 7 / OpenTailSystem** — 未完成走势→状态分支集（标准 6 部分第 5 部分）。

  标准最强调的修正：**未完成走势不能强行唯一分类为最终结果，只能唯一分类为当下状态**；
  未来延伸 `Ext(h) = ⊔ⱼ Bⱼ(h)` 是互斥穷尽的分支集（不预判唯一终局）。

  - `current : Hist → State`：未完成历史的**当下状态**（唯一，可分类）。
  - `Ext : Hist → Hist → Prop`：`Ext h h'` 表示 `h'` 是 `h` 的一个未来延伸。
  - `BranchPred : Hist → Branch → Hist → Prop`：`BranchPred h b h'` 表示延伸 `h'` 落入分支 `b`。
  证明义务（分支集互斥穷尽）：
  - `branch_total`：穷尽——每个延伸至少落入一个分支。
  - `branch_disjoint`：互斥——一个延伸不能落入两个不同分支。
  - `branch_sound`：健全——落入某分支的必是真延伸。

  ★这是对 `classifyMove` "对未完成走势直接给最终 outcome" 越界事后分类的结构性修正：
  当下只给 `current`（状态），未来给 `Ext` 的分支集，**不**把未完成压成 trend/consolidation 结果。
-/
structure OpenTailSystem (Hist State Branch : Type) where
  current : Hist → State
  Ext : Hist → Hist → Prop
  BranchPred : Hist → Branch → Hist → Prop
  branch_total : ∀ h h', Ext h h' → ∃ b, BranchPred h b h'
  branch_disjoint :
    ∀ h h' b₁ b₂, BranchPred h b₁ h' → BranchPred h b₂ h' → b₁ = b₂
  branch_sound : ∀ h b h', BranchPred h b h' → Ext h h'

/--
  **严格动作集** — 完全应对策略 πₗ 的值域（标准 6 部分第 6 部分）。

  七个动作覆盖标准要求的 {买/卖/加/减/持/平/等待}：
  - `buy` 买 / `sell` 卖 / `add` 加（仓） / `reduce` 减（仓） /
    `hold` 持（仓不动） / `close` 平（仓） / `wait` 等待（空仓观望）。

  ★`hold` 与 `wait` 的区分（codex 标准修正）：`hold` = 持有现有仓位不动，
  `wait` = 空仓等待（无仓位）。当前 Operational.lean 只有 `hold` 缺 `wait` 是缺口，此处补全。
-/
inductive StrictAction where
  | buy
  | sell
  | add
  | reduce
  | hold
  | close
  | wait
deriving DecidableEq, Repr

/--
  **结构 8 / Strategy** — 完全应对策略（标准 6 部分第 6 部分）。

  - `π : S → StrictAction`：从完整结构状态 `S` 到严格动作的**全定义**策略（aₜ = πₗ(Cₗ(hₜ))）。
  - `total`：全定义性（`π` 是 Lean 全函数，对每个状态都给出一个动作——平凡成立，
    但显式列为证明义务以使"完全应对"的语义可被下游引用/质询）。

  ★`π : S → StrictAction` 写成 Lean 全函数即编码"完全应对"（每个状态都有对策）。
  无前视因果（操作只依赖已出现数据）由 `S` 经 `CausalOnlineSystem` 读出保证，不在本结构内重复。
-/
structure Strategy (S : Type) where
  π : S → StrictAction
  total : ∀ s, ∃ a, π s = a

end Strict
