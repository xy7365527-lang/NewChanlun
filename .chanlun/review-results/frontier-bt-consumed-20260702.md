# 区间套问题② frontier 链：阶段0 测量裁决 + 根因重定位 + 修复设计

**任务**：#84（区间套问题② frontier 链——b_t/consumed 选取太晚，depth≥2 归零最后一条未测污染链）
**认识论等级**：**L2**（真实 CL+BTC 长历史逐 bar / 终点两口径对拍，可产否定性结果）
**结论**：**非 NO-SHIP——发散坐实**。但根因**不在** task 假设的 `detect_centers_windowed_resume`（tower resume 已 bit-exact），而在**上游 parser 线段增量**（`IncrSegments::append` 单段回退不足）。生产改动须过 codex 审计后才动（子任务已建，阻塞实装）。
**源权威**：`docs/formal-chain/区间套.pdf` §六-§十二（问题② 增量塔 vs 全量重算，编排者 2026-07-01 最后下载=链最新端点）
**HEAD**：eaeee606c3（测量时），零生产代码改动

---

## 一、阶段0 两口径对拍（决定性 L2，非 NO-SHIP）

**探针**：`decisive_endpoint_tower_parity_longhistory`（既有，走生产路径不 fork 坐标，675号）。两口径：
- **现行 frontier 口径**：`IncrementalClassifier`（增量 parser + 增量塔，`run_theta_v0_pi` 实跑 substrate）逐 bar 到终点。
- **PDF §六 canonical 口径**：`classify_with_tower(parse_layer(bars))` 单次全量重算 = F(h_{0:t})。

逐 level `centers`/`bsp` bit-exact 比较。CL（2015→2025 窗口）+ BTC 全量，n∈{50K,150K,300K}。

| 标的/n | L0 | L1 | L2 | L3 | L4 | 判读 |
|---|---|---|---|---|---|---|
| BTC 50K | ✓ | ✓ | ✓ | ✓ | ✓ | bit-exact，bsp/lvl=[97,8,5,0,0] |
| BTC 150K | ✗ 375/374 | ✗ bsp 31/29 | ✗ centers | ✗ bsp 2/2 | — | **L2/L3 发散** |
| BTC 300K | ✗ 784/780 | ✗ 233/230 | ✗ **bsp 19/17** | ✗ **bsp 3/4** | — | **L2 bsp 计数变** |
| CL 各档 | ✗ | ✗ | ✗ | ✗ | ✗ 2/2 | L2/L3/L4 发散 |

**决定性判读**：`Γ^inc_{ℓ≥2} ≠ Γ^full_{ℓ≥2}`（PDF §十）——增量塔在 level≥2 的 centers/bsp 与全量重算发散，**bsp 计数亦变**（BTC 300K L2 = 19 vs 17，L3 = 3 vs 4）。据 PDF §十：`μ̂^inc_{ℓ≥2}` 不是目标估计量，"高级别无 alpha"结论必须**撤回或标注 frontier-bug 污染**。**差异 > 0 ⟹ 非 NO-SHIP**（区别于问题① 的 FN=0 NO-SHIP）。

---

## 二、根因重定位：不在 tower resume，在 parser 线段增量（决定性 L2）

三口径隔离诊断（既有 diag，CL）在 **HEAD 当前** 逐一运行：

| 诊断 | 隔离对象 | 结果 |
|---|---|---|
| `diag_classifier_resume_frontier_divergence` | 同一全量 parse_layer 输入 → 增量塔 vs 全量塔 | **0..1500 无发散** ⟹ tower resume bit-exact |
| `diag_parser_incr_vs_full_segments` | `IncrSegments::append` vs `parse_layer` 线段 | **bar 49291 发散**，`seg[404]`（confirmed_len=406）end 48818/7655 vs 48798/7648；0..50K 共 **709** 次持续发散 |
| `diag_incremental_classifier_first_divergence` | 生产全路径逐 bar | bar 49291，L0 center[120] end 48818/gg 7655 vs 48798/7648 |

**三诊断合成**：
1. **tower `detect_centers_windowed_resume` 已 bit-exact**——task #47/#21 的 `resume_from` 单窗回退 + `pop 末中枢` + `cascade_reset` 修复**有效**（同输入下增量塔=全量塔）。task 假设的 consumed/b_t 塔层修复**已落地**，不是残余 bug。
2. **残余发散源 = parser 线段增量**：生产 center 发散（48818 vs 48798）与 parser seg 发散**逐字段同值** ⟹ 中枢发散是线段发散的**忠实下游传播**，非塔层独立 bug。
3. **发散段 `seg[404] < confirmed_len=406`**：不是末段（seg.len−1=407）——增量把一个**本级更早的确认段**封进了 confirmed 前缀，全量重算（见到 49291 前全部 strokes）却把它重划到更早端点（48798 而非 48818，"多吸收"回退）。这是 confirmed 前缀内部的错误，1 段回退**永不重访** seg[404]。

**这与 task 标题假设的 file 域（econ_positive.rs/nest.rs/mod.rs `detect_centers_windowed_resume`）不符**——真 bug 在 `parser/segment.rs::IncrSegments::append`。此为 scope 重定位（本报告核心发现）。

---

## 三、机制：单段回退 < 真 mutable-frontier 深度（PDF §八 "b_t 选太晚"的线段层实例）

`IncrSegments::append`（segment.rs:489-494）只回退**最后一个 confirmed 段**，从倒数第二段 seg_start 重扫，断言"倒数第二段及之前视为不可变"。该断言在 seg[404] 反例上**证伪**：

- 段确认依赖 `second_seq_has_fractal`（第二特征序列分形）；`config.second_seq_scan_window` **default = 0 = 无限全扫**（feature_seq.rs:146,165；config.rs:65）。
- 无限扫 ⟹ 新 bar 引入的 stroke 可让**任意早**的 `SecondKindPending` 复活（第二序列出现分形），使更早的段端回退。
- ⟹ **不存在固定回退深度（1 段）是 bit-exact 安全的**。1 段回退深度 < 真 mutable-frontier 深度（无限）。

PDF §三/§九 的形式化：增量选的 `b_t`（线段确认边界）太晚，把未确认段放进了可复用前缀 `P_t`，prefix stability `Prefix(F(h_{0:t}),b_t)=Prefix(F(h_{0:t+1}),b_t)` 断裂 ⟹ 归纳证明断裂 ⟹ `T^inc_t ≠ F(h_{0:t})`。PDF §十一 共同结构："最后窗口已登记是局部缓存状态，不是全局确认状态"——此处发生在**线段登记**层。

**定性：实现 bug，非定义冲突**（PDF §七/§十 明文：canonical=全量重算，全量会重估未确认段，增量必须同重估；无定义冲突）。故**不走矛盾上浮**（no-workaround 判据：修复不需改任何定义含义/边界/适用域，只需让增量确认边界回退到可证 sealed 处）。

---

## 四、修复设计（PDF §八，最小 diff 候选——待 codex 审计，阻塞实装）

**改点域**：`parser/segment.rs::IncrSegments::append`（**非** econ/nest/mod tower 域）。

**PDF §八 正确算法**：`b_t = max{end(w): Seal(w,t)=1}`；只保留完全结束于 `b_t` 前的 sealed 段，`b_t` 之后（含所有 SecondKind 复活可触及的段）全部重算。

**候选 A（保守，PDF §八 "更保守写法"，推荐首审）**：段确认边界回退到"当前活跃 SecondKindPending 起点之前最后一个稳定段端"。即回退深度不是固定 1 段，而是回退到**没有任何 pending SecondKind 的第二序列可回指其内**的段端。实现：`IncrSegments` 记录 `earliest_pending_secondkind_stroke_idx`，`confirmed_bound` 取 `min(末段端, 该 pending 起点)`。scan_window=0（无限）下退化为"回退到无 pending 覆盖的最深稳定点"。

**候选 B（有界扫描窗）**：把 `second_seq_scan_window` 从 0（无限）改为有界 W，则回退深度 = W 对应的最大段跨度，固定可算。**风险**：改 W 改变 canonical `divide_segments_with_tail` 判据（全量也用同 scan_window）⟹ 不是纯增量修复，是定义口径改动 ⟹ **越过 no-workaround 边界**（改判据含义）。**不推荐**，仅列作对照。

**候选 C（bit-exact 兜底）**：`IncrSegments::append` 检出"确认前缀内某段被本轮重划改写"时 `cascade` 全量重扫该 parser 层（类比塔层 cascade_reset）。O(n) 退化但保证 bit-exact——ponytail 兜底，性能证书失效但正确性优先。

codex 审计焦点：候选 A 的"pending 起点"锚是否覆盖 scan_window=0 全部复活路径（soundness）；A vs C 的性能/正确性权衡；是否存在 A 无法覆盖的复活模式需退 C。

**验收（PDF §十二 问题②，修后必跑）**：
1. `decisive_endpoint_tower_parity_longhistory` 去 assert 失败 → 全 bit-exact（CL+BTC 50K/150K/300K，L0-L4 全绿）。
2. `diag_parser_incr_vs_full_segments` div_count=0（0..50K）。
3. **depth 分布修前/修后对照**：重跑 `l2_depth_distribution_dx`（l2-depth-distribution-20260702.md 口径），对比修前（120 可锚域/depth≥2=5 例，条件性效力）vs 修后（bit-exact 塔上的真分布）。修后即 depth≥2 污染项清除后的坐实分布。

---

## 结果包六要素

1. **结论**：非 NO-SHIP。增量生产路径与全量重算在 real CL+BTC 长历史 level≥2 发散（BTC 300K L2 bsp 19 vs 17、L3 bsp 3 vs 4；CL L2/L3/L4），`Γ^inc_{ℓ≥2}≠Γ^full_{ℓ≥2}`（PDF §十）坐实——"高级别无 alpha"结论 frontier-bug 污染成立。根因经三诊断隔离**重定位**至 parser 线段增量 `IncrSegments::append` 单段回退不足（**非** task 假设的 tower `detect_centers_windowed_resume`，后者已 bit-exact）。定性=实现 bug 非定义冲突（PDF §七/§十）。修复设计（候选 A 保守回退 / C bit-exact 兜底）+ codex 审计子任务已建（阻塞实装）。
2. **定义依据**：PDF §六「T^inc_t = F(h_{0:t}) ∀t」bit-exact 定义 + §八「b_t=last sealed boundary；未确认 frontier 必须重算」+ §九增量等价定理（prefix stability ⟹ 归纳等价）。实测 seg[404]<confirmed_len=406 且 ≠ 全量 ⟹ 增量把未确认段封进 P_t ⟹ prefix stability 断裂（§三/§九反例的线段层实例）。scan_window=0 无限（config.rs:65）⟹ 无固定回退深度安全。
3. **边界条件（结论何时翻转）**：(a) 若 `second_seq_scan_window` 改为有界 W>0，则复活深度有界，1 段回退可能足够——但改 W 改 canonical 判据（候选 B），越 no-workaround 边界，非纯增量修复。(b) 若换标的/窗口 seg 增量 div_count=0（本 BTC/CL 均 >700），则污染不显——当前两标的均坐实。(c) BTC 50K bit-exact——发散在 n≥150K 才显（confirmed 前缀足够长才触及深回退），小窗回测不暴露此 bug。
4. **下游推论**：三条「depth≥2 归零/退化」链——端点相等链（问题①）已 NO-SHIP 否证、身份层链（anc）已修、**本 frontier 链发散坐实**。l2-depth-distribution-20260702.md 的效力域降级（frontier 污染未清）**成立**，其 120 可锚域/depth≥2=5 例分布在修复前为**条件性**。所有依赖增量塔 level≥2 的 alpha 结论（W-VERIFY 高级别桶、#85 full-z 残差高级别）在修复前须挂 frontier-bug 污染标注（PDF §十）。修复域=parser（非 econ/nest/mod）——task file 域假设需订正。
5. **谱系引用**：`project_frontier_resume_bt_too_late`（memory，本报告**订正**其"改点=recursive_tower.rs/mod.rs consumed"——tower 层已修 bit-exact，残余在 parser IncrSegments 单段回退；bar 49291/多吸收现象在**线段层**复现，非中枢层）；`project_level_hole_window_dependence`（窗口依赖机制，frontier bug 是其一半——本报告坐实该半）；`project_interval_nesting_not_called_in_backtest`；rung-interval-containment-20260702.md §4（问题① 否证后 frontier 链承担污染，本报告收口）；l2-depth-distribution-20260702.md 效力域降级注（本报告坐实污染源）。606号（区间套有效域=Type1）。建议 genealogist 评估新增「增量确认边界回退深度必须 ≥ SecondKind 复活可触深度；scan_window=0 无限 ⟹ 无固定深度安全」条目。
6. **影响声明**：**零生产代码改动**（git clean 验证，rust/src/ 无变更）。仅运行既有 4 个 #[ignore] 诊断/对拍 test（decisive_endpoint_tower_parity_longhistory + 三 diag），无新增 test，无 commit/stash/checkout。护航 `cargo test --release --lib` 1402 基线**不受影响**（无代码变更 ⟹ 无信号集变化 ⟹ 非重跑对象）。产出：本报告 + codex 审计子任务（阻塞 parser 修复实装）。修复实装**不在本任务域内**（gated：生产改动须过 codex 审计后才动）。
