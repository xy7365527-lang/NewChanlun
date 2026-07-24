# Gap-2 Γ_t 候选级 dump 设计稿（L3 前置，不实装）

- **工位**：worktree `/tmp/kimi-nest-mainline`，分支 `kimi-nest-mainline-20260717`
- **日期**：2026-07-19
- **性质**：**纯设计稿**——本工位只交付本文档，不改 `rust/`，不 git mutation，主仓只读。实装属后续授权工位。
- **纪律**：090（声明=能力，照实否定合格；本稿每条接入点均有行号锚，无锚不写）；v3 硬禁令（dump 是频数事实外化，不引入概率/统计推断作决策基础；LCB 求值是 χ 门自身定义的确定性演算，同 `l3-econ-gate-filter-rate-20260719.md` §0 先例，非新增推断；不回测验证策略）。
- **缺口陈述（为什么需要本 dump）**：`trades.jsonl` 只在**已开仓**路径落行（`OpsemDump::mark_entry` 于开仓分支内调用，`rust/src/theta_v0/backtest/runner.rs:1641-1643`；`write_trade` 全部 5 个调用点 runner.rs:1726/1764/1795/1827/1966 均在结算已开腿）。Γ_t 全候选集无任何外化 ⟹ 候选侧准入门（Nest/Xzd 在候选进 μ 桶前拒多少）与 χ 真实候选拒绝率**原理上不可测**（`l3-econ-gate-filter-rate-20260719.md` §4⑤b 明列此为「L0 前置缺口」）。本 dump 补该缺口。

## 0. 术语与口径边界（先声明，防 090 谎报）

- 本稿「Γ_t」= fill loop 每决策 bar 的 `step_gamma`——**本 bar 新确认**买卖点候选（确认-bar 部署，`runner.rs:1275-1278` 注释：非 source_index==i 切片）。v0 实装口径下 Γ_t 就是 newly-confirmed step candidates；它不是「全部历史活跃候选」。凡下游用本 dump 算过滤率，分母口径以此为准，不得外推为「全结构候选」。
- PanDiv 候选**不在** Γ_t：它走独立轨 `pan_candidates`（runner.rs:1450-1453），不经 `filter_gamma`。本 dump **不覆盖** PanDiv 通道——诚实缺席，不伪造并入。
- χ 只滤 `step_gamma` → `step_gamma_trade`（runner.rs:1363-1371），`step_work`（tree/candidates）不受 χ 影响（runner.rs:1360-1362 注释）⟹ **χ=Some 运行时 Γ_t 原集仍完整在 dump 中**，被拒候选可整行回收（这是本设计可测拒绝率的结构基础）。

## 1. ① dump 字段清单（每候选一行 JSON）

产物：`<dir>/gamma_candidates.jsonl`。两种 record，以 `"kind"` 区分（单文件、无第二写入器，最小改动）：

**kind="bar"（每决策 bar 一行，含空 Γ_t bar）**——分母诚实行：无此行则「该 bar 零候选」与「dump 缺口」不可区分（090 禁模糊）。

| 字段 | 类型 | 来源/口径 |
|---|---|---|
| `kind` | `"bar"` | 常量 |
| `bar` | usize | 决策 bar `i`（`!bar.untradable && px > 0.0` 分支内，runner.rs:1267） |
| `gamma_raw_count` | usize | `step_gamma.len()`（χ 滤前） |
| `gamma_trade_count` | usize | `step_gamma_trade.len()`（χ 滤后；chi=None 时 = raw，同 runner.rs:1370） |
| `chi_filter_active` | bool | `chi.is_some()`（同 runner.rs:1663 L1-P1 先例） |

**kind="candidate"（每候选一行）**：

| 字段 | 类型 | 来源/口径（代码锚） |
|---|---|---|
| `kind` | `"candidate"` | 常量 |
| `bar` | usize | 决策 bar `i` |
| `gamma_index` | usize | `Candidate.gamma_index`（Γ 原始序，`rust/src/theta_v0/strategy/interp.rs:86-87`；同 bar 内唯一，作行主键 (bar, gamma_index)） |
| `level` | u32 | `c.level`（interp.rs:73） |
| `source_index` | usize | `c.source_index`（interp.rs:75；与 trades.jsonl `certificate.source_index` 同键，可 join，runner.rs:2489） |
| `bsp_bits_class_index` | u8 | `c.bits.class_index()`（同 runner.rs:2487） |
| `dir` | `"Long"/"Short"/"Flat"` | `c.dir`，经 `voice_side_str`（runner.rs:2747 同款） |
| `bsp_class` | i64 | `c.bsp_class`；`u8::MAX → -1`（同 runner.rs:2488 先例，不伪造类号） |
| `nest_confirmed` | bool | `c.nest_confirmed`（interp.rs:85） |
| `nest_depth` | u8 | `econ_positive::structural_nest_depth(&tower_i, c.level, c.source_index)`（R5-c 同口径，runner.rs:1652-1654；**dump-only，不进 μ 桶键**，R5-1 铁律 runner.rs:2308-2311/2385-2386） |
| `role` | string | `operation_role_str(c.role)`（18 类，同 runner.rs:1655） |
| `chi_filter_active` | bool | `chi.is_some()` |
| `chi_evaluated` | bool | `c.dir != Flat && c.bsp_class != u8::MAX`——χ 门实际求值条件（selector.rs:372-373：Flat/无类候选 χ 跳过不滤）。显式分列防「chi_admit=true」被误读为「χ 放行」 |
| `admission_value` | f64 \| null | chi=Some 且 chi_evaluated ⟹ `est.mu_lcb(&z, z_alpha)`（`shrink_tau_sq=Some` 时 `mu_shrink`），z 经 `selector::z_of_candidate(c, &tower_i, bars, &ext_i)`（selector.rs:379-383 同函数同现场值）；n<2/空类 ⟹ null（`mu_lcb` 诚实返 None，`rust/src/theta_v0/backtest/mu_estimator.rs:468-473`）；chi=None 或 chi_evaluated=false ⟹ null |
| `theta` | f64 \| null | `ctx.theta`；chi=None ⟹ null（门参数随行走，防下游误用漂移口径对比） |
| `chi_admit` | bool | **成员关系判定**：`gamma_index ∈ step_gamma_trade`（不重跑 filter，见 §4②）；chi=None ⟹ 恒 true（χ≡1，`runner.rs:1370`） |
| `opened` | bool | **成员关系判定**：该候选 ∈ `step_trace.opened` 的候选分量（opened 元素即 `(c, leg)` 对，消费点 runner.rs:1573 `for (c, leg) in &step_trace.opened`），匹配键 = gamma_index |

缺席纪律（090）：任何不可得字段写 `null`，不编造（同 trades.jsonl「缺席字段标 null」原则，runner.rs:2373）。

## 2. ② 接入点（Γ_t 产生处与 dump 钩子位置）

Γ_t 在 `pi_theta_fill_loop` 主循环内的产生-过滤-消费链（全部行号为 worktree 当前 runner.rs）：

1. **产生**：`runner.rs:1347-1352`——`interp::coverage_elements_and_gamma_with_tower_cached_gen(&classification_step, &tower_i, …)` 返回 `(step_tree, step_candidates, step_gamma)`；`step_gamma` 即 Γ_t。
2. **χ 过滤**：`runner.rs:1363-1371`——`Some(ctx) → selector::filter_gamma_with_admission(&step_gamma, …)`；`None → step_gamma.clone()`（χ≡1）。过滤谓词本体在 `rust/src/theta_v0/backtest/selector.rs:357-388`。
3. **消费**：`runner.rs:1462-1476`——`coverage::pi_theta_step_traced(step_work, &step_gamma_trade, …)` 返回 `(next_active, standard_p_star, (order, _), step_trace)`。

**dump 钩子位置：`runner.rs:1476` 之后、`runner.rs:1477` PanDiv 段之前**，新增形如：

```
if let Some(gdump) = gamma_dump.as_mut() {
    gdump.write_step(i, &step_gamma, &step_gamma_trade, &step_trace, chi_ctx_view, &tower_i, bars, &ext_i);
}
```

选此位置的理由（逐条对照约束）：
- `step_gamma`/`step_gamma_trade`/`step_trace` 三者**同时**在手 ⟹ `opened` 标记可构（step_trace 仅在 1462 之后存在）。放在 1363-1371 之间则 opened 不可得，放在 1573 消费循环内则退化成「opened-only 顺带 dump 全集」的别扭结构且与 fill/结算纠缠。
- 与既有 opsem per-bar 钩子同构：`dump.diff_tower(i, &tower_i)` 于 runner.rs:1272-1274 同 loop 内 env-gated 调用——先例确立「per-bar 只读外化钩子插在决策段内」的合法位形。
- 在 PanDiv 段（1477+）之前 ⟹ dump 语义纯净：只含标准 Γ_t 通道，与 §0 的 PanDiv 诚实缺席声明一致。
- `chi` 上下文：`pi_theta_fill_loop` 参数 `chi: Option<ChiFilterCtx>`（结构体 runner.rs:973-987，含 `est/theta/z_alpha/treat_empty_as_pass/shrink_tau_sq`）。钩子内只做**只读**访问（`mu_lcb`/`mu_shrink` 均 `&self` 查询）。

`write_step` 内部逻辑（设计描述，非代码）：
1. 写一行 kind="bar"（counts 见 §1）。
2. 构造 `step_gamma_trade` 的 gamma_index 集合 `admitted`（线性扫一遍，O(|Γ|)）；构造 `step_trace.opened` 的 gamma_index 集合 `opened_set`。
3. 遍历 `step_gamma`（保序，`filter` 的 `.cloned().collect()` 保序保 gamma_index，selector.rs:368-387），每候选写一行 kind="candidate"；`chi_admit = admitted.contains(gamma_index)`，`opened = opened_set.contains(gamma_index)`。
4. `admission_value` 仅当 chi=Some 且 chi_evaluated 时求值：`z_of_candidate` + `mu_lcb`/`mu_shrink`（纯只读，见 §4③）。

## 3. ③ env gate：推荐独立 `OPSEM_GAMMA_DUMP_DIR`（方案 A）

**方案 A（推荐）**：新 env `OPSEM_GAMMA_DUMP_DIR=<dir>`，新 struct `GammaDump`（单 BufWriter，`from_env`/`at_dir`/空串拒绝/`#[cfg(test)]` 线程局部 override 四件套，逐条镜像 `OpsemDump` 先例：runner.rs:2413-2422 `from_env`（`.ok().filter(|s| !s.is_empty())`）、2427-2439 `at_dir`（截断打开）、2401-2408 `OPSEM_DUMP_DIR_OVERRIDE` 线程局部注入）。

理由：
- **独立对照重跑是⑤验证协议的硬需求**：「gamma dump 开 vs 关 ⟹ trades.jsonl 逐字节一致」的对照要求两者可独立开关；并入单 env 则只能同开同关，无法隔离扰动源。
- **体量与用途不同级**：trades.jsonl 是 opened-only（518 笔量级），gamma_candidates.jsonl 是每决策 bar × |Γ_t|（bar 行 + 候选行），体量大一到两个数量级；诊断会话常只需其一。
- **不动既有断言面**：`opsem_dump_env_gated_bit_exact`（runner.rs:4382-4434）对 `OPSEM_DUMP_DIR` 的语义断言（未设/空 ⟹ None、trades.jsonl 内容断言）保持原样零改动。

**方案 B（备选，未采纳）**：并入 `OPSEM_DUMP_DIR`，在 `OpsemDump` 加第三写入器。优点：少一个 struct 的样板（约 40 行）。代价：env 语义膨胀（原注释「开两个 JSONL 写入器」runner.rs:1111、2366-2375 全部要改口）、丧失独立对照能力、per-bar 大体量写入与 trades dump 强制绑定。若后续工位裁定统一开关更可取，方案 B 可行且不破坏 bit-exact（新增文件不改 trades.jsonl 字节），但需同步更新上述注释面——**按最小改动与本任务的验证需求，取 A**。

实例化点：`runner.rs:1113` `let mut opsem = OpsemDump::from_env();` 旁加 `let mut gamma_dump = GammaDump::from_env();`（同模式相邻声明，读码路径一致）。

## 4. ④ bit-exact 保证（未设 env ⟹ 逐字节不变；μ 桶键边界）

四条机制性保证，每条有先例锚：

**① env-gated no-op**：`OPSEM_GAMMA_DUMP_DIR` 未设或空串 ⟹ `GammaDump::from_env() → None` ⟹ 唯一调用点（§2 钩子）`if let Some` 不进入 ⟹ 生产路径零额外指令（同 runner.rs:1111-1113/2366-2368 承诺；空串拒绝同 runner.rs:2420 `filter(|s| !s.is_empty())`）。

**② 只读消费，无生产写入**：钩子只持 `&step_gamma`/`&step_gamma_trade`/`&step_trace`/`&tower_i`/`&ext_i` 不可变引用；不向 `LedgerOpen`/`TypedTrade`/`MuClass`/`MuEstimator` 写任何字段；**不调 `est.observe`**（μ 表零观测增量）。结构性免疫先例：TypedTrade 不含 opsem 字段 ⟹ set/unset bit-exact（runner.rs:4373-4374 注释）；同理 GammaDump 不触任何进 `typed_ledger` 的类型。`GammaDump` 实例本身只活于 fill loop 局部变量（同 `opsem`，runner.rs:1113），不进 `FillResult`。

**③ 派生值全部纯函数**：`z_of_candidate`（selector.rs:233-276，纯函数）、`mu_lcb`（mu_estimator.rs:468-473，`&self` 只读查询）、`structural_nest_depth`（纯结构读数，不依赖 hist，R5-c 先例 runner.rs:1652-1654/2383-2386）。无 RNG、无时钟、无哈希序依赖（遍历 Vec 保序，不遍历 HashMap）。

**④ `chi_admit` 用成员关系，不重跑门**：`chi_admit` 由 gamma_index ∈ `step_gamma_trade` 导出（selector.rs:368-387 `filter…cloned().collect()` 保序保 gamma_index，成员关系=门真值）。**禁止**在 dump 内重跑 `chi_t` 二次判定——那会制造第二裁决源，一旦与生产门漂移即伪造（090）。`admission_value` 是**诊断列**（门输入的外化），不参与 `chi_admit` 判定，也不回写任何生产状态——与 `lex_argmin_top3`「dump 专用，不进 p_star/J_Θ/χ」的 R5-1 铁律同款（runner.rs:2331-2335）。

**μ 桶键边界（R5-1 铁律逐字沿用）**：本 dump 全部字段**不进** `entry_z`/`MuClass`/μ 桶键/`J_Θ` 排序/χ 门控（铁律原文 runner.rs:2308-2311、2385-2386、2368-2369「纯只读外化」+ exit-μ-BUCKETING-FROZEN #180）。`nest_depth`/`admission_value` 等新列只是 JSONL 文本，类型层不触碰 `MuClass`（15 维定义 mu_estimator.rs:92-137 零改动）。

## 5. ⑤ 验证协议（实装工位的闸门，本文档预登记）

**A. cargo test（新增测试镜像 R5-1 先例，runner.rs:4382-4434）**：
1. `gamma_dump_env_gated_bit_exact`：`remove_var`/空串 ⟹ `from_env().is_none()` 两断言；合成闭包 `buy1_at3_confirmed_at7`（runner.rs:4362-4369 同款）跑两遍 `pi_theta_fill_loop`（unset vs 线程局部 override 注入），断言 `typed_ledger`/`n_orders`/`trade_pnls_with_forced` 三项 bit-exact（逐项 `assert_eq!`，同 runner.rs:4410-4415）。
2. `gamma_dump_schema_on_synthetic`：dump 启用跑合成例 ⟹ `gamma_candidates.jsonl` 存在；首候选行含 `"kind":"candidate"`、`"chi_filter_active":false`、`"chi_admit":true`（chi=None ⟹ χ≡1）、`"nest_confirmed"`、`"admission_value":null`；kind="bar" 行 `gamma_raw_count == gamma_trade_count`。
3. `gamma_dump_chi_consistency`：chi=Some 且买点 z 的 LCB(μ)<θ（μ 表构造镜像 `chi_filter_shrinks_trade_set_vs_full_coverage`，runner.rs:4442-4451 的 z 口径纪律：H=Some(First)/sigma_higher=Some(0)/账本三维同 fill loop 现场）⟹ 候选行 `chi_admit=false`、`admission_value` 非 null 且 ≤θ，且该 run `n_orders=0`——证明 dump 的 admit 真值与生产门一致（成员关系无漂移）。
4. **并行安全**：测试注入一律走 `#[cfg(test)]` 线程局部 override（runner.rs:2401-2408 先例：进程级 env 会被并行测试读到并 truncate 同一文件，2026-07-13 竞态实录）；每测试唯一目录（pid+nonce，runner.rs:4396-4404 同款）。
5. 闸门：实装后 `cargo test`（theta_v0 全量）+ `cargo build` 全绿，含既有 `opsem_dump_env_gated_bit_exact` 不红。

**B. OPSEM 重跑对照（真实数据，三腿）**：
1. **基线腿**：env 全不设，跑当前生产数据（如 m8 p3fold 窗口），与既有基线输出逐字节 diff ⟹ 一致（既不变量未被实装破坏）。
2. **隔离腿**：同输入跑两次——(a) 只设 `OPSEM_DUMP_DIR`；(b) 同设 `OPSEM_DUMP_DIR` + `OPSEM_GAMMA_DUMP_DIR` ⟹ 两次 `trades.jsonl` + `tower_events.jsonl` 逐字节一致（gamma dump 对既有通道零扰动）。
3. **确定性腿**：同输入设 `OPSEM_GAMMA_DUMP_DIR` 跑两次 ⟹ `gamma_candidates.jsonl` 逐字节一致（无 HashMap 序/时钟泄漏；f64 经同一格式化路径，同 write_trade 的 `format!` 拼串确定性，runner.rs:2468-2470）。

**C. 交叉对账（090 防伪造）**：随机抽 opened=true 的候选行，按 (level, source_index, dir) join 同 run `trades.jsonl` 的 `certificate` 三字段（runner.rs:2484-2494）⟹ 必须笔笔命中；反向 trades.jsonl 每行必在 gamma_candidates.jsonl 有 opened=true 候选。任一方向失配 = dump 与生产账本裂口，实装作废。

## 6. ⑥ 产出后的 L3 过滤率实测方案

拿到候选集后，三项此前不可测的量变为可测（全部为**频数事实计数**，非统计推断；门的输入值演算是门自身定义的确定性求值——v3 边界声明同 `l3-econ-gate-filter-rate-20260719.md` §0）：

**M1. Nest/Xzd 候选侧准入门过滤率（首次可测，直击 §0 缺口）**
- 运行形态：chi=None + `OPSEM_GAMMA_DUMP_DIR`（Γ_t 全集无外生过滤）。
- 测法：`count(nest_confirmed=false) / count(kind="candidate")`，按 `level`/`dir`/`bsp_class` 分桶计数。「若门=无 nest 证书不开仓」的候选侧拒绝率即此数。对照已知事实：opened 子集 nest_confirmed=518/518（m8 D7）只证明门对**已开仓者**常开；M1 给出全集分母，补 `l3-econ-gate-filter-rate-20260719.md` §4⑤b 明列的 L0 前置缺口。
- nest_depth 分桶分布（0..4）同步产出，供「nest 三维进 μ 桶键」的稀疏代价预估（桶数 × 每桶预期 n），对接 `oos_gated_drop` 评估（mu_estimator.rs:694）。

**M2. χ 真实候选过滤率（替代静态边际投影）**
- 运行形态：chi=Some（walk-forward μ 由 L3 工位供；in-sample μ 只证选择器逻辑、属泄漏，runner.rs:970-971 诚实声明沿用）+ `OPSEM_GAMMA_DUMP_DIR`。
- 测法：`count(chi_admit=false ∧ chi_evaluated=true) / count(chi_evaluated=true)`，按 (level, dir, bsp_class, nest_depth) 分桶。这是**因果 μ 下逐 bar 演化**的真实拒绝率——替代 §3.1 那种「静态全样本 μ 边际投影」的下界估计（该报告 F2 已证真实值 ≥ 边际投影，点估【待实测】即由此闭环）。
- `admission_value` vs `theta` 的距离分布（分桶直方计数）诊断 F1「罚项淹没 mean」在全集上的形态；`chi_evaluated=false`（Flat/无类）候选占比单列——它们 χ 不滤，属 interpret 管辖。

**M3. 三道门过滤率分解（χ / ConflictOK / RiskOK 各自份额）**
- 交叉表 `chi_evaluated × chi_admit × opened` 八格计数：
  - `chi_admit=false` ⟹ χ μ 门拒（份额 A）；
  - `chi_admit=true ∧ opened=false` ⟹ 过了 μ 门但被下游拒——ConflictOK（interpret 规则2/3/4 归 𝒦_x）或 RiskOK（k_theta_risk_gate 收窄 𝒦_Θ），两机制在 selector.rs:308-318 已声明为「下游已存在」；份额 B 即此前完全不可见的「非 χ 拒绝」总量（区分到 ConflictOK vs RiskOK 需 interpret 侧记录，**本 dump 不分解 B 的内部**——照实声明能力边界，需另开工位）；
  - `opened=true` ⟹ 全链通过（份额 C，与 trades.jsonl 笔数对账，§5-C）。
- γ=0 bar（kind="bar" 且 gamma_trade_count=0）计数 ⟹ 「χ 致空仓 bar」频率，回答 `l3-econ-gate-filter-rate-20260719.md` §3.3 F3 的「分布左移」推演。

**边界与禁令（照实声明）**：
- dump 交付**当次运行**的 admit/opened 真值与门输入值。换 (θ, z_alpha, treat_empty_as_pass) 的假设性过滤率重放需要 walk-forward μ 的逐 bar 演化表——**本 dump 不含 μ 历史**（est 是外部注入的引用），离线重放属 L3 工位另行设计，本 dump 不伪造该能力。
- 不测任何「若当时开了仓会赚多少」的虚拟盈亏——那是回测策略验证，v3 硬禁令。
- 过滤率本身是结构频数；其经济含义（该不该滤）须经 L3 裁定链，不由本 dump 数字自证。

## 附：锚点索引

- 代码锚（worktree `/tmp/kimi-nest-mainline`，行号为 2026-07-19 当前值）：`rust/src/theta_v0/backtest/runner.rs:1111-1113`（opsem 实例化）、`:1267-1278`（决策 bar 分支/新确认候选口径）、`:1347-1352`（Γ_t 产生）、`:1363-1371`（χ 过滤）、`:1462-1476`（消费+step_trace）、`:1573`（opened 消费）、`:1602-1604`（B1 注释区，任务书原锚）、`:1641-1654`（mark_entry/nest_depth 读数）、`:1661-1663`（gamma_count/chi_filter_active 先例）、`:1726/1764/1795/1827/1966`（write_trade 调用点全集）、`:2284-2361`（OpsemEntrySnapshot 字段先例）、`:2363-2439`（OpsemDump 谱系注释/from_env/at_dir）、`:2401-2408`（线程局部 override）、`:2468-2500`（write_trade 拼串/lex_top3 字段）、`:4371-4434`（R5-1 bit-exact 测试）、`:4442-4451`（χ 收缩测试）；`rust/src/theta_v0/backtest/selector.rs:308-321`（χ 三机制合取声明）、`:334-388`（filter_gamma/_with_admission）、`:233-276`（z_of_candidate）；`rust/src/theta_v0/backtest/mu_estimator.rs:468-473`（mu_lcb）、`:92-137`（MuClass 15 维）、`:694`（oos_gated_drop）；`rust/src/theta_v0/strategy/interp.rs:71-93`（Candidate 全字段）。
- 文档锚：`chanlun/review-results/l3-econ-gate-filter-rate-20260719.md` §0（方法边界/LCB 纪律声明）、§4⑤b（候选级 dump = L0 前置缺口）、§3.3 F3（γ 分布推演待实测）。
- 未做（本工位边界）：未实装、未改 rust/、未 git mutation、未跑 cargo（无代码改动可验）；实装与 §5 闸门的执行属后续授权工位。
