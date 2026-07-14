# 回测报告：GAP3 EarningShares L2 变价数据可达性（gap3-l2-reachability）

工位: ws-gap3-l2（task #6） | 分支: gap3-rework-codex9-fix | 日期: 2026-07-02
认识论等级: **L2**（真实数据单标的确定性运行，产否定性结果）+ L0（架构不变量对照）

## 判决摘要（一句话）

在判据有效域 L2 变价数据（真实 BTC 全量 461万 bar）上跑 `run_closed_loop`，EarningShares **count=0**（final_stage=CostReduction，TW=128=notional_in 守恒，cum_net_cash=0）——判据「∃t TStage=III」在 L2 上**同样 FALSIFIED**。且这不是"数据不够好"：closed_loop 的**三阶段 TW 账本被接在一条价格幅度无关的事件流上**，价格幅度在 `run_closed_loop` 入口即被投影掉，永不进入 TW 账本。故 acc-gap3 依赖的「换 L2 变价即可达」逃生舱在当前 closed_loop 上为空。**本报告倾向判为「实装缺口」而非「不可弥合的定义冲突」**（econ 路径已消费价格，能力在代码库中存在，只是未接进 TW 三阶段机）——但"是否补这条通道"触及 576=C 双账本非合并边界与 TW 守恒语义，属编排者决断，故经 codex 异质审计后上浮。

## L2 实证证据

### 证据1：真实 BTC 全量 461万 bar（L2，否定性结果）
`l2_btc_earning_shares_reachability`（`#[ignore]`，`ECON_L2_MAX_BARS=5000000` 全量），实跑 stderr：
```
[L2-BTC] bars=4613111 (全量4613111) final_stage=CostReduction EarningShares_count=0
         TW=128 free=0 holding=128 withdrawn=0 notional_in=128 cum_net_cash=0
test result: ok. 1 passed; finished in 175.2s
```
- **count=0**：判据在 L2 真实变价数据上 FALSIFIED。
- **TW=128=notional_in（守恒）**：461万根真实变价 bar，TW 恒守恒于注资额 Q=128，无任何利润注入 TW。
- **cum_net_cash=0（关键）**：连"已实现利润"这个中间量都是 0——不是"利润产生了没进 TW"，而是**利润事件（CloseShareLeg）从未被生产路径派发**（schedule 只派 ShortDiff）。
- `run_closed_loop` 是 O(n)（每 bar O(1) hybrid_step），461万 bar 175s 跑通，加载 314M 是瓶颈，非 O(n²)。

### 证据2：价格幅度不变性对照（决定性，非 ignore，秒级）
`price_magnitude_invariance_closed_loop_tw`：同一 rising 布尔序列下，平缓涨（幅度 1 tick）与暴涨暴跌（幅度 99000 tick）跑 `run_closed_loop`，**TW 终态逐字段完全相同**（`assert_eq!(a.tw_state, b.tw_state)` 通过）。价格数值差 5 个数量级 → TW 账本零变化 ⟹ 引擎只消费 rising 布尔，L2 变价与 L0 同价对 TW 无差别 ⟹ EarningShares 可达性与 L0/L2 数据源无关。

## 根因：价格幅度在哪个组件、哪一行被丢弃

因果链「卖高→已实现利润→cum_net_cash→TW>Q→free>0 sound 退本金→CapitalRecovered→EnterEarning→EarningShares」的**第一环即断**，且断点是**结构性投影**：

| 组件 | 位置 | 丢弃动作 |
|------|------|---------|
| `run_closed_loop` | runner.rs:823 | `let rising = bar.close >= prev_close;` —— 把 `Tick`（价格数值）投影为 `bool`（涨/跌）。**价格幅度在此丢失**。每 bar 只把这个 bool 塞进 `MicroEvent::NewBar(rising)`。 |
| `MicroEvent::NewBar(bool)` | state.rs:54 | 事件字母表**只有一个 bool 位**，结构上无处承载幅度（无 price/Δpx 字段）。 |
| `schedule_adapter` | transition.rs:161-169 | 建仓/减仓都派 `TwEvent::ShortDiff(-Δ)`，Δ=仓位手数增量（整数网格）。移动的"现金"=手数 Δ 本身，**不乘价格**（同价语义）。从不派 `CloseShareLeg`（唯一携带 profit 的事件）。 |
| `tw_step(ShortDiff)` | ledger.rs:345 | `free+=d; holding-=d` —— free⇄holding 同价转换，TW 中性。 |

生产路径 TwEvent ∈ {ShortDiff(-Δ), ShortDiff(0), RecoverCapital(w), EnterEarning}，**全部 TW 中性** ⟹ TW 恒守恒=Q。价格幅度对 closed_loop TW 演化零影响——这是架构性质，与数据 L0/L2 无关。

## 与 576=C 双账本 / η⋆ barrier 的定义交点

### 与 576=C 双账本的交点（核心）
576=C：`AssemblyState` 并置 **R 账本**（`ledger_state`，R=Π-A-W，收益表视角）+ **TW 账本**（`tw_state`，现金流+持仓视角），二者不同构（#90 已证，非合并）。

价格幅度**确实被消费**——但只被 R 账本侧消费：
- **R 账本/econ 路径消费价格**：`plan_and_fill_mtm`（runner.rs:938 `px = bar.close × tick_size`）用真实价格 fill，mark-to-market，真实价差进 trade_pnls。
- **TW 三阶段账本不消费价格**：其事件字母表（ShortDiff/RecoverCapital/EnterEarning）by construction 价格无关，全 TW 中性。

EarningShares 是 **TW 账本的 stage**。它被完全接在价格无关的通道上。**双账本之间没有一条边把 R 账本的已实现利润搬进 TW 账本**——`cum_net_cash` 本应是这座桥（`CloseShareLeg(profit)` 让 cum_net_cash+=profit），但它在 `TwState` 内却被排除在 `tw()=free+holding+withdrawn` 之外（ledger.rs:116,147），且生产路径从不派 `CloseShareLeg`。这是 576=C 双账本"清洁分裂"的具体后果：价格活在 R 账本，EarningShares 活在 TW 账本，中间无传输边。

### 与 η⋆ barrier 的交点
`enter_ready` 要求 η≥η⋆，η⋆=L^wc+κ·Q，L^wc=notional_in−withdrawn（在险本金，ledger.rs）。达 EarningShares 需 withdrawn≥notional_in（⟹L^wc=0⟹η⋆=κ·Q=0，κ=0）。而 sound 退本金（RecoverCapital）需 free>0 且 holding≥notional_in=Q。TW=Q 守恒下 holding≥Q ⟹ free+withdrawn≤0 ⟹ free≤0，与 free>0 互斥。**η⋆ barrier 不可满足不是因为 η⋆ 高，而是因为唯一能抬高 η 的东西（已实现利润进 free/TW）没有通道**——η 被 TW=Q 钉死。现有测试 runner.rs:1916 已证此互斥；runner.rs:1932 用 `CloseShareLeg(5)` 断言 tw() 仍不变，锁死"利润不进 TW"。

## 接受 A 则 B 如何（A/B 分析，供上浮）

- **A（acceptance/acc-gap3/本任务前提）**：判据「L2 变价数据 EarningShares count>0 可达」——预设"已实现利润→TW>Q→free>0→退本金"链在 L2 成立。
- **B（引擎不变量）**：TW 守恒（`Origin.TotalWealth.twStep` + `tw_step_preserves_tw` 已结算定理）；价格幅度经 `NewBar(bool)` 投影后永不进 TW 账本；576=C 双账本非合并（#90）。

**接受 A** ⟹ 必须新建价格→已实现利润→TW 的通道：(a) `NewBar` 携带价格数值，(b) `schedule_adapter` 平仓时按真实价差派 `CloseShareLeg(profit)`，(c) profit 进 `tw()`（而非仅 cum_net_cash）。其中 (c) **推翻 B 的 TW 守恒定理**；且让 TW 账本消费价格 = 让它与 R 账本消费同一价格信号，**逼近 #90 证过非同构的两账本的再合并**（576=C 清洁分裂坍塌）。

**接受 B** ⟹ 判据「count>0」在此引擎上对**任何数据**（L0 或 L2）**范畴上不可达**——因可达性要求 B 所禁的通道。此读下判据问错了 estimand：它要求一个 TW 账本 stage 被只到达 R 账本的价格信号驱动。

**本工位倾向**：这更像**实装缺口**而非不可弥合的定义冲突——因为 (a)(b)(c) 的能力在代码库中**已存在于 econ/R 账本路径**（plan_and_fill_mtm 真实 fill），只是未接进 TW 三阶段机；且 TW"守恒"实为"同价守恒"（L0 建模选择），L2 下真实财富本应随已实现利润增长。**但**"补桥"是否违反 576=C 非合并与 TW 守恒 axiom，是 A-vs-B 的价值判断，超工位自决 ⟹ 上浮。

## 结果包六要素

1. **结论**：acc-GAP3 判据「回测 EarningShares count>0」在 L2 变价数据（真实 BTC 461万 bar）上 FALSIFIED（count=0）。根因是 closed_loop 三阶段 TW 账本被接在价格幅度无关的事件流上（`NewBar(bool)` 投影掉幅度 + schedule 只派同价 ShortDiff + cum_net_cash 不进 tw()）。倾向判**实装缺口**（econ/R 账本已消费价格，能力存在未接入），非不可弥合定义冲突；补桥的合法性属编排者决断。

2. **定义依据**：
   - `Origin.TotalWealth.twStep`/`tw()=free+holding+withdrawn`（ledger.rs:147）：TW 同价守恒，生产事件集全中性。
   - `MicroEvent::NewBar(bool)`（state.rs:54）+ `run_closed_loop`（runner.rs:823）：价格→bool 投影，幅度无入口。
   - 576=C 双账本（state.rs:161-164，#90 非同构）：价格被 R 账本消费（plan_and_fill_mtm），未被 TW 账本消费；两账本间无利润传输边。
   - η⋆ barrier（enter_ready，ledger.rs:243）：η 被 TW=Q 钉死，非 η⋆ 过高。

3. **边界条件（结论翻转条件）**：本结论在**当前 closed_loop 架构**下成立。翻转条件：接受 A 侧三处改动（NewBar 携价格 + schedule 派 CloseShareLeg 真实价差 + profit 进 tw()）则 count>0 可能达——但推翻 TW 守恒并逼近双账本合并。若 `price_magnitude_invariance_closed_loop_tw` 断言失败（存在未识别价格通道）则"引擎不消费价格幅度"诊断被否证——**待 codex 异质审计复核**。

4. **下游推论**：acc-GAP3 goal acceptance 以「L2 count>0」字面收口不可行（L2 实证 count=0）。二选一供编排者：(i) 重述 acceptance 为「机制正确（barrier-gated 算子已单元验证）+ 有效域标注（closed_loop TW 账本同价守恒，EarningShares 需 R→TW 利润桥，未实装）」；(ii) 立项实装 R 账本已实现利润→TW 桥（触及 576=C 非合并 + TW 守恒 axiom，需先裁决）。acc-gap3-earningshares-reachable-20260701.md 边界条件节「L2 变价后判据可能成立」需修正为「需实装利润桥，非仅换数据」。

5. **谱系引用**：
   - memory `project_gap3_l0_earning_unreachable`：其「退本金需已实现利润(L2)/需 L2 价格升值」归因**需订正**——L2 价格升值在当前 closed_loop 不进 TW（无桥），换数据不够，需实装。
   - memory `project_gap3_earning_shares_reachable`：机制侧修复确认正确；可达前提在 L2 亦不可达（无利润桥）。
   - acc-gap3-earningshares-reachable-20260701.md：L0 不可达证明成立；「L2 可达」边界假设被本工位 L2 实证否证。
   - `formalization-validity-domain.md`：有效域边界不在 L0/L2 数据之分，而在架构（TW 账本是否接价格）。
   - 疑似新谱系节点（closed_loop TW 账本与 R 账本的价格消费分裂 = 576=C 的可达性后果），待 genealogist 判定。

6. **影响声明**：新增两测试（rust/src/theta_v0/backtest/runner.rs 的 `#[cfg(test)] mod tests`，源码见附录），未改任何生产代码/生产路径。新增本报告。运行 cargo test（读操作）。**持久化告警**：本工位 agent 线程内 Edit/Write 工具产出未落盘（memory `reference_edit_tool_not_persisting_in_agent`）+ 疑似 commit-prep git 操作抹除未提交改动（memory `project_git_stash_wipes_uncommitted`）——本报告经 Bash heredoc 落盘（可靠）；两测试当前**不在磁盘**，附录内联源码供重加，Lead commit 需据此协调。

## 附录：两测试源码（供重加进 rust/src/theta_v0/backtest/runner.rs 的 mod tests）

```rust
    /// ★L2 可达性——价格幅度不变性对照（决定性，非 ignore，秒级确定）。
    #[test]
    fn price_magnitude_invariance_closed_loop_tw() {
        use super::super::super::strategy::ledger::TStage;
        let n = 64usize;
        let gentle: Vec<Bar> = (0..n)
            .map(|i| mk_bar(i, if i % 2 == 0 { 1000 } else { 1001 }, false))
            .collect();
        let violent: Vec<Bar> = (0..n)
            .map(|i| mk_bar(i, if i % 2 == 0 { 1000 } else { 100_000 }, false))
            .collect();
        let rising_of = |bars: &[Bar]| -> Vec<bool> {
            let mut prev = bars[0].close;
            bars.iter().map(|b| { let r = b.close >= prev; prev = b.close; r }).collect()
        };
        assert_eq!(rising_of(&gentle), rising_of(&violent), "对照前提：两组 rising 布尔序列相同");
        let a = run_closed_loop(&gentle, 1.0e6).expect("非空");
        let b = run_closed_loop(&violent, 1.0e6).expect("非空");
        assert_eq!(a.tw_state, b.tw_state, "价格幅度不影响 TW 终态（L2 变价=L0 同价，引擎不消费价格幅度）");
        assert_eq!(a.tw_state.stage, TStage::CostReduction, "两组均恒 CostReduction");
        assert_eq!(a.tw_state.tw(), a.tw_state.notional_in, "TW 守恒=Q");
    }

    /// ★L2 可达性实证（#[ignore]，真实 BTC 变价数据）。
    /// 复算：ECON_L2_MAX_BARS=5000000 cargo test --lib l2_btc_earning_shares_reachability -- --ignored --nocapture
    #[test]
    #[ignore]
    fn l2_btc_earning_shares_reachability() {
        use super::super::super::strategy::ledger::TStage;
        let config = ThetaConfig::default();
        let ds_full = match super::super::data::load_by_symbol("BTC", &config) {
            Ok(d) => d,
            Err(e) => { eprintln!("BTC 加载失败：{e}（DATA BLOCKER）"); return; }
        };
        let n_full = ds_full.bars.len();
        let max_bars: usize = std::env::var("ECON_L2_MAX_BARS")
            .ok().and_then(|s| s.parse().ok()).unwrap_or(300_000);
        let bars: &[Bar] = if n_full > max_bars { &ds_full.bars[n_full - max_bars..] } else { &ds_full.bars };
        let x = run_closed_loop(bars, 1.0e6).expect("非空 BTC bars ⟹ 闭环终态");
        let s = x.tw_state;
        let count = if s.stage == TStage::EarningShares { 1 } else { 0 };
        eprintln!("[L2-BTC] bars={} (全量{}) final_stage={:?} EarningShares_count={} | TW={} free={} holding={} withdrawn={} notional_in={} cum_net_cash={}",
            bars.len(), n_full, s.stage, count, s.tw(), s.free, s.holding, s.withdrawn, s.notional_in, s.cum_net_cash);
        assert_eq!(s.tw(), s.notional_in, "L2 实测 TW 仍守恒=Q（价格幅度不进 TW，架构必然）");
    }
```
