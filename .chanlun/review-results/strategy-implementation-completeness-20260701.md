# 缠论策略实装完整性审计（spec-execution-gap / Task #119）

**日期**：2026-07-01
**审计对象**：econ_positive.rs 的 `decompose_capturable_spread`（生成 12626 信号台账 `/tmp/btc_663_ledger_sigma.csv`）所依赖的完整策略实装链。
**任务性质**：不是找 bug，是划"完整缠论策略 vs 我们检验的实装"的有效域边界。
**认识论标注**：本审计 = **L0/L1 代码事实核验**（读代码确认实装状态，非 L2 实证有效性）。审计结论作用于"我们的 alpha 检验覆盖了哪个策略子集"这一元问题。

---

## 结论

**回测台账（12626 信号）走的是 classifier 识别层（`cls_i.levels[].bsp`），不是 closed_loop Θ 闭环层。** 这一区分是全部审计的关键。两个层的实装完整度不同：

- **识别层（econ_positive 实际用的路径）**：买卖点三类**全部实装**（buy1/2/3, sell1/2/3），力度轴（MACD 背驰）**真消费**。二类买卖点有一个**结构窗口有效域上界**（L0→L1 三段窗口不产二类，需 L1→L2 几何路径）。
- **闭环层（Lean parity 用的路径，回测不走）**：只覆盖第一类/第三类/延续三态，第二类闭环 still-MISSING。

**我们的"无 alpha / inconclusive / 夏普证伪"结论覆盖的是：全体 bsp 端点（三类买卖点混合）转成的方向信号集，出场口径 = 口径4（下一反向确认信号）。** 覆盖的是"完整识别层策略"的一个**方向投影**——类型信息（一/二/三类、买 vs 卖）在信号池中被抹除，无法归因到具体类型；且二类只在结构窗口满足时才进入信号集。

---

## 逐条证据

### 1. 买卖点三类完整性

**识别层（econ_positive 用）：三类全实装。**

| 类型 | 判据实装 | 证据 |
|------|---------|------|
| 一类买/卖 | ✓ | `bsp.rs:56 is_first`（中枢下方∧无前一买∧未离开）；一类=破中枢几何∧MACD背驰真算（mod.rs:240）|
| 二类买/卖 | ✓（有窗口上界）| `bsp.rs:60 is_second`（一买后∧回调结束）；`extract_second_for_level`（mod.rs:1136-1165）经 RMove 递归塔真产 |
| 三类买/卖 | ✓ | `bsp.rs:64 is_third`（离开中枢∧回试不入）；`signal::extract_signals`（mod.rs:247）|

- `endpoint_to_bsp`（bsp.rs:76-88）逐类独立置位，六类全覆盖，2B/3B 可共存（非互斥，`no_exclusive_trichotomy`）。
- 信号方向映射：`conf_plus = buy1||buy2||buy3`、`conf_minus = sell1||sell2||sell3`（types.rs:202-211）——**三类都触发方向，没有类型被丢弃**。`candidate_dir`（interp.rs:206）用 conf_plus/minus 派生 Long/Short。

**二类的结构有效域上界（不是缺陷，是结构上界）**：
- `extract_signals`（L0 层）在任何输入上产 0 个二类（signal.rs:971-972 断言锁定）。
- 二类由 `extract_second_signals` 在**递归组装层**产（消费 RMove 塔）。需要 ≥3 个同向 L1 走势经几何窗口 compose 成 L2 走势，其 descend 取回的走势内识别"第一类离开 + 回拉不创新低"（mod.rs:1254-1261, end-to-end 测试 `end_to_end_second_buy_via_l1_l2_geometric`）。
- **still-MISSING-窗口**：L0→L1 固定三段交替窗口结构上不产 B2/S2（mod.rs:1330-1331 断言，codex 裁决坐实）。故二类信号数依赖行情是否形成 ≥3 同向 L1 走势的 L2 结构。

**闭环层（回测不走，但 Lead 背景提到的 sell.rs:35）：第二类闭环 still-MISSING。**
- `closed_loop/sell.rs:35`、`buy.rs`（第二类买点条目）明确标注：第二类闭环覆盖缺——"定律一「由次级别一类构成」需次级别递归"。
- 但这是 **Lean parity 的账本闭环**（`sell_transition`/`buy_transition` 驱动 R=Π-A-W 账本），**不是 econ_positive 回测的信号生成路径**。descend.rs 已 port 次级别下钻（`second_type_via_sublevel_type1`），但 `center_of` 自动中枢分配未实装（descend.rs 诚实边界）。

**关键 gap**：台账 `SignalDecomp` 只保留 `delta`（±1），**丢弃了 min_class（类号）和买/卖侧**（interp.rs:214 `min_class` 算了但下游不透传）。台账 CSV 列头无类型列（entry_bar/exit_bar/level/delta/a_b/.../sigma_higher）。⟹ **我们的 alpha 检验无法区分一/二/三类，也无法区分买点 vs 卖点** —— 只能看方向 delta。

### 2. 背驰/力度轴实装

**力度轴真在信号生成路径消费，不是纯几何。**

- `divergence::compute_macd`（mod.rs:211）计算全序列 MACD hist。
- 一类：`extract_signals` 传 `config.macd`（mod.rs:247），一类=破中枢∧MACD背驰（mod.rs:240）。
- 二类：`sublevel_diverges`（mod.rs:1176-1200）用 `divergence::segments_diverge` 真算次级别走势面积比较。
- 卖点第一类：`is_type1_sell` = short∧破中枢∧`is_divergence`，`is_divergence` 承载 MACD 真算 bool（sell.rs:64-66 注释"rust 在此领先 Origin：背驰由已实装 MACD 真算"）。

对照 ChatGPT 的 ZigZag 论断："ZigZag 缺力度轴不出奇偶交替，缠论出=有力度"——**核实成立**：缠论识别层真用 MACD 力度，非纯几何 pivot。（注：这不推翻 μ̂ 奇偶交替=beta漂移伪结构的判决——那是反事实置换否证的独立 L2 结果。）

### 3. 中枢/走势级别递归完整性

**识别链完整，走势方向是端点价法近似（有效域受限）。**

- 实装链：`stroke → segment → center.rs（中枢）→ descend.rs（走势下钻）→ recursive_tower.rs（多级别塔）` 全部到位（classifier/ 目录文件齐全）。
- `IncrementalClassifier`（incremental.rs）逐 bar 因果分类，产多级别 tower。
- **走势方向近似（econ_positive 声明的 L2 边界）**：`sigma_higher_at` 用**上级 LeveledMove 的 start_index/end_index close 净差符号**近似"上级走势方向"（econ_positive.rs:74-79）。原因：`RMove::Compose` 无 direction 字段，`MoveKind::Trend` 丢方向（Up/Down 不分），`MoveOutcome::Trend(Direction)` 内部有方向但信号收集作用域不可读。**这是端点价法，不是完整走势方向裁决**——注释明确标注为"该作用域的严格可达解"，与 Trend(Up)⟺端点净涨语义等价但非同一物。

### 4. 出场口径

**口径4 = 下一反向确认信号出场。是回测配对口径，不是缠论正规出场（如背驰卖点/中枢破坏）。**

- econ_positive 退出配对（econ_positive.rs:211-256）：持有到"首个 eb>entry_bar 的反向新确认信号"，ρ_rev 取该配对出场信号的 pivot 端点。无配对出场信号 ⟹ 右删失诚实跳过（`n_unpaired`）。
- **这是"反向信号翻仓"配对，不是缠论正规出场判据**。缠论正规出场（顶背驰卖点清仓 = closeRoot、中枢破坏减核 = reduceCore）**在 closed_loop 层已实装**（sell.rs `sell_transition`），但 econ_positive **不消费 closed_loop**——它自己用反向信号配对做出场。
- ⟹ 出场时机由"下一个反向 bsp 何时出现"决定，不由"是否顶背驰/中枢破坏"决定。虽然反向信号本身多为背驰卖点（因识别层用力度），但**配对逻辑本身是几何时序配对，不是 Θ_risk 的正规缠论出场**。

---

## 有效域边界（对照 spec）

**已完整实装（alpha 结论 L2 可信覆盖）**：
- 三类买卖点识别（一/二/三，买卖对称）+ MACD 力度轴 + 多级别中枢/走势递归塔。
- 方向投影（bsp → Long/Short）+ 口径4 出场配对。

**残缺/简化/未实装（有效域受限）**：
1. **类型归因缺失**：台账丢 min_class + 买卖侧，alpha 检验无法按一/二/三类或买/卖分层。
2. **二类结构窗口上界**：L0→L1 三段窗口不产二类，二类信号数依赖 L1→L2 几何结构形成——低级别行情二类稀疏。
3. **上级走势方向 = 端点价法近似**（sigma_higher），非完整 `MoveOutcome::Trend(Direction)` 裁决。
4. **出场 = 反向信号配对（口径4）**，非缠论正规出场（背驰卖点/中枢破坏）；closed_loop 有正规出场但回测不走它。
5. **closed_loop 层第二类闭环 still-MISSING**（但不影响回测——回测走识别层）。
6. **descend `center_of` 自动中枢分配未实装**（闭包参数，闭环层边界）。

---

## 明确结论（回答编排者元质询）

**我们的"无 alpha / inconclusive / 夏普证伪"覆盖的是完整缠论识别策略的一个方向投影子集，不是完整缠论操盘策略。**

具体不在检验范围内的机制：
- **类型分层 alpha**：一类 vs 二类 vs 三类的独立表现、买点 vs 卖点的独立表现——台账丢类型，无法检验。有可能某一类型有 alpha 而被混合信号池稀释。
- **缠论正规出场**：背驰卖点清仓 / 中枢破坏减核（closed_loop 已实装）——回测用反向信号配对替代，出场时机口径不同。
- **上级走势方向的完整裁决**：sigma_higher 是端点价法近似，若完整走势方向裁决与端点净差不一致，σ_higher 条件化结论的有效域受此近似限制。

**未削弱结论的部分**：三类买卖点识别本身完整、力度轴真用、多级别递归到位——所以"全体 bsp 方向信号 + 口径4"这个策略实例的检验是可信的 L2。**最大的 gap 是：这个检验对象是全类型混合的方向投影，无法回答"是否某个具体买卖点类型有 alpha"。**

---

## 结果包六要素

1. **结论**：见上"明确结论"——检验覆盖识别层三类买卖点的方向投影 + 口径4，不覆盖类型分层 alpha、缠论正规出场、完整走势方向裁决。
2. **定义依据**：bsp.rs:56-88（三类判据 endpoint_to_bsp）、types.rs:202-211（conf_plus/minus 三类并入方向）、mod.rs:240-252（力度真算 + 二类递归提取）、econ_positive.rs:211-256（口径4 出场配对）、econ_positive.rs:74-79（sigma_higher 端点价法）、closed_loop/{buy,sell}.rs 头部（闭环层二类 still-MISSING 但回测不走）。
3. **边界条件**：若 SignalDecomp 透传 min_class + 买卖侧（interp.rs:214 已算），则可做类型分层 alpha 检验，结论有效域从"方向投影"扩至"类型分层"。若 econ_positive 改走 closed_loop 正规出场，则出场口径从口径4 变为背驰/中枢破坏，可能翻转部分 spread_eaten 结论。
4. **下游推论**：候选G=∅ / inconclusive / 夏普证伪等结论应加限定词"（全类型混合方向投影，口径4 出场）"。若要主张"完整缠论策略无 alpha"，需补类型分层检验 + 正规出场口径对比——当前证据不支持从子集结论外推到完整策略。
5. **谱系引用**：[[project_oddeven_mu_identity]]（μ̂奇偶=beta漂移伪结构，L2 独立否证，本审计不推翻）；[[project_signal_layer_bsp_coverage]]（BSP 升跌完备性=100%）；b2s2-still-missing-tower（二类窗口上界谱系，signal.rs:257 引）；[[project_stheta_v1_fullwindow_l3_falsified]]（S_Θ 全窗 L3 否证）。econ_positive 664 号对象错配修复（δ≠ε）已在代码内谱系化。
6. **影响声明**：本审计不改任何生产代码，只产 gap 清单。影响的是**所有基于 /tmp/btc_663_ledger_sigma.csv 的 alpha 结论的有效域声明**——需在这些结论上附"方向投影子集"限定。指出一个可选的严格化路径（透传 min_class 做类型分层），但不实施（属新工位）。
