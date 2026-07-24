# #43 裁定重审材料（V3 活假设状态机切片证据包，只备料不落锤）

- 日期：2026-07-20
- 性质：**重审申请 + 证据包**。本文不代裁、不落锤；#43 现行有效，已投产切片的所有设计均不以其被推翻为前提（实装卡 §8.7）。裁定文书由 escalate 惯例另出，SUPERSEDE 条款须显式列明被替代条款编号（格式参照 `recover-trigger-nesting-ruling-20260718.md:208` 复议触发条件段）。
- 备料来源：实装卡 `chanlun/review-results/v3-lifecycle-statemachine-implementation-card-20260720.md` §8（下称「卡」）；实装证据 `chanlun/review-results/v3-lifecycle-statemachine-impl-20260720.md`（下称「实装报告」）。
- 纪律：零代码改动诉求（本文纯文档）；零 git mutation；主仓 `/Users/silencehan/Projects/NewChanlun` 零写入。

## 1. 被审裁定

- **裁定**：`dparent-leftend-p0-review-20260711.md:1`（P0 定义复议裁决 #43）。核心条款：规范父区间只消费已闭合完整 `c_p`；§3.2(5) 禁止映射（:81——不得把 episode 左端/趋势起点/confirm_src/until_start/产量锚代填 `c_start_full`）。
- **代码落点**：nest.rs:802-810（`d_parent_interval_snapshot` 只消费 `c_interval_full`）、nest.rs:987-989（装配拒绝）、strict_nest_check.rs:725。
- **能力预检**：`p43-replay-cfull-20260712.md`（BLOCKED-CAPABILITY，未产新候选/边/证书）。

## 2. 裁定当时的正确性（重审不否认）

拒绝未来函数、声明=能力——`doctrine-vs-code-nest-recursion-20260720.md:147` 前半「工程上是诚实拒绝未来函数」。本申请不挑战这一点。

## 3. 冲突陈述

同文 :147 后半——「语义上等于把『活假设』从定义域中删除：一个尚未完成的父背驰段在系统中不是任何对象」，与 **061:26**「在没有证据否定背驰之前，就要观察从65开始的一段其内部结构中的背驰情况」直接冲突；E2E-D5 `Provisional` 在系统中零载体（event-lifecycle-existing-inventory-20260720.md:21-27，Provisional/ReviseEvent 零命中、judge_at 一钟多职）。

**本切片已兑现的部分**：`NestLifecycleBook`（rust/src/theta_v0/classifier/nest_lifecycle.rs）已把 Provisional/Confirmed/Invalidated 三态 + 五钟以 sidecar 注册表落地，反超即失效（trend 复用 level_view.rs:619-621 终假、pan 复用 divergence.rs:334-349 活窗）可触发、单调不复活、judge_at 一个 bit 不动——T1–T8 全绿（实装报告 §3-§4）。活假设「在系统中是不是对象」已有第一个诚实载体。

## 4. 本切片不请求变更的部分

- N^δ 装配消费口径：仍只闭合完整 `c_p`（nest.rs:802-810/:987-989 未触碰）。
- `d_parent_interval_snapshot`/`d_parent_interval`/`terminal` 三函数语义；基例门（nest.rs:642 `terminal.confirm_side`）。
- `divergence_confirmed` 布尔口径与 judge_at 写入/回填（触碰须重过 p92 兜底对账 p92:507-510 与 p95 翻转基线 p95_carriedonly_ab.rs:473——本申请不授权）。

## 5. 提请裁定的问题清单（四问，不代裁）

**(a) Provisional 进谱系构建但 Consume 仍 Closed-only**：Provisional 父对象是否允许进入 E2E-L 谱系**构建**（P2 递降并行，E2E §4.1:150），而 `Consume_at` 仍只接 Closed（E2E §1:85）——即「构建放开、消费不放开」是否裁定合法？本切片已证明 Provisional 对象可带五钟与证伪终态存在而不污染装配输入（sidecar 纯旁路，T5 bit-exact 护栏）；待裁的是它能否成为谱系节点。

**(b) Invalidated 证据保留**：Invalidated 事件作为审计证据保留（禁删除模拟失效，E2E §1:81；本切片 entry 终态保留 + 原因码 ForceOvertake/IdentityVanished 入 revision 载荷，T6）是否进入 #43 的诚实域？#43 现行文本只规定「不消费」，未规定「不记录」；反超证据（061:26「一旦力度大于前者，那么就可以断定背驰段不成立」）在现状被丢成 `None`（level_view.rs:619-621 终假无审计），本切片已接住为 Invalidated revision。

**(c) EventKey 并轨**：身份键口径何时并轨——本切片 `LifecycleKey = (level, side, kind, seg_a, seg_c_full, b_center_start)` 结构坐标（卡 §2.2）vs E2E §6.1:248 WireV1 EventKey？并轨前白名单工程桥（除 seg_c 右端外全等判同身份，记 Supersedes）不得进入任何证书真值路径（卡 §9 限制，本切片已自觉遵守并写入模块头）。

**(d) `seg_c_full` 授权**：trend 域 provider 确认分支收束 `interval_b`/`turn_source` 到 `[c_start, t*]/t*`（level_view.rs:699-702）丢弃全段坐标——`seg_c_full` 是否授权为 provider 正式字段（加字段不改既有字段值，卡 §2.2/§6.3）？同族问题：pan 活窗观察（`c_start_live` 定位 + 段序对齐校验，卡 §3）是否授权为 provider 正式产出（现状 pan 候选只在 c 完成后产出 level_view.rs:723，pan 域没有行进中对象——#43 在候选层的同构表现）。授权后白名单桥可退役，(c) 并轨有实质路径。

## 6. 重审证据包

1. 本切片 T1–T8 单测（实装报告 §4 逐条映射卡 §7；`cargo test --release --lib` 1770 passed / 0 failed，输出照录实装报告 §3）。
2. T5 bit-exact 护栏：挂/不挂 lifecycle book 的 provider 事件流逐字节相等（真实 provider 夹具）——sidecar 不反流改生产的可执行证明。
3. 教义行锚（卡 §1.2 亲读）：061:26（活假设/反超证伪/递降观察三语义同句）、024:24（完成时复核仍弱才结算）、024:40（未完成不结算）、024:46（回中枢是后果定理非确认前提）、024:28（面积乘 2 外推，后续切片选项）。
4. `doctrine-vs-code-nest-recursion-20260720.md:102/:147`（活假设排除出定义域的教义冲突）。
5. 既有机器锚：trend 终假 level_view.rs:619-621、t* 重释 level_view.rs:526-539、pan 单调性 divergence.rs:291-349、judge_at 一钟多职 level_view.rs:712/:786 + p92:202-207。
6. E2E-S5 判据（E2E §6.3:322）与 S1 predicates（E2E §6.3:318）最小集——本切片 `assert_invariants`（nest_lifecycle.rs:558-619）= S5 的 entry 级前身。

## 7. 程序

按 escalate 惯例出裁定文书；SUPERSEDE 条款显式列明被替代条款编号（参照 `recover-trigger-nesting-ruling-20260718.md:208` 复议触发条件格式）。重审前 #43 现行有效；本切片及本申请均不以其被推翻为前提。若 (a)-(d) 全部驳回，本切片产出仍作为观察记录自存（sidecar，不进任何真值路径），无回滚义务。
