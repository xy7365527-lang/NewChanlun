# A5 专项确认：γ_t 四桶（Deficit/Zero/PositiveUnsafe/PositiveSafe）closed_loop 承载形态

工位：ws-a5gamma | 任务 #158 | 只读确认，不改代码

## 原文出处（一级权威链内定位）

源 = `docs/formal-chain/完整的策略.pdf`（编排者 2026-07-02 22:50 下载，钦定"严格的实装版"；内部 PDF Title 元数据为"推导完全分类"，文件名与内容标题不一致属该 PDF 自身元数据问题，不影响内容权威性——已用 `pdftotext`/`Read pages` 逐页核对）。

**§6（第4页，z 完整互斥状态）**明确把 `ηBucket` 列为 canonical z 的一维：

> z = (ℓ, e, δ, I_γ, Ndepth, CandType, ForceState, Jchain, σ_higher, σ_p, role, posState, shortDiff, H, TStage, **ηBucket**, RiskMode, CostBucket, MarginState, ExitType)
> ……
> **ηBucket：负成本缓冲状态**；RiskMode：正常/去杠杆/强平（两者是并列独立维，非同一轴）。

**§10（第7页，三阶段资金层）**给出 γ_t 的精确四桶定义：

```
γ_t = { Deficit,         η_t < 0
        Zero,            η_t = 0
        PositiveUnsafe,  0 < η_t < η_*
        PositiveSafe,    η_t ≥ η_* }
```

并给出 η 的递推、预算约束、`η_*(x_t) = L^wc_{t+1} + κQ_t`、`Ready_t`/`EnterReady_t` 严格谓词（全文见 PDF 第7页，已用 Read pages 6-8 逐页核对，非转引）。

**结论：γ_t 与 ηBucket 是同一对象**——§6 z 向量的第 16 维命名"ηBucket"，§10 给出其四值定义。gap-master2 终稿 A5 行、`mu_estimator.rs:81` 注释、Task #149 标题中的"ηBucket"均指此对象。

## 逐桶代码锚点核对

对照 `rust/src/theta_v0/strategy/ledger.rs`（`TwState`/`RiskPolicy`，PDF §10 契约锚点）：

| 底层量 | PDF §10 记号 | 代码锚点 | 一致性 |
|---|---|---|---|
| η_t（在险权益） | η_t | `TwState::tw()` ledger.rs:179（`free+holding+withdrawn`） | 一致（TW=η_t 的账本承载） |
| L^wc（最坏损失/在险本金） | L^wc_{t+1} | `TwState::l_wc()` ledger.rs:193（`notional_in-withdrawn`） | 一致 |
| Q（名义敞口） | Q_t | `TwState::notional()` ledger.rs:198 | 一致 |
| κ（缓冲系数） | κ | `RiskPolicy.kappa` ledger.rs:229（构造时强制≥0） | 一致 |
| η_*（barrier） | η_*(x_t)=L^wc+κQ | `RiskPolicy::eta_star()` ledger.rs:269 | **公式逐项一致** |
| EnterReady 谓词 | S=II∧W≥I0∧openLegacyLegs=0∧RiskNormal∧η≥η_* | `RiskPolicy::enter_ready()` ledger.rs:286-292 | **五合取逐项一致** |
| BuyCore 合法性 | a_n+L^wc_{n+1}+κΔQ_n ≤ η_n+g_n-κQ_n | `RiskPolicy::buy_core_legal()` ledger.rs:307-320 | **公式逐项一致** |

底层连续量与两条布尔闸门**精确对应** PDF §10 公式，这部分是真承载（非臆造，非近似）。

但 γ_t/ηBucket 作为**离散四态分类变量本身**：

| γ_t 桶 | 判据 | 代码中的对应物 | 判定 |
|---|---|---|---|
| Deficit | η_t < 0 | 无——`enter_ready`/`buy_core_legal` 只在整数域比较 η vs η_*（或移项后的复合不等式），从未单独判 η<0 | **未承载** |
| Zero | η_t = 0 | 同上，从未单独判 η=0 | **未承载** |
| PositiveUnsafe | 0<η_t<η_* | 同上，`enter_ready` 的 false 分支把 Deficit/Zero/PositiveUnsafe 三种迥异状态**坍缩为同一个"未 ready"布尔值**，无法从代码状态区分账户是"资金赤字"还是"资金为正但不够安全" | **未承载** |
| PositiveSafe | η_t≥η_* | `enter_ready()` 的 `s.tw() >= self.eta_star(s)` 项为真时对应本桶——但这只是一个匿名布尔条件，代码里没有任何地方把"到达 PositiveSafe"具体化为一个可读、可路由、可入 z 的值 | 底层判据存在，**桶值本身未承载** |

**穷尽性交叉验证**：`grep -rn "Deficit\|Zero\|PositiveUnsafe\|PositiveSafe" rust/src/` 除本报告与 gap-master2 系列 `.md` 文件外零命中——四个桶名在整个 Rust 代码库中不存在任何形态（无 enum、无字符串常量、无注释别名）。

## 代码自身承认此缺口（独立于本次审查的既有证据）

- `rust/src/theta_v0/backtest/mu_estimator.rs:81`：`MuClass` 文档注释明确写"TStage/**ηBucket**/CostBucket=生产路径无数据源，诚实缺口+证明义务"。
- `mu_estimator.rs:87-153`：`MuClass` 结构体 14 个字段止于 `t_stage`（第14维，TStage 三阶段机），**没有第15维 `eta_bucket` 字段**——TStage 进了 z，ηBucket 没有。
- `rust/src/theta_v0/backtest/runner.rs:1248`注释同样只提"TStage/ηBucket『生产者已就位』的可观测物证"（指 TwState 生产者已就位，η/η_* 数据源可得），未声称 ηBucket 分类本身已实装。
- Task #149（当前 `in_progress`）标题即为"zdims-impl: **TStage/ηBucket** 进 canonical z + A4 补 TV/SubMovePower 力度分量"——ηBucket 明确被登记为待做项，尚未完成。

三处独立既有证据（mu_estimator 注释×2 + 未完成任务标题）与本次专项排查结论一致，互相印证非孤证。

## 与既有报告的口径订正

`.chanlun/review-results/full-strategy-spec-conformance-20260703.md:19` 曾把整个"§10 三阶段资金 GAP3"判 ✅（证据：GAP3 rework barrier-gated+funded_campaign+576=C）。该判定**对 TStage/Ready/EnterReady/barrier 公式部分成立**，但把 §10 作为整体打勾时**遗漏了 γ_t/ηBucket 四桶分类本身**（该判定只验证了"barrier 布尔闸门"而非"桶值可观测"）。本报告把这一颗粒度的口径分歧显式钉死：**§10 的连续量子层已装，离散 ηBucket 分类子层未装**——不是"存疑"，是确认性缺口。

## 结果包六要素

1. **结论**：γ_t 四桶（Deficit/Zero/PositiveUnsafe/PositiveSafe = ηBucket）**未在 closed_loop/strategy 任何形态承载**。底层连续量（η_t/η_*/L^wc/Q/κ）与两条布尔闸门（`enter_ready`/`buy_core_legal`）已精确对应 PDF §10 公式，但离散四态分类变量本身在整个 Rust 代码库零命中，MuClass 无对应字段。A5 判定从终稿"中·存疑"收窄为**确认性缺口**。
2. **定义依据**：`docs/formal-chain/完整的策略.pdf` §6（z 向量第16维"ηBucket：负成本缓冲状态"，page 4）+ §10（γ_t 四值分段定义 + η_*/EnterReady/BuyCore 公式，page 7）。逐页 Read pages 核对，非转引。
3. **边界条件**：若未来在 `ledger.rs`/`mutex.rs`/`interp.rs` 之外发现独立的 η 分类路径（本次穷尽 grep 未覆盖到的新增文件/分支），或 Task #149 落地后引入该字段，本判定翻转为"已承载"。当前 HEAD 下判定成立。
4. **下游推论**：①A5 应转为实装任务（枚举+路由，非"专项确认"性质）；②Task #149（zdims-impl，in_progress）范围本就包含 ηBucket，A5 的实装工作量应并入 #149 而非新开任务，避免同一缺口重复立项；③`full-strategy-spec-conformance-20260703.md:19` 的 §10 ✅ 判定需要标注"细分层：TStage/barrier 子层✅，ηBucket 子层✗"的降级备注，防止后续排查误读为"§10 全绿"。
5. **谱系引用**：本次审查未发现该缺口此前有专门谱系条目（gap-master2 终稿 A5 行是首次显式提出，本报告是其确认性收尾）；231号（诚实 None 不伪造，mu_estimator.rs 多处沿用同一先例）适用于说明为何 `t_stage`/`risk_mode` 等字段誠实标 `Option::None` 而非伪造默认值——ηBucket 若后续实装应遵循同一"诚实 None"惯例（无 TW 账本口径时 `Option::None`，非伪造 `Zero`）。
6. **影响声明**：仅新增本文件（`.chanlun/review-results/a5-gamma4-confirm-20260703.md`），不改任何 `.rs` 代码。不影响任何模块行为；影响范围限于 gap-master2 终稿 A5 行判定收窄（存疑→确认缺口）与 Task #149 的范围界定（ηBucket 实装应在 #149 内完成）。

## 严格实装面（供 #149 或后续实装任务参照，供其判断是否并入范围）

若裁定需要实装（本报告不做实装，仅给出实装面）：

- **枚举定义**：`EtaBucket { Deficit, Zero, PositiveUnsafe, PositiveSafe }`，置于 `ledger.rs` `RiskPolicy` 旁；分类函数读 `TwState::tw()` 与 `RiskPolicy::eta_star()`（两者已存在，零新增数据源）。
- **路由接入点**：`MuClass` 补第15维 `pub eta_bucket: Option<EtaBucket>`，生产赋值点与 `t_stage`（`mu_estimator.rs:152`）同源——即 π fill loop 账本态可得处（`runner.rs` `tw_final`/`TwStepCtx.state`）；`from_certificate` 裸证书路径恒 `None`（同 `t_stage`/`risk_mode`/`horizontal` 先例，231号诚实 None）。
- **消费者清单**：perm_test/wverify 分桶键（若需按资金缓冲状态分层看 μ̂ 分布，同 `t_stage` 分桶讨论适用）；`interp.rs`/`mutex.rs` 目前只消费 `enter_ready` 的匿名布尔结果，若要在 EnterReady 触发日志/诊断中区分四态需要接入点（非阻塞性——布尔闸门本身不依赖桶值）。`risk.rs::RiskMode`（M0-M4，equity vs maint_margin+buffer1/buffer2）是独立轴，不与 ηBucket 合并（§6 明文两者并列独立维）。
