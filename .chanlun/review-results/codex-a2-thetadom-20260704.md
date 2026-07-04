# codex 裁定：ForceStateA5 与 Θ_DOM 同构性（A2 收窄边界③）

- 工位：ws-thdom（task #163）
- 日期：2026-07-04
- 触发：gap-master2-final A2 行收窄修订的撤销条件（shard4 边界③——"若 codex 认定 ForceStateA5 与 Θ_DOM 语义不同构则收窄不成立"）
- 方式：`codex exec --skip-git-repo-check --sandbox read-only`，证据 = prereg-beta-div-20260703.md 全文 + beta-bucket-design-20260703.md §2.3/§5.1 + divergence.rs:300-381 现码
- 问题：ForceStateA5（现码 5-proxy 𝒜₅=𝒜₄∪{tv}）与 prereg 冻结的 Θ_DOM（ForceStateA4，𝒜₄）是否同构——A2 收窄（残余仅=判定口径接入+三口径 OOS）成立与否

## 裁定原文（逐字）

> VERDICT: ISOMORPHIC-WITH-AMENDMENT-收窄成立但prereg须改
>
> 支配序结构同构：`ForceStateA5::force_state()` 仍是同一个四态支配序原语：全 ≤ 且至少一维 < 为 `Dominated`，全 ≥ 且至少一维 > 为 `Dominates`，全等为 `Tie`，冲突为 `Incomparable`。所以不需要另起一套 4-proxy 全量重实装。
>
> 差异在 `𝒜` 族，不在判据结构：`𝒜₄ → 𝒜₅` 是给同一 Θ_DOM schema 加一个 proxy 参数 `tv`，会改变 estimand/output，不能把 A5 OOS 冒充 prereg A4 OOS。
>
> 预注册纪律下，`𝒜₄` 属于冻结项，写死不得事后调。若不改 prereg，OOS 必须按 A4 口径跑；若接受当前 A5，则须先 amend prereg，把主口径明确改成 `ForceStateA5 / 𝒜₅`，再跑三口径 OOS。
>
> 工程裁定：A2 "支配序原语已装"成立；残余可以接 `force_state()==Dominated` 到 `judge_first` 开关。但统计/报告口径必须标 A5 amended，不能标原 prereg A4。

## 消费（本工位执行序）

1. **A2 收窄不撤销**——不回 4-proxy 全量重实装，残余=判定口径接入 judge_first 开关 + 三口径 OOS。
2. **prereg amendment 先于跑数**——新 prereg（prereg-a2-thetadom-oos-20260704.md）显式声明主口径 = ForceStateA5/𝒜₅ amended（引用本裁定），三口径 OOS 全部标 A5 口径，不冒充原 A4。
3. **判定口径开关显式非默认**——默认 MacdArea（现行对照口径 bit-exact 不变），ThetaDom/Conjunction 仅经显式配置激活。

## 边界

若编排者否决 A5 amendment（要求严格按原 prereg 𝒜₄ 跑），则 OOS 须构造 A4 投影口径（force_state 比较剔除 tv 维）——本裁定第三段给了该退路，工程上是 force_state() 的 4-proxy 变体，不推翻同构结论。
