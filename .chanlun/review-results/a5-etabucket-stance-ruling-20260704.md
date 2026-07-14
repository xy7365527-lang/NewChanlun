# ★★裁定：A5 γ_t/ηBucket 立场A vs 立场B 对撞——立场B 成立，ηBucket 须补装进 z

工位：ws-gap2rulings | Task #174 | 2026-07-04 | 方法：直接回溯 `docs/formal-chain/完整的策略.pdf`
§6/§10 原文（Read pages 4-9）+ 独立读取 `rust/src/theta_v0/strategy/ledger.rs` 现码逐式核对

**这是本轮唯一需要单独显著标注的裁定——它直接决定 ηBucket 是否补装进 z（供 Lead/genealogist/
#149 后续工位直接消费）。**

---

## 一句话结论

**立场B 成立，立场A 不成立。** γ_t 的定义域 = "现有 η_t 与 η_* 比较判据的离散化"，不需要
"独立的账本负成本缓冲实体"。η_t 已经有生产者——`TwState::tw()`（`free+holding+withdrawn`），
且 `enter_ready()` 内部**直接调用它**（`s.tw() >= self.eta_star(s)`）与 PDF 公式逐式对齐。
立场A 的论证前提本身与它引用的同一段代码矛盾。**ηBucket 应作为纯派生分类函数（零新数据源）
补装进 z 第15维。**

---

## PDF 原文核对（直接引用，非转引）

`docs/formal-chain/完整的策略.pdf`（11页，Read pages 4-9 逐页核对）：

**§6（page 4）**，z 向量第16维定义：
> z = (ℓ, e, δ, I_γ, Ndepth, CandType, ForceState, Jchain, σ_higher, σ_p, role, posState,
> shortDiff, H, TStage, **ηBucket**, RiskMode, CostBucket, MarginState, ExitType)
>
> ηBucket：负成本缓冲状态；RiskMode：正常/去杠杆/强平（两者是并列独立维，非同一轴）。

**§10（page 6-7）**，三阶段资金层——这是唯一给出 γ_t 精确定义的地方，且 η_t 在**同一段落**
内被多次复用（budget 约束 / Ready_t / EnterReady_t），从未被定义为独立于这些用法之外的
另一个对象：

```
完整策略含双账本:
  R_t = Π_t − A_t − W_t,
  TW_t = free_t + holding_t + withdrawn_t.
State_t = (R_t, TW_t)

γ_t = { Deficit,        η_t < 0
        Zero,           η_t = 0
        PositiveUnsafe, 0 < η_t < η_*
        PositiveSafe,   η_t ≥ η_* }

η_{n+1} = η_n + g_n − a_n,
a_n + L^wc_{n+1} ≤ η_n + g_n − η_*.

若要求每股负成本 ≤ −κ, 则:
  η_t ≥ κQ_t,
  a_n + L^wc_{n+1} + κΔQ_n ≤ η_n + g_n − κQ_n.

最严格触发:
  Ready_t = S_t=I ∧ η_t≥I_0−W_t+η_*(x_t)+L^wc_{t+1} ∧ RiskNormal.
  EnterReady_t = S_t=II ∧ W_t≥I_0 ∧ openLegacyLegs_t=0 ∧ RiskNormal_t ∧ η_t≥L^wc_{t+1}+κQ_t.
  其中: η_*(x_t) = L^wc_{t+1} + κQ_t.
```

**关键读法**：PDF 全篇只出现**一个** η_t 对象——它既是 `budget` 递推的状态变量，也是
`Ready_t`/`EnterReady_t` 门槛判据里被比较的那个量，还是 γ_t 四态分类的分类对象。PDF **没有**
在任何地方暗示"γ_t 所分类的 η_t"与"Ready_t/EnterReady_t 里的 η_t"是两个不同对象、需要
分别落地两套数据源。整段文字的唯一自然读法是：γ_t 就是把 Ready_t/EnterReady_t 已经在用
的**同一个** η_t，按 0 和 η_* 两个分割点离散化成四态。

---

## 代码核对（独立读取，非转引 #149/#158 报告）

`rust/src/theta_v0/strategy/ledger.rs`：

```rust
/// 总财富 `TW = free + holding + withdrawn`
pub fn tw(&self) -> i64 {
    self.free + self.holding + self.withdrawn
}
```

```rust
/// 状态依赖 barrier `η⋆(s) = L^wc(s) + κ·Q(s)`（契约锚 PDF §10 `η⋆(x_t)=L^wc_{t+1}+κ·Q_t`）。
pub fn eta_star(&self, s: &TwState) -> i64 { ... }

/// **EnterEarning 合法性谓词 `EnterReady`...**：`S=II ∧ W≥I0 ∧ openLegacyLegs=0 ∧ RiskNormal ∧ η≥η⋆`。
/// - `η≥η⋆`：在险权益 `tw()` 过 barrier `eta_star`（覆盖最坏损失 + κ 缓冲）。
pub fn enter_ready(&self, s: &TwState, i0: i64, risk_normal: bool) -> bool {
    s.stage == TStage::CapitalRecovered
        && s.withdrawn >= i0
        && s.open_legacy_legs == 0
        && risk_normal
        && s.tw() >= self.eta_star(s)   // ← 这就是 PDF 的 η_t ≥ η_*(x_t)
}
```

`enter_ready()` 的最后一个合取项 `s.tw() >= self.eta_star(s)` **逐式对齐** PDF 的
`η_t≥L^wc_{t+1}+κQ_t`，其中 `s.tw()` 就是 `η_t`（doc comment 原话："`η≥η⋆`：**在险权益**
`tw()` 过 barrier `eta_star`"）。`RiskPolicy::buy_core_legal()` 的参数 `eta_n`（当前在险
权益）同样是这个量，与 PDF `η_n` 记号一一对应。

**结论：η_t 早已有生产者，就是 `TwState::tw()`；η_* 早已有生产者，就是
`RiskPolicy::eta_star()`。两者精确对应 PDF 公式（非近似/非改写）。γ_t 只需一个纯函数
`fn eta_bucket(eta: i64, eta_star: i64) -> EtaBucket { match (eta, eta_star) {...} }`
读这两个已存在的量，零新数据源、零新账本实体。**

---

## 对立场A 论证的逐点反驳

立场A（#149 zdims-impl-20260704.md 主体 §1/§2）：

> "closed_loop/ledger 无「账本 η 负成本缓冲实体」生产者；strategy/ledger.rs 的
> η⋆=L^wc+κ·Q 只是 enter_ready 的相变准入门槛谓词分量，非账本缓冲态桶；econ eta_in/eta_out
> 是 adverse spread 分解量。"

逐点核对：

1. **"无生产者"——事实错误。** 立场A 自己引用的 `η⋆=L^wc+κ·Q`（`eta_star()`）在
   `enter_ready()` 内部**直接与 `s.tw()` 比较**（`s.tw() >= self.eta_star(s)`）——`tw()`
   正是 η_t 的生产者，且这行代码正是立场A 自己引用作为"仅门槛判据"的那个函数体的一部分。
   立场A 混淆了"η_*（门槛阈值）本身不是分桶"（这句话本身正确）与"η_t（被分类的量）无生产者"
   （这句话被同一段代码直接证伪）——γ_t 分类的对象是 `η_t=tw()`，不是 `η_*`。
2. **"econ eta_in/eta_out 是 adverse spread 分解量"——正确但无关。** 独立 grep 确认
   `econ_positive.rs` 的 `eta_in`/`eta_out` 是入场/出场滑点损耗分解量（`ExitDecision`/
   `SpreadAttribution` 路径），与 GAP3/§10 的账本 η_t 是完全不同的两个对象，同名不同义
   （类似 `RiskPolicy.kappa` vs `RiskConfig.kappa` 的既有坑）。这个事实**不支持**"η_t 无
   生产者"——它只是提醒"仓库里有两个不同的 η"，而 GAP3 侧的 η_t 已经在 `TwState::tw()`
   落地，与 `eta_in`/`eta_out` 无关也不冲突。
3. **"装维=声明膨胀（090号）"——前提不成立则结论不适用。** 090号声明膨胀是"没有对应的
   实证/实装基础却声称已装/应装"。这里的实际情况是**反过来**：底层数据（η_t/η_*）已经
   100% 落地且逐式对齐 PDF 公式，只缺一个读取已有数据的纯函数分类器——不装才是"该做的
   工作被搁置"，不存在"无中生有"的声明膨胀风险。

**重要发现（文档内部不一致，非我方新论点）**：`zdims-impl-20260704.md` 自身的 §7 影响声明
"收口补丁"段落（该文件末尾，标注"2026-07-03 收口 agent"）已经**自我推翻**了 §1/§2 的"不装"
结论，写道："①ηBucket——#158 专项终裁...与本包核验互相独立佐证：γ_t 即 ηBucket 同一对象、
**确认性缺口非存疑**（底层连续量 η_t/L^wc/Q/κ 已由 ledger 真承载且精确对应 PDF §10 公式；
离散四态...分类变量全仓库零承载）"——这段文字与立场A（引自 §1/§2 主体）**直接矛盾**，且
已经与立场B（#158）一致。也就是说，**"两个工位对撞"某种程度上是 #149 报告自身前后不一致**
（主体文本未随收口补丁同步更新）造成的表象，不完全是两个独立技术判断的真实分歧。

---

## 裁定

1. **立场B 成立**：γ_t 定义域 = 现有 η_t（`TwState::tw()`）与 η_*（`RiskPolicy::eta_star()`）
   比较判据的离散化。#149 的"无生产者"判定基于对自己引用代码的误读（把 η_* 门槛阈值当成
   "η_t 无生产者"的证据，而该门槛判据的另一操作数正是 η_t 的生产者）。
2. **ηBucket 应补装进 z**（第15维，紧邻已实装的 TStage 第14维），实装面：
   - 新枚举 `EtaBucket { Deficit, Zero, PositiveUnsafe, PositiveSafe }`，置于 `ledger.rs`
     `RiskPolicy` 附近；分类函数读 `TwState::tw()` 与 `RiskPolicy::eta_star()`（零新数据源，
     两者均已存在）。
   - `MuClass` 补第15维 `pub eta_bucket: Option<EtaBucket>`；赋值点与 `t_stage`
     （`mu_estimator.rs:152`，同一 π fill loop 账本态可得处）同源——`from_certificate`
     裸证书路径恒 `None`（同 `t_stage`/`risk_mode`/`horizontal` 先例，231号诚实 None）。
   - 装配点：`runner.rs` 构造 `ext_i: ZExt` 处，与 `risk_mode`/`t_stage` 同一批次填入
     （同一账本态读取点，非另开一次账本查询——避免任何时序错位风险）。
3. **不属于本裁定范围**：是否现在就实装（本裁定是判定，非实装授权，同上一轮 8 条裁定的
   性质声明）；具体归入 #149 后续工位还是新开任务，由 Lead 按依赖序调度。

**翻转条件**：若未来发现 PDF 中存在第二个、与 `TwState::tw()` 语义不同的"账本 η"对象（本次
核对未见，`eta_in`/`eta_out` 已排除为同名不同义的另一路径），或发现 `enter_ready()`/
`eta_star()` 与 PDF §10 公式的字段级对应实为巧合而非同一对象（本次逐式核对未见此类证据），
则本裁定翻转。

---

## 结果包六要素

1. **结论**：立场B（须实装）成立，立场A（维持不装）不成立，理由见上。
2. **定义依据**：`docs/formal-chain/完整的策略.pdf` §6（page 4，z 第16维定义）+ §10
   （page 6-7，γ_t 四态定义 + η_t/η_*/Ready_t/EnterReady_t 全部公式，Read pages 4-9 逐页
   核对，非转引）；代码：`rust/src/theta_v0/strategy/ledger.rs` `TwState::tw()`
   `RiskPolicy::eta_star()`/`enter_ready()`/`buy_core_legal()` 全部读取（非转引 #158 表格）。
3. **边界条件**：见上"翻转条件"。
4. **下游推论**：#149 报告 §1/§2 的"维持诚实缺口不装"结论应视为 stale（已被该文件自身
   §7 收口补丁事实上推翻，本裁定是第三方独立确认）；ηBucket 实装工作建议归口 #149 同款
   任务链（zdims 系列）或新开任务，由 Lead 决定；genealogist 若已有/将立 A5 相关谱系条目，
   应引用本裁定作为立场B 成立的定案依据。
5. **谱系引用**：680号（双侧判据，η_t 的"分类层是否有生产者"核对方法论同构）；231号
   （诚实 None 先例，`eta_bucket: Option<EtaBucket>` 的 None 语义遵循）；090号（声明膨胀——
   本裁定指出立场A 对该原则的引用不成立，前提缺失）；`.chanlun/review-results/
   a5-gamma4-confirm-20260703.md`（#158，独立第一次确认）、`codex-gap2rulings-20260703.md`
   条目7（Codex 独立第二次确认）——本裁定是第三次独立确认，三方法论路径收敛同一结论。
6. **影响声明**：新增本文件，零代码改动。本裁定是对 A5/ηBucket 是否必装这一具体问题的
   最终定案（三方法论独立路径收敛）；不代表授权立即实装，实装时机/归属任务由 Lead 决定。
