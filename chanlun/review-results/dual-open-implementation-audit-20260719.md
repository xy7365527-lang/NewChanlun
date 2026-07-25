# 多空双开实装审计：voice 账本 vs 净额仓位（2026-07-19）

**工位**：深度调研（只写本文档，未改 rust/src 一行，无 git mutation）。
**嫌疑**：trades.jsonl 的 voice 账本显示 Long/Short 声部同时存在（518 笔中 Long +23826.69 /
Short −16464.99），但执行层是净额单仓——多空双开在仓位层是否真实存在？
**结论（一句话）**：实装是**账本双开 + 仓位净额单向**——双开在声部账本层真实存在（持仓
bar 的 19.09%），在仓位层被 `N=Net(P^sep)` 有损投影逐 bar 消去；该行为与设计文档
（Lean 定理 + 多空对冲.pdf §10/§11）**一致，无偏离**，代码声明=实际能力（090 通过）。

---

## 1. 实装机制（行号锚）

### 1.1 账户层：单标量净额仓（执行真值）

- `rust/src/theta_v0/backtest/runner.rs:1065`：`let mut units: f64 = 0.0; // p_t = 净 lot
  （apply_order 维护，有符号：正多/负空/0空仓）`——账户持仓是**单一有符号标量**，不是
  多空双账本。结构上不可能同时持有多头与空头。
- `runner.rs:1191-1208`：净额 fill loop——延迟订单在 exec bar 由 `apply_order` 逐笔成交，
  维护 cash/units/entry_cost。
- `runner.rs:3691-3721` `apply_order`：Buy/Add → δ=+1，Sell → δ=−1 直接加减 `units`
  （可先平后开翻转）；Close/Reduce 按**当前持仓符号**反向平仓（close_only，空仓 noop）。
  订单语义本身就是净额增量语义。
- `runner.rs:1214-1223`：funding/borrow 持仓成本按 `units.abs() * px`（**净额名义**）计
  提——净额化渗透进成本层；TW ShortDiff 划转同样按 `units.abs()`（`runner.rs:1234`）。

### 1.2 声部账本层：P^sep 簿保存 (Q⁺,Q⁻)（记账，非仓位）

- `rust/src/theta_v0/strategy/overlay_state.rs:59-79` `VoiceBook`：每活动声部一条单向腿
  `q_v σ_v` + entry_v/exit_v/parent(v)/role(v)/pnl_v。
- `overlay_state.rs:101-113` `OverlayState`：`books`（P^sep 活动腿）+ `net`（=Σσ_v q_v
  冗余缓存）+ `account_price_pnl` + `closed`。
- `overlay_state.rs:15`（模块头自声明）：`N_t = Net(P^sep_t) = Σ_v σ_v q_v` 是**有损投影，
  双开 (Q,Q)↦0**。
- `overlay_state.rs:170-250` `step`：rebalance 到目标 P^sep，`order = N_t − N_{t−1} = ΔN`
  （构造性守恒）。**订单是净额增量**——声部 A 多 100 + 声部 B 空 60 ⟹ N=+40，账户只见 +40。
- `overlay_state.rs:308-327`（测试 `hedged_two_voices_net_zero_but_book_nonzero`）：父多头
  10 + 子空头 10 ⟹ `net_after=0`、`order=0`、`active_voices=2`——"双开在仓位层消失、在账
  本层保留"的构造性证据。
- `runner.rs:1555-1559`：overlay 步进是**只读旁路**，喂的是 `step_trace.sep_legs`（决策
  目标），不改净额 fill 的 cash/units/trade_pnls（`runner.rs:1039-1040`：overlay=None
  ⟹ 净额路径 bit-exact）。
- `overlay_state.rs:22-27`（自声明）：逐声部 pnl_v 是净额价格 PnL 的**分解**（PDF §11 线
  性恒等 `Σσ_v q_v ΔP = N·ΔP`），**不是**独立 self-financing NAV。

### 1.3 trades.jsonl 的来源与字段口径

- `runner.rs:1105-1110`：G4 typed ledger（`open_trades` 以 voice_id/ElementId 为键）+
  OPSEM_DUMP 落盘——trades.jsonl 的每行是**逐声部归因记录**，不是账户仓位记录。
- 行内 `units` 字段是入场 sizing 快照 `SepLeg.q_units`（`runner.rs:1599-1605`），是连续
  sizing 量，非账户持仓。
- `pnl_raw_unlevered = σ·(exit_px − entry_px)`（每单位、不含手数；已用 trade_id=1/2 逐值
  验证：28598.06−26441.55=2156.51 ✓，26417.99−26243.73=174.26 ✓）。

### 1.4 方向语义：§5 多独立根 + 单点双触发消歧

- `rust/src/theta_v0/strategy/voice.rs:98-110` `voice_side`：σ = root_side·(−1)^depth，根
  方向由信号定（3买→Long，3卖→Short）——对齐 `StrategyFamily.lean:570`
  `long_short_both_open_allowed` 的多独立根有效域（`voice.rs:79-95` 注释自引）。
- `rust/src/theta_v0/strategy/mod.rs:266-270` `plan_orders`：用 `voice_side` 定声部绝对方向。
- `voice.rs:265-272` `root_sel`：(χ⁺,χ⁻)=(1,1) 双触发**消歧为 Flat**（镜像反对称强制，
  strict §9）——同一决策点不会同时开多空；双开只能来自**不同声部跨时间的持仓重叠**，
  §3 实证正是如此。

---

## 2. 双开区间实证（/tmp/m8_opsem_fixed/trades.jsonl）

### 2.0 口径声明（090：近似照实标注）

- 持仓区间取 `[entry_bar, exit_bar)`（exit_bar 为离场 bar）。
- 双开 bar：该 bar 上 ≥1 个 Long 声部 ∧ ≥1 个 Short 声部同时持仓（按声部账本重建）。
- **pnl 逐 bar 归属 = 线性均摊** `pnl_raw_unlevered/(exit_bar−entry_bar)`——价格路径非线
  性，此为近似口径（无逐 bar 价格可精算，trades.jsonl 只有 entry/exit 价）。
- `units` 是入场快照，生命周期内 resize 未跟踪——毛/净敞口重构为近似。
- 518 笔中 64 笔 `units=0`（声部账本行但无实际仓位）；报两套口径：**全账本（518）**与
  **units>0（454 笔：247 Long / 207 Short）**，后者为仓位层口径主口径。

### 2.1 双开区间占比

| 指标 | 全账本（518） | units>0（454） |
|---|---|---|
| 窗口 | [596, 264959]，span 264364 bar | 同左 |
| 有持仓 bar | 213770 | 197735 |
| **双开 bar** | **50531** | **37750** |
| 占窗口 | 19.11% | 14.28% |
| **占持仓 bar** | **23.64%** | **19.09%** |
| 连续双开段数 | 141（最长 3373 bar，[51114,54486]） | — |

- 最大并发声部：3（units>0 口径；Long 最多 3、Short 最多 2）。
- 每笔交易持仓时间内处于双开的平均时间份额 29.5%；447 笔（dur>0）中 125 笔全程双开、
  299 笔从未双开。
- 双开配对结构：共存反向声部对 125 对，其中 **0 对是赋格父子（父声部+子对冲腿同时持
  仓）**；36 对双方皆独立根，89 对一方带结构父容器（父容器是 Compose 容器身份，非交易
  声部——247 笔 depth≥1 交易的 parent_id 全部不在 trades.jsonl 账本内）。即**双开 =
  StrategyFamily §5 多独立根型**，非 Origin.VoiceTree 嵌套树父子交替型。

### 2.2 净额化率（双开在仓位层消失的程度）

units>0 口径，双开 bar 上（账本毛敞口 = Σ_L q + Σ_S q；账户净敞口 = |Σ_L q − Σ_S q|）：

- 均值：毛 559.9 / 净 249.0 ⟹ **净/毛 = 45.3%**（中位 48.4%）——平均逾一半账本毛敞口
  在仓位层抵消。
- **59.3% 的双开 bar 毛敞口 > 2×|净额|**（对冲占主导）。
- **11.9% 的双开 bar |净|/毛 < 10%**（近全对冲，账户近乎空仓而账本双开满仓）。

### 2.3 双开区间 pnl 贡献（线性均摊口径）

| 口径 | 总 pnl | 双开 bar 归属 | 占比 | 其中 Long | 其中 Short |
|---|---|---|---|---|---|
| 全账本（518） | +7361.70 | **−488.64** | −6.64% | +9644.64 | −10133.28 |
| units>0（454） | +6510.68 | **−2772.10** | −42.58% | +5943.85 | −8715.95 |

两口径一致为负：双开区间整体是净耗损区间——双开期间 Short 声部的亏损超过 Long 声部
的盈利（均摊近似，见 §2.0）。**这是账本归因读数，不是策略评估结论**（本审计不做策略
优劣判断，v3 纪律：不以回测验证策略）。

### 2.4 账户层恒等（实证侧）

账户仓位恒 = 账本净额 Σσ·q（ΔN 守恒，构造性，`overlay_state.rs:246-249`）⟹ 双开
(Q,Q)↦0 的有损投影在每个双开 bar 上真实发生。**trades.jsonl 的 Long/Short 并存是记账
事实，不是仓位事实。**

---

## 3. 设计 vs 实装对照

三种语义：

- **真双开**（hedge-mode 仓位层）：账户同时持有 (Q⁺>0, Q⁻>0) 两条仓位。
- **账本双开**：声部各自记账（P^sep 簿保存 (Q⁺,Q⁻)），仓位层净额单向。**← 实装。**
- **净额单向**：无声部账本，只有 N。

设计依据：

- **Lean**：`formal/Strict/StrategyFamily.lean:570` `long_short_both_open_allowed` 只证
  **结构允许**（两独立根 side=Long/Short 并存可表达）；docstring 自声明"**不证双开总可
  行**……实际双开是否满足保证金/容量约束属 EmpiricalDomain"（`StrategyFamily.lean:561-
  569`，及 `:669-671` 不声明清单）。⟹ 设计**不要求**仓位层双开。
- **多空对冲.pdf §10.1（p11）**：one-way/netting 账户只保存 N=Σσ_v q_v，"同一标的多空
  同时存在会被净额化……这正是你现在遇到的核心问题"。
- **多空对冲.pdf §10.2（p11-12）**：hedge mode position book 保存 (Q⁺,Q⁻) 而非 Q⁺−Q⁻，
  解决"腿的身份、保证金、执行、止损、归因可分开"，但"**它不改变：同标的同单位数多空
  价格 PnL 抵消**"。
- **多空对冲.pdf §11（p12）+ 定理1（p2-3，净额 NAV 不可识别定理）**：净额账户无法度量
  分账本声部毛捕获；`Q⁺ΔP − Q⁻ΔP = (Q⁺−Q⁻)ΔP` 线性抵消。
- 实装对照：`OverlayState` 正是 PDF §10.2 "position book 保存 (Q⁺,Q⁻)" 的账本层落地
  （`overlay_state.rs:8-9` 自引）；`Order_t=ΔN` 是 PDF §10.1 净额账户语义；pnl_v 分解遵
  PDF §11 线性恒等（`overlay_state.rs:22-27`）。

**判定：实装 = 账本双开（P^sep 簿 (Q⁺,Q⁻)）+ 仓位净额单向（units 单标量）。** 设计从未
要求仓位层真双开——Lean 定理是结构可表达性，PDF §11 明确 hedge mode 也不改变价格
PnL 抵消。代码声明（`overlay_state.rs:15`"有损投影，双开 (Q,Q)↦0"；`runner.rs:1065`
"净 lot"）与实际能力一致。

---

## 4. 偏离评级

- **按设计文档本义（账本层 (Q⁺,Q⁻) 保存 + 净额执行）：无偏离（评级：一致）**。
  声明=能力，090 通过。`long_short_both_open_allowed` 的结构允许由声部账本兑现
  （§2.1：双开 bar 客观存在 37750 个）；净额单向仓位是 PDF §10.1 的明文语义。
- **若有人把"多空双开"解读为仓位层真双开（账户同时持多空仓位）：实装不满足**——但该
  解读与 `StrategyFamily.lean:561-569` docstring 及多空对冲.pdf §10.1/§11 直接矛盾，
  属解读错误，非实装偏离。
- **须照实知情的实质后果**（非偏离，是净额执行的固有语义）：
  1. **成本口径**：funding/borrow 按净额名义计提（`runner.rs:1214-1223`）；真 hedge
     账户按毛敞口计费会更高。TW ShortDiff 划转同样按 |units|（`runner.rs:1234`）。
     portfolio margin（PDF §10.3，p12）未实装。
  2. **归因性质**：pnl_v 是净额价格 PnL 的逐声部分解，非独立 self-financing NAV
     （`overlay_state.rs:26-27` 自声明）。
  3. **实证幅度**：双开 bar 上平均 54.7% 的账本毛敞口在仓位层消失（净/毛 45.3%）；
     11.9% 的双开 bar 账户近空仓而账本双开满仓；双开区间 pnl 贡献为负（两口径一致）。
  4. **嵌套对冲未发生**：赋格父子交替双开（父声部+子 ShortDiff 对冲腿同时持仓）在本数
     据 0 例——depth≥1 交易的父容器身份非交易声部。PDF §8 overlay H_t 的父子对冲场景
     在本窗口未触发，不能据此判定其实装对错（照实否定：未观察，不评价）。

---

## 附：复核方法

- 源码：`rust/src/theta_v0/strategy/overlay_state.rs`（全 360 行）、
  `rust/src/theta_v0/backtest/runner.rs:640-720,1000-1250,1555-1626,3691-3721`、
  `rust/src/theta_v0/strategy/voice.rs`（全 443 行）、
  `rust/src/theta_v0/strategy/mod.rs:130-140,240-400`。
- 设计：`formal/Strict/StrategyFamily.lean:550-585,645-680`；`docs/formal-chain/
  多空对冲.pdf`（pdftotext 提取，§10.1/§10.2 p11-12、§11 p12、定理1 p2-3 逐节核对；
  代码注释自引的"p16 关卡10"在文本层未核到对应表，本文只引用已核验页码）。
- 实证：python3 重建持仓时间线（[entry_bar,exit_bar) 区间扫 264364 bar），脚本即 §2
  各表；pnl 符号约定用 trade_id=1/2 逐值验证。
