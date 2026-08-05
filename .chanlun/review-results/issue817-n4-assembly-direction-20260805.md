# issue #817 N-4 考据：装配方向与递归骨架（top-down ↔ bottom-up；三套 Lean 骨架；记号 vs 实装方向）

- **票**：wayfinder 裁定票 [#817](https://github.com/xy7365527-lang/NewChanlun/issues/817)，N-4 组
- **性质**：**考据报告，不做裁定**。查清事实 + 摆各方承重论据 + 给形态清单与代价。裁定由人做。
- **基线**：`git reset --hard origin/main-rewritten` → `5cc6dcd22a`（接报时工作树停在 `19b4015927` 旧号且
  `rust/src/theta_v0/` 不存在，已按票面开工纪律归正；本仓连撞第十次）
- **分支**：`research/i817-n4-direction`（不合 main、不推远端）
- **前置已裁**（不重推）：N-1（坐标＝原始 K 序号／收缩单边／Lean `Interval` 坐标不可知）、
  N-2（停止判据＝成本门 ∧ 塔底；生产从未求值「停在哪一级」）、N-3（区间套只有纵向下钻一义）

---

## 0. 摘要：本组查出的票面前提错误（三条）

| # | 票面原话 | 实测 | 承重后果 |
|---|---|---|---|
| **E-1** | 「两文件头**互相**声明『刻意不合并』」 | **单向**。`NestingCertificate.lean:22-32/:424-426` 声明了；`IntervalNestCertificate.lean` 与 `Strict/Nest.lean` **全文零处**提到 `NestingCertificate`（repo-wide grep：`NestingCertificate` 在 Rust 侧 **0 命中**，Lean 侧只被 `CandidateSet.lean`、`SelfSimilarity.lean` 消费） | 「互相声明」意味着双方都认这个分工；实际只有后来者认。**另一套从来不知道自己被分工了** ⟹ 不是双边协议，是单边备案 |
| **E-2** | 「记号 `N^δ_{ℓ↓e}` 与实装 `for k in (lvl+1)..max_k` 方向不一致」 | **是命名冲突不是方向冲突**（详见 §5）。记号里 ℓ＝**顶**（操作级），实装里 `lvl`＝**底**（执行级 e）。`econ_positive.rs:1042` 注释逐字「N^δ_{ℓ↓e} 的执行级 **e=信号级 lvl**」。且该循环是**收集**循环，收完 `rung_buf.reverse()`（`:1066`），真正的**校验**递归 `n_delta_rec`（`classifier/nest.rs:340-362`）是 rungs[0]（最高）→base（执行级），**方向与记号一致** | 两种诊断的修法完全不同：方向冲突要改算法；命名冲突改注释/改参数名。**本条实测是后者，零行为改动** |
| **E-3** | 票面把 `[] => false` vs `[] => True` 当成「两套对同一输入相反结论」 | 两个函数**类型不同、定义域不相交**（`List NestLevel → Bool` vs `List LevelNode → Prop`，无任何转换函数），**唯一可直接对比的输入是空链**，而空链上的分歧可归因到**「Confirm 摆在哪」的口径差**，不是对嵌套本身的相反判决（详见 §3） | 「符号相反」这个说法半真：在空链这一点上真相反，但成因不是嵌套判据冲突。**若按「矛盾」裁，会去修错的地方** |

另有 **E-4（票面行号漂移，轻）**：`IntervalNestCertificate.lean` 的 `nestCertB` 是 **416-432**（票面写 416-431，末行 `false` 漏一行）。`NestingCertificate.lean:185-190`、`Strict/Nest.lean:218-222` 票面行号**准确**。

---

## 1. 三套骨架逐套定位核实

三套**全部是 top-down**（链头/高下标＝操作级 ℓ，递归朝执行级 e 走）。**没有一套是 bottom-up**——
bottom-up 只存在于 Rust 侧一个 `#[cfg(test)]` 探针（§4）。

### 1.1 套一：Nat 级别差 —— `formal/Origin/NestingCertificate.lean:185-190`

```lean
def nestCert (δ : Dir) (f : Nat → LevelData) (e : Nat) : Nat → Bool
  | 0 => Conf δ (f e)
  | d + 1 =>
      candOf δ (f (e + d + 1))
        && subB (jOf δ (f (e + d))) (jOf δ (f (e + d + 1)))
        && nestCert δ f e d
```

对外入口（`:199-200`）：

```lean
def N (δ : Dir) (f : Nat → LevelData) (ℓ e : Nat) : Bool :=
  nestCert δ f e (ℓ - e)
```

- **基例（全部，共一个）**：`| 0 => Conf δ (f e)`。`Conf`（`:123-126`）＝ `d.b1||d.b2||d.b3`（buy）／
  `d.s1||d.s2||d.s3`（sell）。
- **装配方向**：**top-down**。求值从 `d = ℓ-e` 出发递减到 0，即从操作级 ℓ 下行到执行级 e。
- **终止性质**：**算术必然**——Nat 结构递归 `d+1 ↦ d`，Lean 自动判定，**无可失败分支**。
  `:196-197` 自陈越界行为：`ℓ < e` 时 Nat 截断 `ℓ-e=0`，**静默退化成基例 Conf**（不是拒绝）。
- **额外定理（另两套都没有）**：级别平移不变 `N_translation_invariant`（`:303-308`）。
- **状态载体是函数** `f : Nat → LevelData`（**全级别恒有数据**），不是列表——这是它「无可失败基例」的
  结构原因：**级别永不缺席**。

### 1.2 套二：列表链 —— `formal/Origin/IntervalNestCertificate.lean:416-432`（订正行号）

```lean
def nestCertB (ev : Nat) : List NestLevel → Bool
  | [] => false
  | [ℓ] =>
      decide (ℓ.lvl = ev) && (ℓ.chosen.isSome) && ℓ.confirmOK
  | ℓ :: ℓ' :: rest =>
      if ℓ.lvl = ev then
        false
      else if ℓ.lvl > ev then
        ℓ.candidateOK && decide (ℓ'.lvl < ℓ.lvl) && subB ℓ' ℓ
          && nestCertB ev (ℓ' :: rest)
      else
        false
```

- **基例（全部，共两个 + 两个失败守卫）**：
  ① `[] => false`；② `[ℓ] => decide (ℓ.lvl = ev) && ℓ.chosen.isSome && ℓ.confirmOK`；
  失败守卫 ③ `ℓ.lvl = ev` 但链未尽 ⟹ `false`；④ `ℓ.lvl < ev`（链反向）⟹ `false`。
- **装配方向**：**top-down**。链头 `ℓ₀` ＝ 操作级，递归进入尾部（更低级别）；`decide (ℓ'.lvl < ℓ.lvl)` 门控级别严格递减。
- **终止性质**：**可失败的运行时检查**——四条路径都能返回 0（空链／级别号不等 ev／无 chosen／链反向）。
- **⚠️ 自陈与实装不符（N-2 已登记为 T-5，本报告独立复核成立）**：`:361` 自陈「`lvl` 严格递减保证良基
  （Nat 度量 `lvl - e_v` 递减到 0）」，但 `decide (ℓ'.lvl < ℓ.lvl)` 只是 **Bool 门控**，终止实际由列表结构递归给。
- **χ 入口**（`:440`）：`chiBool ev chain := nestCertB ev chain`——**Confirm 已折进链末元素**。

### 1.3 套三（票面漏，N-2 坐实）：`formal/Strict/Nest.lean:218-222`

```lean
def NestCertificate : List LevelNode → Prop
  | [] => True
  | [ℓ] => ℓ.Candidate
  | ℓ₀ :: ℓ₁ :: rest =>
      ℓ₀.Candidate ∧ Sub ℓ₁.chosen ℓ₀.chosen ∧ NestCertificate (ℓ₁ :: rest)
```

- **基例（全部，共两个）**：① `[] => True`；② `[ℓ] => ℓ.Candidate`。**无 `ev`、无 `confirm`、无 `chosen` 存在性检查。**
- `:215-216` 自陈：「空链为 `True`（无级别 ⟹ 无约束），单级链退化为该级 candidate 谓词。」
- **装配方向**：**top-down**（`ℓ₀` 在表头 ＝ 操作级；`:12` 逐字「从操作级别 ℓ₀…逐级进入到执行级别 e_v」）。
- **终止性质**：Prop 侧列表结构递归；**三套里最松**。
- **★ 关键：Confirm 不在这里**。χ 是**分开**的合取（`:290`/`:298-303`）：
  `Chi` 是 `structure`，字段 `nest : NestCertificate chain` 与 `confirm : Confirm terminal` **并列**。
  这条是 §3 等价性分析的枢纽。
- **`LevelNode`（`:202-206`）携带证明字段** `selected : IsSelected cands chosen` ⟹ `cands = []` 的级别
  在这套里**类型上不可表达**（`IsSelected.mem : J ∈ cands`）。**「无候选」这个状态套三根本构造不出来。**

### 1.4 三套骨架一览

| | 套一 Nat 差 | 套二 列表链 | 套三 Strict |
|---|---|---|---|
| 文件:行 | `Origin/NestingCertificate.lean:185-190` | `Origin/IntervalNestCertificate.lean:416-432` | `Strict/Nest.lean:218-222` |
| 值域 | `Bool` | `Bool` | `Prop` |
| 载体 | `f : Nat → LevelData`（全函数） | `List NestLevel`（携 `lvl`/`confirmOK`） | `List LevelNode`（携 `IsSelected` 证明） |
| 基例数 | 1（`0 => Conf`） | 2 + 2 守卫 | 2 |
| 空/退化时 | 不存在「空」；`ℓ<e` 静默退化为 `Conf` | `[] => false` | `[] => True` |
| Confirm 位置 | 在基例内（`Conf δ (f e)`） | **折进链末元素**（`confirmOK`） | **在 `NestCertificate` 之外**（`Chi.confirm` 独立字段） |
| 装配方向 | top-down | top-down | top-down |
| 终止 | 算术必然 | 可失败运行时检查 | 结构递归（无失败） |
| 独有产出 | 平移不变 `N_translation_invariant` | `selectΘ` 构造性选择器 + 全序三歧性 | `nest_certificate_unique`（locator 键唯一） |

---

## 2. 「刻意不合并」声明：逐字引用 + 领域理由 vs 工程理由

### 2.1 声明 A（套一 → 套二）：`formal/Origin/NestingCertificate.lean:22-32`

```
  既有 `Origin/IntervalNestCertificate.lean`（import Strict.Nest）已实装 N^δ 的**列表链递归**
  （`nestCertB`/`chiBool` 按 `List NestLevel` cons 展开 + Sel_Θ 全序选择器），证了
  χ^δ∈{0,1} + 无候选=0 + Sel_Θ 唯一。但它**没有**：
    (1) **级别差 (ℓ-e):Nat 结构递归**形式（它用列表链，本 spec §6 的方框是对 ℓ 递减、按级别差
        归纳——两者递归骨架不同：列表链 vs Nat 差归纳）；
    (2) **平移不变 N^δ_{ℓ+k↓e+k}(S_k x)=N^δ_{ℓ↓e}(x)**（S_k 级别平移算子；IntervalNestCertificate
        全文无此定理——这是本 spec §5 P5 自相似要求的核心，"区间套在任何级别看起来都一样"）；
    (3) **基例与 Conf^δ_e=⋁B/⋁S 的显式对接**（IntervalNestCertificate 终端用抽象 `confirmOK`
        Bool 标记，未显式写成三类谓词析取）。
  本文件**补**这三项：Nat 差结构递归 + 平移不变 + Conf=析取基例。**不重复** IntervalNestCertificate
  的列表链/Sel_Θ 层（那是另一套递归骨架，单一权威保留在该文件，避免双份漂移）。
```

同文件 `:424-426`：

```
     · 与 `IntervalNestCertificate.nestCertB`（列表链递归 + Sel_Θ）**互补不冲突**：本文件给 Nat 差
       递归骨架 + 平移不变；IntervalNestCertificate 给列表链 + Sel_Θ 全序消歧。两套递归骨架各自
       单一权威（避免双份漂移）；[需人工确认] 是否后续需证两骨架等价（同一 N^δ 的两种实现）。
```

同文件 `:34-40`（不 import 的真实原因，**这段最承重**）：

```
  ★为何不 import `Origin.BuySellPredicate`（Conf^δ_e 的 canonical 家）：经实测
    `lake env lean Origin/BuySellPredicate.lean` **不通过**（line 117 Decidable synth 失败 + line 164
    free-variable 错误），且它用 `Fintype`/`Finset`（本工位硬约束**禁** Fintype/Finset + 纯 core）。
    import 一个不可构建且非纯 core 的文件会破坏本文件构建并违反硬约束。故本文件**不** import 它，
```

**判定：声明 A 是工程理由，不是领域理由。** 三条承重证据：

1. **它自陈两者是「同一 N^δ 的两种实现」**（`:426` 逐字）。同一个对象的两种实现 ⟹ **不是不同东西**，
   ⟹ 不构成「不该合」的领域理由。
2. **给出的三条差异全是形式/覆盖差异**，不是建模对象差异：(1) 递归形式（列表 vs Nat 差）、
   (2) 缺一条定理、(3) 基例写得抽象。这三条的正常处理是**在同一套上补**，不是另起一套。
3. **「避免双份漂移」是标准的工程避险话术**；`:34-40` 那段更是明写工程约束（既有文件构建不通过 + 硬禁
   Fintype/Finset）。**零 import 是本工位的硬约束**（`:54` 逐字「零 import」），零 import 与「合并」在
   技术上互斥 ⟹ 不合并是**约束的后果**，不是领域判断。

**唯一能替声明 A 说话的领域论据**（摆出来供裁）：套一的载体是全函数 `f : Nat → LevelData`，套二是
`List NestLevel`。「级别永不缺席」vs「级别链可缺可断」**确实是两种不同的世界假设**。若有人主张这是
领域差异，站得住的形式是：**套一量的是「理想无限级别塔上的证书」，套二量的是「有限观察窗口内实际显现
的级别链上的证书」**。spec P1（`.chanlun/specs/2026-06-28-recursive-complete-classification-bsp-pdf-extract.md`
P1「去根化的级别系统」，逐字「𝕃 = ℤ…用 ℤ 或 ℕ 上的**有限支撑结构**」）**同时**要求这两件事。
**但两文件都没写这条理由**——它是本报告替它们补的，不是它们的自陈。

### 2.2 声明 B（套二 → 套三）：`formal/Origin/IntervalNestCertificate.lean:26-32`

```
  ★与 `Strict.Nest` 的关系（无重复、无冲突）：
    - Strict.Nest = **唯一性谓词侧**（Prop `IsSelected` + locator 键唯一性，task #72 cc-nest）。
    - 本文件 = **全定义 Bool 算子侧**（可计算 `chiBool ∈ {0,1}` + 构造性 `selectΘ` + 级别比较良基递归）。
    本文件**只读** `Strict.Nest`（复用其 `Interval`/`selKey`/`selOrder`/`IsSelected`/`Sub` 定义，
    不改它），在其上补 Bool 全定义层。
```

同文件 `:8-12`（对套三的实质指控）：

```
    (1) **全定义 Bool 算子 `χ^δ ∈ {0,1}`**：结果包 §6 明确「最终区间套证书 χ^δ = N^δ_{o_v↓e_v}(D_t)
        ∈ {0,1}」「若不存在候选，则值为 0，**绝不能是"未定义"**」。`Strict.Nest.Chi` 是
        `structure`（Prop 见证），**预设** nest/confirm 成立才能构造——**无候选时无法构造**=
        「未定义」，**违反**结果包「绝不未定义」。
```

**判定：声明 B 也是工程理由，但性质更严重——它自己说套三「违反」spec，却选择「不改它」并在旁边另起一套。**

- Prop/Bool 分家在 Lean 里**本身是正当的领域内技术分工**（谓词 + 可判定镜像）。这一半站得住。
- **但它不止是镜像**：它明说套三**违反** spec 的全定义要求（`:11-12`「违反结果包『绝不未定义』」），
  然后 `:29` 明说「**不改它**」。**「旧的写错了，我不改，我在旁边写个对的」= 工程理由**（不敢碰已有证明），
  不是领域理由。
- **合并的正确形态不是删一套**，而是**证一条桥**：`chiBool ev chain = true ↔ (对应的 Prop 侧命题)`。
  **本仓零处有这条桥**（§3.1 实测）。

### 2.3 小结（承重）

**三套之间的「刻意不合并」，两处声明都是工程理由（避免双份漂移／构建约束／不敢改既有证明），
不是领域理由。** ⟹ 按票面给的判据（「前者说明不该合，后者说明该合而没合」），**落在「该合而没合」一侧**。
唯一的领域论据（理想塔 vs 有限支撑窗）是本报告补的，两文件均未自陈，**若要用它当裁定依据须现场重过推理链**。

---

## 3. 三套之间的等价性

### 3.1 有没有等价证明／互推引理？——**零。实测。**

| 检索 | 结果 |
|---|---|
| `NestingCertificate` 在 `IntervalNestCertificate.lean` / `Strict/Nest.lean` 内 | **0 命中** |
| `NestingCertificate` 在 `rust/` 内 | **0 命中** |
| `NestingCertificate` 的全仓消费者 | 只有 `formal/Origin/CandidateSet.lean`、`formal/Origin/SelfSimilarity.lean`（均为 Lean 内部下游，**均不做等价证明**，只调 `N` 当门／复用 `shift`） |
| 套一 ↔ 套二 等价定理 | **不存在**。套一 `:426` 自标 `[需人工确认] 是否后续需证两骨架等价` |
| 套二 ↔ 套三 桥（`chiBool = true ↔ Prop`） | **不存在**。套二只复用套三的 `Interval`/`selKey`/`selOrder`/`IsSelected`/`Sub` **数据层**定义（`:69` `open Strict.Nest (...)`），`subB_iff_Sub`（`:396-402`）只桥了**单条边**的 `Sub`，**没有**桥整条证书 |
| Rust ↔ Lean parity | 有但只到**测试对拍级**：`strategy/nest.rs` 的 `#[test]` 逐条对照 Lean witness 名（`:154`/`:165`/`:176`/`:188`/`:194`/`:244`/`:250`/`:256`/`:268`/`:274`/`:287`）。**这是同值断言，不是等价证明** |

### 3.2 空链基例 `[] => false` vs `[] => True`：是矛盾还是同名异物？

**结论：两者都不完全对，要分三层说。**

**第一层——定义域**：`nestCertB : Nat → List NestLevel → Bool` 与 `NestCertificate : List LevelNode → Prop`。
`NestLevel`（`IntervalNestCertificate.lean:375-379`）＝ `{lvl : Nat, cands, candidateOK, confirmOK}`；
`LevelNode`（`Strict/Nest.lean:202-206`）＝ `{cands, chosen, selected : IsSelected cands chosen, Candidate : Prop}`。
**字段不重合、无转换函数、一个带证明一个带级别号。** 非空输入上**根本没有共同的输入**
⟹ 在非空段上，这是**同名异物**（两个都叫 χ^δ 的不同函数）。

**第二层——空链是唯一可对比点**。`[] : List α` 对两个元素类型都存在且规范。所以：

```
NestCertificate ([] : List LevelNode) = True     -- Strict/Nest.lean:219
nestCertB ev    ([] : List NestLevel) = false    -- IntervalNestCertificate.lean:417
```

**具体输入（手算，不跑 Lean）**：取「空级别链 + 终端第二类买点成立」这一状态，即
`Λ` 满足 `Λ BSPClass.second = true`，其余 false。

- **套三**：`Chi` 的字段是 `chain := []`、`terminal := Λ`、`nest := NestCertificate [] = True`（`True.intro` 免费给）、
  `confirm := Confirm Λ`。而 `Confirm Λ = nonEmpty Λ`（`:236-246`），`Λ second = true` ⟹ `Confirm Λ` 成立
  （`confirm_of_two_three` 的同型，`:257-260`）。**⟹ `Chi` 可构造 ⟹ χ ＝ 1。**
- **套二**：`chiBool ev [] = false`（有专门定理 `chiBool_empty_eq_zero`，`:452`，`:= rfl`）。**⟹ χ ＝ 0。**

**同一非形式输入，两套给 1 和 0。这个相反是真的。**

**第三层——成因归因（最承重的一句）**：这个相反**不是对「嵌套是否成立」的相反判决**，而是
**Confirm 摆在哪的口径差**：

- 套三把 `Confirm` 摆在 `NestCertificate` **外面**（`Chi.confirm` 独立字段）⟹ `NestCertificate []` 只表达
  「没有级别约束」，**空合取＝True 是数学惯例**，它没在说「无候选也算确认」。
- 套二把 `confirmOK` **折进链末元素** ⟹ 空链意味着「连执行级终端都没有」⟹ `false` 是对的。

**⟹ 若一定要一句话定性：非空段是同名异物；空链段是真矛盾，但矛盾的根在「Confirm 的归属层」，
不在嵌套判据。** 按「矛盾」去修（改一个基例的符号）会修错地方；正确的裁法是先定 Confirm 归哪层
（这条与 N-5/N-6 耦合，见 §8）。

**另一条独立事实（削弱「矛盾」的严重性）**：套三的 `LevelNode` 带 `selected : IsSelected cands chosen`，
而 `IsSelected.mem : J ∈ cands` ⟹ **`cands = []` 的级别在套三类型上不可表达**。所以套二那句
「无候选 ⟹ 0」所针对的病理，在套三里**根本不是一个可表达的状态**。套二对套三的指控
（`:11-12`「无候选时无法构造 ＝ 未定义」）**技术上准确**，但它描述的是**类型层拒绝**而非
**运行时未定义**——两者认识论地位不同（前者是「构造不出来」，后者是「函数没值」）。这一区分
**票面与套二的自陈都没做**。

---

## 4. Rust 侧的装配方向：四条路径 + bottom-up 基线的下落

| # | 路径 | 位置 | 方向 | 生产可达 |
|---|---|---|---|---|
| **R1** | 递归塔 Compose | `rust/src/theta_v0/classifier/recursive_tower.rs`（模块头 `:27-31`） | **往上合成**（L(k) 三段 → L(k+1) 一段） | ✅ 是（全流水线基座） |
| **R2** | `N^δ` 证书构造 + 判定 | 构造 `backtest/econ_positive.rs:995-1088`（`build_nest_certificate`），判定 `classifier/nest.rs:335-362`（`n_delta`/`n_delta_rec`） | **收集往上、校验往下**（见下） | ✅ 是（门 `build_multilevel_nest_cert` `:783`，调用点 `:359`） |
| **R3** | 一类锚下钻 | `backtest/econ_positive.rs:816-843`（`descend_type1_anchor_depth`；票面写 822-843，**订正为 816-843**——`fn` 签名起于 816） | **往下钻**（`s.sub_moves` 逐级下沉，自递归 `:839-842`） | ✅ 是（Type2/3 的 base gate：`:927`/`:946`，被 `build_nest_certificate:1022` 调用） |
| **R4** | `NestInterval` 链（列表链镜像） | `strategy/nest.rs:126-146`（`chi_bool`） | **top-down**（链头＝操作级；`[head, next, rest..]` 递归进尾） | ⚠️ **形式可达、递归不可达**（见下） |
| **R5** | **bottom-up 对照基线** | `backtest/econ_positive.rs:1143-1224`（`build_nest_certificate_bottomup`） | **往上加宽**（`:1212` 逐字「`child = interval_k;` 加宽：下一级用本级 J_k 作 child（真 bottom-up 递归）」） | ❌ **`#[cfg(test)]`，非生产** |

### 4.1 R2 的方向：收集往上、校验往下（E-2 的实测依据）

- **收集**：`econ_positive.rs:1031` `for k in (lvl + 1)..max_k`，`rung_buf.push(...)`（低→高）；
  `:1066` `rung_buf.reverse();`，注释 `:1065` 逐字「n_delta 期望 rungs[0]=最高级，rungs[last]=lvl+1 级——
  rung_buf 是低到高，需反转」。
- **校验**：`classifier/nest.rs:340-362` `n_delta_rec`，`match rungs.split_first()`：
  `None => terminal.confirm_side(side)`（基例 ℓ=e）；`Some((top, rest)) => top.cand && is_sub(child, &top.interval)
  && n_delta_rec(..., rest)`。注释 `:339` 逐字「`rungs` 从高(ℓ)到低(e+1)」。
- **⟹ 真正的递归是 top-down，与记号 `N^δ_{ℓ↓e}` 一致。** `for k in (lvl+1)..max_k` 是**建链的遍历顺序**，
  不是证书的递归方向。

### 4.2 「bottom-up 标为对照基线」：在哪儿、谁标的、还活不活

- **在哪儿**：`rust/src/theta_v0/backtest/econ_positive.rs:1130-1224`。
- **谁标的**：函数 docstring `:1130` 逐字「**阶段0 对拍探针（cfg(test)，NO-SHIP）**：PDF §二
  「最严格实现 = bottom-up」区间套构造」；`:1140-1141` 逐字「差异>0 ⟹ 塔在某处非严格 refinement，
  须把生产改成 bottom-up（PDF §五最终裁决 b）」。⟹ **标注人＝ task #101（bottomup-nest），
  依据＝「区间套.pdf」§二/§五裁决 b**（该 PDF 属推论层，ADR 0012 意义上须现场重过推理链）。
- **还活不活**：**活，但只作为 `#[ignore]` 的等价固化锁，不是候选实装。**
  唯一调用点 `:6322`，在测试 `acc_bottomup_nest_parity_probe`（`:6185-6200` docstring + `#[test] #[ignore]`）内。
  该测试硬断言差异为 0（`:6444`「Some/None 分歧：生产 descend 与 bottom-up 定位存在性不一致」、
  `:6448`「Γ 成员分歧：n_delta 不一致——现口径非 PDF bottom-up 等价，须实装 bottom-up」）。
  与 MEMORY `project_bottomup_nest_equals_pointcontain`（BTC 三窗 bit-exact 差异 0，NO-SHIP）一致。
  **默认不跑**（`#[ignore]` + 需 `--release` + BTC 全量）。
- **⟹ 票面「bottom-up 标为对照基线」成立，但要补两句：它是 `cfg(test)` 探针不是备选实装；
  它的活性是「等价已固化、生产无需改」而不是「候选待选」。**

### 4.3 R4 的递归为何不可达（N-2 已判，本报告独立复核成立）

`strategy/interp.rs:569-587` `nest_confirm` 唯一地构造链：

```rust
let chain = [NestLevel {
    lvl: level,
    cands: vec![Interval::new(source_index as u64, source_index as u64, 0)],
    candidate_ok: true,
    confirm_ok,
}];
nest::chi_bool(level, &chain)
```

**单元素数组字面量，且 `ev == level == chain[0].lvl`** ⟹ `chi_bool` 恒走 `[last]` 分支
（`strategy/nest.rs:129`），`cands` 恒非空 ⟹ 恒等于 `confirm_ok`。
两个生产调用点：`interp.rs:459`、`interp.rs:1256`。
**⟹ `[] => false` 分支（`:128`）与 `[head, next, rest..]` 递归分支（`:130-144`）在生产上均不可达。**

---

## 5. 记号 `N^δ_{ℓ↓e}` vs 实装方向：方向冲突还是命名冲突？

### 5.1 记号出处（逐字）

**正本级出处**：`.chanlun/specs/2026-06-28-recursive-complete-classification-bsp-pdf-extract.md:277-279`
（信源 `:5`＝`/Users/silencehan/Downloads/递归完全分类买卖点.pdf`，`:7` 由编排者判为**权威版**，
超越 15 页《买卖点.pdf》）：

```
    N^δ_{ℓ↓e}(x) = ⎧ Conf^δ_e(x),                                                          ℓ = e,
                    ⎨
                    ⎩ Cand^δ_ℓ(x) ∧ [ J^δ_{ℓ-1}(x) ⊆ J^δ_ℓ(x) ] ∧ N^δ_{ℓ-1↓e}(x),       ℓ > e.
```

同文件 `:265`（ℓ 与 e 的角色，逐字）：「令执行级别为 **e ≤ ℓ**。定义区间套证书：」
同文件 `:311`（**spec 自己就规定了 Rust 侧的方向**，逐字）：
「**Rust 端按 ℓ 从高到低逐级校验区间包含 + 候选 + 确认。**」

**溯源标签**：记号本体 `N^δ_{ℓ↓e}` ＝ **`[新缠论:选择]`**（记号是这份 2026-06-28 PDF 的自造符号，
非缠师原文；该 PDF 页眉「推导完全分类」＝异质源产出的形式化推导）。它**编码的内容**
（逐级下钻定位）＝ **`[旧缠论]`**（`027:36`/`027:44`【正文】，见 §8）。
按 ADR 0012，`.chanlun/specs/` 不在五类既有名分内 ⟹ 属**推论层**，援引须现场重过推理链——
本节即重过：从 spec `:265` 的 `e ≤ ℓ` + `:277` 的分段（ℓ=e 是基例）读出 **ℓ＝顶、e＝底**。

### 5.2 实装那段（逐字）

`rust/src/theta_v0/backtest/econ_positive.rs:1031`：`for k in (lvl + 1)..max_k {`
同文件 `:1004` 注释逐字：「执行级 tower[lvl] 中找 end_index == source_index 的段（候选段 s，**执行级 e=lvl**）」
同文件 `:1042` 注释逐字：「N^δ_{ℓ↓e} 的**执行级 e=信号级 lvl**（非 tower 顶）」

### 5.3 诊断：**命名冲突，不是方向冲突**

| | 记号 | 实装 |
|---|---|---|
| 顶（起点） | `ℓ` | `max_k - 1`，且**是发现出来的**（`:1039` `break` 于首个无包含段），不是入参 |
| 底（终点） | `e` | **`lvl`**（＝入参，注释 `:1042` 明写 `e = lvl`） |
| 递归方向 | ℓ ↓ e | `n_delta_rec` rungs[0](=ℓ) → base(=e)，**同向** |
| `for k in (lvl+1)..max_k` 的角色 | — | **建链遍历**，收完 `:1066` `reverse()` |

**⟹ 记号里的 `ℓ` ≠ 实装里的 `lvl`。`lvl` 对应的是记号里的 `e`。** 这是**命名冲突**：
同一个字母（ℓ / lvl）在两处指相反的端点。方向本身**不冲突**——spec `:311` 要求「按 ℓ 从高到低逐级校验」，
实装 `n_delta_rec` 正是从高到低。

**修法差异（票面要求分清的那件事）**：
- 若是方向冲突 ⟹ 要改算法（改 `n_delta_rec` 的遍历序或改 spec）＝ **翻掉全部在案 Γ 读数**。
- 若是命名冲突 ⟹ 改注释／把 `lvl` 改名 `exec_lvl`／在 spec 侧加一句「Rust 侧 `lvl` 即 `e`」＝
  **零行为改动，零读数翻转**。**本条实测是后者。**

### 5.4 一条真差异（顺带查出，不在票面）

**记号里 `ℓ` 是输入，实装里 ℓ 是输出。** spec `:277` 的 `N^δ_{ℓ↓e}` 对给定的 (ℓ, e) 取值；
实装 `build_nest_certificate` 只收 `lvl`(=e)，顶端由 `:1039` 的 `break` **动态发现**（partial chain 合法，
`:1035-1045` 注释逐字「**设计选择：partial chain 合法**（codex #39 Q1 裁定）…故 break 而非 return None」）。
⟹ **实装算的不是 `N^δ_{ℓ↓e}`，而是 `N^δ_{ℓ*(e)↓e}`，其中 ℓ*(e) ＝ 首个无包含段之下那一级。**
这条在三套 Lean 骨架里**一条都没有对应物**（N-2 §3.5 说法①对勘：三套全「否」）。
**归属存疑**：既可归 N-4（装配方向／链的端点谁定），也可归 N-2（停止判据）。**本报告只登记，不裁。**

---

## 6. 可达性判定（本条依 Notes N-1 预授权判定）

### 6.1 三套 Lean 骨架里哪套是生产 Rust 的契约锚？

| Rust 文件 | 自陈契约锚（逐字） | 核实 |
|---|---|---|
| `strategy/nest.rs:3` | 「契约锚：`formal/Origin/IntervalNestCertificate.lean`」 | ✅ **双向对齐**。套二 `:714` 逐字回指「rust 对照实装：`rust/src/theta_v0/strategy/nest.rs`（select_theta + chi_bool + selKey 全序，parity）」。且 `chi_bool` 与 `nestCertB` **逐分支同构**（四支一一对应，连注释文字都一致） |
| `classifier/nest.rs:1` | 「契约重锚 `Origin.SubLevelDescent`」 | ⚠️ **锚到第四个文件**（不在三套内）。而 `SubLevelDescent.descend` 是**一层展开、不自递归**（N-2 §3.4 实测），**扛不起多级证书** |
| `classifier/nest.rs:328` | 「Lean 端对应 (ℓ-e):Nat 结构递归」 | ❌ **悬空**。这句指向套一，但**套一在 Rust 侧零命中**，且 `n_delta_rec` 实装是**列表递归**（`rungs.split_first()`）——**骨架与它声称对应的那套不同** |
| `strategy/mod.rs:91` | 「对照 `Origin.IntervalNestCertificate`」 | ✅ 与 `strategy/nest.rs` 一致 |

**⟹ 三套里只有套二（列表链 `IntervalNestCertificate`）是真契约锚，且锚的是 R4（生产递归不可达的那条）。
套一（Nat 差 `NestingCertificate`）在 Rust 侧零引用——它不是任何生产路径的契约锚。
套三（`Strict/Nest`）只作数据层被套二 `open`，Rust 侧的 `NestInterval`/`sel_order`/`is_sub`
（`classifier/nest.rs:41-75`）逐字对齐它的 `Interval`/`selOrder`/`Sub`。**

**★ 一处内部不一致（承重，不在票面）**：`classifier/nest.rs` 一个文件里有两个互不相容的锚——
模块头锚 `SubLevelDescent`（一层下钻），`n_delta` docstring 锚套一（Nat 差），而实装是列表递归（套二骨架）。
**三者对不上。** 这是 MEMORY `project_unified_architecture_map_787`「根因非重复造轮子而是没人知道已经有什么」
的又一标本。

### 6.2 Rust 四条装配路径的生产可达性

| 路径 | 生产可达 | 保证它的那一行 |
|---|---|---|
| R1 塔（往上） | ✅ | `classifier/mod.rs::classify` 建塔（模块头 `recursive_tower.rs:5-9`） |
| R2 `N^δ` 证书（收集上／校验下） | ✅ | 门 `econ_positive.rs:783 build_multilevel_nest_cert`；调用点 `:359` 注释逐字「调 build_multilevel_nest_cert：从塔构造 rungs 链，调 NestCertificate::n_delta()」 |
| R3 `descend_type1_anchor_depth`（往下） | ✅ | `econ_positive.rs:927`/`:946`（Type2/3 base gate），经 `:1022 cand_delta_base_gate` 进 R2 |
| R4 `chi_bool`（列表链） | ⚠️ **顶层可达、递归分支不可达** | 入口 `interp.rs:459`/`:1256` → `interp.rs:580-586` 单元素链；**`strategy/nest.rs:128`（`[] => false`）与 `:130-144`（递归）零可达** |
| R5 bottom-up 基线 | ❌ **生产不可达** | `econ_positive.rs:1142` `#[cfg(test)]`；唯一调用点 `:6322` 在 `#[test] #[ignore]` 内 |

### 6.3 可达性对 N-4 差异的定性（本条依 Notes N-1 预授权判定）

1. **`[] => false`（套二）vs `[] => True`（套三）＝ 伪分歧（生产不可达）。**
   点名到保证它的那一行：`strategy/interp.rs:580-585`——链是**单元素数组字面量**，`[]` 分支永不进入；
   套三根本不在 Rust 侧被实例化（只有它的数据层定义被镜像）。
   ⟹ 主张这条差异有生产后果的一方，**必须先点名一条能构造空链的生产代码**。本报告找不到。

2. **套一 vs 套二「两套骨架」＝ 生产层面伪分歧。**
   点名：套一在 `rust/` 零命中；`classifier/nest.rs:328` 那句「对应 Nat 差递归」是**悬空注释**，
   实装用的是列表递归。⟹ **改套一对生产零影响**（唯一 Lean 侧下游是 `CandidateSet.lean`、
   `SelfSimilarity.lean`，两者都不进 fixture 闭包，见 §7）。

3. **真分歧只有一条（生产可达）**：§5.4 的「ℓ 是输入还是输出」——`econ_positive.rs:1039` 的
   `break`（partial chain 合法）在**生产门上真起作用**（MEMORY `project_interval_nesting_not_called_in_backtest`：
   95.36% 信号 rungs 空 ⟹ `break` 在第一步就发生）。**这一条不能判伪分歧。**

---

## 7. fixture 漂移门：独立复核 N-2 的「三套皆不在闭包内」

**方法**：自写导入闭包遍历（正则抓 `^import <mod>`，模块名→路径，传递闭包），起点为两个导出器。

```
Origin.ParityFixtureExport 闭包（17 个模块）：
  Lean.Data.Json, Origin.BspClassification, Origin.CenterStates, Origin.ChanlunElements,
  Origin.ClassifierFamily, Origin.CompleteClassification, Origin.Divergence,
  Origin.FullDefinitionStrategy, Origin.ParityFixtureExport, Origin.RiskProj,
  Origin.SegmentFeatureSeq, Origin.SellClosedLoop, Origin.SellPointRecog,
  Origin.SourceAxioms, Origin.StrategyFamily, Origin.ThetaInstantiation,
  Origin.TrendCompleteClassification

Origin.CenterConstruct 闭包（8 个模块）：
  Lean.Data.Json, Origin.CenterConstruct, Origin.CenterConstruction, Origin.CenterFull,
  Origin.CenterStates, Origin.ChanlunElements, Origin.CompleteClassification, Origin.SourceAxioms
```

| 模块 | 在 ParityFixtureExport 闭包 | 在 CenterConstruct 闭包 |
|---|---|---|
| `Origin.NestingCertificate`（套一） | ❌ | ❌ |
| `Origin.IntervalNestCertificate`（套二） | ❌ | ❌ |
| `Strict.Nest`（套三） | ❌ | ❌ |
| `Origin.SubLevelDescent`（第四文件） | ❌ | ❌ |

**⟹ N-2 的结论独立复核成立：三套（加第四个）皆不在两个导出器的 import 闭包内
⟹ 改它们不须重落 fixture。**

补两条核实：
- 导出器落点核实：`formal/Origin/CenterConstruct.lean:635` `#eval IO.println refV1FixtureJson.compress`
  （票面写 `:634`，那是它上一行的注释；**订正为 :635**）。
- **但改 `formal/` 仍须跑 gate**：`CLAUDE.md` 的节拍要求「凡改动 `formal/` 的 session，收尾必跑
  `python3 scripts/check_fixture_drift.py`」——**不是「不在闭包内就免跑」**。区别是：
  跑了应当 exit=0（无漂移），而不是免跑。**代价量级＝一次冷构建的时间，不是重落 fixture + 翻读数。**

---

## 8. 原文考据 + 形态清单 + 反例自评

### 8.1 缠论 110 课对「装配方向」的说法

**引文均已核实在该课 `---------↑正文---------` 标记之前（＝【正文】）还是之后（＝【答疑】）。**

**往下（定位）——【正文】层有明文，且明确点名区间套：**

- `docs/chanlun/text/blog/027-第27课.md:36`【正文】（↑正文在 `:78`）逐字：
  > 学过数学分析的，都应该对区间套定理有印象。**这种从大级别往下精确找大级别买点的方法**，和区间套是一个道理。
- `027:44`【正文】逐字（定理）：
  > 定理：某大级别的转折点，可以通过不同级别背驰段的逐级收缩范围而确定。
- `027:46`【正文】逐字：
  > 换言之，某大级别的转折点，先找到其背驰段，然后**在次级别图里**，找出相应背驰段在次级别里的背驰段，
  > 将该过程反复进行下去，**直到最低级别**，相应的转折点就在该级别背驰段确定的范围内。
- `017-第17课.md:66`【正文】（↑正文在 `:98`）逐字（买卖点定律一）：
  > 「缠中说禅买卖点定律一」：任何级别的第二类买卖点都由**次级别**相应走势的第一类买点构成。
- `017:68`【正文】逐字：
  > 任何由第一、二类买卖点构成的缠中说禅买卖点，都可以**归结到**不同级别的第一类买卖点。

**往上（构造）——【正文】层也有明文，但从不叫区间套、也不用于定位：**

- `033-第33课.md:16`【正文】（↑正文在 `:52`）逐字：
  > 站在30分钟级别的中枢角度，**3个5分钟级别的走势重合就形成了**，而9段以上的1分钟次级别走势，
  > 每3段构成一个5分钟的中枢，这样也就可以解释成这是一个30分钟的中枢。
- `021-第21课.md:34`【正文】（↑正文在 `:74`）逐字：
  > 中枢扩张**导致一个更大级别的中枢**，而中枢新生，就形成一个上涨的趋势
- `065-第65课.md:182`【答疑】（↑正文在 `:56`，故此行在标记之后）逐字：
  > 两者在递归的形式上是一样的，都是 an=f(an-1)，唯一不同的就是预先给出的 a0
  （**这是往上递归的最干净表述，但只在答疑层**——MEMORY `project_unified_recursive_operator_T` 引它时
  未标层，**本报告订正：`065:182` 属【答疑】**）

**判定：往上合成与往下钻在原文里是两个独立机制，不是一个机制的两个读法。** 承重三点：

1. **词不同**：往下那支缠师**明确命名**为「区间套」（`027:36`【正文】）；往上那支从不叫区间套，
   叫「构成更大级别」「扩张／新生」「an=f(an-1)」。
2. **功能不同**：往下的产物是**一个位置**（转折点落在哪个区间，`027:44`「确定」）；
   往上的产物是**一个对象**（更大级别的中枢／走势，`033:16`「形成」）。定位 vs 构造。
3. **前提不同**：往下要求**先有一个高级别背驰段**（`027:46`「先找到其背驰段，然后在次级别图里」）；
   往上要求**先有三段同级别走势重合**（`033:16`）。**互为前提但不互为逆**：
   Rust 侧唯一声称的对偶关系是 `recursive_tower.rs:30` 逐字「组装-取回对偶 `descend ∘ compose = id`」，
   **这是一层的对偶，不是整条链的等价**（Lean 侧 `SubLevelDescent.descend` 只有一层，N-2 §3.4 实测）。

**与 map #787 三格表的关系**（正文已核，`gh issue view 787`）：该表把「纵向·往上」配塔、
「纵向·往下」配树、「横向」＝同级别分解且**本仓为空**。**本节的原文考据与该表一致**，
并补上表没有的那句：**「纵向往上」与「纵向往下」在原文里也不是同一机制的两读，是两个机制**
（表只说了它们是「两个方向」，没说它们不互为逆）。

**⟹ 对 N-4 的直接后果**：「装配方向」这个提法本身要拆。原文里没有「一个可正可反的装配方向」，
只有「构造（必往上）＋ 定位（必往下）」两件事。**问「装配方向是 top-down 还是 bottom-up」是个
有歧义的问题**——若指定位，原文答案是 top-down 且【正文】层明确；若指构造，答案是 bottom-up 且
【正文】层同样明确。**R5 那个 `build_nest_certificate_bottomup` 干的是「用往上的方式做定位」**——
在原文层**没有依据**（推论层的「区间套.pdf §二最严格实现」是它唯一依据，ADR 0012 意义上须重过推理链）。

### 8.2 形态清单

| 形态 | 内容 | 改哪些码／Lean | 翻掉哪些在案读数 | 与 N-5/N-6 耦合 | 代价量级 |
|---|---|---|---|---|---|
| **F-1 三套并存 + 各自声明有效域**（票面点名的那一支） | 不合并。三个文件头各加一节「有效域 + 我不管什么」：套一＝理想全级别塔上的 L0 结构（含平移不变）；套二＝有限支撑级别链上的 Bool 全定义算子（唯一生产契约锚）；套三＝Prop 唯一性侧（数据层供给者）。**并明写「无等价证明」是已知开口，不是遗漏** | 3 份 Lean 文件头（**注释级**）；`classifier/nest.rs:1` 与 `:328` 两个矛盾锚订正为套二 | **零** | N-5/N-6 **零**耦合 | **最小**。零行为改动、fixture 跑一次应 exit=0 |
| **F-2 补桥不合并** | F-1 + 证一条 `chiBool ev chain = true ↔ <Prop 侧>`（套二↔套三）和／或 `nestCert` ↔ `nestCertB` 在「链完整且级别连续」前提下的等价引理 | 新增 Lean 定理（套二内，不改套三）。**须先造 `NestLevel → LevelNode` 的转换，而该转换要造 `IsSelected` 证明 ⟹ 只在 `cands ≠ []` 时可造** ⟹ 桥必带前件 | 零 | **N-6 中耦合**（桥的前件就是一个新的严格度维度） | 中。纯 Lean 工作量，L0，零生产风险 |
| **F-3 定 Confirm 归属层，消掉空链分歧** | 裁「Confirm 在证书内还是外」。若定「在内」（套二口径）⟹ 套三 `[] => True` 需重述为「`NestCertificate` 只是嵌套部分，χ 才是全部」；若定「在外」（套三口径）⟹ 套二的 `[] => false` 需拆成 `nest([]) = True ∧ confirm` | 1-2 处 Lean 基例 + 文件头 | 零（生产不可达，§6.3-1） | **N-5 强耦合**（Cand^δ 的定义式缺失使「嵌套部分」边界本身不清） | 中低。**但这是 §3 那个「相反」的真正修点** |
| **F-4 合并成一套** | 删两套留一套（留套二＝唯一生产锚）。套一的平移不变定理须移植到列表链骨架上 | 删 `NestingCertificate.lean` ⟹ **连带改 `CandidateSet.lean`（`:58` import + 6 处引用）与 `SelfSimilarity.lean`（`:73` import + `:21`/`:47`/`:109`）**；平移不变在列表链上重证（`shift` 对 `List NestLevel` 无自然定义 ⟹ **可能证不出来**） | 零生产读数；但**翻掉 `SelfSimilarity.lean` 的自相似三件套之一** | N-5/N-6 弱 | **最大**。且有「移植可能失败」的实质风险——平移不变是「只依赖级别差」的性质，列表链骨架里级别是显式字段 `lvl`，**平移会真改字段值**，不变性不自动 |
| **F-5 记号侧改名（E-2 的配套）** | spec 侧或 Rust 侧统一：`econ_positive.rs` 的 `lvl` 改名 `exec_lvl`／`e`；或在 `.chanlun/definitions/qujiantao.md` 加一条「Rust `lvl` ＝ 记号 `e`」译注 | 若只加译注：改 `qujiantao.md`（正本层，允许改）1 处；若改 Rust 参数名：`econ_positive.rs` 约 6 处 + `classifier/nest.rs` docstring | 零 | 零 | **最小**。ADR 0012 裁定五的标准做法（正本层加译注，推论层原文不动） |
| **F-6 认「ℓ 是输出」这条真差异（§5.4）** | 把「顶端动态发现（partial chain）」写进正本，或反过来要求给定 ℓ | 若要求给定 ℓ：改 `build_nest_certificate` 签名 + 所有调用点 ⟹ **改门行为** | **翻掉全部在案 Γ／μ 读数**（95.36% rungs 空这条数就是 `break` 的产物） | **N-2 强耦合**（这是「停在哪一级」的对偶问题：停在哪一级往上） | **高**（若改行为）／**低**（若只写进正本） |

**耦合总表**：F-1 与 F-5 可无条件先做（零行为、零读数）。F-3 是 §3「相反」的唯一正确修点，但要等
N-5 定 `Cand^δ`。F-4 有失败风险且收益只是「少一个文件」。F-6 的归属应先与 N-2 对齐（可能不属 N-4）。

### 8.3 反例自评：本报告最可能错在哪

1. **最可能被打回的一处：§5.3 判「命名冲突不是方向冲突」。**
   我的依据是 `econ_positive.rs:1042` 注释自陈 `e = lvl` + `:1066` 的 `reverse()` + `n_delta_rec` 的
   `split_first()` 遍历序。**反方可以主张**：`reverse()` 之后的「从高到低」是**数据布局**上的从高到低，
   而 `is_sub(child, &top.interval)` 里的 `child` 取自 `rest.first()`（`classifier/nest.rs:353-355`），
   即**先看到父再回头取子**——这在算法上仍是「从父往子」还是「从子往父」，取决于你认哪一步是「装配」。
   **我认为这个反驳站不住**（递归的调用方向明确是 rungs[0] 先、rest 后），但它暴露了一个真问题：
   **「装配方向」这个词在本仓从没被定义过**。若裁定要用这个词，**先定义它是「求值顺序」还是「数据构造顺序」**
   ——本报告全程按「求值/递归调用顺序」读，这是我的选择，不是码里写的。

2. **§8.1 判「两个独立机制」用的是「词不同/功能不同/前提不同」三条，这是我的分类标准，不是原文的。**
   原文从未说「这是两个机制」。反方可主张：`017:66` 定律一（二类点由次级别一类点构成）既可读成往下分解、
   也可读成**往上构成**（「构成」二字字面就是往上）——**这个反驳我认为站得住一半**。
   `017:66` 的「构成」在语法上是「A 由 B 构成」＝往上，但用法（`017:68`「归结到」）是往下。
   **⟹ `017:66` 单独拎出来不足以定方向，须与 `017:68` 连读。本报告已连读，但这是解读。**

3. **§7 的闭包是我自写正则算的，不是 `lake` 算的。**
   若某文件用了 `import` 之外的方式引入（如 `open` 跨库、`include`），我的遍历会漏。
   **缓解**：我另做了 repo-wide grep（`NestingCertificate` 全仓命中只有两个 Lean 消费者），
   两条独立路径同结论。**但「lake 真闭包」我没跑**（跑要冷构建）。**测不出来就写测不出来：
   本条是两条弱证据的合流，不是 lake 级铁证。**

4. **§6.3 的「伪分歧」判定依赖「生产入口只有 `interp.rs:459`/`:1256` 两处」。**
   我用 `grep -rn "nest_confirm("` 得到 3 命中（含定义）。**若有经 trait 对象／函数指针的间接调用，
   grep 会漏。** 本仓 `chi_bool` 是自由函数、`nest_confirm` 是 `pub(crate)` 自由函数，间接调用可能性低，
   **但我没做调用图分析。**

5. **§2 判两处声明都是「工程理由」——这是本报告最主观的一处。**
   我的判据是「它自陈是同一对象的两种实现」＋「给出的差异全是形式差异」＋「明写构建约束」。
   反方最强的反驳：**Prop/Bool 分家在 Lean 生态里是公认的正当分工**（`Decidable` 镜像模式），
   按这个标准声明 B 就是领域理由。**我承认这一半**——所以 §2.2 明写「Prop/Bool 分家这一半站得住」，
   把「不站得住」的部分收窄到「它自己说对方违反 spec 却选择不改」。**若裁定者认为
   『在旁边写个对的、不碰旧的』在形式化工程里是正当做法，那 §2 的结论应改判为「领域理由 + 工程理由各半」。**

---

## 附：本报告核对过的每一处（可复查清单）

| 断言 | 文件:行 |
|---|---|
| 套一递归定义 + 唯一基例 | `formal/Origin/NestingCertificate.lean:185-190`，入口 `:199-200`，`Conf` `:123-126` |
| 套一平移不变 | `formal/Origin/NestingCertificate.lean:303-308` |
| 套一「刻意不合并」声明 | `formal/Origin/NestingCertificate.lean:22-32`、`:424-426`、`:34-40` |
| 套二递归定义 + 两基例两守卫 | `formal/Origin/IntervalNestCertificate.lean:416-432`（**订正**，票面 416-431） |
| 套二 χ 入口 / 空链定理 | `:440` / `:452` |
| 套二对套三的声明 | `formal/Origin/IntervalNestCertificate.lean:26-32`、`:8-12`、`:714` |
| 套三递归定义 + 两基例 | `formal/Strict/Nest.lean:218-222`（票面行号准确），自陈 `:215-216` |
| 套三 χ ＝ 𝒩 ∧ Confirm（Confirm 在外） | `formal/Strict/Nest.lean:290`、`:298-303` |
| 套三 `LevelNode` 带证明字段 | `formal/Strict/Nest.lean:202-206`、`IsSelected` `:131-133` |
| R1 塔往上 + descend∘compose 对偶 | `rust/src/theta_v0/classifier/recursive_tower.rs:27-31`、`:30` |
| R2 收集循环 + reverse + 校验递归 | `rust/src/theta_v0/backtest/econ_positive.rs:1031`、`:1065-1066`；`classifier/nest.rs:335-362`（`:339` 注释） |
| R2 e=lvl 自陈 | `econ_positive.rs:1004`、`:1042` |
| R2 partial chain 设计选择 | `econ_positive.rs:1035-1045` |
| R3 下钻自递归 | `econ_positive.rs:816-843`（**订正**，票面 822-843），自递归 `:839-842`，base gate `:927`/`:946` |
| R4 列表链 + 单元素塌缩 | `strategy/nest.rs:126-146`；`strategy/interp.rs:569-587`（链 `:580-585`），调用点 `:459`/`:1256` |
| R5 bottom-up 基线 | `econ_positive.rs:1130-1224`（`#[cfg(test)]` `:1142`，加宽 `:1212`），唯一调用点 `:6322`，测试 `:6185-6200` `#[test] #[ignore]` |
| 契约锚 | `strategy/nest.rs:3`、`strategy/mod.rs:91`、`classifier/nest.rs:1`、`classifier/nest.rs:328` |
| 记号出处 | `.chanlun/specs/2026-06-28-recursive-complete-classification-bsp-pdf-extract.md:277-279`、`:265`、`:311`、`:5`、`:7` |
| 导出器落点 | `formal/Origin/ParityFixtureExport.lean`、`formal/Origin/CenterConstruct.lean:635`（**订正**，票面 :634） |
| 原文往下 | `blog/027-第27课.md:36`/`:44`/`:46`【正文】（↑正文 `:78`）；`blog/017-第17课.md:66`/`:68`【正文】（↑正文 `:98`） |
| 原文往上 | `blog/033-第33课.md:16`【正文】（↑正文 `:52`）；`blog/021-第21课.md:34`【正文】（↑正文 `:74`）；`blog/065-第65课.md:182`【答疑】（↑正文 `:56`） |
| map #787 三格表 | `gh issue view 787` 正文（双向递归节，含 2026-08-01 #827 订正） |
