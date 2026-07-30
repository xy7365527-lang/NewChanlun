# Agent Roster 2026-07-27

## `/improve-codebase-architecture` 架构摩擦勘察（主控 session）

编排者 invoke skill。按 CLAUDE.md「开之前先搜」先查 tracker，发现本 skill **此前已跑过一轮**（#121–#125 那批 Candidate，其中 #121/#122/#123/#124 已 CLOSED），故本轮定位为**接着上一批走**，不从零重推。

Scope（YAGNI，按近 120 commit 热点，且排除已有票覆盖项）：
- 排除：`strategy/coverage.rs`（#359）、`backtest/runner.rs`（#84 SPEC 齐备待实装）、`backtest/fill.rs`（#125）
- 勘察：热点里**无票覆盖**的部分

| 类型 | 模型 | 任务 | 状态 |
|---|---|---|---|
| Explore（后台，只读） | sonnet | 策略层摩擦勘察：`strategy/` 下 interp/mod/channel/exit/account/short_diff_bucket/center_oscillation_trade/shadow/ledger/voice/oscillation_campaign —— 模块深度 / seam 质量 / locality / 可测性 / 删除测试 | **完成并验收**。3 条（interp fold 双驱动 / mod.rs stop_in 重复 / account 残余判据）。主动报告大片「查无」未凑数。主控订正 1 处：C3「逐字节相同」实为「代码体 22 行逐字相同、行尾注释有别」 |
| Explore（后台，只读） | sonnet | 回测层摩擦勘察：`backtest/` 下 wverify_run/econ_positive/opsem_dump/admission/pan_div + `bin/strict_nest_check.rs` —— 重点找**实验骨架抄两遍**与**同一统计口径多处各自实现**（静默数字不一致风险） | **完成并验收**。命中重点：`econ_positive.rs` 本地 `neff_autocorr`/`block_bootstrap_pvalue` vs 冻结口径 `decontam::effective_n`/`perm_test`，两份独立实现无交叉测试。主控订正 2 处：①不采纳「本地那份是已知有偏的旧法」（decontam 批评的是「单 lag ρk≤0 截断」，econ 用的是「正项全累加不截断」，非同一做法，方向需实测）；②跨文件骨架统一撤回（wverify_run 走锚定日期窗，切分哲学不同，agent 自撤） |

**产出**：`$TMPDIR/architecture-review-20260727-062850.html`（5 候选 + 查无登记 + Top recommendation）

## wayfinder 新图 #390 charting（同日第二轮，主控 session）

编排者选定：候选 C1 上图，C2/C3/簿记走实装线。三轮 grilling 定形：

1. **目的地** → 「验收判据该不该唯一化、唯一化到哪一套」的裁定（图出裁定即交棒 `/to-spec`，不在图内落码）
2. **边界** → 含生产准入门 `selector::chi_open_gate_lcb`（「回测与实盘是否同口径」在图内）
3. **立足点** → 先分「什么算验收」——探索性分析用自己算法是正常科研实践；只有被当作验收结论的产出必须走冻结口径。⟹ 图的主线是**划线**而非**合并**

**开图前先搜**（CLAUDE.md 硬规则）：搜 neff / effective_n / 有效样本 / 置换检验 / interp / fold / stop_in / 止损 / 测试归位 —— **四条均无既有票覆盖**，可放心开。副产品：查出 C2 是 #202 关票时的明知遗留（其 resolution 尾句「fold 变体复制维护风险在案」）。

**范围勘察发现**（主控 grep，非 agent）：统计口径独立实现散在**七个 module**——LCB 四处 / 块 bootstrap 两处 / p 值三种方法，彼此无一致性测试。其中 `selector::chi_open_gate_lcb` 是生产准入门，非实验路径 ⟹ 问题性质从「代码整洁」抬到「回测与实盘是否同口径」。

**图**：#390 + 子票 #391（划线，grilling）/ #392（去向清点，research）/ #393（预注册约束域，research）/ #394（生产门，grilling，blocked by #391）/ #395（结论处置，grilling，blocked by #391+#392）。sub-issue 与 native blocking 均已连，frontier 复核通过。

| 类型 | 模型 | 任务 | 状态 |
|---|---|---|---|
| general-purpose（后台） | sonnet | #392 七处统计实现产出去向清点，**重点第 3 问**：`econ_positive` 本地口径的产出被采信过没有（决定整件事是代码整洁还是预注册纪律事故）→ `.chanlun/review-results/stat-caliber-output-provenance-20260727.md` | 在跑 |
| general-purpose（后台） | sonnet | #393 预注册 acc-alpha 冻结内容与约束域（一次性 / 持续 / 未规定）+ 改判据的合法路径 → `.chanlun/review-results/prereg-binding-scope-20260727.md` | 在跑 |

选 sonnet 而非更便宜的 codex：两票均硬要求落盘产出，codex 只读沙箱写不了文件（够格判定不通过是能力不匹配，非智力不足）。

**#391（划线）不派**——HITL grilling，留编排者专轮（wayfinder：charting session 不解决票）。

### 两票验收结果（主控独立核验后关票）

两份报告质量高，均主动标注"查不到"未凑数。**主控核验推翻了两条主控自己写进 map 的建图前提**：

| 建图时主控写的 | 核验事实 | 影响 |
|---|---|---|
| 「`selector::chi_open_gate_lcb` 是生产准入门」——图的边界据此扩张，编排者据此选了「含生产门」 | 零调用点（全仓仅定义 + 8 处单测）；`pi_bsp_timing.rs:435` 自组 `chi_t`+`mu_lcb` 绕过；`nautilus/` grep 全空 | #394 前提无对象 ⟹ 改造为死代码处置票并移出图 |
| 「预注册纪律 ⟹ 动 decontam 是它明令禁止的事」 | 原文第 4/32/55/105 行全用「**本轮**」限定，非持续规范；改判据合法路径**未规定** | #391 从考据题变立法题 |

其他订正：
- **性质订正**：`neff_autocorr` 首现 2026-06-30 < `decontam.rs` 07-01 < 预注册 07-02 ⟹ **不是"冻结后绕开"**，是"未经冻结未经回溯校验"
- **计数订正**：「LCB 四处独立实现」偏大。逐处比对 se 构造：`wverify_run.rs:340` 与 `mu_estimator.rs:471` 同为 `std/√n`；`econ_positive.rs:3425` 用 `σ/√n_eff` 为唯一不同者；`selector.rs:176` 转调非独立实现。⟹ 实为**两种构造，分歧点唯一**
- **措辞订正**：报告称 `pi_bsp_timing` 「内联复刻」——精确说不是复刻算法（同为 `mu_lcb`），是复刻**组合**

**要害发现**：`econ_positive` 本地口径**被采信过**，链条 = 两份「结果包六要素」归档判决 → 被具名引用为 winner's curse 先例 → 结晶进当前仍在索引的 ★★ 记忆 `project_oddeven_mu_identity.md`（定性结论至今以"结构性稳健"持有）。

### 本轮簿记动作

- #392/#393 关票，resolution 含主控核验与订正；map #390 Decisions-so-far 两行
- map #390 Notes 单列「⚠ 已推翻的建图前提」一节如实登记，标「读本图前必读」
- #394 重写为死代码处置票 + 解 sub-issue + 解 blocking + 移出图，map Out of scope 记一行
- **记忆标注**：`project_oddeven_mu_identity.md` 加「⚠ 待核标注」段（统计判据口径层，与既有 G4 有效域限定不同层）；MEMORY.md 索引行同步加待重验标记。如实登记一处**未核实项**——该记忆末称 codex 异质审查已通，而 #392 查得两份 oddeven 文档自陈审查缺席，**覆盖面待核**，两者不必然矛盾
- 图现状：frontier = #391（划线）；#395 待 #391。实装线 = #394/#396/#397/#398

## 先搜查出的既有事实（本轮不得重推）

| 票 | 状态 | 事实 |
|---|---|---|
| #84 | OPEN | `runner.rs` god module 拆分 SPEC，方案一+ 五 seam，设计文档 `chanlun/plans/runner-rs-seam-designs-20260721.md` 349 行齐备。**未实装**，runner.rs 现 8697 行 |
| #125 | OPEN | `fill.rs` FillContext 提取（fee_rate 单一来源）。Parent 写明「架构重构 Candidate 2（Strong）」= 上一轮本 skill 产物 |
| #359 | CLOSED 07-27 09:34 | coverage.rs 拆 `coverage/` 17 文件全 ≤800。**关票依据存疑，见下** |
| #121/#122/#123/#124 | CLOSED | 上一轮 Candidate 已完成：signal.rs 浅模块清理 / admission.rs 拆三模块 / runner.rs 测试归位 / NestChainGate 17 字段封装 |

### ⚠️ #359 关票依据核实不通过（主控独立核验）

Resolution 写「cherry-pick 合主线：`59248b301b` + `da72db35a2` + `28773fbc99`」。核验：

```
git merge-base --is-ancestor <each> HEAD  →  三者全部 NO
```

main HEAD = `8801183790`；`rust/src/theta_v0/strategy/coverage.rs` 仍在且为 **8436 行**（比拆分前的 6909 更大）；`rust/src/theta_v0/strategy/coverage/` 目录**不存在**。三个 commit 落在 worktree `/private/tmp/nc-review-359`（detached HEAD = `28773fbc99`）。

⟹ 拆分工作**已完成但未合入 main**，票已按「合主线」关闭。已上报编排者，处置待裁。

## 2026-07-27 wayfinder session（Kimi 编排/验收）

- 认领 #384（地图 #379 终验：enabled=true 双臂 wf8），frontier 唯一开放票，blocked-by 全关。本 session 只跑验收不写码。

## 2026-07-27 wayfinder session 2（Kimi 编排）

- 原认领 #384，因另有 session 在做，让出并改认领 #421（图 #59 SPEC：活假设账本接生产，frontier，挡 P0a #299）。
- 清场：发现 worktree 留有 #425 变异验证残留 M2（nest_lifecycle.rs 未提交改挂 ForceOvertake），备份至 /tmp/m2-mutation-residue-backup-20260727.patch 后 checkout 还原。
- 实装按图规派发 claude CLI opus 无头（/implement+/code-review 字面结构），本 session 只编排/验收/落账。

## 2026-07-27 wayfinder session 2 换票记录

- #421 实装派发（task bash-mvq23d6e）已中止：另一 session 换到 #421，让票防撞车；影子评审票 #448 保留有效（谁实装 #421 都受用）。
- 回到 #384（图 #379 终验）继续，按 matt 流程派发执行层，本 session 只编排/验收/落账。

## 2026-07-27 wayfinder session 2 终态

- #384 被并行工位（/tmp/wt-384 纯态）先行关票（resolution 16:50 UTC，commit 26dd7a81d2），图 #379 已由其关图、 fog 移交 §6 落账。本 session 的派发跑成为**独立复跑**：双锚 cmp、四层报告、全部见证读数逐项吻合（含 12 项零读数）——两条路径互证成立。
- 本 session 产物两份未提交：final-verification-dual-arm-wf8-20260727.md（493 行）+ postmortem（155 行），独有内容 = §0.1 非纯态时间线、§5.1 ADR 补充九时效逐项表（G10）、post-mortem §3.4 双次纯态核验教训。处置待编排者定（annex 提交 or 丢弃）。
- 工作区已净：26dd7a81d2 含 wverify_run.rs +5 行补列，工作树与 HEAD 一致。

- 处置落定（编排者「都要」）：annex 两报告已提交 7dbb022e1b；G10 时效注小票已开 #459（needs-triage）。

- #443+#459 派发 codex 5.6-sol fast mode（low effort，workspace-write 沙箱），task bash-jsw5usez。SPEC 全文已备 /tmp/issue443-spec-body.md 供其读。
- 重派：effort 按编排者令改 high（原 low 任务 bash-jsw5usez 已中止），新 task bash-pi9huwpk。

- #443/#459 关票、#390 关图（commit f185f921d1）。本 session 三条线全收：#384 annex（7dbb022e1b）→ #443/#459（f185f921d1）→ #390 关图。

- #429 认领后查实仍 blocked by #421（工位在跑，TDD 第3切片未提交）——影子评审对象是移动靶，按裁定等 #421 收口。已放票。

- /triage 三张落毕：#452 → ready-for-agent（cherry-pick 025b3a44f0 合 main 的机械活，brief 已挂）；#457 → bug + ready-for-human（job 触发机制三选一，等编排者裁定）；#458 → bug + ready-for-agent（补登记两张新票 + ci.yml 清单订正，brief 已挂）。均带 AI triage 免责声明。

- #452 派发 codex 5.6-sol high（task bash-mon1wj2l）：cherry-pick 025b3a44f0 合 main + 复验，硬约束「合不进不许声称完成」。#458 串行候 #452 落地后派。

- #457 裁定落账（方案 2：触发器挂 main-rewritten），转 ready-for-agent 并派发 codex high（danger-full-access 因需 .git 写 + push 快照分支），task bash-iyubnvt3。#452 修复查实另一 session 已直落 main（4f3f3bb5c9），验收 cargo check 在跑（bash-hr0iwwpg）。
- #452 关票：修复 4f3f3bb5c9 已在 main，验收 TwStepCtx 错误 0 + release lib 2093/0/141 全绿。
- #457 关票（方案 2 落地+快照验证 run 30290738502：rust-check 真触发、红因预期内；backtest_bin 步未达照实标注）。新发现落两票：#468（E0433/E0601，#412 引入，default-targets 级，bug+ready-for-agent）、#469（pytest CI 收集期缺 newchan_rust/nautilus_trader，bug+needs-triage）。#458 派发 codex high（bash-lwxy1yqu）。并发守护：另一工位在快照分支的两个 commit 已救至 rescue/ci-438-test-20260727-concurrent-20260727-1348。
- #458 关票：#470/#471 开票、ci.yml 清单钉符号名（1d21934c8a）、复验一一对应。

- 图 #445 首 session：目的地定稿（编排者「可以」）→ #436 烤定并关票（主A辅B：关票必引 hash + #N 禁复用；枚举不立）→ 图体 Decisions/fog 样本①登记。一 session 一票，#440/#444 留下个 session。
- #440 烤定关票（编排者选 A）：降级声明立、一票一提交不立、关票纪律扩三子句；收敛样本②登记。#468 修复派发中（bash-f34f2t84）。
- #468 修复落 main（44a20222c4）并关票：测试下沉子目录，9 测试真跑全绿，check 零错误。
- #444 烤定关票（选 A）：豁免=可检验+附证据，未附跳不过显式验收项；收敛样本③。fog ④⑤ 毕业成票 #476/#477（grilling，挂图 #445）。
- #476 烤定关票（选 A）：关票纪律④子句（多 commit 交付各自可编译，文档豁免），CI 硬门不立；收敛样本④。#470 修复验收过（E0609=0，仅剩 #471 四错），待提交。
- #477 烤定关票（选 A）：开票门——对照型验收必附对照物指针；收敛样本⑤，五条全同族。图 #445 决策票清零。
- #471 修复落 main（ba70e4ff7f）关票——rust-check 门端到端转绿；#482 规范落 main（4b61fe755c）关票；#480 关票；图 #445 关图。交付纪律线全收。
- 影子评审三路：#461 无 HIGH 关票（MED 计数失真已登记 #311）；#464 HIGH×2 回票 #455（UL 文档与实装冲突 + 无消费者未限定），修复派发 bash-fhhjx84t；#460 在跑。#469 方案A 在跑（bash-16ylculy）。
- #460 无 HIGH 关票（MED：coverage_step_prebuilt 消费范围声明失实，登记）。三路评审全收：#460/#461 关、#464 回票 #455 修复中。
- #469 关票（c736d28ef2，四项全过）；8 个真实失败落新票（needs-triage，建议 /diagnosing-bugs）。
- #455 回票修复落 main（11b1a02bbe），#455/#464 双关，回票环闭合。#469 关票（c736d28ef2），8 真失败落 #488。
- triage 清零：#488 → bug+ready-for-agent（diagnosing-bugs 协议，先本地vs CI 对拍 MACD）；#491 → bug+ready-for-agent（digest guard 二分，严禁原 worktree，须另开临时 worktree——#421 工位在内）。
- triage 清零（#488/#491 均 bug+ready-for-agent）；handoff 落 /tmp/handoff-20260727-issue467.md，下一会话主线 #467。
