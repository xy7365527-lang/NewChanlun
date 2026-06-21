# unn 引擎为何 7/8 跑不赢 BH —— 综合交易行为分析

> 任务（编排者 2026-06-16）：详析为什么 7/8 标的没跑赢 buy-and-hold。
> 数据源：`trading_system/data_cache/unn_stream_{8标的}.json`（当前 working-tree，Jun16 12:47–13:08 落盘）
>          + `git show e2a78b0dd7:` commit 快照对照。
> 认识论等级：**L3**（8 标的真实 1min/databento+HL 数据；bit-exact vs 批量；跨两套快照交叉验证）。
> 既有诊断引用：`unn_trading_behavior_analysis.md`（BTC/ES 逐笔）、`why_not_profitable.md`（级别×摩擦）、
>                `qqq_root_short_no_costreduction_diagnosis.md`（T8 叶节点 + C located 天花板）。

---

## 0. 数据口径警告（必读，否则结论失锚）

**用户引用的「commit e2a78b0dd7」表格无法对应到任何已落盘的 JSON。** 三套数字互不一致：

| 标的 | 用户表 strat / BH | commit `e2a78b0dd7` 实际 JSON | 当前 working-tree JSON |
|------|-------------------|-------------------------------|------------------------|
| OKLO | +95.3 / 2492.9 | **+430.7 / 307.1（PASS）** | +256.8 / 307.1 |
| QQQ | +49.6 / 901.8 | —（commit 无 QQQ seg 数据展开） | **+176.9 / 174.6（PASS）** |
| BTC | +159.6 / 1414.0 | +12.0 / 150.8 | +136.9 / 1380.4 |
| GC | +52.6 / 113.4 | — | +227.7 / 257.3 |
| ES | +96.5 / 304.9 | +423.6 / 594.3 | +218.2 / 594.3 |
| DX | +28.9 / 29.5 | — | **+6.0 / 4.1（PASS）** |
| CL | +362.7 / 521.1 | −4.7 / 38.3 | −4.7 / 28.2 |
| BRN | **+332.6 / 60.4（PASS）** | +51.0 / 87.4（fail） | −28.6 / 87.4（fail） |

**三套数据连 BH 基准都不同**（OKLO BH 307.1 vs 用户表 2492.9，差 8 倍；BTC BH 150.8 vs 1380.4 vs 1414）——用户表是**更长时段或不同复权的第三套回测**，已被后续重跑覆盖。

**关键诚实声明**（no-workaround / no-patch）：
- 用户表声称「BRN +332.6 唯一 PASS」，但在我能 `git show` 验证的两套数据里 **BRN 都是 fail**（commit +51.0 / worktree −28.6）。
- 用户表声称「OKLO 严重踏空 +95.3 / 2492.9」，但 commit 里 **OKLO 是唯一 PASS（+430.7）**。
- 因此下文「BRN 为何是赢家」只能给出**机制性假说**（基于 539 号 regime 谱系），并标注其 **L2-fragile / 跨快照不可复现**。

**跨三套数据稳定的，是结构机制，不是具体数字。** 下文分析锚定在可验证的两套 JSON，结论对三套都成立的部分明确标注。

---

## 1. 引擎的结构本体：单根树 + 一笔 root 多头 + segment 子空头

8 标的、两套快照，`nrf_counters.root_entries` 全部 = `[0,0,0,1,0,…]` ——
**每个标的全程只有 1 次根入场，且恒在 ladder3 = move(L1) 涌现层**。入场后：

| 事实 | 证据（worktree） | 含义 |
|------|------------------|------|
| 根多头几乎全程持有 | `phys_long_bars/n_bars` = 93.8%–99.9%（8 标的） | F 正确建仓且不离场 |
| 唯一大额正 alpha = 最高涌现层 long 腿 | recL3/recL4 `/long` pnl：OKLO +307984、BTC +211973、ES +236093、GC +217999… | 整个策略收益 ≈ 一笔 buy&hold |
| segment 子空头几乎全负 | worktree seg/short `wins=0`：OKLO −92854、BTC −206757、ES −87837、CL −22480、BRN −101215 | E 降成本在趋势标的纯失血 |
| C 翻转极罕见 | `flips` 求和：OKLO/QQQ/BTC/DX/CL = **0**；GC/ES/BRN 仅 2–4 | C 几乎从不触发 |

**引擎 = "一笔根多头 buy&hold" − "segment 子空头亏损" − "units 稀释"。** 这是理解所有 fail 的总公式。

---

## 2. 7/8 跑不赢 BH 的结构性原因（三层，跨快照稳定）

### 2.1 第一层：root long 腿本身 ≤ BH（天花板低于 BH）

策略的全部 alpha 集中在最高涌现层那**一笔** long 腿。它注定 ≤ BH，因为：
1. **晚入场**：root 在 move(L1) **涌现后**才建仓（depth_bars 显示涌现前的等待），BH 从第 0 bar 起算；
2. **单根树**：整个走势塔只有 1 个 root，没有在不同级别叠加多笔多头去放大暴露——天花板被钉死在"一笔持有"。

即使子空头零亏损、零稀释，strat 也只能 **≈ BH**（QQQ r=1.01、DX r=1.46 就是这个上限）。**引擎结构上无法系统性超越 BH，只能逼近。**

### 2.2 第二层：segment 子空头是 fail 的充分条件

判别量极其干净（worktree，`spawn_seg` = segment 层 E spawn 次数）：

| 分组 | 标的 | seg 子空头 pnl（笔, wins） | P1 |
|------|------|----------------------------|-----|
| **spawn_seg > 0** | OKLO(36,0) BTC(89,0) ES(22,0) CL(10,0) BRN(14,0) | 全亏 wins=0 | **全 fail** |
| **spawn_seg = 0** | QQQ(0) DX(0) | 无子空头 | **PASS** |
| spawn_seg = 0 但 recL2 子空头小赚 | GC(0, recL2×8 +3910) | 近平 | fail（r=0.88，温和） |

**只要 segment 子空头被触发（spawn_seg>0），该标的必然 fail。** 反之 QQQ/DX 因成本门把 segment 子空头**全部拒绝**（`cost_rejects[2]`=5/24，spawn=0）→ 退化为纯 root 持有 → ≈BH → PASS。

→ **成本门（cost-gated spawn）是否挡住 segment 子空头，决定 P1 成败。** 这是操作层而非信号层/会计层的问题（印证 `why_not_profitable.md`：瓶颈在操作层）。

### 2.3 第三层：units 稀释深度 = `nrf_max_children`（跨快照稳定判别量）

每次 E spawn 子空头，`voices[root].units -= m`——根多头的 units 被借走做空。同时活跃的子声部越多，root 暴露被稀释越重。`nrf_max_children` 直接度量稀释峰值：

| 标的 | max_children | worktree r=strat/BH | commit r=strat/BH |
|------|--------------|---------------------|-------------------|
| **BTC** | **80** | **0.10（最踏空）** | **0.08（最踏空）** |
| OKLO | 36 / 6* | 0.84 | PASS（*commit=6） |
| ES | 22 / 3* | 0.37 | 0.71 |
| BRN | 15 / 3* | −0.33 | 0.58 |
| CL | 11 | −0.17 | −0.12 |
| GC | 2 | 0.88 | — |
| QQQ | 1 | 1.01（PASS） | — |
| DX | 1 | 1.46（PASS） | — |

**BTC 两套数据 max_children 都 = 80 → 两套都最踏空（r=0.08/0.10）。** 稀释深度是跨快照稳定的踏空预测量：
`max_children ↑ ⟹ root 暴露被借走越多 ⟹ 主升浪吃不满 ⟹ 踏空越深`。
（*OKLO/ES/BRN 两套 max_children 不同，正是它们 r 在两套间波动大的原因——见 §4 BRN。）

---

## 3. OKLO / BTC 踏空机制：不是 F 漏建仓，不是 C 翻错，是 E 子空头

针对用户的三选一（确认滞后？E spawn 空头亏钱？翻转被打？）——**逐项排除，真凶是 E**：

| 假设 | 证据（BTC worktree） | 裁决 |
|------|----------------------|------|
| **F 漏建仓 / 确认滞后** | root_entries=1，phys_long_bars=4501350（97.3% 全程持有），recL4/long +211973 | ❌ F 正确，全程持有 |
| **C 翻错（翻转被打）** | `flips`=0，exit_reasons 无 flip_long/flip_short。**C 从未触发** | ❌ C 根本没翻 |
| **E spawn 空头亏钱** | **segment/short 89 笔 wins=0 全亏 −206757，全部 liq（89 次强平），max_children=80** | ✅ **真凶** |

**BTC 机制**：root 多头 +211973，几乎被 segment 子空头 −206757 **完全抵消**——89 个子空头在史上最强单边牛（BH +1380%）里做空，全部亏到强平，且峰值 80 个同时借走 root units，主升浪暴露被稀释到只做出 r=0.10。

**OKLO 机制**（worktree）：root 多头 recL3/long +307984，segment 子空头 36 笔全亏 −92854 全部 liq → 净拖低 ≈50pp，r=0.84。比 BTC 轻，因为 max_children=36 < 80（稀释浅）。

### 3.1 C 不翻是对的（符合 T50 标度律）

C `flips`≈0 **符合螺旋推论 T50**（操作频率径向标度律 `f(k)∝λ^{−k}`：清仓/翻转在最外圈罕见，缠师"十年 1-2 次"）。强单边牛里 root 涌现到 recL3/recL4，源层 type1 顶背驰 + 完整 located 链的翻转条件在最高级别从未满足。**问题不在 C（高层正确地不动），在 E（低层 segment 过度活跃做空）。**

### 3.2 已翻空标的的另一种死法（QQQ 诊断，跨快照可变）

`qqq_root_short_no_costreduction_diagnosis.md` 记录的是**另一套更早快照**里 QQQ 翻空后的死法：root 翻空 @recL2 后，8 个 type1 买点期间 **E 被 T8 叶节点短路**（`is_root && Short ⇒ continue`）、**C 被 located 买链天花板挡住**（买 located 链最高只到 move(L1)=3，从不到 recL2=4）→ 裸空头暴露至 +100% 涨幅强平。
但**当前 worktree QQQ 根本没翻空**（flips=0，纯 root 多头 +177009 PASS）——说明 QQQ 在快照间行为已变。两种死法（裸空头强平 vs 子空头稀释）都指向同一根：**空头侧操作在上行 regime 是负贡献**（539 号有效域）。

---

## 4. "BRN 为何是赢家" —— 不可验证 + regime 机制假说

**用户表声称 BRN +332.6 唯一 PASS（BH 60.4）。但这无法在任何已落盘 JSON 复现：**
- commit `e2a78b0dd7`：BRN **+51.0 fail**（seg/short 5 笔 −330，move(L1)/short 57 笔 +7938 小赚，recL3/long +44924，max_children=3）；
- worktree：BRN **−28.6 fail**（seg/short 14 笔全亏 −101215，recL2/short −7782，max_children=15）。

### 4.1 机制假说（若 BRN 某次确曾赢）

BRN（布伦特原油）是 8 标的中 **BH 最低的品种**（60.4–87.4%，vs 股票 175–2493%），**regime 偏震荡/下行**。这使空头侧操作（segment 子空头、move/short、flip 翻空）从负贡献**有机会转正**：
- commit 快照里 BRN 的 **move(L1)/short 57 笔赚 +7938（26 胜）**——空头在油价回落段确实赚钱；
- 这正是 539 号 / `project_t14_t5_root_flip_necessity` 的有效域：**根翻空 / 子空头降成本有效域 ⊂ 非上行 regime**。BRN 是 8 标的里唯一足够"非上行"的，让空头操作有正域。

用户表那次 BRN 之所以能 +332.6 > BH 60.4，**唯一可能的机制 = 该时段 BRN 子空头/翻空净赚**，叠加 root 多头，在温和 BH 上做出超额。这与 CL（同为油、震荡/下行）在用户表里也 +362.7 的方向一致——**油链是 unn 空头操作的正域候选**。

### 4.2 但它极度不鲁棒（L2-fragile）

同一个 BRN，仅 confirm 机制 / 成本门参数改动，segment 子空头从 commit 的 −330（5 笔）暴增到 worktree 的 **−101215（14 笔全亏）**，max_children 从 3 → 15，strat 从 +51.0 崩到 −28.6。
**BRN 的"赢"对参数极度敏感**（既有 `unn_trading_behavior_analysis.md` §5 已记录：confirm 机制改动让 spawn 暴增 5.7× 放大踏空）——这是**有效域读数**（formalization-validity-domain），不是稳健 alpha。

### 4.3 可验证数据里真正的赢家（两条不同的"避免踏空"路径）

| 快照 | PASS 标的 | 机制 |
|------|-----------|------|
| commit | **OKLO（+430.7）** | 空头侧那次**赚钱**：recL2/short +35435、segment/short +7204（regime 配合） |
| worktree | **QQQ（+176.9）、DX（+6.0）** | 空头侧**零触发**：spawn_seg=0，纯 root 持有，零稀释 |

**两条路径殊途同归**：要么 segment 子空头赚钱（regime 配合，OKLO commit），要么根本不做空头（成本门全拒，QQQ/DX）。**segment 子空头亏损是踏空的唯一来源——它赚（OKLO commit）→PASS、它不触发（QQQ/DX）→PASS、它亏（其余 5 标的）→fail。** 这是跨两套可验证数据都成立的核心结论。

---

## 5. 结果包六要素

1. **结论**：unn 引擎结构 = "一笔 move(L1) 涌现的 root 多头 buy&hold − segment 子空头亏损 − units 稀释"。
   跑不赢 BH 有三层结构原因：(a) **天花板**——单根树唯一 long 腿 ≤ BH（晚入场+无多笔叠加），最好只能 ≈BH；
   (b) **充分条件**——只要 `spawn_seg>0`（segment 子空头触发），该标的必 fail（worktree 5/5 全亏 wins=0）；
   (c) **稀释深度**——`nrf_max_children` 跨快照稳定预测踏空（BTC 两套都=80→两套都 r≈0.09 最踏空）。
   OKLO/BTC 踏空真凶 = **E 在强上行 regime 过度 spawn segment 子空头**（BTC −206757 几乎抵消 root +211973），
   **不是 F 漏建仓**（phys 97.3% 全程持有）、**不是 C 翻错**（flips=0，C 从未翻，符合 T50）。

2. **定义依据**：
   - F 建仓（第11环+第23环恒仓，`unified_necessity.rs`）：root_entries=[0,0,0,1,…] @move(L1)，phys_long_bars 93.8–99.9%。
   - E 降成本（第17环+第22环 N7，`try_spawn_cost_gated`）：spawn_seg + cost_rejects 决定 segment 子空头数量；
     N7 用自层 nf 不查 located 链 → 强上行 regime 无对冲价值，做空全亏。
   - C 翻转（第14环 T14）：flips≈0 符合 T50 标度律（清仓最外圈 `∝λ^{−K}` 罕见）。
   - 成本门（cost-gated）：QQQ/DX `cost_rejects[2]`=5/24 全拒 segment spawn → 零稀释 → PASS。

3. **边界条件**（结论翻转条件）：
   - **regime 转震荡/下行**：segment 子空头 + flip 翻空变正贡献（OKLO commit space/short +7204、BRN commit move/short +7938）→ "踏空"翻转为"空头有效"。油链（CL/BRN）是最可能的正域。
   - **成本门收紧到全拒 segment spawn**：所有标的退化为纯 root 持有 → 全部 ≈BH（消除 fail，但也放弃任何超额）。
   - **多根树 / 多级别叠加多头暴露**：突破 §2.1 天花板——但这是新轴（当前单根树无此能力）。
   - 若实装 regime 门控（上行时抑制 E segment spawn）后 P1 仍未改善，则"segment 子空头是踏空源"被否证。

4. **下游推论**：
   - unn 有效域 = **震荡/双向 regime**；强单边牛是结构弱域（空头侧操作全负，BTC r=0.10）。与 `project_nrf_v4_strict_accounting`（正域翻转牛市侧需"不清仓+配额释放"）、`project_constitutive_throughput_falsified`（清仓频率=regime 函数，不可约）一致。
   - **E segment spawn 需 regime 门控才能跨 regime 普适**——开放轴：`nf_sell` 触发 E 前加上行判别（root 涌现层方向=Up 时抑制 segment 子空头）。
   - **P1 跨快照不稳定本身是一个发现**：同标的（BRN +51.0→−28.6、OKLO +430.7→+256.8）随 confirm/成本门参数大幅漂移 = alpha 不鲁棒（L2-fragile），单配置跑赢 8 标的（P1 6/8）是不可证伪目标（`project_backtest_benchmark_falsifiability`）。

5. **谱系引用**：
   - **539 号 / `project_t14_t5_root_flip_necessity`**：根翻空/子空头降成本有效域 ⊂ 非上行 regime——本分析扩展到 segment 子空头（同有效域边界），并以 BTC 两套 max_children=80 坐实稀释机制。
   - `project_unn_btc_spawn_throwback`：BTC 踏空 = E spawn 强牛过度做空 + 借 units 稀释主升浪——本分析以 worktree −206757 / max_children=80 复现。
   - `project_constitutive_throughput_falsified` / `project_nrf_v4_strict_accounting`：清仓频率=regime 函数不可约——本分析的"spawn_seg>0 必 fail / 油链空头正域"是同一现象。
   - `why_not_profitable.md`：瓶颈在操作层（级别选择），非信号/会计层——本分析以成本门判别量精确化。
   - `qqq_root_short_no_costreduction_diagnosis.md`：T8 叶节点 + C located 天花板（裸空头强平死法）——本分析标注其为另一快照的另一死法，同 regime 根因。
   - 不确定是否有专门针对"P1 跨快照漂移 / 三套数据不一致"的谱系条目，明确声明未检索到——若需追踪回测可复现性，建议 escalate 判定是否升格为语法记录。

6. **影响声明**：本报告（`analysis/unn_latest_behavior_analysis.md`）为新增产出，**未改动任何引擎逻辑、prove 函数、定义或数据**。结论基于当前 working-tree JSON（Jun16 落盘）+ commit `e2a78b0dd7` 快照交叉验证。
   **首要影响**：揭示用户引用的表格无法对应任何已落盘数据源（三套数据 strat 与 BH 均不一致），后续讨论须先固定数据快照。支持"unn 强上行 regime 弱域 + segment 子空头需 regime 门控"判断，为未来 regime 门控轴提供 L3 证据。
