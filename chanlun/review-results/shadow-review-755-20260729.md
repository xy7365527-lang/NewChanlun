# 影子评审 #755 — LEE 决策层接线（M4 资金权 + 级别帽入生产 sizing）

- 评审面：merge `e79293642d` 相对第一父的 diff（`615eb460a7` 接线 + `b0a709616b` 对照测试+报告），268 行 / 5 文件。
- 参照面：`/private/tmp/kimi-nest-mainline`（只读），kimi `7d8b45be70`（#310）/ `19aea33a26`（#351）。
- 被评审方自评：`chanlun/review-results/issue755-lee-decision-wiring-20260729.md`（100 行，本轮**当作待核对声明**，不作证据）。
- 执行：两轴（Standards / Spec）各一个 opus 执行器，**新上下文、互不可见对方结论、禁自评**；第三轮定点实测独立执行。
- 工作区：`/private/tmp/wt-777`，分支 `shadow/755-review`。全程只读；实测轮临时改动已还原，收尾 `git status` 干净。

---

## 0. 裁决

**FAIL（须返工后再合）**。门控本身干净——默认 off 时零执行路径，这一条两轴 + 静态全路径枚举均通过，**生产默认配置不受影响**。但门 on 的那条路径存在一个**已实测坐实**的正确性缺陷：级别帽施加在账户层投影**之后**，用逐级裁剪后的标量和直接覆盖 `order`，不再回到 `KThetaRiskGate` 的可行集，**实盘可产出风控明令禁止的订单**。

两轴从互相独立的入口走到同一处（下表），第三轮实测把它从纸面推理变成实证。

| | Spec 轴 | Standards 轴 |
|---|---|---|
| 总判 | FAIL | PASS-with-findings |
| 判据入口 | kimi 原型语义位移未如实记录 | 订单被裁、腿级账本未裁，两本账分裂 |
| 同一根因 | 帽施加点：投影**前** → 投影**后** | 在 π_Θ 唯一出口之后二次改写订单 |

---

## 1. 门控完备性（问题 1）——**PASS**

静态全路径枚举（Standards 轴逐行核对，独立于自评）：

- `fill.rs:5180` `if config.risk.enforce_level_cap {` 是新增块的**第一行**，其前无任何语句 ⟹ flag 判断之前零副作用：无计数器、无日志、无字段写入、无 ledger 记账、无 `RunResult` 填充、无 Vec 分配。
- 块内全部为局部 `let` + 一次条件赋值；`mut order` 在**父提交** `e79293642d^1:fill.rs:5141` 已存在，非本票新加。
- 门外三处改动零行为：`coverage/mod.rs` 纯 re-export；`wverify_run.rs` 98 行全在一个 `#[ignore]` 测试内；`sizing.rs` 43 行除删掉 `#[allow(dead_code)]` 外全是注释。
- 默认值：`config.rs:229` `enforce_level_cap: false`、`level_weights: Vec::new()`；全仓置 true 的只有测试与**未进编译**的 `m8.rs`。

**门 on 生效证据的归属核对**：自评的四项读数被**逐位复现**（n_orders 7082→274；trade_pnls 6814→218 笔；已实现总和 −1033073.706880 → −13676.224808；equity 终值 0.757374 → 0.996781），且门外无行为改动已静态穷举 ⟹ 差异**确实只来自门内**。

但**成因归属未被证明**。自评第 92 行「证明帽在此紧配置下大量 binding」是未经测量的推断：`_lee_used_residual` / `_lee_rescaled` 两个返回旗标被 `_` 丢弃，`n_cap_narrowed` 走的是未点亮的 `plan_gated` 路径 ⟹ 代码里**没有任何 binding 计数读数**。可排除的只有残差桶通道（实测两臂 `level_attrib_n_residual_bars` 均为 0）。第三轮实测补上了这个数：W1 配置下 168441/176304 bar binding，与 binding 假设相容。

### golden 护栏（cmp=0）

**结论成立，但没有任何测试直接锁它**。`cargo test --lib` 实测 2629 passed / 0 failed / 140 ignored（评审基点 HEAD 已含 #747，与自评基线 2605 的差值 24 与本票无关）。cmp=0 这一条目前**全靠散文论证 + 静态等价证明**，仓内没有一条对拍测试覆盖它——而仓内已有可照抄的先例：`runner_tests.rs:5213-5250` `nest_gate_env_gate_off_bitexact_on_shrinks_only`（合成 60 bar、**非 ignore**、断言门关 bit-exact + 门开只缩不增）。本票没有照抄。

---

## 2. 语义重放保真（问题 2）——**四环节中一等价、三有差异**

| 环节 | kimi 侧 | 本仓侧 | 判定 |
|---|---|---|---|
| ① M4 资金权 w_ℓ | `level_risk.rs:1-105`：`level_weight` 按 `level as usize` 索引，越界/空表 ⟹ 0.0；`level_weights_sum_le_one` 容差 1e-9；default 空表 | 三函数逐字同形；`config.rs:204-229` default 同 | **等价**。无归一化（未用部分保留现金不重分配），边界值一致 |
| ② 级别帽 / 定义域 / 命中判定 | `level_cap` 公式同；施加**定义域** = `regate()` 产出的门控结构基准，位置在 `pi_theta_position` **之前**（模块头表格逐字：「`regate` 之后、`pi_theta_position` 之前」）；命中判定 = **逐级**差集，落 `cap_narrowed` | `sizing.rs:263` 公式逐字等价；施加定义域 = `attribute_total(level_nets(sep_legs), p*.round())` 的输出，位置在 `fill.rs:5180-5205`，即 `pi_theta_step_traced_with_risk_seeds`（内含 `pi_theta_position`）**之后**；命中判定 = **bar 级标量** | **有差异，未如实记录 ⟸ 本轮 FAIL 的直接来源**。多级同时命中：两侧都是逐级独立 `map`、无顺序依赖，该子项等价 |
| ③ Σw 校验 | `19aea33a26` MED-1：`assert!`（**非** `debug_assert!`）钉在 clamp 函数入口，commit 原话「必须在 release 构建下同样生效，否则生产环境的误配不会被拦截」 | clamp 函数内无校验；改为调用点 `fill.rs:5181` 的 `debug_assert!` | **有差异，未记录**。该特性文档化跑法本身就是 `--release`（`wverify_run.rs:1998`）⟹ 校验在唯一使用路径上被编译掉，仍是名义接线 |
| ④ 二次裁剪 | **两次** clamp：clamp₁ 对 `gated`（投影前）、clamp₂ 对 `attribute_total` 缩放后的 targets，clamp₂ 后重算 deltas/order_units + 逐级 `debug_assert`；clamp 函数内对 `LEVEL_ACCOUNT_RESIDUAL` 显式 early-return 透传 | **一次** clamp（归因之后），裁完标量求和覆盖 `order`；无逐级断言；clamp 函数**无残差桶透传分支** | **有差异，部分记录**。差异实质：kimi clamp₁ 改变喂给 `attribute_total` 的比例基准，本仓用未裁剪的 `net_ℓ` 定比例；#644 实测 17.6% bar 走缩放分支 ⟹ 非边角。幂等性两侧均成立 |

---

## 3. 冲突清单三条（问题 3）

### ② 账户层标量二次裁剪替代 per-level 订单路由 —— **取舍论证不成立**

自评给的是二选一：走 `LevelOrderLedger::plan_gated`（需先把 M3 `clock_ℓ` 从只读升门控，范围过大）vs 账户层标量。**遗漏第三条**：kimi 的 clamp₁ 施加对象是「已按级别聚合的结构基准」，本仓完全可以在 `pi_theta_step_*` **之前**对 `level_nets(sep_legs)` 施加同一 clamp、再让账户层投影照常跑——不需要 `LevelOrderLedger`、不需要跨 bar `planned` 状态、更不需要动 M3。

⟹ 「M3 范围过大」只解释了**为什么不点亮 `cap_narrowed_levels`**，**不解释为什么把施加点挪到投影之后**。而后者才是全部实质风险的来源。论证覆盖范围 < 实际影响范围。

语义差异描述**不如实**：`fill.rs:5164` 写「§16 唯一出口，本段不新增第二出口，只在其上追加一次账户层二次裁剪」、5170 写「账户层单净额订单形态不变」。实际 `standard_p_star` = `lex_argmin(feasible_lex_candidates(..., gate))`，可行集由 `KThetaRiskGate::caps_raw`（`sizing.rs:343-355`）收窄；覆盖后的 `lee_capped_total` **从未回过这个可行集**——这就是第二出口的定义。

与仓内既定做法也不自洽：G7 毛帽 `apply_gross_cap`（`coverage/leg.rs:354`）在 legs 折叠成净持仓**之前**改 `leg.units`；M2/M3 保证金限仓走 `KThetaRiskGate::no_increase_cap` **折进 𝒦_Θ**（该 doc 明写「退出不能有第二出口 ⟹ 风控项折入 𝒦_Θ 约束门」）。`sizing.rs:250` 自称「G7 同款模式」，实装恰恰不是。

### ① kimi「默认关」订正 —— **属实**

实核 `git -C /private/tmp/kimi-nest-mainline show 7d8b45be70 -- rust/src/theta_v0/config.rs`：`level_weights: Vec::new()` / `enforce_level_cap: false`。kimi 侧本身即带门控，本仓 default 与之逐字相同。该条订正成立。

### ③ RunResult 不扩面 —— **影响评估不完整**

拿不到的读数：帽逐 bar binding 次数、被裁级别集合、`used_residual`/`rescaled` 在门开路径上的分布（`fill.rs:5191` 两个返回值被显式丢弃）。自评称「n_orders/trade_pnls/equity 三项已足以证明生效」——只够证明**有影响**，不够证明**影响是级别帽按预期方式造成的**。−96% 量级的塌缩，在无 binding 计数时无法区分：真·逐级 binding / 残差桶清零导致反复强平 / 混号级别单边裁剪后净额翻转。第三轮实测表明**第三种确实在发生**（见 §5）。

叠加：全仓唯一把 `enforce_level_cap` 置 true 的编译内代码就是那个 `#[ignore]` 测试 ⟹ 门开路径在 CI 上**零可执行证据**。kimi #310 当年有 `runner.rs::lee_m4_level_cap_narrows_position_when_enabled` 类集成验收，本仓无对应件。

---

## 4. 消费链点亮（问题 4）——**均未点亮，自评的自我登记诚实**

- `cap_narrowed_levels`：全仓唯一写点是 `level_order.rs:563` 的 `Vec::new()`（**恒空**）；读点 `level_order.rs:603/651`；`plan_gated` 在 `level_order.rs` 之外**零调用者**（全仓 grep 只剩 doc 提及）⟹ 所谓「四层」全部是死链，读数不可观测。
- `lee_row_cells`：定义在 `report.rs:384`，唯一调用者 `m8.rs:637`；而 `backtest/mod.rs:66` 只有 `mod wverify_run;`，`wverify_run.rs` 内也无 `mod m8;`/`mod report;` ⟹ **两个文件均不参与编译**。

自评条目 2/3 对这两条的登记与实情一致，属诚实登记。

---

## 5. 定点实测：风控绕过可达性 —— **坐实**

第三轮独立实测。gate 是 fill loop 局部变量、外部探针触及不到 ⟹ 临时在仓内插入纯 `eprintln!` 探针 + 一个临时 `#[ignore]` 测试，跑完原样拷回，**收尾 `git status` clean、未 commit**（本体已复核）。探针只读，不改任何决策值；gate 区间按 `caps_raw` 逻辑就地复算，不调用 `caps()` 以免污染 `CAP_BINDING_PROBE`。

窗口：BTC `btc_1m_full.json` 前 **200000** bar，门 on 臂，margin 模型同票内对照测试。

| 读数 | W1 = `[0.01;6]`（票内配置） | W2 = `[0.30,0.05,0.60,0.01,0.02,0.02]`（Σ=1 非对称） |
|---|---|---|
| 门内决策 bar 总数 | 176304 | 176304 |
| 帽真实 binding（`post != pre`） | 168441 | 162181 |
| `\|post\| > \|pre\|`（裁剪反而放大） | 844 | 13347 |
| 符号翻转（`post·pre < 0`） | 9044 | 43177 |
| **真正越界**（`post` ∉ `[lo,hi]`） | **4** | **9** |
| 级别净额异号（两类反例共同前提） | 68299（38.7%） | 68480 |
| gate 被收窄的 bar | 90（sl=48, ss=42, ff=0, nic=0） | 同 |
| 收窄 bar 中越界比例 | 4/90 | 9/90 |

**票内原始窗口（20k、W1）单独跑：越界 0**——因为该窗 gate 收窄只有 8 bar 且都没撞上。**票面自带的测试窗看不到这个问题**，拉到 200k 立刻出现。

越界样例（W1，全部形态为 `stop_short=true` 禁净空、而 `post<0`）：

| bar | 级别净额 basis | cap | capped | pre → post | gate 区间 |
|---|---|---|---|---|---|
| 32771 | `[(1,-99),(2,-296),(3,593)]` | `[9,9,9]` | `[-9,-9,9]` | +198 → **−9** | `[0, 987.97]` |
| 79501 | `[(1,-263),(2,-262),(3,1575)]` | `[8,8,8]` | `[-8,-8,8]` | +875 → **−8** | `[0, 875.33]` |
| 110134 | `[(1,-382),(2,-382),(3,764)]` | `[6,6,6]` | `[-6,-6,6]` | 0 → **−6** | `[0, 636.20]` |
| 113941 | `[(1,-174),(2,-348),(3,696)]` | `[5,5,5]` | `[-5,-5,5]` | +174 → **−5** | `[0, 579.93]` |

机制：正净额集中在**一个**级别、负净额分散在**两个**级别，每级 cap 相同 ⟹ 正侧只保住 1 份 cap、负侧保住 2 份 ⟹ 求和反号。四条全部 `post != pre` ⟹ 都真的走了 `order = schedule_order(...)` 覆盖分支，是**实下的订单**，非内部中间值。

W2 下越界幅度不再是零头：bar 8062 `+322 → −590`；**bar 8941 `pre=0 → post=−568`**——账户层唯一决策出口给出 `p*=0`（平仓）、风控同时禁净空，M4 二次裁剪把它变成 568 手净空。

**结构性边界（量纲上界，实测一致）**：`Σ_ℓ cap_ℓ = (Σw_ℓ)·γ̄·U ≤ γ̄·U` ⟹ gate **全开**时恒不越界（两组配置下全开 bar 越界数均为 0）。暴露面就是 gate 被 `stop_long`/`stop_short`/`force_flat` 收窄的那批 bar。`force_flat`/`no_increase_cap` 本窗零触发 ⟹「强制全平时仍下单」这一最坏形态未获直接观测，但与已观测到的 `pre=0 → post≠0` 是同一机制，只差 gate 状态。

**修复面很窄**：覆盖 `order` 之前把 `lee_capped_total` 重新投影回 gate 可行区间（或直接 `gate.clamp_position`）即可消除越界；若要与 kimi 形状一致，则应把 clamp 前移到 `pi_theta_step_*` 之前施加于 `level_nets`。

---

## 6. Spec 轴：票面 + #693 两裁对照（问题 5）

| 项 | 结论 | 依据 |
|---|---|---|
| 票面：资金权 + 帽接入生产 sizing | ✅ 帽入了 / ⚠ `level_order` 侧未接（已登记） | `fill.rs:5180-5205` |
| 票面：#310 原语生产者接线 | ✅ | `coverage/mod.rs:137-138` re-export；`fill.rs:5197` 唯一生产调用点 |
| 票面：#351 四 MED 接线 | ❌ 部分 | MED-1 降级 `debug_assert`；MED-2 两次 clamp 只做一次、残差桶透传缺失；MED-3 `cap_narrowed` 归属未接；MED-4 不适用 |
| 票面：接口形状以 kimi 实装为准 | ❌ | 施加点相对 kimi 位移（投影前 → 投影后） |
| 票面：与 main 文档承诺出入点**逐列清单**上报 | ❌ | 三处生产 doc 现指向**不存在的函数** `fill.rs::plan_level_gated_order`（全仓 grep 零命中）：`config.rs:211`、`level_risk.rs:10` + 模块头表格、`level_order.rs:355`。#755 只订正了 `sizing.rs` 一处，冲突清单一条未列 |
| 票面：`cargo test --lib` 零新增红 | ✅ | 实跑 2629/0/140 |
| 票面：靶向前后对照 | ✅ 存在（`#[ignore]` + 需 BTC 数据） | `wverify_run.rs:1999-2085` |
| 票面：golden 护栏 cmp=0 | ✅ 结构成立 / ⚠ 无测试锁 | 见 §1 |
| 票面：留痕入 `chanlun/review-results/` | ✅ | 自评报告在位 |
| #693 裁定①（第二档 = 决策层动 sizing） | ⚠ 部分 | 帽动了 sizing；per-level Δq_ℓ 未接，如实登记 |
| #693 裁定②（kimi 形状为准 + 逐点清单 + 不擅自二选一） | ❌ | 见上两行；冲突②恰恰是在两方案间擅自二选一且遗漏第三方案 |
| 门控预设（default off、M0-M3 bit-exact） | ✅ | `RiskConfig::default()` 未改；`wverify_run.rs:2024` 有 default-off 锚断言 |
| 越界实现 | 无 | 268 行内无票面外功能（多导出一个无消费者的 `level_cap`，见 LOW-1） |

---

## 7. 发现汇总（按严重度）

- **[HIGH-1] 覆盖 `order` 绕过 𝒦_Θ 风控门，实测产出风控禁止的订单。** `fill.rs:5198-5205`。已实测坐实（§5）：200k 窗 W1 4 次、W2 9 次越界，最刺眼者 `p*=0 + stop_short` → −568 手净空。票内 20k 窗恰好 0 次，故票面验证未抓到。
- **[HIGH-2] 帽只裁标量 `order`，腿级账本全线未裁 ⟹ 两本账静默分裂。** `fill.rs:5180-5205` vs `fill.rs:5919`。`next_active`/`step_trace.sep_legs`/`standard_p_star` 保留**未裁剪**目标，下游腿级消费者（`prev_active`、M5 overlay `fill.rs:5328`、M1 级别账本 `fill.rs:5364`、G4 typed ledger、TW 账本、W1 声部挂单 `fill.rs:5837`）全读未裁那份；净额账户 `units`/`cash`/`trade_pnls` 走裁剪后的 `order`。实测偏离 **31 倍**：overlay 腿级 price PnL −494,589.89 → −431,209.43（仅降 13%），净额账户已实现 −1,033,073.71 → −13,676.22（降 98.7%）。现有三条守恒断言（M5 ΔN `fill.rs:5336`、LEE-Net 恒等 `fill.rs:5360`、`reconcile_residual` `runner.rs:588-590`）**两边都读未裁的那份** ⟹ 对分裂完全不可见；debug 实跑 24.4s，assert 全部沉默。后果：μ 账本、TW 三阶段判据、声部归因全部按未成交仓位记账。
- **[HIGH-3] 施加点语义位移未记录，三处生产 doc 悬空。** 见 §6 第 5 行。#755 正是把这个开关接通的票，却只订正一处。
- **[MED-1] Σw≤1 校验降级为 `debug_assert!`，在该特性唯一的 release 跑法上被编译掉。** `fill.rs:5181-5184`。失败场景：`level_weights=[0.6,0.5]`（Σ=1.1）+ release ⟹ 无提示，`Σ_ℓ cap_ℓ ≤ γ̄·U` 静默失效——而这正是 §5 中「gate 全开时恒不越界」的前提。
- **[MED-2] 残差桶未透传，`LEVEL_ACCOUNT_RESIDUAL` 桶会被整桶清零。** `sizing.rs:290-303` 缺 kimi #351 的 early-return ⟹ `level_weight(u32::MAX)=0 ⟹ cap=0 ⟹ capped_total=0 ⟹ 强制平仓`。实测两臂 `level_attrib_n_residual_bars` 均为 0 ⟹ 本窗**不可达**，是潜在坑非当前 bug；但状态已从「函数无人调用」变为「生产决策路径上的静默清零」，冲突清单未登记。`_lee_used_residual` 被显式丢弃说明作者看见了该返回值而未处置。
- **[MED-3] 单源化违反：同一 basis/total/`attribute_total` 在同一 bar 内算两遍。** 新块 `fill.rs:5185-5195` 与既有 M2 诊断块 `fill.rs:5306-5312` **逐字重复**（`default_lot.max(1)`、`level_nets`、`p*.round()`、`attribute_total`）。注释称「同一函数同一 basis 单源」，实为**复制口径**非共享变量：无任何一行保证两者一致，改一侧另一侧不跟随，且两门同开时（overlay 臂就是该组合）逐 bar 白算一遍。
- **[MED-4] 门开路径零 CI 可执行证据；差异断言 ≠ 正确性断言。** `wverify_run.rs:2094-2099` 只断言「n_orders 不等 ‖ |Δpnl|>1e-9」，对「帽退化成无条件全平」「帽把方向裁反」一律放行——而 HIGH-1/MED-2 恰恰会让该断言**更容易变绿**。测试手上现成的 `reconcile_residual`、`level_attrib_n_residual_bars`、`n_rescaled` 三个不变量一个都没断言。
- **[LOW-1] 新增 unused import / dead_code 警告。** `coverage/mod.rs:138` re-export 的 `level_cap` 全仓无调用点；摘掉 `#[allow(dead_code)]` 后 `sizing.rs:246` `level_cap` 变 dead。自评④「0 error（仅既有警告）」中「仅既有」不实。
- **[LOW-2]「与 M2 诊断同源」表述有条件缺失。** 该诊断整段在 `if level_ledger.is_some()`（`fill.rs:5262`）内、仅 overlay 臂执行；M4 决策段无此门控，净额臂/声部臂同样生效 ⟹ 两臂上「有决策、无对应诊断读数」。
- **[LOW-3] `cap_units = cap.floor()` 与 lot 对齐。** `default_lot > 1` 时 `capped_total` 未必是 lot 整数倍，而 `standard_p_star` 是 lot 对齐的。kimi 同源缺陷，非本票新增，仅登记。
- **[LOW-4] 测试内静默吞 Err。** `wverify_run.rs:2085` `fs::write(...).ok()`，落盘失败无提示而「已落盘」的 eprintln 照打。仅测试路径。

## 8. 自评报告如实性核对

| 自评原句 | 核对结论 |
|---|---|
| L27「已核实该区间内 `order` 无其他写入点，插入点安全」 | **属实**（独立 grep 5141-5830，`order =` 仅新增那一处） |
| L31「与下方既有 M2 只读诊断读数**同一函数同一 basis**，未引入第二套归因口径」 | **不实**——是复制不是共享（MED-3） |
| L29-30「#351 MED『Σw 校验接线』，机器断言此前只是纯函数、未被任何调用点消费」 | **半实**——接了，但降级为 `debug_assert` 且在唯一 release 跑法上被编译掉，仍未在真实使用路径上被消费（MED-1） |
| L34「逐级二次裁剪」 | **措辞失真**——kimi 的「二次裁剪」专指两次 clamp 中的第二次，本仓只有一次（§2 ④） |
| L58「与票面裁定②『默认零变化+可验证』的验收要求相容，风险显著更低」 | **不成立**——「默认零变化」成立，「风险更低」被 HIGH-1 实测否证 |
| L92「证明帽在此紧配置下**大量 binding**」 | **未经测量的推断**（代码内无 binding 计数）；第三轮实测事后与之相容 |
| L93-94「测试内置断言已固化为回归锁（未来若接线被意外短路会立即变红）」 | **不成立**——`#[ignore]` + 需真实数据，默认跑批跳过，短路不会让任何 CI 变红 |
| L97「0 error（仅既有警告）」 | error 数属实；「仅既有警告」**不实**（LOW-1） |
| L100「停手项：无」 | **不成立**——HIGH-1/HIGH-2 的架构冲突未上报 |
| 条目 1（kimi 默认关订正） | **属实**，参照面逐字核过 |
| 条目 2 末「`cap_narrowed_levels` 消费链仍未点亮，如实登记」 | **属实且诚实** |
| §3① `cargo test --lib` +1 ignored | **属实**（140 这个数一致；2605 vs 2629 差值来自评审基点含 #747，与本票无关） |

## 9. 未能验证

1. **自评基线 2605 passed / 139 ignored**：评审基点 HEAD 已含 #747，未回退到 `ticket-755` 分支复现（需动工作区）。差值 24 与本票无关。
2. **门 off 相对 #755 之前的逐位实测对拍**：未跑「父提交 vs 合并后」同窗对拍（需第二个构建产物）。给出的是静态等价证明 + 全量测试绿。仓内亦无任何测试做这件事。
3. **`force_flat` / `no_increase_cap` 两条 gate 收窄路径下的越界**：本窗零触发，机制同源但未获直接观测。
4. **实测覆盖边界**：BTC 单标的、200k bar、`ThetaConfig::default()` + `q4_margin_model`、两组 `level_weights`。越界**次数**强依赖配置；越界**可能性**只依赖「gate 收窄 ∧ 级别净额异号」，后者本窗覆盖 38.7% 的门内 bar。

## 10. 建议（供裁决，非本轮授权）

1. **阻断项**：HIGH-1 修复后再合——覆盖 `order` 前把 `lee_capped_total` 投影回 gate 可行区间；或按 kimi 形状把 clamp 前移到 `pi_theta_step_*` 之前施加于 `level_nets`（后者同时解掉 HIGH-2 与 §2 ④ 的比例基准差异，且不需要动 M3，即冲突②遗漏的第三方案）。
2. HIGH-2：若维持账户层施加，须同步裁剪腿级账本，或显式登记「门 on 时腿级账本不可用」并让相关不变量断言能看见分裂。
3. MED-1：按 kimi 原意恢复 `assert!` 并钉回 clamp 函数入口。
4. MED-4 / §1 golden：照抄 `nest_gate_env_gate_off_bitexact_on_shrinks_only` 的形状，补一条合成数据、非 ignore 的门 off bit-exact + 门 on 只缩不增测试；断言中加入 `lee_capped_total ∈ gate 可行区间` 的逐 bar 检查。
5. HIGH-3：把三处悬空 doc 与冲突清单一并订正（#693② 要求的逐点清单）。
