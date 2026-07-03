# 下游高级别重跑——frontier 污染清除后复核（task rerun-high-level, #97）

**工位**：swarm/ws-rerun-high-level | 基线 HEAD=4af6326dee（c546b5633c #93 advancing 变体已是其祖先，
frontier O(n²)→O(n) + earliest_unsealed_from 持久化 bit-exact 归零，e454c1099d #88 亦含）
**对照基线**：所有旧报告锚定 HEAD=122c918da9（strict-alpha-retest/wverify-alpha-retest）或早于 `#84`
坐实 frontier 污染的提交（l2-depth-distribution，HEAD=6fba4696e8）——**均早于 #88/#93 修复**，c546b5633c
是 122c918da9 之后 23 个提交中新增的，非其祖先（`git merge-base --is-ancestor c546 122c918` = 否）。
**认识论等级**：L2（真实 BTC 单标的/全历史，可产否定性结果）。**零生产代码改动**（只跑既有 `#[ignore]` 测试
入口，未改任何 .rs；构建 `cargo build --release --lib` 0 改动，无需重编译）。

---

## 1. 结论

**三项重跑全部完成，方向性结论全部维持（无一项翻转），但发现一个被低估的效应：frontier 修复对信号集
的扰动不限于 ℓ≥2，L0/L1 的信号计数也有小幅（<1.5%量级）漂移**——原任务指令假设「level 0/1 结论预期
不变（发散仅 ℓ≥2）」在计数层面不精确，需订正（见 §3 边界条件）。

| 重跑项 | 旧结论（污染态） | 新结论（污染已清） | 翻转？ |
|---|---|---|---|
| ①L2-depth 全历史分布 | 小转大 91.85%（1353/1473），可锚域 120 条 d=1 主导 95.8% | 小转大 **92.17%**（1365/1481），可锚域 **116** 条 d=1 主导 **94.8%**（110/116） | **否**——比例维持，绝对数微漂 |
| ②W-VERIFY 高级别桶（残差口径） | 全局 INCONCLUSIVE，V=0/F=3/I=25，主桶 L0买 Inconclusive（欠功效） | 全局 **INCONCLUSIVE**，V=0/**F=5**/**I=23**，主桶 L0买仍 **Inconclusive**（欠功效） | **否**——全局判定不变，F 桶数增加（见 §3） |
| ③H2 方向不对称 | L1 β=+356.35 p=0.010（卖>买显著） | L1 β=**+388.14** p=**0.002**（卖>买**更显著**） | **否**——方向不变，显著性增强 |

**frontier 标注处置**：`wverify_run.rs::bucket_verdict`（`wverify_cross_symbol` 消费）中 `lv>=2` 硬编码
标注 `"⚠污染未排除(frontier-bt-consumed)"` 现已**过时**——#88/#93 修复已使增量/全量 bit-exact，该注解
描述的污染源已解除。本工位按「零生产代码改动」硬约束**不改注释**，上浮供 codex/lead 裁定是否更新
（该注解本身不影响任何数值判定，只是显示字符串）。

---

## 2. 定义依据

### ①L2-depth 全历史分布重跑
- 命令：`ECON_L2_MAX_BARS=100000000 cargo test --release l2_depth_distribution_dx -- --ignored --nocapture`
- 数据：BTC 全历史 4,613,599 bars（2017-08-17→2026-05-31，与旧报告**数据本身不变**，无新增/缺失 bar）
- 定律一下沉锚定（第17课 L66）+ 区间套定理有效域=Type1（606号）：对 level1-4 每条 per-delta
  `!is_type1 && (is_type2||is_type3)` 信号跑 `descend_type1_anchor_depth`，None=小转大，Some(d)=可锚深度
- 耗时 238.22s（旧报告 813.67s——c546b5633c 的 O(n²)→O(n) 优化体现在此，非结论差异）

### ②W-VERIFY 高级别桶（残差口径）重跑
- 命令：`cargo test --release --lib theta_v0::backtest::wverify_run::wverify_full -- --ignored --nocapture`
- 与旧报告（strict-alpha-retest-20260702.md）**同一 harness、同一判据链**（`acc-alpha-estimand-prereg-
  20260701.md` 冻结 estimand `(ℓ,bsp_class,δ,σ^H)`，三态判据 `decontam::{classify_bucket,global_verdict}`）
- 确认 `wverify_full` 函数体在 122c918da9→当前 HEAD 之间**逻辑零变化**（`git diff` 仅见
  `walk_forward_oos_residuals` 新增 `symbol_index` 形参，BTC 传 0，代码注释自证「⟹ bit-exact」）——
  故观测到的数值差异 100% 归因于上游 `IncrementalClassifier`（`build_mu_from_bars` 消费，parser frontier
  修复的直接作用对象，非本 harness 逻辑改动

### ③H2 方向不对称重跑
- 同一 `wverify_full` 运行内产出（`perm_test::direction_asymmetry_beta_pvalue`，1000 次 block bootstrap，
  种子 20260701 冻结），非独立 harness

---

## 3. 边界条件（结论何时翻转 / 已观测到的漂移）

### 3.1 L2-depth 分布（①）——四 level 全部微漂，非仅 ℓ≥2

| lvl | 旧信号总数 | 新信号总数 | Δ | 旧小转大% | 新小转大% |
|---|---|---|---|---|---|
| 1 | 1059 | 1069 | +10 | 91.22% | 91.58% |
| 2 | 324 | 321 | −3 | 93.83% | 94.08% |
| 3 | 69 | 71 | +2 | 92.75% | 92.96% |
| 4 | 21 | 20 | −1 | 90.48% | 90.00% |
| **合计** | **1473** | **1481** | **+8** | **91.85%** | **92.17%** |

depth 直方图：lvl2 的 d=2 从 4 例降到 3 例（唯一结构性变化，真递归深度 d≥2 观测从 5 例降到 4 例，
仍稀有）；lvl1/lvl3/lvl4 的 depth 分布 bit-exact 不变。max_depth=3 两版本相同。base_none=0、
残差群=0、锚点正确性 100%（116/116 新 vs 120/120 旧）三项守恒不变。

**订正**：本任务指令假设「level 0/1 结论预期不变（发散仅 ℓ≥2）」——**在计数层面不精确**。
level1（即塔的 lvl=1，非 bsp level0）信号数漂移 +10（0.9%），与 lvl2/3/4 同量级，说明 frontier
修复对信号集的扰动**不是 ℓ≥2 独有效应**，根因是 `l2_depth_distribution_dx` 走的
`IncrementalClassifier`（`assemble_gamma_with_tower` 逐 bar 因果重算），该组件在**所有层级**共享同一
parser frontier 状态机——frontier bug 原本就全域存在，只是 #84 诊断时在 ℓ≥2 因样本稀疏（n=20-25）
使**相对**发散更显眼，不代表 ℓ<2 的**绝对**信号集未受影响。方向性结论（小转大占绝对多数、
depth=1 主导）在此量级漂移下维持，但"发散仅 ℓ≥2"这一因果归因需订正为"发散全域存在，
量级随桶样本量反比放大"。

### 3.2 W-VERIFY 高级别桶（②）——主桶维持 Inconclusive，F 桶数 3→5

主桶 L0/type3/买/σ^H=0（唯一强点估计桶）：

| 口径 | n | n_eff | mean(Y) | LCB | UCB | perm_p | state |
|---|---|---|---|---|---|---|---|
| 旧（122c918，污染态） | 1597 | 166.42 | +178.66 | +102.49 | +254.82 | 0.005 | Inconclusive（166<290 欠功效） |
| 新（当前 HEAD，已清） | 1613 | 167.55 | +184.56 | +108.78 | +260.34 | 0.020 | Inconclusive（167.55<290 欠功效） |

n +16（+1.0%），mean/LCB 同向小幅上移，perm_p 从 0.005→0.020（仍 <0.05 显著，但显著性减弱）——
**结论不翻转**：仍是唯一强点估计桶、仍欠功效 Inconclusive、正号未变。

3 个旧 Falsified 桶中 2 个 **bit-exact 完全不变**（L0卖σ+1: n=3 mean=−648.76 一位不差；
L3卖σ+1: n=2 mean=−387.86 一位不差）——这两个 3 笔/2 笔的边缘小样本桶恰好未被本次修复触及。
第三个（L4卖σ0）n 3→4（+1 笔），mean −189.93→−201.46，仍 Falsified。

**F 从 3→5（新增 2 个 Falsified 桶）**：L1/type2/买/σ^H=0（n=114, mean=−230.52, LCB=−406.38,
UCB=−54.65）与 L3/type2/买/σ^H=−1（n=3, mean=−802.52, LCB=−1566.25, UCB=−38.79）。旧报告
「关键桶」摘录未列出这两行的旧值（仅摘录 6 个代表桶，非全 28 桶），故无法逐值对照其状态迁移路径
（可能旧值已接近 Falsified 边界、或旧值本为 Inconclusive 因样本变化跨过 powered 门槛）——**如实
标注为观测缺口，非虚报**。这两桶均 n≤114 小样本，边界最近处仍是「弱否证」性质（与旧 3 个
Falsified 桶同类瑕疵：powered 判定在 n<10~100 量级的有效性本身待裁，见旧报告 §6 开放问题）。

**全局裁决翻转判据**（旧报告 §6.4）：需**所有**桶 powered-FALSIFIED 才翻 FALSIFIED——23 个
Inconclusive 桶仍阻止该翻转，全局维持 INCONCLUSIVE，未翻转。

### 3.3 H2 不对称（③）——方向全部维持，L1 显著性增强

| 级别 | 旧 β=μ卖−μ买 | 旧 boot_p | 新 β | 新 boot_p | 读出 |
|---|---|---|---|---|---|
| L0 | −224.65 | 0.956 | −224.35 | 0.953 | 买>卖（不变，几乎 bit-exact） |
| L1 | +356.35 | **0.010** | +388.14 | **0.002** | 卖>买显著（不变，**显著性增强**） |
| L2 | +24.48 | 0.477 | +38.55 | 0.405 | 不显著（不变） |
| L3 | +171.74 | 0.000 | +220.15 | 0.000 | 卖>买但 n tiny（不变） |
| L4 | （旧表未列，样本此前为空/未产出） | — | −1569.14 | 1.000 | 不显著（新出现，n_buy=1/n_sell=5 极小样本） |

L4 新出现一行——旧 H2 表止于 L3（旧 wverify_full 跑批中 L4 买腿样本为空，`buy.is_empty()` 跳过该行；
新跑批 L4 买腿出现 1 笔观测，触发该行输出）。此为 §3.1 已确认的「信号集全域微漂」的又一体现，
非独立异常。

---

## 4. 下游推论

1. **acceptance s3-strict-alpha-retest 判定维持**：严格残差口径下预注册 estimand 全局仍 INCONCLUSIVE，
   frontier 污染清除**未改变**该验收状态——PASS/INCONCLUSIVE 边界不受本次修复影响，无需回退 M1
   相关裁定。
2. **「高级别无 alpha」H2 判定加固**：L1 卖>买不对称从 boot_p=0.010→0.002，是**更强**的否证性/支持性
   证据（原文 H2 方向候选未减弱，反而加固）。此前该结论挂着「污染未排除」的条件效力域标注
   （旧报告 §6 任务硬约束条款）——**该条件现已解除**，L1 结论可从条件性升级为无条件（污染已清）。
3. **L2-depth 分布结论可无条件引用**：`.chanlun/review-results/l2-depth-distribution-20260702.md` 顶部
   的效力域降级标注（"frontier 问题②污染仍未清除...结论视为条件性"）——**本次重跑后该条件解除**，
   91.85%→92.17% 方向一致，可将旧报告标注更新为「已解除」（本工位不改文件，仅在此报告中记录解除
   依据，供 genealogist/lead 决定是否回写旧报告头部）。
4. **「发散仅 ℓ≥2」假设需订正**（新发现，非任务原假设）：任何后续依赖「level0/1 结果对 frontier
   免疫」的推论都应重新审视——本次实测显示扰动全域存在（量级 <1.5%），只是在小样本高级别桶
   （n=2-25）表现为状态翻转（Falsified 数量变化），在大样本 L0 桶（n=1400-1600）表现为数值微漂
   不改变三态判定。

---

## 5. 谱系引用

- **#84（frontier 链阶段0）**：原诊断"增量/全量 level≥2 发散坐实，根因重定位 parser"——本次重跑
  显示根因（parser frontier）影响面**不限于 ℓ≥2**，是全域效应在小样本桶被放大显现，订正 #84 的
  归因范围表述（非否定 #84 的核心发现——parser 是根因这一点仍成立且已被此次重跑证实是全域根因）。
- **#88（codex #87 裁决，e454c1095c）+ #93（advancing 变体，c546b5633c）**：frontier 修复本体，
  本报告是其下游验证收口。
- **strict-alpha-retest-20260702.md**（s3，HEAD=122c918da9）：本报告②③项的直接对照基线。
- **l2-depth-distribution-20260702.md**（HEAD=6fba4696e8，隔离副本 `/tmp/l2dist2`）：本报告①项的
  直接对照基线，其头部效力域降级标注在此报告依据下解除。
- **231号（形式化有效域）**：本重跑的有效域=BTC 单标的、当前 HEAD 全历史/全 OOS 窗，不外推跨品种
  （跨品种 L3 见 `16160a7014` 独立工作，未在本工位范围）。
- **no-patch-mentality / no-workaround**：过时的 `frontier(≥2)` 字符串注解未静默改动，如实上浮
  （§1 结论段）。

---

## 6. 影响声明

- **代码改动**：零。仅执行两个既有 `#[ignore]` 测试入口（`l2_depth_distribution_dx`、
  `wverify_run::wverify_full`），未改任何 `.rs` 文件，未 commit/push。
- **产出文件**：本报告 + `/tmp/wv_full_rows.md`（28 桶新表）+ `/tmp/wv_full_h2_asymmetry.md`
  （逐级别新 β 表）+ `/tmp/wv_full_timeblocks.md`（逐窗新分层，均重新生成覆盖旧临时文件）+
  `/tmp/wverify_full_rerun_20260703.log` + `/tmp/l2_depth_rerun_20260703.log`（完整跑批日志留存）。
- **判定语义**：三项重跑的全局/方向性结论**全部维持**（0 翻转）。发现一处需订正的归因表述
  （frontier 影响面非 ℓ≥2 独有，§3.1/§4.4）与两处需 codex/lead 裁定的处置项（过时 frontier 注解
  字符串是否更新代码；旧报告头部效力域降级标注是否回写解除）。
- **未改**：任何生产代码、预注册冻结文件、settled 定理、旧报告原文（保留不改，本报告作为独立
  复核记录）。
