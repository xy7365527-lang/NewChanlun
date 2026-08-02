# #849 勘察：仓内 hedge-mode 双仓状态——执行侧是净额还是双仓

- **基线 commit**：`c92ae8998c313e892d7c3948d5662872f6581a87`（本地 `main`，`docs(#834): ADR 0014 …`）
- 日期：2026-08-02；性质：只读勘察，零生产改动，零远端推送。
- 工位：worktree `.claude/worktrees/agent-a7751dc7041e4d558`（开工时 HEAD=`19b4015927`，与 `main`
  **零共同祖先**——祖传线，已 `git reset --hard main` 归位后才开工；祖传线读数一律作废）。
- 所有行号均在上述基线一手实读（`git show`/`grep`/`Read`），未采信任何票面或历史报告的行号。

---

## 0. 一句话结论

**执行侧仍是 NETTING 单净仓。** 仓内确实建过三套 hedge-mode `(q⁺,q⁻)` 双腿模型，但**没有一套通到
下单出口**：一套是回测内的只读旁路，一套是回测内的模拟成交簿（默认关），一套已成死码。
nautilus 侧（真实 venue 路径）从 `portfolio.net_position()` 单值读持仓，venue OMS 硬编码
`OmsType::Netting`。ADR 0014 那句「未查证，属仓外事实」的**方向**是错的——仓内有充分记载；
但它要否定的那个结论（执行侧已是双仓）**也不成立**。

**落在票面 (b) 侧**：执行侧仍是 NETTING、hedge-mode 簿只在回测侧且不进下单决策 ⟹
「各重保证金逐仓分开」在当前物理层**没有落点，暂时只能是虚拟记账**。

---

## 1. 回测侧 hedge-mode overlay 簿的现役状态

### 1.1 在不在生产路径上：**在，但只读**

`OverlayState`（`rust/src/theta_v0/strategy/overlay_state.rs:102`）由**唯一**生产 runner
`run_theta_v0_pi_overlay`（`rust/src/theta_v0/backtest/runner.rs:493`，实例化在 `:500`）驱动。
消费方三处，都是生产路径或生产探针：

| 消费方 | 位置 | 性质 |
|---|---|---|
| CLI `theta_overlay` | `rust/src/bin/theta_overlay.rs:68` | 生产跑批入口 |
| #837 探针 | `rust/src/theta_v0/backtest/issue837_probe.rs:407` | `#[cfg(test)]`，数据源=生产 runner |
| #841 探针 | `rust/src/theta_v0/backtest/issue841_probe.rs:247,381` | `#[cfg(test)]`，数据源=生产 runner |

**但它是只读旁路**，函数头逐字（`runner.rs:479-480`）：

> **净额路径 bit-exact**：overlay 是 `pi_theta_fill_loop_overlay` 内 sep_legs 的只读旁路——不改
> cash/units/trade_pnls/equity ⟹ `net_result` 与 `run_theta_v0_pi` 逐字节一致（现有臂不污染）。

### 1.2 ★关键：overlay 簿自己吐的订单也是净额

模块头（`overlay_state.rs:13-18`）三个恒等式：

- `P^sep_t = Σ_{v∈A_t} q_v σ_v e_v`（逐声部双坐标簿）
- `N_t = Net(P^sep_t) = Σ_v σ_v q_v`——**有损投影，双开 (Q,Q)↦0**
- `Order_t = N_t − N_{t−1} = ΔN`——**净额账户真实下单量**

即：**hedge-mode 的是"账本"，不是"订单"**。它逐声部保 `entry_v/exit_v/parent(v)/role(v)/pnl_v`
做归因，但对外只发净增量单。模块头自己也标了性质（`overlay_state.rs:26-27`）：
「pnl_v 是净额 PnL 的逐声部**分解**，不是独立 self-financing NAV」。

### 1.3 默认开还是关：控制字段 = env `VOICE_EXEC`

**不是 config 字段，是环境变量。** 规范登记在 `rust/src/theta_v0/env_registry.rs:76`（键名）+
`:204-210`（元数据）：

```
key: VOICE_EXEC
semantic: "W1 gate：声部独立执行臂 vs 净额臂（churn 修复）"
kind: GateKind::Behavior
default_arm: "未设/非\"1\" ⟹ 净额臂（bit-exact 回归锁）"
```

- **默认值 = 未设 ⟹ 关**（净额执行 + overlay 只读旁路）。门读取点 `runner.rs:505`
  （`voice_exec_gate()`），装配点 `:506-512`。
- **生产实际取值**：CLI `theta_overlay`（`bin/theta_overlay.rs`）**不设置**该 env，由调用者
  外部注入。仓内脚本只有一处显式带它：`scripts/check_armR_trades_digest.py:17,355`
  （M8「臂R」复现命令 `M8_WIN_FILTER=<tag> VOICE_EXEC=1 THETA_NEST_CERT_GATE=1 …`）。
  `.github/workflows/*.yml` 零出现 ⟹ **CI 不开**。
- 结论：**默认关；仅 M8 treasury 复验的臂R 显式开**。这是审计臂，不是常态生产臂。

### 1.4 VOICE_EXEC=1 时是什么：真·分腿执行，但仍在回测模拟器内

开门后 `VoiceExecBook`（`overlay_state.rs:393`）接管执行投影，与 `OverlayState` **范畴不同**
（`overlay_state.rs:288-297` 逐字）：

> ① 声部独立持仓（hedge-mode (Q⁺,Q⁻)，PDF §10.2：**对冲两腿各自存续、各自结算，不互相湮灭**）；
> ② fill 只在声部开/合事件产生（事件驱动）；③ sizing 冻结。

订单类型带声部身份（`VoiceOrder{voice, side, qty, kind}`，`overlay_state.rs:316-327`），
注释明写「≠ `types.rs Order` 净额决策层订单」。**这是仓内唯一一处真的按腿下单的东西。**

有效域边界（诚实登记）：
- 它在 `pi_theta_fill_loop_overlay`（`fill.rs:4375` 起）内驱动**模拟撮合**，不是 venue 订单出口。
- 决策层（typed_ledger/TW/sep_legs）仍由净额影子账本驱动（`fill.rs:780-781`），与净额臂逐字节一致
  ⟹ **换的是执行投影的记账，不是决策**。
- 默认关。

### 1.5 ★与 `net_target_units` 净额塌缩的关系：**并存同源，净额那条独占下单路**

这条是本票最要紧的一问。实读 `rust/src/theta_v0/strategy/coverage/step.rs`：

```
:592   let mut legs = strategy_target_legs(&work, &next_idx, base_units, config);   // 逐声部毛腿
:603   gross_zeroed = apply_gross_cap(&work, &mut legs, base_units, r);             // G7 毛帽（默认 off）
:606   let p_tilde = net_target_units(&legs);                                        // ★净额塌缩
:646   let sep_legs: Vec<SepLeg> = legs.iter().filter_map(...)                       // ★只读重打包
```

三点判定：

1. **同源**：`p_tilde`（净）与 `sep_legs`（分腿）来自**同一个 `legs` 列表**，不是两条独立计算。
   `:642-644` 注释逐字：「**只读重打包，不新计算**——`net_target_units(&legs)==p_tilde` 恒等」。
2. **并存，非覆盖**：两者是同一份腿的两个消费者。`p_tilde` 流向 π 净额订单路径；`sep_legs` 流向
   `OverlayState`/`LevelLedgerMirror`（只读镜像）。谁都不覆盖谁。
3. **但只有净额那条到得了下单出口**：`net_target_units`（`coverage/leg.rs:300`）把
   `Long→+units / Short→−units` 求和成有符号标量。其 doc（`leg.rs:296-299`）自认：

   > ★分账本（毛）→ 净账本的**语义降维**（M29 §7 诚实声明）……净额化**丢失**双开的毛敞口信息
   > ……这是 Nautilus 净额兼容的必然降维，**非** bug。**完整毛分账本执行须 hedging 账户（v0 净额）**。

   顺序上唯一的例外是 G7 毛帽 `apply_gross_cap`（`leg.rs:354`）**先于**净额折叠施加
   （`leg.rs:323-324`：「缩放**先于** `net_target_units` 折叠——净标量已丢失毛敞口信息，事后诊断门
   被 #122 终裁拒绝（反例 Long100+Short100：净0毛200）」）。但它默认 `enforce_gross_cap=false`
   （`step.rs:598` 注释 + `:601-604` 门），默认不激活。

**一句话**：分腿信息在 `step.rs:592` 生成、`:606` 被塌缩成一个标量喂给下单路、`:646` 被另存一份
只作归因。**塌缩在下单之前，且是唯一到达下单的路径。**

---

## 2. nautilus 适配层的持仓真相源

### 2.1 `account_adapter.rs:41-46` 那个「诚实 TODO」：**还在，一字未动**

一手实读 `rust/src/theta_v0/nautilus/account_adapter.rs:41-45`（doc block，`:46` = `pub fn
to_account_state`）：

```
41  /// ★诚实有效域：v0 classifier 只产 depth=0 独立根（`recognize` 注释 strategy/mod.rs:594），故
42  /// `net_position` 整体归 voice_qty[0]。多独立根并存（§5）时各根都是 depth=0，net_position 是它们的
43  /// **净和**——这与 S_Θ「多独立根各自 voice_qty」有口径差（Nautilus NETTING 单净仓 vs S_Θ 多根分账）。
44  /// 单根/单方向时无差异（v0 主路径）；多根对冲时需在适配层拆分账（标注非缺陷是有效域边界，TODO）。
45  ///
46  pub fn to_account_state(snap: &PortfolioSnapshot, max_depth: u32) -> AccountState {
```

全文件 115 行，只有 `PortfolioSnapshot`/`to_account_state`/`position_dir` + 4 个单测。
`git log -- rust/src/theta_v0/nautilus/account_adapter.rs`（含 `--all`）只有 2 个提交：
`9c70c4494f`（初建骨架）、`4d993bd646`（#611 rustfmt）。**#68 之后零改动。**

### 2.2 真相源：**NETTING**，机器可查三处

| 证据 | 位置 | 内容 |
|---|---|---|
| ① 读持仓 | `nautilus/theta_strategy.rs:104` | `let net_position = decimal_to_f64(portfolio.net_position(&self.instrument_id));` —— **单个净值** |
| ② 映射 | `nautilus/account_adapter.rs:49` | `voice_qty[0] = snap.net_position.abs() as u32;` |
| ③ venue OMS | `nautilus/backtest_engine.rs:157-158` | `.oms_type(OmsType::Netting)` + `.account_type(AccountType::Cash)` |

`grep -rn "OmsType\|Hedging" rust/src` 全仓命中 3 行，全在 `backtest_engine.rs`，
**`OmsType::Hedging` 零出现**。

Python 侧 NT 系统同结论（`grep -rn "oms_type" trading_system/`）：
`backtest/runner.py:56`、`backtest/rec_backtest.py:111`、`backtest_unn.py:230`、
`backtest_unn_stream.py:184`、`backtest_t_fugue.py:207`、`backtest_fugue_v3.py:245`
—— **全部 `OmsType.NETTING`**，`backtest/catalog_smoke.py:57` 是字符串 `"NETTING"`。
`trading_system/strategy/rec_t_strategy.py:91-93` 逐字：「提单使 NT 净仓位 = 引擎目标净敞口
（NETTING，delta 提单）」。`trading_system/live/runner.py` 是阶段4 骨架，`main()` 打印
「实盘入口为阶段4骨架，尚未实装」并 `return 1`。

补充（避免另一个方向的误判）：`nautilus/mod.rs:11-22` 明载 #524 订正——**适配层已过骨架期**，
`theta_strategy.rs`/`backtest_engine.rs` 真实 `use nautilus_*`（feature `nautilus`，
Cargo.toml:24-33 五件 v0.60.0）。**"NT 没接"是假的，"NT 接的是 NETTING"是真的。**
唯独 `account_adapter.rs:10` 的模块头「★骨架（nautilus 依赖未加，不编译）」是**过期未订正**的
残留声明（#524 订正没扫到这一份）。

### 2.3 #68 声称做完的前三条：**逐条核实，三条全部没有落在 main 上**

#68 的 Resolution 评论（2026-07-21）声称 ①②③ 已实装。逐条查：

| # | 票面声称 | 实测 | 判定 |
|---|---|---|---|
| ① NETTING→dual | 新增 `DualSnapshot`(:68)/`from_ledger`(:104)/`from_netting`(:122)/`to_account_state_dual`(:157)/`position_dir_dual`(:179) + `ThetaCore::plan_for_bar_dual`(strategy.rs:150) | `grep -rn "DualSnapshot\|plan_for_bar_dual\|from_netting" rust/src` → **零命中**。`git log --all -S DualSnapshot` → 只有 2 个 commit，都是 **docs**（`2330ea6a3e` 加报告、`7798007580` 归档删报告），**rust/src 从未有过这些符号** | **没做（代码不存在）** |
| ② TW 接线 | `plan_and_fill_mtm_dual` 内建 TW 账本，`tw_final=Some`(runner.rs:4072) | `plan_and_fill_mtm_dual` 已被 **#499 退役删除**（CLOSED 2026-07-28）。`runner.rs:144,146` 是墓碑注释：「run_theta_v0(_dual) 已按 #499 裁定退役删除」。当前 `rust/src` 内该函数零存在 | **载体已删** |
| ③ F-01 deprecated 保留 | `wverify_run.rs:1722-1741` 改写为正式边界声明、`runner.rs:370-373` 登记 | 被 #499 **超越**：整个 `run_theta_v0`/`run_theta_v0_dual` 入口连同 deprecated 标记一起删除。当前 `runner.rs:355-380` 是 `RunResult` 构造体，无 deprecated 段 | **已被后续裁定作废** |

**根因（票面自证）**：#68 的实装报告 `chanlun/review-results/c1-dual-ledger-wiring-20260720.md`
（已被 #504 归档出仓，可从 `git show 2330ea6a3e:` 取回）文头逐字：

> - 工位：worktree `/tmp/kimi-nest-mainline`（分支 `kimi-nest-mainline-20260717`），2026-07-20。
> - 纪律：**未做 git mutation；未写主仓**；`rust/Cargo.toml` 未动 …

即 **#68 的代码从头到尾没有 commit 过**，只活在一个临时工作树里。该目录 `/private/tmp/kimi-nest-mainline`
今天仍在，但 `rust/src/theta_v0/nautilus/` 目录已不存在（`ls` 报 No such file）。分支
`kimi-nest-mainline-20260717` 已 merge 进 main（`632ae9eb58`），但
`git show kimi-nest-mainline-20260717:…/account_adapter.rs` 里 `DualSnapshot` 零命中，107 行
（= rustfmt 前的同一份骨架）。**代码已丢失。**

090 照实：不是「找不到」，是**机器可证不存在**（`git log --all -S` 覆盖全部 331 个 ref）。

---

## 3. 那条 venue 前提的性质：**未验证的部署前提，不是已验证的 venue 支持**

`rust/src/theta_v0/nautilus/strategy.rs:276-277` 逐字：

```
/// venue 侧 hedge-mode 账户前提（q⁺/q⁻ 双腿共存）列部署裁定（施工图 §7 L7）——
/// 本适配层只产决策/意图，venue 撮合语义不变。
```

「**列部署裁定**」= 列为待裁的部署项，不是「已验证 venue 支持」。后半句自己划了界：
本层只产意图，撮合语义不变。

它引的「施工图 §7 L7」= `p120-nested-voice-dual-ledger-design-20260718.md` §7 遗留项 L7
（该文件已被 #504 归档出仓，正文在本基线不可达）。但**两份独立在案文档**给出同一读法
（均从 `git show 7798007580^:` 取得）：

- `chanlun/review-results/dual-open-e2e-fix-design-20260719.md:313`：
  > **venue 前提**：q_short 是真空头（期货/永续域）；现货标的须部署层 gate=0；
  > **venue 不支持 hedge-mode 时 (Q⁺,Q⁻) 不可共存**——venue 适配是部署裁定（p120 §7 L7），
  > 引擎层不区分标的。
- 同文 `:347`：
  > **venue hedge-mode 部署前提**（p120 §7 L7；C2 恰好是 venue 不支持时的降级路径——
  > **venue 事实核查是裁定项 0 的输入**）。
- `chanlun/review-results/level-exec-existing-inventory-20260720.md:56`：
  > 生产路径（nautilus）不持有 DualLedger……venue hedge-mode（q⁺/q⁻ 双腿共存）前提
  > 列部署裁定（`nautilus/strategy.rs:236-237`，p120 §7 L7）。

**判定：确为未验证的部署前提，明写。** 且 `dual-open-e2e-fix-design:347` 把「venue 事实核查」
本身登记成**尚未提供的输入**——意味着仓内从未做过这次核查。

**仓外事实登记（按边界约束，不外推）**：Binance 永续的逐仓粒度、子账户是否为出路——
**仓内无记载**。`docs/` 全仓 `grep "isolated margin"` 零命中；`docs/drain_and_three_stages.md:194`
明确划界：「缠师原文极少涉及期货/保证金/强平……逐仓/全仓选择是对级别独立性原理的**延伸应用，
非原文直接论述**」；`docs/three_stages_accounting_design.md:29` 同：「缠师零杠杆现货，
margin 概念在原文论域外」。未联网、未猜测。

---

## 4. #68 登记未做的三条：现状与承接票

### 4.1 做空保证金 / 借券成本建模 —— **仍未建模，无专门承接票**

`rust/src/theta_v0/backtest/dual_ledger.rs:25-26` 逐字（声明仍在）：

> - 做空保证金/借券成本未建模（沿用 runner.rs 声明）；双腿共存时 |net| 基 accrual 是近似口径
>   （per-leg 精化列遗留 **L6**，涉成本数字变更须独立裁定）。

同文 `:20-24` 另有一条 spot 裁定的牵连登记：「**触发前提已成立**（#303 裁定本仓 venue = spot，
#340 补登）……该 gate 自此是**待落实的部署层要求**而非假想分支；落实归 **#62** / 部署层清单」。

承接票现状：`#62`（venue 真实费率数据源可得性）**CLOSED**、`#285`（费率标定数据源）**CLOSED**、
`#303`（CostModel 切 spot）**CLOSED**。73 张 OPEN 票中**没有一张**以做空保证金/借券成本为标的。
最接近的两张（都不是同一件事）：
- `#717 [task] L17 短差毛暴露资金参数校准（T6 额度树接线 + 套娃约束参数化）` — **OPEN**
- `#836 [grilling] 总账口径：多区同时满仓算不算杠杆、毛账与净账的缺口怎么处置` — **OPEN**

### 4.2 C2/C3 载体分支 —— **仍未实装，无承接票，且设计正本已出仓**

`level-exec-existing-inventory-20260720.md:60`：
> **载体 C2/C3 分支未实装**：§3 方案是 C1 分支（已实装即本文件），§4 是 C2 分支设计，
> **C3 只有要点素描（§7-0，未展开为方案）**。

现状：C1 的账本载体 `dual_ledger.rs` 本身已成**死码**（见 §5）；C2/C3 在 `rust/src` 零痕迹
（`grep "C2 载体\|C3 载体"` 零命中）。承接票：**无**。设计文档
`dual-open-e2e-fix-design-20260719.md` / `p120-…-20260718.md` 均已被 #504 归档出仓，
只能从 git history 取回。

### 4.3 LEE 合并 —— **已启动并大部分落地，但门 on 路径未过验收，返工票 #783 OPEN**

| 票 | 状态 | 结果 |
|---|---|---|
| `#693` LEE level_* 接线总裁定 | CLOSED | 裁定接，分两档 |
| `#644` LEE M1–M4 level_* fill.rs 接线 | CLOSED | 已并入 main |
| `#755` LEE 决策层接线（level_order/level_risk 入 sizing） | CLOSED | 接线 `615eb460a7`+`b0a709616b` 已并 main，`enforce_level_cap` **默认 off**；门 on 靶向对照 BTC 20k `n_orders 7082→274` |
| `#777` 影子评审 #755 | CLOSED | **FAIL**：门 on 两件 HIGH 阻断（帽施于投影后越出 `KThetaRiskGate` 可行集；帽只裁标量、腿级账本未裁，两账分裂 31 倍） |
| `#783` #755 返工：级别帽 clamp 前移到投影前 | **OPEN** | 未清偿 |

在场物证：`runner.rs:413` `level_ledger: LevelLedgerMirror`（LEE M1 只读旁路）、
`:417` `level_clock`、`:423-425` `level_attrib_*`。`runner.rs:411-412` 逐字：
「**只读**：本字段不影响 `net_result` 任何数值」。

**照实**：LEE 合并 ≠ 双仓落地。LEE 是"按级别分桶记账 + 按级别分资金权/帽"，它的账本层
（`LevelLedgerMirror`）同样是只读镜像，决策层接线（#755）门默认关且门 on 未过验收。

---

## 5. 勘察中浮出的三条附带事实（非票面四问，但直接影响裁定）

### 5.1 `dual_ledger.rs`（C1 双账本引擎）已成死码

`DualLedger`/`apply_fill_dual`（`backtest/dual_ledger.rs:42,145`）在 `rust/src` 内
**零非自身消费者**（`grep -rn "DualLedger" rust/src | grep -v dual_ledger.rs` → 空；
`apply_fill_dual` 的 10 处命中全在 `dual_ledger.rs` 自身及其 `#[cfg(test)]` 段）。
它原来的唯一调用方 `plan_and_fill_mtm_dual` 已被 #499 删除。
`strategy/mod.rs:940-942` 只剩一条 intra-doc 链接引用。

### 5.2 #178 是一条被忽略的 HITL 裁定，且它正是本票的上位答案

`#178 裁决：SplitLegLedger 接线 vs 净额让步`（CLOSED，2026-07-23，用户 HITL）逐字：

> **2.** …… **fill 层证实 = v1 显式让步**，写入 ADR——并记录自指：ADR 行34 批判净额视角，
> 而 v1 执行层仍净额单出口，fill 级「父仓不动」不可证实，挂账。
>
> **3. 档B（逐腿下单出口）：标记——「不是最严格的实现，以后再搞」（用户原话，2026-07-23）。**
> 本轮拦下理由记录在案：**funding/borrow 毛暴露口径在蓝图与 ADR 均无明文，换出口=发明新钱规矩**，
> 违反「不重设计、只对准蓝图」原则；待蓝图补齐该口径后单独立案。

即：**逐腿下单出口（=真双仓执行）是被用户明确挂起的**，理由恰好是 §4.1 那条未建模的
做空保证金/借券成本口径。这条裁定至今未被推翻。

而 #178 裁的「档A 共存」（OscillationBook + SplitLegLedger 双写）**后来又被删了**——
`#282 删除 S6 开空腿账面形态全套（#280 裁定）` CLOSED，删除清单第一项即 `SplitLegLedger`。
残留墓碑：`strategy/ledger.rs:74`、`backtest/fill.rs:400`、`strategy/channel.rs:661`。

### 5.3 `.chanlun/implementation-index-20260723.md` 能力1 已过期（两处）

该索引「能力 1：分账本/双腿不净额」表列 6 个实装体。其中：
- ② `SplitLegLedger` 标「仅测试」→ 实际**已被 #282 删除**；
- ③ `DualLedger` 标「deprecated（入口 `run_theta_v0_dual`）」→ 入口**已被 #499 删除**，现为**死码**。

该表的**横切警示仍然成立且更严重**（原文逐字）：
> P^sep/M14 同概念被独立实装三次（separate.rs 06-28 / OverlayState 07-04 / DualLedger 07-19）
> 互不引用，且均未回溯 ShortBook（06-12）——本仓「找不到前人成果而重造」的头号标本。

按本次实测应订正为**四次**（+ `VoiceExecBook` 07-19/W1），且其中
`ledger/separate.rs`（零生产消费者）、`trading/ledger.rs` ShortBook/DirectionalBook（全 git 史零消费者）、
`backtest/dual_ledger.rs`（现死码）三套是死码，`SplitLegLedger` 一套已删。

---

## 6. 结论落点（票面两个后果声明）

**落在 (b)**：

> 执行侧仍是 NETTING、hedge-mode 簿只在回测侧且不进决策 ⟹ 逐仓分开暂时只能是虚拟记账。

三条支撑，全部机器可查：

1. **venue 层**：`backtest_engine.rs:157` `OmsType::Netting` + `AccountType::Cash`；全仓
   `OmsType::Hedging` 零出现；Python NT 六处 venue 配置全 `OmsType.NETTING`。
2. **适配层**：`theta_strategy.rs:104` 只读 `portfolio.net_position()` 一个标量；
   `account_adapter.rs:49` 塞进 `voice_qty[0]`；那条「多根对冲时需在适配层拆分账（TODO）」原封不动。
3. **决策层**：分腿目标 `legs` 在 `coverage/step.rs:606` 被 `net_target_units` 塌缩成标量后才进
   下单路；`leg.rs:299` 自认「完整毛分账本执行须 hedging 账户（v0 净额）」。

**唯一的例外要照实说清楚**：`VOICE_EXEC=1` 下 `VoiceExecBook` 确实按腿开合、两腿各自存续不互相湮灭
（`overlay_state.rs:292`）。但 (i) 它在 in-crate 回测模拟器内，不是 venue 出口；(ii) 默认关，
CI 不开，仅 M8 臂R 审计用；(iii) 决策层仍由净额影子账本驱动。
**它证明的是「引擎有能力按腿记账」，不是「执行侧已是双仓」。**

---

## 7. 没测出来的项与原因

| 项 | 状态 | 原因 |
|---|---|---|
| 施工图 `p120-…-20260718.md` §7 L7 原文 | **未读到** | 该文件已被 `#504`（`7798007580`）归档出仓，且在归档前的 `7798007580^` 树中也已不存在（更早批次移出）。改用两份独立引用它的在案文档交叉坐实 L7 语义（§3），未直接引原文 |
| `#68` 那份 dual 代码本体 | **不可恢复** | 从未 commit（票面自述「未做 git mutation」），`/private/tmp/kimi-nest-mainline` 中 `rust/src/theta_v0/nautilus/` 已不存在 |
| `cargo build`/`cargo test --no-run` 编译验证 | **未跑** | 本票全部结论来自源码内容与调用图（Read + grep + `git log -S`），无一条依赖"能否编译"。未跑即未跑，不冒充绿。若后续要机器坐实「`dual_ledger` 死码」，可跑 `cargo build --lib --features backtest_bin` 看 dead_code 告警 |
| Binance 永续逐仓粒度 / 子账户是否为出路 | **仓内无记载** | 边界约束禁止外推与联网。`docs/` 两处明确把 margin 划出原文论域（`drain_and_three_stages.md:194`、`three_stages_accounting_design.md:29`）。归另一张票 |
| `VOICE_EXEC` 在真实人工跑批中的取值 | **只能查到脚本层** | 仓内唯一显式设置点 `scripts/check_armR_trades_digest.py:17`；CI 零出现。人工 shell 里怎么设，仓内查不到 |

---

## 附：本票一手核实的证据行号总表（基线 c92ae8998c）

| 编号 | 路径:行 | 内容 |
|---|---|---|
| E1 | `rust/src/theta_v0/nautilus/account_adapter.rs:41-45` | 「诚实 TODO」原文（NETTING 单净仓 vs S_Θ 多根分账）仍在 |
| E2 | `rust/src/theta_v0/nautilus/account_adapter.rs:49` | `voice_qty[0] = snap.net_position.abs() as u32` |
| E3 | `rust/src/theta_v0/nautilus/theta_strategy.rs:104` | `portfolio.net_position(&self.instrument_id)` = 持仓真相源 |
| E4 | `rust/src/theta_v0/nautilus/backtest_engine.rs:157-158` | `OmsType::Netting` + `AccountType::Cash` |
| E5 | `rust/src/theta_v0/nautilus/strategy.rs:276-277` | venue hedge-mode 前提「列部署裁定」 |
| E6 | `rust/src/theta_v0/nautilus/mod.rs:11-22` | #524 订正：适配层已过骨架期（实装在役） |
| E7 | `rust/src/theta_v0/strategy/overlay_state.rs:13-18` | `Order_t = ΔN` = 净额下单量 |
| E8 | `rust/src/theta_v0/strategy/overlay_state.rs:288-297` | `VoiceExecBook` = 事件驱动分腿执行（两腿不湮灭，:292） |
| E9 | `rust/src/theta_v0/strategy/coverage/step.rs:592/606/646` | legs 生成 → 净额塌缩 → 只读重打包（同源并存） |
| E10 | `rust/src/theta_v0/strategy/coverage/leg.rs:296-300` | `net_target_units` 语义降维声明「须 hedging 账户」 |
| E11 | `rust/src/theta_v0/backtest/runner.rs:479-480` | overlay = 只读旁路，`net_result` bit-exact |
| E12 | `rust/src/theta_v0/backtest/runner.rs:144,146` | #499 退役墓碑（`run_theta_v0`/`_dual` 已删） |
| E13 | `rust/src/theta_v0/env_registry.rs:76,204-210` | `VOICE_EXEC` 默认臂 = 净额臂 |
| E14 | `rust/src/theta_v0/backtest/dual_ledger.rs:25-26` | 做空保证金/借券成本未建模（L6 遗留） |
| E15 | `rust/src/theta_v0/backtest/dual_ledger.rs:42,145` | `DualLedger`/`apply_fill_dual` 零外部消费者（死码） |
| E16 | `trading_system/strategy/rec_t_strategy.py:91-93` | Python NT：delta 提单使净仓=目标（NETTING） |
| E17 | `trading_system/live/runner.py:37-73` | 实盘入口 = 阶段4 骨架，未实装 |
| E18 | `scripts/check_armR_trades_digest.py:17` | 仓内唯一显式 `VOICE_EXEC=1` 设置点（M8 臂R） |
