# #1091 三轮下钻残余桶反事实拆解·探针设计与假设排序（预登记）

- 日期：2026-08-18
- 票：[#1091](https://github.com/xy7365527-lang/NewChanlun/issues/1091)（research）
- 类型：只读探针 + 设计/口径 + 假设排序（**预登记**）；判定≠裁定，不自行改口径
- 状态：探针已入仓（`type1_residual_counterfactual_dx`）；**读数未产出**（沙盒无 `analysis/data_cache/btc_1m_full.json`，DATA BLOCKER）

## 0. 交付物

1. **探针** `type1_residual_counterfactual_dx`（`rust/src/theta_v0/backtest/econ_positive.rs`，`#[test] #[ignore]`，只读）——逐例 dump + 三键反事实翻转矩阵，与 #846/#870 三轮同批同口径。
2. **本设计文档**——口径、三键关系、归因判读口径、假设排序（预登记）、若需新探针的规格。
3. **跑批命令**（见 §6）。

## 1. 探针口径（与三轮可对表）

- 同一 `IncrementalClassifier` 逐 bar 循环、同一去重键 `(lvl, source_index, bsp_disc)`、同一 Γ 定向、同一 `LMIN=1`/`LMAX=4`——与 `type1_descend_continuity_dx`（#846 探针）逐格同口径。
- 失败桶 = **链终止 C 条件号**（cond1..=4，`prod_div_cand_why` 直调生产 `div_cand_fail`，parity 由构造保证）——即 #1028 三轮终验的 方向残 13 / D-3 取段 59 / Extreme 36 / Weak 15 四桶。
- 逐例 dump 字段：父走势结构（Segment/Compose·中枢数·subs 数·父中枢）/ D-3 取段键（s 跨界与否、prev 是「进入段」还是「上次离开段」）/ 目标段 lo-hi-area / 前序同向段 lo-hi-area / 期望与实判（方向、Extreme、Weak 三子判定 + 首个失败条件）。
- 三键反事实：对每条链终止 C 失败，把目标子走势对齐键换成 ① `source_index`（生产现锚 = departure 终点）② 窗口极值 bar（δ 侧价格极值，`window_extreme_bar`）③ departure 终点（当前父走势末趋势子段终点，`departure_unit_end`），重跑生产 `div_cand_fail`，记新结局（pass / cond1..4）。**纯测量，不改生产判据、不改生产锚。**

## 2. 三键关系（诚实声明）

① 与 ③ 在**首步**同值（裁定 A 后 `source_index` == departure 终点）——两臂只在下钻**更深步**（cur 已非顶层 C，src 仍是顶层锚、③ 是当前父走势自己的 departure 终点）分叉。**真正的对照差异由 ②（窗口极值 bar，即 #1035 判定的「背驰极值」候选锚）贡献。** 若三臂读数几乎全同，即说明「departure 终点 vs 背驰极值」这个剩余锚差异对 cond2/3/4 **不敏感**——这是与方向桶（159→13 高度敏感）的关键对照。

## 3. 归因排序·判读口径（预登记）

| 桶 | 归因候选 | 判据（读数如何分流） |
|---|---|---|
| cond1 残 13 | 锚错位残余 vs 市场事实 | ② 极值 bar 臂翻转为通过/其他桶高 ⟹ 锚错位残余（departure 终点 ≠ 背驰极值）；三臂恒 cond1 ⟹ 目标子走势方向真 ≠ −δ（市场事实） |
| cond2 D-3 取段 59 | **取段边界** vs 结构事实 | ② 臂翻转为通过高 ⟹ 目标段被错选（departure 子段未跨界，极值子段跨界）＝取段边界投影；三臂恒 cond2 且 dump 显示「无父中枢语境」⟹ D-3 规则的结构缺口；恒 cond2 且 dump 显示「s 未跨界/无同向前段」⟹ 市场事实 |
| cond3 Extreme 36 | 取段边界投影 vs **Extreme 真否** | ② 臂翻转为通过高 ⟹ 换段后 Extreme 即真（取段边界投影）；三臂恒 cond3 ⟹ 目标段对任一前序同向段都不创新极值（Extreme 真否＝市场事实） |
| cond4 Weak 15 | 取段边界投影 vs **Weak 力度真否** | ② 臂翻转为通过高 ⟹ 换段后力度衰减成立（取段边界投影）；三臂恒 cond4 ⟹ L(C)<L(B) 真不成立（Weak 力度真否＝市场事实） |

**证据强度**：翻转为通过率来自真实读数（强）；逐例 dump 的 margin（如 `Extreme(lo=140<lo(s')=120)` 差多少）为二级证据（判「差一点」还是「差很远」）；「仍同桶」占比为三级证据（判市场事实上界）。

## 4. 假设排序（预登记，证据强度＝待读数）

1. **H1（最强）cond2 主因 = 取段边界/锚残余**——D-3 取段 59 例中应有相当比例是「departure 子段未跨界、但窗口极值 bar 所在子段跨界」型：② 臂会把这些翻为通过或 cond3/4。
2. **H2 cond3 主因 = Extreme 真否（市场事实为主，部分取段边界投影）**——② 臂翻转一部分（投影），剩余恒 cond3 的部分是「该级子走势对前序同向段真不创新极值」。
3. **H3 cond4 主因 = Weak 力度真否（市场事实为主）**——力度衰减（L(C)<L(B)）是价格结构事实，换段改变幅度有限，预计翻转率最低。
4. **H4 cond1 残 13 = 锚错位残余**——方向桶修后 159→13 的残余里，若 ② 臂能再翻掉一部分，说明 departure 终点仍晚于背驰极值（#1035 §1.1「点锚系统性晚于 C 走势单元背驰极值 bar，且偏移逐级放大」同源）；若全不翻，这 13 例是「重锚后点集变化」引入的真方向不符。

**排序理由**：方向桶对锚高度敏感（159→13）是已证事实 ⟹ 同一条对齐键对下游 cond2/3/4 的投影按「先取段、后 Extreme、再 Weak」的判据短路序递减 ⟹ H1 > H2 > H3。**反转条件**：若 cond2 的 ② 臂翻转率 ≈ 0 且 dump 全部落「无父中枢语境」，则 H1 不成立，D-3 取段 59 是**结构缺口**（父走势无中枢，D-3 取段无定义）而非取段边界投影——这会把结论从「判据边界」改写为「#814 D-3 规则在该结构形态上无定义」。

## 5. 若需新探针的规格（条件触发，不改生产判据）

- **规格 A（cond2 恒同桶触发）**：「无父中枢语境」子桶普查——父走势无中枢的形态学（Segment 父？单子走势？趋势块中枢被 LevelExpansion 拆走？），量化 D-3 取段在该结构形态上的**不可判定域**大小。
- **规格 B（cond3/cond4 恒同桶触发）**：Extreme/Weak 的 margin 直方图——`lo(s)−lo(s')`（δ=Long）与 `L(C)−L(B)` 的分布，判「差一点」vs「差很远」的比例，供「宽松档该不该开」的裁定前测量（**只测不裁**）。
- **规格 C（② 臂翻转量级大触发）**：点锚迁移重议——若 cond2/3/4 的 ② 臂（背驰极值）显著翻转，「点锚=departure 终点」vs「点锚=背驰极值」需回 #1035 判定（裁定级，本票不裁）。

## 6. 诚实边界与跑批

- **数据**：`analysis/data_cache/btc_1m_full.json`（314MB 真实行情）被 `.gitignore:82` 排除、不入仓；沙盒内无此文件 ⟹ 本票读数未产出（DATA BLOCKER，探针以 panic 明确拒绝伪造合成数据）。
- **解除条件**：在持有 BTC 全量数据的机器上（宿主/编排层环境）跑下述命令。
- **跑批命令**：

```sh
ECON_L2_MAX_BARS=100000000 cargo test --release type1_residual_counterfactual_dx -- --ignored --nocapture
```

- **产物**：`.chanlun/review-results/issue1091-residual-counterfactual-raw.md`（总读数 + 翻转矩阵 + 逐例 md 取样）；`.chanlun/review-results/issue1091-residual-counterfactual-cases.jsonl`（全量逐例 dump）。
- **验证**（沙盒内已完成）：`cargo check --tests` 过、`cargo fmt --check` 净、同模块 48 个非 ignored 合成单测绿、探针本体 fail-fast（DATA BLOCKER panic 即时退出，不空转）。
