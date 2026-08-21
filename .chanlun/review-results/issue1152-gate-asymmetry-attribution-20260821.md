# #1152 证书门出场/入场拒率不对称归因——四维交叉 + cond1 长短拆解 + 生成面对比 + 逐例归因

- **issue**：[#1152](https://github.com/xy7365527-lang/NewChanlun/issues/1152)（#1147 读数的追问票：门开时出场候选 85.3% 被拒 vs 入场候选 26.0%）
- **性质**：测量 + 归因，**零生产码修改**。不裁门的开/关（归编排者），本票只产读数。
- **基线**：`sandcastle/issue-1152`（base `b1640cb2b7`，== main 顶）。探针插桩全部 `#[cfg(test)]`。
- **数据**：Binance 归档重下 2024-01..2025-01 BTCUSDT 1m（`scripts/download_btc_binance.py`，13 个月 zip），窗口 `2024-01-01 .. 2025-01-01`（528480 bar）。
- **日期**：2026-08-21。

---

## 0. 一句话结论

**不对称主要来自「生成面」的级别构成（level composition），不是门的判据层。** 门内**每一级**的拒率出场/入场几乎逐级对齐（level=1 两侧都是 ≈99%，level=0 是 32% vs 20%）；差 3.3× 的根因是**出场候选 75.3% 落在 level=1**（该级近乎全拒），**入场候选 69.2% 落在 level=0**（该级 78% 放行）。Oaxaca-Blinder 分解：级别构成项贡献 **+0.532**（占总差 0.593 的 **89.7%**）；级内判据差项仅 +0.141（且集中在 level=3 的小样本）。

三个疑点：**①cond1 方向偏好——排除**（不是长/短偏好，是 level 构成）；**②生成面更宽——坐实（但形状是「level-1 浓度 + δ 向锚点」，不是「低级别噪声」）**；**③出场侧无参照面——已给出参照**（逐例结构签名完全一致：锚点子段方向 ≡ 信号方向 δ）。

---

## 1. 复现口径

复用 #1147 的探针 `gate_on_stepfail_composition_dx`（`runner_tests.rs:5519`，`#[ignore]`），另加逐例结构 dump（`econ_positive.rs` `stepfail_probe::sample_structure`，`#[cfg(test)]`，env 未设 = no-op，见附录）。

- 门开（线程局部 `NEST_CERT_GATE_OVERRIDE` 注入），窗口 `2024-01-01..2025-01-01`，标准 dump + 样本 dump 同一趟跑出。
- **双臂读数与本票结论无关**（本票只读候选 dump，不开双臂对比），但重跑逐位复现 #1147：`NEST_GATE_STATS total=2212 admitted=1111 rejected=1101`、`NEST_GATE_EXIT_CAND total=886 admitted=130 rejected=756`、`n_orders=238245 strat_return=0.467363`——全部与 #1147 报告一致。
- **标准 dump 与 #1147 原始 dump 逐字节相同**（`diff <(sort 新) <(sort 旧)` 空，2212 行全同）⟹ 重下数据与 #1147 的 `btc_1m_full.json` 在 2024 窗上逐位一致，样本取样与结构读数可直接对齐 #1147 的候选。

---

## 2. 四维交叉（would_close × stepfail 桶 × level × dir）

### 2.1 主表：按 level 切（不对称的主轴）

| level | 出场 拒/总（率） | 入场 拒/总（率） |
|---|---|---|
| 0 | 32/99（32.3%） | 187/917（20.4%） |
| **1** | **662/667（99.3%）** | **97/98（99.0%）** |
| 2 | 28/79（35.4%） | 60/226（26.5%） |
| 3 | 34/40（85.0%） | 1/68（1.5%） |
| 4 | 0/1（0.0%） | 0/17（0.0%） |
| 合计 | 756/886（**85.3%**） | 345/1326（**26.0%**） |

**级内拒率出场/入场几乎逐级对齐**（level=1 两侧 99.3% vs 99.0%），**构成却完全错开**：

| level | 出场候选占比 | 入场候选占比 |
|---|---|---|
| 0 | 11.2% | **69.2%** |
| **1** | **75.3%** | 7.4% |
| 2 | 8.9% | 17.0% |
| 3 | 4.5% | 5.1% |
| 4 | 0.1% | 1.3% |

**反事实**：若出场候选按入场候选的 level 构成跑，出场拒率 = Σ(入场占比_l × 出场级内拒率_l) = **40.1%**（实际 85.3%）；若入场候选按出场构成跑，入场拒率 = **79.2%**（实际 26.0%）。

### 2.2 Oaxaca-Blinder 分解（按 level）

| 项 | 值 | 占总差 |
|---|---|---|
| 构成项 Σ(出场占比−入场占比)×入场级内拒率 | **+0.5321** | **89.7%** |
| 级内判据项 Σ入场占比×(出场级内拒率−入场级内拒率) | +0.1407 | 23.7% |
| 交互项 | −0.0797 | −13.4% |
| 合计（= 85.3%−26.0%） | +0.5931 | 100% |

### 2.3 按桶切（level 塌缩，桶与 level 混淆）

| 桶 | 出场 拒/总（率） | 入场 拒/总（率） |
|---|---|---|
| cond1 方向 | 308/334（92.2%） | 57/208（27.4%） |
| cond2 D-3 | 177/202（87.6%） | 49/102（48.0%） |
| struct_break | 112/112（100%） | 45/45（100%） |
| l0_exempt | 28/95（29.5%） | 155/885（**17.5%，占入场 67%**） |
| cond3 Extreme | 60/66（90.9%） | 11/27（40.7%） |
| cond4 Weak | 39/42（92.9%） | 14/37（37.8%） |
| anchor_baseL0 | 21/22（95.5%） | 7/7（100%） |
| type1_pass | 11/11（100%） | 5/5（100%） |

桶面看「同桶出场拒率 ≫ 入场拒率」，但**这是桶与 level 的混淆**：cond1/cond2/cond3/cond4 这些高拒桶本身集中在 level=1（见 §3），而入场侧被 l0_exempt（level=0 的免门桶，17.5% 拒）稀释。level 才是主轴。

### 2.4 dir 维

出场/入场各自 Long/Short 计数（含全部候选）：出场 Long 421 / Short 465；入场 Long 615 / Short 711。**level 内 Long/Short 拒率无系统偏斜**（level=1 出场 Long 318/320、Short 344/347；level=1 入场 Long 70/70、Short 27/28；level=0 出场 Long 16/45、Short 16/54——两侧逐级对齐）。方向维没有独立贡献（level=1 入场候选数 Long 70 vs Short 28 是生成侧的计数差，拒率仍都 ≈100%）。

---

## 3. cond1 方向桶长/短拆解（疑点①：排除）

cond1 拒（`dir(s)≠−δ`）出场 308 例、入场 57 例（5.4×）。按 level 拆：

| | 出场 cond1 | 入场 cond1 |
|---|---|---|
| level=1 | **301/334 = 90.1%**（拒 299/301 = 99.3%） | 45/208 = 21.6%（拒 45/45 = 100%） |
| level≥2 | 33/334 = 9.9%（拒 9/33 = 27.3%） | 163/208 = 78.4%（拒 12/163 = 7.4%） |

**level=1 内 Long/Short 完全对称**：出场 Long 153/153（100%）、Short 146/148（98.6%）；入场 Long 34/34、Short 11/11（均 100%）。level≥2 拒率也低且无方向偏斜（出场 Long 6/20、Short 2/6；入场 Long 10/60、Short 1/40）。

**结论**：5.4× 不是「方向偏好」，是 level 构成——出场 cond1 有 90% 是 level=1（该级方向检查近乎全拒），入场 cond1 只有 22% 是 level=1、78% 是 level≥2（该级方向检查大多放行）。「子段方向不符是机械检查、长/短应理应对称」的直觉成立——**它确实对称**；不对称的是喂进去的 level 构成。疑点①排除。

---

## 4. 生成面对比（疑点②：坐实，形状订正）

### 4.1 生成路径（代码证据）

- 候选生成：`coverage_elements_and_gamma_with_tower_cached_gen`（`interp.rs:1183`）逐 level 遍历 `classification.levels[*].bsp`，每个 BspPoint 产一个候选，`c.level = 该点所在塔级`。候选流天然包含 level 0..4。
- 出场标记：`exit_candidate_would_close(c.level, c.dir, prev_active)`（`admission.rs:228`）= 存在**同级反向**活动腿。
- **level-1 活动腿的来源不是 level-1 候选**（level-1 候选全年仅 6 个过门、近乎开不了腿），而是**开仓父链恢复**：open 路径 `step.rs:421` 对每个开仓候选调 `restore_ancestor_chain_from_registry`，沿 `structural_parent_id` 把父/祖父载体恢复为活动腿。全年 level-0 的入场侧（would_close=False）候选 730 个过门（其中绝大多数随后开腿）⟹ 其 level-1 父载体被大量恢复为活动腿。
- 结果：level-1 活动腿常年存在 ⟹ 每 bar 生成的 level-1 反向 BSP 候选都命中 `would_close=True` ⟹ 出场候选被 level-1 灌满（667/886 = 75.3%）；level-0 腿开得快平得快，level-0 候选大多不命中反向腿（level-0 出场候选仅 99/886 = 11.2%）。

### 4.2 每 bar 生成频率

1553 个 bar 有候选，每 bar 候选数中位 1、最大 5（1 候选 1055 bar / 2 候选 374 / 3 候选 92 / 4 候选 27 / 5 候选 5）。出场/入场没有「出场每 bar 都在查、入场少查」的生成频率差——**候选流是同一份**，`would_close` 只是给同一候选流打标。

### 4.3 「低级别噪声」形状订正

疑点②原设想「低级别反向波动全算出场候选」。实测**噪声面不是 level=0（最低级），而是 level=1**：level-1 出场候选占 75.3%、其 99.3% 被拒；level-0 出场候选只占 11.2% 且 68% 放行。出场候选生成面「更宽」的准确形状 = **level-1 浓度**（由 restored 祖先腿 + level-1 BSP 候选共同造成），非 level-0 噪声。

### 4.4 附注：同 (level, source_index) 双候选

#1147 dump 的 2212 行里 319 个 (bar, level, source_index) 键有 ≥2 行（147 键同桶、172 键异桶）。样本面坐实：同一 pivot 同时产 **买向 + 卖向**两个 BSP 候选（如 bar=33276 si=33216 同出 buy2-Long 与 sell2-Short）。这是 BSP 端点语义双向置位（2/3 类可共存 + 买卖双向）在候选层的投影，不是门的重复计数。

---

## 5. 逐例归因（42 例出场侧 cond1 拒）

抽样 42 个键（#1147 原始 dump 中出场 cond1 拒 308 例的典型样本，全年时间均匀铺开；样本键：`.chanlun/review-results/issue1152-cond1-sample-keys-2024.jsonl`）。重跑逐例结构 dump 共落 51 条候选线（42 键，其中 9 键同出买卖双向候选）。**结构展开盖 Type2/3 下钻路径**：其中 42 条是 Type2/3 的 cond1（div_cond=1）线（level=1×33 + level=2×8 + level=3×1；Long 24 / Short 18；Type2 22 / Type3 20），另有 2 条 type1_div_fail（Type1 路径，本票逐例结构仅盖 Type2/3 descend，Type1 线未展开锚点结构）。逐例展开执行段/sub_moves/锚点子段方向/父中枢/div_cand 条件（样本 dump：`.chanlun/review-results/issue1152-cond1-sample-structure-2024.jsonl`）。

### 5.1 结构签名（42/42 一致）

对每个 cond1 拒样本，`div_cand` 的**锚点子段（end_index==source_index）几何方向恒等于信号方向 δ**：买候选（δ=Long）锚在 Up 子段（24/24），卖候选（δ=Short）锚在 Down 子段（18/18）。即 `dir(s)=+δ`，故 `dir(s)≠−δ` ⟹ cond1 恒拒。

### 5.2 三档归因

| 档 | 判据 | 计数 |
|---|---|---|
| **判据误伤**（锚点处本有 −δ 段被误读/误定位） | 存在 −δ 子段且其区间**包含** source_index | **0/42** |
| **真该拦**（整段无 −δ 子段，本就无背驰段） | exec_move 的 sub_moves 里**无任何** −δ 子段 | 1/42 |
| **生成面噪声**（−δ 子段存在，但锚点落在 δ 向子段上） | sub_moves **存在** −δ 子段、但不含 source_index | **41/42** |

样本里 9 个键同出买卖双向候选（§4.4）；2 条 Type1 线未展开结构（探针仅盖 Type2/3 descend）。

### 5.3 读法

- **门没有误判方向**：cond1 读到的是塔自己的方向标号，锚点子段确实标着 δ，机械检查 `dir(s)=−δ` 判假是「诚实方向不符」（#1028 终验口径），不是误拒。判据误伤档 0/42 坐实这一点。
- **被拒的是「δ 向锚点」的 BSP 点**：41/42 的执行段里其实**存在** −δ 子段（真背驰段），但 BSP 的 `source_index` 锚在另一个 δ 向子段上（#1028「点锚迁移到 departure 单元终点」后的形态）⟹ 门看到的是锚点处的反向波动，不是背驰段本身。
- 这 41 例到底是「真该拦（门挡住了本不该放行的信号）」还是「生成面噪声（这些候选本不该进出场面）」，**取决于 BSP 锚点口径本身对不对**——那是 BSP 生成/#1028 锚的教义面，**不在本票门归因范围内**。本票只产读数：**出场 cond1 拒的形态主体 = 锚点落在 δ 向子段的 level-1 BSP 候选**。

---

## 6. 三疑点逐条坐实/排除

| 疑点 | 结论 | 一句话证据 |
|---|---|---|
| ①cond1 方向桶有方向偏好（长/短不对称） | **排除** | 5.4× 全由 level 构成解释（出场 cond1 90% 是 level-1 vs 入场 22%）；level-1 内 Long 153/153 vs Short 146/148 对称，level≥2 亦无方向偏斜 |
| ②出场候选生成面更宽（混入低级别噪声） | **坐实，形状订正** | 出场 75.3% 是 level-1（99.3% 拒）vs 入场 69.2% 是 level-0（78% 放行）；噪声面是 level-1（restored 祖先腿喂出），不是 level-0 |
| ③出场侧无参照面（切不开该拦/不该拦） | **已给出参照** | 逐例结构签名 42/42 一致：锚点子段方向 ≡ δ；判据误伤 0/42、生成面噪声 41/42、真该拦 1/42 |

**定量结论**：85.3% vs 26.0% 的差，**89.7% 是级别构成项**；门判据本身级内对称（每级出场/入场拒率基本一致）。要压出场拒率，动门的判据没用——要动的是**出场候选的 level 构成**（level-1 的 δ 向锚点候选），那属于 BSP 生成/锚点口径面，不属本票。

---

## 7. 未测项 / 局限（照实）

1. **单标的单窗**：BTC 全年 2024，别的品种/年份没跑。
2. **level-1 腿的恢复频率**：只读了 restore 代码路径坐实机制，未逐腿计数「level-1 活动腿全年多少 bar 在场」——本票未加该探针。
3. **δ 向锚点是否该判「生成面噪声」还是「真该拦」**：涉及 BSP 生成/#1028 锚的教义面，本票只读数不裁。
4. **42 例抽样**：随机 + 分层抽样，非全量；样本面 51 行含 9 个双候选键。
5. **`cargo test` 全量回归没跑**（越界；只跑了 `cargo check --tests` 绿 + 本探针）。

---

## 8. 复现

```bash
# 数据（Binance 归档，13 个月）
cd /home/agent/workspace
BTC_START_YEAR=2024 BTC_START_MONTH=1 BTC_END_YEAR=2025 BTC_END_MONTH=1   BTC_OUTPUT=btc_1m_full.json python3 scripts/download_btc_binance.py

# 探针（标准 dump + 逐例结构 dump，同窗口同跑）
cd rust
export PYDIR=/home/agent/.local/share/uv/python/cpython-3.11.16-linux-aarch64-gnu
export LD_LIBRARY_PATH=$PYDIR/lib:$LD_LIBRARY_PATH LIBRARY_PATH=$PYDIR/lib
P1147_STEPFAIL_DUMP_PATH=/tmp/p1152_stepfail_2024.jsonl P1147_WINDOW_START=2024-01-01 P1147_WINDOW_END=2025-01-01 P1152_SAMPLE_KEYS_PATH=<样本键 JSONL> P1152_SAMPLE_DUMP_PATH=/tmp/p1152_sample_2024.jsonl cargo test --release --lib gate_on_stepfail_composition_dx -- --ignored --nocapture
```

- 探针源：`rust/src/theta_v0/backtest/econ_positive.rs`（`stepfail_probe` 模块：`classify` 原有 + `sample_structure` 本票新增，均 `#[cfg(test)]`）+ `runner_tests.rs:5519`（驱动，`#[ignore]`）。
- 本票新增代码零生产影响（`#[cfg(test)]`）；标准 dump 与 #1147 逐字节相同，坐实插桩非扰动。
- 样本键文件与逐例结构 dump 入仓 `.chanlun/review-results/`（供复用）。

---

*本报告只产读数，不裁「开门 / 维持关」，不裁 BSP 锚点口径。门默认态 = 编排者按 #1147 + 本票读数裁定。*
