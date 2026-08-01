# issue #841 探针报告：两张按级别分钱的权重表定轴

> map #787 → task #841（喂 #839）。2026-08-01。分支 `issue841-probe`（自 `main` 5efb250876）。
> 零生产行为改动：新增代码全在 `#[cfg(test)]` 模块 `rust/src/theta_v0/backtest/issue841_probe.rs`
> （`backtest/mod.rs:124-129` 声明），主仓 `cargo check --lib --tests` 绿。

## 一句话结论

**两张表分的是同一根轴上的钱**（票面的 (a)），**而且比 (a) 更糟**：`depth` 与绝对级别的实际
映射实测为 `绝对级别 = root_level − depth`，**1,756,996 / 1,756,996 个元素零反例**；同一个绝对
级别同时被 2–3 条不同 `depth` 的路径供给（同 bar 口径 96.10%），拿到的 `w_depth` 分别是
0.60 / 0.30 / 0.10 / 0.00 —— 一个级别拿多少钱**不是级别的函数，是根在哪一级的函数**。
`040:20`「每个级别对应一定的资金与筹码」在本仓**不是良定义的**。

---

## 探针与口径

| 探针 | 测什么 | 数据源（全是生产函数） |
|---|---|---|
| A `issue841_axis_structure` | 结构口径 `(root_level, depth) → 绝对级别` | 生产 `IncrementalClassifier` → `coverage::extract_carrier_forest`（K_i，`interp.rs:838-842` 声明的生产元素宇宙）+ 生产 `coverage::leg_target` 对拍 |
| B `issue841_axis_funded` | 资金口径：真拿到钱的声部 | 生产 `run_theta_v0_pi_overlay` → `OverlayRunResult.level_ledger`（LEE M1 只读旁路，消费与 `OverlayState` 同一份 `sep_legs`；`q>0` 才进 `build_level_targets`，`level_ledger.rs:58-74`） |
| C `issue841_level_cap_counterfactual` | `w_ℓ` 开关反事实 | 同上，off/on 四臂 |

**探针↔生产对拍**（#821/#837 做法）：探针的 `walk_depth`（`issue841_probe.rs:63-72`）是
`coverage/leg.rs:215-232 element_depth` 的逐字同构复刻（生产函数 `pub(super)`，模块外不可见）。
每 bar 抽查前 64 个元素，断言
`leg_target(forest, idx, 1000.0, &VoiceConfig::default()).units == 1000.0 × depth_weight(depth)`
（default 下 `w_dir≡1`、`w_grade≡1`，见 `coverage/leg.rs:159` + `leg.rs:140` + `config.rs:157`）。
**1,058,110 次对拍（20k 窗）/ 1,246,905 次（200k 窗）全过**，探针 depth 与生产 sizing 同一个数。

**窗口（090 照实，不静默截断）**：BTC 1m 全集 = 4,613,599 bar。本票**没有跑全历史**。
实际窗口：探针 A/B/C 各跑 **前 20,000 bar**（#755 同口径）与 **前 200,000 bar**（探针 A 在 200k
窗用 `stride=10` 抽样，采样 bar 数 19,910；A 在 20k 窗 `stride=1` 无抽样）。耗时：A@20k 5.5s、
A@200k(stride10) 34s、B@200k 15s、C@200k 57s（四臂）。

---

## 第 1 问：`(root_level, depth) → 绝对级别` 的实际映射

### 代码链（点名到行）

1. `coverage/leg.rs:159`（热路径 `:183`）
   `w = depth_weight(depth, config) × dir_weight(&role, depth, config) × w_grade(&role, config)`
2. `coverage/leg.rs:156` / `:180` → `depth = element_depth(view, e_idx)`
3. `coverage/leg.rs:215-232` `element_depth` = **沿 `CoverageElement.parent` 链上溯的长度，根=0**。
   函数 doc（`leg.rs:64`）自陈：「**真嵌套深度**（铁律：来自 parent 链，非级别差）」。
4. parent 链是怎么建的：`coverage/element.rs:356-379 push_element_tree` —— 元素的
   `level = lm.rmove.level()`（`:369`），子元素来自 `lm.sub_moves`（`:376-378`），
   `parent = Some(my_idx)`。
5. `coverage/element.rs:200-202` 的构造性事实：「子元素 `level = 父 level − 1`（descend 级别严格
   递减）」。`RMove::level()`（`classifier/descend.rs:82-87`）：`Segment ⟹ 0`、`Compose ⟹ level`。

⟹ **parent 链每上溯一步，`level` 恰好 −1**。所以「真嵌套深度」在本仓**恰恰就是级别差**——
`leg.rs:64` 那句「非级别差」描述的是**取数方式**（不查 level 字段），不是**取值结果**。

### 实测（探针 A）

| 窗口 | 元素观测数（分母） | `level ≠ root_level − depth` 的元素数 |
|---|---|---|
| 前 20,000 bar，stride=1 | 1,756,996 | **0**（0.0000%） |
| 前 200,000 bar，stride=10 | 13,265,200 | **0**（0.0000%） |

**映射就是 `绝对级别 = root_level − depth`**（等价 `depth = root_level − 绝对级别`），
零反例。资金口径（探针 B，882 个真拿到钱的声部）同样 0 反例。

### 实际出现的 (root_level, depth) → 级别 路径表（200k 窗，stride=10）

| root_level | depth | `w_depth` | 落到的绝对级别 → 元素数 |
|---|---|---|---|
| 0 | 0 | 0.60 | L0: 227,502 |
| 1 | 0 | 0.60 | L1: 171,072 |
| 1 | 1 | 0.30 | L0: 772,675 |
| 2 | 0 | 0.60 | L2: 69,884 |
| 2 | 1 | 0.30 | L1: 292,194 |
| 2 | 2 | 0.10 | L0: 1,178,554 |
| 3 | 0 | 0.60 | L3: 95,281 |
| 3 | 1 | 0.30 | L2: 484,289 |
| 3 | 2 | 0.10 | L1: 1,974,383 |
| 3 | 3 | **0.00** | L0: 7,999,366 |

补：`voice::within_max_depth`（`voice.rs:218-220`）在回测/生产路径上**从未被调用**（全仓
grep 只有定义与单测）——深度截断完全由 `depth_weights` 表长越界归 0 实现（`voice.rs:206-212`），
上表 `(3,3)→L0 w=0.00` 就是这条的实证。

---

## 第 2 问：同一绝对级别能不能从多条路径同时拿到钱 —— **能**

### 结构口径（探针 A，200k 窗 stride=10）

| 绝对级别 | 供给它的全部路径 (root,depth)=元素数 | **拿到钱的路径数**(w>0) | 拿到钱的 depth 集合 |
|---|---|---|---|
| L0 | (0,0)=227,502 (1,1)=772,675 (2,2)=1,178,554 (3,3)=7,999,366 | **3** | {0,1,2} |
| L1 | (1,0)=171,072 (2,1)=292,194 (3,2)=1,974,383 | **3** | {0,1,2} |
| L2 | (2,0)=69,884 (3,1)=484,289 | **2** | {0,1} |
| L3 | (3,0)=95,281 | 1 | {0} |

**严格「同一 bar」口径**（不是跨 bar 聚合）：**19,133 / 19,910 采样 bar = 96.10%** 的 bar 上，
至少有一个绝对级别在同一 bar 内被 ≥2 个不同 depth 同时供给。逐级别命中 bar 数：
L0=19,133、L1=19,031、L2=13,060。（20k 窗 stride=1 口径：12,092/19,433 = 62.22%。）

### 资金口径（探针 B，前 200,000 bar，生产跑批）

`n_orders=31,535`；分母 = 真拿到钱的声部 **882**（closed=882 / active=0）；链断不可判 = **0**。
depth 直方图 `{0: 431, 1: 307, 2: 144}`。

| 绝对级别 | 供给路径 (root,depth)=声部数 | 不同 depth 数 |
|---|---|---|
| L0 | (0,0)=162 (1,1)=141 (2,2)=52 | 3 |
| L1 | (1,0)=129 (2,1)=84 (3,2)=92 | 3 |
| L2 | (2,0)=80 (3,1)=82 | 2 |
| L3 | (3,0)=60 | 1 |

**同时持有口径**（同一绝对级别、不同 depth、持仓区间 `[entry_bar, exit_bar]` 真重叠的声部对）：
`{L0: 2, L1: 55, L2: 57}`，共 **114 对**；明细 `{(L0,d0,d1):2, (L1,d0,d1):14, (L1,d0,d2):31,
(L1,d1,d2):10, (L2,d0,d1):57}`。

（20k 窗小样本对照：71 个声部，同时持有对仅 1 对 `(L1,d0,d1)`——样本量小，200k 窗为准。）

### 裁定

**是。`040:20`「每个级别对应一定的资金与筹码」在本仓不是良定义的。**
同一个 L1 元素，挂在 L1 根下拿 `0.60×base_units`，挂在 L2 根下拿 `0.30×base_units`，挂在 L3
根下拿 `0.10×base_units`——**同一绝对级别的单位资金份额取决于它挂在谁下面，而不是它是哪一级**。
这正是票面预告的 (b) 情形，且比 (a) 更麻烦。

---

## 第 3 问：`level_risk.rs` 那段「禁双重定价」的论证成不成立

### 它自陈的可证伪形式——**没被触发**

`level_risk.rs:28-32` 的可证伪形式是：「若未来有人试图在 `strategy_target_legs` 里也乘上
`level_weight`，会导致连乘」。实测：`level_weight` 的**全部**调用点是
`coverage/sizing.rs:247`（`level_cap`）一处，`level_cap`/`clamp_levels_to_weighted_cap` 的全部
生产调用点是 `fill.rs:5195` 一处。`strategy_target_legs`（`coverage/leg.rs:239-283`）与
`leg_target`（`:148-166`）里**没有任何 level_weight 因子**。

而且 `fill.rs:5180-5199` 的施加形态是 **clamp（min/max），不是乘法**：
`q_ℓ = attribute_total(level_nets, standard_p_star)`（按 `net_ℓ` 比例分账，`level_attrib.rs:74-101`
最大余数法）→ `clamp(q_ℓ, ±cap_ℓ)`，`cap_ℓ = γ̄·w_ℓ·|U_ℓ|`（`sizing.rs:246-248`）。
帽 binding 时结果 = `w_ℓ·γ̄·U`（与 depth 无关），不 binding 时 = 纯 depth 加权量——
**取的是 min，不是积**。⟹ **它自陈的那个可证伪形式，现状确实没有实质等价路径。**

### 但支撑该形式的前提——**被否掉了**

「禁双重定价」这一节立在 `level_risk.rs:14`「两个权重表操作的是**不同轴**」+ `:24-26`
「二者是同一笔资金在两个**正交**维度上的依次投影」之上。实测：

- `depth_weight` 轴 = **`root_level − 绝对级别`**（第 1 问，零反例）。它不是「同一根信号内的
  嵌套对冲层」这样一个与级别无关的维度——它是级别轴上的**相对坐标**。
- `level_weight` 轴 = **绝对级别**（`ElementId.level`，`level_ledger.rs:66` 分桶键）。

两根轴在同一个一维级别轴上，靠 `root_level` 平移相关。「正交」是**假的**。
`voice.rs:5` 的自陈（「根 = 当前最高有效决策级别 L\*；最多 3 层 L\*, L\*-1, L\*-2」）与实测
一致；`level_risk.rs` 的「对偶统一声明」与实测不一致。

**裁定**：「禁双重定价」的**结论**（现状没有连乘）成立；**论证**（因为两轴正交）不成立。
两者靠的不是正交性，而是「一个乘、一个 clamp」这个偶然的施加形态。若日后有人把 `cap_ℓ` 换成
比例缩放（而不是 clamp），连乘立刻成立，而现有论证给不出任何保护。

### 顺带查实的三处文档—代码脱节（照实，不改代码）

1. **`plan_level_gated_order` 这个函数在全仓不存在**。`level_risk.rs:10` 写「帽的实际施加点在
   `fill.rs::plan_level_gated_order`」，`config.rs:211`、`level_order.rs:355`、`level_order.rs:392`
   同样点名它。全仓 grep（含 `rust/src` + 测试）只命中这 4 处注释，无定义、无调用。
   真实施加点唯一：`fill.rs:5180-5199`。
2. **`m8.rs:254`「帽的施加点在 `fill.rs::pi_theta_position`（`risk.enforce_level_cap` 双施加点）」
   不成立**：`pi_theta_position`（`sizing.rs:468`）与 `feasible_lex_candidates` 都不读
   `enforce_level_cap`；全仓 `enforce_level_cap` 的生产读点只有 `fill.rs:5180` 一处。
3. **`LevelOrderLedger::regate` / `plan_gated`（`level_order.rs:513` / `:550`）在生产路径上从未被
   调用**（无 `.regate(` / `.plan_gated(` 调用点）。`fill.rs:5169-5175` 自己也承认「不是
   `LevelOrderLedger`/`LevelOrderPlan` 的 per-level 订单路由……`cap_narrowed_levels` 消费链
   仍未被生产路径点亮」。

---

## 第 4 问：`w_ℓ` 关闭（default）时的完整分配链

`config.rs:228-229`：`level_weights: Vec::new()`、`enforce_level_cap: false`。此时
`fill.rs:5180` 整段不构造不驱动。一笔钱从进入到落到某条 leg 上的完整链：

| # | 环节 | 位置 | 做了什么 | default 下是否生效 |
|---|---|---|---|---|
| 1 | `base_units = equity_nav / px` | `fill.rs:4756` | U_ℓ：NAV/价 = 可建名义手数（方案A协变） | 生效 |
| 2 | 活动集 A_{t+1} = AncOK[(A∖D)∪B∪R] | `coverage/step.rs:592` | 决定哪些元素出腿 | 生效 |
| 3 | **`w = depth_weight(depth) × dir_weight(role,depth) × w_grade(role)`** | `coverage/leg.rs:159`（热路径 `:183`） | **唯一的按级别分钱处** | 生效 |
| 3a | `depth_weight` | `voice.rs:206-212` | `[0.60,0.30,0.10]`，越界 → 0.0，剩余留现金不重分配 | **生效（唯一在分钱的表）** |
| 3b | `dir_weight` | `coverage/leg.rs:93-112`，`:140` | `ThetaDirPreset::Neutral` ⟹ 恒 1.0 | 生效但恒等 |
| 3c | `w_grade` | `coverage/leg.rs:124-130`；`config.rs` `w_grade:[1.0,1.0]` | 恒 1.0 | 生效但恒等 |
| 4 | `units = base_units × w` | `coverage/leg.rs:160-165` | 单腿目标敞口 s_e | 生效 |
| 5 | `apply_gross_cap`（G7 逐根 KKT water-filling） | `coverage/leg.rs:354`，门在 `step.rs:601-605` | 毛敞口帽 | **不生效**（`enforce_gross_cap` default=false，`config.rs`） |
| 6 | `p̃ = net_target_units(legs) = Σ ε_e·s_e` | `coverage/leg.rs:300-308` | 毛腿净额化（无级别信息） | 生效 |
| 7 | `p* = LexArgmin_{p∈𝒦_Θ} J_x(p)`，`cap = γ̄·|base_units|` | `coverage/sizing.rs:468`、`:490` | **账户层单一标量帽，不分级别** | 生效 |
| 8 | `order = schedule_order(p*, p_t, exec_index)` | `coverage/sizing.rs` | 单净额订单 | 生效 |
| 9 | LEE M4 级别帽二次裁剪 | `fill.rs:5180-5199` | `attribute_total` → `clamp(±cap_ℓ)` → 覆盖 order | **不生效**（`enforce_level_cap=false`） |
| 10 | `level_nets` / `LevelLedgerMirror` / `level_attrib` 归因 | `fill.rs:5187`、`:5307`、`level_ledger.rs:81` | **只读诊断**，不改 order/cash/units | 生效但不分钱 |

**回答**：`w_ℓ` 关闭时，**按级别分钱的只剩 `depth_weights [0.60,0.30,0.10]` 这一张表**（第 3a 行）。
`sizing.rs` 的 `level_cap`/`clamp_levels_to_weighted_cap` 不被调用；`plan_level_gated_order`
不存在；`LevelOrderLedger::regate` 从不被调；`level_nets`/`level_attrib` 是只读旁路。
账户层还有一道帽（第 7 行 `γ̄·U`），但它**不带级别下标**——是全账户一个标量。

---

## 第 5 问：把 `w_ℓ` 打开的反事实

探针 C，同窗四臂（前 200,000 bar）：

| 臂 | `level_weights` | Σw | n_orders | trade 笔数 | 已实现 PnL 和 | 权益终值 | **与 off 逐位相同？** |
|---|---|---|---|---|---|---|---|
| off（default） | `[]` 空表 | 0 | 31,535 | 29,105 | −1,585,577.525291 | 0.626017 | — |
| on #755 紧 | `[0.01×6]` | 0.06 | 4,290 | 2,859 | −27,802.185301 | 0.993513 | **否** |
| on #310/m8 | `[0.05×6]` | 0.30 | 14,928 | 11,572 | −194,771.676396 | 0.954536 | **否** |
| on Σ=1 宽 | `[1/6×6]` | 1.00 | 35,003 | 28,166 | −1,526,883.930347 | 0.641724 | **否** |

前 20,000 bar 窗同样全部「否」（off: n_orders=7,082 / PnL −1,033,073.71 / eq 0.757374；
on 0.01×6: n_orders=274 / −13,676.22 / 0.996781；on 0.05×6: 2,397 / −80,755.18 / 0.981000；
on 1/6×6: 5,628 / −555,094.89 / 0.869655）。

**结论：`enforce_level_cap` 开关在当前生产路径上不是死的**——三套权重、两个窗口，
八组对照全部与 off 臂不同（`n_orders`、`trade_pnls` 逐笔位比较、权益终值都不同）。
开关真实 binding，影响量级极大（0.01×6 把订单数压到 13.6%）。

诚实补一句：帽越紧亏损越小、权益终值越高，这**不是 alpha 证据**——default 臂本身在这两个
窗上是大亏（eq 0.626），把仓位压小自然少亏。这里只做「开关是否生效」的判定，不做收益裁决。

---

## 090 未做项（照实，区分「确认为 0」与「没测到」）

**确认为 0 的**：
- `level ≠ root_level − depth` 的元素数 = 0（20k 窗 1,756,996 元素 / 200k 窗 13,265,200 元素）。
- 探针 B 链断不可判声部数 = 0（882/882 全解）。
- `plan_level_gated_order` 定义数 = 0（全仓 grep）。
- `strategy_target_legs`/`leg_target` 中 `level_weight` 因子数 = 0。

**没测到 / 未做的**：
1. **没跑全历史**。BTC 全集 4,613,599 bar，本票实际窗口 = 前 20,000 与前 200,000 bar
   （#837 已实测超线性：52.6 万 bar → 657s）。**200k 之后的窗口未测**，不能外推。
2. **只测了 BTC**。其余 7 个品种（ES/CL/GC/BRN/DX/QQQ/OKLO）未跑——「多路径供给」是否跨品种
   稳定，**没测到**。
3. **探针 A 在 200k 窗用了 stride=10 抽样**（19,910/200,000 bar）。20k 窗是 stride=1 无抽样。
   两者结论一致，但 200k 窗的「同 bar 96.10%」是抽样估计，不是全 bar 普查。
4. **`leg_target` 对拍每 bar 只抽查前 64 个元素**（20k 窗 1,058,110 次 / 200k 窗 1,246,905 次），
   不是逐元素全查。且 `depth≥3` 时探针与生产都返 0.0，该区间的对拍是**退化的**——
   depth≥3 的 depth 值本身没有被 `leg_target` 独立承保。
5. **`depth>0` 腿的成交侧行为未单独测**。探针 B 的分母是「进了 `build_level_targets`（q>0）
   的声部」，不是实际成交手数——`ClosedVoice` 不落 `q_v`（#837 已声明的同一限制）。
   「同一绝对级别在**成交手数**口径上被多路径供给多少钱」**没测到**。
6. **没测「把 `cap_ℓ` 从 clamp 改成比例缩放会不会连乘」**——那是改生产代码，超出本票的零改动约束。
7. `root_level` 在本仓的实际取值只观察到 0..3（200k 窗）。更高塔级别下映射是否仍是
   `root_level − depth`，**没测到**（结构上由 `push_element_tree` 保证，但无实测）。

---

## 复现

```bash
cd rust
# 探针 A（结构口径）
ISSUE841_BARS=20000                    cargo test --release --lib -- --ignored --nocapture issue841_axis_structure
ISSUE841_BARS=200000 ISSUE841_STRIDE=10 cargo test --release --lib -- --ignored --nocapture issue841_axis_structure
# 探针 B（资金口径）
ISSUE841_BARS=200000 cargo test --release --lib -- --ignored --nocapture issue841_axis_funded
# 探针 C（w_ℓ 反事实）
ISSUE841_BARS=200000 cargo test --release --lib -- --ignored --nocapture issue841_level_cap_counterfactual
```
报告落 `/tmp/issue841_probe{A,B,C}_*.md`。需要 `analysis/data_cache/btc_1m_full.json`
（worktree 内为指向主仓的软链，gitignored）。

## 建议下一步（不在本票范围）

- `level_risk.rs:12-32` 的「对偶统一声明」需按实测改写：depth 与 level 不正交，
  `depth = root_level − level`；「禁双重定价」的守卫理由应改成「level 侧是 clamp 不是乘法」。
- `level_risk.rs:10` / `config.rs:211` / `level_order.rs:355,392` 对 `plan_level_gated_order`
  的四处点名指向不存在的函数；`m8.rs:254` 的「双施加点」描述与代码不符。
- `040:20`「每个级别一份筹码」如果要在本仓成立，需要先裁定：级别的资金份额到底由
  **绝对级别**定，还是由 **(根, 深度)** 定。当前是后者，而两张表按前者写文档。
