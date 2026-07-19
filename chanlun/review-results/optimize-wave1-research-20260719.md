# 优化研究 wave1：prefix pass 三方向分析与方案（2026-07-19）

**任务**：综合 trend_confirm 单源化 / 窗化·内容比对 / MACD memo 三方向，给改动点、预期收益、难度、bit-exact 影响与优先级。
**证据基座**：p122 Phase 0 实测（`/tmp/p122_phase0_report.md`）+ 本研究新增的 p126 插桩两轮实测（探针法与 p122 同款：纯 `Instant`/计数叠加，判据/账本/钟位/输出零改动；探针 `rust/src/bin/p126_view_split.rs` + lib 插桩，用后回滚，日志 `/tmp/p126_250k.err`、`/tmp/p126_1m.err`、`/tmp/p126_1m_r2.err`）。
**消歧**：本探针编号 p126 与既有 `chanlun/review-results/p126-stage3-runbook-20260718.md`（A10 成本注入工作线）同号不同物——本探针仅为计时插桩，用后已回滚删除，与该 runbook 无涉。
**探针自证**：插桩版与同码 p123（同树新编译）250k stdout **逐字一致**；计数口径与 p123 模块头 M2 自检逐位一致（triggers=2485、reevals=3719、L1 1984/1984=100%、wm_cross_without_lower=0）；1M L1 单 view 19.75ms vs p122 实测 19.0ms（插桩开销 ~4%，叶片比例不受影响）。早先与预编译 p123 二进制的 stdout 差异已归因 = 该二进制陈旧（nest.rs 未提交改动晚于二进制构建），与探针无关。
**推演级别**：[实测]=直接测量；[推演]=实测值代入结构论证；[外推]=到 4.6M，不承诺。
**090/v3 口径**：全部方案只改计算顺序/缓存/合并/并行度，不改任何判定逻辑；无概率推断、无回测、无有效市场假设；每条收益标注来源。

## 1. 现状成本结构（p126 实测分解）

### 1.1 总账（1M bar 前缀，p123 稀疏路径）

prefix pass = 227.66s [实测]。其中：

| 分项 | L1 | L2 | L3 | L4 | 合计 |
|---|---|---|---|---|---|
| evals（重估次数） | 8,494（100% 保留） | 5,509（67%） | 2,661（39%） | 864（25%） | 17,528 |
| projection | 0.205s | 0.107s | 0.045s | 0.011s | 0.368s |
| decompose | 0.043s | 0.013s | 0.003s | 0.000s | 0.059s |
| assemble | 20.160s | 7.245s | 1.005s | 0.188s | 28.60s |
| provide | 147.351s | 12.724s | 1.960s | 0.233s | 162.27s |
| **view 合计** | **167.76s（19.75ms/view）** | 20.09s | 3.06s | 0.46s | **191.4s** |
| 内容比对 cmp | 0.101s | 0.135s | 0.056s | 0.023s | 0.315s |
| 快照重建 rebuild | 1.052s | 0.509s | 0.151s | 0.038s | 1.750s |

读数：L1 = 全部 view 税的 87.6%；provide = view 税的 84.8%；固定税（cmp+rebuild）= 1.1%——p122 §5「固定税不值得作为加速动机」在 p123 稀疏路径上复测**仍然成立** [实测]。

### 1.2 provide/assemble 内部叶片（1M，秒；括号内为单次成本）

| 叶片 | L1 | L2 | L3 | 口径 |
|---|---|---|---|---|
| 盘整支 Extreme（`pan_div_structure_extreme`） | 70.88（17.6M 次 × 4.02µs） | 2.25 | 0.06 | 每 call 2×**全切片**扫描 |
| 盘整支 locate（窄锚+前锚回退） | 24.92（59.5M 次 × 419ns） | 1.27 | 0.05 | 1.92 次/段（92% 走前锚回退） |
| 趋势支 `trend_confirm_time`（provide 侧） | 17.09（1.26M 次 × 13.6µs） | 5.41 | 0.90 | 见 §2.1 双算 |
| 趋势支 Extreme 预滤（`range_envelope`×2） | 7.64 | 0.28 | 0.01 | 每 pair 2×**全切片**扫描 |
| 盘整支力度或关系（`segments_diverge_or`） | 4.06（12.3M 次 × 330ns） | 2.10 | 0.82 | 6 次窗口聚合/call |
| 盘整支 head（nearest+kind 门） | 1.11（38.5M 段） | 0.13 | 0.01 | 80.3% 段过 kind 门 |
| 盘整支 span+position | 2.16 | 0.07 | 0.00 | `blocks.position` 线性扫 |
| 趋势支 position+span | 0.15 | 0.01 | 0.00 | 微 |
| **provide 残余（contains+sort+转换+过滤）** | **19.36** | **1.18** | **0.09** | 见 §2.4 |
| assemble：`trend_confirm_time` | 17.42（1.31M 次 × 13.3µs） | 7.00 | 0.98 | **与 provide 侧同入参双算** |
| assemble：`provide_divergence_pairs` | 2.30 | 0.17 | 0.01 | 含全切片 find（§3.2 d） |

关键计数（L1 @1M）：154.2 pairs/view、295.7 blocks/view、4,534 段/view、1,447 盘背事件/view。
规模律（250k→1M，L0 段数 ×4.33）：pan_ext ×87（calls ×19 × 单 call ×4.6，**二次**）、tr_conf 单 call ×4.7、view 单价 ×16.7 ≈ S² [实测，坐实 p122 §6「单 view 成本随 L0 超线性」]。

## 2. 瓶颈分析

### 2.1 trend_confirm_time 双算（实测钉死）

两个调用点，入参**逐字相同**：

- `assemble_level_view`（`level_view.rs:1062`）：`(&segments, &seeds[block.end_center].center, block.dir, side, pair.seg_a, pair.seg_c.0, query.as_of, hist, dif, close_src)`；
- `provide_nest_candidate_events`（`level_view.rs:696`）：`(&segments, &seeds[pair.id.block_end_center].center, pair.id.direction, side, pair.seg_a, pair.seg_c.0, view.query.as_of, hist, dif, close_src)`。

同一性论证：pair 由 `provide_divergence_pairs` 按 block 构造（`id.block_end_center == block.end_center`）；assemble 侧由 `pair.move_start == block.start_index` 反查同一 block；`side` 同由 direction 派生；`segments` 同为 `lower_legs` 的 `leg_as_segment` 映射。
实测封口 [实测]：assemble 侧 confirm 调用数 == pairs 总数（@250k 69,584==69,584；@1M 1,309,501==1,309,501——mappable 门在生产数据从未拦截），provide 侧 = 96.2% pairs（预滤 Extreme 拒掉 4%）。**assemble 的 confirm 集 = pairs 全集 ⊇ provide 的 confirm 集**，双算合计 49.1s@1M（L1 34.5 + L2 12.4 + L3 1.9 + L4 0.3）。
另：`segments` 向量每 view 建 3 遍（`level_view.rs:656/:820/:1003`），`anchors_self`/`self_anchors` 同理——小额同源冗余。

### 2.2 全切片扫描点（窗化缺失，实测主叶）

`Segment` 序列按 `start_index`/`end_index` 双升序（塔不变量，`recursive_tower.rs:118-121` 有 partition_point 先例），但多处判据仍做 `segments.iter()` 全切片 filter/fold，把 O(窗口) 的语义做成 O(级长) 的扫描：

- (a) `move_range_envelope`（`divergence.rs:864`，单一来源）：`segments.iter().filter(span 条件).fold(...)`。调用点 = provide 趋势预滤 ×2/pair（tr_env 叶 7.9s）、`trend_confirm_time` 的 a_env（tr_conf 内）、`signal.rs` 第一类 037:20 合取（生产 per-bar BSP 路径同样消费）。
- (b) `pan_div_structure_extreme` 局部 envelope（`signal.rs:750-762`）：同模式 ×2/call = **最大单叶** pan_ext 73.2s@1M。
- (c) `locate_pan_div_structure_front_anchor` 的 A′ 反查（`signal.rs:735`）：`segments.iter().rev().find(dir ∧ end ≤ c.start)` 从切片末端回扫，跳过的都是 end>c.start 的新段。
- (d) `provide_divergence_pairs` 的 c_terminal 前向 find（`level_view.rs:839`）与 c_end 反向 find（`level_view.rs:860`）：全切片起扫。
- (e) 盘整支 `blocks.iter().position`（`level_view.rs:753`，pan_span 叶 2.2s）：blocks 按 start_center 升序，可二分。

### 2.3 盘整支结构：跨 view 全量重定位

每 view 对 ~4,534 段全跑「nearest+kind 门（80% 过）→ locate 1.92 次/段（92% 窄锚失败回退前锚）→ Extreme（全切片）→ 力度或关系」，产出 1,447 事件/view。但 locate/extreme 的输入只依赖 **(center, seg.start 之前的段前缀)**——段前缀冻结 ⟹ 同一 (center, segment) 的定位结果跨 view 逐字相同；diverge 的 A/C 窗口固定 ⟹ 力度结果亦固定。**当前架构把前缀稳定的计算每 trigger 全量重做**（L1 每 trigger 仅新增 ~1 个 L0 段：L0 段产出率 0.905%/bar × trigger 间隔 ~108 bar [实测]）。这是 pan 支 103s@1M（loc 24.9+ext 70.9+div 4.1+head 1.1+span 2.2）的结构根因。

### 2.4 provide 残余：`out.contains` O(n²) 纯税

`level_view.rs:798` 的去重 `if !out.contains(&event)`：每盘背事件对 `out`（~154 趋势 + 已积盘背）线性扫。实测 **pan_ev == n_div 逐位相等**（@250k 660,996==660,996；@1M 12,290,012==12,290,012）——去重在生产数据上**从未拦截任何事件**，是纯税。L1 @1M = 1,447 次/view × 平均扫 ~800 事件 ≈ 2.3ms/view ≈ 残余 19.4s 的主体（残余合计 20.6s，含 sort ~2s、pending 过滤 ~1s、转换 ~1s）[实测+结构归因]。

### 2.5 MACD：序列已增量，窗口聚合才是真成本

`TowerCache` 的 `MacdState` 已是 O(1)/bar 增量（`mod.rs:859-874`，hist/dif 锁步产出）——**MACD 序列计算不是瓶颈**，本研究不把它列为收益（090：声明与能力一致）。真成本是窗口聚合：`same_color_area`/`segment_dif_peak`/`same_dir_hist_peak` 按 [lo,hi] 区间逐 bar 扫，调用点 = 盘整支 `segments_diverge_or`（6 次/call，7.1s@1M）+ `trend_confirm_time` 的 A 侧基准与 C 侧渐进累加（tr_conf 内，细分见 §2.6）。
**bit-exact 警示（否决项）**：前缀和加速（`area = P[hi+1]−P[lo]`）对 f64 **不保证逐位一致**——`(Σ[0..hi]) − (Σ[0..lo-1])` 与 `Σ[lo..=hi]` 的舍入路径不同，1 ulp 差可翻转 `area_c < area_a` 的严格 `<`。判据逐字冻结约束下**前缀和方案否决**；合法形态只有「memo 同一计算的结果值」与「同序 += 延续的游标」（见 §3.4/§3.5）。

### 2.6 trend_confirm_time 内部结构（第二轮插桩 [实测]，全级合并）

总账：calls=3,046,526（== 两侧调用计数之和，自洽）48.84s ≈ 双算合计 49.09s（差 = 插桩开销）。分项：

| 段 | 时间 | 占比 | 口径 |
|---|---|---|---|
| T4 回拉 0 轴（`dif_crosses_zero`，B 中枢 span 固定窗） | 0.29s | 0.6% | 早退 41,643 次（1.4%） |
| A 侧基准（a_env+area_a+dif_peak_a+hist_peak_a） | 7.14s | 14.6% | **固定窗/pair**；a_env 为全切片扫描（估 ≈60% 份额 [推演]，方案 2 靶），bar 聚合 ≈40% [推演]（方案 4 靶） |
| T3 三买（`trend_third_class_in_c`） | 0.42s | 0.8% | 早退 141,471 次（4.6%） |
| 渐进扫描段（T2 包络 + T5 bar 累加环） | **41.00s** | **83.7%** | legs=234.8M（77/call）、**bars=20.98G（6,886/call）** |

早退分布：map 失败 0 次、T4 拒 1.4%、T3 拒 4.6%、force 终假 43.1%（1,311,782）、**确认 Some 50.9%**（1,551,630——pair 持久驻留，同一 t* 在每个后续 view 从头重扫再确认）。
结构性读数：扫描段的 bar 累加环是绝对主体；而三个事实使「每 view 从头重扫」成为纯税——(i) 扫描状态在任意 leg 处只依赖该 leg 之前的前缀（重扫必沿同一路径到达同一退出点）；(ii) force 转假即终假（c 侧各 proxy 单调只增，代码注释 033:26 已锚）；(iii) t* = 首个 T2∧T5 同真点由前缀决定，确认后逐 view 稳定。map_src 失败零次 ⟹ assemble 侧 mappable 门在生产数据从不拦截（与 §2.1 计数互证）。

### 2.7 内容比对（p123 哨兵）实测：不值得优化

p123 的值比对哨兵 @1M：cmp 0.315s + rebuild 1.750s = 2.07s ≈ prefix 的 0.9% [实测]。Rc holding trick 的两条旧否决理由仍成立：holding 使 `Rc::make_mut` 写站点强计数 >1 ⟹ 每个写 bar O(级长) 写时复制（放大）；holding=1 时 make_mut 原地写、ptr 不变 ⟹ ptr 哨兵对原地改写**漏判**（不健全）。结论：方向 2 的「内容比对」部分实测否决（<1%），资源全部投给「窗化」部分（§3.2）。

### 2.8 L1 独块与并行维度

p122 §3：每级**恰好一个**携带 pending 的 run（L1 run = 全级），p124 按 run 键分片 ⟹ L1 82-88% 的税集中在单片，S=8 实测 ~3.8×。但被忽略的正交维度：**同一 run 的不同 trigger 的求值是纯函数**（输入 = tower@T 快照 + 因果前缀 hist/dif/close_src + as_of=T，与 pending/账本无关——pending 只过滤事件，不进求值）。trigger 全集可按时间维划分到多 worker，账本按 bar 归并（min-merge 幂等）。见 §3.6。

## 3. 方案列表

收益基准：prefix pass 227.66s@1M [实测]；4.6M 账 = p122 §7 [外推]。
任务三方向映射：方向 1（单源化）→ 方案 1；方向 2（窗化/内容比对）→ 方案 2 + 方案 7 否决项；方向 3（MACD memo）→ 方案 4 + 方案 5a 深化；方案 3/6 为实测中浮出的附加发现。

### 方案 1：trend_confirm_time 单源化（方向 1）★推荐首发

- **改动点**（lib，`level_view.rs`）：`LevelAsOfView` 增字段 `confirm_times: Vec<Option<usize>>`（与 `pairs` 对齐）；`assemble_level_view` 在 pairs 产出后对每 pair 计算一次（mappable 门未过存 `None`，与 trend_confirm 内部 map 失败返 None 等价）；moves 循环查表替代 :1062 调用；`provide_nest_candidate_events` 以 `view.pairs.iter().zip(&view.confirm_times)` 查表替代 :696 调用。
- **bit-exact**：同函数、同入参、同输出，查表替换重算；`None` 语义两侧逐字一致（§2.1）。无判据改动。
- **收益** [实测]：消除 provide 侧 tr_conf = 23.5s@1M（L1 17.1 + L2 5.4 + L3 0.9 + L4 0.1）= **prefix 的 10.3%**；4.6M 同比例 [外推 ~10%]。
- **难度**：小（一个字段 + 两个调用点；bin 零改动——`LevelAsOfView` 只由 lib 构造，各 bin 仅经 `&view` 透传）。
- **风险**：低。结构派生 PartialEq 含新字段——view 相等性语义增强（更严格），测试网内无跨构造路径的 view 相等断言受影响（已核 `level_view.rs` 测试与 p83/p95/p112/p119 消费形态）。
- **验证**：250k/1M stdout+dump 与同码基线 diff=0（白名单仅 views=）；`P123_SHADOW=1` 全程对拍 mismatches=0。

### 方案 2：判据窗化（方向 2 主体）★推荐与方案 1 同批

- **改动点**（lib，§2.2 a-e 五处）：
  - (a) `divergence.rs::move_range_envelope`：`lo=partition_point(start<span.0)`、`hi=partition_point(start≤span.1)`，窗内保持原 filter+fold。
  - (b) `signal.rs::pan_div_structure_extreme` 局部 envelope：同法。
  - (c) `signal.rs::locate_pan_div_structure_front_anchor`：`hi=partition_point(end≤c.start_index)`，`[..hi]` 内 rev().find(direction)。
  - (d) `level_view.rs::provide_divergence_pairs`：c_terminal 以 `lo=partition_point(start<last.end_index)` 起扫、c_end 以 `hi=partition_point(end≤as_of)` 界内 rev 扫。
  - (e) `level_view.rs` 盘整支 `blocks.position`：partition_point（start_center 升序）。
- **bit-exact**（逐处可构造证明）：段序列 start/end 双升序 ⟹ 界化窗口恰好包含满足谓词的同一元素集、同一顺序 ⟹ fold/find 结果逐字不变（(a) 已展开论证：start>span.1 ⟹ end>span.1 恒失败；(c)(d) 同构）。
- **收益** [实测靶叶 + 推演削减]：靶叶 = pan_ext 73.2s + tr_env 7.9s + tr_conf 内 a_env 全切片份额 ~4.3s（双侧，§2.6）+ pan_loc 内前锚份额与 asm_pairs 内 find 份额（小额）≈ **85.4s@1M**。窗化后单 call 成本 O(log S + 窗内段数)，窗内段数 = A/C 段跨度 ≈ 数段~数十段（vs 全切片 4,534）⟹ 保守 [推演] 削减 ≥90% 靶叶 = **省 ~77-81s@1M = prefix 的 34-36%**。规模越大收益越大（主靶叶 ∝ S²，4.6M [外推] 靶叶 ~1,500-1,700s）。
- **难度**：小-中（五处局部改写，每处 ≤10 行；(a)(b) 是同一原语形态）。
- **风险**：低-中。风险点仅在「双升序」前提——塔不变量已有 debug 断言守护（`recursive_tower.rs:124`），且 `divergence.rs:774/848` 已有对同一序列 partition_point 的生产先例。
- **验证**：同方案 1；另对 (a) 跑 `strict_nest_check`（037:20 生产判据同原语消费）。

### 方案 3：盘背去重 O(n²)→排序后 dedup（附加发现 A）★推荐同批

- **改动点**（lib，`level_view.rs:798` 附近）：删除循环内 `out.contains`（改为无条件 push），末尾 `sort_by_key`（稳定排序）后接 `dedup_by(PartialEq)`。
- **bit-exact**：全等事件 ⟹ 排序键相等 ⟹ 稳定排序后相邻 ⟹ `dedup_by` 保留首次出现 = contains 语义（首插胜出）；非全等事件各自保留、相对序由稳定排序保持 = 现行为。输出逐字一致（构造性证明；实测旁证：contains 零拦截）。
- **收益** [实测残余归因]：~17-18s@1M = **prefix 的 ~7.7%**（含 sort/过滤净差）。
- **难度**：小（~5 行）。
- **风险**：低。

### 方案 4：窗口聚合 memo（方向 3 的合法形态）

- **形态**：memo 键 = source 端点 `(seg_a.0, seg_a.1, side)` 等，值 = 同一扫描的结果（a_idx、area、dif_peak、hist_peak）——存结果值，不换算法（§2.5 前缀和否决）。失效 = map 端点跨越（新 bar source_index 落入窗口 ⟹ hi 可能扩展）+ hist 前缀重写（确定性重算逐位相同，p123 头已签——memo 跨 clear 仍有效）。
- **首要落点**：盘整支 `segments_diverge_or` 的 A/C 六聚合（7.1s@1M 靶叶，其中 A 侧对固定结构逐 view 重算）；次级落点 = `trend_confirm_time` A 侧 bar 聚合（§2.6：aside 的 ~40% 份额 ≈ 2.9s 双侧，方案 2 砍掉 a_env 后即为 aside 主体）。
- **收益** [推演]：A 侧 memo 命中后每 call 只算 C 侧 + trend aside 聚合 ≈ 省 4-6s@1M（~2%）。
- **难度**：中（memo 挂在 bin 侧 LevelDerived/RunEntry 旁，或 lib 内带键查询；失效条件链需逐条签字）。
- **风险**：中（失效条件漏判 = 陈旧值 ⟹ 需 shadow 对拍封口）。
- **说明**：本方案**不**含「MACD 序列缓存」——序列已是 O(1)/bar 增量（§2.5），无税可收。

### 方案 5：per-pair trend_confirm 游标驻留（设计 §4.3 落地）+ 细粒度 pan memo

- **5a trend_confirm 游标**：按 pair 驻留 (T4 结果、t3、env、acc_hi、area_c、dif/hist 极值)，重估时增量续扫；force 终假 / 已确认 t* 两态 O(1) 直接返回（§2.6 (ii)(iii) 单调性，判据语义不变）。bit-exact 逐条：+= 序列与从头扫描逐项同序 ⟹ f64 状态逐位一致；max/min 顺序无关；T3 首命中对 append 稳定（windows(2) 前缀序）；t* 首命中点前缀决定 ⟹ 稳定。**收益 [实测+推演]**：靶 = 扫描段 41.0s（双侧）；方案 1 单源化后单侧 ~20.5s，游标省 ~85-90% ≈ **17-18s@1M（~8%）**，trend 族残余降至 ~4s。
- **5b 细粒度 pan memo**（§2.3）：键 (center.start_index, seg.start_index) → (kind 门、structure、extreme、diverge)；失效 = ①lower 塔重写水位（需 lib 暴露 L0/L1 确认界——`segments_confirmed_len` 证书机制已存在于 mod.rs #106，暴露为查询即可）②center 属链尾未稳定块（保守界 = 最新 2 块，与 TURN 稳定规则同构）。收益 [推演]：pan 支 103s@1M（方案 2 落地后残余 ~30s）× 命中率 90%+ ≈ **省 ~25-28s**——与方案 2 叠加后 pan 支从 103s 降到 ~3-5s。
- **共同前提**：5a 的 pair 驻留与 5b 的 memo 都需「重写水位」做保守失效（L1 的 lower=L0 几乎每 trigger 变，全条目失效则 memo 无用）；两份方案的 lib 暴露面是同一份，建议同批做。
- **难度**：5a 中（lib 内 cursor 结构 + RunEntry 携带 + 水位失效）；5b 中-高（失效条件两条链 + lib 水位暴露）。
- **风险**：中-高，必须 `P123_SHADOW=1` 全量对拍 + 全量双跑协议封口。建议排在 1/2/3 落地并验证后。

### 方案 6：trigger 轮转分片并行（方法 B′，破解 L1 独块；附加发现 B）

- **形态**：worker i 重放全部 bar、维护本地增量塔，只对 `trigger 序号 ≡ i (mod W)` 的 trigger 做 view 求值（保留 p123 稀疏判据，worker 内逐字）；账本归并 = p124_merge 同架构（candidates/divergences 按 key min-merge——min 幂等，键无需互斥；DIV/TERM 行按 (bar, 片, seq) 全序；CKPT/TURN 跨片逐字相同去重断言；终态 CERT 由归并方重放 terminal pass 单线程装）。
- **bit-exact**（构造性，与 p124 同一证明模板）：求值是纯函数（tower@T、因果前缀、as_of），与 pending 无关——pending 只过滤，故首证钟 = 产出该事件的 trigger bar 的 min，归并 min 即顺序 or_insert 首插；Σviews=顺序值（trigger 全集划分）；suppress/空 pending 语义由「全集 trigger 划分」保持。
- **收益** [推演]：view 税 191.4s@1M（稀疏保留后）÷W + 塔税 31.4s 复制（并行不叠加）+ 归并 ~1-2s。W=8：prefix 227.7 → ~60s（**~3.8×**）；与方案 1/2/3 落地后叠加：view 税 ~73s → prefix ~45s（**~5.1× vs 现行 p123**）。4.6M [外推]：塔 730s + 稀疏保留 view 税（优化后）÷8 ≈ 1,100s vs p122 §7 的 16,400s 基线。
- **难度**：中-高（p124_shard/p124_merge 骨架复用，改分片键与归并的 min 语义；内存 = W 份塔，p124 S=8 已实证可行）。
- **风险**：中。新归并语义（键非互斥）需重做 p124 的归并不变量审计；全量双跑协议同 p124 §「全量双跑」。
- **定位**：B′ 是 p124（run 空间分片，实测退化 ~3.8×）的正交补——run 退化时时间维仍然稠密（9,290 triggers@1M）。两维可统一为 (trigger, run) 二维划分，但 B′ 单独已够。

### 方案 7（实测否决项，备案）

- 内容比对哨兵优化：实测合计 0.9%（§2.7），不做。
- MACD 面积前缀和：f64 非结合 ⟹ 非 bit-exact（§2.5），否决。
- ptr 哨兵（设计 §4.1 Rc holding）：原地改写漏判 + 写时复制放大（§2.7），维持 p123 模块头的否决。
- `segments`/`anchors` 三建合并（§2.1 尾）：~1-2% 微税，可随方案 1 顺手做（`provide_divergence_pairs` 改收预建 segments 涉及跨 bin 签名，单列不划算）。

## 4. 推荐优先级与实施顺序

| 序 | 方案 | 靶叶@1M [实测] | 省@1M [推演] | 占 prefix | 难度 | 风险 | bit-exact 论证形态 |
|---|---|---|---|---|---|---|---|
| 1 | 方案 1 单源化 | 23.5s | ~23.5s | 10.3% | 小 | 低 | 同入参查表（实测计数封口） |
| 2 | 方案 2 窗化 a-e | 85.4s | ~77-81s | 34-36% | 小-中 | 低-中 | 窗口≡谓词集合同序 |
| 3 | 方案 3 去重 | ~20.6s 残余 | ~17-18s | ~7.7% | 小 | 低 | 稳定排序+首插胜出 |
| 4 | 方案 6 B′ 分片 | view 税全量 | prefix ×3.8-5.1 | — | 中-高 | 中 | 纯函数+min 归并 |
| 5 | 方案 5a 游标 | 20.5s（单源后扫描段） | ~17-18s | ~8% | 中 | 中 | 同序 += 延续+单调终态 |
| 6 | 方案 5b pan memo | ~30s（窗化后 pan 支） | ~25-28s | ~12% | 中-高 | 中-高 | 前缀稳定+失效链签字 |
| 7 | 方案 4 聚合 memo | 7.1s+2.9s | ~4-6s | ~2% | 中 | 中 | memo 存同计算结果值 |

**叠加账 [推演]**：方案 1+2+3 落地后 prefix 227.7 → ~108-112s（**~2.1×**）；再叠方案 6（W=8，view 税 ~72s÷8≈9s）→ prefix ~45s（**~5.1×**）；再叠 5a/5b/4 → prefix ~20-25s（**~9-11×**）。4.6M [外推，不承诺]：p122 §7 的 16,400s 基线 → 同结构外推 ~900-1,300s。

**实施顺序**：(1) 方案 1+2+3 同批（同测同验，合计 ~52% prefix 削减，全部 lib 局部改动，零 bin 改动）；→ (2) 方案 6 B′（把剩余 view 税除以 W，与 (1) 乘性叠加；p124 骨架复用）；→ (3) 方案 5a+5b 同批（共享 lib 重写水位暴露面，在 shadow 机制已被 (1) 验证后落地）；→ (4) 方案 4 补位。每批验证协议：250k/1M stdout+dump 对同码基线 diff=0（白名单仅 views=）+ `P123_SHADOW=1` mismatches=0 + 全量双跑（协议照 p123 §M3/p124 §全量双跑）。

## 5. 未封口项（移交）

- 方案 5b 的两条失效链（lower 重写水位暴露面、链尾未稳定块保守界）需构造性签字（照 p123 §6.4 模板逐函数核验）。
- 方案 6 的归并 min 语义不变量审计（p124 归并审计的键非互斥推广）。
- 4.6M 全量的 run 退化是否仍然成立（p122 §8 移交项，与方案 6 的 W 选择相关）。
- 方案 2(c) 前锚窗化的实测份额：pan_loc 24.9s 内窄锚/前锚未分叶（同计时器合并），窗化对前锚的净收益已按保守侧（小额）计入；若实装后超保守估计，归入方案 2 盈余，无需复议。

**产物**：本报告；探针日志 `/tmp/p126_250k.err`（250k 分项）、`/tmp/p126_1m.err`（1M 分项）、`/tmp/p126_1m_r2.err`（1M + trend_confirm 内部细分）；探针码（`p126_view_split.rs` + lib p126_split 插桩）用后回滚，复现方法 = 本报告 §1 探针自证段 + 插桩点位表（level_view.rs 的 provide/assemble 各调用点 + trend_confirm_time 五段，bin 侧 evaluate_run 四段 + 哨兵两段）。
