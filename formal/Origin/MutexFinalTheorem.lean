/-
  Origin/MutexFinalTheorem.lean — M29 互斥全定义策略最终定理（goal 形式化顶点，三结论合一）

  ── ★去根化偏离声明（20页 spec paradigm 对照，2026-06-28）──────────────────────────────
  **本文件属 15页 paradigm**（M29 三结论合一集成 15页互斥定理链：W14 H + MW8 R + MW4 Σ指示=1，
  全部建立在 `LevelRelation = Root/Same/Sub` + `OperationRole = RootDir/...` 含根特例的分类轴上）。
  **20页权威 paradigm 在 `Origin.OperationRole18`**（18 类 R=H×V×δ，去根化：根从分类轴特例
  消失，被边界胚元 ∂ + Ambient 吸收）。

  - **有效域**：M29 在 15页定义域内是 L0 语法层完备性定理（角色互斥 + 全元素覆盖 + 策略全定义）。
    20页去根化表明其分类基底（含 RootDir/Root 特例）非权威——有效域严格小于 15页定义域声称
    （231号：有效域 ⊊ 定义域）。M29 与 v1 全窗 L3 8/8 否证**不矛盾**（两个有效域，分类完备性
    ≠ 择时盈利）。
  - **域差注明（#240 终裁第4条，#239 实写）**：Lean 量化域（全语法元素 E）大于 rust 实现域
    （活跃候选子域，`rust/src/theta_v0/strategy/coverage.rs:2145-2238` 在案）；「全域是否承重」
    为 #59 交接待查项。
  - **直接矛盾点**：M29 集成的分类基底含 `RootDir`/`Root` 特例，与 20页去根化**直接矛盾**。
    本文件保留该基底是 15paradigm 的历史存在。
  - **为何保留**（no-patch 保留契约锚，MEMORY newchanlun-no-patch-keep-primitive）：M29 是
    15页时期已结算的 L0 goal 形式化顶点，是互斥定理链的终端契约锚。删除会破既有 GREEN。
    保留为**契约锚**（非权威 paradigm），诚实标注有效域——非兼容垫片。本文件不被七链新模块
    import（七链用 `OperationRole18`）。
  - **权威指引**：新模块应 import `Origin.OperationRole18`（去根化权威），不应 import 本文件。
  ──────────────────────────────────────────────────────────────────────────────────────

  ════════════════════════════════════════════════════════════════════════
  ## 存在论位置（M29，互斥分类 canonical spec §B 表 M29 行 / §A 页16–17 §21 / §E 第4要素）

  权威源（唯一）：`.chanlun/specs/2026-06-28-complete-mutex-classification-pdf-extract.md`
    （commit 83377236ec）：§B 表 **M28/M29** 行 + §C 完备性 + §E 结果包（M12+M20+M29=goal 形式化
    顶点）+ §F。

  本文件是 goal「**严格完全互斥分类 + 互斥全定义策略**」的 **形式化顶点**——把互斥分类的三结论
  在**同一组对象**上合一，并叠加 **角色互斥维度** + **祖先闭合 AncOK** + **Role(e) 唯一** + **六条
  定义区分**。M29 = C42 最终定理（三结论合一基线，W14 `theorem_final`）+ **新增「短差角色区分」
  维度**（互斥分类相对完全分类的真增量）+ 六条定义区分。

  ### M29 三结论合一（§21 页16–17，本文件顶点 `mutex_full_definition_final`）

  在分账本头寸组合语义 + 角色互斥分类 + 祖先闭合活动集中，互斥全定义策略满足三结论合一：

    **① 互斥全元素覆盖**：∀e∈E Eat^sep(e)，且腿分配尊重角色互斥分类（引 MW8
        `theorem_role_aware_recursive` / `role_aware_eatSep` / `role_aware_mutex_assignment`——
        每元素被唯一规范腿吃到 + 角色 role-indexed 腿分配 Σ指示=1）。
    **② 严格完全互斥分类**：每元素恰属一 (级别关系 ρ_e × 角色 Role(e)) 联合类（引 MW4
        `completeMutexClassificationSum_eq_one`——12 联合类指示函数求和=1，穷尽+互斥+唯一归属）。
    **③ 互斥全定义策略 ∃!O**：∀x_t ∃!O_{t+1}=π_Θ(x_t)（引 W14 `theorem_final` ③
        `final_strategy_well_defined` = W10 `policy_order_well_defined`），且活动集满足**祖先闭合
        AncOK**（引 MW7 `ancestorClose` / `ancestorClose_anc_closed`——子激活⟹全祖先在场），策略
        前提含 **Role(e) 唯一**（M20，引 MW3 `operationRole_unique`）。

  ### 六条定义区分（M29 §21 页17，本文件 §5）

    1. 做多/做空 = 绝对方向（Side(e)=ε_e，MW8 `RoleLegDuty.rootLeg` / MW3 `side`）。
    2. 短差 = 严格次级别·相对父声部反向·父仓保持的双开操作（MW8 `shortDiffLeg`）。
    3. 同级别反向 = 关闭/切换或反手·**不是短差**（MW8 `same_level_legDuty_is_sameLevelLeg`）。
    4. 次级别同向 = 顺势子腿·不是短差（MW8 `followLeg`）。
    5. 次级别反向 = 短差（MW8 `shortDiffLeg`，与 4 区分）。
    6. 父级多头的短差是做空·父级空头的短差是做多（MW3 `shortDiff_long_parent_is_short` /
       `shortDiff_short_parent_is_long`）。
    + 短差非反手防塌缩（MW8 `shortDiffLeg_ne_same_level_reverse`）——保证四角色不塌缩。

  ════════════════════════════════════════════════════════════════════════
  ## ★三结论合一的严格形式（no-patch / no-workaround，非空壳合并——M29 相对 C42 的真增量）

  M29 **不是**把 MW4/MW8/W14/MW7/MW3 五个前置塞进一个结构的空壳（那会使「三结论合一」退化为
  「五个无关命题的合取」= 090号声明膨胀）。合一的**非平凡内容** = 三结论在**同一组对象**上同时
  成立且**坐标一致**：

  - **覆盖侧同一棵树**（核心约束 `tree_eq : R.T = H.tree`）：M29 要求 MW8 角色感知元素树 R 的
    覆盖树 `R.T` 与 W14 顶点 H 的覆盖树 `H.tree` 是**同一棵 `ElementTree`**——角色互斥维度（MW8）
    叠加在 W14 三结论合一的**同一覆盖树**上，而非两棵无关的树。这是 M29「角色维度叠加在 C42 顶点
    之上」非空壳的形式根据（角色三分覆盖①、联合分类②、策略③的覆盖目标 q̄ 都指向同一棵树的元素）。
  - **策略③ 与覆盖①② 经 q*=q̄ 同一**：M29 结论③策略全定义 ∃!O（W14/W10）在覆盖可行域内输出
    q*=q̄（W14 `strategy_meets_cover_target` = W11 `coverFeasible_qStar_eq_target`），q̄ 正是结论①②
    覆盖的规范腿目标头寸——**策略「全定义」与元素「全覆盖」在覆盖可行域内同一**（W14 已建此桥，
    M29 继承并叠加角色维度）。
  - **AncOK 是策略③活动集的祖先闭合精化**（MW7）：M16 在 C31「先关后开」`targetActiveSet` 上加
    祖先闭合裁剪（MW7 `ancestorClose`），保证「子激活⟹全祖先在场」（`ancestorClose_anc_closed`）
    ——这是结论③策略全定义 q̄ 来源活动集的覆盖不漂浮约束（C31→+AncOK 精化）。
  - **Role(e) 唯一是策略前提的角色维度补全**（M20 / MW3）：C35（W10）七前提是 ν/ε/s 唯一，互斥
    分类多 **Role(e) 唯一**（MW3 `operationRole_unique`）——角色分类对每元素唯一判定，是策略
    Rec_Θ 单值在角色侧的补全。

  本文件用 `MutexFinalTheoremHypotheses` 结构承载 M29 集成前提（W14 H + MW8 R + 同树约束 +
  MW7 AncOK 参数），`mutex_full_definition_final` 同时导出三结论，每结论**真引用**对应前置顶层
  定理（MW8 `theorem_role_aware_recursive` / MW4 `completeMutexClassificationSum_eq_one` / W14
  `final_strategy_well_defined` / MW7 `ancestorClose_anc_closed` / MW3 `operationRole_unique`）。
  **无前置定理在本文件被重证**（顶点只做合一，不做证明）。

  ════════════════════════════════════════════════════════════════════════
  ## 认识论等级（formalization-validity-domain / 231号，强制标注，gatekeeper 类型层钉死）

  ★★★M29 顶点是 goal 的**形式化**顶点 = **L0 结构定理**（严格完全互斥分类 + 互斥全定义策略的
    **语法层完备性** + ∃!）。具体：①角色三分全元素覆盖（语法层每元素被唯一规范腿吃到 + 角色
    role-indexed 分配）+ ②每元素恰属一 (级别×角色) 联合类（分类完备性 Σ指示=1）+ ③策略对每状态
    唯一给出动作（∃!O 代数恒等）——全部是**构造的元素集 / 状态集上的代数/组合恒等**，信息增量
    为零的同义反复（L0）。

  ★★★**NOT L2 实盘 alpha**——M29 与全窗 L3（完整 v1 实盘择时 8 品种 8/8 否证、无 alpha）
    **不矛盾**：
    - M29 证的是「语法元素覆盖 + 分类互斥 + 策略唯一」的**代数恒等**（角色分类是 ρ_e×方向 的纯
      组合派生、覆盖是规范腿覆盖整区间的状态机恒等、∃!O 是字典序 argmin 唯一性）——**纯语法/操作
      语义层**，非市场预测。
    - 全窗 L3 否证的是「这套 v1 完整择时在 1min 尺度这些品种实盘扣成本盈利」（L2/L3 经验域）。
    - 两者有效域不同：M29 = 角色互斥分类完备性 + 全元素覆盖 + 策略全定义（L0 定义域内代数成立）；
      L3 = 净账户实盘扣成本盈利（L2 经验域，被否证）。**分类完备性 ≠ 择时盈利**。
  ★M29 顶点诚实声明（继承 W14 C40 防火墙 + MW8/MW4 gatekeeper）：
    - 全元素覆盖是**分账本声部级毛收益覆盖**（C42/C40，**否定**净资产级每笔盈利——双开净额退化
      G_net=0）；
    - 仅在**覆盖可行域 X^cover_Θ**（W11/W14 `smp` 承载 q̄∈K_Θ）+ **合法操作依附域 WellAttached**
      （ℓ_e≤ℓ_{α_e}，MW8 排除越级塌缩）内成立；
    - 角色三分分类是 L0 结构分类，**不蕴含**该分类在实盘择时有 alpha（L2/L3，PDF 未提供也不可由
      L0 推出）。
  ★下游**不得**把 M29 的 L0 语法层完备性（角色互斥/全元素覆盖/策略全定义）有效域膨胀为「实盘
    每笔盈利」——gatekeeper `MutexFinalVerdict` 类型层钉死（无 `NetProfitGuaranteed` /
    `AlphaInLive` / `CompleteEverywhere` 构造子）。

  ════════════════════════════════════════════════════════════════════════
  ## owner 边界（互斥铁律）

  本文件**只建** `formal/Origin/MutexFinalTheorem.lean`。不碰 W14/MW8/MW4/MW7/MW3 任何文件
  （前置只 import 只读，**无一被重证**）。不编辑 lakefile.toml。root 名 `Origin.MutexFinalTheorem`。
  禁 sorry/admit/axiom。简体中文。

  依赖方向（单向无环，全 GREEN 只读）：
    MutexFinalTheorem → SeparateFinalTheorem（W14：theorem_final / final_strategy_well_defined /
                        final_cover_unique / final_eatSep / final_profit / strategy_meets_cover_target /
                        FinalTheoremHypotheses）
                      → MutexRecursive（MW8：theorem_role_aware_recursive / role_aware_eatSep /
                        role_aware_mutex_assignment / RoleAwareElementTree / RoleLegDuty /
                        legDutyOfRole / shortDiffLeg_ne_same_level_reverse /
                        same_level_legDuty_is_sameLevelLeg）
                      → MutexExhaustive（MW4：completeMutexClassificationSum_eq_one /
                        roleIndicatorSum_eq_one / exists_unique_joint_class）
                      → AncestorClosure（MW7：ancestorClose / ancestorClose_anc_closed /
                        mem_ancestorClose / parent_in_raw_of_mem_ancestorClose）
                      → OperationRole（MW3：operationRole_unique / shortDiff_long_parent_is_short /
                        shortDiff_short_parent_is_long / shortDiff_ne_sameDir_reverse）。

  谱系：M29（互斥分类 PDF §21 三结论合一+角色区分+六条定义区分）→ C42（W14 三结论合一基线顶点）
        + MW8（M26 角色三分覆盖）+ MW4（M12 角色互斥穷尽 Σ指示=1）+ MW7（M16 祖先闭合）+ MW3
        （M20 Role 唯一 / M11 短差绝对方向 / M13 短差非反手）→ 本文件（goal 形式化顶点）。
        M29 = goal「严格完全互斥分类 + 互斥全定义策略」的形式化顶点（M12+M20+M29，§E 第4要素）。
        有效域上界 = 覆盖可行域 + 合法依附域内角色互斥全元素覆盖型全定义策略（C40/C42 净资产每笔
        盈利 + 全窗 L3 实盘择时 alpha 均被否定）。
-/

import Origin.SeparateFinalTheorem
import Origin.MutexRecursive
import Origin.MutexExhaustive
import Origin.AncestorClosure
import Origin.OperationRole

namespace NewChanlun.Origin.MutexFinalTheorem

open NewChanlun.Origin
open NewChanlun.Origin.SeparateLedger (SepPosition)
open NewChanlun.Origin.SeparateEat (LegAssignment EatSep SepCovered gSep)
open NewChanlun.Origin.SeparateCoverRecursive (ElementTree StateMachineProvider)
open NewChanlun.Origin.SeparateStrategyWellDefined (StrategyObjective policyOrder)
open NewChanlun.Origin.CoverFeasibleTarget (CoverObjective)
open NewChanlun.Origin.SeparateFinalTheorem
  (FinalTheoremHypotheses theorem_final final_cover_unique final_eatSep final_profit
   final_strategy_well_defined strategy_meets_cover_target)
open NewChanlun.Origin.MutexRecursive
  (RoleAwareElementTree RoleLegDuty legDutyOfRole)
open MutexElement
open NewChanlun.Origin.AncestorClosure (ancestorClose ancestorClose_anc_closed ancestors)

/-! ════════════════════════════════════════════════════════════════════════
    ## §1 M29 集成前提承载 `MutexFinalTheoremHypotheses`（同一组对象 + 角色维度 + AncOK）

    M29 是 goal 形式化顶点——它把五个前置（W14 三结论合一基线 + MW8 角色维度 + MW4 联合分类 +
    MW7 祖先闭合 + MW3 Role 唯一）集成在**同一组对象**上。承载结构的字段刻画**同一个**分账本 +
    角色 + 祖先闭合语义状态：

    - W14 顶点前提 `H : FinalTheoremHypotheses`（三结论合一基线：tree 覆盖树 + legAssign ν 单射 +
      position/price + smp 状态机 + objective J_t/K_Θ + schedule Schedule_Θ）。
    - MW8 角色感知元素树 `R : RoleAwareElementTree`（角色维度：覆盖树 T + 独立操作依附 α +
      WellAttached 合法依附 + α_root_iff）。
    - **★同一棵覆盖树约束 `tree_eq : R.T = H.tree`**（合一非空壳核心）：角色维度叠加在 W14 三结论
      合一的**同一覆盖树**上，非两棵无关的树。
    - MW7 祖先闭合参数：`par`（α_e 元素级父函数，策略侧 W9 SyntaxElement）+ `fuel`（树深上界）+
      活动集 `active/ending/starting`（C31 先关后开输入）——AncOK 作用在结论③策略 q̄ 来源活动集。

    ★这些字段共同刻画**同一个**分账本+角色+祖先闭合语义状态——三结论在此同一组对象上同时成立。
    ★L0：纯结构承载（无 Θ 数值、不依赖数据），承载 M29 集成前提的形式对象。
    ════════════════════════════════════════════════════════════════════════ -/

/--
  ★M29 集成前提承载 `MutexFinalTheoremHypotheses`（L0，§21 页16–17）—— goal 形式化顶点的同一组
  对象。每字段对应 M29 一个集成维度，共同刻画**同一个**分账本+角色+祖先闭合语义状态。

  - `V : Type`（隐式）：声部类型（W8/W13 ν 值域）。
  - `Order : Type`（隐式）：订单类型（W10 Schedule_Θ 输出）。
  - **`H : FinalTheoremHypotheses V Order`**（W14 三结论合一基线，C42 顶点前提）：覆盖树 tree +
    ν 单射 legAssign + 头寸 position + 价格 price + 状态机 smp + 策略 objective/schedule。
  - **`R : RoleAwareElementTree`**（MW8 角色维度，M26）：覆盖树 T + 独立操作依附 α + WellAttached
    合法依附（排除越级塌缩）。
  - **`tree_eq : R.T = H.tree`**（★合一非空壳核心约束）：MW8 角色感知元素树的覆盖树 = W14 顶点的
    覆盖树——角色互斥维度叠加在 W14 三结论合一的**同一覆盖树**上（非两棵无关的树）。
  - **`par : SyntaxElement → Option SyntaxElement`**（MW7 α_e 元素级父函数，策略侧）：祖先闭合
    AncOK 的依附关系（W9 SyntaxElement，结论③策略活动集侧）。
  - **`fuel : Nat`**（MW7 树深上界，§19）：祖先链有限性界。
  - **`active / ending / starting : List SyntaxElement`**（MW7 C31 先关后开活动集输入，策略侧 W9
    SyntaxElement）：A_t / D_t / B_t，AncOK 作用其上得结论③策略 q̄ 来源活动集。

  ★L0：纯结构承载，承载 M29 五维集成前提的形式对象。
-/
structure MutexFinalTheoremHypotheses (V : Type) (Order : Type) where
  /-- W14 三结论合一基线（C42 顶点 7 前提：覆盖树 + ν 单射 + 头寸/价格 + 状态机 + 策略）。 -/
  H : FinalTheoremHypotheses V Order
  /-- MW8 角色感知元素树（M26：覆盖树 T + 独立操作依附 α + WellAttached 合法依附）。 -/
  R : RoleAwareElementTree
  /-- ★合一非空壳核心：MW8 角色感知树覆盖树 = W14 顶点覆盖树（角色维度叠加在同一覆盖树上）。 -/
  tree_eq : R.T = H.tree
  /-- MW7 α_e 元素级父函数（祖先闭合 AncOK 依附关系，策略侧 W9 SyntaxElement）。 -/
  par : SeparateStrategyTarget.SyntaxElement → Option SeparateStrategyTarget.SyntaxElement
  /-- MW7 树深上界（§19 祖先链有限性界）。 -/
  fuel : Nat
  /-- MW7 已激活集 A_t（C31 先关后开输入，策略侧）。 -/
  active : List SeparateStrategyTarget.SyntaxElement
  /-- MW7 结束集 D_t（C31 先关后开输入，策略侧）。 -/
  ending : List SeparateStrategyTarget.SyntaxElement
  /-- MW7 开始集 B_t（C31 先关后开输入，策略侧）。 -/
  starting : List SeparateStrategyTarget.SyntaxElement

variable {V : Type} {Order : Type}

/-! ════════════════════════════════════════════════════════════════════════
    ## §2 M29 结论①·互斥全元素覆盖（∀e Eat^sep + 角色互斥腿分配，引 MW8 + W14）

    spec M29 ①「∀e∈E Eat^sep(e) 且腿分配尊重角色互斥」。本节在 §1 同一组对象上导出：
    - 覆盖侧：∀e 被唯一规范腿吃到（W14 `final_cover_unique` / MW8 `role_aware_eatSep`，**同一树**）。
    - 角色互斥腿分配侧：∀e 角色 role-indexed 腿分配 Σ指示=1（MW8 `role_aware_mutex_assignment`）。
    本文件**不重证**这些——只把 MW8/W14 顶层定理在 §1 字段上实例化。
    ════════════════════════════════════════════════════════════════════════ -/

/--
  ★M29 结论①·覆盖侧 `mutex_final_cover`（L0，§21 ①，引 W14）：在同一组对象上，元素树内**每个**
  元素 e 被唯一规范头寸腿吃到（覆盖 + 正收益 + ν 唯一）——直接引 W14 `final_cover_unique`
  （在 `M.H` 上实例化）。这是 M29「① ∀e∈E ∃!ν(e), Eat^sep(e)」的覆盖+唯一侧。 -/
theorem mutex_final_cover (M : MutexFinalTheoremHypotheses V Order) :
    ∀ e, M.H.tree.inTree e →
      SepCovered M.H.legAssign M.H.position e M.H.price
      ∧ (∀ e', M.H.legAssign.nu e' = M.H.legAssign.nu e → e' = e) :=
  final_cover_unique M.H

/--
  ★M29 结论①·纯覆盖侧 `mutex_final_eatSep`（L0，§21 ①，引 W14）：∀e∈E 分账本吃到 `EatSep`
  （规范腿覆盖整操作区间）——引 W14 `final_eatSep`。M29「∀e∈E, Eat^sep(e)」的纯覆盖陈述。 -/
theorem mutex_final_eatSep (M : MutexFinalTheoremHypotheses V Order) :
    ∀ e, M.H.tree.inTree e → EatSep M.H.legAssign M.H.position e :=
  final_eatSep M.H

/--
  ★★M29 结论①·角色互斥腿分配 `mutex_final_role_assignment`（L0，§21 ①，引 MW8，**同一树**）：
  在 §1 同一组对象（`M.R.T = M.H.tree`）上，角色感知元素树内**每个**元素 e 的角色 role-indexed
  腿分配满足 Σ指示=1（`(M.R.roleElem e).roleIndicatorSum = 1`，恰属一操作角色 ⟹ 腿按唯一角色
  分配，不漏类不重类）。

  ★这坐实 M29 ①「腿分配尊重角色互斥分类」：引 MW8 `role_aware_mutex_assignment`（在 `M.R` +
    经 `tree_eq` 改写的 smp 上实例化）——角色维度叠加在 W14 **同一覆盖树**上（`tree_eq` 把 W14 smp
    转为 MW8 R.T 上的 smp）。**腿分配尊重角色互斥**：每元素恰属一角色 ⟹ role-indexed 腿分配唯一。
  ★`tree_eq ▸` 把 W14 顶点 `M.H.smp : StateMachineProvider M.H.tree ...` 改写为 MW8 需要的
    `StateMachineProvider M.R.T ...`（同一棵树，类型对齐）——这是「同一组对象」的形式兑现。 -/
theorem mutex_final_role_assignment (M : MutexFinalTheoremHypotheses V Order) :
    ∀ e, M.R.T.inTree e → (M.R.roleElem e).roleIndicatorSum = 1 :=
  M.R.role_aware_mutex_assignment M.H.legAssign M.H.position M.H.price
    (M.tree_eq ▸ M.H.smp)

/--
  ★★M29 结论①·角色版全元素覆盖 `mutex_final_role_eatSep`（L0，§21 ①，引 MW8，**同一树**）：在
  同一组对象上，角色感知元素树内**每个**元素 e 被分账本规范腿吃到 `EatSep`（角色三分不破坏覆盖）
  ——引 MW8 `role_aware_eatSep`（覆盖侧导出，`tree_eq ▸ smp`）。这是 M29 ①「角色互斥维度下的全
  元素覆盖」（角色三分覆盖 ⟹ ∀e Eat^sep）。 -/
theorem mutex_final_role_eatSep (M : MutexFinalTheoremHypotheses V Order) :
    ∀ e, M.R.T.inTree e → EatSep M.H.legAssign M.H.position e :=
  M.R.role_aware_eatSep M.H.legAssign M.H.position M.H.price (M.tree_eq ▸ M.H.smp)

/--
  ★★★M29 结论①·角色感知递归四合一 `mutex_final_role_aware_recursive`（L0，§21 ①，引 MW8）：在
  同一组对象上，角色感知元素树内**每个**元素 e 同时满足（①覆盖∧②角色互斥分配∧③角色三分∧
  ④防塌缩）——直接引 MW8 `theorem_role_aware_recursive`（`tree_eq ▸ smp`）。这是 M29 结论①「全元素
  覆盖 + 腿分配尊重角色互斥」的完整 MW8 承载（覆盖侧复用 W13，角色三分互斥分配 + 防塌缩是真增量）。 -/
theorem mutex_final_role_aware_recursive (M : MutexFinalTheoremHypotheses V Order) :
    ∀ e, M.R.T.inTree e →
      EatSep M.H.legAssign M.H.position e
      ∧ (M.R.roleElem e).roleIndicatorSum = 1
      ∧ ((e.isRoot → M.R.role e = OperationRole.rootDir)
         ∧ (¬ e.isRoot → M.R.role e = OperationRole.sameDir
                          ∨ M.R.role e = OperationRole.subFollow
                          ∨ M.R.role e = OperationRole.shortDiff))
      ∧ ((M.R.roleElem e).levelRelation = LevelRelation.same
          → M.R.legDuty e ≠ RoleLegDuty.shortDiffLeg) :=
  M.R.theorem_role_aware_recursive M.H.legAssign M.H.position M.H.price (M.tree_eq ▸ M.H.smp)

/-! ════════════════════════════════════════════════════════════════════════
    ## §3 M29 结论②·严格完全互斥分类（每元素恰属一 (级别×角色) 联合类，引 MW4）

    spec M29 ②「严格完全互斥分类」。本节引 MW4 `completeMutexClassificationSum_eq_one`——每元素
    恰属一 (级别关系 ρ_e × 角色 Role(e)) 联合类（12 联合类指示函数求和=1，穷尽+互斥+唯一归属）。
    这是 goal「严格完全互斥分类」的形式化（M12 联合完备性顶点）。
    ════════════════════════════════════════════════════════════════════════ -/

/--
  ★★★M29 结论②·严格完全互斥分类 `mutex_final_classification`（L0，§21 ②，引 MW4）：角色感知
  元素树内**每个**元素 e 恰属一个 (级别关系 ρ_e, 角色 Role(e)) 联合类（`completeMutexClassificationSum
  (M.R.roleElem e) = 1`）——12 联合类（3 级别 × 4 角色）指示函数求和=1（穷尽+互斥+唯一归属）。

  ★直接引 MW4 `completeMutexClassificationSum_eq_one`（在 `M.R.roleElem e` 升格元素上实例化）——
    本文件不重证联合分类完备性。这是 M29「② 严格完全互斥分类」（每元素在级别×角色联合分类中
    恰一归属，spec §C「级别三分穷尽 × 角色四分穷尽 → 完全互斥分类」的 canonical 完备性顶点）。 -/
theorem mutex_final_classification (M : MutexFinalTheoremHypotheses V Order) :
    ∀ e, (M.R.roleElem e).completeMutexClassificationSum = 1 :=
  fun e => MutexElement.completeMutexClassificationSum_eq_one (M.R.roleElem e)

/--
  ★M29 结论②·联合类存在唯一归属 `mutex_final_unique_joint_class`（L0，§21 ②，引 MW4）：每元素 e
  **恰存在唯一**一对 (级别关系 r, 角色 R) 使其 (levelRelation, operationRole) 联合等于 (r, R)
  ——`exists_unique_joint_class` 的 ∃! 形式（Σ指示=1 ⟺ 恰一联合类归属）。这是「严格完全互斥
  分类」的存在唯一性表述（spec §C「唯一归属」三要素之一）。 -/
theorem mutex_final_unique_joint_class (M : MutexFinalTheoremHypotheses V Order) :
    ∀ e, ∃ rR : LevelRelation × OperationRole,
      ((M.R.roleElem e).levelRelation = rR.1 ∧ (M.R.roleElem e).operationRole = rR.2)
      ∧ ∀ rR' : LevelRelation × OperationRole,
          ((M.R.roleElem e).levelRelation = rR'.1 ∧ (M.R.roleElem e).operationRole = rR'.2)
            → rR' = rR :=
  fun e => MutexElement.exists_unique_joint_class (M.R.roleElem e)

/-! ════════════════════════════════════════════════════════════════════════
    ## §4 M29 结论③·互斥全定义策略 ∃!O + AncOK + Role 唯一（引 W14 + MW7 + MW3）

    spec M29 ③「∀x_t ∃!O_{t+1}=π_Θ(x_t)，活动集满足祖先闭合 AncOK，策略前提含 Role(e) 唯一」。
    本节合成三部分：
    - 策略全定义 ∃!O：引 W14 `final_strategy_well_defined`（= W10 `policy_order_well_defined`）。
    - 祖先闭合 AncOK：引 MW7 `ancestorClose_anc_closed`（子激活⟹全祖先在策略活动集，覆盖不漂浮）。
    - Role(e) 唯一：引 MW3 `operationRole_unique`（M20，角色对每元素唯一判定，Rec_Θ 角色侧补全）。
    ════════════════════════════════════════════════════════════════════════ -/

/--
  ★★★M29 结论③·互斥全定义策略 ∃!O `mutex_final_strategy_well_defined`（L0，§21 ③，引 W14）：
  对**任意**当前头寸 q_t（x_t 的头寸分量），策略输出订单存在唯一：

      ∀ qPrev, ∃! O, O = policyOrder M.H.objective M.H.schedule qPrev

  直接引 W14 `final_strategy_well_defined`（= W10 `policy_order_well_defined`，在 `M.H` 上实例化）
  ——本文件不重证 ∃!O。这是 M29「③ ∀x_t, ∃!O_{t+1}=π_Θ(x_t)」（互斥全定义策略在分账本坐标
  P^sep 上对每合法状态唯一给出动作——前言第二个证明目标）。 -/
theorem mutex_final_strategy_well_defined (M : MutexFinalTheoremHypotheses V Order) :
    ∀ qPrev : SepPosition,
      ExistsUnique (fun O : Order => O = policyOrder M.H.objective M.H.schedule qPrev) :=
  final_strategy_well_defined M.H

/--
  ★★M29 结论③·祖先闭合 AncOK `mutex_final_ancestor_closed`（L0，§21 ③ / M16 §八，引 MW7）：
  策略活动集（C31 先关后开 `(A_t\D_t)∪B_t` 后施祖先闭合 AncOK）满足——若元素 e 在祖先闭合活动集
  `A_{t+1}`，则 e 的**全部祖先**都在 W9 先关后开激活集 `(A_t\D_t)∪B_t` 中：

      ∀ e, e ∈ ancestorClose par fuel active ending starting →
        ∀ a, a ∈ ancestors par fuel e → a ∈ (A_t\D_t)∪B_t

  直接引 MW7 `ancestorClose_anc_closed`（在 `M.par/M.fuel/M.active/M.ending/M.starting` 上实例化）
  ——本文件不重证祖先闭合代数。这是 M29 ③「活动集满足祖先闭合 AncOK」（子激活⟹全祖先在场，
  覆盖不漂浮——子声部不漂浮在不存在的父声部上）。M16 相对 C31「先关后开」的精化（+AncOK 一层）。 -/
theorem mutex_final_ancestor_closed (M : MutexFinalTheoremHypotheses V Order) :
    ∀ e, e ∈ ancestorClose M.par M.fuel M.active M.ending M.starting →
      ∀ a, a ∈ ancestors M.par M.fuel e →
        a ∈ SeparateStrategyTarget.targetActiveSet M.active M.ending M.starting :=
  fun e he => ancestorClose_anc_closed M.par M.fuel M.active M.ending M.starting e he

/--
  ★★M29 结论③·Role(e) 唯一 `mutex_final_role_unique`（L0，§21 ③ / M20，引 MW3）：每元素 e 的
  操作角色 Role(e) 唯一判定——任意两个等于 `operationRole e` 的角色必相等：

      ∀ e, ∀ R₁ R₂, operationRole(e)=R₁ → operationRole(e)=R₂ → R₁=R₂

  直接引 MW3 `operationRole_unique`（在 `M.R.roleElem e` 上实例化）——本文件不重证。这是 M29 ③
  策略前提的角色维度补全（M20 / spec §3「操作语义必须唯一判定」）：C35（W10）七前提 ν/ε/s 唯一，
  互斥分类多 **Role(e) 唯一**——角色分类对每元素唯一判定，是策略 Rec_Θ 单值在角色侧的补全。 -/
theorem mutex_final_role_unique (M : MutexFinalTheoremHypotheses V Order) :
    ∀ e, ∀ R₁ R₂ : OperationRole,
      (M.R.roleElem e).operationRole = R₁ → (M.R.roleElem e).operationRole = R₂ → R₁ = R₂ :=
  fun e => MutexElement.operationRole_unique (M.R.roleElem e)

/-! ════════════════════════════════════════════════════════════════════════
    ## §5 六条定义区分（M29 §21 页17，互斥分类相对完全分类的角色区分维度）

    spec M29「六条定义区分」（§21 页17）：做多/做空=绝对方向 / 短差=严格次级别·相对父反向·父仓
    保持双开 / 同级别反向=反手非短差 / 次级别同向=顺势非短差 / 次级别反向=短差 / 父多头短差=做空·
    父空头短差=做多。本节逐条形式化（引 MW8 腿分配区分 + MW3 短差绝对方向 + 短差非反手防塌缩）。
    ════════════════════════════════════════════════════════════════════════ -/

namespace MutexFinalTheoremHypotheses

/-- ★区分1·做多/做空=绝对方向（M29 六条之1，L0，引 MW3）：每元素的绝对方向 Side(e)=ε_e ∈ {+1,-1}
    （做多/做空），非零——这是与操作角色 Role(e) 正交的绝对方向维度（M10 Side⊥Role）。 -/
theorem distinction_1_side_is_absolute (M : MutexFinalTheoremHypotheses V Order)
    (e : SyntaxElement) :
    (M.R.roleElem e).side = 1 ∨ (M.R.roleElem e).side = -1 :=
  MutexElement.side_cases (M.R.roleElem e)

/-- ★★区分2/5·短差=严格次级别反向短差腿（M29 六条之2/5，L0，引 MW8）：短差元素的腿分配义务是
    `shortDiffLeg`（次级别反向短差腿，父声部不动建独立反向子声部，父腿不删）——这统一区分2「短差=
    严格次级别·相对父反向·父仓保持双开」与区分5「次级别反向=短差」。由角色 ShortDiff ⟹ 腿分配
    `shortDiffLeg`（MW8 `legDutyOfRole` 路由）。 -/
theorem distinction_2_5_shortDiff_is_shortDiffLeg (M : MutexFinalTheoremHypotheses V Order)
    (e : SyntaxElement) (h : M.R.role e = OperationRole.shortDiff) :
    M.R.legDuty e = RoleLegDuty.shortDiffLeg := by
  unfold RoleAwareElementTree.legDuty
  rw [h]; rfl

/-- ★★★区分3·同级别反向=反手·不是短差（M29 六条之3，L0，引 MW8 防塌缩）：同级别元素（ρ_e=Same，
    **含反向反手** ε_e=−σ_{α_e}）的腿分配义务 = `sameLevelLeg`（同级别处理/反手），**且 ≠
    shortDiffLeg**（同级别反向不走短差腿）。这坐实「同级别反向是反手·不是短差」——同级别反向不
    因方向相反而变短差腿。引 MW8 `same_level_legDuty_is_sameLevelLeg` + `same_level_legDuty_ne_shortDiffLeg`。 -/
theorem distinction_3_same_reverse_is_not_shortDiff (M : MutexFinalTheoremHypotheses V Order)
    (e : SyntaxElement) (h : (M.R.roleElem e).levelRelation = LevelRelation.same) :
    M.R.legDuty e = RoleLegDuty.sameLevelLeg ∧ M.R.legDuty e ≠ RoleLegDuty.shortDiffLeg :=
  ⟨M.R.same_level_legDuty_is_sameLevelLeg h, M.R.same_level_legDuty_ne_shortDiffLeg h⟩

/-- ★区分4·次级别同向=顺势·不是短差（M29 六条之4，L0，引 MW8）：次级别顺势元素（SubFollow，
    ρ_e=Sub ∧ ε_e=σ_{α_e}）的腿分配义务 = `followLeg`（次级别顺势腿，**非** shortDiffLeg）——
    次级别同向是顺势子腿不是短差。由角色 SubFollow ⟹ 腿分配 `followLeg`（MW8 `legDutyOfRole` 路由）。 -/
theorem distinction_4_subFollow_is_followLeg (M : MutexFinalTheoremHypotheses V Order)
    (e : SyntaxElement) (h : M.R.role e = OperationRole.subFollow) :
    M.R.legDuty e = RoleLegDuty.followLeg ∧ M.R.legDuty e ≠ RoleLegDuty.shortDiffLeg := by
  constructor
  · unfold RoleAwareElementTree.legDuty; rw [h]; rfl
  · unfold RoleAwareElementTree.legDuty; rw [h]; decide

/-- ★★区分6·父级多头短差=做空 / 父级空头短差=做多（M29 六条之6，L0，引 MW3）：短差元素的绝对
    方向由父级方向**翻转**决定——父级多头（σ_{α_e}=+1）的短差是做空（Side=−1），父级空头
    （σ_{α_e}=−1）的短差是做多（Side=+1）。引 MW3 `shortDiff_long_parent_is_short` /
    `shortDiff_short_parent_is_long`（M11 精化 C03 赋格交替）。 -/
theorem distinction_6_shortDiff_abs_dir (M : MutexFinalTheoremHypotheses V Order)
    (e : SyntaxElement) (h : (M.R.roleElem e).operationRole = OperationRole.shortDiff) :
    ((M.R.roleElem e).attachedDirSign = 1 → (M.R.roleElem e).side = -1)
    ∧ ((M.R.roleElem e).attachedDirSign = -1 → (M.R.roleElem e).side = 1) :=
  ⟨fun hp => MutexElement.shortDiff_long_parent_is_short (M.R.roleElem e) h hp,
   fun hp => MutexElement.shortDiff_short_parent_is_long (M.R.roleElem e) h hp⟩

/-- ★★★六条区分防塌缩·短差腿 ≠ 同级别反手（M29 §C 防塌缩顶点，L0，引 MW8）：不存在元素同时腿
    分配为短差腿（shortDiffLeg）且级别关系为同级别（ρ_e=Same，反手的级别关系）——短差（Sub）与
    同级别反手（Same）在级别维度不可同存。这是六条区分（区分3 vs 区分5）的互斥根据：若短差等同
    同级别反手，则 SameDir/ShortDiff 塌缩，MW4 Σ指示=1 互斥性崩溃（区分3/5 失去意义）。引 MW8
    `shortDiffLeg_ne_same_level_reverse`。 -/
theorem distinction_shortDiff_ne_same_reverse (M : MutexFinalTheoremHypotheses V Order)
    (e : SyntaxElement) :
    ¬ (M.R.legDuty e = RoleLegDuty.shortDiffLeg
       ∧ (M.R.roleElem e).levelRelation = LevelRelation.same) :=
  M.R.shortDiffLeg_ne_same_level_reverse e

end MutexFinalTheoremHypotheses

/-! ════════════════════════════════════════════════════════════════════════
    ## §6 ★★★M29 三结论合一最终定理（goal 形式化顶点）

    把 §2/§3/§4 三结论（互斥全元素覆盖 / 严格完全互斥分类 / 互斥全定义策略+AncOK+Role唯一）合成
    M29 单一陈述——三结论在**同一组对象** M 上同时成立。这是 goal「严格完全互斥分类 + 互斥全定义
    策略」的 canonical 形式化顶点。
    ════════════════════════════════════════════════════════════════════════ -/

/--
  ★★★★★M29 三结论合一最终定理 `mutex_full_definition_final`★★★★★（L0，§21 页16–17，
  **goal「严格完全互斥分类 + 互斥全定义策略」形式化顶点**）：

  在分账本头寸组合语义 + 角色互斥分类 + 祖先闭合活动集中，M29 集成前提
  （`M : MutexFinalTheoremHypotheses V Order` 承载，**同一组对象 + 同一覆盖树 R.T=H.tree**）
  ⟹ **三结论合一**：

      ① 互斥全元素覆盖：∀e∈R.T, EatSep ∧ (M.R.roleElem e).roleIndicatorSum = 1
         （∀e 被唯一规范腿吃到 + 腿分配尊重角色互斥分类 Σ指示=1，引 MW8）
      ∧ ② 严格完全互斥分类：∀e, (M.R.roleElem e).completeMutexClassificationSum = 1
         （每元素恰属一 (级别×角色) 联合类，引 MW4）
      ∧ ③ 互斥全定义策略 ∃!O：∀qPrev, ∃!O = π_Θ(qPrev)
         （策略对每状态唯一给出动作，引 W14）
      ∧ ③ Role 唯一：∀e R₁ R₂, operationRole(e)=R₁ → operationRole(e)=R₂ → R₁=R₂
         （角色对每元素唯一判定，M20，引 MW3）

  - 结论① `mutex_final_role_eatSep` + `mutex_final_role_assignment`（引 MW8 `role_aware_eatSep` /
    `role_aware_mutex_assignment`，**同一树 tree_eq ▸ smp**）：∀e 被唯一规范腿吃到 + 角色互斥腿
    分配 Σ指示=1。
  - 结论② `mutex_final_classification`（引 MW4 `completeMutexClassificationSum_eq_one`）：每元素
    恰属一 (级别×角色) 联合类。
  - 结论③ `mutex_final_strategy_well_defined`（引 W14 `final_strategy_well_defined`）：∀qPrev ∃!O。
  - 结论③ Role 唯一 `mutex_final_role_unique`（引 MW3 `operationRole_unique`）：角色唯一判定。

  ★三结论建立在**同一组对象** M 上（同一 H 覆盖树/策略 + 同一 R 角色树 + `tree_eq : R.T=H.tree`）
    ——这是「合一」非空壳的形式根据：①用 R 的角色覆盖侧（经 tree_eq 与 H 同树）、②用 R 的角色分类、
    ③用 H 的策略 + R 的角色唯一，覆盖目标 q̄ 由 W14 `strategy_meets_cover_target`（q*=q̄）连接覆盖
    ①②与策略③（见 `mutex_strategy_meets_cover_target`）。**无一前置被重证**。

  ★★★goal 形式化顶点诚实声明（gatekeeper 类型层钉死）：M29 = goal 的**形式化**顶点
    （L0 分类完备性 + 策略全定义 + 全元素覆盖的**语法层**完备性 + ∃!），**NOT L2 实盘 alpha**——
    与全窗 L3（完整 v1 实盘择时 8/8 否证无 alpha）**不矛盾**：M29 证语法元素覆盖 + 分类互斥 + 策略
    唯一的**代数恒等**，非实盘盈利。**分类完备性 ≠ 择时盈利**。仅覆盖可行域 X^cover_Θ（H.smp 承载
    q̄∈K_Θ）+ 合法依附域 WellAttached（R 排除越级）内成立。 -/
theorem mutex_full_definition_final (M : MutexFinalTheoremHypotheses V Order) :
    -- ① 互斥全元素覆盖（∀e Eat^sep + 腿分配尊重角色互斥 Σ指示=1，引 MW8）
    (∀ e, M.R.T.inTree e →
        EatSep M.H.legAssign M.H.position e
        ∧ (M.R.roleElem e).roleIndicatorSum = 1)
    -- ② 严格完全互斥分类（每元素恰属一 (级别×角色) 联合类，引 MW4）
    ∧ (∀ e, (M.R.roleElem e).completeMutexClassificationSum = 1)
    -- ③ 互斥全定义策略 ∃!O（策略对每状态唯一给出动作，引 W14）
    ∧ (∀ qPrev : SepPosition,
        ExistsUnique (fun O : Order => O = policyOrder M.H.objective M.H.schedule qPrev))
    -- ③ Role 唯一（角色对每元素唯一判定，M20，引 MW3）
    ∧ (∀ e, ∀ R₁ R₂ : OperationRole,
        (M.R.roleElem e).operationRole = R₁ → (M.R.roleElem e).operationRole = R₂ → R₁ = R₂) :=
  ⟨fun e he => ⟨mutex_final_role_eatSep M e he, mutex_final_role_assignment M e he⟩,
   mutex_final_classification M,
   mutex_final_strategy_well_defined M,
   mutex_final_role_unique M⟩

/--
  ★★合一桥·策略输出 = 覆盖目标 `mutex_strategy_meets_cover_target`（L0，§21 + §八，引 W14）：
  给定覆盖目标函数 `C : CoverObjective`（覆盖可行 q̄∈K_Θ），策略最终仓位 q*（W10/W11 字典序最小
  点）**等于**覆盖目标头寸 q̄（`C.toStrategyObjective.qStar = C.target`）——直接引 W14
  `strategy_meets_cover_target`（= W11 `coverFeasible_qStar_eq_target`）。

  ★这是 M29 三结论合一**非空壳**的形式核心桥：结论①②（互斥全元素覆盖）与结论③（互斥全定义
    策略）经 q̄ = q* 连接——**策略全定义（③）的输出 q* = 元素全覆盖（①②）的目标腿 q̄**（覆盖可行
    域内）。策略「全定义」与元素「全覆盖」在覆盖可行域内**同一**（M29 在 W14 此桥上叠加角色维度，
    覆盖目标 q̄ 是结论①② 角色互斥覆盖的那些规范腿的目标头寸）。q̄∉K_Θ 时本桥不成立（有效域上界）。 -/
theorem mutex_strategy_meets_cover_target (C : CoverObjective) :
    C.toStrategyObjective.qStar = C.target :=
  strategy_meets_cover_target C

/-! ════════════════════════════════════════════════════════════════════════
    ## §7 诚实标签（formalization-validity-domain gatekeeper，goal 形式化顶点 ≠ 实盘 alpha）
    ════════════════════════════════════════════════════════════════════════ -/

/--
  ★M29 goal 形式化顶点裁定标签 `MutexFinalVerdict`（gatekeeper，诚实分层）。
  唯一构造子 `mutexFullDefinitionInCoverAndAttachedDomain`——类型层钉死 M29 结论数学名称 =
  **「覆盖可行域 + 合法依附域内的角色互斥全元素覆盖型全定义策略（语法层完备）」**（goal 形式化
  顶点：严格完全互斥分类 + 互斥全定义策略 + 全元素覆盖的 L0 语法层完备性 + ∃!）。

  ★**没有** `NetProfitGuaranteed` / `AlphaInLive` / `CompleteEverywhere` 构造子——拒绝三类声明膨胀：
    (1)「角色互斥全元素覆盖 ⟹ 净账户每笔盈利」（覆盖是 P^sep 分账本声部级毛收益，非净资产盈利——
        C40/C42 否定净账户每笔盈利，双开净额退化 G_net=0）；
    (2)「分类完备 + 策略全定义 ⟹ 实盘 alpha」（M29 是 L0 语法层；实盘 alpha 是 L2 经验域，全窗 L3
        v1 8/8 否证——M29 与 L3 不矛盾，两个有效域。**分类完备性 ≠ 择时盈利**）；
    (3)「三结论合一在任意状态/任意依附成立」（仅覆盖可行域 X^cover_Θ + 合法依附域 WellAttached
        内成立；q̄∉K_Θ 或越级依附 ℓ_e>ℓ_{α_e} 时不适用——W14/MW8/MW7 有效域上界继承）。
-/
inductive MutexFinalVerdict where
  | mutexFullDefinitionInCoverAndAttachedDomain
deriving DecidableEq, Repr

/-- ★裁定见证（L0，gatekeeper）：M29 goal 形式化顶点裁定**必是**「覆盖可行域 + 合法依附域内的
    角色互斥全元素覆盖型全定义策略（语法层完备）」（**不是**实盘 alpha / 净资产每笔盈利）。
    支撑：§6 `mutex_full_definition_final`（三结论合一）+ MW8 角色互斥覆盖 + MW4 联合分类完备 +
    W14 策略全定义 + MW7 祖先闭合 + MW3 Role 唯一 + 合一桥（q*=q̄）。

    ★这正是 spec §E 第4要素 + §F：M29 是 goal 的**形式化**顶点（L0 语法层完备性），与全窗 L3
      实盘否证不矛盾——类型层钉死，无「实盘 alpha」/「净盈利保证」构造子。 -/
theorem mutex_final_verdict_is_syntactic_completeness (v : MutexFinalVerdict) :
    v = MutexFinalVerdict.mutexFullDefinitionInCoverAndAttachedDomain := by
  cases v; rfl

/-! ════════════════════════════════════════════════════════════════════════
    ## 交付总结（M29 互斥全定义策略最终定理，goal 形式化顶点）

    本文件**证**（L0，machine-checked，无 sorry/admit/axiom，import W14/MW8/MW4/MW7/MW3 链）：

    1. §1 M29 集成前提 `MutexFinalTheoremHypotheses`（同一组对象 + 角色维度 + AncOK）：W14 顶点 H
       （三结论合一基线）+ MW8 角色树 R + **★同一覆盖树约束 tree_eq : R.T=H.tree**（合一非空壳
       核心）+ MW7 祖先闭合参数（par/fuel/活动集）。

    2. §2 M29 结论①·互斥全元素覆盖（引 MW8 + W14，**同一树 tree_eq ▸ smp**）：
       - `mutex_final_cover`（W14 `final_cover_unique`）：∀e 被唯一规范腿吃到。
       - `mutex_final_eatSep` / `mutex_final_role_eatSep`：∀e EatSep（覆盖侧 + 角色版）。
       - ★`mutex_final_role_assignment`（MW8 `role_aware_mutex_assignment`）：∀e 角色互斥腿分配 Σ=1。
       - ★`mutex_final_role_aware_recursive`（MW8 `theorem_role_aware_recursive`）：覆盖∧角色互斥∧
         三分∧防塌缩四合一。

    3. §3 M29 结论②·严格完全互斥分类（引 MW4）：
       - ★`mutex_final_classification`（MW4 `completeMutexClassificationSum_eq_one`）：每元素恰属一
         (级别×角色) 联合类（12 联合类 Σ指示=1）。
       - `mutex_final_unique_joint_class`（MW4 `exists_unique_joint_class`）：联合类 ∃! 归属。

    4. §4 M29 结论③·互斥全定义策略 ∃!O + AncOK + Role 唯一（引 W14 + MW7 + MW3）：
       - ★`mutex_final_strategy_well_defined`（W14 `final_strategy_well_defined`）：∀x_t ∃!O。
       - ★`mutex_final_ancestor_closed`（MW7 `ancestorClose_anc_closed`）：子激活⟹全祖先在策略
         活动集（覆盖不漂浮，C31→+AncOK 精化）。
       - ★`mutex_final_role_unique`（MW3 `operationRole_unique`）：Role(e) 唯一判定（M20 策略前提
         角色侧补全）。

    5. §5 六条定义区分（引 MW8 腿分配 + MW3 短差绝对方向 + 防塌缩）：
       - `distinction_1_side_is_absolute`（区分1：做多/做空=绝对方向）。
       - `distinction_2_5_shortDiff_is_shortDiffLeg`（区分2/5：短差=次级别反向短差腿）。
       - ★`distinction_3_same_reverse_is_not_shortDiff`（区分3：同级别反向=反手·≠短差）。
       - `distinction_4_subFollow_is_followLeg`（区分4：次级别同向=顺势·≠短差）。
       - ★`distinction_6_shortDiff_abs_dir`（区分6：父多头短差=做空·父空头短差=做多）。
       - ★★★`distinction_shortDiff_ne_same_reverse`（防塌缩：短差腿≠同级别反手，MW8）。

    6. ★★★★★§6 M29 三结论合一最终定理（**goal 形式化顶点**）：
       - ★★★★★`mutex_full_definition_final`：M29 集成前提 ⟹ ① 互斥全元素覆盖+角色互斥腿分配 ∧
         ② 严格完全互斥分类 ∧ ③ 互斥全定义策略 ∃!O ∧ Role 唯一——三结论在**同一组对象** M 上
         同时成立。
       - ★★`mutex_strategy_meets_cover_target`（W14 `strategy_meets_cover_target`）：策略输出 q* =
         覆盖目标 q̄（合一桥，覆盖①②与策略③在覆盖可行域内同一）。

    7. §7 诚实标签 `mutex_final_verdict_is_syntactic_completeness`（裁定=覆盖可行域+合法依附域内
       角色互斥全元素覆盖型全定义策略·语法层完备，**无** NetProfitGuaranteed / AlphaInLive /
       CompleteEverywhere 构造子）。

    本文件**不证**（formalization-validity-domain 诚实边界）：
    - ✗ 三结论合一 ⟹ 实盘 alpha / 净账户每笔盈利（M29 是 L0 语法层完备性；C40/C42 否定净账户每笔
      盈利；全窗 L3 v1 8/8 否证实盘择时无 alpha——M29 与 L3 **不矛盾**，两个有效域。分类完备性 ≠
      择时盈利）。
    - ✗ 三结论合一在覆盖可行域**外**（q̄∉K_Θ）或合法依附域**外**（越级 ℓ_e>ℓ_{α_e}）成立（W14/MW8/
      MW7 有效域上界继承）。
    - ✗ 任何前置工位的内部证明（MW8 角色三分归纳 / MW4 Σ指示=1 / W14 三结论合一 / MW7 祖先闭合
      代数 / MW3 角色四分派生）——顶点只做合一，所有前置定理作为已证黑盒**真引用**，无一被本文件
      重证。

    ★goal 形式化顶点（§E 第4要素 + §F）：M29 = M12（角色互斥穷尽 Σ指示=1，本文件结论②引 MW4）+
      M20（策略全定义 ∃!O + Role 唯一，本文件结论③引 W14/MW3）+ 三结论合一（本文件
      `mutex_full_definition_final`）——前言两个证明目标（分类必须互斥穷尽 + 策略必须对每合法
      状态唯一给出动作）的形式化合一顶点。

    谱系：M29（互斥分类 PDF §21 三结论合一+角色区分+六条定义区分）→ C42（W14 三结论合一基线）+
          MW8（M26 角色三分覆盖）+ MW4（M12 角色互斥穷尽）+ MW7（M16 祖先闭合）+ MW3（M20 Role
          唯一 / M11 短差绝对方向 / M13 短差非反手）→ 本文件（goal 形式化顶点）。有效域上界 =
          覆盖可行域 + 合法依附域内角色互斥全元素覆盖型全定义策略（C40/C42 净资产每笔盈利 + 全窗
          L3 实盘择时 alpha 均被否定）。
    ════════════════════════════════════════════════════════════════════════ -/

end NewChanlun.Origin.MutexFinalTheorem
