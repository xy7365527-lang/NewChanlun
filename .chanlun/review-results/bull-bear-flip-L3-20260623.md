# 牛熊切换 flip short 吃跌真 alpha — L3 判决（#37）

> 工位：swarm/bull-bear-flip（069号 RTAS 子蜂群节点 + 275号局部依赖）｜母任务 #37（吃跌核心）
> 日期：2026-06-23 ｜ 分支：bull-bear-flip-37-20260623（worktree 隔离，基于 #36 commit a8d43d6fab）
> 上游：#36 route-bsp-complete §4.4（真顶 trend_done_clear 清现金⟹不吃跌）｜539号做空腿失血｜552号 HOLD_ANCHOR
> 认识论等级：实装规则导出 = **L0/原文+#36**；bit-exact OFF = **L1**；零panic+守恒 = **L1**；8标的×3变体收益 = **L3**

---

## 一、结论（一句话判决）

**从"真顶清现金（不吃跌）"改为"真顶 flip short（吃跌）"已严格实装并 L3 全跑通（8标的×3变体，零穿仓、final_nav 全有限、OFF bit-exact）。三态判决：**

1. **真顶 flip short（type1 门控，FLIP 变体）= 结构性失效（bbflip=0 全 8 标的）**。`core_trend_done`（核心级 type1）与 `emergent_top→Short`（升跌完备性转下）**从不同时发生** ⟹ block A'' 的 flip 分支永不触发。FLIP 相对 OFF 的差异**全部来自 emergent-long HOLD**（把核心走势完成处的"清现金"换成"持多/持空"）：5/8 改善 / 3/8 税，**全 8 < BH**。

2. **核心跟随 emergent（FOLLOW 变体）= 吃跌可达但被假空挤兑吞没**。FOLLOW 主动翻转（bbflip 19–73），**真顶/真底的熊市空腿确实盈利**（BTC 2021–2022：+8.9%、+17.3%；2018：+15.2%、+13.1%），**但 emergent_top→Short 同时在牛市途中回调（假顶）触发** ⟹ 假空腿被挤兑巨亏（BTC：做空 2020 年大底 10872→15300=**−40.7%**、2018 大底 3842→5682=**−47.9%**）⟹ 净价格捕获 **eps_capture%=−123%（BTC）**，6/8 标的为负 ⟹ **FOLLOW 普遍劣于 OFF（BTC −69.2%），唯 OKLO +319.2% 超 BH（但来自多头非吃跌）**。

3. **吃跌真 alpha 存在但不可达净正**：熊市真空腿赚钱（吃跌 alpha 真实存在），但信号层 `emergent_top→Short` **无法区分真顶与假顶**，假顶挤兑损失支配真顶吃跌收益。**这是 539号"做空 alpha 在熊非牛"的精化——不是做空必亏，而是真顶/假顶判别缺失。**

**OFF bit-exact 守住**（rec≡flat：BTC final_nav=126027.46378992868 逐位一致 + clears 2==2 + 114 单测全绿，含 6 新 TDD）。

---

## 二、实装内容（L0/原文 + #36 封闭性）

worktree 隔离改 `rust/`，全部新逻辑 flag 门控，OFF（`enable_bull_bear_flip=false ∧ enable_flip_follow_emergent=false`）逐字不变。

### 2.1 FLIP 变体 — `enable_bull_bear_flip`（type1 门控，env `T_BULL_BEAR_FLIP`）

**block A''（核心走势完成处）重构**：原 OFF 在 `core_trend_done`（核心 Long 顶背驰 t1sell[cc] / 核心 Short 底背驰 t1buy[cc]）时 `clear_all` 清现金。bull_bear_flip ON 时由**升跌完备性**（`cur_emergent_dir = view.emergent_top` 方向）裁决三态：

| (核心向, emergent_top) | 行为 | 缠论依据 |
|---|---|---|
| (Long, **Short**) | **flip long→short**（clear+反向 enter，开熊市空腿吃跌）| 第21课走势终完美 + 最高级别走势转下 |
| (Short, **Long**) | **flip short→long**（关闭熊市空腿=**有终点**，区别于 reading_b 冻结空腿）| 底背驰=下跌走势终完美 |
| (Long, Long) / (Short, Short) | **HOLD**（持多/持空，不清不翻=honor emergent-long，**非清现金踏空、非假空腿**）| 021:40 上涨趋势未完成 |
| (_, None) | 保守清现金（无升跌完备性信息不臆测翻空，回退 OFF）| — |

**route_bsp 核心反向门控**（次要一致性）：非 type1 反核心 BSP 的既有 flip 仅当 `cur_emergent_dir == 反向 dir` 时放行，否则抑制（`n_suppressed_core_flips`）——honor emergent-long 全覆盖。OFF 时 `emergent_confirms = !enable_bull_bear_flip` 恒真 ⟹ 无条件 flip（bit-exact）。

### 2.2 FOLLOW 变体 — `enable_flip_follow_emergent`（解耦 type1，env `T_FLIP_FOLLOW_EMERGENT`）

**block A^FOLLOW（新增，旁路 block A''）**：核心方向 ≠ `emergent_top` 方向 ⟹ 直接 flip 核心到 emergent 向（clear+反向 enter）。config 中关 `trend_done_clear`（核心跟随 emergent，不被清现金）。**动机**：FLIP 的 type1∧emergent 同级约束 ⟹ bbflip 恒 0，无法实测吃跌；FOLLOW 直接让核心骑最高级别走势，测"吃跌是否可达"。

### 2.3 非 ANCHOR（编排者三态要求）

三态均**条件化于升跌完备性**，**无硬编码永不翻空**（552号 ANCHOR 死扣份额不下放 vs 本工位条件化 flip/hold，二者正交）。"骑牛下熊"（ANCHOR）被显式拒绝：emergent→Short 时 FLIP/FOLLOW 都放行翻空（非死守多头）。

---

## 三、L3 对照（8标的×3变体，PerfectionMode::Structural，a₀=Segment）

跑法：`BBF_EPISODES=1 cargo test --release recursive_t::rec_stream::tests::bull_bear_flip_l3 -- --exact --ignored --nocapture`

### 3.1 收益矩阵（strat% vs BH，* = 超 BH）

| 标的 | OFF（清现金）| FLIP（type1门控）| FOLLOW（跟随emergent）| BH | bbflip(F/Fo) | c0_down% |
|------|------|------|--------|------|----|----|
| CL   | +20.3 | +12.7 | −1.5 | +28.2 | 0/38 | 73% |
| BRN  | −26.0 | −6.0 | −23.8 | +87.4 | 0/56 | 69% |
| DX   | −0.7 | −1.3 | +1.8 | +4.1 | 0/56 | 59% |
| GC   | −22.7 | −27.1 | −19.3 | +257.3 | 0/73 | 12% |
| ES   | −2.3 | +3.2 | −14.8 | +594.3 | 0/71 | 6% |
| QQQ  | −8.6 | +2.0 | −15.9 | +174.6 | 0/19 | 1% |
| BTC  | +26.0 | +37.6 | **−69.2** | +1380.4 | 0/67 | 22% |
| OKLO | +55.3 | +191.6 | **+319.2\*** | +307.1 | 0/19 | 16% |

- **FLIP bbflip=0 全 8 标的**：真顶 flip short 路径结构性失效。FLIP vs OFF = 纯 HOLD 效应（5/8 改善{BRN+20.0/ES+5.5/QQQ+10.6/BTC+11.6/OKLO+136.3}，3/8 税{CL−7.6/DX−0.6/GC−4.4}），**全 8 < BH**。
- **FOLLOW bbflip=19–73**：主动翻转，但仅 OKLO 超 BH（且来自多头，见 §3.3）。BTC −69.2% 灾难（假空挤兑）。
- **c0_down%**（emergent_top=Short 的 bar 占比）：震荡标的{CL 73%/BRN 69%/DX 59%}顶层频繁转 Short，强牛标的{QQQ 1%/ES 6%/GC 12%/OKLO 16%/BTC 22%}顶层多数时间 Long。

### 3.2 吃跌可达性诊断（核心做空 episode 价格捕获%）

| 标的 | eps%cap(OFF) | eps%cap(FOLLOW) | 判读 |
|------|------|------|------|
| CL | −62.5 | **+5.8** | 微正（震荡 regime 红利残余）|
| BRN | +33.9 | −76.5 | FOLLOW 假空挤兑 |
| DX | −1.7 | −5.6 | 噪声 |
| GC | −75.3 | −11.9 | 改善但仍负 |
| ES | −112.2 | −24.1 | 仍负 |
| QQQ | −3.7 | +0.0 | 中性（顶层几乎不转 Short）|
| BTC | +19.0 | **−123.0** | 假空挤兑支配 |
| OKLO | −133.0 | −113.4 | 负（多头主导收益）|

**FOLLOW 核心做空价格捕获 6/8 为负** ⟹ 跟随 emergent_top 做空净亏（吃跌不可达净正）。

### 3.3 BTC 逐 episode 实证（吃跌真 alpha 存在 vs 假空挤兑支配）

bar→date 后处理（数据 2017-08-17→2026-06-08）。BTC FOLLOW 核心做空 episode 分两类：

**吃跌（熊市真空腿，盈利）—— 真 alpha 存在：**
| entry→exit | 价 | chg% | regime |
|---|---|---|---|
| 2021-04-20 → 2021-12-07 | 55739→50749 | **+8.9** | 2021 顶部 |
| 2021-12-20 → 2022-01-31 | 46497→38450 | **+17.3** | 2022 熊市启动 |
| 2018-05-12 → 2018-05-28 | 8550→7253 | **+15.2** | 2018 熊 |
| 2018-11-21 → 2018-12-04 | 4502→3911 | **+13.1** | 2018 capitulation |

**假空（牛市途中回调=假顶，挤兑巨亏）—— 支配项：**
| entry→exit | 价 | chg% | regime |
|---|---|---|---|
| 2018-12-22 → 2019-05-03 | 3842→5682 | **−47.9** | 做空 2018 大底，2019 挤空 |
| 2020-09-15 → 2020-11-05 | 10872→15300 | **−40.7** | 做空 2020 牛市启动 |
| 2019-05-03 → 2019-05-19 | 5784→7959 | **−37.6** | 2019 反弹 |
| 2018-04-14 → 2018-05-05 | 8026→9904 | **−23.4** | 牛市回调 |
| 2020-01-26 → 2020-02-09 | 8450→9936 | **−17.6** | 牛市回调 |

**机制坐实**：假空挤兑损失（−47.9/−40.7/−37.6/−23.4/−17.6）**支配**真顶吃跌收益（+17.3/+15.2/+13.1/+8.9）⟹ 净 −123%。`emergent_top→Short` 在真顶**和**牛市回调（假顶）都触发，二者不可分 ⟹ 假空腿被挤兑（做空 2020 大底 = 最大单笔 −40.7%）。

### 3.4 守恒/穿仓

全 24 跑（8×3）`fin.is_finite()` 断言全过 + `prove_tw_neutral` 每 bar 守恒不 panic + liq 有界（0–30）⟹ 无爆仓、无杠杆失控（FOLLOW 的 flip = clear+enter 恒仓，无几何爆仓）。

---

## 四、结果包六要素

### 1. 结论
真顶 flip short 三态严格实装（FLIP type1门控 + FOLLOW 跟随emergent + 非ANCHOR），OFF bit-exact。**L3 判决：吃跌真 alpha 存在（熊市真空腿盈利）但不可达净正——FLIP 的 type1 门控结构性失效（bbflip=0 全8），FOLLOW 跟随 emergent 吃跌但被牛市假顶挤兑吞没（6/8 eps%cap<0，BTC −123%）。根因 = emergent_top→Short 无法区分真顶与假顶。**

### 2. 定义依据
- **第21课走势终完美 + 021:40**：上涨趋势确定后只可能第三类买点 ⟹ emergent=Long 时最高级别走势未完成 ⟹ HOLD honor emergent-long（不开假空）。emergent→Short = 最高完成走势转下 = 真顶 flip 条件。输入满足：`view.emergent_top`（`tree.emergent_top()` = 最高 completed 走势级别+最新单元方向，types.rs:282）+ `view.t1sell/t1buy`（信号层 type1 背驰去重子集）。
- **#36 §4.4**：cc_orbit 体制真顶由 trend_done_clear 清现金⟹不吃跌——本工位把清现金改为 flip（化解 #36 点明的冲突）。
- **第26课名义敞口**：flip = clear_all + enter 恒仓无杠杆（dir_notional ≤ NAV）。

### 3. 边界条件（结论翻转条件）
- **吃跌可达性翻转**：若信号层能区分真顶（最高级别走势真完成）与假顶（牛市回调背驰段），则 FOLLOW 的假空挤兑损失消除 ⟹ 吃跌净正可达。本 L3 用 `emergent_top→Short`（粗粒度顶层方向）不可分二者 ⟹ 不可达。**已实现的翻转**：type1 门控（FLIP，bbflip=0 不吃跌）↔ 跟随 emergent（FOLLOW，吃跌但挤兑）——门控严格性 vs 翻转活性是定性边界。
- **regime 边界**：c0_down%（顶层转 Short 频率）= regime 函数（震荡 59–73% vs 强牛 1–22%）。OKLO FOLLOW 超 BH 是 regime 异类（idiosyncratic，eps%cap=−113 负 ⟹ 收益来自多头非吃跌，与 project_unn_btc_spawn_throwback / 历次 OKLO 异类一致）。
- **mode/a0 边界**：本 L3=Structural/Segment/8标的。换 AND/OR 或 a0=Stroke 或时段，c0_down 划分与吃跌可达性可能移（regime 函数非普适）。

### 4. 下游推论
- **#36 §4.4 冲突已化解**：真顶清现金（OFF）vs flip short（本工位）= 两 flag 门控变体，无 workaround。但 L3 证 flip short 净负 ⟹ 不应入主引擎默认（保留为诊断变体）。
- **539号精化**：做空腿失血**不是做空本身错**，而是**真顶/假顶判别缺失**——熊市真空腿 L3 盈利（吃跌 alpha 真实），假顶假空腿挤兑亏损支配。539 有效域细化：做空 alpha 在熊（真顶后）成立，在牛市回调（假顶）为负，二者由当前信号层不可分。
- **emergent_top 信号层限制坐实**：`emergent_top→Short` 既在真顶又在假顶触发（BTC 1.04M bars Short 中多数是牛市回调）⟹ 作为 flip 触发器制造假空腿。下一步轴 = **真顶/假顶判别器**（如：要求最高级别走势 settled 完成 + 多级别区间套逐级背驰一致 + 顶层级别上移确认），非粗粒度 emergent 方向。
- **与 #37 母任务三态**：emergent-long HOLD（强牛持多，FLIP 已对，5/8 改善）/ 真顶 flip（吃跌 alpha 存在但被假顶污染）/ 非 ANCHOR（已拒骑牛下熊）。HOLD 是唯一净正增量（FLIP>OFF 5/8），flip-short 净负。

### 5. 谱系引用
- **539号**（做空腿失血/clearance regime gating）：本工位精化——真顶吃跌 alpha 真实存在，但假顶假空挤兑支配 ⟹ 539 根因 = 真顶/假顶判别缺失，非做空必亏。
- **567号**（冻结空腿=穿仓+杠杆同根）：本工位的熊市空腿**有终点**（真底 flip 关闭），区别于 567 冻结空腿——但 FOLLOW 的假空腿仍被挤兑亏损（终点不解决方向判别问题）。
- **#36 route-bsp-complete §4.4**：本工位实装其点明的"清现金→flip short"冲突解，L3 证 flip 净负。
- **552号 HOLD_ANCHOR**：三态非 anchor 硬编码（编排者明确区分）——条件化 flip/hold vs anchor 死扣，正交。
- **project_unn_btc_spawn_throwback / project_t_short_leg_regime_function**：BTC FOLLOW −69.2%（假空挤兑）= 强牛过度做空亏的第 N 次复现，机制坐实为真顶/假顶不可分。
- **不确定是否有"真顶/假顶判别"专属谱系**：检索 settled/ 未见独立节点；本文是"吃跌 alpha 存在但被假顶污染不可达"首次 L3 落定。建议 genealogist 评估结晶。

### 6. 影响声明
- **改动文件**（worktree `bull-bear-flip-37-20260623` 隔离，OFF bit-exact）：
  - `rust/src/recursive_t/rec_engine.rs`（+136 行）：EngineConfig +`enable_bull_bear_flip`/`enable_flip_follow_emergent`（env + `bull_bear_flip()`/`flip_follow_emergent()` 构造）；TRoot +同名字段 +`cur_emergent_dir` +计数器（`n_bull_bear_flips`/`n_suppressed_core_flips`/`n_emergent_holds`）；on_bar 设 `cur_emergent_dir` + block A''（三态 flip/hold/clear）+ block A^FOLLOW（跟随 emergent）；route_bsp 核心 flip 门控。
  - `rust/src/recursive_t/rec_stream.rs`（+283 行）：6 TDD 单测（真顶flip/真底flip关熊腿/强牛hold/None保守清现金/route_bsp门控/OFF bit-exact）+ `bull_bear_flip_l3` 三变体全量回测 test（含 eps_capture% 吃跌代理 + c0_down 诊断 + episode/flip dump）。
  - 新增本报告（主仓库 `.chanlun/review-results/`）。
- **不改定义**：flip=clear+enter（恒仓无杠杆）、三态条件化（非硬编码）、OFF 路径不动——纯实现补全 + 验证，**无定义冲突，无 /escalate**（#36 §4.4 冲突由 flag 门控变体化解，非绕过）。

---

## 五、认识论等级标注（formalization-validity-domain 强制）

| 命题 | 等级 | 增量 |
|---|---|---|
| 三态 flip/hold/clear 条件化升跌完备性（非硬编码）| **L0**（021:40 + #36）| 增量：真顶/强牛/真底判据落码 |
| flip=clear+enter 恒仓无杠杆 | **L0**（第26课）| 零（复用 clear_all/enter）|
| OFF bit-exact（rec≡flat BTC 逐位 + clears 2==2）+ 零panic + TW 守恒 | **L1**（114 单测 + rec_flat_btc_bit_exact）| 零（管线正确性）|
| FLIP bbflip=0 全8（type1 门控结构性失效）| **L3**（8标的）| 高：真顶 flip 不触发的经验坐实（core-level type1 ≠ 最高级别反转）|
| FOLLOW 吃跌可达但假顶挤兑支配（6/8 eps%cap<0，BTC −123%）| **L3**（8标的 + BTC 逐episode date 映射）| 高：吃跌 alpha 存在性 + 不可达净正根因（真顶/假顶不可分）|
| FLIP vs OFF = HOLD 效应（5/8改善/3/8税，全<BH）| **L3**（8标的）| 中：emergent-long HOLD = regime 函数 |

> **核心诚实声明**：(a) 真顶 flip short **实装正确**（单测证三态），但 type1 门控 L3 **bbflip=0 全8 = 结构性失效**（不声称"实装⟹吃跌"）。(b) FOLLOW 证**吃跌 alpha 真实存在**（熊市真空腿盈利）**但不可达净正**（假顶挤兑支配，BTC −123%）——否定性结果（吃跌不可达）缩小有效域，比确认更有价值（231号）。(c) 唯一净正增量 = emergent-long HOLD（避免清现金踏空，5/8），非 flip-short。(d) OKLO FOLLOW 超 BH 来自多头非吃跌（eps%cap=−113 负），regime 异类不可迁移。
