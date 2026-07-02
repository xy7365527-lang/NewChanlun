# Phase B 并行实装计划（全引擎优化，2026-07-02）

输入：6 子系统已复核热点清单（tower/signal/econ/incremental/closedloop/primitives，含 codex verdict）。
去重说明：econ#1、incremental#1、primitives#1 三条 confirmed 是**同一个 mod.rs 克隆簇**（:854/:999-1002/:1081/:1100/:1114/:979），合并为一个工作项（A1+A3 两半，见下）。

## 全局约束（先于一切泳道）

1. **基线协议**：每泳道落地前后各跑一次计时对照——400K bar THETA_PROFILE_STAGES=1（现基线 8.7s）+ 全历史 decompose（现基线 ~6-7min）。
   > **[2026-07-02 基线过时订正（B1 实测 + quality-guard 判定）]** 括号内基线数字系 confirmed_len perf 提交前的陈旧值：400K 端到端实测已是 ~18-27s wall / stage-sum ~23s（并发噪声内），350K decompose 窗实测 ~30s。**各泳道落地前必须在当前 HEAD 重测自己的基线**，target 按重测值降准（A1 锁 1.13-1.17x 缩窗上限、B2 落地前先重测 signal-stage 基线）；本表「预期增益」列的绝对秒数不再作数，相对比值仍供参考。**基线跑与修后跑之间不得夹入 #13 落地**（econ verdict 程序性约束：#13 改信号集，夹入即对照失效）。
2. **YAGNI 重开门（A0）**：mod.rs 克隆簇已被 Lead 2026-06-30 裁定 YAGNI 暂缓。三份 verdict 一致指出量级声称（35min/60-70% memcpy/5-10x）陈旧或无据。重开该裁定的唯一合法证据 = A0 实测（全历史或 ≥1M bar profile + :854/:1081 补插桩标签）。**A0 不通过（克隆簇占比低）则 A1 缩水或撤项。**
3. **bit-exact 语法**：所有项以现有守卫封门——GOLDEN digest、bit_exact_per_bar / bit_exact_synthetic / gen_fastpath_bit_exact_debug oracle、incremental_tower_* 守卫、bit_exact_incr_segments_per_bar 电池。high 风险项额外要求 codex 审 + 新增定向 oracle（见「护航组」）。

## 一、排序（预期增益 × bit-exact 风险 × 文件归属）

| # | 项 | 文件 | 风险 | 增益（verdict 校准后） | #13冲突 | 泳道 |
|---|-----|------|------|------|------|------|
| 1 | first_match_idx 建表移入 is_some() 分支（一行） | signal.rs | none | ~15-30s 白捡 | 无 | B1 |
| 2 | 插桩补标签(:854/:1081) + 全历史 profile（YAGNI 重开门） | mod.rs | none | 决策数据 | 无 | A0 |
| 3 | IncrSegments/IncrStrokes 相同输入早退 | segment.rs | none | parser ~1-2min，病理段更多 | 无 | E1 |
| 4 | decompose 拆 collect_signals+pair_signals，dx 尾部复用 | econ_positive.rs | none | dx harness 2x | **时序** | C1 |
| 5 | mod.rs 克隆簇 Rc 半边（centers/bsp/units Rc+make_mut/借用） | mod.rs | low | 端到端 ~1.5-2x 封顶（verdict 校准，非 3-5x） | **文本** | A1 |
| 6 | bsp 收集循环 Rc::ptr_eq 跳级（依赖 A1） | econ_positive.rs | low | 该组件 ~100x，收集路径再 2-3x | **文件** | C2 |
| 7 | extract_signals_with_hist（消 2×MACD） | signal.rs+mod.rs:248 | none | 数十ms/次全量 classify；逐窗 harness 分钟级 | 无 | B4 |
| 8 | signal 提取 frontier-resume 增量 + 尾部排序归并 | signal.rs | low | 该阶段 10x+，合计 1-3min | **digest基线** | B2 |
| 9 | perm_test 缓冲复用+融合扫描；多 stratum 可复现 bug 上浮 | perm_test.rs | none(+正确性) | 2-3x 常数 + 预注册硬约束修复 | 无 | D1 |
| 10 | MACD 面积 (start,end)→f64 memo（禁前缀和差分） | signal.rs+divergence.rs | none | 常数因子；#13 后成路径 2-10x | 无 | B3 |
| 11 | cand_predicate/econ 线性 find 改 partition_point 二分 + ctx 惰性 | cand_predicate.rs+econ_positive.rs | low | 今日秒级；#13 后成主路径 10-50x | **nest区直接** | C3 |
| — | 护航组：frontier L1+ 投影证书 + cached_units 后缀快照 | mod.rs | **high** | ~115s→秒级，全 harness 省 ~2min | 无 | A3 |
| — | 护航组（条件触发）：resume_from 窗口封存 | mod.rs+recursive_tower.rs | **high** | 当前 <30s，仅中枢稀疏 regime | 无 | A4 |

## 二、并行泳道（同文件串行，异文件并行）

### 泳道 A — classifier/mod.rs（串行：A0 → A1 → A3 → A4）
- **A0（立即）**：给 :854 l0_units clone、:1081 bsp memo 命中 clone 补 THETA_PROFILE_STAGES 标签；跑全历史（或 ≥1M bar）profile，产出克隆簇真实占比。这是 A1 的数据依赖 + YAGNI 重开证据。
- **A1（#13 后）**：Rc 半边。LevelState/LevelCache 的 centers、bsp、cached_bsp 改 Rc<Vec<_>>（照搬 upper_moves 先例 mod.rs:1043-1046，make_mut 写时复制）；memo 命中 = Rc::clone O(1)；l0_units/projected_units 用借用或 mem::take 出借+放回（closes 先例）。**范围裁剪（verdict）**：l0_units 的 take/放回不可直接照搬（:932 move、:873/:879 clear 路径交互），干净做法是 TowerCache 解构拆借；波及 ~28 处消费点机械适配（econ_positive.rs:233/2245/2569 改 .iter()、coverage.rs:544、runner.rs 3 处、interp）。**陷阱**：incremental.rs:645 对拍 harness 跨 bar 持有 Classification → make_mut 退化全拷（仍 bit-exact，收益归零，生产路径不受影响）——文档注明即可。
- **A3（护航组，A1 后，须 codex 审）**：证书半边。cached_units 全量快照(:999) 与 frontier L1+ 全量比较(:979) 的正确修法 = **从 project_to_units_resume 的 pop/extend 簿记导出 per-level dirty_from 证书**（最小改写下标），非「cascade_reset/did_extend 判前缀未动」（verdict 已证伪：did_extend 是全局标志，pop-and-rescan 可在 did_extend=false 时改写尾部 → bar-1464 同类 bug）。约束：cached_units 终长必须 == units.len()（:969 截断语义）；debug_assert 保留全量比较作 debug 护栏。
- **A4（条件触发，默认不做）**：resume_from 窗口封存。触发条件 = 某数据段 profile 显示 05_compose_resume 占比升高。触发后须重跑区间套 PDF §九等价论证 + 全套 incremental_tower_* 守卫。
- **验收**：bit_exact_per_bar + bit_exact_synthetic + gen_fastpath_bit_exact_debug + GOLDEN digest 全绿；A3 另加「稀疏变异事件」定向合成 oracle（bit_exact_synthetic 2000 bar 对 pop-and-rescan 覆盖弱）；400K 计时对照（基线 8.7s，A1 目标砍掉 04/08/10 段 ≥70%）。

### 泳道 B — classifier/signal.rs（+divergence.rs）（串行：B1 → B4 → B2 → B3）
- **B1（立即）**：first_match_idx 建表移入 trend_dir.is_some() 分支。一行，建表无副作用、消费点唯一（:632），逐位恒等。
- **B4**：mod.rs:248 改调 extract_signals_with_hist（hist 已在作用域）。跨文件动 mod.rs 一行——与泳道 A 的 merge 协调点，A0 落地后随时可插。
- **B2（#13 后，digest 基线约束）**：frontier-resume 增量提取 + 尾部排序归并。重判窗口从「末 center 变更点」起（verdict 精化：覆盖 seg[i-1].start_index>=e 或 seg[i].start_index>=e 的段 i）；τ 翻转退化全量重算；归并的前缀截断按 push 位置持久化（稳定排序 tie 语义）。GOLDEN digest 封门。
- **B3**：a_seg_cache 扩为 (区间,面积,close_idx映射)；div_cand/sublevel_diverges 加 (start,end)→f64 memo。**禁用 |hist| 前缀和差分**（改浮点累加序 = bit-exact 必破）。
- **验收**：GOLDEN digest + 400K 计时对照（signal 阶段目标 10x）。

### 泳道 C — backtest/econ_positive.rs + cand_predicate.rs（**整泳道排 #13 之后**；串行：C1 → C2 → C3）
- **C1**：decompose_capturable_spread 拆 collect_signals（纯收集→Vec<tuple>）+ pair_signals（纯函数）；dx 循环门后 push 元组（补 sigma_higher_at 调用），尾部改调 pair_signals。**真封②退化处理（verdict）**：decomp_sum==n_signals 从交叉验证退化为自证——dx 尾部改为对拍生产 collect_signals 输出（assert_eq，O(信号)）替代，或注释如实降级。
- **C2（数据依赖：泳道 A 的 A1 先落地）**：收集循环保存上一 bar 各级 bsp 的 **Rc::clone 强引用**（禁裸指针，防 ABA 假命中），Rc::ptr_eq 命中即跳级。decompose 与 dx 复刻循环同改。
- **C3**：end_index 精确匹配/含段查找改 partition_point（leftmost 语义 + debug_assert 排序不变量）；div_cand 直接在 sub_moves 上取 target+rfind，删 per-rung Vec。
- **C4（撤项）**：FxHashSet——C2 落地后收益归零，仅作 C2 失败的 fallback。
- **验收**：GOLDEN digest + dx 真封检测器全绿（含 C1 的替代对拍）+ 全历史 decompose 计时对照（基线 ~6-7min，C1 目标减半，C2 再 2-3x）。

### 泳道 D — backtest/perm_test.rs（单项，立即可做）
- **D1**：(a) perm_ds 缓冲提外+copy_from_slice（RNG 消耗序不变）；(b) 单遍融合累加 sum_plus/sum_minus（勿用 total−plus 派生）；(c) ge/obs 引用提外。**正确性半边单列**：多 stratum 下 HashMap 迭代序 × RNG 串行消耗 = 冻结种子不可复现（违反预注册硬约束）——修复（BTreeMap/排序迭代）会改多 stratum 输出，**走 /escalate 连同预注册一起裁决**，不与性能半边捆绑落地。
  > **[2026-07-02 状态订正（codex 裁决④，codex-permtest-correctness-ruling-20260702.md）]** 本条状态标签过时：该 bug 真实存在于创世提交 8ccbe58137，但**同日下一提交 7682aa4024 已改 BTreeMap 修复**并加 multi_stratum_reproducible 测试，当前 HEAD 延续修复。裁决：实装 bug 修复不改 estimand（留 errata 不标方法学修订）；W-VERIFY 11 桶 Inconclusive 结果产出晚于修复、不受影响；无需再走 /escalate。遗留独立测试缺口：跨进程复现（父进程 spawn 子进程两跑逐字节比对）未覆盖，单列 follow-up。
- **验收**：same_seed_reproducible + 新增多 stratum 复现测试（裁决后）；无生产调用点，W-VERIFY 接入前落地最省。

### 泳道 E — parser/segment.rs + tail.rs（单项，立即可做）
- **E1**：IncrSegments::append 入口加相同输入早退（strokes.len() 同 ∧ 末笔逐字段等 ⟹ 返回 self；confirmed_len 保旧值 = 更保守证书，sound）；IncrStrokes::append 同理。tail.rs 的 pending 复用**仅搭 E1 顺风车**，不单独立项。
- **验收**：bit_exact_incr_segments_per_bar 电池 + 新增「重复 append 同输入」属性测试；parser 侧计时对照。

## 三、高风险护航组（oracle battery + codex 审强制）

| 项 | 风险源 | 护航要求 |
|----|--------|---------|
| A3 frontier L1+ 证书 | codex 曾出反例区（bar-1464/cascade 投影有损）；「投影相等⟹底层相等」不成立 | 证书从 pop/extend 簿记推导；codex 审设计；稀疏变异定向 oracle；debug 构建保留全量比较 |
| A4 resume_from 封存 | 触及 #47/#21 修过真 bug 的 frontier 语义 | 默认不做；触发后重跑 PDF §九等价论证 + incremental_tower_* 全套 |

## 四、#13 冲突标注（Cand^δ 段2 下沉锚定，改 econ_positive/cand_predicate nest 区 + bsp 提取路径）

| 项 | 冲突类型 | 处置 |
|----|---------|------|
| C3 | nest 区直接文本冲突 + #13 使 div_cand 变热（修了才有大收益） | 排 #13 后 |
| C1/C2 | 同文件（econ_positive.rs 是 #12/#13 在制品）；C1 另有基线时序约束 | 排 #13 后 |
| A1 | cached_bsp Rc 化与 #13 的 1078-1098 memo 区文本冲突；LevelState 消费点波及 econ_positive | 排 #13 后（A0 不冲突，先做） |
| B2 | #13 改 BSP 语义会动 GOLDEN digest 基线 | 排 #13 后 |
| B1/B4/A0/D1/E1 | 无冲突 | 立即可做 |

## 五、不做清单

- closedloop 全部（2 项均 refuted）；l3_delta_r_alpha.rs:192-196（refuted）。
- mu_estimator shrunk_view O(B²)：B 数百、绝对量小，YAGNI——B 或调用频次显著增长再开（其 pooled_mean 既有 HashMap 迭代序不定性记入债务，与 D1 裁决同类）。
- C4 FxHashSet：C2 的 fallback，默认撤。
- A4：条件触发，默认不做。

## 六、执行摘要（首波并行 spawn）

无 #13 依赖、可立即并行：**B1（signal 一行）∥ A0（插桩+profile）∥ E1（parser 早退）∥ D1 性能半边（perm_test）**。
D1 正确性半边同步 /escalate。#13 落地后第二波：**A1 → {A3, C1→C2, C3, B2}**（A1 是 C2 数据依赖，A3 排 A1 后；C 泳道内串行；B2/B4 与 C 并行）。
