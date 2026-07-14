# parser 线段增量 frontier 修复实装（#88，codex #87 修补版 A）

**任务**：#88 parser 线段增量 frontier 回退修复（codex #87 裁定=修补版 A）
**规格权威**：`.chanlun/review-results/codex-87-parser-frontier-fix-ruling-20260702.md`（修补版 A 全文 + 验收清单）
**基线权威**：`.chanlun/review-results/frontier-bt-consumed-20260702.md`（发散测量 + 根因重定位）
**认识论等级**：L2（真实 CL+BTC 逐 bar / 终点两口径对拍，可产否定性结果）+ L1（合成 bit-exact 管线验证 + CPU 计数）
**HEAD**：9e3e8ab066（实装前），零 commit（交付给 team-lead 集成）

---

## 一、结论

**修复正确性达成（bit-exact），但性能触发裁定边界条件3（退化为候选 C 的 O(n²) 特征）。**

- **发散归零**：CL 50K parser 增量 vs 全量 `div_count 709→0`；`decisive_endpoint_tower_parity_longhistory` 在 CL+BTC 的 50K/150K/300K **全 level bit-exact**（修复前 BTC 150K/300K L2/L3、CL 全档发散）。frontier bug 坐实消除。
- **性能红线**：`earliest_unsealed_from` 在 CL 上**锚定于 stroke 10 且永不前移**（euf_adv=1 仅初始 None→10），rescan 段数随 n 线性增长（411→1278→2534），ParseLayerIncr 墙钟 **exp=2.04 = O(n²)**。这正是裁定**边界条件3**预警的"修补版 A 退化为候选 C 性能特征"——历史最小值单调非增把 confirmed_bound 永久锚死在早点。**此为需上浮的"选择"**（correctness-first 已达成；性能补救=accept O(n²) / 候选 C / advancing 变体，须编排者/codex 重评+re-audit）。

**定性**：实现 bug 修复（非定义冲突，PDF §七/§十 canonical=全量重算明文）。修复未改任何定义含义/边界/适用域，只让增量确认边界回退到可证 sealed 处——no-workaround 判据满足。

---

## 二、实装（严格按裁定，不越界）

**改点域**：`parser/feature_seq.rs`（FeatureSeqState）+ `parser/segment.rs`（IncrSegments）+ `parser/mod.rs`（ParseLayer 诊断字段透出）。**不碰** `second_kind.rs`（静态确认逻辑）、**不碰** tower 层（econ/nest/mod，塔 resume 已 bit-exact，#84 重定位结论保持）。**不碰** `second_seq_scan_window`（候选 B 维持排除）。**未启用候选 C**（cascade_count = N/A）。

### 机制（codex 裁定具体设计逐条落地）

1. `FeatureSeqState` 新增 `skipped_secondkind: bool`——`scan_trigger` 遇 `has_gap && !second_seq_has_fractal(...)`（即将跳过的 SecondKind 候选）时置真；`reset` 清零（每段独立累积）。
2. `IncrSegments` 新增持久化字段 `earliest_unsealed_from: Option<usize>`——重扫循环中若本段（`seg_start`）扫描期间 `feat.skipped_secondkind()`，把 **seg_start**（不是 apex/b_stroke 偏移）记入 `euf_rescan`，跨 append 取历史最小值（`min(euf_persisted, euf_rescan)`，不因候选后续 confirm 或被新 confirmed 段覆盖而丢弃）。
3. `append` 的 `confirmed_bound` 由"末段 end_array_idx"改为 `min(last_end_idx, earliest_unsealed_from.unwrap_or(last_end_idx))`——回退到 unsealed 起点前的最深稳定段端，非固定 1 段。`partition_point(end_idx < confirmed_bound)` 定位 keep。
4. `from_full` 恢复态设 euf=None，**诚实标注**其不重建 skip 历史（仅无 unsealed 前缀处恢复安全；无生产调用者，生产路径 empty()+append 持久累积）。

### soundness 论证（bit-exact 为何成立）

cascade 只向前传播（seg_start=S 的候选复活只影响起点 ≥S 的段）⟹ `[0, confirmed_bound)` 稳定；从 confirmed_bound 后重扫 = 全量重算该后缀（同一 `divide_segments_with_tail` 循环）⟹ 前缀稳定 + 后缀重算 = 全量结果，逐字段 bit-exact。scan_window=0 无限 ⟹ 任意早候选可复活 ⟹ 历史最小值持久化（不丢弃）保证不漏 unsealed 起点。

---

## 三、验收清单逐项（codex #87 六项）

| # | 验收项 | 结果 | 证据 |
|---|--------|------|------|
| 1 | 发散归零（CL/BTC 50K/150K/300K bit-exact + diag div_count=0） | **PASS** | `decisive_endpoint_tower_parity_longhistory` 全 bit-exact（34.4s）；`diag_parser_incr_vs_full_segments` CL 50K div_count=0（修前 709） |
| 2 | 性能计数器（rescan / euf 前移轨迹 / cascade / p95-max） | **数据齐（红线触发）** | `perf_frontier_rescan_counters_88`：见下表。cascade=N/A（未启用候选C） |
| 3 | 级联复活合成测试 | **PASS** | `cascade_revival_bit_exact_incr_vs_full`（always-run，覆盖率自证 euf.is_some+深回退命中） |
| 4 | 全 lib 1403 基线不退化 | **PASS** | 1404 passed / 0 failed（基线 1403 + 新增级联测试 1） |
| 5 | scan_window 敏感性（候选B补测，非阻塞） | **数据齐** | `diag_segment_window_effect` ES≤128K：W∈{50,200} canonical 输出与 W=0 bit-identical（见下） |
| 6 | 信号集变化如实记录 | **见 §五** | 修前/修后 bsp 计数对照 + 下游重跑清单 |

### 性能计数器（`perf_frontier_rescan_counters_88`，CL，ParseLayerIncr 生产路径，L1）

```
       n |   wall_s |    exp  euf_adv  rs_max  rs_p95 | euf_fin  euf_min  adv_r
   50000 |     0.05 |    NaN        1     411     391 |      10       10   1.00
  150000 |     0.43 |   1.90        1    1278    1210 |      10       10   1.00
  300000 |     1.76 |   2.04        1    2534    2413 |      10       10   1.00
```

**判读**：`euf_min=euf_fin=10` 且 `euf_adv=1`（仅初始 None→10，之后永不移动）⟹ confirmed_bound 永久锚死 stroke 10 ⟹ 每 bar 重扫近全部段（rs_max≈段总数）⟹ **wall exp=2.04 = O(n²)**。外推 1M CL ≈ 1.76×(1e6/3e5)^2 ≈ **20s，超报告 15s 基线且二次增长**。**边界条件3 触发**。

---

## 四、边界条件（结论何时翻转）+ 上浮"选择"

1. **性能红线（边界条件3，触发）**：修补版 A 在 CL 上退化为候选 C 的 O(n²)。**根因待判别**：euf 锚死是"persist-forever 过保守"还是"stroke-10 候选真的永不 resolve"——两者性能后果不同：
   - 若候选最终 resolve（second_seq 出现分形，monotone 一旦 true 恒 true）：则**不持久化的 advancing 变体**（euf 每 append 从重扫区间新算，不跨 append 保留历史最小值）可 sound 前移、退回 O(n)。sound 草证：kept 区间 [0, confirmed_bound) 恒无 false（pending）候选 ⟹ 重扫区间的 min-false = 全局 min-false。**但 advancing 变体是裁定明确未授权的算法变更**（裁定 prescribe "历史最小值/不丢弃"，且 codex 曾 reject `candidate_a_direct`），须 codex re-audit soundness，不在本工位单方实装。
   - 若候选真永不 resolve：则 O(n²) 是 scan_window=0 bit-exact 的**内在下界**，候选 C 同底，修补版 A 不更差。
   **建议**：codex 重评 advancing 变体（附 resolve 判别探针），或裁定 accept O(n²)（correctness-first），或切候选 C。
2. **候选 B 补测（边界条件2，留余地）**：ES≤128K，W∈{50,200} 的 canonical `divide_segments` 输出与 W=0 **bit-identical**（eq 全 true）。经验上 canonical 对 W≥50 不敏感——但 (a) 仅 ES 且 ≤128K（<bug 显现的 150K）；(b) W 仍是 canonical-config 定义量，改 W = 定义改动非纯性能。候选 B 维持排除（原则性），经验不敏感性入册供未来 `F_W` 实验对象参考。

---

## 五、信号集变化 + 下游重跑清单（验收6，不自行重跑）

修复**改变 level≥2 信号集**（这是 bug 修复的目的——修前增量塔 level≥2 是 frontier-bug 污染值）：

| 标的/n | 修前增量（buggy） | 修前全量 GT | 修后（bit-exact=GT） |
|---|---|---|---|
| BTC 300K L2 bsp | 19 | 17 | **17** |
| BTC 300K L3 bsp | 3 | 4 | **4** |
| BTC 150K L2/L3 | 发散 | — | bit-exact `bsp/lvl=[317,29,9,2,0]` |
| CL 全档 L2+ | 发散 | — | bit-exact（如 300K `[776,66,7,1,0]`） |

**修后坐实分布**（增量=全量 GT）：
- BTC：50K `[97,8,5,0,0]` / 150K `[317,29,9,2,0]` / 300K `[705,67,17,4,0]`
- CL：50K `[133,13,1,0]` / 150K `[399,33,4,0,0]` / 300K `[776,66,7,1,0]`

**下游需重跑清单**（依赖增量塔 level≥2，修前挂 frontier-bug 污染标注）：
1. `l2-depth-distribution-20260702.md`——depth 分布修前为 frontier 污染上的条件性效力（120 可锚域/depth≥2=5 例），修后须在 bit-exact 塔上重跑得真分布。
2. W-VERIFY 高级别桶 alpha（level≥2 桶）——#13/#86 重测口径。
3. #85 full-z × 残差高级别 alpha 结论。
4. 任何引用"高级别无 alpha / level2-4=0"的结论——H2 判定依赖 bit-exact 塔，现 frontier 污染已清，H2 否证（窗口依赖坐实）可升回，但须在修后塔上复核。

---

## 六、结果包六要素

1. **结论**：修补版 A 实装完成，发散归零（bit-exact CL+BTC 50K/150K/300K + diag div_count 709→0），级联复活合成测试 + 全 lib 1404/0 通过。**性能触发边界条件3**：euf 锚死 stroke 10，ParseLayerIncr 退化 O(n²)（exp 2.04），须上浮重评补救方案。
2. **定义依据**：PDF §六 `T^inc_t=F(h_{0:t})` bit-exact 定义 + §八 `b_t=last sealed boundary`（未确认 frontier 必须重算）+ §九增量等价定理（prefix stability ⟹ 归纳等价）。实装的 `confirmed_bound=min(末段end, earliest_unsealed_from)` 是 §八 "b_t" 的线段层实例——回退到无 SecondKind 复活可触及的最深稳定段端。cascade 前向单调 ⟹ [0,confirmed_bound) 满足 prefix stability。
3. **边界条件**：见 §四——(a) 边界条件3 性能红线已触发（O(n²)，advancing 变体/候选C/accept 三选一待 codex 重评）；(b) 边界条件2 候选B 经验不敏感但定义性维持排除。
4. **下游推论**：level≥2 信号集从 frontier-bug 污染值改为真值（BTC 300K L2 19→17/L3 3→4）；§五 四类下游结论须在 bit-exact 塔上重跑；"高级别无 alpha"H2 判定的 frontier 污染清除，可复核升回。
5. **谱系引用**：`project_frontier_resume_bt_too_late`（本报告坐实其 parser 层根因 + 修复）；`project_level_hole_window_dependence`（frontier bug 半边，本修复收口）；`project_interval_nesting_not_called_in_backtest`。建议 genealogist 评估新增条目：「增量确认边界回退深度必须 ≥ SecondKind 复活可触深度；scan_window=0 无限 ⟹ 无固定深度 sound；持久化历史最小值 sound 但退化 O(n²)（边界条件3）」。
6. **影响声明**：改 `parser/feature_seq.rs`（+skipped_secondkind 字段/getter）、`parser/segment.rs`（+earliest_unsealed_from 字段/getter + append 回退逻辑 + 级联复活测试）、`parser/mod.rs`（+segments_earliest_unsealed 诊断字段透出，PartialEq 排除不影响 bit-exact 对拍）、`backtest/incremental.rs`（+perf_frontier_rescan_counters_88 诊断）。零 commit（交付 team-lead）。影响模块：所有消费增量塔 level≥2 的下游（§五）。**性能**：ParseLayerIncr segment 层从 O(pending)/bar 退化为 O(segs)/bar = O(n²)（边界条件3）。
