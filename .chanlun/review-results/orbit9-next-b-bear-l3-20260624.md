# orbit9-next-b：真 bear 段 L3 验 H⁰ flip 门有效域

**轴**：orbit9 下一轮轴 (b) — 真 bear 段 L3 验 H⁰ 有效域
**日期**：2026-06-24
**整合 worktree**：`/private/tmp/orbit9-integration-wt`（HEAD `1700b1f3b2`，分支 `orbit9-D-integration-20260624`，A-only / B·C 仍死代码）
**测试**：`rust/src/recursive_t/rec_stream.rs::bear_validate_h0_flip_gate_l3`（新增 `#[ignore]` 诊断测试，纯只读对照，不改任何业务逻辑）
**日志**：`/tmp/bear_h0_flip_gate.log`
**认识论等级**：**L3**（真 bear regime 真实数据，CL/ES/BRN 五窗，可否证）

---

## 1. 结论

**双层结论：**

**（A，L0/定义层）H⁰ flip 门在 A-only 生产配置（Face A）下是架构级死代码。** Face A 走 LegPair 路径
（`consume_leg_pairs`，rec_engine.rs:2386），完全旁路 H⁰ 门所在的 instances `route_bsp`（rec_engine.rs:2566/2568）。
5/5 bear 窗口实证：`enable_h0_skeleton=true` 下 `n_h0_flip_blocked=0`，nav 与 OFF **bit-exact**（逐字一致），`flips=0`。
这比 lead 假设依赖的 "A'' 抢先" 更根本——不是 type1 被 trend_done_clear 先清，而是 H⁰ 门所在的整条 route_bsp 路径在 Face A 下从不被调用。

**（B，L3/经验层）在 H⁰ 门真触达的路径（instances，`enable_reading_b_pair=false`）上，H⁰ 门在 bear 段不是单调避灾——有效域是 regime×标的函数，2/5 窗口 Δ 翻正。**

| bear 窗口 | BH | INST OFF | INST+H0 | Δ | h0_blocked | 判定 |
|---|---|---|---|---|---|---|
| CL_2014-16油崩 | −74.3% | −28.8% | −32.7% | **−3.8pp** | 14 | 未避灾 |
| CL_2020COVID崩 | −69.3% | +7.0% | −29.2% | **−36.2pp** | 10 | 未避灾（最大灾难） |
| ES_2022标普熊 | −22.9% | +16.7% | +22.1% | **+5.4pp** | 22 | 避灾✓Δ翻正 |
| BRN_2020COVID崩 | −61.7% | +46.7% | +29.4% | **−17.2pp** | 3 | 未避灾 |
| BRN_2022H2跌 | −36.3% | +14.0% | +15.3% | **+1.2pp** | 4 | 避灾✓Δ翻正 |

**净判定：lead 的"bear 段 H⁰ 必避灾⇒Δ翻正"假设被 L3 否证（3/5 反例）。** H⁰ 门在其"理应有益"的 bear regime 下仍只 2/5 正，最大反例 CL_2020COVID −36.2pp（H⁰ 拦截把 +7.0% 拖到 −29.2%，short_pnl 从 +21171 翻成 −12874，且引入 liq=1）。

**FACE_A 生产路径**：5/5 全部 Δ=+0.0pp（bit-exact），h0_blocked=0——生产配置下 H⁰ 门无任何效果（死代码），bear/net-up 同理。

## 2. 定义依据

- **H⁰ flip 门定义**（`enable_h0_skeleton`，rec_engine.rs:160-169 + task#39 / 587 候选A / 541 P2 时序约束）：
  instances 路径 `route_bsp` 核心级（None 分支）反向 flip（O1 整仓换向）须 `flip_confirmed = type1 走势完成`
  （φ=0 seam，`view.t1buy[j]`/`view.t1sell[j]` 派生）才合法；非 confirmed（背驰段 ⊋ type1，未创新高）⇒ no-op（骑走势）。
- **flip_confirmed 来源**（rec_engine.rs:2561-2568）：`view.t1buy: [bool; MAX_LEVEL]`（类型已核验，非 bar index，无类型错位 bug）。
  组2 OFF 的 `flips=2/2/3/1/2` 在 H0-ON 后全变 0 ⇒ instances 路径的核心 flip 触发时 `t1buy/t1sell=false`（全是背驰段提前翻向），
  被 H⁰ 门**正确全拦**——这恰证明 instances 核心 flip 本来就全是非 confirmed 提前翻向。
- **路径分派定义**（rec_engine.rs:2386-2387）：`if self.enable_reading_b_pair { consume_leg_pairs(...) }` ⇒ Face A（生产）
  走 LegPair，不调 route_bsp ⇒ H⁰ 门不可达。`off()`（`enable_reading_b_pair=false`）走 route_bsp ⇒ H⁰ 门唯一真触达路径。
- **bear 窗口口径**：与 `bear_validate_core_short_l3` 同口径（`load_clean_ohlc_window`，dates 列闭区间切片，CL/ES/BRN parallel-array+dates 已核验）。

## 3. 边界条件（结论翻转条件）

- **结论 A 翻转**：若生产引擎从 Face A（LegPair）切回 instances 路径（`enable_reading_b_pair=false`），H⁰ 门即从死代码变为活跃——
  但那不再是当前 A-only 生产配置。当前 orbit9 整合（HEAD 1700b1f3b2）是 A-only，故 H⁰ 门在生产层无效成立。
- **结论 B（2/5 正）翻转**：若 bear 窗口选择改变（更多窗口 / 不同标的 / 不同 regime 强度），Δ 符号比例会变——
  2/5 是当前五窗的经验值，非定理。若接通 B/C（9 轨道完整分类）后 H⁰ 门与 dispatch/nest 协同，Δ 符号可能重新分布（不在本轴范围，见 §6）。
- **若 BTC1m 补 dates 列**：用户押注的 BTC2022 核心 bear（−78%）当前不可测（btc_1m_full.json 无 dates 列，fail-loud），补列后可加入对照，可能改变净判定。

## 4. 下游推论

- **对 orbit9-next-a（接通 B/C）**：H⁰ 门要在生产配置（A-only Face A）下产生任何效果，前提是引擎在 Face A 下也能调到 H⁰ 门所在路径——
  当前不能。a 轴若要测"完整 9 轨道分类的 bear L3"，必须先解决 H⁰ 门在 LegPair 路径下的可达性（H⁰ 门是 instances 路径专属 vs LegPair 路径需独立的 flip confirmed 门），否则 a 轴的 bear 分类同样测不到 H⁰ 门。
- **对 H⁰ 门的有效域声明**：H⁰ 门"在 bear 段避假翻向灾难"的声明，**有效域 ⊊ 定义域**——定义域=所有 instances 核心反向 flip；
  有效域 = {ES_2022, BRN_2022H2} 这类窗口（2/5），其余 bear 窗 H⁰ 拦截反而劣化（拦掉了有益的做空翻向 / 引入 liq）。
- **对 osc 腿失血诊断链**：CL_2020COVID 反例（H⁰ 拦 flip ⇒ short_pnl +21171→−12874）说明在急崩 bear（−69%）中，instances 核心 flip 翻空是**有益的吃熊做空**，H⁰ 门把它当背驰段拦掉 = 误伤吃熊腿。与 `project_t_short_leg_regime_function`（空头腿=regime 函数）同构。

## 5. 谱系引用

- **H⁰ 有效域 = regime×标的函数**：与 `project_t1_direction_fix_level_split`（T1 次级别 wrong-side 可约 ⊥ 574 lag 不可约）、
  `project_constitutive_throughput_falsified`（settle 门 L3 否证，唯一存活 {CL,DX}）同模式——形式化工具有效域严格小于定义域（231号规则）。
- **死代码判别**：与 `project_signal_layer_prove`（raw type1≠走势完美点，candidate/located 分离非 bug）、
  rec_stream.rs:1097-1102 ORBIT9 观测注释（"type1 走势完成若先被 A'' 清仓 ⇒ H⁰门触达率≈0=生产死代码"）同链——本轴坐实了比 A'' 抢先更根本的**架构旁路死代码**。
- **bear 段做空**：与 `project_unn_btc_spawn_throwback`（空头操作域 ⊂ 非上行）、`project_t_short_close_level_mismatch`（下跌段做空平空级别错配）相关——bear 段做空翻向的收益依赖级别匹配，H⁰ 门一刀切拦截不区分级别。
- **是否有相关谱系**：H⁰ flip 门（587 候选A / task#39）本身是 orbit9 在建谱系，本轴是其 bear regime 有效域的首个 L3 否定性结果。

## 6. 影响声明

- **改动**：在 `/private/tmp/orbit9-integration-wt/rust/src/recursive_t/rec_stream.rs` 新增 `#[ignore]` 诊断测试
  `bear_validate_h0_flip_gate_l3`（约 90 行，纯只读对照：构造 EngineConfig 唯一翻转 `enable_h0_skeleton` 跑 OFF/ON）。
  **不改任何业务逻辑、不改现有测试、不改数据文件** ⇒ A-only observation-only 整合不变，生产 nav bit-exact 不受影响。
- **影响模块**：仅测试层（rec_stream tests 模块）。无下游模块受影响。
- **未提交**：该测试改动当前在 worktree 未 commit（`git status` 仅 `M rust/src/recursive_t/rec_stream.rs`）。是否合入由 lead/a 轴裁决。
- **数据可得性**：bear 数据**可得**（CL/ES/BRN 5 窗，dates 列切片）。**缺口**：BTC1m 无 dates 列 ⇒ 用户押注的 BTC2022 核心 bear 不可日期切片（fail-loud 声明，未假装跑）。

---

## 附：本轴与 lead 假设的张力

lead 假设链：「H⁰ flip 门拦所有反向 flip → net-up 拦益 flip 踏空劣化；bear 段拦假翻向避灾 → Δ 翻正」。

本轴 L3 发现该假设有**两个未声明前提**：
1. **前提 1（H⁰ 门可达）在 A-only 生产配置下不成立**——Face A 旁路 route_bsp，H⁰ 门 0 触达（架构死代码）。
2. **前提 2（bear flip 全是假翻向）不成立**——bear 段 instances 核心 flip 中有相当部分是**有益的吃熊做空翻向**（CL_2020COVID 急崩 +21171 short_pnl），H⁰ 门一刀切拦截误伤之 ⇒ 3/5 窗口劣化。

H⁰ 门的真实有效域不是"bear regime"这个粗粒度划分，而是更细的"背驰段假翻向 vs 走势完成真翻向"——后者在 bear 急崩段恰恰频繁出现且有益，H⁰ 门用 `flip_confirmed`（type1）作判据时把它们与背驰段一起拦了（instances 核心 flip 触发时 t1=false 是普遍现象，见 §2 全拦观测）。
