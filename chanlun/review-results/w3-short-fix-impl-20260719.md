# W3 Short 机制缝隙修复实装报告（F1/F2/F4 机制层 + closePred 第五析取）

- **实装工位**：worktree `/tmp/kimi-nest-mainline`（分支 `kimi-nest-mainline-20260717`）
- **设计依据**：`chanlun/review-results/short-side-loss-attribution-20260719.md`（F1-F5，下称「审计」）
- **实装日期**：2026-07-19
- **授权文件**：`rust/src/theta_v0/strategy/exit.rs` + `rust/src/theta_v0/strategy/ledger.rs` + `rust/src/theta_v0/strategy/voice.rs`（voice.rs 评估后**零改**——F1 绑定复用既有 `voice_side`/`depth` 机制，最小改动纪律）
- **纪律**：090（声明=能力，照实登记接线边界）；v3 硬禁令（无概率/统计推断、无回测验证、无 EMH）；未做 git mutation；主仓只读；`rust/Cargo.toml` 未触碰。

---

## 0. 总览

| 任务 | 实装点 | 状态 |
|---|---|---|
| ① closePred 第五析取 MFE 回撤止盈 | exit.rs:298-358（机制）+ exit.rs:156-170（wrapper） | **机制+单测完成；生产接线 armed-but-inert**（runner.rs/nautilus 授权外，§4.1） |
| ② F1 ShortDiff 退出锚定父级回调段终点 | exit.rs:200-211（绑定）+ exit.rs:377-392（锚函数） | **即时生效**（runner 嵌套循环已消费 `exit_decision_for_nested`） |
| ③ hedge efficiency 配对考核指标 | ledger.rs:776-826 | **指标生产者完成；OPSEM dump 字段接线授权外**（runner.rs:2364-2396，§4.2） |
| ④ 单测（方向对称/锚定/指标） | exit.rs:562-728、ledger.rs:1447-1490 | 6 个新测试全绿 |
| ⑤ `cargo test --release --lib` | — | **1743 passed / 0 failed**（基线 1737 + 新增 6，零变红，§5） |

bit-exact 边界：决策层（typed_ledger/TW/sep_legs/分类器/塔）**逐字节不变**（本工位未触碰）；`exec::close_pred` 四析取（exec.rs:242-244）**bit-exact 保留**——第五析取与 F1 锚在策略层（exit.rs）以显式 `∨` 并入，不洗进 Lean 锚结构。改动属授权内的**设计性改变**，逐条登记如下。

---

## 1. ① closePred 第五析取：MFE 回撤止盈

### 1.1 机制

- **直接原因锚**：审计 §2.2/§3.2——四析取无止盈项，234 笔 Short 的 MFE 浮盈 60,141 点捕获率 −0.274。
- **语义**：浮盈峰值回撤 ≥ X·峰值 ⟹ 触发止盈（「先锁定」）。**方向完全镜像**：
  Long 峰值=持仓期 max `bar.high`、回撤探针 `bar.low`；Short 峰值=min `bar.low`、探针 `bar.high`
  （触及语义与 spec:52 止损触及同族）。`mfe ≤ 0`（未曾浮盈）恒 false——止盈不退化为第二止损。
- **代码锚**：
  - `MfeState` 侧车状态（exit.rs:317）——**不进 `HeldVoice`**：nautilus/strategy.rs:382/:430 有
    `HeldVoice` 结构字面量构造（非授权文件），加字段会破编译；侧车与 held 同构按 depth 槽管理。
  - `mfe_init`（exit.rs:325）/ `mfe_advance`（exit.rs:333）/ `mfe_take_profit_hit`（exit.rs:350，
    i128 中间域整数比较，与 `RiskPolicy::eta_star` 有理定点同手法，无浮点非确定性）。
  - `exit_decision_for_nested_mfe`（exit.rs:156）：`X' = closePred ∨ F1锚 ∨ TakeProfit`
    （exit.rs:231 单一析取点）。既有公开函数签名零改；`mfe=None`/默认路径 ⟹ 行为逐字节
    不变（回归锁见 §3 测试 W3-①c）。

### 1.2 参数 X 来源（090 如实声明）

**设计文档无数值锚**（审计 §6 F1-F5 通篇无回撤比例；多空对冲.pdf p9-10 只有考核纪律）。
故 X 取**算子声明式政策参数** `MFE_TP_RETRACE_NUM/MFE_TP_RETRACE_DEN = 1/2`（exit.rs:306-308）——
先例 = k的条件.pdf 不可识别性定理2 的 κ（κ 不是价格可推的值，由 operator 声明；本 repo
`RiskPolicy` 同先例）。**非回测寻优产物**（v3：不做参数寻优）。1/2 = 「浮盈峰值回撤一半即锁定」，
是「先锁定」纪律的最小对称声明（Long/Short 同一 X，零方向偏置）。若设计侧后续给出锚定值，
改这一对常量即可，机制不变。

### 1.3 与 Lean 锚的关系（诚实标注）

`Origin.SubVoiceOpenClose.closePred`（line 552-562）四析取**不动**（exec.rs 未触碰）。
第五析取是 Rust 策略层扩展，不冒充 Lean 锚——与 `TwEvent::Realize`（Rust 先行构造子，
strategy/ledger.rs:542-545 同款诚实标注）同一先例。

---

## 2. ② F1：ShortDiff 退出锚定父级回调段终点

### 2.1 角色识别

depth>0 ⟺ ShortDiff 对冲腿——这是**生产门不变量**：子声部门仅对 `role.v == Vertical::ShortDiff`
开 depth>0（strategy/mod.rs:711-727），非假设而是判据。FollowParent 级联腿在当前单脊柱赋格树
不产 depth>0（若未来引入，绑定须改读快照 V 轴——登记为边界）。

### 2.2 绑定语义（exit.rs:200-211）

depth>0 腿的**反向确认析取项（χ^{σ_p}）由段终点锚替代**——审计 §3.4：买侧 BSP 确认落地时
回调已反转、锁定失效，「先锁定下跌，再保留反弹」（多空对冲.pdf p10）要求退出 ≤ 段终点。
Stop/RiskClose/¬ParentValid 三项保留（安全项不角色化）。depth=0 根/独立声部通用反向项
**逐字节不变**（回归锁 §3 测试 W3-②b 末段）。

### 2.3 锚定义（`short_diff_anchor_hit`，exit.rs:377-392）

- **父多子空**（Short 腿）：回调段完成代理 = 价格**触及中枢上沿 ZG**（`bar.low <= zg`），
  武装条件 `entry > zg`（入场在中枢上方；三卖类入场已在中枢下 ⟹ 解除武装回退通用谓词）。
- **父空子多**（Long 腿，镜像）：反弹触及中枢下沿 ZD（`bar.high >= zd`），武装 `entry < zd`。
- **设计决定登记**：真「父级回调段终点」需父容器身份持久化（D1 零字段案下快照未携带），
  本锚取入场快照 `stop_in.center` 的中枢对向沿作 v0 一阶代理——零新增参数、方向对称。
  验收口径 = 机制一致性（退出时点 ≤ 段终点），**非 pnl 转正**（v3 不回测验证策略）。
- **生效状态**：runner 嵌套循环（runner.rs:3376）已消费 `exit_decision_for_nested`，
  本项**即时生效**（无需接线）。mod.rs close 桶路径（mod.rs:699-708 常规反向关闭）授权外——
  F1「锚定压过反向确认」的完整语义需该路径同源改造，登记 §4.1。

---

## 3. ③ hedge efficiency 配对考核指标（ledger.rs:776-826）

`HedgePairOutcome { parent_window_pnl, hedge_leg_gross_pnl, hedge_leg_cost }`：

- `prevented_loss()`（ledger.rs:789）：父仓窗口 pnl<0 时 `min(−parent, max(0, hedge_gross))`
  （被对冲抵消的部分，以父亏为上界不记超额对冲盈利）；父仓顺风 ⟹ 0。
- `hedge_cost()`（ledger.rs:799）：`max(0, −hedge_gross) + cost`——父仓顺风时对冲腿的必然
  毛亏 + 费用（p9「零减成本」的减项）。
- `hedge_efficiency_bp()`（ledger.rs:810）：`prevented·10000/cost` 整数截断（账本整数域
  bit-exact 纪律，无浮点）；`cost==0` ⟹ **`None`**（比率无定义不伪造，090）。
- `pair_net_pnl()`（ledger.rs:821）：`parent + gross − cost`（p9 `Π^overlay` 窗口离散版，
  诊断读出非 alpha 声明——p9「voice-capture score > 0 ⇏ NAV alpha > 0」「两目标不能混用」）。

**口径声明**：全部字段与 trades.jsonl 毛腿费前口径对齐（审计 §0）。指标为 L0 算术配对核算，
**不含任何统计量**（无期望/显著性/Sharpe——p10 的 E[Π^overlay]/std 类指标属 L2/L3，本工位
不实装，v3 硬禁令）。

---

## 4. 授权边界与待接线项（090 照实登记）

### 4.1 授权外未接线（声明=能力：以下机制已实装+已单测，但生产路径尚未消费）

1. **MFE 第五析取生产接线**：runner.rs 两个退出循环（runner.rs:3043、:3376）与 nautilus
   （nautilus/strategy.rs）需逐 bar 调 `mfe_advance` 并改调 `exit_decision_for_nested_mfe`。
   runner.rs/nautilus 非授权文件 ⟹ 回测/生产路径当前 **armed-but-inert**（MFE 永 false，
   行为逐字节不变）。
2. **F1 完整语义**：mod.rs close 桶路径（mod.rs:699-708）仍按通用反向候选关 ShortDiff 腿；
   「锚定压过反向确认」全链路需 mod.rs 同源改造（非授权）。
3. **OPSEM dump 字段**：`HedgePairOutcome` 的落盘（runner.rs:2364-2396 dump 时逐 ShortDiff
   腿配对父仓同窗口 realized+unrealized 填充）非授权。配对字段落盘前，毛腿 pnl 孤立归因
   不得用于对冲腿考核（报告层纪律，多空对冲.pdf p9）。

### 4.2 未做的（审计 F3/F5，非本任务范围）

F3（SameReverse|FollowParent 角色错配准入定案）与 F5（止损空转排查）审计自定「需设计侧
裁定/不定案」，本工位未触碰。

---

## 5. 验证（cargo test --release --lib）

基线（改动前）：`1737 passed; 0 failed; 128 ignored`。
改动后：**`1743 passed; 0 failed; 128 ignored; finished in 0.95s`**——零变红，新增 6 全绿：

```
test theta_v0::strategy::exit::tests::mfe_sidecar_advance_direction_mirror ... ok
test theta_v0::strategy::exit::tests::mfe_take_profit_direction_symmetric ... ok
test theta_v0::strategy::exit::tests::mfe_tp_fifth_disjunct_via_wrapper ... ok
test theta_v0::strategy::exit::tests::short_diff_anchor_hit_semantics ... ok
test theta_v0::strategy::exit::tests::f1_anchor_replaces_reverse_for_shortdiff ... ok
test theta_v0::strategy::ledger::tests::hedge_pair_efficiency_arithmetic ... ok
```

单测覆盖（任务④）：
- **止盈方向对称**（W3-①a/①b/①c）：Long/Short 镜像几何产出镜像判定；X=1/2 边界零截断
  精确（回撤恰=mfe/2 触发、−1 不触发）；mfe≤0 恒 false；`mfe=None` 回归锁=默认路径逐字节不变。
- **ShortDiff 锚定语义**（W3-②a/②b）：武装/解除武装、depth=0 恒 false、镜像（父空子多 ZD）；
  角色绑定压过通用反向项（depth>0 有反向信号不退出、锚触无反向信号退出；depth=0 通用反向
  项回归锁）。
- **hedge efficiency 正确性**（ledger）：被防损失上界封顶、成本=毛亏+费用、cost=0⟹None、
  bp 截断、pair_net 口径。

---

## 附：改动 diff（仅本工位两文件；voice.rs 零改）

```diff
diff --git a/rust/src/theta_v0/strategy/exit.rs b/rust/src/theta_v0/strategy/exit.rs
index 3ef74e84a1..2a765ad522 100644
--- a/rust/src/theta_v0/strategy/exit.rs
+++ b/rust/src/theta_v0/strategy/exit.rs
@@ -16,6 +16,35 @@
 //! ## 契约锚 `Origin.SubVoiceOpenClose.closePred`（line 552-562）
 //!
 //! X_{v,t} = ¬ParentValid ∨ χ^{σ_p}（反向信号）∨ Stop ∨ RiskClose。
+//!
+//! ## W3 Short 机制缝隙修复（short-side-loss-attribution-20260719.md F1/F4/因子 C）
+//!
+//! 本模块在**不改动** `exec::close_pred`（Lean 锚四析取，bit-exact 保留）的前提下，于策略层
+//! （本文件）实装两个角色/方向对称的退出机制扩展：
+//!
+//! 1. **第五析取：MFE 回撤止盈**（因子 C②「closePred 无止盈项」——234 笔 Short 的 MFE 浮盈
+//!    60,141 点全漏的直接原因，审计 §2.2/§3.2）。由 [`MfeState`] 侧车状态 +
+//!    [`mfe_take_profit_hit`] 承载，经 [`exit_decision_for_nested_mfe`] 并入析取：
+//!    `X' = X ∨ TakeProfit`。方向完全镜像（Long 峰值=持仓期最高 high、回撤探针=bar.low；
+//!    Short 峰值=最低 low、探针=bar.high）。**诚实标注**：Lean `closePred` 仍四析取，本项是
+//!    Rust 策略层扩展（非 Lean 锚）；回撤比例 X 为**算子声明式政策参数**（无设计文档数值锚，
+//!    先例 = k的条件.pdf 不可识别性定理2 的 κ——不由价格识别，由 operator 声明；见
+//!    [`MFE_TP_RETRACE_NUM`]/[`MFE_TP_RETRACE_DEN`]），非回测寻优产物（v3 禁令）。
+//! 2. **F1 角色-退出谓词绑定**（审计 §6-F1）：depth>0 声部 = ShortDiff 对冲腿（生产子声部门
+//!    仅对 `role.v==ShortDiff` 开 depth>0，strategy/mod.rs:711-727——depth>0 ⟺ ShortDiff 是
+//!    现行单脊柱赋格树的门不变量），其**反向确认析取项由「父级回调段终点锚」替代**：
+//!    [`short_diff_anchor_hit`]（父多子空：回调触及中枢上沿 ZG；父空子多镜像：反弹触及中枢
+//!    下沿 ZD）。锚 = 入场快照 `stop_in.center`（D1 零字段案下快照内唯一结构载体）。**设计
+//!    决定登记**：真父级回调段终点需父容器身份持久化（快照扩维），授权外；本锚是 F1 的 v0
+//!    一阶实装（中枢对向沿首次触及 = 段完成的结构代理），方向对称、零新增参数。
+//!
+//! **接线状态（090 声明=能力）**：F1 锚定随 [`exit_decision_for_nested`] 即时生效（runner
+//! 嵌套循环 runner.rs:3376 已消费该函数）；MFE 第五析取需调用方逐 bar 推进 [`mfe_advance`]
+//! 并改调 [`exit_decision_for_nested_mfe`]——runner.rs/nautilus 两个调用方在本工位授权
+//! 之外，**MFE 止盈在回测/生产路径当前为 armed-but-inert**（既有公开函数签名零改、
+//! 默认路径 mfe=None ⟹ 行为逐字节不变，回归锁 C4 覆盖）。mod.rs close 桶路径
+//! （recognize_nested 的常规反向关闭，mod.rs:699-708）同样在授权外——F1 的「锚定压过
+//! 反向确认」完整语义需该路径同源改造，登记为授权外待接线项。
 
 use super::exec::{close_pred, reverse_signal, stop_hit, CloseTriggers, FillSide};
 use super::risk::{global_risk_close, risk_mode, structural_stop, RiskModeInput, StopSide};
@@ -109,6 +138,49 @@ pub fn exit_decision_for_nested(
     groups: &[Vec<&VoiceDecision>],
     equity_now: f64,
     parent_invalid: bool,
+) -> Option<VoiceDecision> {
+    // 无 MFE 侧车状态 ⟹ 第五析取 take_profit=false（默认路径与历史行为逐字节相同）。
+    exit_decision_impl(hv, depth, bar, i, groups, equity_now, parent_invalid, false)
+}
+
+/// **MFE 回撤止盈版退出决策生成器（W3 第五析取，因子 C② 修复实装点）**。
+///
+/// 与 [`exit_decision_for_nested`] 的唯一差异：`mfe` 侧车状态（逐 bar 由调用方经
+/// [`mfe_advance`] 推进）非 None 时，**第五析取项 TakeProfit**（[`mfe_take_profit_hit`]，
+/// 方向镜像）并入关闭谓词：`X' = ¬ParentValid ∨ χ^{σ_p}（或 F1 段终点锚）∨ Stop ∨
+/// RiskClose ∨ TakeProfit`。`mfe=None` ⟹ 与 [`exit_decision_for_nested`] 逐字节同行为
+/// （回归锁：默认路径不变）。
+///
+/// **接线状态**：runner.rs/nautilus 两个调用方在 W3 工位授权外，生产路径尚未改调本函数
+/// （armed-but-inert，见模块头「W3 Short 机制缝隙修复」接线声明）。
+pub fn exit_decision_for_nested_mfe(
+    hv: &HeldVoice,
+    depth: usize,
+    bar: &Bar,
+    i: usize,
+    groups: &[Vec<&VoiceDecision>],
+    equity_now: f64,
+    parent_invalid: bool,
+    mfe: Option<&MfeState>,
+) -> Option<VoiceDecision> {
+    let take_profit = mfe
+        .map(|m| mfe_take_profit_hit(m, hv.side, hv.decision.entry, bar))
+        .unwrap_or(false);
+    exit_decision_impl(hv, depth, bar, i, groups, equity_now, parent_invalid, take_profit)
+}
+
+/// 退出决策实现（[`exit_decision_for_nested`] / [`exit_decision_for_nested_mfe`] 单一来源）。
+///
+/// `take_profit` = 第五析取项的当下读出（W3 MFE 回撤止盈；false ⟹ 四析取原语义）。
+fn exit_decision_impl(
+    hv: &HeldVoice,
+    depth: usize,
+    bar: &Bar,
+    i: usize,
+    groups: &[Vec<&VoiceDecision>],
+    equity_now: f64,
+    parent_invalid: bool,
+    take_profit: bool,
 ) -> Option<VoiceDecision> {
     // Stop（line 559）：当前 bar 触及止损价 hv.stop。平仓方向 = 持仓反向（平多=Sell，平空=Buy）。
     let exit_side = match hv.side {
@@ -120,11 +192,23 @@ pub fn exit_decision_for_nested(
 
     // 反向信号 χ^{σ_p}（line 596-601）：当前 bar 的开仓 decisions 含反向方向根决策 ⟹ 触发。
     // 用 reverse_signal 判每个当前 bar 决策的 bsp 是否与持仓反向（持多遇卖 / 持空遇买）。
-    let reverse = groups
+    let reverse_raw = groups
         .get(i)
         .map(|ds| ds.iter().any(|d| reverse_signal(hv.side, &d.bsp)))
         .unwrap_or(false);
 
+    // ★F1 角色-退出谓词绑定（W3，short-side-loss-attribution-20260719.md §6-F1）：
+    // depth>0 ⟺ ShortDiff 对冲腿（生产子声部门仅对 role.v==ShortDiff 开 depth>0，
+    // strategy/mod.rs:711-727 门不变量）——其反向确认析取项由「父级回调段终点锚」
+    // （short_diff_anchor_hit，多空对冲.pdf p10「先锁定下跌，再保留反弹」）**替代**；
+    // 反向 BSP 确认落地时回调已反转、锁定失效（审计 §3.4），故对冲腿不再走通用 χ^{σ_p} 项。
+    // depth=0 根/独立声部保持通用反向项（逐字节不变）。
+    let (reverse, seg_anchor) = if depth > 0 {
+        (false, short_diff_anchor_hit(hv, bar))
+    } else {
+        (reverse_raw, false)
+    };
+
     // RiskClose（line 561）：GlobalRiskClose（μ_t ∈ {Insolvent, Liquidation}）。
     // v0 可计算 Insolvent（equity≤0）——maint_margin/buffer/liq_flag 账户层输入未建模，
     // 用占位（maint_margin=0, buffer=0, liq_flag=false）使 μ_t 退化到 equity≤0 ⟹ Insolvent 子集。
@@ -143,7 +227,8 @@ pub fn exit_decision_for_nested(
         stop,
         risk_close,
     };
-    if !close_pred(&triggers) {
+    // X' = closePred（Lean 锚四析取，exec.rs bit-exact 不动）∨ F1 段终点锚 ∨ W3 第五析取止盈。
+    if !close_pred(&triggers) && !seg_anchor && !take_profit {
         return None; // X=false：不关闭，持仓延续
     }
 
@@ -205,6 +290,104 @@ pub fn cascade_exit_decisions(
     out
 }
 
+// ──────────────────────────────────────────────────────────────────────────
+//  W3 Short 机制缝隙修复（short-side-loss-attribution-20260719.md）：
+//  ① MFE 回撤止盈（第五析取）② F1 ShortDiff 父级回调段终点锚
+// ──────────────────────────────────────────────────────────────────────────
+
+/// MFE 回撤止盈的回撤比例 X = [`MFE_TP_RETRACE_NUM`] / [`MFE_TP_RETRACE_DEN`]（默认 1/2）。
+///
+/// **参数来源诚实声明（090 声明=能力）**：设计文档（short-side-loss-attribution-20260719.md
+/// §6 F1-F5）未给数值锚；多空对冲.pdf p9-10 只给考核纪律（hedge efficiency），不给回撤比例。
+/// 故 X 是**算子声明式政策参数**——先例 = k的条件.pdf 不可识别性定理2 的 κ（κ 不是价格可推的
+/// 值，由 operator 声明；trading/ledger.rs RiskPolicy 同先例）。**非回测寻优产物**（v3 硬禁令：
+/// 不做参数寻优、不以回测验证策略）。默认 1/2 = 浮盈峰值回撤一半即锁定，是「先锁定」纪律的
+/// 最小对称声明（Long/Short 同一 X，方向不偏置）。
+pub const MFE_TP_RETRACE_NUM: i64 = 1;
+/// 见 [`MFE_TP_RETRACE_NUM`]（分母 >0，构造期常量保证）。
+pub const MFE_TP_RETRACE_DEN: i64 = 2;
+
+/// MFE 侧车状态（W3 第五析取）：持仓期**方向调整最有利极值价**。
+///
+/// 不进 `HeldVoice`（nautilus/strategy.rs:382/:430 有结构字面量构造，加字段会破非授权文件；
+/// 侧车数组与 held 同构索引，调用方按 depth 槽管理）：
+/// - Long：`peak` = 入场以来最高 `bar.high`（浮盈峰值 = peak − entry）。
+/// - Short：`peak` = 入场以来最低 `bar.low`（浮盈峰值 = entry − peak，镜像）。
+#[derive(Debug, Clone, Copy, PartialEq, Eq)]
+pub struct MfeState {
+    /// 方向调整最有利极值价（Long=max high，Short=min low）。
+    pub peak: Tick,
+}
+
+/// 开仓时初始化 MFE 侧车（峰值=入场价，MFE=0 ⟹ 止盈项不触发，armed-but-inert 起点）。
+///
+/// `Flat` ⟹ `None`（无持仓方向，与 [`record_held_voice`] 同防御）。
+pub fn mfe_init(d: &VoiceDecision) -> Option<MfeState> {
+    match voice_side(d.root_side, d.depth) {
+        VoiceSide::Flat => None,
+        _ => Some(MfeState { peak: d.entry }),
+    }
+}
+
+/// 逐 bar 推进 MFE 峰值（方向镜像）：Long 取 `bar.high` 上移峰值；Short 取 `bar.low` 下移峰值。
+pub fn mfe_advance(m: &mut MfeState, side: VoiceSide, bar: &Bar) {
+    match side {
+        VoiceSide::Long => m.peak = m.peak.max(bar.high),
+        VoiceSide::Short => m.peak = m.peak.min(bar.low),
+        VoiceSide::Flat => {}
+    }
+}
+
+/// **第五析取 TakeProfit：MFE 回撤止盈判定（方向完全镜像，spec:52 触及语义同族）**。
+///
+/// - Long：浮盈峰值 `mfe = peak − entry`；当前 bar 的回撤探针 `bar.low`。
+///   触发 ⟺ `(peak − low)·DEN ≥ mfe·NUM`（自峰值回撤 ≥ X·浮盈峰值）。
+/// - Short（镜像）：`mfe = entry − peak`；探针 `bar.high`。触发 ⟺ `(high − peak)·DEN ≥ mfe·NUM`。
+///
+/// `mfe ≤ 0`（未曾浮盈）⟹ 恒 false（止盈项只锁定**已有**浮盈，不退化为第二止损）。
+/// 整数域比较（i128 中间域防溢出，与 RiskPolicy 有理定点同手法）——X=num/den 有理，
+/// 无浮点非确定性。
+pub fn mfe_take_profit_hit(m: &MfeState, side: VoiceSide, entry: Tick, bar: &Bar) -> bool {
+    let (mfe, retrace) = match side {
+        VoiceSide::Long => (m.peak - entry, m.peak - bar.low),
+        VoiceSide::Short => (entry - m.peak, bar.high - m.peak),
+        VoiceSide::Flat => return false,
+    };
+    if mfe <= 0 {
+        return false;
+    }
+    (retrace as i128) * (MFE_TP_RETRACE_DEN as i128) >= (mfe as i128) * (MFE_TP_RETRACE_NUM as i128)
+}
+
+/// **F1 ShortDiff 退出锚：父级回调段终点（段完成结构代理），方向镜像**。
+///
+/// 只对 depth>0 声部（= ShortDiff 对冲腿，strategy/mod.rs:711-727 门不变量：生产子声部门
+/// 仅对 `role.v==ShortDiff` 开 depth>0）有意义；depth=0 ⟹ 恒 false（根走通用谓词）。
+///
+/// - **父多子空**（`hv.side == Short`，对冲父多头的回调段）：回调段完成的结构代理 =
+///   价格**触及中枢上沿 ZG**（`bar.low <= zg`，触及语义与 spec:52 同族）。武装条件
+///   `entry > zg`（入场在中枢上方，回调有触及 ZG 的空间；入场已在 ZG 下方则锚无意义，
+///   解除武装回退通用谓词——三卖类入场快照的诚实边界）。
+/// - **父空子多**（`hv.side == Long`，镜像）：反弹段完成 = 触及中枢下沿 ZD
+///   （`bar.high >= zd`），武装条件 `entry < zd`。
+///
+/// **设计决定登记（090）**：真「父级回调段终点」需父容器身份持久化（D1 零字段案下快照
+/// 未携带），本锚取入场快照 `stop_in.center` 的中枢对向沿作 v0 一阶代理——零新增参数、
+/// 方向对称；验收口径 = 机制一致性（对冲腿退出时点 ≤ 段终点），非 pnl 转正（v3 不回测验证）。
+pub fn short_diff_anchor_hit(hv: &HeldVoice, bar: &Bar) -> bool {
+    if hv.decision.depth == 0 {
+        return false; // 根声部无父级回调段——通用谓词域
+    }
+    let c = &hv.decision.stop_in.center;
+    match hv.side {
+        // 父多子空：回调触及中枢上沿 ZG（武装：入场在 ZG 上方）。
+        VoiceSide::Short => hv.decision.entry > c.zg && bar.low <= c.zg,
+        // 父空子多（镜像）：反弹触及中枢下沿 ZD（武装：入场在 ZD 下方）。
+        VoiceSide::Long => hv.decision.entry < c.zd && bar.high >= c.zd,
+        VoiceSide::Flat => false,
+    }
+}
+
 #[cfg(test)]
 mod tests {
     use super::*;
@@ -341,4 +524,191 @@ mod tests {
         assert_eq!(out.len(), 1);
         assert_eq!(out[0].depth, 0);
     }
+
+    // ──────────────────────────────────────────────────────────────────────
+    //  W3 Short 机制缝隙修复（F1 段终点锚 + 第五析取 MFE 回撤止盈）
+    // ──────────────────────────────────────────────────────────────────────
+
+    /// 自定义持仓声部（W3 测试）：显式 side/depth/entry/center/stop。
+    /// root_side 与 (side, depth) 保持 voice_side 代数一致（偶深=side，奇深=side.flip()）。
+    fn held_w3(side: VoiceSide, depth: u32, entry: Tick, zd: Tick, zg: Tick, stop: Tick) -> HeldVoice {
+        let root_side = if depth % 2 == 0 { side } else { side.flip() };
+        HeldVoice {
+            side,
+            stop,
+            decision: VoiceDecision {
+                depth,
+                root_side,
+                exit: false,
+                enter_ok: true,
+                bsp: BspBits { sell1: true, ..Default::default() },
+                signal_index: 0,
+                stop_in: StopInput {
+                    pivot_low: zd,
+                    pivot_high: zg,
+                    center: Center { zd, zg, dd: zd - 10, gg: zg + 10, start_index: 0, end_index: 5 },
+                },
+                entry,
+                cost_per_unit: 0.0,
+                level: 1,
+            },
+            exit_pending: false,
+        }
+    }
+
+    /// W3-①a（MFE 侧车方向镜像）：Long 峰值=逐 bar max high；Short 峰值=逐 bar min low（镜像）；
+    /// Flat ⟹ mfe_init=None；峰值初始化=入场价（MFE=0，止盈项起点不触发）。
+    #[test]
+    fn mfe_sidecar_advance_direction_mirror() {
+        let d_long = held_w3(VoiceSide::Long, 0, 1000, 950, 1080, 950).decision;
+        let d_short = held_w3(VoiceSide::Short, 1, 1000, 950, 1080, 1100).decision;
+        let mut ml = mfe_init(&d_long).expect("Long 初始化");
+        let mut ms = mfe_init(&d_short).expect("Short 初始化");
+        assert_eq!(ml.peak, 1000);
+        assert_eq!(ms.peak, 1000);
+        // 同一根 bar 序列，两侧峰值各自朝有利方向走（镜像）。
+        let bars = [
+            bar_at(1, 1000, 1050, 980, 1020),
+            bar_at(2, 1020, 1030, 960, 1010), // high 回落 / low 更深
+            bar_at(3, 1010, 1060, 990, 1040),
+        ];
+        for b in &bars {
+            mfe_advance(&mut ml, VoiceSide::Long, b);
+            mfe_advance(&mut ms, VoiceSide::Short, b);
+        }
+        assert_eq!(ml.peak, 1060, "Long 峰值=max high（1060）");
+        assert_eq!(ms.peak, 960, "Short 峰值=min low（960，镜像）");
+        // Flat 方向无 MFE 状态（防御，与 record_held_voice 同）。
+        let mut d_flat = d_long;
+        d_flat.root_side = VoiceSide::Flat;
+        assert!(mfe_init(&d_flat).is_none(), "Flat ⟹ None");
+    }
+
+    /// W3-①b（止盈判定方向对称 + 边界精确）：同一 (entry, 峰值, 探针) 几何在 Long/Short
+    /// 镜像下产出镜像结果；回撤比例 X=NUM/DEN=1/2 的边界零截断精确（整数域）；
+    /// mfe≤0（未曾浮盈）恒 false（止盈不退化为第二止损）。
+    #[test]
+    fn mfe_take_profit_direction_symmetric() {
+        // Long：entry=1000，峰值 high=1200（mfe=200）。bar.low=1080 ⟹ 回撤 120 ≥ 100 ⟹ 触发。
+        let m_long = MfeState { peak: 1200 };
+        let bar_l_fire = bar_at(5, 1190, 1195, 1080, 1090);
+        assert!(mfe_take_profit_hit(&m_long, VoiceSide::Long, 1000, &bar_l_fire));
+        // Short 镜像：entry=1000，峰值 low=800（mfe=200）。bar.high=920 ⟹ 回撤 120 ≥ 100 ⟹ 触发。
+        let m_short = MfeState { peak: 800 };
+        let bar_s_fire = bar_at(5, 810, 920, 805, 915);
+        assert!(mfe_take_profit_hit(&m_short, VoiceSide::Short, 1000, &bar_s_fire));
+        // 边界精确：回撤恰 = mfe/2（100）⟹ 触发（≥）；回撤 99 ⟹ 不触发。
+        let bar_l_edge = bar_at(5, 1190, 1195, 1100, 1110); // 回撤 100
+        assert!(mfe_take_profit_hit(&m_long, VoiceSide::Long, 1000, &bar_l_edge), "边界 ≥ 触发");
+        let bar_l_below = bar_at(5, 1190, 1195, 1101, 1110); // 回撤 99
+        assert!(!mfe_take_profit_hit(&m_long, VoiceSide::Long, 1000, &bar_l_below));
+        let bar_s_edge = bar_at(5, 810, 900, 805, 890); // 回撤 100
+        assert!(mfe_take_profit_hit(&m_short, VoiceSide::Short, 1000, &bar_s_edge), "镜像边界 ≥ 触发");
+        let bar_s_below = bar_at(5, 810, 899, 805, 890); // 回撤 99
+        assert!(!mfe_take_profit_hit(&m_short, VoiceSide::Short, 1000, &bar_s_below));
+        // mfe=0（峰值=入场价）/ mfe<0（峰值反向）⟹ 恒 false。
+        let m_zero = MfeState { peak: 1000 };
+        assert!(!mfe_take_profit_hit(&m_zero, VoiceSide::Long, 1000, &bar_l_fire));
+        assert!(!mfe_take_profit_hit(&m_zero, VoiceSide::Short, 1000, &bar_s_fire));
+        let m_neg = MfeState { peak: 900 }; // Long 峰值低于入场（从未浮盈）
+        assert!(!mfe_take_profit_hit(&m_neg, VoiceSide::Long, 1000, &bar_l_fire));
+        // Flat ⟹ false。
+        assert!(!mfe_take_profit_hit(&m_long, VoiceSide::Flat, 1000, &bar_l_fire));
+    }
+
+    /// W3-①c（第五析取经 wrapper 并入 + 默认路径回归锁）：MFE 状态触发时
+    /// `exit_decision_for_nested_mfe` 产退出决策而四析取全假的基线返回 None；
+    /// `mfe=None` ⟹ 与 `exit_decision_for_nested` 逐字节同行为。
+    #[test]
+    fn mfe_tp_fifth_disjunct_via_wrapper() {
+        // Long 根（depth=0），entry=1000，stop=950；当前 bar 不触止损/无反向/权益正。
+        let h = held_w3(VoiceSide::Long, 0, 1000, 950, 1080, 950);
+        let bar = bar_at(7, 1090, 1095, 1080, 1085); // low 1080 > 950 不触止损
+        let groups: Vec<Vec<&VoiceDecision>> = vec![];
+        let equity = 1_000_000.0;
+        // 基线（无 MFE）：四析取全假 ⟹ None。
+        assert!(exit_decision_for_nested(&h, 0, &bar, 7, &groups, equity, false).is_none());
+        // MFE 峰值 1200（mfe=200），bar.low=1080 ⟹ 回撤 120 ≥ 100 ⟹ 第五析取触发。
+        let m = MfeState { peak: 1200 };
+        let d = exit_decision_for_nested_mfe(&h, 0, &bar, 7, &groups, equity, false, Some(&m))
+            .expect("第五析取 TakeProfit 单项 ⟹ X'=true 产退出决策");
+        assert!(d.exit && !d.enter_ok);
+        assert_eq!(d.signal_index, 7);
+        // mfe=None ⟹ 与基线同行为（回归锁）。
+        assert!(
+            exit_decision_for_nested_mfe(&h, 0, &bar, 7, &groups, equity, false, None).is_none(),
+            "mfe=None ⟹ 四析取原语义"
+        );
+        // Short 镜像（depth=1 子声部，锚解除武装 entry=1000 < zg=1080 ⟹ 只剩第五析取）：
+        let hs = held_w3(VoiceSide::Short, 1, 1000, 950, 1080, 1100);
+        let bar_s = bar_at(7, 910, 920, 905, 915); // high 920 < 1100 不触止损
+        let ms = MfeState { peak: 800 }; // mfe=200，回撤 120 ≥ 100 ⟹ 触发
+        let ds = exit_decision_for_nested_mfe(&hs, 1, &bar_s, 7, &groups, equity, false, Some(&ms))
+            .expect("Short 镜像：第五析取触发");
+        assert!(ds.exit && ds.depth == 1);
+        assert!(
+            exit_decision_for_nested_mfe(&hs, 1, &bar_s, 7, &groups, equity, false, None).is_none(),
+            "Short 镜像：mfe=None ⟹ None"
+        );
+    }
+
+    /// W3-②a（F1 锚语义）：父多子空 ShortDiff 腿（depth=1, Short, entry>zg）在
+    /// `bar.low ≤ zg`（回调触及中枢上沿）时锚触发；武装条件不满足（entry≤zg）⟹ 解除武装；
+    /// depth=0 ⟹ 恒 false；父空子多 Long 腿镜像（entry<zd，`bar.high ≥ zd` 触发）。
+    #[test]
+    fn short_diff_anchor_hit_semantics() {
+        // 父多子空：entry=1050 > zg=1000，回调 bar.low=990 ≤ 1000 ⟹ 触发。
+        let h_short = held_w3(VoiceSide::Short, 1, 1050, 900, 1000, 1100);
+        let bar_touch = bar_at(3, 1020, 1030, 990, 995);
+        assert!(short_diff_anchor_hit(&h_short, &bar_touch), "回调触及 ZG ⟹ 段终点锚触发");
+        let bar_above = bar_at(3, 1020, 1030, 1001, 1010); // low 1001 > zg 未触及
+        assert!(!short_diff_anchor_hit(&h_short, &bar_above));
+        // 解除武装：entry=980 ≤ zg=1000（入场已在中枢下方）⟹ 锚不触。
+        let h_disarmed = held_w3(VoiceSide::Short, 1, 980, 900, 1000, 1100);
+        assert!(!short_diff_anchor_hit(&h_disarmed, &bar_touch), "entry≤zg ⟹ 解除武装");
+        // depth=0 根 ⟹ 恒 false（通用谓词域）。
+        let h_root = held_w3(VoiceSide::Short, 0, 1050, 900, 1000, 1100);
+        assert!(!short_diff_anchor_hit(&h_root, &bar_touch), "depth=0 无锚");
+        // 父空子多镜像：entry=950 < zd=1000，反弹 bar.high=1010 ≥ 1000 ⟹ 触发。
+        let h_long = held_w3(VoiceSide::Long, 1, 950, 1000, 1100, 900);
+        let bar_rebound = bar_at(3, 980, 1010, 970, 1005);
+        assert!(short_diff_anchor_hit(&h_long, &bar_rebound), "镜像：反弹触及 ZD ⟹ 触发");
+        let bar_below = bar_at(3, 980, 999, 970, 990); // high 999 < zd 未触及
+        assert!(!short_diff_anchor_hit(&h_long, &bar_below));
+        let h_long_disarmed = held_w3(VoiceSide::Long, 1, 1020, 1000, 1100, 900);
+        assert!(!short_diff_anchor_hit(&h_long_disarmed, &bar_rebound), "镜像：entry≥zd ⟹ 解除武装");
+    }
+
+    /// W3-②b（F1 角色绑定压过通用反向项）：depth>0 ShortDiff 腿在反向信号出现但锚未触时
+    /// **不退出**（反向确认项已被段终点锚替代）；锚触时无反向信号也退出。depth=0 根的
+    /// 通用反向项逐字节不变（回归锁）。
+    #[test]
+    fn f1_anchor_replaces_reverse_for_shortdiff() {
+        let groups_holder;
+        let h = held_w3(VoiceSide::Short, 1, 1050, 900, 1000, 1100); // entry 1050 > zg 1000
+        let buy_d = h.decision; // bsp=sell1… 需反向（买侧）决策：构造 buy1
+        let mut rev_d = buy_d;
+        rev_d.bsp = BspBits { buy1: true, ..Default::default() };
+        groups_holder = vec![Vec::new(), vec![&rev_d]];
+        let equity = 1_000_000.0;
+        // bar 未触锚（low 1010 > zg 1000）、未触止损（high 1020 < 1100）、反向信号存在：
+        // depth>0 ⟹ 反向项被替代 ⟹ None。
+        let bar_no_anchor = bar_at(1, 1015, 1020, 1010, 1012);
+        assert!(
+            exit_decision_for_nested(&h, 1, &bar_no_anchor, 1, &groups_holder, equity, false).is_none(),
+            "F1：ShortDiff 腿不再由反向 BSP 确认退出"
+        );
+        // 锚触（low 990 ≤ zg）且无反向信号的 bar ⟹ Some。
+        let groups_empty: Vec<Vec<&VoiceDecision>> = vec![Vec::new(), Vec::new()];
+        let bar_anchor = bar_at(1, 1020, 1030, 990, 995);
+        let d = exit_decision_for_nested(&h, 1, &bar_anchor, 1, &groups_empty, equity, false)
+            .expect("F1：段终点锚单项触发退出");
+        assert!(d.exit && !d.enter_ok && d.depth == 1);
+        assert_eq!(d.signal_index, 1);
+        // depth=0 回归锁：根持空遇买侧反向信号 ⟹ 仍由通用反向项退出（逐字节不变）。
+        let h_root = held_w3(VoiceSide::Short, 0, 1050, 900, 1000, 1100);
+        let d_root = exit_decision_for_nested(&h_root, 0, &bar_no_anchor, 1, &groups_holder, equity, false)
+            .expect("depth=0 根：通用反向项不变");
+        assert!(d_root.exit && d_root.depth == 0);
+    }
 }
diff --git a/rust/src/theta_v0/strategy/ledger.rs b/rust/src/theta_v0/strategy/ledger.rs
index abe5bd3f78..01d0b05e1c 100644
--- a/rust/src/theta_v0/strategy/ledger.rs
+++ b/rust/src/theta_v0/strategy/ledger.rs
@@ -743,6 +743,88 @@ pub fn forget_stage_to_ledger_view(s: &TwState) -> StageFreeTwLedgerView {
     }
 }
 
+// ──────────────────────────────────────────────────────────────────────────
+//  W3 ③ 对冲腿配对考核（short-side-loss-attribution-20260719.md §6-F2，
+//  多空对冲.pdf p9-10 纪律）
+// ──────────────────────────────────────────────────────────────────────────
+
+/// 对冲腿配对考核 outcome（多空对冲.pdf p9-10：对冲腿 standalone 预期 = 零减成本，
+/// **配对考核，非毛腿记分**）。
+///
+/// 设计锚（PDF 原文）：
+/// - p9「voice-capture score > 0 ⇏ self-financing NAV alpha > 0」「这两个目标不能混用」——
+///   故本结构是**诊断型配对读出**（不声称 NAV alpha），毛腿 pnl 孤立归因对对冲腿禁用
+///   （审计 §5.2「考核错位」行的修复）。
+/// - p10「hedge efficiency: prevented loss / hedge cost」——本结构实现该比率的整数域版本。
+/// - p9「机械对冲 standalone PnL 预期 = 零减成本」——`hedge_leg_gross_pnl` 的期望基线是 0，
+///   `hedge_leg_cost` 是「减成本」项；二者不合并为单一净额再归因（成本是考核分母的分量）。
+///
+/// 字段口径（与 trades.jsonl 毛腿费前口径对齐，审计 §0：delta×(exit−entry)，费前未乘 units）：
+/// - `parent_window_pnl`：父仓在**对冲窗口**（对冲腿开仓→平仓）内的 pnl 变化（毛腿费前，
+///   可正可负——父仓顺风时为正、逆势时为负）。
+/// - `hedge_leg_gross_pnl`：对冲腿 standalone 毛 pnl（费前，与父仓同口径）。
+/// - `hedge_leg_cost`：对冲腿成本（佣金+滑点，≥0）。
+///
+/// ★认识论 L0（formalization-validity-domain）：本结构的读出是给定输入下的**算术**（零信息
+/// 增量的配对核算），不含任何概率/统计推断（无期望、无显著性、无 Sharpe——p10 的
+/// E[Π^overlay]/std(Π^overlay) 类指标属 L2/L3 统计层，本工位不实装，v3 硬禁令）。
+/// ★接线状态（090 声明=能力）：本结构是 OPSEM dump 字段的**生产者**；dump 侧接线
+/// （runner.rs:2364-2396 opsem-dump 从 TypedTradeLedger 写出时逐 ShortDiff 腿配对父仓同窗口
+/// realized+unrealized 填充）在 W3 工位授权外（runner.rs 非授权文件），登记为待接线项——
+/// 毛腿 pnl 孤立归因在配对字段落盘前不得用于对冲腿考核（报告层纪律）。
+#[derive(Debug, Clone, Copy, PartialEq, Eq)]
+pub struct HedgePairOutcome {
+    /// 父仓对冲窗口 pnl（毛腿费前，可正可负）。
+    pub parent_window_pnl: i64,
+    /// 对冲腿 standalone 毛 pnl（费前）。
+    pub hedge_leg_gross_pnl: i64,
+    /// 对冲腿成本（佣金+滑点，≥0）。
+    pub hedge_leg_cost: i64,
+}
+
+impl HedgePairOutcome {
+    /// **被防损失 `prevented loss`**（p10 分子）：父仓窗口 pnl 为负（逆势）时，被对冲腿
+    /// 毛利抵消的部分 = `min(−parent, max(0, hedge_gross))`；父仓顺风（≥0）⟹ 0
+    /// （无损失可防）；对冲腿亦亏 ⟹ 0（无抵消发生）。
+    pub fn prevented_loss(&self) -> i64 {
+        if self.parent_window_pnl < 0 {
+            (-self.parent_window_pnl).min(self.hedge_leg_gross_pnl.max(0))
+        } else {
+            0
+        }
+    }
+
+    /// **对冲成本 `hedge cost`**（p10 分母）：对冲腿 standalone 毛亏（父仓顺风时对冲腿的
+    /// 必然代价，≥0）+ 费用成本（「零减成本」的减项，p9）。i128 中间域防御溢出。
+    pub fn hedge_cost(&self) -> i64 {
+        ((-self.hedge_leg_gross_pnl).max(0) as i128 + self.hedge_leg_cost.max(0) as i128)
+            .min(i64::MAX as i128) as i64
+    }
+
+    /// **hedge efficiency（p10）= prevented loss / hedge cost**，整数域 **bp（万分之一）**
+    /// 读出：`prevented_loss·10000 / hedge_cost`（截断除法，与账本整数域 bit-exact 纪律一致；
+    /// 不用浮点——barrier 比较同手法，RiskPolicy::eta_star 先例）。
+    ///
+    /// `hedge_cost == 0` ⟹ **`None`**（比率无定义——零成本完美对冲是退化情形，**不伪造**
+    /// 数值，090：声明=能力；调用方按「无成本」事件单列报告，不得当作 0% 或 ∞%）。
+    pub fn hedge_efficiency_bp(&self) -> Option<i64> {
+        let cost = self.hedge_cost();
+        if cost == 0 {
+            return None;
+        }
+        Some(((self.prevented_loss() as i128) * 10_000 / (cost as i128)) as i64)
+    }
+
+    /// 配对净 pnl（父仓窗口 + 对冲腿净额 = 对冲 overlay 的增量 PnL 口径，p9
+    /// `Π^overlay = H_t·ΔP − ΔC` 的窗口离散版）：`parent + gross − cost`。
+    /// 这是 p10「W#5 − W#0」窗口级分量——诊断读出，非 alpha 声明（p9 两目标不混用）。
+    pub fn pair_net_pnl(&self) -> i64 {
+        (self.parent_window_pnl as i128 + self.hedge_leg_gross_pnl as i128
+            - self.hedge_leg_cost as i128)
+            .clamp(i64::MIN as i128, i64::MAX as i128) as i64
+    }
+}
+
 #[cfg(test)]
 mod tests {
     //! ★★模块级诚实标注（codex §9.2，照实 161/no-workaround）：本模块的机制单元测试
@@ -1348,4 +1430,48 @@ mod tests {
             "财务标量保留（stage-无关分量无损）"
         );
     }
+
+    // ──────────────────────────────────────────────────────────────────────
+    //  W3 ③ 对冲腿配对考核（多空对冲.pdf p9-10：hedge efficiency = prevented loss / hedge cost）
+    // ──────────────────────────────────────────────────────────────────────
+
+    /// hedge efficiency 计算正确性（p10 比率定义 + p9「零减成本」基线 + p9 两目标不混用）。
+    ///
+    /// 逐语义分量验证（全部为给定输入下的 L0 算术，无统计推断）：
+    /// 1. 父仓逆势 + 对冲腿盈利 ⟹ prevented_loss = min(父亏, 对冲毛利)（抵消部分，封顶上界）。
+    /// 2. 父仓顺风 ⟹ prevented_loss = 0（无损失可防），hedge_cost = 对冲毛亏 + 费用。
+    /// 3. cost=0 ⟹ efficiency = None（无定义不伪造，090）。
+    /// 4. bp 截断精确（整数域，无浮点）。
+    /// 5. pair_net = parent + gross − cost（overlay 增量 PnL 窗口离散版）。
+    #[test]
+    fn hedge_pair_efficiency_arithmetic() {
+        // ① 父仓逆势：parent=-1000，对冲毛利 +600，费用 50。
+        // prevented = min(1000, 600) = 600；cost = 0 + 50 = 50；eff = 600·10000/50 = 120000bp。
+        let adverse = HedgePairOutcome { parent_window_pnl: -1000, hedge_leg_gross_pnl: 600, hedge_leg_cost: 50 };
+        assert_eq!(adverse.prevented_loss(), 600, "被防损失 = min(父亏, 对冲毛利)");
+        assert_eq!(adverse.hedge_cost(), 50, "对冲无毛亏 ⟹ 成本仅费用");
+        assert_eq!(adverse.hedge_efficiency_bp(), Some(120_000));
+        assert_eq!(adverse.pair_net_pnl(), -1000 + 600 - 50);
+        // 封顶上界：对冲毛利 +1500 > 父亏 1000 ⟹ prevented 只记父亏部分（1000）。
+        let over = HedgePairOutcome { parent_window_pnl: -1000, hedge_leg_gross_pnl: 1500, hedge_leg_cost: 50 };
+        assert_eq!(over.prevented_loss(), 1000, "被防损失以父亏为上界（不记超额对冲盈利）");
+        // 对冲腿亦亏（毛 -300）：无抵消发生 ⟹ prevented=0；cost = 300 + 50。
+        let both_down = HedgePairOutcome { parent_window_pnl: -1000, hedge_leg_gross_pnl: -300, hedge_leg_cost: 50 };
+        assert_eq!(both_down.prevented_loss(), 0, "对冲腿亦亏 ⟹ 无抵消");
+        assert_eq!(both_down.hedge_cost(), 350, "成本 = 对冲毛亏 + 费用");
+        assert_eq!(both_down.hedge_efficiency_bp(), Some(0));
+        // ② 父仓顺风：parent=+800，对冲毛亏 -400（p9「零减成本」的代价面），费用 50。
+        let favorable = HedgePairOutcome { parent_window_pnl: 800, hedge_leg_gross_pnl: -400, hedge_leg_cost: 50 };
+        assert_eq!(favorable.prevented_loss(), 0, "父仓顺风 ⟹ 无损失可防");
+        assert_eq!(favorable.hedge_cost(), 450, "对冲成本 = 毛亏 400 + 费用 50");
+        assert_eq!(favorable.hedge_efficiency_bp(), Some(0));
+        assert_eq!(favorable.pair_net_pnl(), 800 - 400 - 50);
+        // ③ cost=0 ⟹ None（完美对冲退化情形，不伪造数值）。
+        let perfect = HedgePairOutcome { parent_window_pnl: -500, hedge_leg_gross_pnl: 500, hedge_leg_cost: 0 };
+        assert_eq!(perfect.hedge_cost(), 0);
+        assert_eq!(perfect.hedge_efficiency_bp(), None, "零成本 ⟹ 比率无定义（None，不伪造）");
+        // ④ bp 截断精确：prevented=1, cost=3 ⟹ 1·10000/3 = 3333（截断，非四舍五入）。
+        let trunc = HedgePairOutcome { parent_window_pnl: -1, hedge_leg_gross_pnl: 1, hedge_leg_cost: 3 };
+        assert_eq!(trunc.hedge_efficiency_bp(), Some(3333), "整数截断除法（无浮点）");
+    }
 }
```
