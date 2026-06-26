/-
  Foundation/EngineVerdict.lean

  引擎诚实分层声明框架（task #95，架构吸收：codex Origin/EngineBridge.lean）

  本文件把「哪个引擎在完全分类体系中处于什么地位」形式化为一个枚举 `EngineVerdict`，
  并为我们项目当前实存的各引擎给出**与实际一致**的分层声明。

  这是 codex「立场 B 裁定」中可独立迁移、与 gaps 无依赖的两块之一——
  只迁移诚实声明框架（枚举 + 各引擎 verdict def），**不迁移桥定理**
  （`strict_assembly_*` 系列依赖 final `Strict.HybridAssembly`，与 #93 账本扩维耦合，
   延到 #88 全定义策略重基后再做）。

  ## 存在论位置：诚实声明框架，不是能力声明

  EngineVerdict 不声明任何引擎「做到了什么」，它声明的是「该引擎在体系中被授予什么地位」。
  地位与能力的区分（spec-execution-gap）：
  - `canonicalOrigin`（权威源）：体系的真理来源，分类/策略以它为准。**当前无任何引擎持有此地位。**
  - `referenceOnly`（仅参考）：只能作为对照，不能作为真理来源。
  - `tOperatorOnly`（仅 T 算子）：只实现了 T 算子层（写回/状态转移），未实现完整闭环。
  - `operationalBacktestOnly`（仅操作回测）：只在操作/回测语境下被使用，未取得 canonical 地位。
  - `legacyRejected`（遗留拒绝）：旧版本，已被否定，不参与体系。

  ## 认识论等级：L0（纯定义/枚举）

  本文件全部内容是定义内蕴的（枚举构造 + 命题判定），不依赖任何真实/合成数据。
  分层声明的「与实际一致」是一个关于**项目当前状态**的断言（哪个引擎现在是什么地位），
  不是关于引擎**经验有效性**的断言——后者需要 L2+ 真实数据验证（见 backtest-protocol-v0）。
  Lean build 通过 = 枚举与判定逻辑正确（L0），不是引擎有效域的声明（formalization-validity-domain）。

  ## 防声明膨胀的核心：当前无 canonical 引擎

  codex 原文以 `origin_is_only_canonical_verdict : canonicalOrigin = canonicalOrigin := rfl`
  收尾——这是空洞同义反复（`a = a`），不携带信息。本文件用有实质内容的定理替换它：
  `no_current_engine_is_canonical`——证明我们当前声明的**每一个**引擎 verdict 都 ≠ canonicalOrigin。
  这才是与实际一致的诚实声明：canonical 地位是体系的目标，当前尚未被任何引擎兑现。

  standalone：不 import HybridAssembly（避免与 #93 耦合）。本文件零 import。
-/

namespace NewChanlun.Foundation

/-! ## 引擎地位枚举 -/

/-- 一个引擎在完全分类体系中被授予的地位。
    五个构造子穷尽（`DecidableEq` ⟹ 判定相等，构造归纳 ⟹ 无第六地位）。 -/
inductive EngineVerdict where
  /-- 权威源：体系的真理来源。分类与策略以它为准。 -/
  | canonicalOrigin
  /-- 仅参考：只能作为对照，不作为真理来源。 -/
  | referenceOnly
  /-- 仅 T 算子：只实现 T 算子层（状态写回/转移），未实现完整闭环。 -/
  | tOperatorOnly
  /-- 仅操作回测：只在操作/回测语境被使用，未取得 canonical 地位。 -/
  | operationalBacktestOnly
  /-- 遗留拒绝：旧版本，已被否定，不参与体系。 -/
  | legacyRejected
deriving DecidableEq, Repr

/-- 一个引擎地位是否为「权威源」。判定谓词（`DecidableEq` 自动可判定）。 -/
def EngineVerdict.isCanonical : EngineVerdict → Bool
  | EngineVerdict.canonicalOrigin => true
  | _ => false

/-! ## 项目当前各引擎的分层声明（与实际一致）

  以下 def 把项目当前实存的各引擎绑定到其**当前**地位。
  「当前」是关键限定词：theta_v0 在实装封印（bit-exact 对齐 Parse/classifier/strategy
  + 回测协议验证）后可升级 canonical，但**现在**它是 referenceOnly——不是 canonical。
  在此处把它写成 canonicalOrigin 就是声明膨胀（no-patch-mentality §5）。
-/

/-- `theta_v0` 引擎（Rust 实装：parser/classifier/strategy）的当前地位。

    **referenceOnly，不是 canonical。** theta_v0 已 bit-exact 对齐 Parse.lean/classifier/strategy
    （task #78/#79/#80），但完全分类的 canonical 地位要求完整闭环 S_Θ（#88 重基 + #94 闭环实装），
    且需通过预注册回测协议（backtest-protocol-v0）的 L2+ 验证。两者均未完成，故当前仅参考。 -/
def thetaV0Verdict : EngineVerdict := EngineVerdict.referenceOnly

/-- `recursive_t` 的 T 算子层（Rust：状态写回/转移）的当前地位。

    **tOperatorOnly。** 实现了 T 算子（写回完整 x_t），但未实现完整的
    分类→应对→写回单一闭环（闭环装配在 Strict.HybridAssembly，#93 进行中）。 -/
def recursiveTOperatorVerdict : EngineVerdict := EngineVerdict.tOperatorOnly

/-- `t_engine`（操作/回测引擎）的当前地位。

    **operationalBacktestOnly。** 在操作与回测语境被使用，但未取得 canonical 地位
    （回测产生信号 ≠ 经验有效，见 l2-engine-incompleteness-vs-theta-falsification）。 -/
def tEngineVerdict : EngineVerdict := EngineVerdict.operationalBacktestOnly

/-- 旧版 `legacy v0` 引擎的地位。

    **legacyRejected。** 旧实现，已被否定，不参与当前体系。 -/
def legacyVerdict : EngineVerdict := EngineVerdict.legacyRejected

/-! ## 诚实声明定理（替换 codex 的空洞同义反复）

  codex 原文的 `origin_is_only_canonical_verdict : canonicalOrigin = canonicalOrigin := rfl`
  是 `a = a` 的同义反复，零信息。这里替换为携带信息的实质断言。
-/

/-- 当前声明的引擎清单。canonical 地位是体系目标，当前清单中无引擎持有它。 -/
def declaredEngineVerdicts : List EngineVerdict :=
  [thetaV0Verdict, recursiveTOperatorVerdict, tEngineVerdict, legacyVerdict]

/-- **诚实声明（核心反声明膨胀定理）**：当前声明的每一个引擎地位都不是 `canonicalOrigin`。

    这不是同义反复——它对 `declaredEngineVerdicts` 的每个元素逐一判定，
    若未来把任一引擎错误地声明为 canonical（声明膨胀），此定理立即编译失败。
    定理成立 = 项目当前诚实地承认：尚无引擎取得权威源地位。 -/
theorem no_current_engine_is_canonical :
    ∀ v ∈ declaredEngineVerdicts, v ≠ EngineVerdict.canonicalOrigin := by
  intro v hv
  simp only [declaredEngineVerdicts, thetaV0Verdict, recursiveTOperatorVerdict,
    tEngineVerdict, legacyVerdict, List.mem_cons, List.not_mem_nil, or_false] at hv
  rcases hv with h | h | h | h <;> subst h <;> decide

/-- 推论：当前声明清单里没有任何引擎的 `isCanonical` 为 `true`。 -/
theorem no_current_engine_isCanonical_true :
    ∀ v ∈ declaredEngineVerdicts, v.isCanonical = false := by
  intro v hv
  simp only [declaredEngineVerdicts, thetaV0Verdict, recursiveTOperatorVerdict,
    tEngineVerdict, legacyVerdict, List.mem_cons, List.not_mem_nil, or_false] at hv
  rcases hv with h | h | h | h <;> subst h <;> rfl

/-- 升级合法性条件（语法记录）：一个引擎从非 canonical 升级到 `canonicalOrigin`
    需要外部条件 `sealed`（实装封印 + 回测协议 L2+ 验证）成立。
    本谓词只记录「升级是有条件的」这一语法，**不**断言条件已满足
    （满足与否是经验问题，不在 L0 范围内）。 -/
def canPromoteToCanonical (current : EngineVerdict) (sealed : Prop) : Prop :=
  current ≠ EngineVerdict.canonicalOrigin → sealed → True

/-- 任何当前地位在 `sealed` 成立时都满足升级合法性谓词——
    谓词本身恒真（它只记录条件结构，不携带兑现承诺）。
    这显式地把「封印后可升 canonical」编码为**条件**而非**既成事实**。 -/
theorem promote_is_conditional (current : EngineVerdict) (sealed : Prop) :
    canPromoteToCanonical current sealed := by
  intro _ _
  trivial

end NewChanlun.Foundation
