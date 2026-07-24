# 裁定 #64：#43 活假设定义域重审——SUPERSEDE 裁定书

- 日期：2026-07-21
- 性质：**SUPERSEDE 裁定**（编排者直接拍板，非代理签署）。编排者原话：「我觉得禅师说的是对的 你已经证伪了 问题是你该如何端到端实现」——061:26 活假设语义合法，#43 将活假设排除出定义域的副作用被证伪，进入端到端实装阶段。
- 重审材料：`chanlun/escalate/v3-lifecycle-r43-review-request-20260720.md`（四问证据包，下称「申请」）。
- 纪律：本文纯文档裁定，零代码改动；零 git mutation；主仓 `/Users/silencehan/Projects/NewChanlun` 零写入；实装授权范围见 §5。

## 1. SUPERSEDE 条款（显式列明被替代条款）

**本裁定替代：`chanlun/escalate/dparent-leftend-p0-review-20260711.md` §3.2(5)（:81 禁止映射条款）**——「不得把 episode 左端/趋势起点/confirm_src/until_start/产量锚代填 `c_start_full`」一禁了之的副作用（未完成父背驰段在系统中不是任何对象）自此解除。替代范围仅限该禁止条款所封锁的「活假设对象存在性」；§3.2 其余各款不受影响。

**本裁定不替代（#43 中维持有效的部分）**：

- #43 的**拒绝未来函数**正确性维持：N^δ 装配仍只消费已闭合完整 `c_p`——`rust/src/theta_v0/classifier/nest.rs:802-810`（`d_parent_interval_snapshot` 只消费 `c_interval_full`）、`nest.rs:987-989`（装配拒绝）、`strict_nest_check.rs:725` 全部不动。活假设对象可以**存在**于系统，但**不得作为装配输入**。
- `divergence_confirmed` 布尔口径与 `judge_at` 写入/回填口径不动（触碰须重过 p92:507-510 兜底对账与 p95_carriedonly_ab.rs:473 翻转基线——本裁定不授权）。

## 2. 四问裁定

**(a) Provisional 进谱系构建、Consume 仍 Closed-only——准。**

「构建放开、消费不放开」裁定合法。教义锚：061:26「在没有证据否定背驰之前，就要观察从65开始的一段其内部结构中的背驰情况」——「观察」以对象存在为前提；E2E §4.1:150 P2 递降并行构建 + E2E §1:85 Consume_at 只接 Closed 的分层与此一致。活假设（Provisional 父对象）自此是谱系节点；消费侧（装配/证书真值路径）仍只接终态 Closed 对象，未来函数防线不变。

**(b) Invalidated 证据保留入审计域——准。**

Invalidated 事件作为审计证据保留进入 #43 的诚实域。#43 现行文本只规定「不消费」，未规定「不记录」；061:26「一旦力度大于前者，那么就可以断定背驰段不成立」的超反证据在现状被丢成 `None`（level_view.rs:619-621 终假无审计），自此必须接住所留档（原因码 ForceOvertake/IdentityVanished 入 revision 载荷）。禁删除模拟失效（E2E §1:81）维持。

**(c) EventKey 并轨——挂起，随 (d) 实装启动。**

本切片 `LifecycleKey = (level, side, kind, seg_a, seg_c_full, b_center_start)` 结构坐标 vs E2E §6.1:248 WireV1 EventKey 的并轨，不在本裁定内单独解决；(d) 授权 `seg_c_full` 为 provider 正式字段后即具实质并轨路径，届时随实装一并定。并轨前白名单工程桥（除 seg_c 右端外全等判同身份，记 Supersedes）**不得进入任何证书真值路径**——此限制为裁定义务，非自觉遵守。

**(d) `seg_c_full` 授权为 provider additive 字段——准。**

trend 域 provider 确认分支收束 `interval_b`/`turn_source` 到 `[c_start, t*]/t*`（level_view.rs:699-702）丢弃全段坐标——授权 `seg_c_full` 为 provider 正式字段：**加字段不改既有字段值**（additive，既有字段逐 bit 不动）。同族问题一并裁定：pan 活窗观察（`c_start_live` 定位 + 段序对齐校验）授权为 provider 正式产出——pan 域行进中对象自此合法存在（#43 在候选层的同构封锁同步解除）。授权后白名单桥退役面随实装核定。

## 3. 裁定理由（教义层）

1. **061:26 一句三义全部要求活假设有载体**：「没有证据否定背驰之前」（Provisional 是默认态，非例外态）→「就要观察其内部结构中的背驰情况」（递降观察义务，观察须有对象）→「一旦力度大于前者，就可以断定背驰段不成立」（反超证伪是正常终态之一，Invalidated 非错误路径）。#43 §3.2(5) 使该三义在系统中无一可执行——证伪成立。
2. **024:24/024:40 完成时复核**：「背驰段完成的时刻」才复核力度结算（024:24）、未完成不结算（024:40）——确认时钟应是「完成时复核」，而非终态回填。此与 #43 拒绝未来函数不冲突：复核的是已闭合段的力度，不是拿未来数据回填过去判定。
3. **#43 的原始动机（拒绝未来函数、声明=能力）不被本裁定触碰**：构建放开 ≠ 消费放开；Provisional 对象携带的全部信息都是因果可得的（已发生 bar 的结构坐标），无未来函数。

## 4. 边界与禁令（实装期裁定义务）

- 装配输入不变：nest.rs:802-810/:987-989 一行不动；Provisional 对象**禁止**出现在任何 `d_parent_interval_snapshot`/装配候选/证书真值路径。
- `judge_at` 与 `divergence_confirmed` 一个 bit 不动（V3 切片已守，实装续守）。
- 白名单桥不得进证书真值路径（§2(c)）。
- additive 纪律：`seg_c_full`/`c_start_live` 只加不改；既有字段值任何 bit 变化 = 实装失败，回滚。
- 谱系学保留：Invalidated 终态 entry 禁删除；反超证据（力度对比量）入 revision 载荷。

## 5. 实装授权（端到端落地范围）

授权文件（worktree `/tmp/kimi-nest-mainline` 内，此外零写入）：

- `rust/src/theta_v0/classifier/level_view.rs`——(d) seg_c_full / c_start_live additive 字段。
- `rust/src/theta_v0/classifier/nest_lifecycle.rs`——NestLifecycleBook 从 sidecar 升格为谱系构建节点（Provisional 入谱系）。
- `rust/src/theta_v0/classifier/nest.rs`——仅允许加「谱系构建侧消费 lifecycle book」的旁路线（装配输入路径不动）。
- `rust/src/theta_v0/classifier/mod.rs`——模块导出。

验收线：`cargo test --release --lib` 全绿零变红（基线 1777 passed）；T5 级 bit-exact 护栏（挂/不挂 lifecycle book 的 provider 事件流逐字节相等）实装后重跑仍过；Invalidated 至少一个真实案例可触发、可查账（审计留档——消费侧仍 Closed-only，见 §2(a)；原文「可消费」系笔误，2026-07-21 编排者裁定改为「可查账」）。

## 6. 复议通道

本裁定为编排者直接拍板。后续若实测出现「Provisional 入谱系构建后污染装配输入/证书真值」的具体案例（带文件行号与事件流证据），可发起复议；推翻以新 escalate 文档显式 SUPERSEDE 本文为准，推翻前本文有效。

## 签字位

- [x] SUPERSEDE：dparent-leftend-p0-review-20260711.md §3.2(5)（:81 禁止映射的活假设封锁面）——编排者拍板（2026-07-21）
- [x] (a) 构建放开/消费 Closed-only——准
- [x] (b) Invalidated 留档入审计域——准
- [x] (c) EventKey 并轨——挂起随 (d)
- [x] (d) seg_c_full / c_start_live provider additive 授权——准
- [x] #43 拒绝未来函数面维持（nest.rs:802-810/:987-989 不动）
