# 陈旧票交付状态核对（#696）——L53/L54/L62/L63/L64–L68 逐票判定

> 2026-07-29 research 子代理执行。map #695 deprecated 核销尾的事实前置。
> 总账锚：`debt-ledger-census-20260729.md`（research/debt-ledger，§3 L53/L54/L62/L63/L64–L68）。
> 方法：22 张关联票 body+全评论 zoom + 代码面核对（worktree `/private/tmp/research-stale/wt` @ main `23a1869849`，主仓只读）+ digest guard 单测实跑。
> 090：核不动的照实「未能判定 + 缺什么」，见 §8。

## 0. 判定总表

| L 号 | 票 | 判定 | 一行理由 |
|---|---|---|---|
| L53 | #143 | **可关** | 同物 = `extract_signals_bit_exact_digest_guard` 红；#610 重锚+归因、#491 复核基线全绿闭合，main 实跑绿 |
| L54 | #156 | **可关** | #154 四张清单逐项勾销/显式移交（§2）；主体经 map #135/#278 双图收口交付 |
| L62 | #386 | **可关** | 执行链全关 + map #379 收口列「教义落盘」；#526/#566「现役路径」= 引 spec 内容为现行口径，非留开声明 |
| L63 | #368 | **刻意留开** | 票面 spec 范围已交付（#382/#383/#384）；留开唯一理由 = L18（阶段三真实窗口见证）候选住所 |
| L64 | #125 | **未交付** | FillContext 未提取（代码零命中）、fee_rate 未单一化；scope 前提（三 loop 含 plan_and_fill_mtm/_dual）已被 #499 退役改写 |
| L65 | #158 | **未交付** | R1 栈空性前置验证未做（0 评论，代码面无对应断言票痕） |
| L66 | #159 | **未交付** | R3 放闸未实装：`ReverseRoot` 代码零命中，最高级别向下仍只观望（recursive_t/backtest.rs:246） |
| L67 | #160 | **未交付** | R4 对称退出未实装（依赖 R3，链未起动） |
| L68 | #161 | **未交付** | R5 治理收口未实装（依赖 R3，链未起动） |

汇总：**可关 3（#143/#156/#386）｜刻意留开 1（#368）｜未交付 5（#125/#158/#159/#160/#161）**。

---

## 1. L53 #143（digest guard 待修 1763+1）——判定：可关

**对象同一性（三票比对）**：
- #143（OPEN，0 评论）：「代码恢复 opus 完成后，1763+1 的 digest guard 待修」，源 = 会话导出 `kimi-export-session_-20260721-173955.md` todo #1（2026-07-21）。
- git 考古：2026-07-21 时点全仓**唯一** digest guard 测试 = `theta_v0::classifier::signal::tests::extract_signals_bit_exact_digest_guard`（引入 `b36aa3975e` 2026-06-28；retrace_ledger 的 `GOLDEN_DIGEST` 锚是 2026-07-29 #624 才加的 `148c814312`，不可能是 #143 对象）。
- #491（CLOSED）/ #610（CLOSED）对象同为该测试。三票同物成立（090 细节见 §8.1）。

**交付证据锚**：
- 归因：#610 resolution——翻转引入提交 = `bbbd8f89fa`（P1→#214→#218 三部曲，`level_origin` 进 Debug derive + `Option<Center>`→`Option<OwnerRef>`），bisect 实证；报告 `issue610-digest-guard-attribution-20260728.md`（`e1fbda71c3`）+ `issue610-id65-supplement-20260729.md`。
- 修复：GOLDEN 诚实重锚 `f6cd18aebc`（0x90c7…→0xe6a2…）+ `d9f1860124`（#631 删空转字段二次重锚）；main 侧现行值 `0xe371_3897_d9bf_978c`（`rust/src/theta_v0/classifier/signal.rs:4385`，含完整历史值链与 #614 并线取证注释）。
- 复核：#491 resolution（2026-07-29，报告 `issue491-digest-guard-baseline-already-closed-20260729.md`，`bc98cc900a`）——基线 2205/0/138 全绿，signal 定向 56/0。
- **本 audit 独立复跑**（worktree @ main `23a1869849`）：`cargo test --lib extract_signals_bit_exact_digest_guard` → **ok（1 passed，2710 filtered）**。

## 2. L54 #156（多重赋格多空双开 e2e SPEC）——判定：可关

#156 自述：「#154 报告四张清单是本 spec 的验收对账单；收口前逐条勾销或显式移交」。逐项核对：

### 清单 1（已实现 inventory）——描述性，无待交付项
专项发现「独立反向声部代码面不存在」→ 已喂 #157 裁定（见清单 3.2）。✔

### 清单 2（T2–T7 合龙缺口）——全勾销 ✔

| 项 | 交付 | 锚 |
|---|---|---|
| T2 确认门消费（nest_confirmed 不穿透） | #146 CLOSED | merge `d83bd9422f`（+`df40d2ef6f`） |
| T3 P1–P8 单点 first-match 互斥解释器 | #147 CLOSED | merge `1dba02e86c` |
| T4 AncOK 子树清仓进主链 | #148 CLOSED | merge `ced46f4f62` |
| T5 短差显式机制（P4/CloseShortDiff/分腿/TriggerProjectionSound） | #149 CLOSED | merge `fcd18cb389` |
| T7 P7 记录桶 + 次级别完整走势检测器 | #150 CLOSED | merge `bbd6b1e39e` |
| T6 毛暴露递归约束 | #151 CLOSED | merge `744da5ff81`；**机制**落地，参数校准显式移交（map #135 移交清单 → census L16 在案） |

结构面补记：#152 终验A 曾判「行为对齐、结构偏离」（channel.rs 零生产调用），经 map #174 收口翻转为「对齐含显式让步」（#204 复审），ADR 0001 落稿（#203）。

### 清单 3（距回测全链）——3 勾销 + 1 挂起 + 1 移交

1. 清单 2 全部 ✔（上表）。
2. 独立多空双开 Spec 裁定 ✔——#157 grilling CLOSED → ADR-0002 + 修正案一（`docs/adr/0002-reverse-root-admission.md`：证书即准入/涌现互斥时序定理/机检不变量）；map #278 补完生产面：散装域全谱合法化（#281 裁定）+ ShortDiff→ReverseOpen 词汇对齐（#283，`34fed43877`，行为零改动双证）。
3. 净值验收协议 **▲ 部分勾销 + 显式挂起**——实证锚一次性交付：#153 终验B（v4_C5 重跑，L1 −3810→+62.68 转正，map #135 采纳为 Destination 实证锚关图）；但 A7 判据（μ 估计器 (entry_z, exit_z, exit_type) 分桶消费语义）至今「裁量分离」挂账在代码注释：`rust/src/theta_v0/backtest/ledger.rs:71` + `selector.rs:299`（「A7 裁量分离，team-lead 令：只生产 exit_z 入 TypedTrade」）。**挂账有代码内登记、无票**（census 漏登，见 §8.3）。
4. 真实数据 0-orders 根因——显式移交（#156 Out of Scope「另立票」）。票未立；**症状已消失**：#153 重跑真实三窗 856 笔、#283 wf8 194 笔 CloseReverseOpen、后续谱系真实数据持续出单（见 §8.4 照实登记）。

### 清单 4（距生产执行）——2 勾销 + 2 移交

1. nautilus/ 骨架不编译 ✔ 过时已解——适配层已过骨架期：`rust/src/theta_v0/nautilus/mod.rs:11-29`（订正 #524：编译进 crate、nautilus 依赖 v0.60.0 已入 Cargo.toml、真实 BacktestEngine 非空订单流 L2 验收、#333 60k 窗两口径逐位一致）。
2. 生产路径无 typed 出场 ✔——v1/dual 家族退役（#499 CLOSED）消灭两路径分叉；生产 π loop 产 typed exit/TypedTrade（wf8 194 笔 CloseReverseOpen 见证，#283 resolution）。
3. 清单 3 前置——见上。
4. 订单/持仓对账 + venue 撮合语义——显式移交（#156 Out of Scope「Nautilus 实盘接线另立票」；现役归属 = nautilus 适配层线/IBKR 支柱）。

### #156 超出清单的自带项补核

- 事件流缝（唯一行为表达缝 + 台账物化 + 禁止旁路）：**语义等价物已交付**——T 线 fold + StepTrace 事件 + TypedTrade 台账 + opsem_dump seam（`backtest/opsem_dump.rs`，B-M5/#85）；字面形态（单一 VoiceLifecycleEvent 流）未按字面实装。#169（OPEN spec）仍以「#156 已裁定的声部生命周期事件流」为其缝前提——关 #156 不杀 #169。
- US10 净值判据定死写入测试：▲ 同清单 3.3（A7 挂起）。
- 反向根半面（US1 的「大级别多头与小级别空头同时在场」之顶层形态）：**未交付**，由 R 簇 #158–#161 各自 OPEN 承担（§5–§7 独立判定），不构成 #156 关票障碍（子票独立在册）。

**结论**：对账单实质勾销/移交完毕，母图（#135）与对齐图（#278）均已按 Destination 收口，spec 票为漏关。尾巴三条照实登记：A7 挂账（代码内在案）／R 簇未动工（票在册）／0-orders 票未立（症状消失）。

## 3. L62 #386（短差账本 SPEC）——判定：可关（漏关性质）

- **交付证据**：map #379 关图决议（2026-07-27）——Destination 六项全达成，票清单 #380/#381/#366/#382/#383/#384/#414/#441/#442 **全关**，SPEC #386 明列「教义落盘」（ADR 0001 补充六~十二）；终验 #384（`26dd7a81d2` + `shortdiff-ledger-final-20260727.md`：双臂逐字节 + 六项见证）。
- **#526/#566「现役路径」引用语境**：#526 挂挡评论「#386 已落地（git log + 源内 SPEC #386 明文 + short_diff_bucket.rs 守恒路径），G5 锁随其验收补 π e2e」；#566 关图决议「短差 G5｜随 #386 现役路径｜pi_g5_realized_amount_matches」。两处均为**引 spec 内容为现行口径/承重路径**（spec 已落地之后作为教义参照），非「票须留开」的声明。全仓未见 #386 刻意留开的票面声明。
- spec 文本已入教义正本链（ADR 补充六~十二 + CONTEXT.md），关票不丢教义。

## 4. L63 #368（EarningShares 增股数通道）——判定：刻意留开

- **票面 spec 范围已交付**：#368 末条评论（2026-07-27）自报路径「#382（grilling，三题全裁）→ ADR 补充八 → #383（实装，`95e44bd813` + 补充十「到 0 判据=挂起空∧free≥本金」）→ #384（终验）」；map #379 Destination 第 4 项 ✔。
- **留开理由（归口推断，090 见 §8.2）**：L18「阶段三真实窗口见证」——map #379 fog 移交第二条：「wf8 无 campaign 到 0，EarningShares 只有合成单测背书——需能到 0 的窗」。census L18 已指 #368 为候选住所。该见证**未交付**，且除 #368 外无其他票/图住所；关 #368 即把 L18 孤儿化。
- 处置建议（供核销尾裁）：保留 #368 作 L18 住所（票面补一句 L18 挂靠留痕），或另立 L18 专票后关 #368。两案择一即闭环。

## 5. L64 #125（fill.rs FillContext 提取）——判定：未交付

- **代码面核对**（main `23a1869849`）：`FillContext` 全仓零命中；`apply_order` 已共享（fill.rs:73/:3497），但 fee_rate 仍逐 loop 现算（fill.rs:47、:3285 两处 `let fee_rate = (config.exec.commission_bps + …)`，全文件 22 处 fee_rate），cash/units/equity_curve 状态管理未集中。AC 五条无一落地：0 评论、无动工痕。
- **前提已被改写**：#125 的 scope 对象「三 bar-loop（pi_theta_fill_loop_overlay / plan_and_fill_mtm / plan_and_fill_mtm_dual）」中后两个已被 **#499 退役删除**（v1/dual 家族，CLOSED 2026-07-28；现仅存注释引用）；现存 loop 族 = `pi_theta_fill_loop`（fill.rs:718）/ `_voice`（:742）/ `_overlay`（:3264）。
- **现产线覆盖核对**：#281/#283（词汇对齐，行为零改动）与 #156 线均未触及本范围——未覆盖。
- 缺什么：FillContext 未提取、fee_rate 未单一来源化；且 scope 前提部分过时——若保留须按 #499 后 loop 拓扑重议，否则按「前提改写」关闭。

## 6/7. L65–L68 #158/#159/#160/#161（反向根 R1/R3/R4/R5）——判定：全部未交付

- **票面**：四票均 OPEN、0 评论、ready-for-agent，父 #157（grilling 已裁，ADR-0002 + 修正案一）；依赖边（#146/#147/#148/#151）全 CLOSED，R1（#158）自 #148 关闭起即可动工，从未动工。
- **代码面核对**：`ReverseRoot` 全仓零命中；「反向根」仅见于注释引用 CONTEXT.md 词条（runner.rs:1256/1860/1955 互斥完备分类断言，非放闸实装）；最高级别向下**仍只空仓观望**（`rust/src/recursive_t/backtest.rs:246`「当前最高级别向下时只空仓观望。反手做空需 TradeDir::Short 单相」）——R3 放闸分支未实装。
- **现产线覆盖核对**：map #278（#281/#283）交付的是**有父**路径（次级别首开反向 ReverseOpen 全谱合法化）；#279 深查明示「跨级反向开独立声部（正面裁定）生产无实装」。无父顶层反向根 = ADR-0002 互斥完备分类的另一半，**无任何现役线覆盖**。ADR-0002 与 CONTEXT.md:164-167 词条仍为现行教义（未废止）。
- 缺什么（逐票）：#158 = 顶层子树清仓后栈空性的端到端验证测试与结论落档；#159 = 放闸+先平后开实装（五组断言）；#160 = 对称退出（typed exit 镜像语义复用）；#161 = 记账/毛暴露/censoring 治理纳入。全链未动工，属「已立案未执行」而非「交付未关票」。

## 8. 090 未能判定/照实登记

1. **#143「1763+1」解读**：会话导出原文按仓纪律禁入仓，不可得；「1763+1」解读为「1763 过 + 1 红」依票面引文。对象唯一性已由 git 考古兜底（07-21 唯一 digest guard 测试），同物判定不依赖该解读。缺：原 session 文本（若线主存档可调）。
2. **#368「刻意留开」无票面声明**：判定依 map #379 fog 移交第二条 + census L18 候选住所的归口推断。缺：线主一句话确认（若确认 L18 另立票，#368 改判可关）。
3. **A7 判据挂账 census 漏登**：`ledger.rs:71`/`selector.rs:299` 的「A7 裁量分离（team-lead 令）」未入欠账总账 68 笔——建议核销尾补登（性质 = 挂起-有触发条件，代码内登记在案）。
4. **0-orders 另立票未立**：#154 关票登记于 map #135 候补雾，#135 关图移交清单未带走；症状自 #153（2026-07-25）起消失（真实三窗 856 笔），后续谱系持续出单。照实登记：欠账实质消尽，手续缺「另立票或销记」一句。
5. **#156 事件流缝字面形态**：字面（单一生命周期事件流 + 台账纯物化 + 禁旁路）未逐字实装，语义等价物（fold+StepTrace+TypedTrade+opsem_dump）已交付；是否接受等价物为「勾销」属裁定，本 audit 按 map #135/#278 双图已收口的事实判可关，差异照实留此。

---

*只查不改（除本报告与 #696 票内 resolution）；主仓工作区全程只读。*
