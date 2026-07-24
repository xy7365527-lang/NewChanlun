# C1 双仓生产接线前三条——实装报告（wayfinder #68）

- 工位：worktree `/tmp/kimi-nest-mainline`（分支 `kimi-nest-mainline-20260717`），2026-07-20。
- 授权文件：`rust/src/theta_v0/backtest/runner.rs`、`rust/src/theta_v0/backtest/wverify_run.rs`（③报告串）、
  `rust/src/theta_v0/nautilus/strategy.rs`、`rust/src/theta_v0/nautilus/account_adapter.rs`；
  `backtest/dual_ledger.rs` / `strategy/ledger.rs` 只读对接（引擎/TW 语义零改）。
- 纪律：未做 git mutation；未写主仓；`rust/Cargo.toml` 未动；typed_ledger/TW/sep_legs 决策层语义未动；
  090（声明=能力）/v3（无概率推断、无回测验证策略、无 EMH 假设）。
- 注意：本 worktree 四文件在开工前已含前波未提交改动（runner.rs/wverify_run.rs 的 M8/W1 系工作），
  本报告 §7 的 diff 经锚点过滤只含本票（#68）hunk。

## ① nautilus 持仓真相源 NETTING→dual（account_adapter.rs:41-46 诚实 TODO 结算）

**实装**（`rust/src/theta_v0/nautilus/account_adapter.rs`）：

- 新增 `DualSnapshot`（account_adapter.rs:68）——双腿持仓快照，M14 `P^sep` 账户层坐标在
  nautilus 适配层的兑现：`q_long`/`q_short` 独立坐标（分腿不先净额，M13），`net_units()`/
  `gross_units()` 为派生投影（与 `DualLedger::net_units`/`gross_units` 同式）。
- `DualSnapshot::from_ledger`（account_adapter.rs:104）——**p120 dual_ledger 引擎对接**：
  双腿直取 `DualLedger`、`nav=equity(px)`、`unrealized=q⁺·(px−cost⁺)+q⁻·(cost⁻−px)`；
  守恒恒等 `equity = cash + (q⁺·cost⁺−q⁻·cost⁻) + unrealized`（单测逐字节锁）。
  门控 `any(test, feature="backtest_bin")`（随 `dual_ledger` 引擎同门控，theta_v0/mod.rs:95——
  默认 cdylib 构建无 backtest；venue 直读双腿构造 `DualSnapshot` 不受门控影响）。
- `DualSnapshot::from_netting`（account_adapter.rs:122）——NETTING 投影桥（诚实标注：
  信息已坍缩后的退化还原，产出恒 `q⁺·q⁻=0`，嵌入恒等适用域）。
- `to_account_state_dual`（account_adapter.rs:157）/ `position_dir_dual`（account_adapter.rs:179）——
  真相源映射；旧 `to_account_state`/`position_dir` 实现单源化为 ∘`from_netting`（**API 形状不变、
  数值逐字节一致**，既有调用方 `theta_strategy.rs`/`strategy.rs` 零改）。
- `ThetaCore::plan_for_bar_dual`（nautilus/strategy.rs:150）——双仓生产入口：基础投影 +
  **held 台账 × 双腿 join**（在飞声部槽 `voice_qty[depth]` = 该声部 side 的 venue 腿手数）——
  双腿共存时净投影会把退出 sizing 算成 `|q⁺−q⁻|`（甚至 0）导致缩量/丢单，join 后退出 Close
  sizing 吃腿级真值。决策管线与 `plan_for_bar` 同一实现体（`plan_for_bar_inner`，strategy.rs:177），
  入口语义不变。

原 TODO（NETTING 单净仓 vs S_Θ 多根分账口径差）就此结算：多根对冲的分账真相源 = 双腿坐标 +
held 台账 join，不再"待拆分账"。

## ② TW 接线（runner.rs:3988 TW=None 处置）

**实装**（`rust/src/theta_v0/backtest/runner.rs`，`plan_and_fill_mtm_dual` 内建 TW 账本，
镜像 π loop #124 口径 runner.rs:1416-1437，决策层零消费——纯物证）：

- 注资：整窗=一个 campaign，`free=notional_in=⌊nav0⌋`（runner.rs:3740-3752）。
- ②' ShortDiff 成本划转（runner.rs:3945-3968）：`basis = q⁺·cost⁺+q⁻·cost⁻`（`|units|·entry_cost`
  的双腿推广；空腿 `cost⁻` 为卖出净收/单位=在险市值，同净额空头口径）方向差分派
  `TwEvent::ShortDiff`，shadow 追真实 + cash-sound 钳制。
- ②'' Realize 平仓腿入账（runner.rs:3779/:3858 累计、:3969-3979 派发）：平仓腿费后 PnL
  （`FillOutcomeDual.realized`，A' 结算源）量化差分派 `TwEvent::Realize(d_pi)`——TW 漂移恒
  =⌊Σ平仓腿 realized⌋；**强平 PnL 不入**（同净额路径 forced_pnl 排除口径）。
- legacy 腿计数（#124 同语义）：ShortDiff 子声部（depth>0）开仓派 `OpenShareLeg`
  （runner.rs:3880-3887，OQ-9 守卫同形）、全平派 `CloseShareLeg(0)`（runner.rs:3802/:3891）。
- 产出：`tw_final: Some(tw)`（runner.rs:4072，快照取强平前）；函数头文档同步
  （runner.rs:3687-3704）。

**照实登记的边界**（接入点已给，非缺陷）：

1. **stage 推进机构未接**（`coverage::TwStepCtx` + κ policy + `stage_progression`，π loop
   runner.rs:1750/:2205 消费侧机构）——dual 路径无 P2/P3/P4 消费方，`stage` 恒
   `CostReduction` 是「无推进机构」的诚实镜像；接入点 = π loop 同款 TwStepCtx 装配处。
2. **E2 G5 同数锁未启用**（runner.rs:8339-8347 保持 ignore，理由已更新）：G5 断言的
   「TW ShortDiff 事件金额≡子腿 realized」是 T37 双层记账的**另一会计身份**；本接线的
   ShortDiff 承载成本基划转、子腿 realized 走 Realize 通道——同数锁需独立会计裁定
   （决策层语义，非本票范围）。
3. **RunResult 装配层不透传 tw_final**（`run_theta_v0_dual` → `RunResult` 无该字段，同净额臂
   口径）——wverify 臂① treasury 读数需 RunResult 扩展，属另一裁定（字段变更影响全部
   装配点）。

## ③ F-01 deprecated 处置（wverify_run.rs:1728-1733 + runner.rs:366-369 同族）

**处置 = 正式边界声明**（不扩大语义、不解除 deprecated、不在前视入口接线因果验收）：

- `wverify_run.rs:1722-1741`：报告串由「留待下一波协调」改写为正式边界声明——F-01
  deprecated **保留**（它不是待办项，是因果纪律的边界本身：全窗分类回放=结构确认前视 ⟹
  禁 L2/L3；合法用途=诊断/冒烟/夹具对拍）；因果路径归 `ThetaCore::recognize_current`
  （per-bar 窗口，关⑤已接 + #68① `plan_for_bar_dual` 入口）；per-bar 因果重放下的双仓臂
  验收归 LEE 合并波次（后三条登记）。TW 注记同步更新为接线后真实状态（声明=能力）。
- `runner.rs:370-373`：`run_theta_v0_dual` deprecated 文档追加 #68③ 处置登记（确认为
  正式边界声明终态）。

## ④ 单测（7 个新增，全部通过）

| 测试 | 位置 | 锁什么 |
|---|---|---|
| `dual_snapshot_from_ledger_conservation` | account_adapter.rs:263 | 双腿真相源守恒：坐标直取/投影同式/`equity=cash+basis+unrealized` 逐字节 |
| `netting_bridge_bitexact_compat` | account_adapter.rs:292 | NETTING 桥下新旧映射逐字节一致（调用方零漂移） |
| `dual_snapshot_coexistence_domain` | account_adapter.rs:309 | 共存有效域：voice_qty[0]=\|net\| 声明投影 + gross 可观测 + 等量对冲 Flat |
| `dual_entry_netting_bridge_same_intents` | strategy.rs:509 | 入口语义不变：投影桥下 dual 入口==NETTING 入口意图序列 |
| `dual_entry_close_qty_uses_leg_truth` | strategy.rs:552 | 共存时退出 sizing 吃腿级真值 300（对照净投影 250） |
| `dual_tw_wiring_conservation` | runner.rs:8457 | TW 接线守恒：漂移=⌊Σrealized⌋、强平排除、腿计数平衡、stage 恒 I |
| `dual_tw_legacy_leg_count_open_at_end` | runner.rs:8483 | 在飞腿计数=1、无平仓⟹ShortDiff 保 TW 守恒 |

E4 bit-exact 锁（`e2e_nested_disabled_bitexact`）更新为 `old.tw_final=None ∧ dual.tw_final=Some`
（equity/pnls/trades 逐字节对拍不动——决策层零漂移由该锁继续保证）；E2 G5 测试保持
ignore、理由更新（见 §2 边界 2）。

## ⑤ 测试输出（全绿零变红）

```
$ cargo test --release --lib        # /tmp/kimi-nest-mainline/rust
test result: ok. 1777 passed; 0 failed; 129 ignored; 0 measured; 0 filtered out; finished in 1.07s
```

- 基线 1770 passed → 1777 passed（+7 本票新测试），**0 failed，既有测试无一变红**。
- `cargo check --lib`（默认 cdylib 配置，无 backtest feature）：0 error——`from_ledger`
  引擎桥门控正确，默认构建零依赖膨胀。

## ⑥ 后三条未接线项登记（本票不做，照实登记）

1. **做空保证金/借券成本未建模**：dual_ledger.rs:22-23 声明沿用（runner.rs:3119-3121 同族
   声明）；双腿共存时 |net| 基 accrual 是近似口径，per-leg 精化列遗留 L6——涉成本数字
   变更须独立裁定。
2. **C2/C3 载体分支未实装**：C1（dual_ledger 引擎 + 本票生产接线）之外的 C2/C3 载体分支
   仍缺（level-exec-existing-inventory-20260720.md §② 同源登记）。
3. **LEE 合并未启动**：per-bar 因果重放下的双仓臂验收（③登记的因果闭环）随 LEE 合并
   波次推进；接入点 = `ThetaCore::recognize_current` + `plan_for_bar_dual`（#68① 已就位）。

另有两项本票新登记的边界（非后三条，见 §2）：G5 同数锁独立会计裁定、stage 推进机构
接入点、RunResult tw_final 透传裁定。

## ⑦ 改动 diff（本票 hunk，经锚点过滤；完整 diff 受前波未提交改动污染，见文头注意）

```diff
diff --git a/rust/src/theta_v0/backtest/runner.rs b/rust/src/theta_v0/backtest/runner.rs
index e43f61d88b..6a8ccd86cb 100644
--- a/rust/src/theta_v0/backtest/runner.rs
+++ b/rust/src/theta_v0/backtest/runner.rs
@@ -367,4 +367,8 @@ pub fn run_theta_v0(（节选：本票改动行±6 行上下文）
 /// 诊断/管线冒烟与合成夹具对拍。嵌套产量的因果口径验收由后续重放承担（施工图 §2 依赖层）；
 /// 生产因果接线在 `nautilus::strategy::ThetaCore::recognize_current`（per-bar 窗口）。
+///
+/// ★#68③ 处置登记：本节经 C1 票审认为**正式边界声明终态**——deprecated 保留（不解除、不扩大
+/// 语义、不在本入口接线因果验收）；前视有效域即上述三行，因果路径归 `recognize_current` /
+/// LEE 合并波次的 per-bar 因果重放。
 #[deprecated(
     note = "结构确认前视（同 run_theta_v0 F-01 口径）：全窗分类决策回放历史，产出禁用于 L2/L3 声明；因果验收由后续重放承担"

@@ -3116,5 +3683,22 @@ fn apply_voice_fill_dual(（节选：本票改动行±6 行上下文）
 ///
 /// `apply_voice_fill` 语义不变（voice_qty[depth]=手数——单脊柱下 depth↔腿 1:1，
-/// side 由 held 台账定）。TW/R 分解/typed_ledger 无接线（诚实 None/空，同 v1 净额路径）。
+/// side 由 held 台账定）。R 分解/typed_ledger 无接线（诚实 None/空，同 v1 净额路径）。
+///
+/// ★#68② TW 账本已接线（tw_final=Some，镜像 π loop #124 口径 runner.rs:1416-1437）：
+/// - **注资**：整窗=一个 campaign，`free=notional_in=⌊nav0⌋`（同 :1416）。
+/// - **②' ShortDiff 成本划转**：`basis = q⁺·cost⁺ + q⁻·cost⁻`（分腿在险成本基——
+///   净额口径 `|units|·entry_cost` 的双腿推广；空腿 `cost⁻` 为卖出净收/单位，其绝对额=
+///   在险市值，同净额空头 `entry_cost.abs()` 口径）方向差分派 `TwEvent::ShortDiff`，
+///   shadow 追真实+入账 cash-sound 钳制（同 :1555-1573）。
+/// - **②'' Realize 平仓腿入账**：平仓腿费后 PnL 累计（`FillOutcomeDual.realized`，A'
@@ -3154,4 +3738,17 @@ fn plan_and_fill_mtm_dual(（节选：本票改动行±6 行上下文）
     let mut leg_log: Vec<LegFillRec> = Vec::new();
 
+    // ── #68② TW 账本（镜像 π loop #124 口径，runner.rs:1416-1437 同源语义；tw_final 物证）。
+    //    注资口径同 funded_campaign：整窗=一个 campaign，free=notional_in=⌊nav0⌋。──
+    let mut tw = TwState {
+        free: nav0 as i64,
+        notional_in: (nav0 as i64).max(1),
+        ..TwState::initial()
+    };
@@ -3180,4 +3777,5 @@ fn plan_and_fill_mtm_dual(（节选：本票改动行±6 行上下文）
                         continue;
                     }
+                    realized_cum += fill.realized; // #68② ②''：平仓腿费后 PnL 累计（开仓腿=0 无效应）
                     track_position_transition(
                         &mut trades, &mut entry_bar_long, ql_b, ledger.q_long, i, false,

@@ -3202,4 +3800,8 @@ fn plan_and_fill_mtm_dual(（节选：本票改动行±6 行上下文）
                             *slot = None;
                         }
+                        // #68② TW 腿计数（#124 同语义）：ShortDiff 子声部全平 ⟹ CloseShareLeg(0)。
+                        if depth > 0 && tw.open_legacy_legs >= 1 {
+                            tw = tw_step(&tw, TwEvent::CloseShareLeg(0));
+                        }
                     } else if let Some(Some(h)) = held.get_mut(depth) {
                         h.exit_pending = false;

@@ -3254,4 +3856,5 @@ fn plan_and_fill_mtm_dual(（节选：本票改动行±6 行上下文）
                         continue;
                     }
+                    realized_cum += fill.realized; // #68② ②''：平仓腿费后 PnL 累计（开仓腿=0 无效应）
                     track_position_transition(
                         &mut trades, &mut entry_bar_long, ql_b, ledger.q_long, i, false,

@@ -3275,8 +3878,19 @@ fn plan_and_fill_mtm_dual(（节选：本票改动行±6 行上下文）
                     if fill.opened_qty() > 0.0 && !lo.close {
                         record_held_voice(&mut held, d);
+                        // #68② TW 腿计数（#124 同语义，:2196-2204 镜像）：ShortDiff 角色
+                        // （depth>0 子声部）开仓 ⟹ OpenShareLeg（OQ-9 守卫：EarningShares
+                        // 阶段开 legacy 腿非法——本路径无 stage 推进机构，恒 CostReduction，
+                        // is_legal_from 恒真；保留守卫调用与 π loop 同形）。
+                        if d.depth > 0 && TwEvent::OpenShareLeg.is_legal_from(&tw) {
+                            tw = tw_step(&tw, TwEvent::OpenShareLeg);
+                        }
                     } else if voice_qty.get(depth).copied().unwrap_or(0) == 0 {
                         if let Some(slot) = held.get_mut(depth) {
                             *slot = None;
                         }
+                        // #68② TW 腿计数：ShortDiff 子声部全平 ⟹ CloseShareLeg(0)。
+                        if depth > 0 && tw.open_legacy_legs >= 1 {
+                            tw = tw_step(&tw, TwEvent::CloseShareLeg(0));
+                        }
                     }
                     n_orders_executed += 1;

@@ -3320,4 +3943,37 @@ fn plan_and_fill_mtm_dual(（节选：本票改动行±6 行上下文）
         }
 
+        // ── #68② TW ②' 成本划转 + ②'' 已实现入账（π loop :1555-1591 同式镜像，置于 bar 末、
+        //    权益推进前；dual 路径无 ③ TwStepCtx stage 判据消费方——物证账本，决策层零消费）。──
+        {
+            // ②' 分腿在险成本基（q⁺·cost⁺ + q⁻·cost⁻，|units|·entry_cost 的双腿推广）方向差分
+            // ⟹ ShortDiff 划转（free⇄holding，TW 守恒构造子）；shadow 追真实、入账钳制（cash-sound）。
+            let basis_now =
+                (ledger.q_long * ledger.cost_long + ledger.q_short * ledger.cost_short) as i64;
@@ -3414,5 +4070,5 @@ fn plan_and_fill_mtm_dual(（节选：本票改动行±6 行上下文）
             n_orders: n_orders_executed,
             typed_ledger: Vec::new(), // 无腿级台账（诚实空，同 v1 净额路径）
-            tw_final: None,           // 无 TW 接线（E2 G5 同数锁待接线后启用，诚实 None）
+            tw_final: Some(tw),       // #68② TW 已接线（ShortDiff 成本划转 + Realize 平仓入账 + ShortDiff 腿计数；快照取强平前，强平 PnL 不入 TW）
             r_decomp: None,
         },

@@ -7312,7 +8338,11 @@ mod tests {（节选：本票改动行±6 行上下文）
 
     /// E2（G5 双层记账同数锁）：E1 中子空腿 realized == TW ShortDiff 事件的父降成本金额。
-    /// TW 接线到位后启用（施工图 §6 E2 原文）；当前 v1 dual 路径 tw_final=None（诚实空）。
+    /// #68② 后 TW 物证已接线（tw_final=Some：ShortDiff **分腿成本基划转** + Realize 平仓入账
+    /// + 腿计数）——但 G5 断言的「ShortDiff(d_cash) 金额 ≡ 子腿 realized」是**另一会计身份**
+    /// （T37 child.P&L≡parent.cost_reduction 的双层记账同数），本接线的 ShortDiff 承载成本基
+    /// 划转、子腿 realized 走 `Realize` 通道入账，二者不经同一事件 ⟹ G5 同数锁是**独立会计
+    /// 裁定**（决策层语义，非 #68② 接线范围），保持 ignore 待裁定后启用。
     #[test]
-    #[ignore = "TW ShortDiff 通道未接线 v1 dual 路径（tw_final=None 诚实空）；G5 同数锁待 TW 接线后启用（施工图 §6 E2）"]
+    #[ignore = "G5 同数锁（TW ShortDiff 事件金额≡子腿 realized 会计身份）需独立裁定；#68② 已接 TW 物证（tw_final=Some），但 ShortDiff 承载成本基划转非父降成本同数——语义差即未接线项"]
     fn e2e_g5_double_entry_same_number() {
         let mut cfg = ThetaConfig::default();

@@ -7412,6 +8442,57 @@ mod tests {（节选：本票改动行±6 行上下文）
         assert!(old.n_orders > 0, "夹具有效（确有交易，非空对拍）");
         assert_eq!(dual.typed_ledger.len(), old.typed_ledger.len());
-        assert!(dual.tw_final.is_none() && old.tw_final.is_none());
+        assert!(old.tw_final.is_none(), "净额 v1 路径无 TW 接线（诚实 None，不变）");
+        // #68②：dual 路径 TW 已接线（物证账本，不进 equity/pnls/trades 对拍字段——上方逐字节
+        // 断言已锁决策层零漂移；TW 守恒由 dual_tw_wiring_conservation 专测）。
+        assert!(dual.tw_final.is_some(), "dual 路径 TW 账本已接线（#68②）");
         assert!(dual.r_decomp.is_none() && old.r_decomp.is_none());
     }
+
+    /// #68②/④-b TW 接线守恒（E1 夹具：父多腿 + 子 ShortDiff 空腿共存 → 子平 → 父终点强平）：
+    /// 1. tw_final=Some；2. TW 漂移不变量 `tw()−注资 == ⌊Σ平仓腿 realized⌋`（Realize 唯一漂移
+    ///    构造子；强平 PnL 不入——父腿强平盈亏若入账此式即破，本断言同锁「强平排除」口径）；
+    /// 3. holding>0（快照取强平前，父腿在飞）；4. legacy 腿计数开合平衡归 0；5. stage 恒
+    ///    CostReduction（stage 推进机构未接的诚实镜像）。
+    #[test]
+    fn dual_tw_wiring_conservation() {
+        use super::super::super::strategy::ledger::TStage;
+        let mut cfg = ThetaConfig::default();
+        cfg.tick.tick_size = 1.0;
+        let classification = e_classification(true);
+        let tower = e_tower();
+        let bars = e1_bars();
+        let dual = plan_and_fill_mtm_dual(&classification, &tower, &bars, 1_000_000.0, &cfg);
+        let tw = dual.fill.tw_final.expect("#68②：dual 路径 TW 已接线");
+        let realized_sum: f64 = dual.fill.trade_pnls_realized.iter().sum();
+        assert!(realized_sum > 0.0, "夹具有效：子空腿平仓 realized>0");
+        assert_eq!(
+            tw.tw() - 1_000_000,
+            realized_sum as i64,
+            "TW 漂移 == ⌊Σ平仓腿 realized⌋（Realize 唯一漂移构造子；强平 PnL 不入）"
~ …（同 hunk 前波既有内容省略，与本票无关）…
+        assert!(tw.holding > 0, "强平前快照：父腿成本基在 holding");
+        assert_eq!(tw.withdrawn, 0, "无退本金事件（stage 推进机构未接）");
+        assert_eq!(tw.open_legacy_legs, 0, "ShortDiff 子腿开合平衡（腿计数通道已接）");
+        assert_eq!(tw.stage, TStage::CostReduction, "无推进机构 ⟹ stage 恒 CostReduction（诚实镜像）");
+    }
+
+    /// #68②/④-b TW 腿计数在飞态（截窗到子 ShortDiff 开仓 bar：子腿终点仍在飞）：
+    /// open_legacy_legs==1；无平仓 ⟹ 无 Realize 事件 ⟹ ShortDiff 保 TW 守恒（tw()==注资）；
+    /// holding 承载双腿成本基（q⁺·cost⁺+q⁻·cost⁻ > 0）。
+    #[test]
+    fn dual_tw_legacy_leg_count_open_at_end() {
+        let mut cfg = ThetaConfig::default();
+        cfg.tick.tick_size = 1.0;
+        let classification = e_classification(false);
+        let tower = e_tower();
+        // 截窗到 bar 17（子 ShortDiff 开仓 bar）——子腿尚无退出触发，强平前快照在飞。
+        // （E1 全窗下子腿 @19 另有退出通道平仓——引擎既有行为，非本测试目标。）
+        let bars: Vec<Bar> = e1_bars().into_iter().take(18).collect();
+        let dual = plan_and_fill_mtm_dual(&classification, &tower, &bars, 1_000_000.0, &cfg);
+        let tw = dual.fill.tw_final.expect("#68②：dual 路径 TW 已接线");
+        assert_eq!(tw.open_legacy_legs, 1, "子 ShortDiff 腿在飞 ⟹ legacy 腿计数=1");
+        assert!(dual.fill.trade_pnls_realized.is_empty(), "无平仓（强平不计 realized 口径）");
+        assert_eq!(tw.tw(), 1_000_000, "无 Realize 事件 ⟹ ShortDiff 保 TW 守恒");
+        assert!(tw.holding > 0, "双腿成本基在 holding（q⁺·cost⁺+q⁻·cost⁻）");
+    }
 }
diff --git a/rust/src/theta_v0/backtest/wverify_run.rs b/rust/src/theta_v0/backtest/wverify_run.rs
index 9f264866f3..650ee5104c 100644
--- a/rust/src/theta_v0/backtest/wverify_run.rs
+++ b/rust/src/theta_v0/backtest/wverify_run.rs
@@ -1351,4 +1367,381 @@ fn m8_e2e_all_systems_oos() {（节选：本票改动行±6 行上下文）
+    }
+
+    report.push_str(&conservation_notes);
+    report.push_str(&coexist_notes);
+    report.push_str(
+        "\n## 层3 treasury 口径注记\n\n\
+         臂①（dual 路径）TW 已接线（#68②：`plan_and_fill_mtm_dual` 内建 TW 账本——ShortDiff \
+         分腿成本基划转 + Realize 平仓腿入账 + ShortDiff 腿计数，`fill.tw_final=Some`；stage \
+         推进机构未接 ⟹ stage 恒 CostReduction，接入点=coverage::TwStepCtx 同 π loop）；但 \
+         `run_theta_v0_dual` 的 RunResult 装配层不透传 tw_final（RunResult 无该字段，同净额臂 \
+         口径）⟹ 本表臂① treasury 仍无读数，G5 同数锁（ShortDiff 事件金额≡子腿 realized 会计 \
+         身份）未启用——独立裁定项。臂②（v1 净额 `plan_and_fill_mtm`）`tw_final=None`（诚实空，\
+         不变）；臂③ 终Stage 见表列（同 m8 口径，κ=0 冻结）。\n\n\
+         ## 前视声明处置（runner.rs:363-371，#68③ 已处置=正式边界声明）\n\n\
+         臂①②入口 `run_theta_v0_dual`/`run_theta_v0` 的 F-01 deprecated **保留**——它不是待办项，\
+         而是因果纪律的正式边界：全窗分类回放 = 结构确认前视 ⟹ 产出**禁 L2/L3 认识论声明**，合法\
+         用途 = 诊断/管线冒烟/合成夹具对拍（runner.rs:363-371 deprecated note 原文即边界声明；\
+         本票处置 = 确认其为终态边界，不扩大语义、不解除 deprecated、不接线因果验收至此入口）。\
+         因果口径不走此入口：生产因果接线在 `nautilus::strategy::ThetaCore::recognize_current`\
+         （runner.rs:368，per-bar 窗口，关⑤已接 classify_with_tower + recognize_nested + \
+         #68① plan_for_bar_dual 双腿真相源入口）；per-bar 因果重放下的双仓臂验收归 LEE 合并波次\
+         （后三条登记项）。本报告臂①②一切数值按诊断口径读，不进 alpha 论据。\n",
+    );
+    std::fs::write("/tmp/m8_e2e_dual_oos.md", &report).ok();
+    eprintln!("[m8-dual] 双仓臂三窗报告落盘 /tmp/m8_e2e_dual_oos.md");
+}
+
diff --git a/rust/src/theta_v0/nautilus/account_adapter.rs b/rust/src/theta_v0/nautilus/account_adapter.rs
index a858788696..62976fb910 100644
--- a/rust/src/theta_v0/nautilus/account_adapter.rs
+++ b/rust/src/theta_v0/nautilus/account_adapter.rs
@@ -8,4 +8,13 @@（节选：本票改动行±6 行上下文）
 //! - `voice_qty[0]` ← `portfolio.net_position(instrument_id)` 的手数（v0 单声部 depth=0）。
 //!
+//! ## ★C1 双仓真相源（#68 ①）：NETTING 单净仓 → dual 双腿
+//!
+//! hedge-mode 下 venue 持仓是**双腿坐标** `q⁺/q⁻`（M14 `P^sep`，与 p120 dual_ledger 引擎
+//! `DualLedger{q_long,q_short}` 同构），NETTING `net_position = q⁺−q⁻` 是其**净投影**——
+//! 投影不可逆（`q⁺=q⁻=N` 与空仓投影同为 0，毛敞口信息坍缩）。本模块的生产真相源类型是
+//! [`DualSnapshot`]（双腿 + 引擎对接 [`DualSnapshot::from_ledger`]）；[`PortfolioSnapshot`]
+//! （NETTING）保留为**兼容投影桥**（[`DualSnapshot::from_netting`]），既有调用方 API 形状
+//! 不变、单边持仓下数值逐字节一致（dual_ledger §4.4 嵌入恒等）。
+//!
 //! ## ★骨架（nautilus 依赖未加，不编译）：真实接口锚以注释 + TODO 标注。
 //!

@@ -14,4 +23,10 @@（节选：本票改动行±6 行上下文）
 //! - `unrealized_pnl`/`realized_pnl(instrument_id) -> Money`、`account(venue) -> Account`。
 
+/// 引擎对接桥的门控说明（#68 ①）：`dual_ledger` 模块与 backtest 同门控
+/// `any(test, feature = "backtest_bin")`（theta_v0/mod.rs:95）——默认 cdylib 构建无 backtest，
+/// 故 `DualLedger` 引用与 [`DualSnapshot::from_ledger`] 随引擎同门控（生产 venue 直读双腿
+/// 构造 [`DualSnapshot`] 不依赖本桥，不受门控影响）。
+#[cfg(any(test, feature = "backtest_bin"))]
+use crate::theta_v0::backtest::dual_ledger::DualLedger;
 use crate::theta_v0::strategy::AccountState;
 

@@ -23,4 +38,8 @@ use super::order_adapter::PositionDir;（节选：本票改动行±6 行上下文）
 /// 字段对齐 context7 portfolio API：`net_position`（Decimal→f64）、`nav`（账户净值美元）、
 /// `realized_pnl`/`unrealized_pnl`（Money→f64，供 ledger R=Π−A−W 对账）。
+///
+/// ★C1 口径（#68 ①）：本类型是 [`DualSnapshot`] 的**净投影兼容桥**——NETTING 单净仓只在
+/// 单边持仓下与双腿等价；对冲双腿共存（nested ShortDiff 子腿，p120 §4.4 的新订单形态）时
+/// 投影坍缩毛敞口，生产路径应消费 [`DualSnapshot`]。
 #[derive(Debug, Clone, Copy)]
 pub struct PortfolioSnapshot {

@@ -35,4 +54,81 @@ pub struct PortfolioSnapshot {（节选：本票改动行±6 行上下文）
 }
 
+/// 双腿持仓快照（**C1 双仓真相源**，#68 ①：M14 `P^sep` 账户层坐标在 nautilus 适配层的兑现）。
+///
+/// 与 p120 dual_ledger 引擎 [`DualLedger`] 同构对接（[`DualSnapshot::from_ledger`]）：
+/// `q_long`/`q_short` 是两个独立坐标（分腿不先净额，M13 父仓保持），`net_units()` 是派生
+/// 投影（`DualLedger::net_units` 同式），`gross_units()` 暴露毛敞口（strict §11 `G=q⁺+q⁻`——
+/// 双开时净=0 但毛=2k，净约束不能替代毛约束）。
+///
+/// ★有效域（090 声明=能力）：
+/// - venue 须 hedge-mode（双腿可共存）；venue 只给 NETTING 时经 [`DualSnapshot::from_netting`]
+///   投影桥进入（此时 `q⁺·q⁻=0` 恒成立，与旧 NETTING 路径逐字节一致——嵌入恒等）。
+/// - `nav`/`realized_pnl`/`unrealized_pnl` 语义与 [`PortfolioSnapshot`] 同（账户净值/已实现/浮盈）。
+#[derive(Debug, Clone, Copy, PartialEq)]
+pub struct DualSnapshot {
+    /// 账户净值（美元，sizing 基数；引擎对接时取 `DualLedger::equity(px)` 净投影估值，M30）。
+    pub nav: f64,
+    /// 多腿手数（≥0，e⁺ 坐标）。
+    pub q_long: f64,
+    /// 空腿手数（≥0，e⁻ 坐标）。
+    pub q_short: f64,
+    /// 已实现盈亏（平仓腿费后 PnL 累计，A' 结算源——对账 TW `Realize` 侧）。
+    pub realized_pnl: f64,
+    /// 未实现盈亏（双腿浮盈和：`q⁺·(px−cost⁺) + q⁻·(cost⁻−px)`，分腿 mark-to-market）。
+    pub unrealized_pnl: f64,
+}
+
+impl DualSnapshot {
+    /// 净额投影 `Net = q⁺−q⁻`（[`DualLedger::net_units`] 同式；正=净多，负=净空）。
+    pub fn net_units(&self) -> f64 {
+        self.q_long - self.q_short
+    }
+
+    /// 毛敞口 `G = q⁺+q⁻`（[`DualLedger::gross_units`] 同式；净投影坍缩的毛信息在此可观测）。
+    pub fn gross_units(&self) -> f64 {
+        self.q_long + self.q_short
+    }
+
+    /// **p120 dual_ledger 引擎对接**（#68 ① 核心接线）：账本终态 → 适配层快照。
+    ///
+    /// - `q_long`/`q_short` 直取引擎双腿坐标（真相源，不经净投影）。
+    /// - `nav = ledger.equity(px)`（净投影估值 `cash + Net·px`，M30 有效域声明沿用）。
+    /// - `unrealized_pnl = q⁺·(px−cost⁺) + q⁻·(cost⁻−px)`（分腿浮盈和）——守恒恒等
+    ///   `equity = cash + (q⁺·cost⁺ − q⁻·cost⁻) + unrealized`（成本基+浮盈=持仓市值口径，
+    ///   单测 `dual_snapshot_from_ledger_conservation` 逐字节锁）。
+    /// - `realized_cum`：平仓腿费后 PnL 累计由调用方（fill loop）显式给——`DualLedger`
+    ///   引擎不累计 realized（其 `FillOutcomeDual.realized` 是单笔口径），不臆造累计源。
+    ///
+    /// 门控：`any(test, feature = "backtest_bin")`（随 `dual_ledger` 引擎同门控，见模块头注）。
+    #[cfg(any(test, feature = "backtest_bin"))]
+    pub fn from_ledger(ledger: &DualLedger, px: f64, realized_cum: f64) -> Self {
+        let unrealized = ledger.q_long * (px - ledger.cost_long)
+            + ledger.q_short * (ledger.cost_short - px);
+        DualSnapshot {
+            nav: ledger.equity(px),
+            q_long: ledger.q_long,
+            q_short: ledger.q_short,
+            realized_pnl: realized_cum,
+            unrealized_pnl: unrealized,
+        }
+    }
+
+    /// **NETTING 投影桥**（兼容路径）：净仓 → 双腿（`net>0 ⟹ (net,0)`；`net<0 ⟹ (0,−net)`）。
+    ///
+    /// ★诚实：这是**信息已坍缩后的退化还原**——venue 只报净仓时对冲双腿不可恢复
+    /// （`q⁺=q⁻=N` 与空仓同投影）。本桥产出恒满足 `q⁺·q⁻=0`（净语义子集，dual_ledger
+    /// §4.4 嵌入恒等的适用域）；真实对冲共存必须走 [`DualSnapshot::from_ledger`] 或
+    /// hedge-mode venue 双腿直读。
+    pub fn from_netting(snap: &PortfolioSnapshot) -> Self {
+        DualSnapshot {
+            nav: snap.nav,
+            q_long: snap.net_position.max(0.0),
+            q_short: (-snap.net_position).max(0.0),
+            realized_pnl: snap.realized_pnl,
+            unrealized_pnl: snap.unrealized_pnl,
+        }
@@ -40,12 +136,27 @@ pub struct PortfolioSnapshot {（节选：本票改动行±6 行上下文）
 /// 长度 = max_depth，仅 `[0]` 非零（= |net_position|）。多声部时按 depth 分配持仓待嵌套树扩展（TODO）。
 ///
-/// ★诚实有效域：v0 classifier 只产 depth=0 独立根（`recognize` 注释 strategy/mod.rs:594），故
-/// `net_position` 整体归 voice_qty[0]。多独立根并存（§5）时各根都是 depth=0，net_position 是它们的
-/// **净和**——这与 S_Θ「多独立根各自 voice_qty」有口径差（Nautilus NETTING 单净仓 vs S_Θ 多根分账）。
-/// 单根/单方向时无差异（v0 主路径）；多根对冲时需在适配层拆分账（标注非缺陷是有效域边界，TODO）。
+/// ★C1 处置（#68 ①，原 :41-46 诚实 TODO 已结算）：NETTING 单净仓 vs S_Θ 多根分账的口径差
+/// 由**双腿真相源**承接——本函数降级为 [`to_account_state_dual`] ∘ [`DualSnapshot::from_netting`]
+/// 的投影桥（实现单源化，数值与旧实装逐字节一致）；多根对冲并存的分账真相源是
+/// [`DualSnapshot`]（双腿坐标），per-depth 归属由 `ThetaCore::plan_for_bar_dual` 的
+/// held 台账 × 双腿 join 承载（nautilus/strategy.rs，缠论语义×数量的接入点），不在本层臆造。
 pub fn to_account_state(snap: &PortfolioSnapshot, max_depth: u32) -> AccountState {
+    to_account_state_dual(&DualSnapshot::from_netting(snap), max_depth)
+}
+
+/// 双腿快照 → S_Θ `AccountState`（**C1 真相源映射**，`to_account_state` 的单源实现体）。
+///
+/// 映射规则（与 NETTING 桥同式，`from_netting` 下逐字节一致——嵌入恒等）：
+/// `voice_qty[0] = |net_units()|`，其余 depth 槽 0；`nav` 负/零兜底为 0（防御性，同旧口径）。
+///
+/// ★有效域（090 声明=能力）：`AccountState.voice_qty` 是 per-depth 手数标量（v0 单声部单槽），
+/// **无毛敞口坐标**——双腿共存（`q⁺>0 ∧ q⁻>0`）时本投影只给净额绝对值，毛敞口 `G` 经
+/// [`DualSnapshot::gross_units`] 另读（毛闸门消费在 runner `risk::gross_units_ok`，非 sizing）。
+/// per-depth 双腿归属（根多腿 depth0 / ShortDiff 子空腿 depth1…）是 held 台账侧 join 的事
+/// （`plan_for_bar_dual`），本函数不越权分账。
+pub fn to_account_state_dual(snap: &DualSnapshot, max_depth: u32) -> AccountState {
     let depth = max_depth.max(1) as usize;
     let mut voice_qty = vec![0u32; depth];
-    voice_qty[0] = snap.net_position.abs() as u32; // v0 单声部 depth=0
+    voice_qty[0] = snap.net_units().abs() as u32; // v0 单声部 depth=0（净投影，有效域见上）
     AccountState {
         nav: if snap.nav > 0.0 { snap.nav } else { 0.0 },
@@ -55,8 +166,19 @@ pub fn to_account_state(snap: &PortfolioSnapshot, max_depth: u32) -> AccountStat（节选：本票改动行±6 行上下文）
 
 /// 从 Nautilus net_position 推出持仓方向（order_adapter 的 Close/Reduce 反向 side 用）。
+///
+/// ★C1：实现单源化为 [`position_dir_dual`] ∘ [`DualSnapshot::from_netting`]（数值逐字节不变）。
 pub fn position_dir(snap: &PortfolioSnapshot) -> PositionDir {
-    if snap.net_position > 0.0 {
+    position_dir_dual(&DualSnapshot::from_netting(snap))
+}
+
+/// 从双腿快照推持仓方向（**C1 真相源方向**）：方向 = 净投影符号。
+///
+/// ★有效域：`net=0 ∧ gross>0`（双腿等量对冲）归 `Flat`——这是**净口径方向**声明（order_adapter
+/// 的 Close/Reduce 反向 side 只需净方向），毛敞口非零由 [`DualSnapshot::gross_units`] 另读；
+/// 对冲态下的腿级退出方向由 held 台账 side 承载（`plan_for_bar_dual` join），不在本函数臆造。
+pub fn position_dir_dual(snap: &DualSnapshot) -> PositionDir {
+    if snap.net_units() > 0.0 {
         PositionDir::Long
-    } else if snap.net_position < 0.0 {
+    } else if snap.net_units() < 0.0 {
         PositionDir::Short
     } else {
@@ -105,3 +227,94 @@ mod tests {（节选：本票改动行±6 行上下文）
         assert_eq!(to_account_state(&snap(-1.0, 0.0), 1).nav, 0.0);
     }
+
+    // ── #68 ① C1 双仓真相源（DualSnapshot）测试组 ──
+
+    use crate::theta_v0::backtest::dual_ledger::apply_fill_dual;
+    use crate::theta_v0::strategy::voice::VoiceSide;
+    use crate::theta_v0::strategy::LegOrder;
+    use crate::theta_v0::types::{Order, StrictAction};
+
~ …（同 hunk 前波既有内容省略，与本票无关）…
+            },
+            leg: side,
+            close: false,
+        }
+    }
+
+    fn dsnap(nav: f64, ql: f64, qs: f64) -> DualSnapshot {
+        DualSnapshot { nav, q_long: ql, q_short: qs, realized_pnl: 0.0, unrealized_pnl: 0.0 }
+    }
+
+    /// 双腿真相源守恒（#68 ④-a）：`from_ledger` 对接 p120 引擎——双腿坐标直取（真相不经
+    /// 净投影），net/gross 投影与引擎同式，估值守恒恒等
+    /// `equity == cash + (q⁺·cost⁺ − q⁻·cost⁻) + unrealized` 逐字节锁。
+    #[test]
+    fn dual_snapshot_from_ledger_conservation() {
+        let mut ledger = DualLedger::new(1_000_000.0);
+        let mut pnls = Vec::new();
+        apply_fill_dual(&leg_open(VoiceSide::Long, 10), 100.0, FEE, &mut ledger, &mut pnls);
+        apply_fill_dual(&leg_open(VoiceSide::Short, 6), 105.0, FEE, &mut ledger, &mut pnls);
+        let px = 110.0;
+        let s = DualSnapshot::from_ledger(&ledger, px, 0.0);
+        // 双腿坐标 = 引擎坐标（不先净额，M13）。
+        assert_eq!(s.q_long.to_bits(), ledger.q_long.to_bits());
+        assert_eq!(s.q_short.to_bits(), ledger.q_short.to_bits());
+        assert_eq!(s.net_units().to_bits(), ledger.net_units().to_bits());
+        assert_eq!(s.gross_units().to_bits(), ledger.gross_units().to_bits());
+        // nav = 引擎净投影估值。
~ …（同 hunk 前波既有内容省略，与本票无关）…
+        );
+        // 共存态：净 4 但毛 16——毛信息在 NETTING 投影下坍缩，在双腿快照下可观测。
+        assert_eq!(s.net_units(), 4.0);
+        assert_eq!(s.gross_units(), 16.0);
+    }
+
+    /// NETTING 投影桥兼容恒等（#68 ④-a）：`from_netting` 下 dual 映射与旧 NETTING 映射
+    /// 逐字节一致（嵌入恒等；既有调用方零漂移）。
+    #[test]
+    fn netting_bridge_bitexact_compat() {
+        for net in [300.0, -50.0, 0.0] {
+            let s = snap(1_000_000.0, net);
+            let d = DualSnapshot::from_netting(&s);
+            assert_eq!(d.net_units().to_bits(), s.net_position.to_bits());
+            assert!(d.q_long == 0.0 || d.q_short == 0.0, "投影桥产出恒净语义子集（q⁺·q⁻=0）");
+            let a_old = to_account_state(&s, 4);
+            let a_new = to_account_state_dual(&d, 4);
+            assert_eq!(a_old.nav.to_bits(), a_new.nav.to_bits());
+            assert_eq!(a_old.voice_qty, a_new.voice_qty);
+            assert_eq!(position_dir(&s), position_dir_dual(&d));
+        }
+    }
+
+    /// 双腿共存映射有效域（#68 ④-a）：共存时 voice_qty[0]=|net|（声明的净投影），毛敞口
+    /// 经 gross_units 可观测；`net=0 ∧ gross>0`（等量对冲）⟹ 方向 Flat（净口径）+ 毛非零。
+    #[test]
+    fn dual_snapshot_coexistence_domain() {
+        let s = dsnap(1_000_000.0, 300.0, 50.0);
+        let acct = to_account_state_dual(&s, 4);
+        assert_eq!(acct.voice_qty[0], 250, "共存时单槽投影=|net|（有效域声明，非分账）");
+        assert_eq!(s.gross_units(), 350.0, "毛敞口可观测（NETTING 下此信息坍缩）");
+        // 等量对冲：净方向 Flat，毛敞口 2N。
+        let hedged = dsnap(1_000_000.0, 100.0, 100.0);
+        assert_eq!(position_dir_dual(&hedged), PositionDir::Flat);
+        assert_eq!(hedged.gross_units(), 200.0);
+        assert_eq!(to_account_state_dual(&hedged, 1).voice_qty[0], 0);
+    }
 }

diff --git a/rust/src/theta_v0/nautilus/strategy.rs b/rust/src/theta_v0/nautilus/strategy.rs
index 7a374881fe..c8419b839f 100644
--- a/rust/src/theta_v0/nautilus/strategy.rs
+++ b/rust/src/theta_v0/nautilus/strategy.rs
@@ -124,11 +124,64 @@ impl ThetaCore {（节选：本票改动行±6 行上下文）
     /// 撮合时序差异归 venue，不分叉退出判定。
     pub fn plan_for_bar(&mut self, new_bar: Bar, snap: &PortfolioSnapshot) -> Vec<OrderIntent> {
-        self.bars.push(new_bar);
-        let i = self.bars.len() - 1;
-
-        // account / 方向从 Nautilus portfolio（真实持仓真相源）。
+        // account / 方向从 Nautilus portfolio（真实持仓真相源；#68 ① 起为 dual 映射的投影桥）。
         let account: AccountState =
             account_adapter::to_account_state(snap, self.config.voice.max_depth);
         let pos_dir = account_adapter::position_dir(snap);
+        let equity_now = snap.nav + snap.unrealized_pnl;
+        self.plan_for_bar_inner(new_bar, account, pos_dir, equity_now)
+    }
+
+    /// **C1 双仓入口**（#68 ①）：持仓真相源 = 双腿快照 [`DualSnapshot`]（M14 `P^sep`，
+    /// hedge-mode venue 双腿直读 / p120 dual_ledger 引擎 [`DualSnapshot::from_ledger`] 对接）。
+    ///
+    /// 与 [`ThetaCore::plan_for_bar`]（NETTING 投影桥）的差异只在 **account 构造**：
+    /// 1. 基础投影 [`account_adapter::to_account_state_dual`]（`voice_qty[0]=|net|`，同式）；
+    /// 2. **held 台账 × 双腿 join**（退出 sizing 口径实义化）：每个在飞声部槽
+    ///    `voice_qty[depth] = 该声部 side 的 venue 腿手数`（Long→`q_long`，Short→`q_short`）——
+    ///    双腿共存（根多腿 + ShortDiff 子空腿）时净投影会把 `voice_qty[0]` 算成 `|q⁺−q⁻|`
+    ///    （甚至 0），退出 Close 单 sizing（`plan_orders` 的 `qty_at(depth)`）将缩量/丢单；
+    ///    join 后退出 sizing 吃**腿级真值**（venue hedge-mode 腿是该侧聚合，故同侧声部共享
+    ///    腿手数——v0 口径声明：同侧多声部的 per-声部拆分需 venue 腿分账，非本层臆造）。
+    ///    未持声部槽保持投影值（depth0 为 `|net|`，其余 0），与 NETTING 路径同口径。
+    ///
+    /// 决策管线（recognize_nested/退出生成器/plan_orders）与 [`ThetaCore::plan_for_bar`] **同一
+    /// 实现体**（`plan_for_bar_inner`）——入口语义不变，只换持仓数量真相源（090：声明=能力，
+    /// 单测 `dual_entry_close_qty_uses_leg_truth` 锁共存场景 sizing 差异）。
+    pub fn plan_for_bar_dual(
+        &mut self,
+        new_bar: Bar,
+        snap: &account_adapter::DualSnapshot,
+    ) -> Vec<OrderIntent> {
+        let mut account: AccountState =
+            account_adapter::to_account_state_dual(snap, self.config.voice.max_depth);
+        // held 台账 × 双腿 join：在飞声部槽吃腿级真值（见函数头注释 2）。
+        for depth in 0..self.held.len() {
+            if let Some(h) = &self.held[depth] {
+                let leg_qty = match h.side {
+                    VoiceSide::Long => snap.q_long,
+                    VoiceSide::Short => snap.q_short,
~ …（同 hunk 前波既有内容省略，与本票无关）…
+                };
+                if let Some(slot) = account.voice_qty.get_mut(depth) {
+                    *slot = leg_qty.round().clamp(0.0, u32::MAX as f64) as u32;
+                }
+            }
+        }
+        let pos_dir = account_adapter::position_dir_dual(snap);
+        let equity_now = snap.nav + snap.unrealized_pnl;
+        self.plan_for_bar_inner(new_bar, account, pos_dir, equity_now)
+    }
+
+    /// `plan_for_bar` / `plan_for_bar_dual` 的**同一实现体**（#68 ① 抽出的共享段——两入口
+    /// 只差 account/方向/equity 的构造，决策管线零分叉）。
+    fn plan_for_bar_inner(
+        &mut self,
+        new_bar: Bar,
+        account: AccountState,
+        pos_dir: super::order_adapter::PositionDir,
+        equity_now: f64,
+    ) -> Vec<OrderIntent> {
@@ -444,3 +495,108 @@ mod tests {（节选：本票改动行±6 行上下文）
         );
     }
+
+    // ── #68 ①/④ C1 双仓入口（plan_for_bar_dual）测试组 ──
+
+    use super::account_adapter::DualSnapshot;
+
+    fn dual_snap(nav: f64, ql: f64, qs: f64) -> DualSnapshot {
+        DualSnapshot { nav, q_long: ql, q_short: qs, realized_pnl: 0.0, unrealized_pnl: 0.0 }
+    }
+
+    /// 入口语义不变（#68 ④-c）：NETTING 投影桥（`from_netting`）下 `plan_for_bar_dual`
+    /// 与 `plan_for_bar` 产出**同一意图序列**（决策管线同一实现体，只换真相源入口）。
+    #[test]
+    fn dual_entry_netting_bridge_same_intents() {
+        let mut cfg = ThetaConfig::default();
+        cfg.exec.entry_delay_bars = 0;
+        let mk_core = |cfg: &ThetaConfig| {
+            let mut core = ThetaCore::new(cfg.clone());
+            core.bars.push(Bar {
+                source_index: 0, timestamp: 0, open: 1000, high: 1010, low: 900, close: 920,
~ …（同 hunk 前波既有内容省略，与本票无关）…
+        // NETTING 入口。
+        let mut core_net = mk_core(&cfg);
+        let snap_net = long_snap(1_000_000.0, 300.0);
+        let intents_net = core_net.plan_for_bar(exit_eval_bar, &snap_net);
+        // dual 入口（同一持仓经投影桥进入）。
+        let mut core_dual = mk_core(&cfg);
+        let snap_dual = DualSnapshot::from_netting(&snap_net);
+        let intents_dual = core_dual.plan_for_bar_dual(exit_eval_bar, &snap_dual);
+        // 意图序列逐项相同（side/reduce_only/qty）。
+        assert_eq!(intents_net.len(), intents_dual.len(), "投影桥下两入口意图数一致");
+        for (a, b) in intents_net.iter().zip(intents_dual.iter()) {
+            assert_eq!(a.side, b.side);
+            assert_eq!(a.reduce_only, b.reduce_only);
+            assert_eq!(a.qty, b.qty);
+        }
+        assert!(intents_dual.iter().any(|oi| oi.reduce_only), "dual 入口同样产 Close 意图");
+    }
+
+    /// 双腿真相源 sizing（#68 ④-a 核心）：双腿共存（根多腿 300 + 子空腿 50）时，退出 Close
+    /// sizing 吃**腿级真值** q_long=300（非净投影 |300−50|=250）——held×双腿 join 生效。
+    #[test]
+    fn dual_entry_close_qty_uses_leg_truth() {
+        let mut cfg = ThetaConfig::default();
+        cfg.exec.entry_delay_bars = 0;
+        let mut core = ThetaCore::new(cfg);
+        core.bars.push(Bar {
+            source_index: 0, timestamp: 0, open: 1000, high: 1010, low: 900, close: 920,
+            volume: 100, untradable: false,
~ …（同 hunk 前波既有内容省略，与本票无关）…
+        core2.held[0] = Some(HeldVoice {
+            side: VoiceSide::Long,
+            stop: 950,
+            decision: buy1_decision(0),
+            exit_pending: false,
+        });
+        let intents2 = core2.plan_for_bar_dual(exit_eval_bar, &snap);
+        let close_dual = intents2.iter().find(|oi| oi.reduce_only).expect("dual 入口产 Close");
+        assert_eq!(close_dual.qty, 300, "双腿共存 ⟹ 退出 sizing 吃腿级真值 q_long（非 |net|=250）");
+        assert_eq!(close_net.qty, 250, "净投影对照：sizing=|net|（坍缩口径，仅作对照）");
+        assert_eq!(close_dual.side, order_adapter::OrderSideLike::Sell, "平多腿 ⟹ Sell");
+        assert!(close_dual.reduce_only);
+    }
```
