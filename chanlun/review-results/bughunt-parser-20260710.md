# 解析器 / 分类器深度 Bug 审查报告

审查快照：`gap3-rework-codex9-fix@bf1854d71960`  
范围：`rust/src/theta_v0/parser/`、`rust/src/theta_v0/classifier/`、`rust/src/bin/strict_nest_check.rs`。  
结论：确认 **12 项**高置信缺陷：P0 × 1、P1 × 5、P2 × 6。以下不包含风格意见。

## 发现

### BUG-01：第二类买卖点状态机提前一个次级别走势确认

- **位置**：`rust/src/theta_v0/classifier/rmove_compose.rs:144-166`；错误被测试固定在 `:215-220,282-302`
- **严重级**：**P0**
- **问题描述**：`find_second_type_structure` 将 `i1` 后任意满足“不创新低/高”的走势直接作为 `i2`，只要求 `i2 > i1`，不检查走势方向、回抽阶段或“第一买点后的第二段次级别走势”。Long 测试甚至把 `Direction::Up` 的走势当作“再次下跌”。
- **为什么错**：第一类买点结束后，应先有向上的第一段次级别走势，再由其后的向下第二段走势低点形成第二类买点；卖侧镜像。当前代码会把紧接第一类离开的上涨段 `i1+1` 当作第二类回调段，在上涨结束点提前发出 B2。一级权威 `docs/chanlun/text/blog/021-第21课.md:44` 明确是“第一买点出现后的第二段次级别走势低点”。
- **最小修复建议**：显式加入方向和阶段约束：Long 要求 `i1` 为向下第一类走势，先出现向上段，再选择首个向下且不创新低的 `i2`；Short 完全镜像。补充拒绝 `i2=i1+1`、错误方向以及跳过首个合法回调的测试。

### BUG-02：第二类点价格与其 `source_index` 可能来自不同位置

- **位置**：`rust/src/theta_v0/classifier/rmove_compose.rs:102-110`；`rust/src/theta_v0/classifier/signal.rs:659-675`
- **严重级**：**P1**
- **问题描述**：`second_point_price` 用整个 `RMove` 的 `lo()`/`hi()` 作为“走势结束点价格”，但 `extract_second_signals` 把该价格绑定到走势的 `end_index`。
- **为什么错**：`lo()`/`hi()` 是整段或整棵 Compose 子树的外缘极值，极值可能发生在起点或内部，并不必然发生在 `end_index`。现有 Long 测试的 `m2` 是 Up，`lo=-8` 实际对应上涨起点，却被写到上涨终点坐标。对 Compose 走势，即使方向正确也无法从外缘推出终点价格。
- **最小修复建议**：在 `LeveledMove` 坐标侧车中保存真实 `start_price/end_price`，第二类点直接读取 `subs[i2].end_price`；同时断言输出价格与 `source_index=end_index` 来自同一端点。

### BUG-03：用结构值充当走势身份，重复结构会映射到第一处坐标

- **位置**：`rust/src/theta_v0/classifier/recursive_tower.rs:655-671`；`rust/src/theta_v0/classifier/mod.rs:2037-2045`
- **严重级**：**P1**
- **问题描述**：`index_of_in` 和 `sublevel_diverges` 都通过 `rmove == target` / `.position()` 定位走势，命中第一个结构相等项。
- **为什么错**：两个不同时间位置的走势可以具有相同方向、区间及递归子结构。后一个走势会被映射到前一个走势的 `end_index`，并使用前一个走势计算背驰。源码注释 `recursive_tower.rs:663-665` 已明确承认这种坐标歧义。
- **最小修复建议**：不要把 `RMove::PartialEq` 当身份。让第二类识别和背驰闭包消费 `ElementId` 或数组下标；`SecondTypeStructure` 已持有 `i1/i2`，可直接用这些索引读取对应 `LeveledMove`，无需再次按结构值反查。

### BUG-04：Cand 缓存路径在多 bar 输入上永远失效

- **位置**：`rust/src/theta_v0/classifier/mod.rs:445-459`；状态写入在 `:912-959`
- **严重级**：**P2**
- **问题描述**：`cand_delta_tower_cached` 要求 `cache.macd_state_len == merged_bars.len()`；但多 bar 路径明确只把稳定前缀写入状态，令 `macd_state_len = closes.len()-1`，hist/dif 才包含不稳定尾 bar。
- **为什么错**：当 `n>=2` 时缓存守卫恒假，函数总是回退到全量 `cand_delta_tower`。这使声称用于逐 bar 校验的缓存入口重新退化为 O(n²)，也是 `strict_nest_check` 逐 bar模式不可用的重要原因。
- **最小修复建议**：按真实覆盖契约校验：`n==1` 时 state 长度为 1；`n>=2` 时为 `n-1`，而 hist/dif/close 缓冲长度为 `n`。增加测试断言多 bar 调用确实进入缓存分支，并与全量结果逐字段相等。

### BUG-05：`Chi` 不验证 `chosen` 来自候选集或满足 `Sel_Θ`

- **位置**：`rust/src/theta_v0/classifier/nest.rs:90-132`
- **严重级**：**P1**
- **问题描述**：`LevelNode` 同时保存 `cands` 和 `chosen`，但 `Chi::nest_valid` 只检查各 `chosen` 之间的包含关系。
- **为什么错**：可构造 `cands=[]` 或 `chosen` 不属于 `cands` 的节点，只要伪造的 chosen 区间互相嵌套且 terminal 有信号，`is_confirmed()` 就返回 true。这绕过了文件声明的“`Sel_Θ` 从候选集合选出唯一定位见证”。
- **最小修复建议**：在 `nest_valid` 中逐节点要求 `select_best(&node.cands) == Some(node.chosen)`，空候选直接失败；随后再检查相邻区间包含。增加空候选和非最优 chosen 的反例测试。

### BUG-06：P2“硬门”允许错误证书数量，并可被恒空装配器通过

- **位置**：`rust/src/bin/strict_nest_check.rs:1267-1269,1417-1435`
- **严重级**：**P1**
- **问题描述**：P2 通过条件仅为 `cert_total <= 9 && cert_missing_terminal == 0`。
- **为什么错**：当前基线 `n_C=0` 时，0 到 9 个证书全部 PASS；9 个伪证书不会失败。反过来，若装配器被改坏为始终返回空，零产量仍 PASS。当前 `STRICT-NEST-CHECK.md:41-55` 正是“所有级别证书为 0，但 P2 PASS”，因此该门无法证明装配器真的工作。
- **最小修复建议**：生产数据上使用冻结的精确逐级期望值，而非上界；另增加至少一个必产非零证书的合成 golden fixture，使恒空实现必然失败。

### BUG-07：默认 P1 模式不能证明注释声明的“每 bar 每级一致”

- **位置**：`rust/src/bin/strict_nest_check.rs:440-443,495-536,621-701,1058-1118`
- **严重级**：**P1**
- **问题描述**：P1 注释声明逐 bar 比较 Cand 与 BSP，但默认模式只每 1000 bar 比较一次，最终再比较终态；其余 bar 只累积 BSP 侧，没有计算 Cand 侧。
- **为什么错**：若 Cand/BSP 不一致只持续几十个 bar，随后双方恢复或事件撤回，它可能完全落在两个 checkpoint 之间。终态相等和 BSP 单侧累积都无法发现这种瞬态分叉，`mismatch_bars` 仍为 0。
- **最小修复建议**：修复 BUG-04 后在正式硬门启用真正的逐 bar Cand 增量比较；否则必须把当前模式降级标注为“checkpoint + 终态抽样门”，不能宣称每 bar 等价。

### BUG-08：独立 SecondKind 路径使用错误的“外包络包含合并”

- **位置**：`rust/src/theta_v0/parser/segment.rs:141-160`；消费者 `rust/src/theta_v0/parser/second_kind.rs:91-104`
- **严重级**：**P2**
- **问题描述**：`process_feature_inclusion` 遇到包含时总取 `[min(lo), max(hi)]` 外包络。
- **为什么错**：标准包含处理是有方向的：向上取 high/low 的 max，向下取 min。比如 `[10,20]` 包含 `[12,18]`，向上结果应为 `[12,20]`，当前结果却仍是 `[10,20]`，可能增加或消灭后续底分型。正确方向性实现已经存在于 `feature_seq.rs:109-140,159-181`，因此 `second_kind.rs` 声称与 Python bit-exact，实际却走了不同算法。
- **最小修复建议**：删除中性外包络实现，让 SecondKind 复用 `FeatureSeqState/apply_inclusion` 的方向状态；同时携带原始 `stroke_idx`，避免包含合并后丢失来源身份。

### BUG-09：公开 MACD 状态构造函数在空输入上直接越界

- **位置**：`rust/src/theta_v0/classifier/divergence.rs:211-220`
- **严重级**：**P2**
- **问题描述**：`macd_state_from_closes` 无空输入检查，直接读取 `closes[0]`。
- **为什么错**：同文件 `compute_macd` 明确定义空输入返回空序列，但这个公开增量辅助入口对相同边界 panic，形成全量/状态路径不一致。
- **最小修复建议**：返回 `Option<MacdState>` 或 `Result`，空输入返回 `None/Err`；增加与 `compute_macd(&[])` 对齐的边界测试。

### BUG-10：价格振幅和全变差在合法 `i64` Tick 边界上溢出

- **位置**：`rust/src/theta_v0/classifier/divergence.rs:522-553`
- **严重级**：**P2**
- **问题描述**：`segment_price_amplitude`、`segment_price_speed` 和 `segment_total_variation` 使用 `(b-a).abs()` 及 `i64` 求和。
- **为什么错**：例如 `a=i64::MIN, b=i64::MAX`，减法已超出 `i64`；debug 构建 panic，release 构建会回绕。单步差等于 `i64::MIN` 时 `.abs()` 本身也无法表示正值；TV 的多步和还可能再次溢出。最终会污染 `ForceFeatures` 和力度支配分类。
- **最小修复建议**：用 `i64::abs_diff` 得到 `u64`，TV 用 `u128` 或 checked accumulation；同步把 `ForceFeatures.price_amplitude/tv` 改成能覆盖完整 Tick 差域的无符号宽类型。

### BUG-11：`ForceMeasureAdapter` 实际没有验证其声明的精确单调公理

- **位置**：`rust/src/theta_v0/classifier/force_conformance.rs:56-59,72-95,113-128`
- **严重级**：**P2**
- **问题描述**：文件引用的 `mono` 公理要求 `strength(a)≤strength(b) ⇒ area(a)≤area(b)`，但 `mono_holds` 实际检查 `area_a <= area_b + quantum`。
- **为什么错**：取 `quantum=1`、`area_a=1.4`、`area_b=0.6`，两者 round 后 strength 都为 1，前件成立但精确面积序反向；函数仍因 `1.4 <= 1.6` 返回 true。它验证的是近似容差关系，不是注释和 Origin 声明的公理。另外构造器只用 `debug_assert!(quantum>0)`，release 下 `0`、负数或 NaN 会进入除法和浮点转整数。
- **最小修复建议**：构造器返回 `Result` 并要求 `quantum.is_finite() && quantum>0`；若需要声称精确 conformance，就使用保序且不会把不同面积错误等价的强度表示，或把接口正式改名为近似关系并停止宣称满足原公理。

### BUG-12：公开锚方向 API 对长度不匹配只做 debug 检查，release 可越界

- **位置**：`rust/src/theta_v0/classifier/signal.rs:771-806`；同构路径 `rust/src/theta_v0/classifier/recursive_tower.rs:763-782`
- **严重级**：**P2**
- **问题描述**：`anchor_dirs` 是与 segments 平行的可选切片。乱序分支在验证长度前直接执行 `a[i]`；有序分支也只以 `debug_assert_eq!` 检查，随后按 segment 下标访问。
- **为什么错**：公开函数接收短于 `segments` 的 `Some(anchor_dirs)` 时，debug 构建可能在断言处失败，release 构建则在排序置换或判定循环中索引越界。相同输入在构建模式间呈现不同失败点。
- **最小修复建议**：函数入口先做真实长度校验并返回 `Result`；至少在任何置换或索引前拒绝 `a.len()!=segments.len()`。`level_cand_delta` 应复用同一校验函数，避免两份实现再次漂移。

## 未列为 Bug 的项目

- 没有把严格闭区间 `J_child ⊆ J_parent`、当前证书零产量或 LiveCand 尚未实现本身列为 bug；这些涉及已冻结口径或需要权威裁定，不能仅凭零产量推翻。
- `canonical.rs`、`fractal.rs`、`center.rs`、`decompose.rs`、`bsp.rs`、`six_state.rs`、`level_state.rs`、`tail.rs` 等其余文件未发现达到本报告置信门槛的新增逻辑缺陷。
- 本报告为只读源码、调用链及仓库权威文本交叉审查；未修改工作树。


